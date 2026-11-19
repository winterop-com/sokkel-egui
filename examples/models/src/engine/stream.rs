//! Answers as they are written, as the web's `engine/stream.ts` writes them: each one loads its
//! model if it must, waits out the time the model takes to read the prompt, then streams word by
//! word at the model's pace until it ends, is stopped, or fails. What is being written lives in
//! `Backend::live`; the conversation or the comparison it belongs to keeps the answer once it ends.

use serde::Serialize;

use super::generate::{Ask, pieces, plan, tokens};
use crate::data::backend::Backend;
use crate::data::model::{Level, LogSource, Model, Usage};
use crate::js::{fixed, to_fixed};
use crate::runtime::Host;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Loading,
    Waiting,
    Streaming,
    Done,
    Stopped,
    Failed,
}

impl Phase {
    pub fn over(self) -> bool {
        matches!(self, Phase::Done | Phase::Stopped | Phase::Failed)
    }
}

/// An answer being written.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Live {
    pub model: String,
    pub phase: Phase,
    pub reasoning: String,
    pub text: String,
    pub input: f64,
    pub output: f64,
    /// When it was asked, when its first token came and when it ended.
    pub asked: i64,
    pub first: Option<i64>,
    pub ended: Option<i64>,
    pub rate: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

impl Live {
    fn blank() -> Live {
        Live {
            model: String::new(),
            phase: Phase::Loading,
            reasoning: String::new(),
            text: String::new(),
            input: 0.0,
            output: 0.0,
            asked: 0,
            first: None,
            ended: None,
            rate: 0.0,
            note: None,
            problem: None,
        }
    }
}

/// The usage an answer ended with, or has so far at `now`.
pub fn usage_of(answer: &Live, now: i64) -> Usage {
    let first = answer.first.map_or(0, |first| first - answer.asked);
    let writing = answer.first.map_or(0, |first| answer.ended.unwrap_or(now) - first);
    Usage {
        input: answer.input,
        output: answer.output,
        first_token: first as f64,
        rate: fixed(
            if writing > 0 {
                (answer.output / writing as f64) * 1000.0
            } else {
                answer.rate
            },
            1,
        ),
        latency: (answer.ended.unwrap_or(now) - answer.asked) as f64,
        note: answer.note.clone(),
    }
}

/// Whether any answer is being written for one of the keys that start with `prefix`.
pub fn writing(live: &[(String, Live)], prefix: &str) -> bool {
    live.iter().any(|(key, _)| key.starts_with(prefix))
}

/// What keeps an answer once it ends: the message's version in a conversation, or a comparison's
/// column.
#[derive(Clone, Debug, PartialEq)]
pub enum OnEnd {
    Keep {
        conversation: String,
        message: String,
        at: usize,
    },
    Column {
        comparison: String,
        index: usize,
    },
}

/// One answer under way: its key, whether it ended, and what keeps it.
#[derive(Clone, Debug)]
pub struct AnswerRun {
    pub key: String,
    pub over: bool,
    pub on_end: OnEnd,
}

/// An answer streaming: what it writes and how far it is.
#[derive(Clone, Debug)]
pub struct Writer {
    pub run: u64,
    pub model: Model,
    pub rate: f64,
    pub thinking: Vec<String>,
    pub words: Vec<String>,
    pub at: usize,
    pub output: f64,
    pub first: Option<i64>,
    pub wait_until: i64,
}

impl Backend {
    /// The answer under `key`, if one is being written.
    pub fn live_of(&self, key: &str) -> Option<&Live> {
        self.live.iter().find(|(one, _)| one == key).map(|(_, live)| live)
    }

    /// Changes the answer under `key`, starting from nothing when there is none.
    pub(crate) fn set_live(&mut self, key: &str, change: impl FnOnce(&mut Live)) -> Live {
        match self.live.iter_mut().find(|(one, _)| one == key) {
            Some((_, live)) => {
                change(live);
                live.clone()
            }
            None => {
                let mut live = Live::blank();
                change(&mut live);
                self.live.push((key.to_owned(), live.clone()));
                live
            }
        }
    }

    fn drop_live(&mut self, key: &str) {
        self.live.retain(|(one, _)| one != key);
    }

    /// Writes the answer to `ask` under `key`, and hands it to `on_end` once it ends, stopped or
    /// failed; the answer stays live until what keeps it is answered.
    pub fn answer(&mut self, key: String, ask: Ask, on_end: OnEnd) {
        self.stopping.remove(&key);
        let asked = self.now();
        let model_id = ask.model.id.clone();
        self.set_live(&key, |live| {
            live.model = model_id;
            live.phase = Phase::Loading;
            live.reasoning = String::new();
            live.text = String::new();
            live.input = 0.0;
            live.output = 0.0;
            live.asked = asked;
            live.first = None;
            live.ended = None;
            live.rate = 0.0;
        });
        self.answers_made += 1;
        let run = self.answers_made;
        self.runs.insert(
            run,
            AnswerRun {
                key: key.clone(),
                over: false,
                on_end,
            },
        );
        // Stopped while its model loads, the answer ends at once; the load carries on.
        self.waiting.insert(key, run);
        let loading = self.ensure_loaded(&ask.model, true);
        self.on(loading, move |backend, outcome| match outcome {
            Ok(_) => {
                if !backend.runs.get(&run).is_some_and(|one| one.over) {
                    backend.write(run, ask);
                }
            }
            Err(problem) => backend.end(run, move |live| {
                live.phase = Phase::Failed;
                live.problem = Some(problem);
            }),
        });
    }

