use eframe::egui::{self, Ui, vec2};
use sokkel_ui_egui::{Card, TextRole, tokens};
use sokkel_ui_egui_charts::{BarChart, LineChart, Series, Sparkline, legend};

use super::{State, section};

const DAYS: [&str; 14] = [
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14",
];

pub fn line(_: &mut State, ui: &mut Ui) {
    section(ui, "Runs per day, by project");
    legend(ui, &["harbor", "juniper", "larkspur"]);
    ui.add(
        LineChart::new(DAYS)
            .series(Series::new(
                "harbor",
                [
                    42.0, 45.0, 51.0, 48.0, 56.0, 61.0, 58.0, 64.0, 70.0, 66.0, 72.0, 75.0, 71.0, 78.0,
                ],
            ))
            .series(Series::new(
                "juniper",
                [
                    30.0, 28.0, 33.0, 35.0, 31.0, 38.0, 41.0, 39.0, 44.0, 47.0, 45.0, 49.0, 52.0, 50.0,
                ],
            ))
            .series(Series::new(
                "larkspur",
                [
                    12.0, 15.0, 14.0, 18.0, 22.0, 20.0, 25.0, 24.0, 23.0, 28.0, 31.0, 29.0, 33.0, 36.0,
                ],
            )),
    );
    section(ui, "An area and a band");
    ui.add(
        LineChart::new(DAYS)
            .series(Series::new(
                "rows exported",
                [
                    8.2, 8.9, 9.4, 9.1, 10.2, 11.0, 10.6, 11.8, 12.4, 12.1, 13.0, 13.6, 13.2, 14.1,
                ],
            ))
            .band(
                vec![
                    7.4, 8.0, 8.4, 8.2, 9.1, 9.8, 9.4, 10.4, 10.9, 10.6, 11.3, 11.8, 11.3, 12.0,
                ],
                vec![
                    9.0, 9.8, 10.4, 10.0, 11.3, 12.2, 11.8, 13.2, 13.9, 13.6, 14.7, 15.4, 15.1, 16.2,
                ],
            )
            .area(true)
            .height(200.0),
    );
}

pub fn bars(_: &mut State, ui: &mut Ui) {
    let jobs = [
        "nightly-export",
        "sync-inventory",
        "rebuild-index",
        "send-digest",
        "import-orders",
    ];
    section(ui, "Runs per job by state");
    legend(ui, &["succeeded", "failed", "cancelled"]);
    ui.add(
        BarChart::new(jobs)
            .series(Series::new("succeeded", [124.0, 310.0, 48.0, 30.0, 96.0]))
            .series(Series::new("failed", [6.0, 22.0, 9.0, 1.0, 12.0]))
            .series(Series::new("cancelled", [2.0, 8.0, 3.0, 0.0, 4.0])),
    );
    section(ui, "Stacked");
    ui.add(
        BarChart::new(jobs)
            .series(Series::new("succeeded", [124.0, 310.0, 48.0, 30.0, 96.0]))
            .series(Series::new("failed", [6.0, 22.0, 9.0, 1.0, 12.0]))
            .series(Series::new("cancelled", [2.0, 8.0, 3.0, 0.0, 4.0]))
            .stacked(true)
            .height(200.0),
    );
}

pub fn sparkline(_: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    section(ui, "In stat cards");
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        for (i, (title, figure, values)) in [
            (
                "Runs today",
                "1,284",
                [40.0, 42.0, 39.0, 45.0, 51.0, 48.0, 55.0, 58.0, 62.0, 60.0],
            ),
            ("Failed", "37", [8.0, 6.0, 9.0, 5.0, 4.0, 6.0, 3.0, 4.0, 2.0, 3.0]),
            (
                "Median time",
                "4m 12s",
                [260.0, 255.0, 270.0, 248.0, 252.0, 244.0, 251.0, 249.0, 240.0, 252.0],
            ),
        ]
        .into_iter()
        .enumerate()
        {
            ui.allocate_ui(vec2(220.0, 140.0), |ui| {
                Card::new().show(ui, |ui| {
                    ui.label(TextRole::Help.rich(&t, title).color(t.color("muted-foreground")));
                    ui.label(TextRole::Statistic.rich(&t, figure).color(t.color("foreground")));
                    ui.add(Sparkline::new(values).size(186.0, 36.0).color(i));
                });
            });
        }
    });
    section(ui, "In a row");
    for (i, job) in ["nightly-export", "sync-inventory", "rebuild-index"]
        .into_iter()
        .enumerate()
    {
        ui.horizontal(|ui| {
            ui.add_sized(
                [160.0, 20.0],
                egui::Label::new(TextRole::Body.rich(&t, job).color(t.color("foreground"))),
            );
            let values = (0..16).map(|k| ((k * (i + 3)) % 7) as f64 + k as f64 * 0.4);
            ui.add(Sparkline::new(values).size(120.0, 24.0).color(i));
        });
    }
}
