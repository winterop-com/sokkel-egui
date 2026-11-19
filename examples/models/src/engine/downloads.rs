//! Pulls, simulated, as the web's `engine/downloads.ts` runs them: a few at a time as the
//! preferences allow, the rest queued; each at the speed its source gives, shared under the
//! bandwidth limit; paused, resumed, cancelled and tried again; refused when the disk has no room.
//! A finished pull lands in the library with its files.

use crate::data::backend::Backend;
use crate::data::catalog::{CatalogEntry, Variant, entry_of, model_id};
use crate::data::disk::usage_of;
use crate::data::format::{MB, bytes};
use crate::data::model::{Download, DownloadState, Level, LogSource, Source};
use crate::data::seed::model_of;
use crate::runtime::{Host, Promise, unit};

/// Bytes a second each source gives one pull, before the limit.
fn source_speed(source: Source) -> f64 {
    match source {
        Source::Hub => 92.0 * MB,
        Source::Runner => 640.0 * MB,
        Source::Desktop => 480.0 * MB,
    }
}

/// How often the pulls move along, in milliseconds.
pub const TICK: i64 = 400;

/// How fast a source gives a pull at a moment: it wanders a little, as a network does.
pub fn speed_of(download: &Download, now: i64) -> f64 {
    let base = source_speed(download.source);
    let phase = (download.started % 997) as f64 / 997.0;
    let now = now as f64;
    base * (0.82 + 0.18 * (now / 1700.0 + phase * 6.0).sin() + 0.06 * (now / 430.0 + phase * 11.0).sin())
}

/// A tick moving the pulls along: what it read as it began, and how far it is.
struct Tick {
    now: i64,
    parallel: f64,
    bandwidth: f64,
    root: String,
    rows: Vec<Download>,
    running: Vec<Download>,
    waiting: Vec<Download>,
}

impl Backend {
    /// The pull of an entry's variant under way or waiting, if there is one.
    pub fn active_pull(&self, entry: &str, quant: &str) -> Option<&Download> {
        self.downloads
            .seen
            .iter()
            .find(|one| one.entry == entry && one.quant == quant && one.state.going())
    }

    /// Starts pulling an entry's variant, or says why it cannot.
    pub fn pull(&mut self, entry: &CatalogEntry, variant: &Variant) -> Promise {
        let done = self.pending();
        let id = model_id(&entry.name, &variant.quant);
        if self.models.seen(&id).is_some() {
            self.settle(
                done,
                Err(format!("{} {} is already in the library.", entry.name, variant.quant)),
            );
            return done;
        }
        let key = format!("{}:{}", entry.id, variant.quant);
        if self.active_pull(&entry.id, &variant.quant).is_some() || self.queueing.contains(&key) {
            self.settle(done, Ok(unit()));
            return done;
        }
        self.queueing.insert(key.clone());
        let now = self.now();
        let (entry, variant) = (entry.clone(), variant.clone());
        let row = {
            let (entry, variant) = (entry.clone(), variant.clone());
            move |id| Download {
                id,
                entry: entry.id,
                quant: variant.quant,
                name: entry.name,
                publisher: entry.publisher,
                source: entry.source,
                format: variant.format,
                size: variant.size,
                done: 0.0,
                speed: 0.0,
                state: DownloadState::Queued,
                started: now,
                ended: None,
                error: None,
            }
        };
        let made = self.create(|b| &mut b.downloads, row, None);
        let kept = self.finally(made, move |backend| {
            backend.queueing.remove(&key);
        });
        self.on(kept, move |backend, outcome| {
            if let Err(problem) = outcome {
                return backend.settle(done, Err(problem));
            }
            backend.log(
                Level::Info,
                LogSource::Pull,
                format!(
                    "queued {} {} ({}) from {}",
                    entry.name,
                    variant.quant,
                    bytes(variant.size),
                    entry.source.label()
                ),
            );
            backend.settle(done, Ok(unit()));
        });
        done
    }

    fn awaited(&mut self, step: Promise) -> Promise {
        let done = self.pending();
        self.on(step, move |backend, outcome| {
            backend.settle(done, outcome.map(|_| unit()))
        });
        done
    }

    pub fn pause_pull(&mut self, id: &str) -> Promise {
        let updated = self.update(
            |b| &mut b.downloads,
            id,
            |row| {
                row.state = DownloadState::Paused;
                row.speed = 0.0;
            },
        );
        self.awaited(updated)
    }

