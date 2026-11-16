//! What the dashboard app does: a widget configured, a dashboard made, duplicated and deleted, the
//! edits kept or thrown away after asking, the list of every dashboard, and settings.

use dashboard::{ALL, Asking, DashboardApp, model::Metric};
use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::{Appearance, toasts};

fn render(setup: impl FnOnce(&mut DashboardApp)) -> Harness<'static, DashboardApp> {
    let mut app = DashboardApp::on(Appearance::default());
    app.prefs.live = false;
    setup(&mut app);
    let mut harness = Harness::builder().with_size([1440.0, 1500.0]).build_ui_state(
        |ui, app: &mut DashboardApp| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(6);
    harness
}

fn board<'a>(app: &'a DashboardApp, id: &str) -> &'a dashboard::model::Dashboard {
    app.boards.iter().find(|b| b.id == id).expect("the board")
}

#[test]
fn a_widget_configured_and_kept() {
    let mut harness = render(|_| {});
    harness.get_by_label("Edit").click();
    harness.run_steps(3);
    harness.get_by_label("Configure Runs").click();
    harness.run_steps(4);
    assert!(harness.state().configure.is_some());
    // what the stat reads, chosen in its select
    harness.get_all_by_label("Reads").last().expect("the select").click();
    harness.run_steps(3);
    harness.get_by_label("Queue depth").click();
    harness.run_steps(3);
    harness.get_by_label("Apply").click();
    harness.run_steps(3);
    harness.get_by_label("Done").click();
    harness.run_steps(3);
    let widget = &board(harness.state(), "overview").widgets[0];
    assert_eq!(widget.options.metric, Some(Metric::Queue));
}

#[test]
fn a_dashboard_made_from_a_copy_duplicated_and_deleted() {
    let mut harness = render(|_| {});
    harness.get_by_label("New dashboard").click();
    harness.run_steps(4);
    harness.get_by_label("Name").click();
    harness.run_steps(2);
    harness.event(egui::Event::Text("Night shift".into()));
    harness.run_steps(2);
    harness.get_by_label("Create").click();
    harness.run_steps(4);
    let app = harness.state();
    let made = app.boards.last().expect("made");
    assert_eq!(made.name, "Night shift");
    assert!(made.widgets.is_empty());
    assert_eq!(app.place, made.id);
    let id = made.id.clone();
    harness.state_mut().duplicate(&id);
    harness.run_steps(3);
    assert_eq!(harness.state().boards.last().unwrap().name, "Night shift copy");
    harness.state_mut().asking = Some(Asking::Delete(id.clone()));
    harness.run_steps(3);
    harness.get_by_label("Delete").click();
    harness.run_steps(4);
    assert!(harness.state().boards.iter().all(|b| b.id != id));
    assert!(!harness.state().prefs.order.contains(&id));
    // each said so in a toast
    let said: Vec<String> = toasts(&harness.ctx).into_iter().map(|t| t.title).collect();
    assert_eq!(
        said,
        ["Night shift made", "Night shift copy made", "Night shift deleted"]
    );
}

#[test]
fn a_name_in_use_is_refused() {
    let mut harness = render(|app| app.ask_name(None));
    harness.get_by_label("Name").click();
    harness.run_steps(2);
    harness.event(egui::Event::Text("overview".into()));
    harness.run_steps(3);
    harness.get_by_label("Create").click();
    harness.run_steps(3);
    assert!(harness.state().naming.is_some());
    assert_eq!(harness.state().boards.len(), 3);
}

#[test]
fn cancel_asks_before_throwing_edits_away() {
    let mut harness = render(|_| {});
    harness.get_by_label("Edit").click();
    harness.run_steps(3);
    harness.get_by_label("Remove Runs").click();
    harness.run_steps(3);
    harness.get_by_label("Cancel").click();
    harness.run_steps(3);
    assert_eq!(harness.state().asking, Some(Asking::Leave));
    harness.get_by_label("Throw away").click();
    harness.run_steps(4);
    assert!(!harness.state().edit.editing());
    assert_eq!(board(harness.state(), "overview").widgets.len(), 13);
}

#[test]
fn the_list_of_every_dashboard() {
    let mut harness = render(|app| app.open(ALL));
    harness.snapshot("dashboard_list");
}

#[test]
fn settings_change_the_clock() {
    let mut harness = render(|app| app.settings = Some("live".into()));
    harness.get_by_label("12-hour").click();
    harness.run_steps(3);
    assert!(harness.state().prefs.twelve_hour);
}

#[test]
fn the_palette_opened_from_search_follows_live_data_and_goes_to_a_dashboard() {
    let mut harness = render(|_| {});
    harness.get_by_label("Search").click();
    harness.run_steps(3);
    harness.event(egui::Event::Text("follow live".into()));
    harness.run_steps(2);
    harness.key_press(egui::Key::Enter);
    harness.run_steps(2);
    assert!(harness.state().prefs.live);
    assert!(harness.state().palette.is_none());
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::K);
    harness.run_steps(2);
    harness.event(egui::Event::Text("team".into()));
    harness.run_steps(2);
    harness.key_press(egui::Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().place, "team");
}
