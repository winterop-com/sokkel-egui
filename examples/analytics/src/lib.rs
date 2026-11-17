//! sokkel-ui's analytics example, one to one in egui: Tidevann's live sales dashboard over a
//! simulated history of teams, their plans and the invoices those bill.

pub mod app;
pub mod customer;
pub mod customers;
pub mod days;
pub mod invoices;
pub mod metrics;
pub mod model;
pub mod overview;
pub mod panels;
pub mod revenue;
pub mod seed;
pub mod stream;
pub mod view;
pub mod world;

pub use app::AnalyticsApp;
