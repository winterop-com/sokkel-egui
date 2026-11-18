//! The console's front page, as the web's `pages/Overview.tsx`: how the last day went, what is
//! going now, what failed and what comes next, as panels the person arranges.

use eframe::egui::Ui;
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, Cell, Column, DashboardArrange, DashboardEditActions, DashboardLayoutItem,
    DashboardPanel, DashboardWidget, DataTable, Icon, PageHeader, PanelSpan, RefreshControl, StatColorMode, StatPanel,
    TableVariant, Threshold,
};
use sokkel_egui_charts::{BarPanel, BoxPlotPanel, DonutPanel, Series};

use crate::app::{LiveRun, RunsApp};
use crate::model::{STATES, Status, Trigger, clock, counted, took};
use crate::parts::{state_cell, when};
use crate::schedule::{next_fire, say_schedule};
use crate::timeline::is_live;

const HOUR: i64 = 3_600_000;
/// The intervals the refresh offers, in seconds, 0 for never.
const REFRESH: [u32; 5] = [0, 10, 30, 60, 300];

const SUCCESS: [Threshold; 3] = [
    Threshold {
        value: None,
        color: "critical",
    },
    Threshold {
        value: Some(85.0),
        color: "warning",
    },
    Threshold {
        value: Some(95.0),
        color: "good",
    },
];
const FAILURES: [Threshold; 3] = [
    Threshold {
        value: None,
        color: "good",
    },
    Threshold {
        value: Some(1.0),
        color: "warning",
    },
    Threshold {
        value: Some(10.0),
        color: "critical",
    },
];

/// Every panel the overview can show: its id, label, icon and width.
const WIDGETS: [(&str, &str, Icon, PanelSpan); 15] = [
    ("runs", "Runs", Icon::ListChecks, PanelSpan::Sixth),
    ("success", "Success rate", Icon::CircleCheck, PanelSpan::Sixth),
    ("failed", "Failed", Icon::CircleX, PanelSpan::Sixth),
    ("running", "Going now", Icon::CircleDot, PanelSpan::Sixth),
    ("queued", "Queued", Icon::Hourglass, PanelSpan::Sixth),
    ("median", "Median duration", Icon::Timer, PanelSpan::Sixth),
    (
        "per-hour",
        "Runs per hour",
        Icon::ChartColumnStacked,
        PanelSpan::TwoThirds,
    ),
    ("by-job", "Failures by job", Icon::ChartPie, PanelSpan::Third),
    ("durations", "Durations", Icon::Activity, PanelSpan::Half),
    ("trouble", "Runs in trouble", Icon::TriangleAlert, PanelSpan::Half),
    ("going", "Running and queued", Icon::CircleDot, PanelSpan::Half),
    ("next", "Next up", Icon::Clock, PanelSpan::Half),
    ("by-trigger", "Runs by trigger", Icon::ChartPie, PanelSpan::Third),
    ("busiest", "Runs by job", Icon::ChartColumn, PanelSpan::Third),
    ("latest", "Latest runs", Icon::ListOrdered, PanelSpan::Half),
];

/// The overview as it comes: the figures in a row, the charts, then the lists.
pub fn default_layout() -> Vec<DashboardLayoutItem> {
    WIDGETS[..12]
        .iter()
        .map(|(id, _, _, span)| DashboardLayoutItem::new(*id, *span))
        .collect()
}

fn label_of(id: &str) -> &'static str {
    WIDGETS.iter().find(|w| w.0 == id).map_or("", |w| w.1)
}

fn median(values: &[i64]) -> Option<i64> {
    let mut sorted = values.to_vec();
    sorted.sort();
    sorted.get(sorted.len() / 2).copied()
}

/// What the panels read: the last 24 hours, worked out once a minute.
struct Day {
    recent: Vec<LiveRun>,
    hours: Vec<String>,
    per_hour: Vec<(Status, Vec<f64>)>,
    by_job: Vec<(String, f64)>,
    durations: Vec<(String, Vec<f64>)>,
    success: Option<f64>,
    failed: usize,
    median: Option<i64>,
    spark_runs: Vec<f64>,
    spark_failed: Vec<f64>,
    by_trigger: Vec<(String, f64)>,
    by_job_runs: Vec<(String, f64)>,
}

