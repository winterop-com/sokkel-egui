//! What the pages, the rail, the palette and the dialogs ask of the app, and how it does it: each
//! the web's handler of the same name, on the backend's calls.

use crate::app::{Dialog, VisualizerApp};
use crate::backend::Backend;
use crate::model::{Dataset, SavedChart, ViewSpec};
use crate::share::{link_to, pack_chart};

/// A question asked in a dialog before it is done.
#[derive(Clone, Debug, PartialEq)]
pub enum Asking {
    /// Delete a dataset, and the charts drawn from it.
    DeleteDataset(String),
    /// Delete a column of a dataset.
    DropColumn {
        dataset: String,
        column: String,
    },
    DeleteChart(String),
    DeleteBoard(String),
    /// Reset the demo data.
    Reset,
}

/// A change the backend makes.
pub type Change = Box<dyn FnOnce(&mut Backend)>;

/// One thing asked of the app.
pub enum Act {
    /// Go to an address, as the web's hash route.
    Go(String),
    /// A change to the backend.
    Do(Change),
    /// Open the import dialog.
    Import,
    /// Open the new board dialog, starting with a chart when given.
    NewBoard(Option<SavedChart>),
    /// Open a board's details.
    EditBoard(String),
    /// Open the dialog that adds a saved chart to a board.
    AddChart(String),
    /// Share a chart as a link: its name, its view and its dataset.
    Share(String, ViewSpec, Box<Dataset>),
    Ask(Asking),
    /// Close the settings.
    CloseSettings,
}

impl Act {
    pub fn change(change: impl FnOnce(&mut Backend) + 'static) -> Self {
        Act::Do(Box::new(change))
    }
}

/// Where a shared link opens: the web's demo's address.
pub const ORIGIN: &str = "https://tidevann.example.org";

impl VisualizerApp {
    /// Does what was asked.
    pub fn apply(&mut self, act: Act) {
        match act {
            Act::Go(path) => self.go(&path),
            Act::Do(change) => change(&mut self.backend),
            Act::Import => self.dialog = Some(Dialog::Import(Box::default())),
            Act::NewBoard(adding) => {
                self.dialog = Some(Dialog::Board {
                    editing: None,
                    adding: adding.map(Box::new),
                    name: String::new(),
                    description: String::new(),
                    then_open: true,
                });
            }
            Act::EditBoard(id) => {
                if let Some(board) = self.backend.boards.seen_get(&id).cloned() {
                    self.dialog = Some(Dialog::Board {
                        name: board.name.clone(),
                        description: board.description.clone(),
                        editing: Some(Box::new(board)),
                        adding: None,
                        then_open: false,
                    });
                }
            }
            Act::AddChart(board) => {
                self.dialog = Some(Dialog::AddChart {
                    board,
                    picked: None,
                    span: 1,
                });
            }
            Act::Share(name, spec, dataset) => {
                let packed = pack_chart(&name, &spec, &dataset, true);
                let link = link_to(&format!("#/shared/{packed}"), ORIGIN, "/");
                let size = crate::dialogs::link_size(&link);
                self.dialog = Some(Dialog::Share {
                    name,
                    link,
                    size,
                    dataset,
                });
            }
            Act::Ask(asking) => self.asking = Some(asking),
            Act::CloseSettings => self.settings = None,
        }
    }
}
