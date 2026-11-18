//! The questions the app asks in a dialog, as the web's `Dialogs.tsx`: a name for a folder, a new
//! name, where to move, whether to delete or to make a note.

use eframe::egui;
use sokkel_egui::{
    ConfirmAnswer, ConfirmDialog, Consequence, ConsequenceList, DialogShell, Icon, TextField, Toast, TreeSelect, field,
};

use crate::app::NotesApp;
use crate::backend::{Asking, NewNote, TabClosed};
use crate::daily::notify;
use crate::model::bad_title;
use crate::vault::{ROOT, folder_and_below, folder_tree};

/// A dialog's own state while it is open: the name typed, the backend's refusal of it, and the
/// folder picked to move into.
#[derive(Clone, Debug, Default)]
pub struct DialogState {
    pub name: String,
    pub refused: Option<String>,
    pub into: Option<String>,
}

/// The `sm` dialog's width.
const SMALL: f32 = 384.0;

impl NotesApp {
    /// Asks a question in a dialog, its fields started from what it is about.
    pub fn ask(&mut self, asking: Asking) {
        let vault = self.backend.vault();
        let mut state = DialogState::default();
        match &asking {
            Asking::NewFolder { .. } => state.name = "New folder".into(),
            Asking::RenameNote { id } => state.name = vault.note(id).map(|n| n.title.clone()).unwrap_or_default(),
            Asking::RenameFolder { id } => {
                state.name = vault.folder(id).map(|f| f.name.clone()).unwrap_or_default();
            }
            Asking::MoveNote { id } => {
                state.into = Some(
                    vault
                        .note(id)
                        .and_then(|n| n.folder.clone())
                        .unwrap_or_else(|| ROOT.into()),
                );
            }
            Asking::MoveFolder { id } => {
                state.into = Some(
                    vault
                        .folder(id)
                        .and_then(|f| f.parent.clone())
                        .unwrap_or_else(|| ROOT.into()),
                );
            }
            _ => {}
        }
        self.dialog = state;
        self.asking = Some(asking);
    }

