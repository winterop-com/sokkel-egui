//! sokkel-ui's cut example in egui, its data layer one to one: Tidevann's video editor on a
//! simulated backend. Projects cut from a bin of procedural clips (`media`), edits made as pure
//! functions from one edit to the next (`model`) with undo and redo over them (`history`), the
//! playhead and its sound (`playback`, `sound`), a frame of the edit painted (`render`), the export's
//! frames and words (`export`), the projects and settings kept (`backend`), and the logic of the
//! editor's panes (`editor`).
//!
//! `tests/web.rs` holds each against the web's own modules, run under bun.

pub mod backend;
pub mod editor;
pub mod export;
pub mod history;
pub mod js;
pub mod media;
pub mod model;
pub mod playback;
pub mod render;
pub mod seed;
pub mod sound;
