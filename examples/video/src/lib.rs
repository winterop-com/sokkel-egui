//! sokkel-ui's video example in egui, its data layer one to one: Tidevann's video site on a
//! simulated backend. The channels and their videos (`catalog`); each video's clip, painted from
//! a shared scene with its sound, its renditions, poster and captions (`clips`); what the viewer
//! keeps, on a clock of its own: following, likes, the history and where they stopped, Watch later
//! and playlists, comments, downloads and the player's settings (`data`); how numbers and ages are
//! written (`format`); the site around every page (`site`), the parts and dialogs several pages
//! share (`parts`, `dialogs`), and each page's view and actions (`pages`).
//!
//! `captions` and `reactions` are sokkel-ui's pure parts the site leans on, carried here until
//! the core has them.
//!
//! `tests/web.rs` holds each against the web's own modules, run under bun.

pub mod captions;
pub mod catalog;
pub mod clips;
pub mod data;
pub mod dialogs;
pub mod format;
pub mod js;
pub mod pages;
pub mod parts;
pub mod reactions;
pub mod site;
pub mod themes;