/// What the overview holds between frames: its refresh, and the minute it was read at.
#[derive(Default)]
pub struct OverviewView {
    pub every: u32,
    pub paused: bool,
    pub updated: Option<i64>,
}

impl RunsApp {
    fn day(&self, runs: &[LiveRun], now: i64) -> Day {
        let minute = now.div_euclid(60_000);
        let end = (minute + 1) * 60_000;
        let start = end - 24 * HOUR;
        let recent: Vec<LiveRun> = runs
            .iter()
            .filter(|r| r.run.created > start && r.run.created <= end)
            .cloned()
            .collect();
        let first = (start + HOUR).div_euclid(HOUR) * HOUR;
        let hours: Vec<String> = (0..24).map(|i| clock(first + i * HOUR, false)).collect();
        let hour_of = |at: i64| clock(at.div_euclid(HOUR) * HOUR, false);
        let per_hour = STATES
            .iter()
            .map(|state| {
                let counts = hours
                    .iter()
                    .map(|hour| {
                        recent
                            .iter()
                            .filter(|r| r.at.state == *state && &hour_of(r.run.created) == hour)
                            .count() as f64
                    })
                    .collect();
                (*state, counts)
            })
            .collect();
        let finished = recent
            .iter()
            .filter(|r| matches!(r.at.state, Status::Succeeded | Status::Failed))
            .count();
        let failed: Vec<&LiveRun> = recent.iter().filter(|r| r.at.state == Status::Failed).collect();
        let jobs = &self.backend.jobs;
        let by_job = jobs
            .iter()
            .map(|job| {
                (
                    job.id.clone(),
                    failed.iter().filter(|r| r.run.job == job.id).count() as f64,
                )
            })
            .collect();
        let succeeded: Vec<&LiveRun> = recent.iter().filter(|r| r.at.state == Status::Succeeded).collect();
        let usual = |job: &str| {
            let own: Vec<i64> = succeeded
                .iter()
                .filter(|r| r.run.job == job)
                .map(|r| r.at.duration)
                .collect();
            median(&own)
        };
        // each run against its job's usual time, so a job of seconds and one of minutes read alike
        let mut durations: Vec<(String, Vec<f64>)> = Vec::new();
        for run in &succeeded {
            let base = usual(&run.run.job).unwrap_or(run.at.duration).max(1);
            let share = crate::model::js_round(run.at.duration as f64 / base as f64 * 100.0);
            match durations.iter_mut().find(|(job, _)| *job == run.run.job) {
                Some((_, values)) => values.push(share),
                None => durations.push((run.run.job.clone(), vec![share])),
            }
        }
        let spark = |keep: &dyn Fn(&LiveRun) -> bool| -> Vec<f64> {
            hours
                .iter()
                .map(|hour| {
                    recent
                        .iter()
                        .filter(|r| keep(r) && &hour_of(r.run.created) == hour)
                        .count() as f64
                })
                .collect()
        };
        let spark_runs = spark(&|_| true);
        let spark_failed = spark(&|r| r.at.state == Status::Failed);
        let by_trigger = [Trigger::Schedule, Trigger::Manual, Trigger::Retry]
            .iter()
            .map(|trigger| {
                (
                    trigger.word().to_owned(),
                    recent.iter().filter(|r| r.run.trigger == *trigger).count() as f64,
                )
            })
            .collect();
        let mut by_job_runs: Vec<(String, f64)> = jobs
            .iter()
            .map(|job| {
                (
                    job.id.clone(),
                    recent.iter().filter(|r| r.run.job == job.id).count() as f64,
                )
            })
            .filter(|(_, n)| *n > 0.0)
            .collect();
        by_job_runs.sort_by(|a, b| b.1.total_cmp(&a.1));
        let succeeded_durations: Vec<i64> = succeeded.iter().map(|r| r.at.duration).collect();
        Day {
            hours,
            per_hour,
            by_job,
            durations,
            success: (finished > 0).then(|| (finished - failed.len()) as f64 / finished as f64 * 100.0),
            failed: failed.len(),
            median: median(&succeeded_durations),
            spark_runs,
            spark_failed,
            by_trigger,
            by_job_runs,
            recent,
        }
    }

