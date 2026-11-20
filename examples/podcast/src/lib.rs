//! sokkel-ui's podcast example in egui, its data layer one to one: Tidevann's podcasts on a
//! simulated backend. The shows and their episodes (`catalog`); what the listener keeps, the shows
//! followed, each episode's place, star, download and New, the player, the settings and the time
//! listened (`backend`); each episode's script, timed syllable by syllable (`script`), and the
//! voices and bells that speak it (`voice`); the provider's state and every action on it, on a
//! clock of its own (`state`); the search (`discover`), the Stats page's numbers (`stats`), the
//! words several pages say (`parts`) and the themes offered (`themes`).
//!
//! `chapters`, `transcript` and `sleep_timer` are the media add-on's, re-exported here as the app
//! leans on them.
//!
//! `tests/web.rs` holds each against the web's own modules, run under bun.
//!
//! The UI over it is the web's one to one: the frame with the rail, the top bar and the status line
//! (`app`), the pages (`pages`) with the episode's rows, lists and the grid of shows (`episodes`),
//! the player along the foot (`player_bar`), the settings (`settings`) and the palette with the
//! player's keys (`palette`), each asking the app for what it does (`act`). `tests/screens.rs` draws
//! its screens.

pub mod act;
pub mod app;
pub mod backend;
pub mod catalog;
pub mod discover;
pub mod episodes;
pub use tidevann_shared::js;
pub mod pages;
pub mod palette;
pub mod parts;
pub mod player_bar;
pub mod script;
pub mod settings;
pub mod state;
pub mod stats;
pub mod themes;
pub mod voice;

pub use app::PodcastApp;
pub use sokkel_egui_media::{chapters, sleep_timer, transcript};

/// The playback speeds the player offers, as the web's `Player.tsx` lists them.
pub const RATES: [f64; 6] = [0.8, 1.0, 1.2, 1.5, 1.8, 2.0];
