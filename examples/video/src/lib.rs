//! sokkel-ui's video example in egui, its data layer one to one: Tidevann's video site on a
//! simulated backend. The channels and their videos (`catalog`); each video's clip, painted from
//! a shared scene with its sound, its renditions, poster and captions (`clips`); what the viewer
//! keeps, on a clock of its own: following, likes, the history and where they stopped, Watch later
//! and playlists, comments, downloads and the player's settings (`data`); how numbers and ages are
//! written (`format`); the site around every page (`site`), the parts and dialogs several pages
//! share (`parts`, `dialogs`), and each page's view and actions (`pages`).
//!
//! `captions` and `reactions` are the core's (`sokkel_egui::captions`, `toggle_reaction`), with
//! the track the viewer chose kept beside the captions' style.
//!
//! `tests/web.rs` holds each against the web's own modules, run under bun.
//!
//! The UI over it is the web's one to one: the frame with the rail, the top bar and the status line
//! (`app`), the pages (`screens`) with a video's card and its menu, the watch page with the player,
//! the actions, the comments and what plays next (`watch_page`), the settings (`settings_view`), Save
//! to and Share (`overlays`) and the palette with the video's keys (`palette`), each asking the app
//! for what it does (`act`). `tests/screens.rs` draws its screens.

pub mod act;
pub mod app;
pub mod captions;
pub mod catalog;
pub mod clips;
pub mod data;
pub mod dialogs;
pub mod format;
pub mod js;
pub mod overlays;
pub mod pages;
pub mod palette;
pub mod parts;
pub mod reactions;
pub mod screens;
pub mod settings_view;
pub mod site;
pub mod themes;
pub mod watch_page;

pub use app::VideoApp;
