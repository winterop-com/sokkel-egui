//! What the app does from anywhere, as the web's `actions.tsx` does it: a conversation opened with
//! a model, and models taken off the disk, with what the dialog that asks first says.

use crate::data::backend::Backend;
use crate::data::disk::model_path;
use crate::data::format::{bytes, counted};
use crate::data::model::{Level, LogSource, Model};
use crate::runtime::{Host, Promise, unit};

impl Backend {
    /// Opens a new conversation with a model and goes to it.
    pub fn chat_with(&mut self, model: &Model) {
        let made = self.start_conversation(model, "New chat");
        self.on(made, |backend, outcome| {
            if let Ok(value) = outcome
                && let Some(made) = value.downcast_ref::<crate::data::model::Conversation>()
            {
                let path = format!("/chat/{}", made.id);
                backend.go(path);
            }
        });
    }

    /// Takes models off the disk: unloads them, removes their files, and moves the default model
    /// on if it went.
    pub fn delete_models(&mut self, gone: Vec<Model>, all: Vec<Model>, default_model: &str) -> Promise {
        let done = self.pending();
        let default_model = default_model.to_owned();
        self.delete_each(gone.clone(), 0, all, gone, default_model, done);
        done
    }

    fn delete_each(
        &mut self,
        gone: Vec<Model>,
        at: usize,
        all: Vec<Model>,
        doomed: Vec<Model>,
        default_model: String,
        done: Promise,
    ) {
        let Some(model) = gone.get(at).cloned() else {
            if doomed.iter().any(|model| model.id == default_model) {
                let next = all.iter().find(|model| !doomed.iter().any(|one| one.id == model.id));
                if let Some(next) = next {
                    let id = next.id.clone();
                    let updated = self.update(|b| &mut b.prefs, "prefs", move |row| row.default_model = id);
                    self.on(updated, move |backend, outcome| {
                        backend.settle(done, outcome.map(|_| unit()))
                    });
                    return;
                }
            }
            self.settle(done, Ok(unit()));
            return;
        };
        let unloaded = self.unload_model(&model.id, "as it was deleted");
        self.on(unloaded, move |backend, _| {
            let removed = backend.remove(|b| &mut b.models, &model.id);
            backend.on(removed, move |backend, _| {
                backend.log(
                    Level::Info,
                    LogSource::Library,
                    format!("deleted {} {}, freeing {}", model.name, model.quant, bytes(model.size)),
                );
                backend.delete_each(gone, at + 1, all, doomed, default_model, done);
            });
        });
    }
}

/// What the delete dialog says before models go.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct DeleteDialog {
    pub question: String,
    pub detail: String,
    /// What it frees, and how many files go.
    pub freed: String,
    pub files: usize,
    /// The conversations that use them, with their label, when there are any.
    pub talks: Option<(String, usize)>,
}

/// The delete dialog for `doomed`, as the web's `useDeleteModels` draws it.
pub fn delete_dialog(doomed: &[Model], conversations: &[crate::data::model::Conversation], root: &str) -> DeleteDialog {
    let size = doomed.iter().fold(0.0, |sum, model| sum + model.size);
    let files = doomed.iter().map(|model| model.files.len()).sum();
    let talks = conversations
        .iter()
        .filter(|one| doomed.iter().any(|model| model.id == one.model))
        .count();
    let one = doomed.len() == 1;
    DeleteDialog {
        question: if one {
            format!("Delete {} {}?", doomed[0].name, doomed[0].quant)
        } else {
            format!("Delete {}?", counted(doomed.len() as f64, "model"))
        },
        detail: if one {
            format!(
                "Its files are removed from {}. It can be pulled again later.",
                model_path(root, &doomed[0])
            )
        } else {
            "Their files are removed from the disk. Each can be pulled again later.".to_owned()
        },
        freed: bytes(size),
        files,
        talks: (talks > 0).then(|| {
            (
                if one {
                    "Conversations that use it"
                } else {
                    "Conversations that use them"
                }
                .to_owned(),
                talks,
            )
        }),
    }
}

/// The note shown once models are deleted: its title and description.
pub fn deleted_note(gone: &[Model]) -> (String, String) {
    let title = if gone.len() == 1 {
        format!("Deleted {}", gone[0].name)
    } else {
        format!("Deleted {}", counted(gone.len() as f64, "model"))
    };
    let size = gone.iter().fold(0.0, |sum, model| sum + model.size);
    (title, format!("{} freed on the disk", bytes(size)))
}
