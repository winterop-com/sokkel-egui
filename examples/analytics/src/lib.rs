//! sokkel-ui's analytics example, one to one in egui: Tidevann's live sales dashboard over a
//! simulated history of teams, their plans and the invoices those bill.

pub mod app;
pub mod csv;
pub mod customer;
pub mod customers;
pub mod days;
pub mod funnels;
pub mod goals;
pub mod invoices;
pub mod metrics;
pub mod model;
pub mod overview;
pub mod palette;
pub mod panels;
pub mod reports;
pub mod revenue;
pub mod seed;
pub mod settings;
pub mod stream;
pub mod view;
pub mod world;

pub use app::AnalyticsApp;
