//! A panel's own actions: its window, its data inspected, a copy of it, and deleting it.

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use metrics::MetricsApp;
use sokkel_egui::Appearance;

fn harness() -> Harness<'static, MetricsApp> {
    harness_of(900.0)
}

fn harness_of(height: f32) -> Harness<'static, MetricsApp> {
    let mut app = MetricsApp::on(Appearance::default());
    app.live = false;
    let mut harness = Harness::builder().with_size([1440.0, height]).build_ui_state(
        |ui, app: &mut MetricsApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(3);
    harness
}

/// The `n`th panel of the board shown that is not a stat tile.
fn chart(harness: &Harness<'static, MetricsApp>, n: usize) -> (String, String) {
    let app = harness.state();
    let board = app.dashboards.iter().find(|b| b.id == app.place).unwrap();
    let panel = board
        .panels
        .iter()
        .filter(|p| p.kind != metrics::model::PanelKind::Stat)
        .nth(n)
        .unwrap();
    (panel.id.clone(), panel.title.clone())
}

#[test]
fn a_panel_opens_in_its_window() {
    let mut harness = harness();
    let (id, _) = chart(&harness, 2);
    // the first chart over time: the board's third row opens on Requests by service
    harness.get_all_by_label("Expand").nth(2).unwrap().click();
    harness.run_steps(4);
    assert_eq!(harness.state().panels.viewing.as_deref(), Some(id.as_str()));
    harness.snapshot("metrics_panel_window");
    harness.get_by_label("Close").click();
    harness.run_steps(3);
    assert!(harness.state().panels.viewing.is_none());
}

#[test]
fn a_panel_s_data_is_inspected() {
    let mut harness = harness();
    let (id, _) = chart(&harness, 0);
    let index = {
        let app = harness.state();
        let board = app.dashboards.iter().find(|b| b.id == app.place).unwrap();
        board.panels.iter().position(|p| p.id == id).unwrap()
    };
    harness.get_all_by_label("Panel actions").nth(index).unwrap().click();
    harness.run_steps(2);
    harness.snapshot("metrics_panel_menu");
    harness.get_by_label("Inspect data").click();
    harness.run_steps(4);
    assert_eq!(harness.state().panels.inspecting.as_deref(), Some(id.as_str()));
    harness.snapshot("metrics_inspect");
    harness.get_by_label("Close").click();
    harness.run_steps(3);
    assert!(harness.state().panels.inspecting.is_none());
}

#[test]
fn a_panel_is_duplicated_and_deleted() {
    let mut harness = harness();
    let before = harness.state().dashboards[0].panels.len();
    let first = harness.state().dashboards[0].panels[0].title.clone();
    harness.get_all_by_label("Panel actions").next().unwrap().click();
    harness.run_steps(2);
    harness.get_by_label("Duplicate").click();
    harness.run_steps(3);
    let panels = &harness.state().dashboards[0].panels;
    assert_eq!(panels.len(), before + 1);
    assert_eq!(panels[1].title, format!("{first} copy"));
    // deleting asks first
    harness.get_all_by_label("Panel actions").nth(1).unwrap().click();
    harness.run_steps(2);
    harness.get_by_label("Delete").click();
    harness.run_steps(3);
    assert!(harness.query_by_label(&format!("Delete {first} copy?")).is_some());
    harness.get_by_label("Delete").click();
    harness.run_steps(3);
    let panels = &harness.state().dashboards[0].panels;
    assert_eq!(panels.len(), before);
    assert!(panels.iter().all(|p| p.title != format!("{first} copy")));
}

#[test]
fn a_table_s_column_is_hidden() {
    // the Jobs board's table stands under the fold of a shorter window
    let mut harness = harness_of(1100.0);
    harness.state_mut().place = "jobs".into();
    harness.run_steps(3);
    harness.get_by_label("Columns").click();
    harness.run_steps(3);
    harness.snapshot("metrics_table_columns");
    harness.get_by_label("Mean").click();
    harness.run_steps(3);
    let app = harness.state();
    let table = app.tables.values().find(|t| !t.hidden.is_empty()).unwrap();
    assert!(table.hidden.contains(&2));
}