    pub fn overview(&mut self, ui: &mut Ui, runs: &[LiveRun]) {
        let now = self.now();
        let prefs = self.prefs();
        let day = self.day(runs, now);
        let going: Vec<LiveRun> = runs.iter().filter(|r| is_live(r.at.state)).cloned().collect();
        let mut trouble: Vec<LiveRun> = Vec::new();
        for run in &day.recent {
            if run.at.state == Status::Failed && !trouble.iter().any(|t| t.run.job == run.run.job) {
                trouble.push(run.clone());
            }
        }
        let mut latest = day.recent.clone();
        latest.sort_by_key(|r| std::cmp::Reverse(r.run.created));
        latest.truncate(8);
        let mut fires: Vec<(String, i64, String)> = self
            .backend
            .jobs
            .iter()
            .filter(|job| !job.paused)
            .filter_map(|job| {
                job.schedule
                    .as_ref()
                    .and_then(|s| next_fire(s, now))
                    .map(|at| (job.id.clone(), at, say_schedule(job.schedule.as_ref())))
            })
            .collect();
        fires.sort_by_key(|f| f.1);
        fires.truncate(5);

        let editing = self.overview_edit.editing();
        let description = if editing {
            "Drag a panel by its grip to move it, set its width or remove it, and drag a widget in to add it."
        } else {
            "The last 24 hours of Tidevann's runs, on the UTC clock."
        };
        let mut kept = prefs.overview.clone().unwrap_or_else(default_layout);
        let base = default_layout();
        let updated = self.overview_view.updated.unwrap_or(now);
        let mut refreshed = false;
        let mut saved = false;
        {
            let (edit, view) = (&mut self.overview_edit, &mut self.overview_view);
            PageHeader::new("Overview").description(description).show(ui, |ui| {
                let outcome = DashboardEditActions::new(edit, &mut kept)
                    .default_layout(&base)
                    .show_with(ui, |ui| {
                        refreshed = RefreshControl::new(&mut view.every, &mut view.paused, &REFRESH)
                            .updated(format!("updated {}", clock(updated, true)))
                            .show(ui)
                            .refresh;
                    });
                saved = outcome.saved;
            });
        }
        if refreshed {
            self.overview_view.updated = Some(now);
        }
        if saved && let Some(mine) = self.backend.prefs.first_mut() {
            mine.overview = Some(kept.clone());
        }
        let shown: Vec<String> = self
            .overview_edit
            .layout(&mut kept)
            .iter()
            .map(|item| item.id.clone())
            .collect();
        let palette: Vec<DashboardWidget> = WIDGETS
            .iter()
            .filter(|w| !shown.iter().any(|s| s == w.0))
            .map(|(id, label, icon, span)| DashboardWidget::new(*id, *label).icon(*icon).span(*span))
            .collect();
        let label = |item: &DashboardLayoutItem| label_of(&item.id).to_owned();
        let mut opened = None;
        let mut go = None;
        let layout = self.overview_edit.layout(&mut kept);
        let arranged = DashboardArrange::new("overview", layout, &label)
            .arranging(editing)
            .palette(&palette)
            .show(ui, |ui, item| {
                panel(
                    ui,
                    &item.id,
                    &day,
                    &going,
                    &trouble,
                    &latest,
                    &fires,
                    now,
                    prefs.times,
                    &mut opened,
                    &mut go,
                );
            });
        if let Some((id, at)) = arranged.added
            && let Some((_, _, _, span)) = WIDGETS.iter().find(|w| w.0 == id)
        {
            let layout = self.overview_edit.layout(&mut kept);
            let at = at.min(layout.len());
            layout.insert(at, DashboardLayoutItem::new(id, *span));
        }
        if let Some(id) = opened {
            self.open(&id);
        }
        if let Some(to) = go {
            self.go(&to);
        }
    }
}

