//! The questions the studio asks in a dialog, as the web's `Dialogs.tsx`: a new file's name and
//! place, a new name, whether to delete, and what to do with changes not saved.

use eframe::egui;
use sokkel_egui::{ConfirmAnswer, ConfirmDialog, DialogShell, Segmented, Select, ShellAnswer, TextField, field};

use crate::app::StudioApp;
use crate::backend::Asking;
use crate::model::{FileKind, KINDS, PROJECTS, bad_name, kind_of, stem};

/// The `sm` dialog's width.
const SMALL: f32 = 384.0;

/// What a dialog holds while it is open: the name typed, and the new file's kind and project.
#[derive(Clone, Debug, Default)]
pub struct DialogState {
    pub name: String,
    pub kind: usize,
    pub project: usize,
    /// The question these fields were started for.
    pub started: Option<String>,
}

impl StudioApp {
    /// Whichever question the studio is asking now.
    pub fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(asking) = self.backend.asking.clone() else {
            return;
        };
        let now = self.now();
        // the fields start from what the question is about, once
        let key = format!("{asking:?}");
        if self.dialog.started.as_deref() != Some(key.as_str()) {
            self.dialog = DialogState {
                started: Some(key),
                ..Default::default()
            };
            match &asking {
                Asking::New { project, kind } => {
                    self.dialog.project = PROJECTS.iter().position(|p| p == project).unwrap_or(0);
                    self.dialog.kind = KINDS.iter().position(|k| k.kind == *kind).unwrap_or(0);
                }
                Asking::Rename { id } => {
                    if let Some(file) = self.backend.file(id) {
                        self.dialog.name = stem(&file.name, file.kind).to_owned();
                    }
                }
                _ => {}
            }
        }
        let mut done = false;
        match &asking {
            Asking::New { .. } => {
                let kind: FileKind = KINDS[self.dialog.kind].kind;
                let project = PROJECTS[self.dialog.project];
                let taken: Vec<String> = self
                    .backend
                    .files
                    .iter()
                    .filter(|f| f.project == project && f.kind == kind)
                    .map(|f| stem(&f.name, f.kind).to_owned())
                    .collect();
                let taken: Vec<&str> = taken.iter().map(String::as_str).collect();
                let issue = bad_name(&self.dialog.name, &taken);
                let state = &mut self.dialog;
                let mut entered = false;
                let answer = DialogShell::new("new-file", "New file", "Make")
                    .width(SMALL)
                    .why(issue.clone())
                    .show(ctx, |ui| {
                        field(ui, "Kind", None, |ui| {
                            ui.add(Segmented::new(&mut state.kind, KINDS.map(|k| k.label)).fill(true));
                        });
                        field(ui, "Project", None, |ui| {
                            ui.add(
                                Select::new(&mut state.project, PROJECTS.map(|p| p.name()), "Project")
                                    .width(ui.available_width()),
                            );
                        });
                        let typed = state.name.trim().to_owned();
                        let saved = format!(
                            "Saved as {}{}.",
                            if typed.is_empty() { "name" } else { typed.as_str() },
                            kind_of(KINDS[state.kind].kind).extension
                        );
                        let error = issue.clone().filter(|_| !typed.is_empty());
                        let response = ui.add(TextField::new("Name", &mut state.name).help(saved).error(error));
                        if !ui.memory(|m| m.had_focus_last_frame(response.id)) && !response.has_focus() {
                            response.request_focus();
                        }
                        entered = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    });
                match answer {
                    Some(true) => {
                        let file = self.backend.create_file(project, kind, self.dialog.name.trim(), now);
                        self.open_tab(&file.id);
                        done = true;
                    }
                    Some(false) => done = true,
                    None if entered && issue.is_none() => {
                        let file = self.backend.create_file(project, kind, self.dialog.name.trim(), now);
                        self.open_tab(&file.id);
                        done = true;
                    }
                    None => {}
                }
            }
            Asking::Rename { id } => {
                let Some(file) = self.backend.file(id).cloned() else {
                    self.backend.asking = None;
                    return;
                };
                let own = stem(&file.name, file.kind).to_owned();
                let taken: Vec<String> = self
                    .backend
                    .files
                    .iter()
                    .filter(|f| f.id != file.id && f.project == file.project && f.kind == file.kind)
                    .map(|f| stem(&f.name, f.kind).to_owned())
                    .collect();
                let taken: Vec<&str> = taken.iter().map(String::as_str).collect();
                let typed = self.dialog.name.trim().to_owned();
                let issue = if typed == own {
                    Some("That is its name now.".to_owned())
                } else {
                    bad_name(&self.dialog.name, &taken)
                };
                let state = &mut self.dialog;
                let mut entered = false;
                let answer = DialogShell::new("rename-file", format!("Rename {}", file.name), "Rename")
                    .width(SMALL)
                    .why(issue.clone())
                    .show(ctx, |ui| {
                        let error = issue.clone().filter(|_| !typed.is_empty() && typed != own);
                        let response = ui.add(TextField::new("Name", &mut state.name).error(error));
                        if !ui.memory(|m| m.had_focus_last_frame(response.id)) && !response.has_focus() {
                            response.request_focus();
                        }
                        entered = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    });
                let go = answer == Some(true) || (answer.is_none() && entered && issue.is_none());
                if go {
                    self.backend.rename_file(id, &typed);
                    done = true;
                } else if answer == Some(false) {
                    done = true;
                }
            }
            Asking::Close { ids } => {
                let dirty = self.backend.dirty();
                let unsaved: Vec<String> = ids.iter().filter(|id| dirty.contains(*id)).cloned().collect();
                let names: Vec<String> = unsaved
                    .iter()
                    .map(|id| self.backend.file(id).map_or(id.clone(), |f| f.name.clone()))
                    .collect();
                let one = names.len() == 1;
                let (title, description) = if one {
                    (
                        format!("Save the changes to {}?", names[0]),
                        "They are lost if the tab closes without saving.".to_owned(),
                    )
                } else {
                    (
                        format!("Save the changes to {} files?", names.len()),
                        format!(
                            "{} hold changes that are lost if their tabs close without saving.",
                            names.join(", ")
                        ),
                    )
                };
                let answer = DialogShell::new("close-tabs", title, "Save")
                    .width(SMALL)
                    .description(description)
                    .other("Don't save")
                    .answer(ctx, |_| {});
                match answer {
                    Some(ShellAnswer::Confirm) => {
                        for id in &unsaved {
                            self.backend.save(id, now);
                        }
                        if let Some(to) = self.backend.close_now(ids) {
                            self.go(&to);
                        }
                        done = true;
                    }
                    Some(ShellAnswer::Other) => {
                        if let Some(to) = self.backend.close_now(ids) {
                            self.go(&to);
                        }
                        done = true;
                    }
                    Some(ShellAnswer::Cancel) => done = true,
                    None => {}
                }
            }
            Asking::Delete { id } => {
                let name = self.backend.file(id).map_or("this file".to_owned(), |f| f.name.clone());
                let detail = if self.backend.dirty().contains(id) {
                    "The file and the changes not saved in it are gone for good."
                } else {
                    "The file is gone for good; its runs stay in Runs."
                };
                let answer = ConfirmDialog::new("delete-file", format!("Delete {name}?"), "Delete")
                    .detail(detail)
                    .destructive(true)
                    .show(ctx);
                match answer {
                    Some(ConfirmAnswer::Confirm) => {
                        if let Some(to) = self.backend.delete_file(id) {
                            self.go(&to);
                        }
                        done = true;
                    }
                    Some(_) => done = true,
                    None => {}
                }
            }
        }
        if done {
            self.backend.asking = None;
            self.dialog = DialogState::default();
        }
    }
}
