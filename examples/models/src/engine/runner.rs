//! The local runner, simulated, as the web's `engine/runner.ts` runs it: it starts and stops, loads
//! models into memory (making room by unloading the one used longest ago when the memory it may
//! fill runs out), unloads them by hand or after the keep-alive, and logs what it does.

use crate::data::backend::Backend;
use crate::data::format::{GB, bytes};
use crate::data::model::{Level, LoadState, Loaded, LogSource, Model, RunnerStatus};
use crate::data::seed::model_memory;
use crate::js::{number_to_string, to_fixed};
use crate::runtime::{Handled, Host, Promise, unit};

/// The memory this machine has, of which the runner may fill what the preferences allow.
pub const MACHINE_MEMORY: f64 = 36.0 * GB;

const RUNNER_MISSING: &str = "No runner with the id runner.";

impl Backend {
    pub fn mark_busy(&mut self, model: &str, on: bool) {
        let count = self.busy.get(model).copied().unwrap_or(0);
        self.busy
            .insert(model.to_owned(), (count + if on { 1 } else { -1 }).max(0));
    }

    fn busy_with(&self, model: &str) -> i64 {
        self.busy.get(model).copied().unwrap_or(0)
    }

    pub fn runner_up(&self) -> bool {
        self.runner
            .seen
            .first()
            .is_some_and(|one| one.status == RunnerStatus::Up)
    }

    pub fn start_runner(&mut self) -> Promise {
        let done = self.pending();
        let now_row = self.runner.seen.first().cloned();
        if now_row
            .as_ref()
            .is_some_and(|one| matches!(one.status, RunnerStatus::Up | RunnerStatus::Starting))
        {
            self.settle(done, Ok(unit()));
            return done;
        }
        let version = now_row.map(|one| one.version).unwrap_or_default();
        let since = self.now();
        let starting = self.update(
            |b| &mut b.runner,
            "runner",
            move |row| {
                row.status = RunnerStatus::Starting;
                row.since = since;
            },
        );
        self.on(starting, move |backend, outcome| {
            if outcome.is_err() {
                return backend.settle(done, Err(RUNNER_MISSING.to_owned()));
            }
            backend.log(Level::Info, LogSource::Runner, "starting the runner");
            let paused = backend.sleep(1200);
            backend.on(paused, move |backend, _| {
                let since = backend.now();
                let up = backend.update(
                    |b| &mut b.runner,
                    "runner",
                    move |row| {
                        row.status = RunnerStatus::Up;
                        row.since = since;
                    },
                );
                backend.on(up, move |backend, outcome| {
                    if outcome.is_err() {
                        return backend.settle(done, Err(RUNNER_MISSING.to_owned()));
                    }
                    let said = format!("runner {version} listening on runner.example.org:8400").replacen("  ", " ", 1);
                    backend.log(Level::Info, LogSource::Runner, said);
                    backend.settle(done, Ok(unit()));
                });
            });
        });
        done
    }

    pub fn stop_runner(&mut self) -> Promise {
        let done = self.pending();
        let up = self
            .runner
            .seen
            .first()
            .is_some_and(|one| !matches!(one.status, RunnerStatus::Down | RunnerStatus::Stopping));
        if !up {
            self.settle(done, Ok(unit()));
            return done;
        }
        let since = self.now();
        let stopping = self.update(
            |b| &mut b.runner,
            "runner",
            move |row| {
                row.status = RunnerStatus::Stopping;
                row.since = since;
            },
        );
        self.on(stopping, move |backend, outcome| {
            if outcome.is_err() {
                return backend.settle(done, Err(RUNNER_MISSING.to_owned()));
            }
            backend.on_runner_stop();
            backend.log(Level::Info, LogSource::Runner, "stopping: unloading every model");
            let held: Vec<String> = backend.loaded.seen.iter().map(|one| one.id.clone()).collect();
            backend.unload_each(held, done);
        });
        done
    }

    fn unload_each(&mut self, mut held: Vec<String>, done: Promise) {
        if held.is_empty() {
            let paused = self.sleep(700);
            self.on(paused, move |backend, _| {
                let since = backend.now();
                let down = backend.update(
                    |b| &mut b.runner,
                    "runner",
                    move |row| {
                        row.status = RunnerStatus::Down;
                        row.since = since;
                    },
                );
                backend.on(down, move |backend, outcome| {
                    if outcome.is_err() {
                        return backend.settle(done, Err(RUNNER_MISSING.to_owned()));
                    }
                    backend.log(Level::Warn, LogSource::Runner, "runner stopped");
                    backend.settle(done, Ok(unit()));
                });
            });
            return;
        }
        let id = held.remove(0);
        let removed = self.remove(|b| &mut b.loaded, &id);
        self.on(removed, move |backend, _| backend.unload_each(held, done));
    }

