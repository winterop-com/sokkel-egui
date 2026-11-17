//! The goals people set, as the web's `Goals.tsx`: each read live, how far along, whether it is on
//! pace, and its line toward the target.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, ConfirmAnswer, ConfirmDialog, DashboardGrid, DateField, Day, Dialog, Empty,
    Field, Icon, IconButton, NumberField, PageHeader, PanelSpan, Select, StatPanel, TextField, Toast, tokens,
};
use sokkel_egui_charts::{Series, TimeSeriesPanel};

use crate::app::AnalyticsApp;
use crate::days::{civil, day_from, day_of, long_day, short_day, start_of};
use crate::metrics::{Figures, Segment};
use crate::model::{Goal, METRICS, Metric, PLANS, Person, REGIONS};
use crate::world::World;

/// The demo's goals, as the web's `goals` seeds them around the day of `now`.
pub fn seed_goals(now: i64) -> Vec<Goal> {
    let today = day_of(now);
    let (year, month, _) = civil(today);
    let quarter_end = day_from(year, (month - 1) / 3 * 3 + 4, 0);
    let month_start = day_from(year, month, 1);
    let month_end = day_from(year, month + 1, 0);
    use crate::model::{Plan, Region};
    vec![
        Goal {
            id: "mrr-quarter".into(),
            name: "MRR this quarter".into(),
            metric: Metric::Mrr,
            target: 615_000.0,
            start: today - 40,
            due: quarter_end,
            plan: None,
            region: None,
            by: Person::Margaret,
        },
        Goal {
            id: "business-europe".into(),
            name: "Business teams in Europe".into(),
            metric: Metric::Paying,
            target: 210.0,
            start: today - 60,
            due: quarter_end,
            plan: Some(Plan::Business),
            region: Some(Region::Europe),
            by: Person::Ada,
        },
        Goal {
            id: "signups-month".into(),
            name: "Sign-ups this month".into(),
            metric: Metric::Signups,
            target: 700.0,
            start: month_start,
            due: month_end,
            plan: None,
            region: None,
            by: Person::Grace,
        },
    ]
}

/// A goal's figure at each of the moments, which come in order: a level such as MRR then, or a
/// sum such as revenue since the goal began.
fn figures(world: &World, goal: &Goal, moments: &[i64]) -> Vec<f64> {
    let figures = Figures::new(world);
    let segment = Segment {
        plan: goal.plan,
        region: goal.region,
    };
    if !goal.metric.sums() {
        let level = figures.levels(segment, moments);
        let read = match goal.metric {
            Metric::Mrr => level.mrr,
            Metric::Paying => level.paying,
            _ => level.active,
        };
        return read.into_iter().map(|v| v as f64).collect();
    }
    let mut sum = 0.0;
    let mut last = start_of(goal.start);
    moments
        .iter()
        .map(|&at| {
            if at > last {
                sum += if goal.metric == Metric::Revenue {
                    figures.revenue(segment, last, at) as f64
                } else {
                    figures.flows(segment, last, at).signups as f64
                };
                last = at;
            }
            sum
        })
        .collect()
}

/// Where a goal stands: where it began, where it is, how far along, where it should be by now,
/// and its line from start to due.
pub struct Progress {
    pub began: f64,
    pub current: f64,
    pub share: f64,
    pub projected: f64,
    pub on_track: bool,
    pub done: bool,
    pub over: bool,
    pub labels: Vec<String>,
    pub actual: Vec<Option<f64>>,
    pub pace: Vec<f64>,
}

pub fn progress(world: &World, goal: &Goal) -> Progress {
    let (first, last) = (goal.start, goal.due);
    let (from, to) = (start_of(first), start_of(last + 1));
    let now = world.now.clamp(from, to.max(from));
    let step = if last - first > 120 { 7 } else { 1 };
    let days: Vec<i64> = (first..=last).step_by(step as usize).collect();
    let ends: Vec<i64> = days
        .iter()
        .filter(|&&day| start_of(day) <= world.now)
        .map(|&day| start_of(day + step).min(now))
        .collect();
    let mut moments = vec![from];
    moments.extend(&ends);
    moments.push(now);
    let read = figures(world, goal, &moments);
    let began = if goal.metric.sums() { 0.0 } else { read[0] };
    let current = *read.last().unwrap_or(&0.0);
    let share = if goal.target == began {
        1.0
    } else {
        (current - began) / (goal.target - began)
    };
    let elapsed = ((now - from) as f64 / (to - from) as f64).clamp(0.0, 1.0);
    let expected = began + (goal.target - began) * elapsed;
    let projected = if elapsed == 0.0 {
        current
    } else {
        began + (current - began) / elapsed
    };
    let count = days.len();
    Progress {
        began,
        current,
        share,
        projected,
        on_track: current >= expected,
        done: current >= goal.target,
        over: world.now > to,
        labels: days.iter().map(|&d| short_day(d)).collect(),
        actual: (0..count)
            .map(|i| read.get(i + 1).copied().filter(|_| i < ends.len()))
            .collect(),
        pace: (0..count)
            .map(|i| began + (goal.target - began) * ((i + 1) as f64 / count as f64).min(1.0))
            .collect(),
    }
}

