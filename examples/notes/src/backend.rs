//! The vault on a simulated backend, as the web's `backend.ts` keeps it in the shared collection
//! store and `state.tsx` changes it: its folders, its notes and the reader's settings, the drafts
//! being typed, and every change a note or a folder takes.
//!
//! Every call is at once and takes the moment it is made at. Rows are kept as the store keeps them:
//! a new one first, a changed one in its place, and a new row's id `<collection>-<now in base 36>-
//! <count>`, the count running over both collections. A draft is saved `SAVE_AFTER` after its last
//! edit, when `tick` comes to it, or at once by `flush`.

use crate::model::{Folder, Note, Order, Prefs, RECENT, View, bad_title, base36, free_name, from_template, trim};
use crate::seed::{seed_folders, seed_notes};
use crate::vault::{Vault, folder_and_below, folders_in, index_vault, notes_in, notes_in_order, rename_links};

/// How long typing stops before a draft is saved, in milliseconds.
pub const SAVE_AFTER: i64 = 700;

/// Where the latest edit stands: nothing saved yet, saved at a time, or refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveState {
    Idle,
    Saved { at: i64 },
    Failed { reason: String },
}

/// What a change says to the reader as a toast: its title, the line under it, and whether it reads
/// as a success.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub title: String,
    pub description: Option<String>,
    pub success: bool,
}

impl Notice {
    fn new(title: impl Into<String>, description: Option<String>, success: bool) -> Self {
        Self {
            title: title.into(),
            description,
            success,
        }
    }
}

/// What closing a tab did to the note shown: nothing, or it was the shown one and the next tab
/// (none when no tab is left) takes its place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TabClosed {
    Inactive,
    Active { next: Option<String> },
}

/// Where following a link's name leads: to its note, or to asking to make it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Follow {
    Open { id: String },
    Create { title: String, folder: Option<String> },
}

/// A question the app is asking in a dialog, about a note or a folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asking {
    NewFolder { parent: Option<String> },
    RenameNote { id: String },
    RenameFolder { id: String },
    MoveNote { id: String },
    MoveFolder { id: String },
    DeleteNote { id: String },
    DeleteFolder { id: String },
    Create { title: String, folder: Option<String> },
}

/// What `place` puts somewhere: a note or a folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placing {
    Note,
    Folder,
}

/// What a new note is made from; every field may be left out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NewNote {
    pub title: Option<String>,
    pub folder: Option<String>,
    pub body: Option<String>,
    pub day: Option<String>,
}

/// A note's text typed and not saved yet, and when it is due to be.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Draft {
    id: String,
    text: String,
    due: i64,
}

/// The rank after every one of a level's notes or folders.
fn rank_after(level: impl Iterator<Item = Option<i64>>) -> i64 {
    level.fold(-1, |most, rank| most.max(rank.unwrap_or(-1))) + 1
}

/// The rank before every one of a level's notes or folders.
fn rank_before(level: impl Iterator<Item = Option<i64>>) -> i64 {
    level.fold(0, |least, rank| least.min(rank.unwrap_or(0))) - 1
}

/// Where a note or a folder went, as its toast says it.
fn moved_to(vault: &Vault, into: Option<&str>) -> String {
    match into {
        None => "Moved to the top of the vault".to_owned(),
        Some(folder) => format!("Moved to {}", vault.path(Some(folder))),
    }
}

/// The first line of a text that is exactly `# <title>`, as `^# <title>$` finds it with `m`.
fn heading_line(text: &str, title: &str) -> Option<std::ops::Range<usize>> {
    let wanted = format!("# {title}");
    let ends = |c: char| matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}');
    let starts = std::iter::once(0).chain(
        text.char_indices()
            .filter(|(_, c)| ends(*c))
            .map(|(at, c)| at + c.len_utf8()),
    );
    starts
        .filter(|start| text[*start..].starts_with(&wanted))
        .map(|start| start..start + wanted.len())
        .find(|range| text[range.end..].chars().next().is_none_or(ends))
}

