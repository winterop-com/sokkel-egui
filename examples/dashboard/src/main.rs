//! Run: `cargo run -p dashboard` (or `make dashboard`)

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: sokkel_ui_egui::desktop_viewport(
            eframe::egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]),
        ),
        ..Default::default()
    };
    eframe::run_native(
        "Dashboard",
        options,
        Box::new(|cc| Ok(Box::new(dashboard::DashboardApp::new(cc)))),
    )
}
