//! Run: `cargo run -p storage` (or `make storage`)

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: sokkel_egui::desktop_viewport(
            eframe::egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]),
        ),
        ..Default::default()
    };
    eframe::run_native(
        "Storage",
        options,
        Box::new(|cc| Ok(Box::new(storage::StorageApp::new(cc)))),
    )
}