/// The vault's data and the drafts typed into it.
#[derive(Clone, Debug)]
pub struct Backend {
    pub folders: Vec<Folder>,
    /// The notes as saved, without the drafts.
    pub notes: Vec<Note>,
    /// The reader's settings; the web keeps them as a collection of one row.
    pub prefs: Prefs,
    /// Where the latest save stands.
    pub save: SaveState,
    drafts: Vec<Draft>,
    /// How many rows were made, for the next one's id.
    made: u64,
    vault: Vault,
}

impl Backend {
    /// The vault as a new reader starts with it at `now`, each folder and note ranked as it sorts by
    /// name, so the reader's own order starts from that; two notes open as tabs.
    pub fn seeded(now: i64) -> Self {
        let vault = index_vault(seed_notes(now), seed_folders());
        let levels = std::iter::once(None).chain(vault.folders.iter().map(|folder| Some(folder.id.as_str())));
        let folders: Vec<Folder> = levels
            .flat_map(|parent| {
                folders_in(&vault, parent, Order::Name)
                    .into_iter()
                    .enumerate()
                    .map(|(rank, folder)| Folder {
                        rank: Some(rank as i64),
                        ..folder.clone()
                    })
            })
            .collect();
        let notes: Vec<Note> = notes_in_order(&vault, None, Order::Name, Some("daily"))
            .into_iter()
            .enumerate()
            .map(|(rank, note)| Note {
                rank: Some(rank as i64),
                ..note.clone()
            })
            .collect();
        let prefs = Prefs {
            tabs: vec!["retry".to_owned(), "nightly-export".to_owned()],
            active: Some("retry".to_owned()),
            recent: vec![
                "retry".to_owned(),
                "nightly-export".to_owned(),
                "weekly-sync".to_owned(),
            ],
            ..Prefs::default()
        };
        Self::kept(folders, notes, prefs)
    }

    /// The vault as it was kept.
    pub fn kept(folders: Vec<Folder>, notes: Vec<Note>, prefs: Prefs) -> Self {
        let mut backend = Self {
            folders,
            notes,
            prefs,
            save: SaveState::Idle,
            drafts: Vec::new(),
            made: 0,
            vault: Vault::default(),
        };
        backend.reindex();
        backend
    }

    /// The vault as it reads now, every draft in its note's place.
    pub fn vault(&self) -> &Vault {
        &self.vault
    }

    /// How the vault orders each folder.
    pub fn order(&self) -> Order {
        self.prefs.order.unwrap_or(Order::Manual)
    }

    /// A note's text not saved yet, if it has one.
    pub fn draft(&self, id: &str) -> Option<&str> {
        self.drafts
            .iter()
            .find(|draft| draft.id == id)
            .map(|draft| draft.text.as_str())
    }

    /// The ids of the notes with a draft, in the order they were first edited.
    pub fn drafted(&self) -> Vec<&str> {
        self.drafts.iter().map(|draft| draft.id.as_str()).collect()
    }

    /// A note's text as it reads now: its draft, or what is saved.
    pub fn body_of(&self, id: &str) -> String {
        self.draft(id)
            .map(str::to_owned)
            .or_else(|| self.vault.note(id).map(|note| note.body.clone()))
            .unwrap_or_default()
    }

    fn reindex(&mut self) {
        let live: Vec<Note> = self
            .notes
            .iter()
            .map(|note| match self.drafts.iter().find(|draft| draft.id == note.id) {
                Some(draft) => Note {
                    body: draft.text.clone(),
                    ..note.clone()
                },
                None => note.clone(),
            })
            .collect();
        self.vault = index_vault(live, self.folders.clone());
    }

    /// The next id the store gives a row of `collection` made at `now`.
    fn next_id(&mut self, collection: &str, now: i64) -> String {
        self.made += 1;
        format!("{collection}-{}-{}", base36(now), self.made)
    }

    fn update_note(&mut self, id: &str, change: impl FnOnce(&mut Note)) -> Result<(), String> {
        let note = self
            .notes
            .iter_mut()
            .find(|note| note.id == id)
            .ok_or_else(|| format!("No notes with the id {id}."))?;
        change(note);
        Ok(())
    }

