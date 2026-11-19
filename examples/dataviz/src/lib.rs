//! sokkel-ui's dataviz example in egui, its data layer one to one: Tidevann's visualizer on a
//! simulated backend. What it keeps (`model`): datasets of typed columns, the charts saved from
//! them and boards of charts with their filters; the demo's first data (`seed`); tables read from
//! a file or pasted text and written back as CSV (`parse`); a chart's figures read from its
//! dataset's columns, the views suggested for them and a map's breaks (`shape`); the views offered
//! (`views`); a chart shared as a link that carries its data (`share`); the backend and every
//! change the pages make to it, on a clock of its own (`backend`); and each page's and dialog's
//! own words and logic (`visualizer`, `chart_view`, `dialogs`, `import_dialog`, `inputs` and
//! `pages`).
//!
//! `chart_data` is sokkel-ui's charts data module (the pivot, the quantile and the ways figures
//! are combined) as far as the visualizer leans on it, carried here until the charts add-on has
//! it; `js` is what the web leans on the browser for.
//!
//! `tests/web.rs` holds each against the web's own modules, run under bun.

pub mod backend;
pub mod chart_data;
pub mod chart_view;
pub mod dialogs;
pub mod import_dialog;
pub mod inputs;
pub mod js;
pub mod model;
pub mod pages;
pub mod parse;
pub mod seed;
pub mod shape;
pub mod share;
pub mod themes;
pub mod views;
pub mod visualizer;
