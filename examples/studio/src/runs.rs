//! The runs started in the studio and one run's page, as the web's `pages/Runs.tsx`.

use eframe::egui::Ui;
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, Cell, ChipTone, Column, DataTable, Empty, Icon, PageHeader, Problem, SplitPane,
    TableWords,
};
use sokkel_egui_graph::{RunGraph, RunStep};

use crate::app::StudioApp;
use crate::model::{Run, clock, counted, say_duration};
use crate::work::{pane, titled};

/// How long a run took, or that it is still going.
fn took(run: &Run) -> String {
    match run.finished {
        None => "running".into(),
        Some(end) => say_duration((end - run.started) as f64),
    }
}

/// A run's state as a chip.
fn state_cell(run: &Run) -> Cell {
    let word = run.state.name();
    Cell::Chip(word.into(), ChipTone::Status(word))
}

impl StudioApp {
    /// Every run started in the studio, newest first; a running one moves as it goes.
    pub fn runs_page(&mut self, ui: &mut Ui) {
        let now = self.now();
        let runs: Vec<Run> = self.backend.runs_newest().into_iter().cloned().collect();
        PageHeader::new("Runs")
            .count(runs.len().to_string())
            .description("The pipelines run from the studio, each with its steps and its log.")
            .show(ui, |_| {});
        if runs.is_empty() {
            let first = self
                .backend
                .files
                .iter()
                .find(|f| f.kind == crate::model::FileKind::Pipeline)
                .cloned();
            let mut open = false;
            ui.add_space((ui.available_height() / 2.0 - 76.0).max(0.0));
            Empty::new("No runs yet")
                .description("Open a pipeline and press Run, or Cmd/Ctrl+Enter in its editor.")
                .show(ui, |ui| {
                    if let Some(first) = &first {
                        open = ui
                            .add(
                                Button::new(format!("Open {}", first.name))
                                    .icon(Icon::FileCode)
                                    .variant(ButtonVariant::Outline)
                                    .size(ButtonSize::Sm),
                            )
                            .clicked();
                    }
                });
            if open && let Some(first) = first {
                self.open_tab(&first.id);
            }
            return;
        }
        let mut query = self.queries.remove("runs").unwrap_or_default();
        let typed = query.trim().to_lowercase();
        let shown: Vec<&Run> = runs
            .iter()
            .filter(|run| {
                typed.is_empty()
                    || format!(
                        "{} {} {} {}",
                        run.pipeline,
                        run.project.name(),
                        run.state.name(),
                        run.failed.as_ref().map_or("", |f| f.step.as_str())
                    )
                    .contains(&typed)
            })
            .collect();
        let columns = vec![
            Column::new("Run", 80.0).sortable(true),
            Column::new("Pipeline", 160.0).fill().sortable(true),
            Column::new("Project", 112.0).sortable(true),
            Column::new("State", 128.0).sortable(true),
            Column::new("Failed at", 112.0).sortable(true),
            Column::new("Started", 96.0).numeric().sortable(true),
            Column::new("Took", 96.0).numeric().sortable(true),
            Column::new("From", 112.0),
        ];
        let rows: Vec<Vec<Cell>> = shown
            .iter()
            .map(|run| {
                vec![
                    Cell::Number {
                        value: run.number as f64,
                        text: run.number.to_string(),
                    },
                    Cell::Text(run.pipeline.clone()),
                    Cell::Text(run.project.name().into()),
                    state_cell(run),
                    Cell::Text(run.failed.as_ref().map_or(String::new(), |f| f.step.clone())),
                    Cell::Number {
                        value: run.started as f64,
                        text: clock(run.started),
                    },
                    Cell::Number {
                        value: (run.finished.unwrap_or(now) - run.started) as f64,
                        text: took(run),
                    },
                    Cell::Text(if run.draft { "unsaved" } else { "saved" }.into()),
                ]
            })
            .collect();
        let keys: Vec<String> = shown.iter().map(|r| r.id.clone()).collect();
        let state = self.tables.entry("runs".into()).or_default();
        DataTable::new("studio-runs", &columns, &rows, state)
            .keys(keys.iter())
            .search(&mut query)
            .words(TableWords {
                rows: "{count} runs".into(),
                row: "{count} run".into(),
                ..Default::default()
            })
            .footer(false)
            .empty(|ui| {
                Empty::new("No run matches.").show(ui, |_| {});
            })
            .height(f32::INFINITY)
            .show(ui);
        let opened = state.opened.take();
        self.queries.insert("runs".into(), query);
        if let Some(id) = opened.and_then(|row| keys.get(row)) {
            self.go(&format!("runs/{id}"));
        }
    }

