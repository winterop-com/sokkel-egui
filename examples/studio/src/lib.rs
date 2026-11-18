//! sokkel-ui's studio example in egui, its data layer for now, one to one: Tidevann's workspace of
//! pipelines, schemas and queries, the runs of its pipelines on a simulated backend, and a small SQL
//! engine over three weeks of simulated runs.

pub mod backend;
pub mod js;
pub mod model;
pub mod pipeline;
pub mod pipeline_edit;
pub mod seed;
pub mod simulate;
pub mod sql;
pub mod tables;
pub mod yaml;
