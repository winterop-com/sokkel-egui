//! A run beside the page, as the web's `RunPanel.tsx`: its facts and what can be done with it, its
//! steps, and its log.

use eframe::egui::Ui;
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, DescriptionList, Icon, JsonTree, LogView, Problem, ProblemTone, SidePanel,
    TextRole, Timeline, panel_section, tokens,
};

use crate::app::LiveRun;
use crate::logs::{outputs_of, run_log};
use crate::model::{Prefs, Status, say_by, stamp, thousands, took};
use crate::parts::{state_cell, step_events};
use crate::timeline::is_live;

/// What the panel's buttons asked this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Asked {
    pub page: bool,
    pub retry: bool,
    pub cancel: bool,
}

/// What can be done with a run, as a row of buttons: Open page (with `page`), Retry and Cancel.
pub fn run_buttons(ui: &mut Ui, run: &LiveRun, page: bool) -> Asked {
    let live = is_live(run.at.state);
    let mut asked = Asked::default();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if page {
            asked.page = ui
                .add(
                    Button::new("Open page")
                        .icon(Icon::ExternalLink)
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm),
                )
                .clicked();
        }
        let mut retry = Button::new("Retry")
            .icon(Icon::RotateCcw)
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Sm);
        if live {
            retry = retry.reason("The run has not finished yet.");
        }
        asked.retry = ui.add(retry).clicked();
        let mut cancel = Button::new("Cancel")
            .icon(Icon::X)
            .variant(ButtonVariant::Ghost)
            .size(ButtonSize::Sm);
        if !live {
            cancel = cancel.reason("Only a run going can be cancelled.");
        }
        asked.cancel = ui.add(cancel).clicked();
    });
    asked
}

/// The run's facts: its job, state, step, who started it, when, and how long it took.
pub fn run_facts<'a>(run: &'a LiveRun, name: Option<&str>) -> DescriptionList<'a> {
    let mut list = DescriptionList::new()
        .side(96.0)
        .code("Run", &run.run.id)
        .text(
            "Job",
            match name {
                Some(name) => format!("{name} ({})", run.run.job),
                None => run.run.job.clone(),
            },
        )
        .text("Project", run.run.project.word())
        .item("State", move |ui| {
            if let sokkel_egui::Cell::Chip(label, tone) = state_cell(run.at.state) {
                ui.add(sokkel_egui::Chip::new(label).tone(tone));
            }
        });
    if let Some(step) = &run.at.step {
        list = list.code("Step", step);
    }
    list = list
        .text("Started by", say_by(run.run.by, run.run.trigger))
        .text("Asked for", stamp(run.run.created));
    if run.at.state != Status::Skipped {
        list = list.text("Duration", took(run.at.duration as f64));
    }
    list = list.text("Records", thousands(run.run.records));
    if let Some(of) = &run.run.retry_of {
        list = list.code("Retries", of);
    }
    list
}

/// The panel: Details, Steps and Log.
pub fn run_panel(ui: &mut Ui, run: &LiveRun, name: Option<&str>, prefs: &Prefs, now: i64, tab: &mut usize) -> Asked {
    let mut asked = Asked::default();
    SidePanel::new("run-panel", &["Details", "Steps", "Log"], tab).show(ui, |tab, ui| match tab {
        0 => {
            panel_section(ui, "Run", |ui| {
                asked = run_buttons(ui, run, true);
                if run.at.state == Status::Failed {
                    let mut problem = Problem::new(
                        "run-failed",
                        format!("Failed at {}", run.at.step.as_deref().unwrap_or("a step")),
                    );
                    if let Some(error) = &run.run.error {
                        problem = problem.technical(error);
                    }
                    problem.show(ui);
                }
                if let Some(skipped) = &run.run.skipped {
                    Problem::new("run-skipped", "Skipped")
                        .tone(ProblemTone::Warning)
                        .detail(skipped)
                        .show(ui);
                }
                run_facts(run, name).show(ui);
            });
            panel_section(ui, "Inputs", |ui| {
                let inputs = serde_json::Value::Object(run.run.inputs.clone());
                JsonTree::new(("inputs", &run.run.id), &inputs).show(ui);
            });
            panel_section(ui, "Outputs", |ui| match outputs_of(&run.run, &run.at) {
                None => {
                    let t = tokens(ui.ctx());
                    ui.label(
                        TextRole::Body
                            .rich(&t, "Once the run ends.")
                            .color(t.color("muted-foreground")),
                    );
                }
                Some(outputs) => {
                    JsonTree::new(("outputs", &run.run.id), &outputs).show(ui);
                }
            });
        }
        1 => panel_section(ui, "Steps", |ui| {
            let t = tokens(ui.ctx());
            Timeline::new(step_events(&t, run)).show(ui);
        }),
        _ => {
            let lines = run_log(&run.run, &run.at, now);
            egui_margin(ui, |ui| {
                LogView::new(("run-log", &run.run.id), &lines)
                    .follow(prefs.follow_log)
                    .numbered(false)
                    .show(ui);
            });
        }
    });
    asked
}

/// 12 around the log, as the web's `m-3`.
fn egui_margin(ui: &mut Ui, body: impl FnOnce(&mut Ui)) {
    eframe::egui::Frame::new()
        .inner_margin(eframe::egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui);
        });
}