/// One panel of the overview, by its id.
#[allow(clippy::too_many_arguments)]
fn panel(
    ui: &mut Ui,
    id: &str,
    day: &Day,
    going: &[LiveRun],
    trouble: &[LiveRun],
    latest: &[LiveRun],
    fires: &[(String, i64, String)],
    now: i64,
    times: crate::model::Times,
    opened: &mut Option<String>,
    go: &mut Option<String>,
) {
    let title = label_of(id);
    match id {
        "runs" => StatPanel::new(title, Some(day.recent.len() as f64))
            .sparkline(&day.spark_runs)
            .color("chart-2")
            .note("in 24 hours")
            .show(ui),
        "success" => StatPanel::new(title, day.success)
            .unit("%")
            .decimals(1)
            .thresholds(&SUCCESS)
            .note("of those finished")
            .show(ui),
        "failed" => StatPanel::new(title, Some(day.failed as f64))
            .thresholds(&FAILURES)
            .color_mode(StatColorMode::Background)
            .sparkline(&day.spark_failed)
            .show(ui),
        "running" => StatPanel::new(
            title,
            Some(going.iter().filter(|r| r.at.state == Status::Running).count() as f64),
        )
        .color("status-running")
        .show(ui),
        "queued" => StatPanel::new(
            title,
            Some(going.iter().filter(|r| r.at.state == Status::Queued).count() as f64),
        )
        .color("status-queued")
        .show(ui),
        "median" => StatPanel::new(title, day.median.map(|ms| ms as f64 / 1000.0))
            .unit("s")
            .decimals(1)
            .show(ui),
        "per-hour" => {
            let mut chart = BarPanel::new(title, day.hours.clone()).subtitle("by state").stacked(true);
            for (state, counts) in &day.per_hour {
                chart = chart.series(Series::new(state.word(), counts.clone()).color(status_token(*state)));
            }
            chart.show(ui);
        }
        "by-job" => DonutPanel::new(
            title,
            day.by_job
                .iter()
                .enumerate()
                .map(|(i, (job, n))| (job.clone(), *n, chart_token(i))),
        )
        .subtitle("24 hours")
        .total("failed")
        .empty(day.failed == 0)
        .show(ui),
        "durations" => BoxPlotPanel::new(title, day.durations.clone())
            .subtitle("each run against its job's median")
            .info("How long each run that succeeded took, as a share of its job's median: 100% is a usual run, 200% one that took twice as long.")
            .unit("%")
            .empty(day.durations.is_empty())
            .show(ui),
        "by-trigger" => DonutPanel::new(
            title,
            day.by_trigger
                .iter()
                .enumerate()
                .map(|(i, (trigger, n))| (trigger.clone(), *n, chart_token(i))),
        )
        .subtitle("24 hours")
        .total("runs")
        .empty(day.recent.is_empty())
        .show(ui),
        "busiest" => {
            let jobs: Vec<String> = day.by_job_runs.iter().map(|(j, _)| j.clone()).collect();
            BarPanel::new(title, jobs)
                .subtitle("24 hours")
                .horizontal(true)
                .legend(false)
                .series(Series::new("runs", day.by_job_runs.iter().map(|(_, n)| *n)))
                .empty(day.by_job_runs.is_empty())
                .show(ui);
        }
        "trouble" => list_panel(
            ui,
            title,
            &counted(trouble.len() as i64, "job"),
            Some(("All runs", "runs")),
            "No job failed in the last 24 hours.",
            &[
                Column::new("Job", 124.0),
                Column::new("Failed at", 92.0),
                Column::new("What it said", 84.0).fill(),
                Column::new("When", 84.0).numeric(),
            ],
            trouble
                .iter()
                .map(|r| {
                    vec![
                        Cell::Mono(r.run.job.clone()),
                        Cell::Mono(r.at.step.clone().unwrap_or_default()),
                        Cell::Muted(r.run.error.clone().unwrap_or_default()),
                        Cell::Text(when(r.run.created, now, times)),
                    ]
                })
                .collect(),
            trouble.iter().map(|r| r.run.id.clone()).collect(),
            opened,
            go,
        ),
        "going" => list_panel(
            ui,
            title,
            &counted(going.len() as i64, "run"),
            None,
            "Nothing is running. Start a job from Jobs or the palette.",
            &[
                Column::new("Job", 116.0).fill(),
                Column::new("State", 100.0),
                Column::new("Step", 100.0),
                Column::new("So far", 68.0).numeric(),
            ],
            going
                .iter()
                .map(|r| {
                    vec![
                        Cell::Mono(r.run.job.clone()),
                        state_cell(r.at.state),
                        Cell::MutedCode(r.at.step.clone().unwrap_or_else(|| "-".into())),
                        Cell::Text(took(r.at.duration as f64)),
                    ]
                })
                .collect(),
            going.iter().map(|r| r.run.id.clone()).collect(),
            opened,
            go,
        ),
        "latest" => list_panel(
            ui,
            title,
            "the last 24 hours",
            Some(("All runs", "runs")),
            "No run in the last 24 hours.",
            &[
                Column::new("Job", 116.0).fill(),
                Column::new("State", 100.0),
                Column::new("By", 84.0),
                Column::new("When", 84.0).numeric(),
            ],
            latest
                .iter()
                .map(|r| {
                    vec![
                        Cell::Mono(r.run.job.clone()),
                        state_cell(r.at.state),
                        Cell::Muted(r.run.by.word().into()),
                        Cell::Text(when(r.run.created, now, times)),
                    ]
                })
                .collect(),
            latest.iter().map(|r| r.run.id.clone()).collect(),
            opened,
            go,
        ),
        _ => {
            let mut keys = Vec::new();
            let rows: Vec<Vec<Cell>> = fires
                .iter()
                .map(|(job, at, said)| {
                    keys.push(format!("jobs/{job}"));
                    vec![
                        Cell::Mono(job.clone()),
                        Cell::Muted(said.clone()),
                        Cell::Text(clock(*at, false)),
                    ]
                })
                .collect();
            let mut pressed = None;
            list_panel(
                ui,
                title,
                "on the schedules",
                Some(("Schedules", "schedules")),
                "No schedule is on.",
                &[
                    Column::new("Job", 116.0).fill(),
                    Column::new("Schedule", 164.0),
                    Column::new("At", 60.0).numeric(),
                ],
                rows,
                keys,
                &mut pressed,
                go,
            );
            if let Some(to) = pressed {
                *go = Some(to);
            }
        }
    }
}

