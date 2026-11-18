//! Run: `cargo run -p notes`

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: sokkel_egui::desktop_viewport(
            eframe::egui::ViewportBuilder::default().with_inner_size([1440.0, 900.0]),
        ),
        ..Default::default()
    };
    eframe::run_native("Notes", options, Box::new(|cc| Ok(Box::new(notes::NotesApp::new(cc)))))
}
