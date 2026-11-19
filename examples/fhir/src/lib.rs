//! sokkel-ui's fhir example in egui, its data layer one to one: Tidevann's FHIR workbench on a
//! simulated server. The shapes it reads (`model`): a resource as its JSON, an issue, a refusal;
//! the types the server keeps and their elements (`types`); what it starts with (`seed`); the check
//! of a resource before it is kept and the name and version a CQL library declares (`validate`);
//! FHIRPath read and evaluated against the R4 model (`fhirpath`) and as the FHIRPath page runs it
//! (`evaluate`); search by parameters (`search`); and the server with its versions, history and
//! saved searches, on a clock of its own (`server`). What is being edited is in `editing`, the
//! words every page shares in `ui`, the frame in `app` and each page in `pages`.
//!
//! `js` is what the web leans on the browser for, `regex` the patterns FHIRPath's `matches` reads,
//! and `diff` the line diff sokkel-ui's diff view draws a version against the editor with.
//!
//! `tests/web.rs` holds each against the web's own modules, run under bun.

pub mod app;
pub mod diff;
pub mod editing;
pub mod evaluate;
pub mod fhirpath;
pub mod js;
pub mod model;
pub mod pages;
pub mod regex;
pub mod search;
pub mod seed;
pub mod server;
pub mod themes;
pub mod types;
pub mod ui;
pub mod validate;
