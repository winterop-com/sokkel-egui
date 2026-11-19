//! What the chat and the comparison do, as the web's `engine/chat.ts` does it: a message sent and
//! answered, an answer asked for again or stopped, a conversation started, and one prompt sent to
//! several models at once. Each answer is written by the stream engine and kept in its
//! conversation or comparison once it ends; the changes to one conversation or comparison run one
//! after another (`serially`), each reading it as the one before left it.

use std::rc::Rc;

use sokkel_egui::toggle_reaction;

use super::generate::{Ask, title_of, tokens};
use super::stream::{Live, OnEnd, Phase, usage_of};
use crate::data::backend::Backend;
use crate::data::model::{
    AssistantMessage, Attachment, ColumnState, CompareColumn, Comparison, Conversation, Feedback, Message, Model,
    ReplyState, Sampling, SamplingPatch, UserMessage, Version,
};
use crate::runtime::{Handled, Host, Promise, unit};

/// The key an answer streams under: its conversation, its message and its version.
pub fn chat_key(conversation: &str, message: &str, version: usize) -> String {
    format!("chat:{conversation}:{message}:{version}")
}

/// The key a comparison's column streams under.
pub fn compare_key(comparison: &str, column: usize) -> String {
    format!("cmp:{comparison}:{column}")
}

fn version_of(ended: &Live, base: &Version, now: i64) -> Version {
    Version {
        text: ended.text.clone(),
        reasoning: Some(ended.reasoning.clone()),
        state: match ended.phase {
            Phase::Done => ReplyState::Done,
            Phase::Failed => ReplyState::Failed,
            _ => ReplyState::Stopped,
        },
        usage: ended.first.map(|_| usage_of(ended, ended.ended.unwrap_or(now))),
        problem: ended.problem.clone(),
        ..base.clone()
    }
}

/// Tokens of the conversation before message `up_to`, as the model reads them again.
pub fn history_of(messages: &[Message], up_to: usize) -> f64 {
    messages.iter().take(up_to).fold(0.0, |sum, one| match one {
        Message::User(user) => sum + tokens(&user.text) + 4.0,
        Message::Assistant(reply) => sum + tokens(&reply.versions[reply.at].text) + 4.0,
    })
}

/// A failed answer for a model that is gone.
fn failed(model: &str, problem: &str, now: i64) -> Live {
    Live {
        model: model.to_owned(),
        phase: Phase::Failed,
        reasoning: String::new(),
        text: String::new(),
        input: 0.0,
        output: 0.0,
        asked: now,
        first: None,
        ended: Some(now),
        rate: 0.0,
        note: None,
        problem: Some(problem.to_owned()),
    }
}

fn settings_of(comparison: &Comparison) -> Sampling {
    Sampling {
        temperature: comparison.temperature,
        top_p: 0.9,
        max_tokens: comparison.max_tokens,
        system: String::new(),
    }
}

impl Backend {
    fn model_now(&self, id: &str) -> Option<Model> {
        self.models.seen(id).cloned()
    }

    fn conversation_now(&self, id: &str) -> Option<Conversation> {
        self.conversations.seen(id).cloned()
    }

    fn comparison_now(&self, id: &str) -> Option<Comparison> {
        self.comparisons.seen(id).cloned()
    }

    /// Runs the changes to one conversation or comparison one after another; `run` is an `async`
    /// function, its promise what it gives.
    pub(crate) fn serially(&mut self, key: &str, run: impl FnOnce(&mut Backend) -> Promise + 'static) -> Promise {
        let previous = match self.queues.get(key) {
            Some(promise) => *promise,
            None => self.resolved(),
        };
        let caught = self.caught(previous);
        let next = self.then(caught, move |backend, _| Handled::Follow(run(backend)));
        self.queues.insert(key.to_owned(), next);
        next
    }

    /// An `async` function whose body awaits `step` and ends: its promise.
    fn after(&mut self, step: Promise) -> Promise {
        let done = self.pending();
        self.on(step, move |backend, outcome| {
            backend.settle(done, outcome.map(|_| unit()))
        });
        done
    }

    /// Changes one answer of a conversation, as the conversation is when the change is made.
    fn change_message(
        &mut self,
        conversation: &str,
        message: &str,
        change: impl FnOnce(&AssistantMessage) -> AssistantMessage + 'static,
        touch: bool,
    ) -> Promise {
        let conversation = conversation.to_owned();
        let message = message.to_owned();
        self.serially(&conversation.clone(), move |backend| {
            let Some(now) = backend.conversation_now(&conversation) else {
                return backend.resolved();
            };
            let mut change = Some(change);
            let messages: Vec<Message> = now
                .messages
                .iter()
                .map(|one| match one {
                    Message::Assistant(reply) if reply.id == message => match change.take() {
                        Some(change) => Message::Assistant(change(reply)),
                        None => one.clone(),
                    },
                    _ => one.clone(),
                })
                .collect();
            let time = backend.now();
            let updated = backend.update(
                |b| &mut b.conversations,
                &conversation,
                move |row| {
                    row.messages = messages;
                    if touch {
                        row.time = time;
                    }
                },
            );
            backend.after(updated)
        })
    }

