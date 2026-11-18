//! sokkel-ui's records example, one to one in egui: Tidevann's survey records, forms filled in
//! sections and filed to a simulated server, the responses it holds, the households it follows,
//! the places it covers, and the health of its metadata.

pub mod app;
pub mod backend;
pub mod count_grid;
pub mod days;
pub mod form_fill;
pub mod form_layout;
pub mod forms;
pub mod forms_page;
pub mod geo;
pub mod health;
pub mod health_page;
pub mod households;
pub mod overview;
pub mod palette;
pub mod places_page;
pub mod responses;
pub mod server_page;
pub mod settings;
pub mod ui;

pub use app::RecordsApp;
