//! Run: `cargo run -p storybook` (or `make storybook`)

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1280.0, 820.0]),
        ..Default::default()
    };
    eframe::run_native(
        "sokkel-egui",
        options,
        Box::new(|cc| Ok(Box::new(storybook::Storybook::new(cc)))),
    )
}
