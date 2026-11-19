//! sokkel-ui's music example in egui, its data layer one to one: Tidevann's music library on a
//! simulated backend. The catalog of artists, albums and songs (`catalog`), each song's sound made
//! from its seed (`synth`) and its words (`lyrics`, read back by the media add-on's `lrc`, re-exported
//! here); what the listener keeps,
//! favorites, playlists, made stations, history and settings (`data`), and downloads coming in
//! (`downloads`); the player and its queue (`player`, `queue`); the search (`search`), the
//! visualizer's looks (`look`) and the themes offered (`themes`).
//!
//! `tests/web.rs` holds each against the web's own modules, run under bun.
//!
//! The UI over it is the web's one to one: the frame (`app`), the pages (`pages`) with their cards,
//! rows and menus (`songs`), the player bar (`player_bar`), the queue beside the page
//! (`queue_panel`), the dialogs, the settings and the palette with the player's keys, each asking
//! the app for what it does (`act`). `tests/screens.rs` draws its screens.

pub mod act;
pub mod app;
pub mod catalog;
pub mod data;
pub mod dialogs;
pub mod downloads;
pub mod js;
pub mod look;
pub mod lyrics;
pub mod pages;
pub mod palette;
pub mod player;
pub mod player_bar;
pub mod queue;
pub mod queue_panel;
pub mod search;
pub mod settings;
pub mod songs;
pub mod synth;
pub mod themes;

pub use app::MusicApp;
pub use sokkel_egui_media::lrc;
