//! sokkel-ui's runs example, one to one in egui: Tidevann's jobs console, an overview, runs going
//! live, a run's page, jobs, schedules and settings, on a simulated backend.

pub mod app;
pub mod backend;
pub mod job_page;
pub mod jobs_page;
pub mod logs;
pub mod model;
pub mod new_job;
pub mod palette;
pub mod parts;
pub mod run_page;
pub mod run_panel;
pub mod runs_page;
pub mod schedule;
pub mod schedules_page;
pub mod seed;
pub mod settings;
pub mod themes;
pub mod timeline;

pub use app::RunsApp;
