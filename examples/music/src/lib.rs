//! sokkel-ui's music example in egui, its data layer one to one: Tidevann's music library on a
//! simulated backend. The catalog of artists, albums and songs (`catalog`), each song's sound made
//! from its seed (`synth`) and its words (`lyrics`, read back by `lrc`); what the listener keeps,
//! favorites, playlists, made stations, history and settings (`data`), and downloads coming in
//! (`downloads`); the player and its queue (`player`, `queue`); the search (`search`), the
//! visualizer's looks (`look`) and the themes offered (`themes`).
//!
//! `tests/web.rs` holds each against the web's own modules, run under bun.

pub mod catalog;
pub mod data;
pub mod downloads;
pub mod js;
pub mod look;
pub mod lrc;
pub mod lyrics;
pub mod player;
pub mod queue;
pub mod search;
pub mod synth;
pub mod themes;