    /// Puts an ended answer in its message's version.
    pub(crate) fn keep(&mut self, conversation: &str, message: &str, at: usize, ended: Live) -> Promise {
        let now = self.now();
        self.change_message(
            conversation,
            message,
            move |one| {
                let mut one = one.clone();
                if let Some(version) = one.versions.get_mut(at) {
                    *version = version_of(&ended, version, now);
                }
                one
            },
            true,
        )
    }

    /// Shows another version of an answer asked for more than once.
    pub fn show_version(&mut self, conversation: &str, message: &str, at: usize) {
        self.change_message(
            conversation,
            message,
            move |one| AssistantMessage { at, ..one.clone() },
            false,
        );
    }

    /// Keeps the feedback given on the version of an answer shown.
    pub fn give_feedback(&mut self, conversation: &str, message: &str, feedback: Option<Feedback>) {
        self.change_message(
            conversation,
            message,
            move |one| {
                let mut one = one.clone();
                let at = one.at;
                if let Some(version) = one.versions.get_mut(at) {
                    version.feedback = feedback;
                }
                one
            },
            false,
        );
    }

    /// Gives or takes back the person's reaction `emoji` on one of their questions.
    pub fn react(&mut self, conversation: &str, message: &str, emoji: &str, name: &str) {
        let conversation = conversation.to_owned();
        let message = message.to_owned();
        let emoji = emoji.to_owned();
        let name = name.to_owned();
        self.serially(&conversation.clone(), move |backend| {
            let Some(now) = backend.conversation_now(&conversation) else {
                return backend.resolved();
            };
            let messages: Vec<Message> = now
                .messages
                .iter()
                .map(|one| match one {
                    Message::User(user) if user.id == message => {
                        let given = toggle_reaction(user.reactions.as_deref().unwrap_or(&[]), &emoji, &name);
                        Message::User(UserMessage {
                            reactions: Some(given),
                            ..user.clone()
                        })
                    }
                    _ => one.clone(),
                })
                .collect();
            let updated = backend.update(
                |b| &mut b.conversations,
                &conversation,
                move |row| row.messages = messages,
            );
            backend.after(updated)
        });
    }

    /// Starts a conversation with a model, its settings from the model's defaults.
    pub fn start_conversation(&mut self, model: &Model, title: &str) -> Promise {
        let now = self.now();
        let (model_id, title, settings) = (model.id.clone(), title.to_owned(), model.defaults.clone());
        let made = self.create(
            |b| &mut b.conversations,
            move |id| Conversation {
                id,
                title,
                model: model_id,
                time: now,
                pinned: false,
                messages: Vec::new(),
                settings,
            },
            None,
        );
        let done = self.pending();
        self.adopt(done, made);
        done
    }

    /// Sends a message in a conversation and has its model answer it.
    pub fn send(&mut self, conversation_id: &str, text: &str, attachments: Vec<Attachment>) -> Promise {
        let done = self.pending();
        let Some(conversation) = self.conversation_now(conversation_id) else {
            self.settle(done, Ok(unit()));
            return done;
        };
        let model = self.model_now(&conversation.model);
        let now = self.now();
        let user = UserMessage {
            id: self.ids.next("m", now),
            text: text.to_owned(),
            time: now,
            attachments: attachments.clone(),
            reactions: None,
        };
        let reply = AssistantMessage {
            id: self.ids.next("m", now),
            versions: vec![Version {
                text: String::new(),
                reasoning: None,
                model: conversation.model.clone(),
                state: ReplyState::Streaming,
                usage: None,
                time: now,
                problem: None,
                feedback: None,
            }],
            at: 0,
        };
        let id = conversation_id.to_owned();
        let reply_id = reply.id.clone();
        let written = {
            let (id, conversation, text) = (id.clone(), conversation.clone(), text.to_owned());
            self.serially(&id.clone(), move |backend| {
                let latest = backend.conversation_now(&id).unwrap_or(conversation);
                let title = if latest.messages.is_empty() && latest.title == "New chat" {
                    title_of(&text)
                } else {
                    latest.title.clone()
                };
                let mut messages = latest.messages.clone();
                messages.push(Message::User(user));
                messages.push(Message::Assistant(reply));
                let updated = backend.update(
                    |b| &mut b.conversations,
                    &id,
                    move |row| {
                        row.messages = messages;
                        row.time = now;
                        row.title = title;
                    },
                );
                backend.after(updated)
            })
        };
        let text = text.to_owned();
        self.on(written, move |backend, outcome| {
            if let Err(problem) = outcome {
                backend.settle(done, Err(problem));
                return;
            }
            let Some(model) = model else {
                let now = backend.now();
                let kept = backend.keep(
                    &id,
                    &reply_id,
                    0,
                    failed(
                        &conversation.model,
                        "This model is no longer in the library. Pick another one for the conversation.",
                        now,
                    ),
                );
                backend.on(kept, move |backend, outcome| {
                    backend.settle(done, outcome.map(|_| unit()))
                });
                return;
            };
            let history = history_of(&conversation.messages, conversation.messages.len());
            backend.answer(
                chat_key(&id, &reply_id, 0),
                Ask {
                    prompt: text,
                    model,
                    settings: conversation.settings.clone(),
                    attachments,
                    history,
                    attempt: 0,
                },
                OnEnd::Keep {
                    conversation: id.clone(),
                    message: reply_id.clone(),
                    at: 0,
                },
            );
            backend.settle(done, Ok(unit()));
        });
        done
    }

