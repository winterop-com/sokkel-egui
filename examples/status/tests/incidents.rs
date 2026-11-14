//! The incidents as a person looks through them: every incident shown, then "/" reaching the
//! search and a word narrowing the rows.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use sokkel_ui_egui::Appearance;
use status::StatusApp;

fn incidents() -> Harness<'static, StatusApp> {
    let mut app = StatusApp::on(Appearance::default());
    app.go("incidents");
    app.shown = 2;
    let mut harness = Harness::builder().with_size([1280.0, 800.0]).build_ui_state(
        |ui, app: &mut StatusApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(4);
    harness
}

#[test]
fn slash_reaches_the_search_and_a_word_narrows_the_rows() {
    let mut harness = incidents();
    assert!(harness.query_by_label("9 rows").is_some());
    harness.event(egui::Event::Text("/".into()));
    harness.run_steps(2);
    harness.event(egui::Event::Text("search".into()));
    harness.run_steps(3);
    assert_eq!(harness.state().incident_search, "search");
    // "Slow searches in Asia" and "Search results out of date"
    assert!(harness.query_by_label("2 rows").is_some());
}