    pub fn resume_pull(&mut self, id: &str) -> Promise {
        let updated = self.update(
            |b| &mut b.downloads,
            id,
            |row| {
                row.state = DownloadState::Queued;
                row.error = None;
            },
        );
        self.awaited(updated)
    }

    pub fn cancel_pull(&mut self, id: &str) -> Promise {
        let one = self.downloads.seen(id).cloned();
        let now = self.now();
        let updated = self.update(
            |b| &mut b.downloads,
            id,
            move |row| {
                row.state = DownloadState::Cancelled;
                row.speed = 0.0;
                row.done = 0.0;
                row.ended = Some(now);
            },
        );
        let done = self.pending();
        self.on(updated, move |backend, outcome| {
            if let Err(problem) = outcome {
                return backend.settle(done, Err(problem));
            }
            if let Some(one) = one {
                backend.log(
                    Level::Info,
                    LogSource::Pull,
                    format!("cancelled {} {}; its partial files were removed", one.name, one.quant),
                );
            }
            backend.settle(done, Ok(unit()));
        });
        done
    }

    pub fn retry_pull(&mut self, id: &str) -> Promise {
        let now = self.now();
        let updated = self.update(
            |b| &mut b.downloads,
            id,
            move |row| {
                row.state = DownloadState::Queued;
                row.done = 0.0;
                row.speed = 0.0;
                row.error = None;
                row.ended = None;
                row.started = now;
            },
        );
        self.awaited(updated)
    }

    /// Takes the pulls that ended (landed, cancelled or failed) off the list.
    pub fn clear_ended(&mut self) -> Promise {
        let done = self.pending();
        let ended: Vec<String> = self
            .downloads
            .seen
            .iter()
            .filter(|one| one.state.ended())
            .map(|one| one.id.clone())
            .collect();
        self.remove_each(ended, done);
        done
    }

    fn remove_each(&mut self, mut ids: Vec<String>, done: Promise) {
        if ids.is_empty() {
            self.settle(done, Ok(unit()));
            return;
        }
        let id = ids.remove(0);
        let removed = self.remove(|b| &mut b.downloads, &id);
        self.on(removed, move |backend, _| backend.remove_each(ids, done));
    }

    /// Removes a pull from the list, as the Downloads page's row action does.
    pub fn remove_pull(&mut self, id: &str) -> Promise {
        self.remove(|b| &mut b.downloads, id)
    }

    /// Moves the pulls along while the app is open.
    pub(crate) fn start_download_engine(&mut self) {
        self.set_interval(TICK, |backend| {
            if backend.ticking {
                return;
            }
            backend.ticking = true;
            let ticked = backend.tick();
            backend.finally(ticked, |backend| backend.ticking = false);
        });
    }

    fn tick(&mut self) -> Promise {
        let done = self.pending();
        let prefs = self.prefs_now();
        let rows = self.downloads.seen.clone();
        let running: Vec<Download> = rows
            .iter()
            .filter(|one| one.state == DownloadState::Running)
            .cloned()
            .collect();
        let mut waiting: Vec<Download> = rows
            .iter()
            .filter(|one| one.state == DownloadState::Queued)
            .cloned()
            .collect();
        waiting.sort_by_key(|one| one.started);
        let tick = Tick {
            now: self.now(),
            parallel: prefs.parallel,
            bandwidth: prefs.bandwidth,
            root: prefs.root,
            rows,
            running,
            waiting,
        };
        self.start_waiting(tick, 0, done);
        done
    }