    fn update_folder(&mut self, id: &str, change: impl FnOnce(&mut Folder)) -> Result<(), String> {
        let folder = self
            .folders
            .iter_mut()
            .find(|folder| folder.id == id)
            .ok_or_else(|| format!("No folders with the id {id}."))?;
        change(folder);
        Ok(())
    }

    /// Changes the reader's settings.
    pub fn set_prefs(&mut self, change: impl FnOnce(&mut Prefs)) {
        change(&mut self.prefs);
    }

    /// Puts a note's new text in, saved `SAVE_AFTER` after typing stops.
    pub fn edit(&mut self, id: &str, text: &str, now: i64) {
        let due = now + SAVE_AFTER;
        match self.drafts.iter_mut().find(|draft| draft.id == id) {
            Some(draft) => {
                draft.text = text.to_owned();
                draft.due = due;
            }
            None => self.drafts.push(Draft {
                id: id.to_owned(),
                text: text.to_owned(),
                due,
            }),
        }
        self.reindex();
    }

    /// Saves the drafts come due by `now`; whether one was.
    pub fn tick(&mut self, now: i64) -> bool {
        let due: Vec<String> = self
            .drafts
            .iter()
            .filter(|draft| draft.due <= now)
            .map(|draft| draft.id.clone())
            .collect();
        for id in &due {
            self.flush(Some(id), now);
        }
        !due.is_empty()
    }

    /// Saves a note's draft now, or every draft.
    pub fn flush(&mut self, only: Option<&str>, now: i64) {
        let drafts = self.drafts.clone();
        for draft in drafts.iter().filter(|draft| only.is_none_or(|only| only == draft.id)) {
            let text = draft.text.clone();
            match self.update_note(&draft.id, |note| {
                note.body = text;
                note.updated = now;
            }) {
                Ok(()) => {
                    self.save = SaveState::Saved { at: now };
                    self.drafts.retain(|one| one.id != draft.id);
                }
                Err(reason) => self.save = SaveState::Failed { reason },
            }
        }
        self.reindex();
    }

    /// Rewrites one note's text, as linking a mention does: its draft dropped, the change saved.
    pub fn rewrite(&mut self, id: &str, change: impl FnOnce(&str) -> String, now: i64) -> Result<(), String> {
        let text = change(&self.body_of(id));
        self.drafts.retain(|draft| draft.id != id);
        let saved = self.update_note(id, |note| {
            note.body = text;
            note.updated = now;
        });
        self.reindex();
        saved
    }

    /// Makes a note: named `Untitled` unless given a name, a number after a name taken; last in its
    /// folder in the reader's own order, or first for a daily note, as the newest day.
    pub fn create_note(&mut self, fields: NewNote, now: i64) -> Note {
        let taken: Vec<&str> = self.vault.notes.iter().map(|note| note.title.as_str()).collect();
        let asked = fields.title.as_deref().map(trim).filter(|title| !title.is_empty());
        let title = free_name(asked.unwrap_or("Untitled"), &taken);
        let level = self
            .vault
            .notes
            .iter()
            .filter(|note| note.folder == fields.folder)
            .map(|note| note.rank);
        let rank = if fields.day.is_none() {
            rank_after(level)
        } else {
            rank_before(level)
        };
        let note = Note {
            id: self.next_id("notes", now),
            body: fields.body.unwrap_or_else(|| format!("# {title}\n\n")),
            title,
            folder: fields.folder,
            created: now,
            updated: now,
            day: fields.day,
            rank: Some(rank),
        };
        self.notes.insert(0, note.clone());
        self.reindex();
        note
    }

