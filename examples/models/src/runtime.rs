//! What the browser runs the web's engines on, held by hand: a clock, its timers and intervals,
//! and promises with their reactions run as microtasks, every microtask run before the next timer
//! fires. The web's engines are written with `async` functions, `then`, `catch` and `finally`;
//! here each is spelled with the same steps (`then`, `adopt`, `finally`), so two answers that end
//! in the same moment reach the backend in the order the web's do and the log, the ids and the
//! rows come out alike.
//!
//! An `await` on a promise is `on`: its continuation runs one microtask after the promise
//! settles. An `async` function's own promise is settled where its body ends; one that returns a
//! promise is `adopt`ed, two microtasks later, as the web resolves a promise with a promise.

use std::any::Any;
use std::collections::VecDeque;
use std::rc::Rc;

/// What a promise is fulfilled with.
pub type Value = Rc<dyn Any>;
/// A promise's outcome: its value, or the message it was rejected with.
pub type Outcome = Result<Value, String>;

/// The value of a promise fulfilled with nothing.
pub fn unit() -> Value {
    Rc::new(())
}

/// A promise, by its place in the runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Promise(usize);

/// What a `then` handler gives: a value or a refusal, or a promise its derived promise follows.
pub enum Handled {
    Done(Outcome),
    Follow(Promise),
}

pub type Task<B> = Box<dyn FnOnce(&mut B)>;
type Reaction<B> = Box<dyn FnOnce(&mut B, Outcome)>;

struct Slot<B> {
    settled: Option<Outcome>,
    reactions: Vec<Reaction<B>>,
}

enum Job<B> {
    Once(Task<B>),
    Every(Rc<dyn Fn(&mut B)>),
}

struct Timer<B> {
    at: i64,
    seq: u64,
    every: Option<i64>,
    job: Job<B>,
}

/// The clock, the timers and the microtasks of one page.
pub struct Runtime<B> {
    pub now: i64,
    seq: u64,
    timers: Vec<Timer<B>>,
    micro: VecDeque<Task<B>>,
    promises: Vec<Slot<B>>,
}

impl<B> Runtime<B> {
    pub fn new(now: i64) -> Self {
        Runtime {
            now,
            seq: 0,
            timers: Vec::new(),
            micro: VecDeque::new(),
            promises: Vec::new(),
        }
    }

    /// The time of the next timer due, if any.
    pub fn next_due(&self) -> Option<i64> {
        self.timers.iter().map(|timer| timer.at).min()
    }

    /// What a promise settled with, once it has: the app reads its answer as a page's `then` would.
    pub fn settled(&self, promise: Promise) -> Option<&Outcome> {
        self.promises.get(promise.0).and_then(|slot| slot.settled.as_ref())
    }
}