    /// Makes sure a model is in memory, loading it if it is not (for an answer, only when loading
    /// on demand is on): unloading idle models used longest ago when there is no room, and
    /// refusing when even that leaves too little.
    pub fn ensure_loaded(&mut self, model: &Model, for_answer: bool) -> Promise {
        if !self.runner_up() {
            return self.rejected("The runner is not running. Start it from the top bar or the Runner page.");
        }
        if for_answer && model.tags.iter().any(|tag| tag == "embedding") {
            return self.rejected(format!(
                "{} turns text into vectors for search; it does not chat. Pick another model.",
                model.name
            ));
        }
        let there = self.loaded.seen(&model.id).cloned();
        if there.as_ref().is_some_and(|one| one.state == LoadState::Ready) {
            let now = self.now();
            self.update(|b| &mut b.loaded, &model.id, move |row| row.used_at = now);
            return self.resolved();
        }
        if for_answer && there.is_none() && !self.prefs_now().auto_load {
            return self.rejected(format!(
                "{} is not loaded, and loading on demand is off. Load it on the Runner page, or turn loading on demand on in Settings.",
                model.name
            ));
        }
        if let Some(going) = self.loads.get(&model.id) {
            return *going;
        }
        let loading = self.load_model(model.clone());
        let id = model.id.clone();
        let load = self.finally(loading, move |backend| {
            backend.loads.remove(&id);
        });
        self.loads.insert(model.id.clone(), load);
        load
    }

    fn load_model(&mut self, model: Model) -> Promise {
        let done = self.pending();
        let need = model_memory(&model);
        let limit = self.prefs_now().memory * GB;
        if need > limit {
            self.log(
                Level::Error,
                LogSource::Loader,
                format!(
                    "{} needs {}, more than the {} the runner may use",
                    model.name,
                    bytes(need),
                    bytes(limit)
                ),
            );
            self.settle(
                done,
                Err(format!(
                    "{} needs {} of memory and the runner may use {}. Raise the limit in Settings, or pick a smaller model.",
                    model.name,
                    bytes(need),
                    bytes(limit)
                )),
            );
            return done;
        }
        let started = self.now();
        let reserving = match self.reserving {
            Some(promise) => promise,
            None => self.resolved(),
        };
        let reserve = {
            let model = model.clone();
            self.then(reserving, move |backend, _| {
                Handled::Follow(backend.reserve(model, need, limit, started))
            })
        };
        let caught = self.caught(reserve);
        self.reserving = Some(caught);
        self.on(reserve, move |backend, outcome| {
            if let Err(problem) = outcome {
                return backend.settle(done, Err(problem));
            }
            backend.log(
                Level::Info,
                LogSource::Loader,
                format!("loading {} {} ({})", model.name, model.quant, bytes(need)),
            );
            let takes = 500.0 + (model.size / (2.4 * GB)) * 1000.0;
            backend.progress(model, need, started, takes, 250.0, done);
        });
        done
    }

    /// Room made and taken for one load, after the load before it: an `async` function's promise.
    fn reserve(&mut self, model: Model, need: f64, limit: f64, started: i64) -> Promise {
        let done = self.pending();
        let used = self.loaded.seen.iter().fold(0.0, |sum, one| sum + one.memory);
        let mut idle: Vec<Loaded> = self
            .loaded
            .seen
            .iter()
            .filter(|one| one.state == LoadState::Ready && self.busy_with(&one.id) == 0)
            .cloned()
            .collect();
        idle.sort_by_key(|one| one.used_at);
        self.make_room(model, need, limit, started, used, idle, done);
        done
    }

