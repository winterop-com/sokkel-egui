//! The fake backend the web examples share (`tidevann-shared/backend`, sokkel-ui's
//! `examples/shared/backend/store.ts`), on a held clock instead of the browser's timers.
//!
//! Each `Collection` is the web's `collection`: a change is made and kept at once, its answer comes
//! `LATENCY` milliseconds later, and every change is told to those listening that much later with
//! the rows as they were when it was made. A page reads a collection's rows as they were last told
//! or listed (`Collection::seen`, the web's `useCollection`), none until the first answer, so what
//! a page shows lags a change by the same moment the web's does. The `Clock` keeps the web's timers
//! as the web sets them, each with what the example does when it fires, and an example moves it
//! with `Clock::take_due`. Where the changes stand (`Saving`) follows the web's `savingNow`.

/// How long each call to the backend takes, in milliseconds.
pub const LATENCY: i64 = 150;

/// A row of a collection, kept under its id.
pub trait Row: Clone {
    fn id(&self) -> &str;
}

/// The name a collection is kept under, as the web's `collection(name, seed)` names it: the words
/// a refusal says (`No <key> with the id ...`) and the ids the backend makes.
pub trait Name: Copy {
    fn key(self) -> &'static str;
}

/// A timer set on the clock: when it fires, the order it was set in, how often it fires again if it
/// is an interval, and what the example does when it fires.
#[derive(Clone, Debug)]
pub struct Timer<J> {
    pub at: i64,
    pub seq: u64,
    pub every: Option<i64>,
    pub job: J,
}

/// The web's clock and its timers, `setTimeout` and `setInterval` held: what an example does when a
/// timer fires is its own `J`.
#[derive(Clone, Debug)]
pub struct Clock<J> {
    pub now: i64,
    seq: u64,
    timers: Vec<Timer<J>>,
}

impl<J> Clock<J> {
    /// The clock at `now`, no timer set.
    pub fn new(now: i64) -> Self {
        Clock {
            now,
            seq: 0,
            timers: Vec::new(),
        }
    }

    /// Sets a timer `ms` from now, as `setTimeout` does; its seq, which orders it among those due
    /// at the same time.
    pub fn after(&mut self, ms: i64, job: J) -> u64 {
        self.set(ms, None, job)
    }

    /// Sets a timer that fires every `ms` from now on, as `setInterval` does; its seq.
    pub fn every(&mut self, ms: i64, job: J) -> u64 {
        self.set(ms, Some(ms), job)
    }

    fn set(&mut self, ms: i64, every: Option<i64>, job: J) -> u64 {
        self.seq += 1;
        self.timers.push(Timer {
            at: self.now + ms,
            seq: self.seq,
            every,
            job,
        });
        self.seq
    }

    /// Takes a timer away, as `clearTimeout` and `clearInterval` do.
    pub fn clear(&mut self, seq: u64) {
        self.timers.retain(|timer| timer.seq != seq);
    }

    /// The time of the next timer due, if any.
    pub fn next_due(&self) -> Option<i64> {
        self.timers.iter().map(|timer| timer.at).min()
    }

    /// The clock moved on to `to` once every timer due by then has fired; never back.
    pub fn reach(&mut self, to: i64) {
        self.now = self.now.max(to);
    }
}

impl<J: Clone> Clock<J> {
    /// The next timer due by `to`, the earliest first and those due together in the order they
    /// were set: the clock moved to its time, an interval set again, and its seq and job; none when
    /// none is due.
    pub fn take_due(&mut self, to: i64) -> Option<(u64, J)> {
        let index = self
            .timers
            .iter()
            .enumerate()
            .filter(|(_, timer)| timer.at <= to)
            .min_by_key(|(_, timer)| (timer.at, timer.seq))
            .map(|(index, _)| index)?;
        let timer = &mut self.timers[index];
        self.now = timer.at;
        match timer.every {
            Some(every) => {
                timer.at += every;
                Some((timer.seq, timer.job.clone()))
            }
            None => {
                let timer = self.timers.remove(index);
                Some((timer.seq, timer.job))
            }
        }
    }
}

/// One collection: its rows as the backend keeps them, and as the pages last heard them.
#[derive(Clone, Debug)]
pub struct Collection<T, N> {
    pub name: N,
    /// The rows as kept, every change made at once.
    pub rows: Vec<T>,
    /// The rows as last told or listed, none until the first answer.
    pub seen: Option<Vec<T>>,
    /// Whether a change was kept since the seed, as the browser keeps the rows once one is.
    pub written: bool,
    /// The rows each change or list will tell, under its timer's seq.
    pub pending: Vec<(u64, Vec<T>)>,
}

impl<T: Row, N: Name> Collection<T, N> {
    pub fn new(name: N, rows: Vec<T>) -> Self {
        Collection {
            name,
            rows,
            seen: None,
            written: false,
            pending: Vec::new(),
        }
    }

    /// The rows a page shows: none until the first answer.
    pub fn seen(&self) -> &[T] {
        self.seen.as_deref().unwrap_or_default()
    }

    /// Whether a page has heard the rows yet.
    pub fn loaded(&self) -> bool {
        self.seen.is_some()
    }

    /// The row under `id` a page shows.
    pub fn seen_get(&self, id: &str) -> Option<&T> {
        self.seen().iter().find(|row| row.id() == id)
    }

    /// The row under `id` as kept.
    pub fn get(&self, id: &str) -> Option<&T> {
        self.rows.iter().find(|row| row.id() == id)
    }

    /// The rows the timer under `seq` tells, taken.
    pub fn take(&mut self, seq: u64) -> Option<Vec<T>> {
        let at = self.pending.iter().position(|(one, _)| *one == seq)?;
        Some(self.pending.remove(at).1)
    }

