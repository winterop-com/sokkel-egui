//! The questions the editor asks in a dialog, as the web's `ProjectDialog.tsx` and `Home.tsx`: a
//! new project's name, a new name, and whether to delete one.

use eframe::egui;
use sokkel_egui::{ConfirmAnswer, ConfirmDialog, DialogShell, TextField};

use crate::app::{CutApp, toast_of};
use crate::backend::{delete_detail, delete_question, name_problem};

/// The `sm` dialog's width.
const SMALL: f32 = 384.0;

/// What the app asks, by the project's id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asking {
    New,
    Rename(String),
    Delete(String),
}

/// What a dialog holds while it is open: the name typed, once started from the project's.
#[derive(Clone, Debug, Default)]
pub struct DialogState {
    pub name: String,
    pub started: bool,
}

impl CutApp {
    /// Whichever question the app is asking now.
    pub fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(asking) = self.asking.clone() else {
            return;
        };
        let now = self.now();
        let mut done = false;
        match &asking {
            Asking::New | Asking::Rename(_) => {
                let renaming = match &asking {
                    Asking::Rename(id) => match self.backend.project(id) {
                        Some(project) => Some(project.clone()),
                        None => {
                            self.asking = None;
                            return;
                        }
                    },
                    _ => None,
                };
                if !self.dialog.started {
                    self.dialog.name = renaming.as_ref().map(|p| p.name.clone()).unwrap_or_default();
                    self.dialog.started = true;
                }
                let taken: Vec<String> = self.backend.projects.iter().map(|p| p.name.clone()).collect();
                let taken: Vec<&str> = taken.iter().map(String::as_str).collect();
                let why = name_problem(&self.dialog.name, &taken, renaming.as_ref().map(|p| p.name.as_str()));
                let clash = why.filter(|w| *w == "Another project has this name.");
                let (title, confirm) = if renaming.is_some() {
                    ("Rename project", "Rename")
                } else {
                    ("New project", "Create")
                };
                let state = &mut self.dialog;
                let mut entered = false;
                let answer = DialogShell::new("project", title, confirm)
                    .width(SMALL)
                    .why(why)
                    .show(ctx, |ui| {
                        let response = ui.add(TextField::new("Name", &mut state.name).error(clash));
                        if !ui.memory(|m| m.had_focus_last_frame(response.id)) && !response.has_focus() {
                            response.request_focus();
                        }
                        entered = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    });
                let go = answer == Some(true) || (answer.is_none() && entered && why.is_none());
                if go {
                    let name = crate::js::trim(&self.dialog.name).to_owned();
                    match &renaming {
                        None => {
                            let (id, notice) = self.backend.create_project(&name, now);
                            toast_of(&notice).show(ctx);
                            self.go(&format!("p/{id}"));
                        }
                        Some(project) => {
                            if let Err(notice) = self.backend.rename_project(&project.id, &name, now) {
                                toast_of(&notice).show(ctx);
                            }
                        }
                    }
                    done = true;
                } else if answer == Some(false) {
                    done = true;
                }
            }
            Asking::Delete(id) => {
                let Some(project) = self.backend.project(id).cloned() else {
                    self.asking = None;
                    return;
                };
                let answer = ConfirmDialog::new("delete-project", delete_question(&project), "Delete")
                    .detail(delete_detail(&project))
                    .destructive(true)
                    .show(ctx);
                match answer {
                    Some(ConfirmAnswer::Confirm) => {
                        // the editor on it closes first, keeping what it had not saved
                        if self.editor.as_ref().is_some_and(|e| &e.project == id) {
                            self.close_editor(ctx);
                            self.go("projects");
                        }
                        let notice = self.backend.delete_project(id);
                        toast_of(&notice).show(ctx);
                        done = true;
                    }
                    Some(_) => done = true,
                    None => {}
                }
            }
        }
        if done {
            self.asking = None;
            self.dialog = DialogState::default();
        }
    }
}
