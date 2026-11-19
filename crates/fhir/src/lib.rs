//! sokkel-egui's FHIR add-on, the counterpart of `@winterop-com/sokkel-ui-fhir`: the health-data
//! standards' two languages, FHIRPath and CQL, for the core's code editor, UI only.
//!
//! Both are read by one grammar a line at a time (`program`), each spelling its own keywords,
//! operators, numbers and quoted names (`fhirpath`, `cql`). An app installs them with
//! `register_plugins(ctx, [fhir()])` and draws a `CodeEditor` with
//! `CodeLanguage::Plugin(FHIRPATH)` or `CodeLanguage::Plugin(CQL)`.

pub mod cql;
pub mod fhirpath;
pub mod plugin;
pub mod program;

pub use cql::{CQL, CQL_MEDIA_TYPES, cql_language, cql_tokens};
pub use fhirpath::{FHIRPATH, FHIRPATH_MEDIA_TYPES, fhirpath_language, fhirpath_tokens};
pub use plugin::fhir;
