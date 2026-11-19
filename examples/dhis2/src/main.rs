//! Run: `cargo run -p dhis2`

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: sokkel_egui::desktop_viewport(
            eframe::egui::ViewportBuilder::default().with_inner_size([1440.0, 900.0]),
        ),
        ..Default::default()
    };
    eframe::run_native(
        "DHIS2 Workbench",
        options,
        Box::new(|cc| Ok(Box::new(dhis2::WorkbenchApp::new(cc)))),
    )
}
