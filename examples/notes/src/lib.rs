//! sokkel-ui's notes example in egui, one to one: Tidevann's vault of linked notes, markdown beside
//! its preview, backlinks, tags, a graph of the links and a journal.

pub mod all_notes;
pub mod app;
pub mod backend;
pub mod daily;
pub mod dialogs;
pub mod graph_page;
pub mod model;
pub mod note_page;
pub mod note_panel;
pub mod palette;
pub mod search;
pub mod search_page;
pub mod seed;
pub mod settings;
pub mod tags;
pub mod vault;
pub mod wiki_links;

pub use app::NotesApp;