    /// Asks for another version of an answer, from the conversation's model as it is now.
    pub fn regenerate(&mut self, conversation_id: &str, message_id: &str) -> Promise {
        let done = self.pending();
        let Some(conversation) = self.conversation_now(conversation_id) else {
            self.settle(done, Ok(unit()));
            return done;
        };
        let index = conversation.messages.iter().position(|one| one.id() == message_id);
        let (reply, question) = match index {
            Some(index) if index > 0 => match (&conversation.messages[index], &conversation.messages[index - 1]) {
                (Message::Assistant(reply), Message::User(question)) => (reply.clone(), question.clone()),
                _ => {
                    self.settle(done, Ok(unit()));
                    return done;
                }
            },
            _ => {
                self.settle(done, Ok(unit()));
                return done;
            }
        };
        let index = index.unwrap_or(0);
        let model = self.model_now(&conversation.model);
        let at = reply.versions.len();
        let version = Version {
            text: String::new(),
            reasoning: None,
            model: conversation.model.clone(),
            state: ReplyState::Streaming,
            usage: None,
            time: self.now(),
            problem: None,
            feedback: None,
        };
        let changed = self.change_message(
            conversation_id,
            message_id,
            move |one| {
                let mut one = one.clone();
                one.versions.push(version);
                one.at = at;
                one
            },
            true,
        );
        let (id, message) = (conversation_id.to_owned(), message_id.to_owned());
        self.on(changed, move |backend, _| {
            let Some(model) = model else {
                let now = backend.now();
                let kept = backend.keep(
                    &id,
                    &message,
                    at,
                    failed(&conversation.model, "This model is no longer in the library.", now),
                );
                backend.on(kept, move |backend, outcome| {
                    backend.settle(done, outcome.map(|_| unit()))
                });
                return;
            };
            let history = history_of(&conversation.messages, index - 1);
            backend.answer(
                chat_key(&id, &message, at),
                Ask {
                    prompt: question.text.clone(),
                    model,
                    settings: conversation.settings.clone(),
                    attachments: question.attachments.clone(),
                    history,
                    attempt: at as i64,
                },
                OnEnd::Keep {
                    conversation: id.clone(),
                    message: message.clone(),
                    at,
                },
            );
            backend.settle(done, Ok(unit()));
        });
        done
    }

    /// Stops whatever the conversation's model is writing.
    pub fn stop_conversation(&mut self, conversation_id: &str) {
        let prefix = format!("chat:{conversation_id}:");
        let keys: Vec<String> = self.live.iter().map(|(key, _)| key.clone()).collect();
        for key in keys {
            if key.starts_with(&prefix) {
                self.stop_answer(&key);
            }
        }
    }

    pub fn set_settings(&mut self, conversation_id: &str, patch: SamplingPatch) {
        let id = conversation_id.to_owned();
        self.serially(&id.clone(), move |backend| match backend.conversation_now(&id) {
            None => backend.resolved(),
            Some(conversation) => {
                let settings = patch.apply(&conversation.settings);
                let updated = backend.update(|b| &mut b.conversations, &id, move |row| row.settings = settings);
                backend.after(updated)
            }
        });
    }