/// What owns a runtime: the app its engines run in.
pub trait Host: Sized + 'static {
    fn runtime(&mut self) -> &mut Runtime<Self>;

    fn now(&mut self) -> i64 {
        self.runtime().now
    }

    /// Runs `task` after `ms` milliseconds, as `setTimeout`; its id clears it.
    fn set_timeout(&mut self, ms: i64, task: impl FnOnce(&mut Self) + 'static) -> u64 {
        let runtime = self.runtime();
        runtime.seq += 1;
        let seq = runtime.seq;
        let at = runtime.now + ms.max(0);
        runtime.timers.push(Timer {
            at,
            seq,
            every: None,
            job: Job::Once(Box::new(task)),
        });
        seq
    }

    /// Runs `task` every `ms` milliseconds, as `setInterval`; its id clears it.
    fn set_interval(&mut self, ms: i64, task: impl Fn(&mut Self) + 'static) -> u64 {
        let runtime = self.runtime();
        runtime.seq += 1;
        let seq = runtime.seq;
        let at = runtime.now + ms;
        runtime.timers.push(Timer {
            at,
            seq,
            every: Some(ms),
            job: Job::Every(Rc::new(task)),
        });
        seq
    }

    fn clear_timer(&mut self, id: u64) {
        self.runtime().timers.retain(|timer| timer.seq != id);
    }

    /// Queues a microtask.
    fn queue(&mut self, task: impl FnOnce(&mut Self) + 'static) {
        self.runtime().micro.push_back(Box::new(task));
    }

    /// Runs every microtask queued, and those they queue.
    fn drain(&mut self) {
        while let Some(task) = self.runtime().micro.pop_front() {
            task(self);
        }
    }

    /// Fires every timer due by `to`, each at its time and in the order they were set, the
    /// microtasks drained after each; the clock then stands at `to`.
    fn advance(&mut self, to: i64) {
        self.drain();
        loop {
            let runtime = self.runtime();
            let next = runtime
                .timers
                .iter()
                .enumerate()
                .filter(|(_, timer)| timer.at <= to)
                .min_by_key(|(_, timer)| (timer.at, timer.seq))
                .map(|(index, _)| index);
            let Some(index) = next else { break };
            runtime.now = runtime.timers[index].at;
            let task: Task<Self> = match runtime.timers[index].every {
                None => match runtime.timers.remove(index).job {
                    Job::Once(task) => task,
                    Job::Every(task) => Box::new(move |host: &mut Self| task(host)),
                },
                Some(every) => {
                    runtime.timers[index].at += every;
                    match &runtime.timers[index].job {
                        Job::Every(task) => {
                            let task = task.clone();
                            Box::new(move |host: &mut Self| task(host))
                        }
                        Job::Once(_) => Box::new(|_: &mut Self| {}),
                    }
                }
            };
            task(self);
            self.drain();
        }
        let runtime = self.runtime();
        runtime.now = runtime.now.max(to);
        self.drain();
    }

    /// A promise not yet settled.
    fn pending(&mut self) -> Promise {
        let runtime = self.runtime();
        runtime.promises.push(Slot {
            settled: None,
            reactions: Vec::new(),
        });
        Promise(runtime.promises.len() - 1)
    }

    /// A promise fulfilled with nothing, as `Promise.resolve()`.
    fn resolved(&mut self) -> Promise {
        let promise = self.pending();
        self.settle(promise, Ok(unit()));
        promise
    }

    /// A promise rejected with `message`, as `Promise.reject(new Error(message))`.
    fn rejected(&mut self, message: impl Into<String>) -> Promise {
        let promise = self.pending();
        self.settle(promise, Err(message.into()));
        promise
    }

    /// Fulfills or rejects a promise; one already settled stays as it is.
    fn settle(&mut self, promise: Promise, outcome: Outcome) {
        let slot = &mut self.runtime().promises[promise.0];
        if slot.settled.is_some() {
            return;
        }
        slot.settled = Some(outcome.clone());
        let reactions = std::mem::take(&mut slot.reactions);
        for reaction in reactions {
            let outcome = outcome.clone();
            self.queue(move |host| reaction(host, outcome));
        }
    }

    /// Fulfills or rejects a promise as `other` settles, as resolving one promise with another
    /// does: two microtasks after `other` settles.
    fn adopt(&mut self, promise: Promise, other: Promise) {
        self.queue(move |host| host.on(other, move |host, outcome| host.settle(promise, outcome)));
    }

    /// Runs `reaction` one microtask after `promise` settles, as `await` and a `then` do.
    fn on(&mut self, promise: Promise, reaction: impl FnOnce(&mut Self, Outcome) + 'static) {
        let slot = &mut self.runtime().promises[promise.0];
        match &slot.settled {
            Some(outcome) => {
                let outcome = outcome.clone();
                self.queue(move |host| reaction(host, outcome));
            }
            None => slot.reactions.push(Box::new(reaction)),
        }
    }

    /// `promise.then(handler)`, with its rejection handled there too: the promise it gives.
    fn then(&mut self, promise: Promise, handler: impl FnOnce(&mut Self, Outcome) -> Handled + 'static) -> Promise {
        let derived = self.pending();
        self.on(promise, move |host, outcome| match handler(host, outcome) {
            Handled::Done(outcome) => host.settle(derived, outcome),
            Handled::Follow(other) => host.adopt(derived, other),
        });
        derived
    }

    /// `promise.catch(() => undefined)`: fulfilled either way.
    fn caught(&mut self, promise: Promise) -> Promise {
        self.then(promise, |_, outcome| {
            Handled::Done(Ok(outcome.unwrap_or_else(|_| unit())))
        })
    }

    /// `promise.finally(step)`: the step once it settles, and a promise settled as it was, as the
    /// web chains a `finally`.
    fn finally(&mut self, promise: Promise, step: impl FnOnce(&mut Self) + 'static) -> Promise {
        self.then(promise, move |host, outcome| {
            step(host);
            let done = host.resolved();
            let after = host.then(done, move |_, _| Handled::Done(outcome));
            Handled::Follow(after)
        })
    }

    /// Settles `promise` after `ms` milliseconds, as `new Promise((done) => setTimeout(done, ms))`.
    fn sleep(&mut self, ms: i64) -> Promise {
        let promise = self.pending();
        self.set_timeout(ms, move |host| host.settle(promise, Ok(unit())));
        promise
    }
}