    /// Waiting pulls start, oldest first, as slots free up.
    fn start_waiting(&mut self, mut tick: Tick, at: usize, done: Promise) {
        let Some(one) = tick.waiting.get(at).cloned() else {
            return self.move_running(tick, 0, done);
        };
        if tick.running.len() as f64 >= tick.parallel {
            return self.move_running(tick, 0, done);
        }
        let usage = usage_of(&tick.root, &self.models.seen, &tick.rows);
        if usage.free < one.size - one.done {
            let error = format!(
                "Not enough room on {}: {} needed, {} free.",
                usage.disk.name,
                bytes(one.size - one.done),
                bytes(usage.free)
            );
            let now = tick.now;
            let said = error.clone();
            let updated = self.update(
                |b| &mut b.downloads,
                &one.id,
                move |row| {
                    row.state = DownloadState::Failed;
                    row.error = Some(said);
                    row.ended = Some(now);
                    row.speed = 0.0;
                },
            );
            self.on(updated, move |backend, outcome| {
                if let Err(problem) = outcome {
                    return backend.settle(done, Err(problem));
                }
                backend.log(
                    Level::Error,
                    LogSource::Pull,
                    format!("{} {}: {error}", one.name, one.quant),
                );
                backend.start_waiting(tick, at + 1, done);
            });
            return;
        }
        let updated = self.update(|b| &mut b.downloads, &one.id, |row| row.state = DownloadState::Running);
        self.on(updated, move |backend, outcome| {
            if let Err(problem) = outcome {
                return backend.settle(done, Err(problem));
            }
            backend.log(
                Level::Info,
                LogSource::Pull,
                format!("pulling {} {} from {}", one.name, one.quant, one.source.label()),
            );
            tick.running.push(Download {
                state: DownloadState::Running,
                ..one
            });
            backend.start_waiting(tick, at + 1, done);
        });
    }

    fn move_running(&mut self, tick: Tick, at: usize, done: Promise) {
        let Some(one) = tick.running.get(at).cloned() else {
            self.settle(done, Ok(unit()));
            return;
        };
        let limit = if tick.bandwidth > 0.0 {
            (tick.bandwidth * MB) / (tick.running.len().max(1) as f64)
        } else {
            f64::INFINITY
        };
        let speed = speed_of(&one, tick.now).min(limit);
        let moved = one.size.min(one.done + (speed * TICK as f64) / 1000.0);
        if moved < one.size {
            let updated = self.update(
                |b| &mut b.downloads,
                &one.id,
                move |row| {
                    row.done = moved;
                    row.speed = speed;
                },
            );
            self.on(updated, move |backend, outcome| match outcome {
                Err(problem) => backend.settle(done, Err(problem)),
                Ok(_) => backend.move_running(tick, at + 1, done),
            });
            return;
        }
        let landed = self.land(one);
        self.on(landed, move |backend, outcome| match outcome {
            Err(problem) => backend.settle(done, Err(problem)),
            Ok(_) => backend.move_running(tick, at + 1, done),
        });
    }

    fn land(&mut self, one: Download) -> Promise {
        let done = self.pending();
        let found = entry_of(&one.entry).and_then(|entry| {
            entry
                .variants
                .iter()
                .find(|each| each.quant == one.quant)
                .map(|variant| (entry, variant))
        });
        let Some((entry, variant)) = found else {
            let now = self.now();
            let updated = self.update(
                |b| &mut b.downloads,
                &one.id,
                move |row| {
                    row.state = DownloadState::Failed;
                    row.error = Some("The source no longer lists this model.".to_owned());
                    row.ended = Some(now);
                    row.speed = 0.0;
                },
            );
            self.on(updated, move |backend, outcome| {
                backend.settle(done, outcome.map(|_| unit()))
            });
            return done;
        };
        let model = model_of(entry, variant, one.source, self.now());
        let id = model.id.clone();
        let adding = model.clone();
        // A model already in the library, as from a pull of the same variant from elsewhere, is kept once.
        let then_mark = move |backend: &mut Backend| {
            let ended = backend.now();
            let size = one.size;
            let updated = backend.update(
                |b| &mut b.downloads,
                &one.id,
                move |row| {
                    row.state = DownloadState::Done;
                    row.done = size;
                    row.speed = 0.0;
                    row.ended = Some(ended);
                },
            );
            backend.on(updated, move |backend, outcome| {
                if let Err(problem) = outcome {
                    return backend.settle(done, Err(problem));
                }
                backend.log(
                    Level::Info,
                    LogSource::Pull,
                    format!(
                        "{} {} is in the library ({}, {} files)",
                        one.name,
                        one.quant,
                        bytes(model.size),
                        model.files.len()
                    ),
                );
                backend.pulled.push(Download {
                    state: DownloadState::Done,
                    done: size,
                    speed: 0.0,
                    ended: Some(ended),
                    ..one
                });
                backend.settle(done, Ok(unit()));
            });
        };
        if self.models.seen(&id).is_none() {
            let inserted = self.insert(|b| &mut b.models, vec![adding]);
            self.on(inserted, move |backend, outcome| match outcome {
                Err(problem) => backend.settle(done, Err(problem)),
                Ok(_) => then_mark(backend),
            });
        } else {
            then_mark(self);
        }
        done
    }
}