    fn end(&mut self, run: u64, change: impl FnOnce(&mut Live)) {
        let Some(answer) = self.runs.get_mut(&run) else { return };
        if answer.over {
            return;
        }
        answer.over = true;
        let key = answer.key.clone();
        let on_end = answer.on_end.clone();
        self.waiting.remove(&key);
        let now = self.now();
        let ended = self.set_live(&key, |live| {
            change(live);
            live.ended = Some(now);
        });
        let kept = match on_end {
            OnEnd::Keep {
                conversation,
                message,
                at,
            } => self.keep(&conversation, &message, at, ended),
            OnEnd::Column { comparison, index } => self.keep_column(&comparison, index, ended),
        };
        self.finally(kept, move |backend| backend.drop_live(&key));
    }

    fn write(&mut self, run: u64, ask: Ask) {
        let key = self.runs[&run].key.clone();
        if self.stopping.contains(&key) {
            self.end(run, |live| live.phase = Phase::Stopped);
            return;
        }
        let planned = plan(&ask);
        self.set_live(&key, |live| {
            live.phase = Phase::Waiting;
            live.input = planned.input;
            live.rate = planned.rate;
            live.note = planned.note.clone();
        });
        self.mark_busy(&ask.model.id, true);
        let wait_until = self.now() + planned.prefill as i64;
        let writer = Writer {
            run,
            model: ask.model.clone(),
            rate: planned.rate,
            thinking: pieces(&planned.reasoning),
            words: pieces(&planned.text),
            at: 0,
            output: 0.0,
            first: None,
            wait_until,
        };
        let timer = std::rc::Rc::new(std::cell::Cell::new(0u64));
        let id = {
            let timer = timer.clone();
            self.set_interval(45, move |backend| backend.step(timer.get()))
        };
        timer.set(id);
        self.writers.insert(id, writer);
    }

    /// One beat of an answer streaming.
    fn step(&mut self, timer: u64) {
        let Some(mut writer) = self.writers.get(&timer).cloned() else {
            return;
        };
        let now = self.now();
        let key = self.runs[&writer.run].key.clone();
        if self.stopping.contains(&key) {
            self.finish(timer, &writer, now, |live| {
                live.phase = Phase::Stopped;
                live.note = None;
            });
            return;
        }
        if now < writer.wait_until {
            return;
        }
        let first = *writer.first.get_or_insert(now);
        let due = (writer.rate * (now - first) as f64) / 1000.0;
        let all = writer.thinking.len() + writer.words.len();
        while writer.at < all && writer.output < due {
            let piece = if writer.at < writer.thinking.len() {
                &writer.thinking[writer.at]
            } else {
                &writer.words[writer.at - writer.thinking.len()]
            };
            writer.output += tokens(piece);
            writer.at += 1;
        }
        let reasoning = writer.thinking[..writer.at.min(writer.thinking.len())].concat();
        let text = if writer.at > writer.thinking.len() {
            writer.words[..writer.at - writer.thinking.len()].concat()
        } else {
            String::new()
        };
        let output = writer.output;
        self.set_live(&key, |live| {
            live.phase = Phase::Streaming;
            live.first = Some(first);
            live.output = output;
            live.reasoning = reasoning;
            live.text = text;
        });
        self.writers.insert(timer, writer.clone());
        if writer.at >= all {
            self.finish(timer, &writer, now, |live| live.phase = Phase::Done);
        }
    }

    fn finish(&mut self, timer: u64, writer: &Writer, now: i64, change: impl FnOnce(&mut Live)) {
        self.clear_timer(timer);
        self.writers.remove(&timer);
        self.mark_busy(&writer.model.id, false);
        self.answered_with(&writer.model.id);
        let mut live = Live::blank();
        change(&mut live);
        if live.phase == Phase::Done {
            let rate = match writer.first {
                None => 0.0,
                Some(first) => (writer.output / ((now - first).max(1) as f64)) * 1000.0,
            };
            self.log(
                Level::Info,
                LogSource::Chat,
                format!(
                    "{} answered {} tokens at {} tok/s",
                    writer.model.name,
                    crate::js::number_to_string(writer.output),
                    to_fixed(rate, 1)
                ),
            );
        }
        let phase = live.phase;
        let stopped = phase == Phase::Stopped;
        self.end(writer.run, move |live| {
            live.phase = phase;
            if stopped {
                live.note = None;
            }
        });
    }

    /// Stops the answer under `key`, keeping what it has written.
    pub fn stop_answer(&mut self, key: &str) {
        let Some(live) = self.live_of(key) else { return };
        let phase = live.phase;
        self.stopping.insert(key.to_owned());
        if phase == Phase::Loading
            && let Some(run) = self.waiting.get(key).copied()
        {
            self.end(run, |live| live.phase = Phase::Stopped);
        }
    }

    /// The runner stopped: every answer stops with it.
    pub(crate) fn on_runner_stop(&mut self) {
        let keys: Vec<String> = self.live.iter().map(|(key, _)| key.clone()).collect();
        for key in keys {
            self.stopping.insert(key.clone());
            let phase = self.live_of(&key).map(|live| live.phase);
            if matches!(phase, Some(Phase::Loading | Phase::Waiting)) {
                self.set_live(&key, |live| {
                    live.problem = Some("The runner stopped before the answer began.".to_owned())
                });
            }
        }
    }
}

/// A model's name for the log, from the library.
pub fn name_of(library: &[Model], id: &str) -> String {
    library
        .iter()
        .find(|model| model.id == id)
        .map_or_else(|| id.to_owned(), |model| model.name.clone())
}
