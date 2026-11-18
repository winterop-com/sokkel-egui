#[test]
fn an_arrow_in_the_sans_face_draws() {
    let mut harness = egui_kittest::Harness::builder()
        .with_size([200.0, 40.0])
        .build_ui(|ui| {
            sokkel_egui::install(ui.ctx(), &sokkel_egui::Appearance::default());
            ui.label(sokkel_egui::TextRole::Body.rich(&sokkel_egui::tokens(ui.ctx()), "a \u{2192} b"));
        });
    harness.run();
}
