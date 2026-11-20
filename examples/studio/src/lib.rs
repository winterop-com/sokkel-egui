//! sokkel-ui's studio example in egui, one to one: Tidevann's workspace of
//! pipelines, schemas and queries, the runs of its pipelines on a simulated backend, and a small SQL
//! engine over three weeks of simulated runs.

pub mod app;
pub mod backend;
pub mod dialogs;
pub use tidevann_shared::js;
pub mod model;
pub mod palette;
pub mod pipeline;
pub mod pipeline_edit;
pub mod pipeline_editor;
pub mod runs;
pub mod schema_editor;
pub mod seed;
pub mod settings;
pub mod simulate;
pub mod sql;
pub mod tables;
pub mod work;
pub mod yaml;

pub use app::StudioApp;