    /// Renames a note and every link to it, with a first heading that repeats the old name; how
    /// many notes linked to it. Refused for a name a note cannot have.
    pub fn rename_note(&mut self, id: &str, title: &str, now: i64) -> Result<usize, String> {
        let note = self.vault.note(id).ok_or("That note is gone.")?.clone();
        let name = trim(title).to_owned();
        if name == note.title {
            return Ok(0);
        }
        let others: Vec<&str> = self
            .vault
            .notes
            .iter()
            .filter(|one| one.id != id)
            .map(|one| one.title.as_str())
            .collect();
        if let Some(refused) = bad_title(&name, &others) {
            return Err(refused);
        }
        let linking: Vec<String> = self.vault.backlinks(id).iter().map(|link| link.from.clone()).collect();
        let mut own = self.body_of(id);
        self.flush(None, now);
        // A first heading that repeats the old name takes the new one.
        if let Some(range) = heading_line(&own, &note.title) {
            own.replace_range(range, &format!("# {name}"));
        }
        self.update_note(id, |note| {
            note.title = name.clone();
            note.body = own;
            note.updated = now;
        })?;
        self.reindex();
        for from in &linking {
            self.rewrite(from, |text| rename_links(text, &note.title, &name), now)?;
        }
        Ok(linking.len())
    }

    /// Moves a note into a folder (none for the top of the vault), last in the reader's own order.
    pub fn move_note(&mut self, id: &str, folder: Option<&str>) -> Result<Notice, String> {
        let rank = rank_after(
            self.vault
                .notes
                .iter()
                .filter(|note| note.folder.as_deref() == folder && note.id != id)
                .map(|note| note.rank),
        );
        self.update_note(id, |note| {
            note.folder = folder.map(str::to_owned);
            note.rank = Some(rank);
        })?;
        self.reindex();
        let title = self.vault.note(id).map(|note| note.title.clone());
        Ok(Notice::new(moved_to(&self.vault, folder), title, true))
    }

    /// Deletes a note: its draft, its tab and its place among the recent ones.
    pub fn delete_note(&mut self, id: &str, now: i64) -> (Notice, TabClosed) {
        let title = self.vault.note(id).map(|note| note.title.clone());
        self.drafts.retain(|draft| draft.id != id);
        self.notes.retain(|note| note.id != id);
        self.reindex();
        let closed = if self.prefs.tabs.iter().any(|tab| tab == id) {
            self.close_tab(id, now)
        } else {
            TabClosed::Inactive
        };
        self.prefs.recent.retain(|one| one != id);
        (Notice::new("Note deleted", title, false), closed)
    }

    /// Makes a folder, `New folder` unless given a name, a number after a name taken beside it;
    /// last among them in the reader's own order.
    pub fn create_folder(&mut self, name: &str, parent: Option<&str>, now: i64) -> Folder {
        let level: Vec<&Folder> = self
            .folders
            .iter()
            .filter(|folder| folder.parent.as_deref() == parent)
            .collect();
        let siblings: Vec<&str> = level.iter().map(|folder| folder.name.as_str()).collect();
        let asked = trim(name);
        let name = free_name(if asked.is_empty() { "New folder" } else { asked }, &siblings);
        let rank = rank_after(level.iter().map(|folder| folder.rank));
        let folder = Folder {
            id: self.next_id("folders", now),
            name,
            parent: parent.map(str::to_owned),
            rank: Some(rank),
        };
        self.folders.insert(0, folder.clone());
        self.reindex();
        folder
    }

    /// Renames a folder; refused for a blank name or one a folder beside it has.
    pub fn rename_folder(&mut self, id: &str, name: &str) -> Result<(), String> {
        let trimmed = trim(name).to_owned();
        if trimmed.is_empty() {
            return Err("A folder needs a name.".to_owned());
        }
        let parent = self.vault.folder(id).map(|folder| folder.parent.clone());
        let lower = trimmed.to_lowercase();
        let taken = self
            .vault
            .folders
            .iter()
            .any(|one| Some(&one.parent) == parent.as_ref() && one.id != id && one.name.to_lowercase() == lower);
        if taken {
            return Err(format!("A folder named {trimmed} is already there."));
        }
        self.update_folder(id, |folder| folder.name = trimmed)?;
        self.reindex();
        Ok(())
    }

