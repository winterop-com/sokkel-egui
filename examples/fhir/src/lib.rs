//! sokkel-ui's fhir example in egui, its data layer one to one: Tidevann's FHIR workbench on a
//! simulated server. The shapes it reads (`model`): a resource as its JSON, an issue, a refusal;
//! the types the server keeps and their elements (`types`); what it starts with (`seed`); the check
//! of a resource before it is kept and the name and version a CQL library declares (`validate`);
//! FHIRPath read and evaluated against the R4 model (`fhirpath`) and as the FHIRPath page runs it
//! (`evaluate`); search by parameters (`search`); and the server with its versions, history and
//! saved searches, on a clock of its own (`server`). What is being edited is in `editing`, the
//! words every page shares in `ui`, the frame in `app` and each page in `pages`.
//!
//! `js` is what the web leans on the browser for. The line diff a version is drawn against the
//! editor with is the core's (`sokkel_egui::diff_lines`), and FHIRPath's `matches` tests with the
//! core's patterns (`sokkel_egui::regex`).
//!
//! `tests/web.rs` holds each against the web's own modules, run under bun.
//!
//! The UI over it is the web's one to one: the frame with the rail, the top bar, the status line,
//! the palette, Settings and the questions (`workbench`), the overview, a type's resources, the CQL
//! libraries and search (`screens`), and a resource with its side panel, FHIRPath and a library's
//! editor (`detail`), FHIRPath and CQL colored by the fhir add-on's languages. `tests/screens.rs`
//! draws its screens.

pub mod app;
pub mod detail;
pub mod editing;
pub mod evaluate;
pub mod fhirpath;
pub use tidevann_shared::js;
pub mod model;
pub mod pages;
pub mod screens;
pub mod search;
pub mod seed;
pub mod server;
pub mod themes;
pub mod types;
pub mod ui;
pub mod validate;
pub mod workbench;

pub use workbench::WorkbenchApp;
