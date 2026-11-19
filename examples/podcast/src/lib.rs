//! sokkel-ui's podcast example in egui, its data layer one to one: Tidevann's podcasts on a
//! simulated backend. The shows and their episodes (`catalog`); what the listener keeps, the shows
//! followed, each episode's place, star, download and New, the player, the settings and the time
//! listened (`backend`); each episode's script, timed syllable by syllable (`script`), and the
//! voices and bells that speak it (`voice`); the provider's state and every action on it, on a
//! clock of its own (`state`); the search (`discover`), the Stats page's numbers (`stats`), the
//! words several pages say (`parts`) and the themes offered (`themes`).
//!
//! `chapters`, `transcript` and `sleep_timer` are sokkel-ui-media's pure parts the app leans on,
//! carried here until the media add-on has them.
//!
//! `tests/web.rs` holds each against the web's own modules, run under bun.

pub mod backend;
pub mod catalog;
pub mod chapters;
pub mod discover;
pub mod js;
pub mod parts;
pub mod script;
pub mod sleep_timer;
pub mod state;
pub mod stats;
pub mod themes;
pub mod transcript;
pub mod voice;

/// The playback speeds the player offers, as the web's `Player.tsx` lists them.
pub const RATES: [f64; 6] = [0.8, 1.0, 1.2, 1.5, 1.8, 2.0];