    /// Moves a folder into another (none for the top of the vault), last in the reader's own order;
    /// refused into itself or a folder under it.
    pub fn move_folder(&mut self, id: &str, parent: Option<&str>) -> Result<Notice, String> {
        if let Some(parent) = parent
            && folder_and_below(&self.vault, id).iter().any(|one| one == parent)
        {
            return Err("A folder cannot go inside itself.".to_owned());
        }
        let rank = rank_after(
            self.folders
                .iter()
                .filter(|folder| folder.parent.as_deref() == parent && folder.id != id)
                .map(|folder| folder.rank),
        );
        self.update_folder(id, |folder| {
            folder.parent = parent.map(str::to_owned);
            folder.rank = Some(rank);
        })?;
        self.reindex();
        let name = self.vault.folder(id).map(|folder| folder.name.clone());
        Ok(Notice::new(moved_to(&self.vault, parent), name, true))
    }

    /// Deletes a folder, every folder under it and every note in them; the daily folder goes back to
    /// none if it was among them. Each note's toast, then the folder's; and what the last tab closed
    /// did to the note shown.
    pub fn delete_folder(&mut self, id: &str, now: i64) -> (Vec<Notice>, TabClosed) {
        let gone = folder_and_below(&self.vault, id);
        let name = self.vault.folder(id).map(|folder| folder.name.clone());
        let inside: Vec<String> = self
            .vault
            .notes
            .iter()
            .filter(|note| note.folder.as_ref().is_some_and(|folder| gone.contains(folder)))
            .map(|note| note.id.clone())
            .collect();
        let mut notices = Vec::new();
        let mut shown = TabClosed::Inactive;
        for note in &inside {
            let (notice, closed) = self.delete_note(note, now);
            notices.push(notice);
            if closed != TabClosed::Inactive {
                shown = closed;
            }
        }
        self.folders.retain(|folder| !gone.contains(&folder.id));
        self.reindex();
        if self
            .prefs
            .daily_folder
            .as_ref()
            .is_some_and(|daily| gone.contains(daily))
        {
            self.prefs.daily_folder = None;
        }
        notices.push(Notice::new(
            "Folder deleted",
            Some(format!(
                "{} and {} notes in it",
                name.as_deref().unwrap_or("The folder"),
                inside.len()
            )),
            false,
        ));
        (notices, shown)
    }

    /// Puts a note or a folder into a folder (none for the top of the vault) at a place among the
    /// notes or the folders there, counted without it; by name, only the folder it goes into
    /// counts. The toast when it changed folder.
    pub fn place(
        &mut self,
        what: Placing,
        id: &str,
        into: Option<&str>,
        index: usize,
    ) -> Result<Option<Notice>, String> {
        let manual = self.order() == Order::Manual;
        let into_owned = into.map(str::to_owned);
        if what == Placing::Folder {
            let Some(folder) = self.vault.folder(id).cloned() else {
                return Ok(None);
            };
            if !manual {
                return if folder.parent == into_owned {
                    Ok(None)
                } else {
                    self.move_folder(id, into).map(Some)
                };
            }
            if let Some(into) = into
                && folder_and_below(&self.vault, id).iter().any(|one| one == into)
            {
                return Err("A folder cannot go inside itself.".to_owned());
            }
            let mut level: Vec<Folder> = folders_in(&self.vault, into, Order::Manual)
                .into_iter()
                .filter(|one| one.id != id)
                .cloned()
                .collect();
            level.insert(index.min(level.len()), folder.clone());
            // Every folder of the level is ranked by its new place; only those that changed are saved.
            for (rank, one) in level.iter().enumerate() {
                if one.rank == Some(rank as i64) && one.parent == into_owned {
                    continue;
                }
                self.update_folder(&one.id, |saved| {
                    saved.rank = Some(rank as i64);
                    saved.parent = into_owned.clone();
                })?;
            }
            let notice = (folder.parent != into_owned)
                .then(|| Notice::new(moved_to(&self.vault, into), Some(folder.name), true));
            self.reindex();
            return Ok(notice);
        }
        let Some(note) = self.vault.note(id).cloned() else {
            return Ok(None);
        };
        if !manual {
            return if note.folder == into_owned {
                Ok(None)
            } else {
                self.move_note(id, into).map(Some)
            };
        }
        let daily = self.prefs.daily_folder.clone();
        let mut level: Vec<Note> = notes_in(&self.vault, into, Order::Manual, daily.as_deref())
            .into_iter()
            .filter(|one| one.id != id)
            .cloned()
            .collect();
        level.insert(index.min(level.len()), note.clone());
        for (rank, one) in level.iter().enumerate() {
            if one.rank == Some(rank as i64) && one.folder == into_owned {
                continue;
            }
            self.update_note(&one.id, |saved| {
                saved.rank = Some(rank as i64);
                saved.folder = into_owned.clone();
            })?;
        }
        let notice =
            (note.folder != into_owned).then(|| Notice::new(moved_to(&self.vault, into), Some(note.title), true));
        self.reindex();
        Ok(notice)
    }

