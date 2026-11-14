//! Run: `cargo run -p tasks` (or `make tasks`)

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: sokkel_ui_egui::desktop_viewport(
            eframe::egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]),
        ),
        ..Default::default()
    };
    eframe::run_native("Tasks", options, Box::new(|cc| Ok(Box::new(tasks::TasksApp::new(cc)))))
}
