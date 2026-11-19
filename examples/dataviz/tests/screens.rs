//! The visualizer's screens at the web's routes, at 1440 by 900 on the parity day
//! (`parity/dataviz.json`), each a second after the page opened as the backend's first answers
//! come: `bun scripts/parity.ts board dataviz` puts them beside the web's. Then a few things the
//! reader does: a chart saved from the builder is listed under Charts, a chart added to a board
//! stands on it, a board's filter changes a panel's figure, and rows pasted in make a dataset.

use dataviz::VisualizerApp;
use dataviz::app::{DEMO_START, Dialog};
use dataviz::backend::ImportSource;
use dataviz::model::PanelSpan;
use egui_kittest::{Harness, SnapshotResults};
use sokkel_egui::{Appearance, Filter, FilterOperator};

pub fn render(path: &str, wait: i64) -> Harness<'static, VisualizerApp> {
    let mut app = VisualizerApp::at(DEMO_START, Appearance::default());
    app.go(path);
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut VisualizerApp| {
            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(2);
    harness.state_mut().wait(wait);
    harness.run_steps(4);
    harness
}

/// Moves the clock on past the backend's answers and draws again.
fn settle(harness: &mut Harness<'static, VisualizerApp>) {
    harness.run_steps(2);
    harness.state_mut().wait(1_000);
    harness.run_steps(4);
}

#[test]
fn every_screen() {
    let mut results = SnapshotResults::new();
    for (name, path) in [
        ("dataviz_datasets", "datasets"),
        ("dataviz_dataset", "datasets/ds-rainfall"),
        ("dataviz_build", "build"),
        ("dataviz_charts", "charts"),
        ("dataviz_chart", "charts/ch-rain-map"),
        ("dataviz_boards", "boards"),
        ("dataviz_board", "boards/bd-weather"),
        ("dataviz_settings", "settings"),
    ] {
        let mut harness = render(path, 1_000);
        results.add(harness.try_snapshot(name));
    }
}

#[test]
fn a_chart_saved_from_the_builder_is_listed_under_charts() {
    let mut harness = render("build/ds-rainfall", 1_000);
    let before = harness.state().backend.charts.seen().len();
    {
        let app = harness.state_mut();
        let (_, builder) = app.views.builder.as_mut().expect("the builder");
        builder.set_name("Rain by month");
        let mut builder = builder.clone();
        builder.save(&mut app.backend, false);
    }
    settle(&mut harness);
    let charts = harness.state().backend.charts.seen().to_vec();
    assert_eq!(charts.len(), before + 1);
    let saved = charts
        .iter()
        .find(|one| one.name == "Rain by month")
        .expect("the chart saved")
        .clone();
    // the builder goes on to the chart saved, as the web replaces the address
    assert_eq!(harness.state().route, ["charts", saved.id.as_str()]);
    harness.state_mut().go("charts");
    settle(&mut harness);
    let frame = dataviz::visualizer::Frame::of(&harness.state().backend, &harness.state().route);
    assert!(
        dataviz::pages::charts::search(frame.charts, frame.datasets, "rain by month")
            .iter()
            .any(|one| one.id == saved.id)
    );
}

#[test]
fn a_chart_added_to_a_board_stands_on_it() {
    let mut harness = render("boards/bd-weather", 1_000);
    let board = harness
        .state()
        .backend
        .boards
        .seen_get("bd-weather")
        .cloned()
        .expect("the board");
    let charts = harness.state().backend.charts.seen().to_vec();
    let placed = |id: &str, board: &dataviz::model::Board| board.items.iter().any(|item| item.chart_id == id);
    let chosen = charts
        .iter()
        .find(|chart| !placed(&chart.id, &board))
        .map(|chart| chart.id.clone())
        .expect("a chart not on the board");
    harness
        .state_mut()
        .backend
        .add_to_board(&board, &chosen, PanelSpan::Half, &charts);
    settle(&mut harness);
    let after = harness
        .state()
        .backend
        .boards
        .seen_get("bd-weather")
        .cloned()
        .expect("the board");
    assert_eq!(after.items.len(), board.items.len() + 1);
    assert!(placed(&chosen, &after));
}

#[test]
fn a_board_filter_changes_a_panels_figure() {
    let mut harness = render("boards/bd-weather", 1_000);
    let figure = |harness: &Harness<'static, VisualizerApp>| {
        let app = harness.state();
        let board = app.backend.boards.seen_get("bd-weather").cloned().expect("the board");
        let chart = app
            .backend
            .charts
            .seen_get("ch-rain-total")
            .cloned()
            .expect("the figure");
        let dataset = app
            .backend
            .datasets
            .seen_get(&chart.dataset_id)
            .cloned()
            .expect("its dataset");
        let filters = dataviz::shape::filters_for(&dataset, &board.filters);
        dataviz::shape::shape(&dataset, &chart.spec, &filters)
            .stat
            .value
            .as_number()
    };
    let all = figure(&harness);
    {
        let app = harness.state_mut();
        let filter = Filter::new("Region", FilterOperator::Is, vec!["Bomi".to_owned()]);
        let page = app.views.board.as_mut().expect("the board page");
        page.set_filters(&mut app.backend, vec![filter]);
    }
    settle(&mut harness);
    let one = figure(&harness);
    assert!(all.is_some() && one.is_some());
    assert!(
        one < all,
        "Bomi's rain alone, {one:?}, is less than every region's, {all:?}"
    );
}

#[test]
fn rows_pasted_in_make_a_dataset() {
    let mut harness = render("datasets", 1_000);
    let before = harness.state().backend.datasets.seen().len();
    {
        let app = harness.state_mut();
        let mut import = dataviz::import_dialog::ImportDialog::default();
        import.set_source(ImportSource::Paste);
        import.paste("Month,Harbor,Arrivals\n2025-01,Bergen,412\n2025-01,Cork,198\n2025-02,Bergen,437");
        assert_eq!(import.why(), None);
        assert_eq!(import.name, "Pasted data");
        app.dialog = Some(Dialog::Import(Box::new(import.clone())));
        import.save(&mut app.backend);
    }
    settle(&mut harness);
    let datasets = harness.state().backend.datasets.seen().to_vec();
    assert_eq!(datasets.len(), before + 1);
    let made = datasets
        .iter()
        .find(|one| one.name == "Pasted data")
        .expect("the dataset");
    assert_eq!(made.rows.len(), 3);
    assert_eq!(harness.state().route, ["datasets", made.id.as_str()]);
}
