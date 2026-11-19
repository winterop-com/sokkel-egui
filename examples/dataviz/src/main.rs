//! Run: `cargo run -p dataviz`

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: sokkel_egui::desktop_viewport(
            eframe::egui::ViewportBuilder::default().with_inner_size([1440.0, 900.0]),
        ),
        ..Default::default()
    };
    eframe::run_native(
        "Visualizer",
        options,
        Box::new(|cc| Ok(Box::new(dataviz::VisualizerApp::new(cc)))),
    )
}
