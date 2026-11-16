//! A log that grows fills a dashboard panel stretched to a taller row, as the web's `grow` over
//! `h-72`, and holds there: its growth is not counted in the panel's own height, so the row does
//! not creep taller frame by frame.

use egui::{Rect, vec2};
use egui_kittest::Harness;
use sokkel_ui_egui::{DashboardGrid, DashboardPanel, LogLine, LogView, PanelSpan, install};

struct Seen {
    log: Rect,
    beside: Rect,
}

impl Default for Seen {
    fn default() -> Self {
        Self {
            log: Rect::NOTHING,
            beside: Rect::NOTHING,
        }
    }
}

fn harness(grow: bool, beside: f32) -> Harness<'static, Seen> {
    let lines: Vec<LogLine> = (0..40).map(|i| LogLine::new(format!("line {i}"))).collect();
    Harness::builder().with_size([1200.0, 1400.0]).build_ui_state(
        move |ui, seen: &mut Seen| {
            install(ui.ctx(), &Default::default());
            let lines = lines.clone();
            let (mut log, mut other) = (Rect::NOTHING, Rect::NOTHING);
            DashboardGrid::new("grid").show(ui, |grid| {
                let (log, other) = (&mut log, &mut other);
                grid.cell(PanelSpan::TwoThirds, move |ui| {
                    *log = DashboardPanel::new("worker log").flush(true).show(ui, |ui| {
                        let top = ui.cursor().top();
                        LogView::new("log", &lines).follow(true).grow(grow).show(ui);
                        Rect::from_min_max(egui::pos2(ui.min_rect().left(), top), ui.min_rect().max)
                    });
                });
                grid.cell(PanelSpan::Third, move |ui| {
                    *other = DashboardPanel::new("beside").show(ui, |ui| {
                        ui.allocate_exact_size(vec2(ui.available_width(), beside), egui::Sense::hover());
                        ui.min_rect()
                    });
                });
            });
            seen.log = log;
            seen.beside = other;
        },
        Seen::default(),
    )
}

#[test]
fn alone_it_stands_at_its_height() {
    let mut harness = harness(true, 40.0);
    harness.run_steps(6);
    assert!(
        (harness.state().log.height() - 288.0).abs() < 1.0,
        "{}",
        harness.state().log.height()
    );
}

#[test]
fn beside_a_taller_panel_it_fills_its_row_and_holds() {
    let mut harness = harness(true, 600.0);
    harness.run_steps(6);
    let first = harness.state().log.height();
    // the log's foot is the panel's foot, less its 1 border; the other panel's body ends 16 and
    // its border above its own
    let (log, beside) = (harness.state().log, harness.state().beside);
    assert!(
        (log.bottom() - (beside.bottom() + 16.0)).abs() < 1.5,
        "it fills its row: {log:?} beside {beside:?}"
    );
    harness.run_steps(20);
    let later = harness.state().log.height();
    assert!((later - first).abs() < 0.5, "and holds: {first} then {later}");
}

#[test]
fn without_grow_it_keeps_its_height() {
    let mut harness = harness(false, 600.0);
    harness.run_steps(6);
    assert!((harness.state().log.height() - 288.0).abs() < 1.0);
}
