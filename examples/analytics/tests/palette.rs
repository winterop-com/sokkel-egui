//! The palette: its commands as the web lists them, and the teams found by name.

use analytics::AnalyticsApp;
use analytics::app::DEMO_START;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use sokkel_egui::Appearance;

fn harness() -> Harness<'static, AnalyticsApp> {
    let mut app = AnalyticsApp::at(DEMO_START, Appearance::default());
    app.palette = Some((String::new(), 0));
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut AnalyticsApp| {
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
fn the_palette_lists_the_web_commands() {
    let mut harness = harness();
    harness.snapshot("analytics_palette");
}

#[test]
fn a_team_is_found_by_name_and_opened() {
    let mut harness = harness();
    harness.state_mut().palette = Some(("audio".into(), 0));
    harness.run_steps(4);
    harness.snapshot("analytics_palette_customers");
    let first = harness
        .state()
        .world
        .teams
        .iter()
        .filter(|t| t.name.to_lowercase().contains("audio"))
        .max_by_key(|t| analytics::world::standing(t, harness.state().world.now).mrr)
        .map(|t| (t.id.clone(), t.name.clone()))
        .unwrap();
    harness.get_by_label(&first.1).click();
    harness.run_steps(3);
    assert_eq!(harness.state().view.path, vec!["customers".to_owned(), first.0]);
}