    #[allow(clippy::too_many_arguments)]
    fn make_room(
        &mut self,
        model: Model,
        need: f64,
        limit: f64,
        started: i64,
        used: f64,
        mut idle: Vec<Loaded>,
        done: Promise,
    ) {
        if !idle.is_empty() && used + need > limit {
            let one = idle.remove(0);
            let removed = self.remove(|b| &mut b.loaded, &one.id);
            self.on(removed, move |backend, _| {
                let used = used - one.memory;
                backend.log(
                    Level::Info,
                    LogSource::Loader,
                    format!("unloaded {} to make room for {}", one.model, model.name),
                );
                backend.make_room(model, need, limit, started, used, idle, done);
            });
            return;
        }
        if used + need > limit {
            self.log(
                Level::Error,
                LogSource::Loader,
                format!(
                    "no room for {}: {} in use by models still answering",
                    model.name,
                    bytes(used)
                ),
            );
            self.settle(
                done,
                Err(format!(
                    "There is no room for {}: the models still answering hold {} of {}.",
                    model.name,
                    bytes(used),
                    bytes(limit)
                )),
            );
            return;
        }
        let row = Loaded {
            id: model.id.clone(),
            model: model.name.clone(),
            state: LoadState::Loading,
            progress: 0.0,
            memory: need,
            context: model.context.min(8192.0),
            loaded_at: started,
            used_at: started,
            answers: 0,
        };
        let inserted = self.insert(|b| &mut b.loaded, vec![row]);
        self.on(inserted, move |backend, outcome| {
            backend.settle(done, outcome.map(|_| unit()))
        });
    }

    fn progress(&mut self, model: Model, need: f64, started: i64, takes: f64, at: f64, done: Promise) {
        if at < takes {
            let paused = self.sleep(250);
            self.on(paused, move |backend, _| {
                if backend.loaded.seen(&model.id).is_none() {
                    return backend.settle(done, Err(format!("{} was unloaded while it loaded.", model.name)));
                }
                let progress = (at / takes).min(0.97);
                let updated = backend.update(|b| &mut b.loaded, &model.id, move |row| row.progress = progress);
                backend.on(updated, move |backend, outcome| match outcome {
                    Err(problem) => backend.settle(done, Err(problem)),
                    Ok(_) => backend.progress(model, need, started, takes, at + 250.0, done),
                });
            });
            return;
        }
        let now = self.now();
        let ready = self.update(
            |b| &mut b.loaded,
            &model.id,
            move |row| {
                row.state = LoadState::Ready;
                row.progress = 1.0;
                row.loaded_at = now;
                row.used_at = now;
            },
        );
        self.on(ready, move |backend, outcome| {
            if let Err(problem) = outcome {
                return backend.settle(done, Err(problem));
            }
            let took = (backend.now() - started) as f64 / 1000.0;
            backend.log(
                Level::Info,
                LogSource::Loader,
                format!(
                    "loaded {} {} in {} s ({})",
                    model.name,
                    model.quant,
                    to_fixed(took, 1),
                    bytes(need)
                ),
            );
            backend.settle(done, Ok(unit()));
        });
    }

    /// Takes a model out of memory, saying why in the log.
    pub fn unload_model(&mut self, id: &str, why: &str) -> Promise {
        let done = self.pending();
        let Some(one) = self.loaded.seen(id).cloned() else {
            self.settle(done, Ok(unit()));
            return done;
        };
        let removed = self.remove(|b| &mut b.loaded, id);
        let why = why.to_owned();
        self.on(removed, move |backend, _| {
            let said = if why.is_empty() {
                format!("unloaded {}", one.model)
            } else {
                format!("unloaded {} {why}", one.model)
            };
            backend.log(Level::Info, LogSource::Loader, said);
            backend.settle(done, Ok(unit()));
        });
        done
    }

    /// Notes an answer written by a loaded model, so its keep-alive starts again.
    pub fn answered_with(&mut self, id: &str) {
        let now = self.now();
        if let Some(one) = self.loaded.seen(id).cloned() {
            self.update(
                |b| &mut b.loaded,
                id,
                move |row| {
                    row.used_at = now;
                    row.answers = one.answers + 1;
                },
            );
        }
        // The library says when each model last answered.
        if self.models.seen(id).is_some() {
            self.update(|b| &mut b.models, id, move |row| row.used = Some(now as f64));
        }
    }

    /// Unloads the models idle past the keep-alive, every few seconds while the app is open.
    pub(crate) fn start_runner_engine(&mut self) {
        // The web also settles a start, a stop or a load cut off by a reload once the runner, the
        // preferences and the loaded models are listed; the seed has none of them.
        self.set_interval(5000, |backend| {
            let keep = backend.prefs_now().keep_alive;
            if keep <= 0.0 {
                return;
            }
            let now = backend.now();
            let held = backend.loaded.seen.clone();
            for one in held {
                if one.state != LoadState::Ready || backend.busy_with(&one.id) > 0 {
                    continue;
                }
                if (now - one.used_at) as f64 > keep * 60_000.0 {
                    backend.unload_model(&one.id, &format!("after {} min idle", number_to_string(keep)));
                }
            }
        });
    }
}
