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
//!
//! The UI over it is the web's one to one: the frame with the rail, the top bar with the server
//! menu, the status line, the palette, Settings and the questions (`workbench`), the list pages and
//! the organization units (`screens`), an indicator and a validation rule (`detail`), the program's
//! rules beside its form (`program_page`), and an expression in its field, colored, checked and
//! completed by the dhis2 add-on's languages (`editor`). `tests/screens.rs` draws its screens.

pub mod app;
pub mod backend;
pub mod check;
pub mod detail;
pub mod editor;
pub mod expression;
pub use tidevann_shared::js;
pub mod metadata;
pub mod pages;
pub mod pattern;
pub mod program;
pub mod program_page;
pub mod screens;
pub mod themes;
pub mod ui;
pub mod values;
pub mod versions;
pub mod workbench;

pub use workbench::WorkbenchApp;