    /// The rows the timer under `seq` tells, heard by the pages.
    pub fn heard(&mut self, seq: u64) {
        if let Some(rows) = self.take(seq) {
            self.seen = Some(rows);
        }
    }

    /// The rows as kept now told `LATENCY` from now, by the timer `job` makes of the collection's
    /// name: a list's answer, or a change told to those listening.
    pub fn tell<J>(&mut self, clock: &mut Clock<J>, job: impl FnOnce(N) -> J) {
        let seq = clock.after(LATENCY, job(self.name));
        self.pending.push((seq, self.rows.clone()));
    }

    /// Keeps the rows a change makes and tells them (`tell`).
    pub fn save<J>(&mut self, clock: &mut Clock<J>, next: Vec<T>, job: impl FnOnce(N) -> J) {
        self.rows = next;
        self.written = true;
        self.tell(clock, job);
    }

    /// Adds a row at the front, as the backend's `create` does.
    pub fn prepend<J>(&mut self, clock: &mut Clock<J>, row: T, job: impl FnOnce(N) -> J) {
        let mut next = vec![row];
        next.extend(self.rows.iter().cloned());
        self.save(clock, next, job);
    }

    /// Changes the row under `id`, as the backend's `update` does: there being none is refused
    /// with the web's words, and nothing is kept.
    pub fn update<J>(
        &mut self,
        clock: &mut Clock<J>,
        id: &str,
        change: impl FnOnce(&mut T),
        job: impl FnOnce(N) -> J,
    ) -> Result<(), String> {
        let Some(mut row) = self.get(id).cloned() else {
            return Err(format!("No {} with the id {id}.", self.name.key()));
        };
        change(&mut row);
        let next = self
            .rows
            .iter()
            .map(|one| if one.id() == id { row.clone() } else { one.clone() })
            .collect();
        self.save(clock, next, job);
        Ok(())
    }

    /// Takes the row under `id` out, as the backend's `remove` does.
    pub fn remove<J>(&mut self, clock: &mut Clock<J>, id: &str, job: impl FnOnce(N) -> J) {
        let next = self.rows.iter().filter(|row| row.id() != id).cloned().collect();
        self.save(clock, next, job);
    }
}

/// Where the example's changes stand: some still on their way, the last one refused, or all kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Saving {
    Saving,
    Failed(String),
    Saved,
}

impl Saving {
    /// A change made: followed until it is answered, as the web's `follow` follows it; `pending`
    /// counts those on their way.
    pub fn follow(&mut self, pending: &mut usize) {
        *pending += 1;
        if *self != Saving::Saving {
            *self = Saving::Saving;
        }
    }

    /// A change answered: refused with its reason, or kept, and all saved once none is on its way.
    pub fn settle(&mut self, pending: &mut usize, failure: Option<&str>) {
        *pending -= 1;
        match failure {
            Some(reason) => *self = Saving::Failed(reason.to_owned()),
            None => {
                if *pending == 0 && *self == Saving::Saving {
                    *self = Saving::Saved;
                }
            }
        }
    }
}

/// The words a status line says for where the changes stand, its tone and whether it pulses, as
/// the web's `savingNote` gives them.
pub fn saving_note(saving: &Saving) -> (String, &'static str, bool) {
    match saving {
        Saving::Saving => ("Saving".to_owned(), "quiet", true),
        Saving::Failed(reason) => (format!("Not saved: {reason}"), "error", false),
        Saving::Saved => ("All changes saved".to_owned(), "quiet", false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Item {
        id: String,
    }

    impl Row for Item {
        fn id(&self) -> &str {
            &self.id
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Items;

    impl Name for Items {
        fn key(self) -> &'static str {
            "items"
        }
    }

    fn item(id: &str) -> Item {
        Item { id: id.to_owned() }
    }

    #[test]
    fn a_change_is_kept_at_once_and_heard_later() {
        let mut clock: Clock<Items> = Clock::new(1000);
        let mut items = Collection::new(Items, vec![item("a")]);
        items.tell(&mut clock, |name| name);
        items.prepend(&mut clock, item("b"), |name| name);
        assert_eq!(items.rows.len(), 2);
        assert!(!items.loaded());
        assert_eq!(clock.next_due(), Some(1000 + LATENCY));
        while let Some((seq, _)) = clock.take_due(1000 + LATENCY) {
            items.heard(seq);
        }
        clock.reach(2000);
        assert_eq!(clock.now, 2000);
        assert_eq!(items.seen().len(), 2);
        assert_eq!(
            items.update(&mut clock, "c", |_| {}, |name| name),
            Err("No items with the id c.".to_owned())
        );
    }

    #[test]
    fn an_interval_fires_again() {
        let mut clock = Clock::new(0);
        let tick = clock.every(100, "tick");
        clock.after(150, "once");
        let fired: Vec<_> = std::iter::from_fn(|| clock.take_due(300)).map(|(_, job)| job).collect();
        assert_eq!(fired, ["tick", "once", "tick", "tick"]);
        clock.clear(tick);
        assert_eq!(clock.next_due(), None);
    }

    #[test]
    fn saving_follows_the_changes() {
        let (mut saving, mut pending) = (Saving::Saved, 0);
        saving.follow(&mut pending);
        saving.follow(&mut pending);
        saving.settle(&mut pending, None);
        assert_eq!(saving, Saving::Saving);
        saving.settle(&mut pending, None);
        assert_eq!(saving, Saving::Saved);
        assert_eq!(saving_note(&saving).0, "All changes saved");
    }
}