    /// The dialog the app is asking in, if any.
    pub fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(asking) = self.asking.clone() else { return };
        let now = self.now();
        let vault = self.backend.vault();
        let mut done = false;
        match &asking {
            Asking::NewFolder { parent } => {
                let title = match parent {
                    None => "New folder".to_owned(),
                    Some(p) => format!("New folder in {}", vault.path(Some(p))),
                };
                if let Some(yes) = self.name_dialog(ctx, "new-folder", &title, "Make", None) {
                    if yes {
                        let name = self.dialog.name.clone();
                        self.backend.create_folder(&name, parent.as_deref(), now);
                    }
                    done = true;
                }
            }
            Asking::RenameNote { id } => {
                let Some(note) = vault.note(id).cloned() else {
                    self.asking = None;
                    return;
                };
                let others: Vec<String> = vault
                    .notes
                    .iter()
                    .filter(|n| n.id != note.id)
                    .map(|n| n.title.clone())
                    .collect();
                let typed = self.dialog.name.clone();
                let check = if typed.trim() == note.title {
                    None
                } else {
                    let taken: Vec<&str> = others.iter().map(String::as_str).collect();
                    bad_title(&typed, &taken)
                };
                if let Some(yes) = self.name_dialog(ctx, "rename-note", "Rename note", "Rename", check) {
                    if !yes {
                        done = true;
                    } else {
                        match self.backend.rename_note(&note.id, &typed, now) {
                            Ok(count) => {
                                if count > 0 {
                                    Toast::success("Renamed")
                                        .description(format!("{count} notes linking to it now say {}.", typed.trim()))
                                        .show(ctx);
                                }
                                done = true;
                            }
                            Err(reason) => self.dialog.refused = Some(reason),
                        }
                    }
                }
            }
            Asking::RenameFolder { id } => {
                let typed = self.dialog.name.clone();
                let check = typed.trim().is_empty().then(|| "A folder needs a name.".to_owned());
                if let Some(yes) = self.name_dialog(ctx, "rename-folder", "Rename folder", "Rename", check) {
                    if !yes {
                        done = true;
                    } else {
                        match self.backend.rename_folder(id, &typed) {
                            Ok(()) => done = true,
                            Err(reason) => self.dialog.refused = Some(reason),
                        }
                    }
                }
            }
            Asking::MoveNote { id } | Asking::MoveFolder { id } => {
                let is_note = matches!(asking, Asking::MoveNote { .. });
                let (name, now_in) = if is_note {
                    let note = vault.note(id);
                    (note.map(|n| n.title.clone()), note.and_then(|n| n.folder.clone()))
                } else {
                    let folder = vault.folder(id);
                    (folder.map(|f| f.name.clone()), folder.and_then(|f| f.parent.clone()))
                };
                let Some(name) = name else {
                    self.asking = None;
                    return;
                };
                let blocked = if is_note {
                    Vec::new()
                } else {
                    folder_and_below(vault, id)
                };
                let target = self.dialog.into.clone().filter(|i| i != ROOT);
                let why = if target == now_in {
                    Some("It is there already.")
                } else if target.as_ref().is_some_and(|t| blocked.contains(t)) {
                    Some("A folder cannot go inside itself.")
                } else {
                    None
                };
                let nodes = folder_tree(vault, "Vault", self.backend.order());
                let into = &mut self.dialog.into;
                let answer = DialogShell::new("move", format!("Move {name}"), "Move")
                    .width(SMALL)
                    .why(why)
                    .show(ctx, |ui| {
                        field(ui, "Into", None, |ui| {
                            TreeSelect::new("move-into", &nodes, into).expanded([ROOT]).show(ui);
                        });
                    });
                if let Some(yes) = answer {
                    if yes {
                        let moved = if is_note {
                            self.backend.move_note(id, target.as_deref())
                        } else {
                            self.backend.move_folder(id, target.as_deref())
                        };
                        match moved {
                            Ok(notice) => notify(ctx, notice),
                            Err(reason) => {
                                Toast::error("Not moved").description(reason).show(ctx);
                            }
                        }
                    }
                    done = true;
                }
            }
            Asking::DeleteNote { id } => {
                let Some(note) = vault.note(id).cloned() else {
                    self.asking = None;
                    return;
                };
                let linking = vault.backlinks(id).len();
                let detail = if linking == 0 {
                    "No note links to it.".to_owned()
                } else {
                    format!(
                        "{linking} {} to it; those links will lead nowhere.",
                        if linking == 1 { "note links" } else { "notes link" }
                    )
                };
                let answer = ConfirmDialog::new("delete-note", format!("Delete {}?", note.title), "Delete")
                    .detail(detail)
                    .destructive(true)
                    .show(ctx);
                match answer {
                    Some(ConfirmAnswer::Confirm) => {
                        let (notice, closed) = self.backend.delete_note(id, now);
                        notify(ctx, notice);
                        self.tab_closed(closed);
                        done = true;
                    }
                    Some(_) => done = true,
                    None => {}
                }
            }
            Asking::DeleteFolder { id } => {
                let Some(folder) = vault.folder(id).cloned() else {
                    self.asking = None;
                    return;
                };
                let gone = folder_and_below(vault, id);
                let notes = vault
                    .notes
                    .iter()
                    .filter(|n| n.folder.as_ref().is_some_and(|f| gone.contains(f)))
                    .count();
                let mut items = vec![
                    Consequence::new(if notes == 1 { "Note" } else { "Notes" })
                        .icon(Icon::FileText)
                        .count(notes),
                ];
                if gone.len() > 1 {
                    items.push(
                        Consequence::new(if gone.len() == 2 { "Folder" } else { "Folders" })
                            .icon(Icon::Folder)
                            .count(gone.len() - 1),
                    );
                }
                let answer = ConfirmDialog::new("delete-folder", format!("Delete {}?", folder.name), "Delete")
                    .detail("What it holds goes with it.")
                    .destructive(true)
                    .show_with(ctx, |ui| ConsequenceList::new(&items).show(ui));
                match answer {
                    Some(ConfirmAnswer::Confirm) => {
                        let (notices, closed) = self.backend.delete_folder(id, now);
                        for notice in notices {
                            notify(ctx, notice);
                        }
                        self.tab_closed(closed);
                        done = true;
                    }
                    Some(_) => done = true,
                    None => {}
                }
            }
            Asking::Create { title, folder } => {
                let place = folder
                    .as_ref()
                    .map(|f| format!(" in {}", vault.path(Some(f))))
                    .unwrap_or_default();
                let answer = ConfirmDialog::new("make-note", format!("Make {title}?"), "Make note")
                    .detail(format!("No note is named {title} yet. Make it{place} and open it."))
                    .show(ctx);
                match answer {
                    Some(ConfirmAnswer::Confirm) => {
                        let note = self.backend.create_note(
                            NewNote {
                                title: Some(title.clone()),
                                folder: folder.clone(),
                                body: None,
                                day: None,
                            },
                            now,
                        );
                        self.open_note(&note.id);
                        done = true;
                    }
                    Some(_) => done = true,
                    None => {}
                }
            }
        }
        if done {
            self.asking = None;
        }
    }

    /// A name typed in a dialog, refused with `check` (or the backend's last refusal) until it
    /// can go; Enter in the field answers as the confirm does.
    fn name_dialog(
        &mut self,
        ctx: &egui::Context,
        id: &str,
        title: &str,
        verb: &str,
        check: Option<String>,
    ) -> Option<bool> {
        let issue = self.dialog.refused.clone().or(check);
        let state = &mut self.dialog;
        let mut entered = false;
        let answer = DialogShell::new(id, title, verb)
            .width(SMALL)
            .why(issue.clone())
            .show(ctx, |ui| {
                let before = state.name.clone();
                let error = issue.as_ref().filter(|_| !state.name.trim().is_empty());
                let response = ui.add(TextField::new("Name", &mut state.name).error(error));
                if state.name != before {
                    state.refused = None;
                }
                if !ui.memory(|m| m.had_focus_last_frame(response.id)) && !response.has_focus() {
                    response.request_focus();
                }
                entered = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            });
        if entered && issue.is_none() {
            return Some(true);
        }
        answer
    }

    /// Makes a note, opens it and asks for its name, as the web's `newNote`.
    pub fn new_note_in(&mut self, folder: Option<&str>) {
        let now = self.now();
        let made = self.backend.new_note(folder, now);
        self.open_note(&made.id);
        self.ask(Asking::RenameNote { id: made.id });
    }

    /// An entry pressed in a note's menu (`note_menu`): Open, Rename, Move to, Delete.
    pub fn note_action(&mut self, id: &str, index: usize) {
        match index {
            0 => self.open_note(id),
            1 => self.ask(Asking::RenameNote { id: id.to_owned() }),
            2 => self.ask(Asking::MoveNote { id: id.to_owned() }),
            4 => self.ask(Asking::DeleteNote { id: id.to_owned() }),
            _ => {}
        }
    }

    /// An entry pressed in a folder's menu (`folder_menu`): New note, New folder, Rename, Move to,
    /// Delete.
    pub fn folder_action(&mut self, id: &str, index: usize) {
        match index {
            0 => self.new_note_in(Some(id)),
            1 => self.ask(Asking::NewFolder {
                parent: Some(id.to_owned()),
            }),
            3 => self.ask(Asking::RenameFolder { id: id.to_owned() }),
            4 => self.ask(Asking::MoveFolder { id: id.to_owned() }),
            6 => self.ask(Asking::DeleteFolder { id: id.to_owned() }),
            _ => {}
        }
    }

    /// Where the page goes when a tab closed with the note it showed.
    pub fn tab_closed(&mut self, closed: TabClosed) {
        if let TabClosed::Active { next } = closed
            && self.path.first().is_some_and(|p| p == "note")
        {
            match next {
                Some(next) => self.open_note(&next),
                None => self.go("notes"),
            }
        }
    }
}
