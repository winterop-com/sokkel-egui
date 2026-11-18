//! Undo and redo for what an editor works on, after sokkel-ui-graph's `use-graph-history`.

use egui::{Context, Key, Modifiers};

/// What a key press asks of a history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryKey {
    Undo,
    Redo,
}

/// What a key press asks of a history: Cmd or Ctrl with Z undoes, with Shift and Z, or Ctrl with Y,
/// redoes; `None` for anything else.
pub fn history_key(key: Key, modifiers: Modifiers) -> Option<HistoryKey> {
    if !(modifiers.mac_cmd || modifiers.command || modifiers.ctrl) || modifiers.alt {
        return None;
    }
    match key {
        Key::Z if modifiers.shift => Some(HistoryKey::Redo),
        Key::Z => Some(HistoryKey::Undo),
        Key::Y if modifiers.ctrl && !modifiers.shift => Some(HistoryKey::Redo),
        _ => None,
    }
}

/// The undo and redo keys for a history kept anywhere, such as the text a graph is drawn from:
/// the press this frame asks for, taken so nothing else answers it, or `None`. Off while `enabled`
/// is false, and while a field has the keyboard, since a field keeps its own undo.
pub fn history_keys(ctx: &Context, enabled: bool) -> Option<HistoryKey> {
    if !enabled || ctx.text_edit_focused() {
        return None;
    }
    ctx.input_mut(|input| {
        let mut asked = None;
        input.events.retain(|event| match event {
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } if asked.is_none() => match history_key(*key, *modifiers) {
                Some(which) => {
                    asked = Some(which);
                    false
                }
                None => true,
            },
            _ => true,
        });
        asked
    })
}

/// A value with what it was before and what an undo took back.
#[derive(Clone, Debug)]
pub struct GraphHistory<T> {
    past: Vec<T>,
    present: T,
    future: Vec<T>,
    /// The group the last change belongs to; a change in the same group joins it rather than
    /// adding a step.
    group: Option<String>,
    revision: u64,
    limit: usize,
}

impl<T: Clone + PartialEq> GraphHistory<T> {
    /// A history of `value`, keeping at most 100 steps to undo.
    pub fn new(value: T) -> Self {
        Self {
            past: Vec::new(),
            present: value,
            future: Vec::new(),
            group: None,
            revision: 0,
            limit: 100,
        }
    }

    /// Earlier values to start with, oldest first, as when a document is reopened with its history.
    pub fn with_past(mut self, past: Vec<T>) -> Self {
        self.past = past;
        self
    }

    /// The most steps kept to undo.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }

    pub fn value(&self) -> &T {
        &self.present
    }

    /// Changes the value as one step to undo. A change that leaves the value as it was adds
    /// nothing. Every change given the same `merge` key in a row, such as the keystrokes typed into
    /// one field, is one step; give the frame's number to make every change of one frame one step,
    /// as the web's history makes every change of one task one.
    pub fn set(&mut self, value: T, merge: Option<&str>) {
        if value == self.present {
            return;
        }
        let group = merge.map(str::to_owned);
        if group.is_some() && group == self.group {
            self.present = value;
            self.future.clear();
            return;
        }
        self.past.push(std::mem::replace(&mut self.present, value));
        if self.past.len() > self.limit {
            let over = self.past.len() - self.limit;
            self.past.drain(..over);
        }
        self.future.clear();
        self.group = group;
    }

    /// Changes the value in place as one step to undo, as `set` does.
    pub fn update(&mut self, merge: Option<&str>, change: impl FnOnce(&mut T)) {
        let mut next = self.present.clone();
        change(&mut next);
        self.set(next, merge);
    }

    /// Takes back the last step; false when there was none.
    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.past.pop() else {
            return false;
        };
        self.future.insert(0, std::mem::replace(&mut self.present, previous));
        self.group = None;
        self.revision += 1;
        true
    }

    /// Brings back the step an undo took; false when there was none.
    pub fn redo(&mut self) -> bool {
        if self.future.is_empty() {
            return false;
        }
        let next = self.future.remove(0);
        self.past.push(std::mem::replace(&mut self.present, next));
        self.group = None;
        self.revision += 1;
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// Starts over from `value`, with nothing to undo.
    pub fn reset(&mut self, value: T) {
        self.past.clear();
        self.future.clear();
        self.present = value;
        self.group = None;
        self.revision += 1;
    }

    /// Changes on every undo, redo and reset: a form keyed on it shows the value they brought back.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Answers the undo and redo keys pressed this frame, as `history_keys` reads them; the key
    /// answered, if one was.
    pub fn keys(&mut self, ctx: &Context, enabled: bool) -> Option<HistoryKey> {
        let asked = history_keys(ctx, enabled)?;
        match asked {
            HistoryKey::Undo => self.undo(),
            HistoryKey::Redo => self.redo(),
        };
        Some(asked)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_undoes_and_redoes() {
        let mut history = GraphHistory::new(1);
        history.set(2, None);
        history.set(3, None);
        assert!(history.undo());
        assert_eq!(*history.value(), 2);
        assert!(history.redo());
        assert_eq!(*history.value(), 3);
        assert!(!history.redo());
        assert_eq!(history.revision(), 2);
    }

    #[test]
    fn changes_merged_under_one_key_are_one_step() {
        let mut history = GraphHistory::new(String::new());
        for typed in ["n", "ni", "nig"] {
            history.set(typed.to_owned(), Some("name"));
        }
        history.undo();
        assert_eq!(history.value(), "");
    }

    #[test]
    fn a_change_to_the_same_value_adds_nothing() {
        let mut history = GraphHistory::new(1);
        history.set(1, None);
        assert!(!history.can_undo());
    }

    #[test]
    fn the_keys_read_as_the_web_reads_them() {
        let cmd = Modifiers::COMMAND;
        assert_eq!(history_key(Key::Z, cmd), Some(HistoryKey::Undo));
        assert_eq!(history_key(Key::Z, cmd | Modifiers::SHIFT), Some(HistoryKey::Redo));
        assert_eq!(history_key(Key::Y, Modifiers::CTRL), Some(HistoryKey::Redo));
        assert_eq!(history_key(Key::Z, Modifiers::NONE), None);
        assert_eq!(history_key(Key::Z, cmd | Modifiers::ALT), None);
    }
}
