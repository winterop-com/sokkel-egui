//! Run: `cargo run -p metrics` (or `make metrics`)

fn main() -> eframe::Result {
    // the computer's zone, read before any other thread starts, as the zone must be on Unix
    let local_offset = time::UtcOffset::current_local_offset().map_or(0.0, |o| f64::from(o.whole_seconds()));
    let options = eframe::NativeOptions {
        viewport: sokkel_egui::desktop_viewport(
            eframe::egui::ViewportBuilder::default().with_inner_size([1440.0, 900.0]),
        ),
        ..Default::default()
    };
    eframe::run_native(
        "Metrics",
        options,
        Box::new(|cc| Ok(Box::new(metrics::MetricsApp::new(cc, local_offset)))),
    )
}