    /// One run: how it stands, its steps as a graph and its log with the failing step marked.
    pub fn run_page(&mut self, ui: &mut Ui, id: &str) {
        let now = self.now();
        let Some(run) = self.backend.runs.iter().find(|r| r.id == id).cloned() else {
            let mut all = false;
            Empty::new("No such run")
                .description("It may be from before the demo data was reset.")
                .show(ui, |ui| {
                    all = ui
                        .add(
                            Button::new("All runs")
                                .icon(Icon::History)
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
        let file = self.backend.file(&run.file).cloned();
        let going = run.state.name() == "running";
        let gone = "The pipeline was deleted.";
        let (mut open, mut again, mut cancel) = (false, false, false);
        let meta = [
            run.state.name().to_owned(),
            run.project.name().to_owned(),
            format!("started {} by {}", clock(run.started), run.by),
            took(&run),
            if run.draft {
                "from unsaved changes"
            } else {
                "from the saved file"
            }
            .to_owned(),
        ]
        .join(" \u{00b7} ");
        PageHeader::new(&run.pipeline)
            .code(format!("run {}", run.number))
            .meta(meta)
            .show(ui, |ui| {
                if going {
                    cancel = ui
                        .add(
                            Button::new("Cancel")
                                .icon(Icon::Square)
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Sm),
                        )
                        .clicked();
                } else {
                    let mut button = Button::new("Run again").icon(Icon::Play).size(ButtonSize::Sm);
                    if file.is_none() {
                        button = button.reason(gone);
                    }
                    again = ui.add(button).clicked() && file.is_some();
                }
                let mut button = Button::new("Open pipeline")
                    .icon(Icon::FileCode)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Sm);
                if file.is_none() {
                    button = button.reason(gone);
                }
                open = ui.add(button).clicked() && file.is_some();
            });
        if let Some(failed) = &run.failed {
            Problem::new(("run-failed", &run.id), format!("{} failed", failed.step))
                .detail("The steps after it were skipped; notify steps still ran.")
                .technical(&failed.reason)
                .show(ui);
        }
        let steps: Vec<RunStep> = run
            .steps
            .iter()
            .map(|s| {
                let mut step = RunStep::new(&s.id, s.state.name()).kind(&s.kind).needs(s.needs.clone());
                if let Some(ms) = s.duration_ms {
                    step = step.duration_ms(Some(ms as f64));
                }
                step
            })
            .collect();
        let mut chosen = self.views.entry(format!("run:{id}")).or_default().selected.clone();
        let label = format!("{} run {}", run.pipeline, run.number);
        SplitPane::new("studio-run").down(true).default_share(0.45).show(
            ui,
            |ui| {
                pane(
                    ui,
                    |ui| titled(ui, "Steps"),
                    Some(&if going {
                        "live".to_owned()
                    } else {
                        counted(run.steps.len() as f64, "step", None)
                    }),
                    |_| {},
                    |ui| {
                        let out = RunGraph::new(("run-graph", id), &steps)
                            .selected(chosen.as_deref())
                            .words(sokkel_egui_graph::GraphWords {
                                label: label.clone(),
                                ..Default::default()
                            })
                            .show(ui);
                        if let Some(pressed) = out.pressed {
                            chosen = Some(pressed);
                        }
                    },
                );
            },
            |ui| {
                pane(
                    ui,
                    |ui| titled(ui, "Log"),
                    Some(&counted(run.log.len() as f64, "line", None)),
                    |_| {},
                    |ui| self.run_log(ui, &run),
                );
            },
        );
        self.views.entry(format!("run:{id}")).or_default().selected = chosen;
        if open && let Some(file) = &file {
            self.open_tab(&file.id);
        }
        if cancel {
            self.backend.cancel(&run.id, now);
        }
        if again
            && let Some(file) = &file
            && let Some(Ok(made)) = self.backend.run(&file.id, now)
        {
            self.go(&format!("runs/{}", made.id));
        }
    }
}
