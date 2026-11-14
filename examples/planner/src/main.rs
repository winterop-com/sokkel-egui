//! Run: `cargo run -p planner` (or `make planner`)

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: sokkel_ui_egui::desktop_viewport(
            eframe::egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]),
        ),
        ..Default::default()
    };
    eframe::run_native(
        "Planner",
        options,
        Box::new(|cc| Ok(Box::new(planner::PlannerApp::new(cc)))),
    )
}
