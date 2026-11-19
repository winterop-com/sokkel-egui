//! The playlist dialogs, as the web's `parts/dialogs.tsx`: a new playlist's name (holding songs
//! when made from a menu), a new name for one, and whether to delete one; and the question before
//! every download goes.

use eframe::egui;
use sokkel_egui::{ConfirmAnswer, ConfirmDialog, DialogShell, TextField, Toast};

use crate::act::songs_word;
use crate::app::MusicApp;

/// The `sm` dialog's width.
const SMALL: f32 = 384.0;

/// What the app asks, by the playlist's id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asking {
    /// A new playlist, holding these songs, with what they are.
    New(Vec<String>, Option<String>),
    Rename(String),
    Delete(String),
}

/// What a name dialog holds while it is open: the name typed, once started from the playlist's.
#[derive(Clone, Debug, Default)]
pub struct DialogState {
    pub name: String,
    pub started: bool,
}

impl MusicApp {
    /// Asks a question in a dialog, its fields started anew.
    pub fn ask(&mut self, asking: Asking) {
        self.dialog = DialogState::default();
        self.asking = Some(asking);
    }

    /// Whichever question the app is asking now.
    pub fn dialogs(&mut self, ctx: &egui::Context) {
        self.clearing_dialog(ctx);
        let Some(asking) = self.asking.clone() else {
            return;
        };
        let now = self.now();
        let mut done = false;
        match &asking {
            Asking::New(..) | Asking::Rename(_) => {
                let renaming = match &asking {
                    Asking::Rename(id) => match self.library.playlist(id) {
                        Some(playlist) => Some(playlist.clone()),
                        None => {
                            self.asking = None;
                            return;
                        }
                    },
                    _ => None,
                };
                let start = renaming.as_ref().map(|p| p.name.clone()).unwrap_or_default();
                if !self.dialog.started {
                    self.dialog.name = start.clone();
                    self.dialog.started = true;
                }
                let trimmed = crate::js::trim(&self.dialog.name).to_owned();
                let why = if trimmed.is_empty() {
                    Some("Give it a name")
                } else if trimmed == start {
                    Some("The name is the same")
                } else {
                    None
                };
                let (title, confirm) = if renaming.is_some() {
                    ("Rename playlist", "Rename")
                } else {
                    ("New playlist", "Create")
                };
                let mut shell = DialogShell::new("playlist", title, confirm).width(SMALL).why(why);
                if let Asking::New(songs, what) = &asking
                    && !songs.is_empty()
                {
                    shell = shell.description(format!(
                        "With {}.",
                        what.clone().unwrap_or_else(|| songs_word(songs.len()))
                    ));
                }
                let state = &mut self.dialog;
                let mut entered = false;
                let answer = shell.show(ctx, |ui| {
                    let response = ui.add(TextField::new("Name", &mut state.name));
                    if !ui.memory(|m| m.had_focus_last_frame(response.id)) && !response.has_focus() {
                        response.request_focus();
                    }
                    entered = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                });
                if answer == Some(true) || (answer.is_none() && entered && why.is_none()) {
                    match (&asking, &renaming) {
                        (Asking::New(songs, _), _) => {
                            let id = self.library.create_playlist(&trimmed, songs, now);
                            let mut toast = Toast::success(format!("Made {trimmed}"));
                            if !songs.is_empty() {
                                toast = toast.description(format!("With {}.", songs_word(songs.len())));
                            }
                            toast.show(ctx);
                            self.go(&format!("playlists/{id}"));
                        }
                        (_, Some(playlist)) => {
                            let id = playlist.id.clone();
                            self.library.rename_playlist(&id, &trimmed, now);
                        }
                        _ => {}
                    }
                    done = true;
                } else if answer == Some(false) {
                    done = true;
                }
            }
            Asking::Delete(id) => {
                let Some(playlist) = self.library.playlist(id).cloned() else {
                    self.asking = None;
                    return;
                };
                let answer = ConfirmDialog::new("delete-playlist", format!("Delete {}?", playlist.name), "Delete")
                    .detail("The playlist goes; its songs stay in the library.")
                    .destructive(true)
                    .show(ctx);
                match answer {
                    Some(ConfirmAnswer::Confirm) => {
                        self.library.delete_playlist(id);
                        Toast::new(format!("Deleted {}", playlist.name)).show(ctx);
                        self.go("playlists");
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
