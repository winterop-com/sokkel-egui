//! Undo and redo over an edit, as the web's `history.ts` keeps them: each change keeps the edit
//! before it, and changes made in quick turn under one `merge` key (the keys typed into one field)
//! are one step.

use crate::model::Edit;

/// How many steps back undo reaches.
pub const DEPTH: usize = 200;

/// How long after a change another under the same key joins it, in milliseconds.
pub const JOIN: i64 = 1200;

#[derive(Clone, Debug)]
pub struct EditHistory {
    past: Vec<Edit>,
    present: Edit,
    future: Vec<Edit>,
    /// The key and moment of the last change made under one.
    last: Option<(String, i64)>,
}

impl EditHistory {
    pub fn new(initial: Edit) -> Self {
        Self {
            past: Vec::new(),
            present: initial,
            future: Vec::new(),
            last: None,
        }
    }

    /// The edit as it stands.
    pub fn edit(&self) -> &Edit {
        &self.present
    }

    /// Makes a change at `now`: `next`, or nothing when it is none. A change under the same
    /// `merge` key as the last, within `JOIN`, joins its step.
    pub fn apply(&mut self, next: Option<Edit>, merge: Option<&str>, now: i64) {
        let joins = match (merge, &self.last) {
            (Some(key), Some((last, at))) => last == key && now - at < JOIN,
            _ => false,
        };
        self.last = merge.map(|key| (key.to_owned(), now));
        let Some(next) = next else {
            return;
        };
        if !joins {
            let previous = std::mem::replace(&mut self.present, next);
            self.past.push(previous);
            if self.past.len() > DEPTH {
                let over = self.past.len() - DEPTH;
                self.past.drain(..over);
            }
        } else {
            self.present = next;
        }
        self.future.clear();
    }

    /// Makes the change `change` makes of the edit as it stands, as `apply` does.
    pub fn apply_with(&mut self, change: impl FnOnce(&Edit) -> Option<Edit>, merge: Option<&str>, now: i64) {
        let next = change(&self.present);
        self.apply(next, merge, now);
    }

    pub fn undo(&mut self) {
        self.last = None;
        if let Some(previous) = self.past.pop() {
            let present = std::mem::replace(&mut self.present, previous);
            self.future.insert(0, present);
        }
    }

    pub fn redo(&mut self) {
        self.last = None;
        if !self.future.is_empty() {
            let next = self.future.remove(0);
            let present = std::mem::replace(&mut self.present, next);
            self.past.push(present);
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }
}
