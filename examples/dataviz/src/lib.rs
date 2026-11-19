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
//! The figures are shaped with the charts add-on's data module (`sokkel_egui_charts::data`: its
//! cells and rows, the pivot, the quantile and the ways figures are combined), as the web's with
//! sokkel-ui's; `js` is what the web leans on the browser for.
//!
//! `tests/web.rs` holds each against the web's own modules, run under bun.
//!
//! The UI over it is the web's one to one: the frame with the rail, the top bar and the status line,
//! the palette and the settings (`app`), the pages (`screens`), each chart drawn as its view
//! (`draw`), and the dialogs and the questions (`overlays`), each asking the app for what it does
//! (`act`). `tests/screens.rs` draws its screens.

pub mod act;
pub mod app;
pub mod backend;
pub mod chart_view;
pub mod dialogs;
pub mod draw;
pub mod import_dialog;
pub mod inputs;
pub mod js;
pub mod model;
pub mod overlays;
pub mod pages;
pub mod parse;
pub mod screens;
pub mod seed;
pub mod shape;
pub mod share;
pub mod themes;
pub mod views;
pub mod visualizer;

pub use app::VisualizerApp;

/// Now, in milliseconds since 1970, as the web's `Date.now()`.
pub fn js_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as i64)
}