    /// Makes a note the shown tab and the newest of the recent ones, opening it in the default
    /// view the first time.
    pub fn show(&mut self, id: &str) {
        let prefs = &mut self.prefs;
        let opened = prefs.tabs.iter().any(|tab| tab == id);
        if prefs.active.as_deref() == Some(id) && opened && prefs.recent.first().map(String::as_str) == Some(id) {
            return;
        }
        if !opened {
            prefs.tabs.push(id.to_owned());
        }
        prefs.recent.retain(|one| one != id);
        prefs.recent.insert(0, id.to_owned());
        prefs.recent.truncate(RECENT);
        prefs.active = Some(id.to_owned());
        prefs.views.entry(id.to_owned()).or_insert(prefs.default_view);
    }

    /// Closes a note's tab, its draft saved first; the tab after it is shown if it was the shown one.
    pub fn close_tab(&mut self, id: &str, now: i64) -> TabClosed {
        self.flush(Some(id), now);
        let prefs = &mut self.prefs;
        let at = prefs.tabs.iter().position(|tab| tab == id);
        prefs.tabs.retain(|tab| tab != id);
        prefs.views.remove(id);
        if prefs.active.as_deref() != Some(id) {
            return TabClosed::Inactive;
        }
        let next = at
            .and_then(|at| prefs.tabs.get(at.min(prefs.tabs.len().saturating_sub(1))))
            .cloned();
        prefs.active = next.clone();
        TabClosed::Active { next }
    }

    /// Sets what an open note shows.
    pub fn set_view(&mut self, id: &str, view: View) {
        self.prefs.views.insert(id.to_owned(), view);
    }

    /// A day's note, made from the template the first time: its id, and the toast when it was made.
    pub fn open_day(&mut self, date: &str, now: i64) -> (String, Option<Notice>) {
        let daily = self.prefs.daily_folder.clone();
        let found = self
            .vault
            .notes
            .iter()
            .find(|note| note.day.as_deref() == Some(date) || (note.title == date && note.folder == daily));
        if let Some(found) = found {
            return (found.id.clone(), None);
        }
        let body = from_template(&self.prefs.template, date);
        let note = self.create_note(
            NewNote {
                title: Some(date.to_owned()),
                folder: daily,
                body: Some(body),
                day: Some(date.to_owned()),
            },
            now,
        );
        let notice = Notice::new(
            format!("Daily note for {date}"),
            Some("Made from the template.".to_owned()),
            true,
        );
        (note.id, Some(notice))
    }

    /// Follows a link's name, written in the note `from`: to its note, or to asking to make it in
    /// that note's folder.
    pub fn follow(&self, target: &str, from: Option<&str>) -> Follow {
        match self.vault.resolve(target) {
            Some(found) => Follow::Open { id: found.id.clone() },
            None => Follow::Create {
                title: trim(target).to_owned(),
                folder: from
                    .and_then(|id| self.vault.note(id))
                    .and_then(|note| note.folder.clone()),
            },
        }
    }

    /// A new note in a folder, which the app opens and asks to name.
    pub fn new_note(&mut self, folder: Option<&str>, now: i64) -> Note {
        self.create_note(
            NewNote {
                folder: folder.map(str::to_owned),
                ..NewNote::default()
            },
            now,
        )
    }
}
