//! A run on its own page, as the web's `pages/RunPage.tsx`: what it is and where it stands, its
//! steps as a graph, its log, and what it took in and gave back.

use eframe::egui::Ui;
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, ChipTone, Columns, DescriptionList, Empty, Gap, Icon, JsonTree, LogView,
    PageHeader, Problem, ProblemTone, Section, Stack, TextRole, Timeline, tokens,
};
use sokkel_egui_graph::{GraphCanvas, GraphLink, GraphNode, GraphWords};

use crate::app::{LiveRun, RunsApp};
use crate::logs::{outputs_of, run_log};
use crate::model::{Status, say_by, stamp, took};
use crate::parts::step_events;
use crate::run_panel::run_facts;
use crate::timeline::is_live;

/// The states a run has not reached yet, whose boxes are dashed and whose edges in are quiet.
fn ahead(state: Status) -> bool {
    matches!(state, Status::Queued | Status::Skipped | Status::Cancelled)
}

impl RunsApp {
    pub fn run_page(&mut self, ui: &mut Ui, runs: &[LiveRun], id: &str) {
        let ctx = ui.ctx().clone();
        let t = tokens(&ctx);
        let Some(run) = runs.iter().find(|r| r.run.id == id).cloned() else {
            let mut all = false;
            Empty::new(format!("No run {id}"))
                .description("It may be from before the demo data was reset.")
                .show(ui, |ui| {
                    all = ui
                        .add(
                            Button::new("All runs")
                                .icon(Icon::Search)
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Sm),
                        )
                        .clicked();
                });
            if all {
                self.go("runs");
            }
            return;
        };
        let now = self.now();
        let prefs = self.prefs();
        let job = self.backend.job(&run.run.job).cloned();
        let live = is_live(run.at.state);
        let mut meta = Vec::new();
        if let Some(step) = &run.at.step {
            meta.push(format!("at {step}"));
        }
        if run.at.state != Status::Skipped {
            meta.push(took(run.at.duration as f64));
        }
        meta.push(format!("by {}", say_by(run.run.by, run.run.trigger)));
        meta.push(stamp(run.run.created));
        let mut open_job = false;
        let mut asked = crate::run_panel::Asked::default();
        Stack::new(Gap::Page).show(ui, |ui| {
            PageHeader::new(job.as_ref().map_or(run.run.job.clone(), |j| j.name.clone()))
                .code(&run.run.id)
                .chip(run.at.state.word(), ChipTone::Status(run.at.state.word()))
                .meta(format!("· {}", meta.join(" · ")))
                .meta_row(true)
                .show(ui, |ui| {
                    // the header lays its actions from the end: Cancel, Retry, then Open job before them
                    let live = is_live(run.at.state);
                    let mut cancel = Button::new("Cancel")
                        .icon(Icon::X)
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Sm);
                    if !live {
                        cancel = cancel.reason("Only a run going can be cancelled.");
                    }
                    asked.cancel = ui.add(cancel).clicked();
                    let mut retry = Button::new("Retry")
                        .icon(Icon::RotateCcw)
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm);
                    if live {
                        retry = retry.reason("The run has not finished yet.");
                    }
                    asked.retry = ui.add(retry).clicked();
                    let mut open = Button::new("Open job")
                        .icon(Icon::ListChecks)
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm);
                    if job.is_none() {
                        open = open.reason("The job was deleted.");
                    }
                    open_job = ui.add(open).clicked();
                });
            if run.at.state == Status::Failed {
                let mut problem = Problem::new(
                    ("run-page-failed", &run.run.id),
                    format!("{} failed", run.at.step.as_deref().unwrap_or("A step")),
                )
                .detail("The steps after it were skipped. Retry runs the job again with the same inputs.");
                if let Some(error) = &run.run.error {
                    problem = problem.technical(error);
                }
                problem.show(ui);
            }
            if let Some(skipped) = &run.run.skipped {
                Problem::new(("run-page-skipped", &run.run.id), "The scheduler skipped this run")
                    .tone(ProblemTone::Warning)
                    .detail(skipped)
                    .show(ui);
            }
            let steps_meta = if live {
                "live".to_owned()
            } else {
                format!("{} steps", run.at.steps.len())
            };
            let chosen = self.run_step.clone();
            let mut pressed = None;
            Section::new("Steps").meta(steps_meta).flush(true).show(ui, |ui| {
                let nodes: Vec<GraphNode> = run
                    .at
                    .steps
                    .iter()
                    .map(|step| {
                        let color = t.color(&format!("status-{}", step.status.word()));
                        GraphNode::new(&step.code)
                            .kind(&step.kind)
                            .status(step.status.word(), color)
                            .duration(step.duration_ms.map(|ms| took(ms as f64)))
                            .tint((step.status == Status::Failed).then_some(color))
                            .dashed(ahead(step.status))
                    })
                    .collect();
                let links: Vec<GraphLink> = run
                    .at
                    .steps
                    .iter()
                    .flat_map(|step| {
                        step.needs.iter().map(|need| {
                            let source = run.at.steps.iter().find(|s| &s.code == need).map(|s| s.status);
                            let tint = source
                                .filter(|s| !ahead(*s) && *s != Status::Running)
                                .map(|s| t.color(&format!("status-{}", s.word())));
                            GraphLink::new(need, &step.code)
                                .tint(tint)
                                .dashed(step.status == Status::Skipped)
                        })
                    })
                    .collect();
                pressed = GraphCanvas::new(("run-graph", &run.run.id), &nodes, &links)
                    .selected(chosen.as_deref())
                    .words(GraphWords {
                        label: format!("{} run {}", run.run.job, run.run.id),
                        ..Default::default()
                    })
                    .show(ui);
            });
            if let Some(code) = pressed {
                self.run_step = if chosen.as_ref() == Some(&code) {
                    None
                } else {
                    Some(code)
                };
            }
            if let Some(step) = run.at.steps.iter().find(|s| Some(&s.code) == self.run_step.as_ref()) {
                let mut close = false;
                Section::new(&step.code)
                    .meta(&step.kind)
                    .actions(|ui| {
                        close = ui
                            .add(Button::new("Close").variant(ButtonVariant::Ghost).size(ButtonSize::Xs))
                            .clicked();
                    })
                    .show(ui, |ui| {
                        let mut list = DescriptionList::new().side(112.0).item("State", |ui| {
                            ui.add(
                                sokkel_egui::Chip::new(step.status.word()).tone(ChipTone::Status(step.status.word())),
                            );
                        });
                        list = list.code(
                            "Needs",
                            if step.needs.is_empty() {
                                "nothing".into()
                            } else {
                                step.needs.join(", ")
                            },
                        );
                        if !matches!(step.status, Status::Queued | Status::Skipped) {
                            list = list.text("Started", stamp(step.start));
                        }
                        if let Some(ms) = step.duration_ms {
                            list = list.text("Took", took(ms as f64));
                        }
                        if step.status == Status::Failed
                            && let Some(error) = &run.run.error
                        {
                            list = list.text("Says", error);
                        }
                        list.show(ui);
                    });
                if close {
                    self.run_step = None;
                }
            }
            let lines = run_log(&run.run, &run.at, now);
            let mut log = Section::new("Log").flush(true);
            if live {
                log = log.meta("following");
            }
            log.show(ui, |ui| {
                LogView::new(("run-page-log", &run.run.id), &lines)
                    .follow(prefs.follow_log)
                    .framed(false)
                    .fit(true)
                    .show(ui);
            });
            let name = job.as_ref().map(|j| j.name.clone());
            Columns::new(3)
                .min(240.0)
                .gap(24.0)
                .show(ui, 3, |ui, index, _| match index {
                    0 => Section::new("Run").show(ui, |ui| run_facts(&run, name.as_deref()).show(ui)),
                    1 => Section::new("Inputs").show(ui, |ui| {
                        let inputs = serde_json::Value::Object(run.run.inputs.clone());
                        JsonTree::new(("page-inputs", &run.run.id), &inputs).show(ui);
                    }),
                    _ => Section::new("Outputs").show(ui, |ui| match outputs_of(&run.run, &run.at) {
                        None => {
                            ui.label(
                                TextRole::Body
                                    .rich(&t, "What the run gives back shows once it ends.")
                                    .color(t.color("muted-foreground")),
                            );
                        }
                        Some(outputs) => {
                            JsonTree::new(("page-outputs", &run.run.id), &outputs).show(ui);
                        }
                    }),
                });
            Section::new("Timeline").show(ui, |ui| {
                Timeline::new(step_events(&t, &run)).show(ui);
            });
        });
        if open_job {
            self.go(&format!("jobs/{}", run.run.job));
        }
        if asked.retry {
            self.retry(&ctx, &run.run.id);
        }
        if asked.cancel {
            self.cancel(&ctx, std::slice::from_ref(&run.run.id));
        }
    }
}
