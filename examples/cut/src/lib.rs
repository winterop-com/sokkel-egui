//! sokkel-ui's cut example in egui, its data layer one to one: Tidevann's video editor on a
//! simulated backend. Projects cut from a bin of procedural clips (`media`), edits made as pure
//! functions from one edit to the next (`model`) with undo and redo over them (`history`), the
//! playhead and its sound (`playback`, `sound`), a frame of the edit painted (`render`), the export's
//! frames and words (`export`), the projects and settings kept (`backend`), and the logic of the
//! editor's panes (`editor`).
//!
//! `tests/web.rs` holds each against the web's own modules, run under bun.
//!
//! The UI over it is the web's one to one: the frame (`app`), the projects (`projects_page`), the
//! editor (`editor_page`) with its bin (`bin_pane`), preview, inspector and timeline, the export
//! (`export_dialog`), the dialogs, the settings and the palette. `tests/screens.rs` draws its screens.

pub mod app;
pub mod backend;
pub mod bin_pane;
pub mod dialogs;
pub mod editor;
pub mod editor_page;
pub mod export;
pub mod export_dialog;
pub mod history;
pub mod inspector;
pub use tidevann_shared::js;
pub mod media;
pub mod model;
pub mod palette;
pub mod playback;
pub mod preview;
pub mod projects_page;
pub mod render;
pub mod seed;
pub mod settings;
pub mod sound;

pub use app::CutApp;