/// A goal being set or changed, as the dialog holds it.
#[derive(Clone, Debug)]
pub struct Draft {
    pub of: Option<String>,
    pub name: String,
    pub metric: usize,
    /// In the person's currency for money, as it is typed.
    pub target: f64,
    pub plan: usize,
    pub region: usize,
    pub start: Option<Day>,
    pub due: Option<Day>,
    pub tried: bool,
}

/// The goals page: the goals, the one in the dialog, and a delete asked.
pub struct GoalsPage {
    pub goals: Vec<Goal>,
    pub editing: Option<Draft>,
    pub deleting: Option<String>,
    made: u32,
}

impl GoalsPage {
    pub fn new(now: i64) -> Self {
        Self {
            goals: seed_goals(now),
            editing: None,
            deleting: None,
            made: 0,
        }
    }
}

fn money_metric(metric: Metric) -> bool {
    matches!(metric, Metric::Mrr | Metric::Revenue)
}

impl AnalyticsApp {
    pub fn goals_page(&mut self, ui: &mut Ui) {
        let mut new = false;
        PageHeader::new("Goals")
            .description("Targets read live as the figures move.")
            .show(ui, |ui| {
                if ui.add(Button::new("New").icon(Icon::Plus)).clicked() {
                    new = true;
                }
            });
        let money = self.money();
        let good = tokens(ui.ctx()).color("good");
        let goals = self.goals.goals.clone();
        let read: Vec<(Goal, Progress)> = goals
            .into_iter()
            .map(|g| {
                let at = progress(&self.world, &g);
                (g, at)
            })
            .collect();
        // a pencil or a bin pressed in a goal's head: its id, and whether it is to be changed
        let asked: std::cell::RefCell<Option<(String, bool)>> = std::cell::RefCell::new(None);
        if read.is_empty() {
            Empty::new("No goals yet")
                .description("Set a target for MRR, revenue, teams or sign-ups, and see it fill as the figures move.")
                .show(ui, |ui| {
                    if ui
                        .add(
                            Button::new("New goal")
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Sm),
                        )
                        .clicked()
                    {
                        new = true;
                    }
                });
        } else {
            let asked = &asked;
            DashboardGrid::new("goals").show(ui, |cells| {
                for (goal, at) in &read {
                    let metric = goal.metric;
                    let write = move |v: f64| {
                        if money_metric(metric) {
                            money.short(v)
                        } else {
                            sokkel_egui::format_value(v.round(), Some(0))
                        }
                    };
                    let state = if at.done {
                        "reached"
                    } else if at.over {
                        "missed"
                    } else if at.on_track {
                        "on pace"
                    } else {
                        "behind pace"
                    };
                    let fine = at.done || at.on_track;
                    cells.cell(PanelSpan::Third, move |ui| {
                        StatPanel::new(&goal.name, Some(at.current))
                            .format(&write)
                            .color(if fine { "good" } else { "warning" })
                            .target(
                                goal.target,
                                Some((at.began.min(goal.target), goal.target.max(at.current))),
                            )
                            .delta(format!("{:.0}% of the way", at.share.max(0.0) * 100.0))
                            .good(Some(fine))
                            .note(format!("Due {}, {state}", short_day(goal.due)))
                            .show(ui);
                    });
                    let segment: Vec<&str> = [goal.plan.map(|p| p.name()), goal.region.map(|r| r.name())]
                        .into_iter()
                        .flatten()
                        .collect();
                    let segment = if segment.is_empty() {
                        String::new()
                    } else {
                        format!(" of {}", segment.join(", "))
                    };
                    cells.cell(PanelSpan::TwoThirds, move |ui| {
                        TimeSeriesPanel::new(&goal.name, at.labels.clone())
                            .subtitle(format!("heading for {}", write(at.projected)))
                            .info(format!(
                                "{}{segment}, {} to {}. The dashed line is the pace that reaches the target on the day it is due.",
                                goal.metric.name(),
                                long_day(goal.start),
                                long_day(goal.due)
                            ))
                            .series(
                                Series::gappy(goal.metric.name(), at.actual.clone())
                                    .color(if fine { "chart-2" } else { "warning" }),
                            )
                            .series(Series::new("Pace to the target", at.pace.clone()).color("muted-foreground").dashed())
                            .mark(goal.target, good)
                            .format(write)
                            .height(160.0)
                            .show_with(
                                ui,
                                Some(|ui: &mut Ui| {
                                    // laid out from the end: the bin last, as the web has it
                                    if ui.add(IconButton::new(Icon::Trash2, format!("Delete {}", goal.name)).xs()).clicked() {
                                        *asked.borrow_mut() = Some((goal.id.clone(), false));
                                    }
                                    if ui.add(IconButton::new(Icon::Pencil, format!("Change {}", goal.name)).xs()).clicked() {
                                        *asked.borrow_mut() = Some((goal.id.clone(), true));
                                    }
                                }),
                            );
                    });
                }
            });
        }
        let today = Day(day_of(self.world.now) as i32);
        let rate = self.prefs.currency.rate();
        if new {
            self.goals.editing = Some(Draft {
                of: None,
                name: String::new(),
                metric: 0,
                target: 0.0,
                plan: 0,
                region: 0,
                start: Some(today),
                due: Some(Day(today.0 + 90)),
                tried: false,
            });
        }
        match asked.into_inner() {
            Some((id, true)) => {
                if let Some(goal) = self.goals.goals.iter().find(|g| g.id == id) {
                    self.goals.editing = Some(Draft {
                        of: Some(goal.id.clone()),
                        name: goal.name.clone(),
                        metric: METRICS.iter().position(|m| *m == goal.metric).unwrap_or(0),
                        target: if money_metric(goal.metric) {
                            (goal.target * rate).round()
                        } else {
                            goal.target
                        },
                        plan: goal
                            .plan
                            .map_or(0, |p| PLANS.iter().position(|q| *q == p).map_or(0, |i| i + 1)),
                        region: goal
                            .region
                            .map_or(0, |r| REGIONS.iter().position(|q| *q == r).map_or(0, |i| i + 1)),
                        start: Some(Day(goal.start as i32)),
                        due: Some(Day(goal.due as i32)),
                        tried: false,
                    });
                }
            }
            Some((id, false)) => self.goals.deleting = Some(id),
            None => {}
        }
        self.goal_dialog(ui.ctx(), today);
        self.goal_delete(ui.ctx());
    }

    /// Sets a goal up, or changes one: what it counts, for which segment, the target and its days.
    fn goal_dialog(&mut self, ctx: &egui::Context, today: Day) {
        let Some(mut draft) = self.goals.editing.take() else {
            return;
        };
        let rate = self.prefs.currency.rate();
        let metric = METRICS[draft.metric];
        let name_error = draft.name.trim().is_empty().then_some("A goal needs a name.");
        let target_error = (draft.target <= 0.0).then_some("A goal needs a target above zero.");
        let due_error = match (draft.start, draft.due) {
            (Some(start), Some(due)) if due <= start => Some("It is due after it starts."),
            (Some(_), Some(_)) => None,
            _ => Some("A goal needs its days."),
        };
        let plan = draft.plan.checked_sub(1).map(|i| PLANS[i]);
        let region = draft.region.checked_sub(1).map(|i| REGIONS[i]);
        let from = draft.start.map_or(day_of(self.world.now), |d| d.0 as i64);
        let probe = Goal {
            id: String::new(),
            name: draft.name.clone(),
            metric,
            target: 0.0,
            start: from,
            due: draft.due.map_or(from, |d| d.0 as i64),
            plan,
            region,
            by: Person::Ada,
        };
        let now = figures(&self.world, &probe, &[start_of(from).max(self.world.now)])[0];
        let now = if money_metric(metric) { now * rate } else { now };
        let title = match &draft.of {
            None => "New goal".to_owned(),
            Some(id) => format!(
                "Change {}",
                self.goals
                    .goals
                    .iter()
                    .find(|g| g.id == *id)
                    .map_or("", |g| g.name.as_str())
            ),
        };
        let mut confirmed = false;
        let mut cancelled = false;
        let tried = draft.tried;
        let closed = Dialog::new("goal-dialog", title)
            .description("Progress is read live, from the day it starts to the day it is due.")
            .show(
                ctx,
                |ui| {
                    ui.spacing_mut().item_spacing.y = 16.0;
                    ui.add(
                        TextField::new("Name", &mut draft.name)
                            .placeholder("MRR this quarter")
                            .error(name_error.filter(|_| tried)),
                    );
                    Field::new("Counts")
                        .description(if metric.sums() {
                            "Added up from the day it starts."
                        } else {
                            "The level on the day it is read."
                        })
                        .show(ui, |ui| {
                            let width = ui.available_width();
                            ui.add(Select::new(&mut draft.metric, METRICS.map(Metric::name), "Counts").width(width))
                        });
                    // the number field labels itself; the figure now goes under it, as the web's hint
                    ui.scope(|ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        let mut target = NumberField::new("goal-target", "Target", &mut draft.target);
                        if money_metric(metric) {
                            target = target.suffix(self.prefs.currency.code());
                        }
                        ui.add(target);
                        let t = tokens(ui.ctx());
                        ui.label(
                            sokkel_egui::TextRole::Help
                                .rich(&t, format!("now {}", sokkel_egui::format_value(now.round(), Some(0))))
                                .color(t.color("muted-foreground")),
                        );
                    });
                    if tried && let Some(error) = target_error {
                        ui.label(
                            sokkel_egui::TextRole::Help
                                .rich(&tokens(ui.ctx()), error)
                                .color(tokens(ui.ctx()).color("critical-ink")),
                        );
                    }
                    Field::new("Plan").show(ui, |ui| {
                        let width = ui.available_width();
                        let names = std::iter::once("All plans").chain(PLANS.map(|p| p.name()));
                        ui.add(Select::new(&mut draft.plan, names, "Plan").width(width))
                    });
                    Field::new("Region").show(ui, |ui| {
                        let width = ui.available_width();
                        let names = std::iter::once("Everywhere").chain(REGIONS.map(|r| r.name()));
                        ui.add(Select::new(&mut draft.region, names, "Region").width(width))
                    });
                    DateField::new("goal-start", "Starts", &mut draft.start, today).show(ui);
                    DateField::new("goal-due", "Due", &mut draft.due, today)
                        .error(due_error.filter(|_| tried))
                        .show(ui);
                },
                |ui| {
                    let label = if draft.of.is_none() { "Set goal" } else { "Save" };
                    if ui.add(Button::new(label)).clicked() {
                        confirmed = true;
                    }
                    if ui.add(Button::new("Cancel").variant(ButtonVariant::Outline)).clicked() {
                        cancelled = true;
                    }
                },
            );
        if confirmed {
            draft.tried = true;
            if let (None, None, None, Some(start), Some(due)) =
                (name_error, target_error, due_error, draft.start, draft.due)
            {
                let id = draft.of.clone().unwrap_or_else(|| {
                    self.goals.made += 1;
                    format!("goal-{}", self.goals.made)
                });
                let goal = Goal {
                    id: id.clone(),
                    name: draft.name.trim().to_owned(),
                    metric,
                    target: if money_metric(metric) {
                        draft.target / rate
                    } else {
                        draft.target
                    },
                    start: start.0 as i64,
                    due: due.0 as i64,
                    plan,
                    region,
                    by: Person::Ada,
                };
                let said = if draft.of.is_none() { "set" } else { "changed" };
                Toast::success(format!("{} {said}", goal.name))
                    .description(format!("Due {}", long_day(goal.due)))
                    .show(ctx);
                match self.goals.goals.iter_mut().find(|g| g.id == id) {
                    Some(known) => *known = goal,
                    None => self.goals.goals.push(goal),
                }
                return;
            }
        }
        if !(closed || cancelled) {
            self.goals.editing = Some(draft);
        }
    }

    fn goal_delete(&mut self, ctx: &egui::Context) {
        let Some(id) = self.goals.deleting.clone() else { return };
        let Some(goal) = self.goals.goals.iter().find(|g| g.id == id).cloned() else {
            self.goals.deleting = None;
            return;
        };
        let answer = ConfirmDialog::new("delete-goal", format!("Delete {}?", goal.name), "Delete")
            .detail("The goal goes for everyone; the figures it reads stay as they are.")
            .destructive(true)
            .show(ctx);
        match answer {
            Some(ConfirmAnswer::Confirm) => {
                self.goals.deleting = None;
                self.goals.goals.retain(|g| g.id != id);
                Toast::new(format!("{} deleted", goal.name)).show(ctx);
            }
            Some(_) => self.goals.deleting = None,
            None => {}
        }
    }
}