/// A panel holding a short list, as the web's `DashboardPanel` around a list `DataTable`; a row
/// opens what its key names, a run's id or a page.
#[allow(clippy::too_many_arguments)]
fn list_panel(
    ui: &mut Ui,
    title: &str,
    subtitle: &str,
    action: Option<(&str, &str)>,
    empty: &str,
    columns: &[Column],
    rows: Vec<Vec<Cell>>,
    keys: Vec<String>,
    opened: &mut Option<String>,
    go: &mut Option<String>,
) {
    let mut asked = false;
    let asked_ref = &mut asked;
    let mut panel = DashboardPanel::new(title).subtitle(subtitle).flush(true);
    if let Some(words) = action.map(|(words, _)| words) {
        panel = panel.actions(move |ui| {
            *asked_ref = ui
                .add(Button::new(words).variant(ButtonVariant::Ghost).size(ButtonSize::Xs))
                .clicked();
        });
    }
    panel.show(ui, |ui| {
        if rows.is_empty() {
            let t = sokkel_egui::tokens(ui.ctx());
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(
                    sokkel_egui::TextRole::Body
                        .rich(&t, empty)
                        .color(t.color("muted-foreground")),
                );
            });
            return;
        }
        let mut state = sokkel_egui::TableState::default();
        DataTable::new(("overview-list", title), columns, &rows, &mut state)
            .variant(TableVariant::List)
            .framed(false)
            .fits(true)
            .stack_below(420.0)
            .height(f32::INFINITY)
            .footer(false)
            .show(ui);
        if let Some(row) = state.opened
            && let Some(key) = keys.get(row)
        {
            *opened = Some(key.clone());
        }
    });
    if asked && let Some((_, to)) = action {
        *go = Some(to.into());
    }
}

/// The status color token of a state, for a chart's series.
fn status_token(state: Status) -> &'static str {
    match state {
        Status::Succeeded => "status-succeeded",
        Status::Failed => "status-failed",
        Status::Running => "status-running",
        Status::Queued => "status-queued",
        Status::Cancelled => "status-cancelled",
        Status::Skipped => "status-skipped",
    }
}

/// The series color token at a place, as the charts take them.
fn chart_token(index: usize) -> &'static str {
    sokkel_egui_charts::SERIES_ORDER[index % sokkel_egui_charts::SERIES_ORDER.len()]
}