    pub(crate) fn keep_column(&mut self, id: &str, index: usize, ended: Live) -> Promise {
        let id = id.to_owned();
        self.serially(&id.clone(), move |backend| {
            let Some(comparison) = backend.comparison_now(&id) else {
                return backend.resolved();
            };
            let now = backend.now();
            let columns: Vec<CompareColumn> = comparison
                .columns
                .iter()
                .enumerate()
                .map(|(at, column)| {
                    if at != index {
                        return column.clone();
                    }
                    CompareColumn {
                        text: ended.text.clone(),
                        reasoning: Some(ended.reasoning.clone()),
                        state: match ended.phase {
                            Phase::Done => ColumnState::Done,
                            Phase::Failed => ColumnState::Failed,
                            _ => ColumnState::Stopped,
                        },
                        usage: ended.first.map(|_| usage_of(&ended, ended.ended.unwrap_or(now))),
                        problem: ended.problem.clone(),
                        ..column.clone()
                    }
                })
                .collect();
            let updated = backend.update(|b| &mut b.comparisons, &id, move |row| row.columns = columns);
            backend.after(updated)
        })
    }

    fn run_column(&mut self, comparison: &Comparison, index: usize, attempt: i64) {
        let column = &comparison.columns[index];
        let Some(model) = self.model_now(&column.model) else {
            let now = self.now();
            self.keep_column(
                &comparison.id,
                index,
                failed(&column.model, "This model is no longer in the library.", now),
            );
            return;
        };
        self.answer(
            compare_key(&comparison.id, index),
            Ask {
                prompt: comparison.prompt.clone(),
                model,
                settings: settings_of(comparison),
                attachments: Vec::new(),
                history: 0.0,
                attempt,
            },
            OnEnd::Column {
                comparison: comparison.id.clone(),
                index,
            },
        );
    }

    /// Sends one prompt to every model named, side by side, and keeps it as a comparison.
    pub fn compare(&mut self, prompt: &str, models: &[String], temperature: f64, max_tokens: f64) -> Promise {
        let now = self.now();
        let prompt = prompt.to_owned();
        let columns: Vec<CompareColumn> = models
            .iter()
            .map(|model| CompareColumn {
                model: model.clone(),
                state: ColumnState::Waiting,
                text: String::new(),
                reasoning: None,
                usage: None,
                problem: None,
            })
            .collect();
        let made = self.create(
            |b| &mut b.comparisons,
            move |id| Comparison {
                id,
                prompt,
                time: now,
                temperature,
                max_tokens,
                columns,
            },
            None,
        );
        let done = self.pending();
        self.on(made, move |backend, outcome| match outcome {
            Err(problem) => backend.settle(done, Err(problem)),
            Ok(value) => {
                let made = value.downcast_ref::<Comparison>().cloned().expect("a comparison");
                for index in 0..made.columns.len() {
                    backend.run_column(&made, index, 0);
                }
                backend.settle(done, Ok(Rc::new(made)));
            }
        });
        done
    }

    /// Asks one column's model again.
    pub fn rerun_column(&mut self, id: &str, index: usize) -> Promise {
        let key = id.to_owned();
        let next = self.serially(id, move |backend| {
            let run = backend.pending();
            match backend.comparison_now(&key) {
                None => backend.settle(run, Ok(unit())),
                Some(comparison) => {
                    let columns: Vec<CompareColumn> = comparison
                        .columns
                        .iter()
                        .enumerate()
                        .map(|(at, column)| {
                            if at != index {
                                return column.clone();
                            }
                            CompareColumn {
                                state: ColumnState::Waiting,
                                text: String::new(),
                                reasoning: Some(String::new()),
                                usage: None,
                                problem: None,
                                ..column.clone()
                            }
                        })
                        .collect();
                    let updated = backend.update(|b| &mut b.comparisons, &key, move |row| row.columns = columns);
                    backend.adopt(run, updated);
                }
            }
            run
        });
        let done = self.pending();
        self.on(next, move |backend, outcome| {
            if let Ok(value) = &outcome
                && let Some(next) = value.downcast_ref::<Comparison>()
            {
                let next = next.clone();
                let attempt = backend.now() % 97;
                backend.run_column(&next, index, attempt);
            }
            backend.settle(done, outcome.map(|_| unit()));
        });
        done
    }

    pub fn stop_column(&mut self, id: &str, index: usize) {
        self.stop_answer(&compare_key(id, index));
    }

    pub fn stop_comparison(&mut self, id: &str) {
        let prefix = format!("cmp:{id}:");
        let keys: Vec<String> = self.live.iter().map(|(key, _)| key.clone()).collect();
        for key in keys {
            if key.starts_with(&prefix) {
                self.stop_answer(&key);
            }
        }
    }
}
