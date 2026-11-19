//! sokkel-ui's models example in egui, its data layer one to one: Tidevann's local model runner on
//! a simulated backend. What the app keeps (`data`): the models and their files, the sources'
//! catalog and its variants, the seed, the disk, and how sizes and times are written; the engines
//! that run on the backend's clock (`engine`): the simulated model and its written answers, the
//! stream, the chat and the comparison, the pulls and the runner; what the app does from anywhere
//! (`actions`), the app's frame (`app`) and each page's own logic (`pages`).
//!
//! `js` is what the web leans on the browser for, and `runtime` the clock, timers and promises the
//! engines run on, held by hand.
//!
//! `tests/web.rs` holds each against the web's own modules, run under bun.

pub mod actions;
pub mod app;
pub mod data;
pub mod engine;
pub mod js;
pub mod pages;
pub mod runtime;
pub mod themes;
