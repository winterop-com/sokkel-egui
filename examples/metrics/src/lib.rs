//! sokkel-ui's metrics example, one to one in egui: live dashboards over a simulated source,
//! arranged by drag, a panel editor with a live preview, and alerts.

pub mod app;
pub mod board;
pub mod editor;
pub mod model;
pub mod panel_data;
pub mod panels;
pub mod query;
pub mod source;

pub use app::MetricsApp;
