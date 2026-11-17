//! sokkel-ui's records example, one to one in egui: Tidevann's survey records, forms filled in
//! sections and filed to a simulated server, the responses it holds, the households it follows,
//! the places it covers, and the health of its metadata.

pub mod app;
pub mod backend;
pub mod days;
pub mod forms;
pub mod geo;
pub mod overview;
pub mod ui;

pub use app::RecordsApp;
