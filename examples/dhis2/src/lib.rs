//! sokkel-ui's dhis2 example in egui, its data layer one to one: Tidevann's metadata workbench on a
//! simulated server. What the server holds (`metadata`): organization units, category option
//! combos, data elements, the data set, constants and the months; each site's monthly report
//! (`values`); the versions it can run as and the names each knows (`versions`); its two
//! expression languages read, checked against the version and evaluated (`expression`, `check`);
//! the program's form and the engine that runs its rules on it (`program`); and the backend with
//! the indicators, validation rules and program rules the pages edit, on a clock of its own
//! (`backend`). Each page's and the frame's own words and logic are in `ui`, `app` and `pages`.
//!
//! `js` is what the web leans on the browser for, and `pattern` the regular expressions
//! `d2:validatePattern` tests a value with.
//!
//! `tests/web.rs` holds each against the web's own modules, run under bun.

pub mod app;
pub mod backend;
pub mod check;
pub mod expression;
pub mod js;
pub mod metadata;
pub mod pages;
pub mod pattern;
pub mod program;
pub mod themes;
pub mod ui;
pub mod values;
pub mod versions;
