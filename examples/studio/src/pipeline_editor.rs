//! A pipeline: its YAML beside its steps as a graph that edits them, as the web's
//! `editors/PipelineEditor.tsx`, `PipelineCanvas.tsx` and `StepSettings.tsx`.

use std::cell::RefCell;
use std::collections::HashMap;

use eframe::egui::{self, Pos2, Rect, Ui};
use serde_json::{Map, Value};
use sokkel_egui::{
    ActiveLine, Button, ButtonSize, ButtonVariant, Chip, ChipTone, CodeCompletion, CodeEditor, CodeLanguage, CodeMark,
    CodeVariant, Empty, Icon, MarkTone, MenuItem, SchemaForm, Section, Segmented, SplitPane, TextField, TextRole,
    WindowedPaneWhere, tokens,
};
use sokkel_egui_graph::{
    BlockGroup, BlockPicker, BlockPickerWords, GraphAnchor, GraphControlButton, GraphEditing, GraphEvent, GraphHistory,
    GraphMenu, GraphNotice, GraphOverlay, GraphToolbar, GraphWords, HistoryKey, NodeIssue, NoticeTone, PipelineBlock,
    PipelineGraph, PipelineStep, RunGraph, RunStep, cycle_path, history_keys,
};

use crate::app::StudioApp;
use crate::model::{PipelineView, WorkFile, counted};
use crate::pipeline::{
    Pipeline, RUN_FIELDS, STEP_FIELDS, STEP_KINDS, Tone, describe_word, holds_reference, kind_docs, pipeline_schema,
    read_pipeline, reference_words,
};
use crate::pipeline_edit::{
    add_step, connect, disconnect, is_step_id, remove_steps, rename_step, set_step_fields, step_fields,
};
use crate::work::{code_size, editor_pane, pane, problems_list, titled};

/// Each kind of step's glyph, on its box and in the picker.
pub fn kind_icon(kind: &str) -> Option<Icon> {
    Some(match kind {
        "fetch" => Icon::Download,
        "transform" => Icon::Shuffle,
        "validate" => Icon::ShieldCheck,
        "upload" => Icon::Upload,
        "notify" => Icon::Bell,
        _ => return None,
    })
}

/// The group a kind is listed under in the picker.
fn group_of(kind: &str) -> &'static str {
    match kind {
        "fetch" => "read",
        "upload" | "notify" => "write",
        _ => "shape",
    }
}

/// Where a step picked from the picker goes.
#[derive(Clone, Debug)]
pub enum Picker {
    After(String),
    Insert {
        source: String,
        target: String,
        edge: String,
    },
    /// A connection from `source` let go on the ground at `at`, in the graph's coordinates.
    Drop {
        source: String,
        at: Pos2,
    },
    /// The ground right-clicked at `at`.
    Here(Pos2),
    /// The add button, or the empty canvas's: after the chosen step, if one is.
    Button {
        rect: Rect,
        after: Option<String>,
    },
}

/// What the pipeline's graph keeps between frames: the picker, the notice, and its undo.
#[derive(Default)]
pub struct CanvasState {
    pub picker: Option<Picker>,
    pub notice: Option<(bool, String)>,
    pub history: Option<GraphHistory<String>>,
    /// The toolbar's undo and redo pressed this frame.
    pub asked: (bool, bool),
}

/// How far a box goes from the one it follows, across and down.
const GAP_ACROSS: f32 = 64.0;
const GAP_DOWN: f32 = 48.0;
/// How far a box's corner sits from the point it was put at, so the box lands under the pointer.
const DROP_OFFSET: egui::Vec2 = egui::vec2(16.0, 22.0);

/// What a step of a kind is set with: its retries and timeout, and the settings its kind takes.
fn settings_schema(kind: &str) -> Value {
    let schema = pipeline_schema();
    let step = &schema["$defs"]["step"];
    let with_title = |value: &Value, title: &str| {
        let mut value = value.clone();
        if let Value::Object(map) = &mut value {
            map.insert("title".into(), Value::String(title.into()));
        }
        value
    };
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "retries": with_title(&step["properties"]["retries"], "Retries"),
            "timeout": with_title(&step["properties"]["timeout"], "Timeout"),
            "with": with_title(&schema["$defs"][kind], "Settings"),
        }
    })
}

/// The code editor's words from the pipeline's references.
fn completions(pipeline: &Pipeline) -> Vec<CodeCompletion> {
    reference_words(pipeline)
        .into_iter()
        .map(|w| CodeCompletion {
            label: w.label,
            kind: Some(w.kind.into()),
            detail: Some(w.detail),
            info: w.docs,
            apply: None,
        })
        .collect()
}

/// What a pipeline is written with: the kinds of step, and what a reference can name.
fn reference(ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let list = |ui: &mut Ui, items: &mut dyn Iterator<Item = (&str, &str)>| {
        for (term, value) in items {
            ui.label(TextRole::Label.rich(&t, term));
            ui.label(TextRole::Body.rich(&t, value).color(t.color("muted-foreground")));
            ui.add_space(8.0);
        }
    };
    Section::new("Step kinds").flat(true).show(ui, |ui| {
        list(ui, &mut STEP_KINDS.iter().map(|k| (*k, kind_docs(k).unwrap_or(""))));
    });
    Section::new("Step fields")
        .flat(true)
        .description("Named as ${steps.<id>.<field>}.")
        .show(ui, |ui| list(ui, &mut STEP_FIELDS.iter().copied()));
    Section::new("Run fields")
        .flat(true)
        .description("Named as ${run.<field>}.")
        .show(ui, |ui| list(ui, &mut RUN_FIELDS.iter().copied()));
}

impl StudioApp {
    /// A pipeline: its YAML checked and completed against the pipeline schema as it is typed, the
    /// references checked beside it, the problems listed, and a run with its graph and log. The
    /// steps are edited as text, on a graph, or both; each edit on the graph is written into the
    /// YAML, and one undo serves both.
    pub fn pipeline_editor(&mut self, ui: &mut Ui, file: &WorkFile) {
        let id = file.id.clone();
        let now = self.now();
        let text = self.backend.doc_for(&id).unwrap_or_default().to_owned();
        let pipeline = read_pipeline(&text);
        let unsaved = self.backend.dirty().contains(&id);
        let prefs = self.backend.prefs.clone();
        let view_mode = prefs.pipeline_view;
        let latest = self.backend.runs_newest().into_iter().find(|r| r.file == id).cloned();
        let going = latest.as_ref().is_some_and(|r| r.state.name() == "running");
        // a pipeline opens on its problems under the graph
        let mut view = self.views.remove(&id).unwrap_or_else(|| crate::work::FileView {
            below: 1,
            ..Default::default()
        });
        let mut canvas = self.canvases.remove(&id).unwrap_or_default();
        let history = canvas.history.get_or_insert_with(|| GraphHistory::new(text.clone()));
        if history.value() != &text {
            history.set(text.clone(), Some("type"));
        }
        let errors = view.problems.iter().filter(|p| p.tone == MarkTone::Error).count();
        let run_reason = if going {
            None
        } else if errors > 0 {
            Some(format!("Fix the {} first.", counted(errors as f64, "error", None)))
        } else if pipeline.broken {
            Some("The YAML does not read.".to_owned())
        } else {
            None
        };
        // the line a run failed on, washed in the editor while the step is still there
        let mut marks: Vec<CodeMark> = pipeline
            .marks
            .iter()
            .map(|m| CodeMark {
                line: None,
                range: Some((m.from, m.to)),
                path: None,
                tone: if m.tone == Tone::Error {
                    MarkTone::Error
                } else {
                    MarkTone::Warning
                },
                message: Some(m.message.clone()),
            })
            .collect();
        if let Some(failed) = latest.as_ref().and_then(|r| r.failed.as_ref())
            && let Some(step) = pipeline.steps.iter().find(|s| s.id == failed.step)
        {
            let line = text[..step.from.min(text.len())].matches('\n').count() + 1;
            marks.push(CodeMark {
                line: Some(line),
                range: None,
                path: None,
                tone: MarkTone::Error,
                message: None,
            });
        }
        let words = completions(&pipeline);
        // what the checks found within each step's lines, marked on its box
        let issues: HashMap<String, Vec<NodeIssue>> = pipeline
            .steps
            .iter()
            .filter_map(|step| {
                let within: Vec<NodeIssue> = view
                    .problems
                    .iter()
                    .filter(|p| p.from >= step.from && p.from < step.to)
                    .map(|p| {
                        if p.tone == MarkTone::Error {
                            NodeIssue::critical(&p.message)
                        } else {
                            NodeIssue::warning(&p.message)
                        }
                    })
                    .collect();
                (!within.is_empty()).then(|| (step.id.clone(), within))
            })
            .collect();
        let reveal = view.reveal.take();
        let mut popped = view.popped;
        // a step is chosen on the graph, so the text alone has no step to set
        let mut below = view.below;
        if view_mode == PipelineView::Text && below == 0 {
            below = 1;
        }
        let positions = layout_of(&self.backend, &id);
        // what the panes change while they are drawn, read by each in turn
        let st = RefCell::new(Local {
            text: text.clone(),
            below,
            graph_tab: if latest.is_none() { 0 } else { view.graph },
            mode: match view_mode {
                PipelineView::Text => 0,
                PipelineView::Split => 1,
                PipelineView::Graph => 2,
            },
            problems: view.problems.clone(),
            selected: view.selected.clone(),
            ..Local::default()
        });
        let canvas_cell = RefCell::new(&mut canvas);
        let tables = RefCell::new(&mut self.tables);
        let file_actions = |ui: &mut Ui| {
            let mut l = st.borrow_mut();
            if going {
                l.cancel |= ui
                    .add(
                        Button::new("Cancel")
                            .icon(Icon::Square)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm),
                    )
                    .clicked();
            } else {
                let mut button = Button::new("Run").icon(Icon::Play).size(ButtonSize::Sm);
                if let Some(reason) = &run_reason {
                    button = button.reason(reason);
                }
                l.run |= ui.add(button).clicked() && run_reason.is_none();
            }
            l.save |= ui
                .add_enabled(
                    unsaved,
                    Button::new("Save")
                        .icon(Icon::Save)
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Sm),
                )
                .clicked();
            ui.add(Segmented::new(&mut l.mode, ["Text", "Both", "Graph"]));
        };
        let only = (st.borrow().mode == 2).then_some(1);
        SplitPane::new("studio-pipeline")
            .only(only)
            .default_share(0.55)
            .min_share(0.3)
            .max_share(0.75)
            .show(
                ui,
                |ui| {
                    editor_pane(
                        ui,
                        &id,
                        &file.name,
                        unsaved,
                        &mut popped,
                        Some(Box::new(reference)),
                        file_actions,
                        |ui, place| {
                            let mut l = st.borrow_mut();
                            let out =
                                CodeEditor::new(("pipeline", &id, place == WindowedPaneWhere::Window), &mut l.text)
                                    .label(&file.name)
                                    .language(CodeLanguage::Yaml)
                                    .variant(CodeVariant::Pane)
                                    .schema(pipeline_schema())
                                    .deferred(&holds_reference)
                                    .marks(&marks)
                                    .completions(&words)
                                    .hover(|word| describe_word(&pipeline, word))
                                    .reveal(reveal)
                                    .wrap(prefs.wrap)
                                    .line_numbers(prefs.line_numbers)
                                    .size(code_size(prefs.size))
                                    .folding(prefs.folding)
                                    .indent_guides(prefs.indent_guides)
                                    .bracket_colors(prefs.bracket_colors)
                                    .active_line(if prefs.line_wash { ActiveLine::Always } else { ActiveLine::Focused })
                                    .search(true)
                                    .show(ui);
                            l.edited |= out.changed;
                            l.run |= out.run && run_reason.is_none() && !going;
                            l.save |= out.saved;
                            if place == WindowedPaneWhere::Pane {
                                l.problems = out.problems;
                            }
                        },
                    );
                },
                |ui| {
                    let only_below = (st.borrow().mode == 0).then_some(1);
                    SplitPane::new("studio-pipeline-side")
                        .down(true)
                        .only(only_below)
                        .default_share(0.6)
                        .show(
                            ui,
                            |ui| {
                                let graph_tab = st.borrow().graph_tab;
                                let chip = graph_tab == 1 && latest.is_some();
                                let meta = match (&latest, graph_tab) {
                                    (Some(run), 1) => format!("Run {} {}", run.number, run.state.name()),
                                    _ => counted(pipeline.steps.len() as f64, "step", None),
                                };
                                pane(
                                    ui,
                                    |ui| {
                                        titled(ui, "Steps");
                                        if chip && let Some(run) = &latest {
                                            ui.add(Chip::new(&meta).tone(ChipTone::Status(run.state.name())));
                                        }
                                    },
                                    if chip { None } else { Some(meta.as_str()) },
                                    |ui| {
                                        if st.borrow().mode == 2 {
                                            file_actions(ui);
                                        }
                                        let off = latest.is_none().then(|| "Not run yet".to_owned());
                                        let mut l = st.borrow_mut();
                                        ui.add(
                                            Segmented::new(&mut l.graph_tab, ["Steps", "Last run"]).disabled([None, off]),
                                        );
                                    },
                                    |ui| {
                                        if pipeline.broken {
                                            ui.add_space(48.0);
                                            Empty::new("The YAML does not read")
                                                .description(
                                                    "The steps show again once the problem marked in the editor is fixed.",
                                                )
                                                .show(ui, |_| {});
                                        } else if graph_tab == 1
                                            && let Some(run) = &latest
                                        {
                                            let steps: Vec<RunStep> = run
                                                .steps
                                                .iter()
                                                .map(|s| {
                                                    RunStep::new(&s.id, s.state.name())
                                                        .kind(&s.kind)
                                                        .needs(s.needs.clone())
                                                        .duration_ms(s.duration_ms.map(|ms| ms as f64))
                                                })
                                                .collect();
                                            let mut l = st.borrow_mut();
                                            let out = RunGraph::new(("pipeline-run", &id), &steps)
                                                .selected(l.selected.as_deref())
                                                .show(ui);
                                            if let Some(pressed) = out.pressed {
                                                l.selected = Some(pressed);
                                            }
                                        } else {
                                            let mut l = st.borrow_mut();
                                            let shown_text = l.text.clone();
                                            let mut canvas = canvas_cell.borrow_mut();
                                            let canvas_out = pipeline_canvas(
                                                ui,
                                                &id,
                                                &shown_text,
                                                &pipeline,
                                                &issues,
                                                &positions,
                                                &mut l.selected,
                                                &mut canvas,
                                            );
                                            if let Some(next) = canvas_out.text {
                                                l.replaced = Some(next);
                                            }
                                            if canvas_out.settings {
                                                l.below = 0;
                                            }
                                            if canvas_out.show_text {
                                                if l.mode == 2 {
                                                    l.mode = 1;
                                                }
                                                l.jump = l.selected.as_ref().and_then(|s| {
                                                    pipeline.steps.iter().find(|one| &one.id == s).map(|one| {
                                                        shown_text[..one.from.min(shown_text.len())].matches('\n').count() + 1
                                                    })
                                                });
                                            }
                                            if let Some(at) = canvas_out.moved {
                                                l.moved = Some(at);
                                            }
                                            l.relayout |= canvas_out.relayout;
                                        }
                                    },
                                );
                            },
                            |ui| {
                                let below = st.borrow().below;
                                let meta = match (&latest, below) {
                                    (Some(run), 2) => match &run.failed {
                                        Some(failed) => Some(format!("Failed at {}: {}", failed.step, failed.reason)),
                                        None if run.draft => Some("Run from unsaved changes".to_owned()),
                                        None => None,
                                    },
                                    _ => None,
                                };
                                let shows_step = st.borrow().mode != 0;
                                pane(
                                    ui,
                                    |ui| {
                                        let off = latest.is_none().then(|| "Not run yet".to_owned());
                                        let mut l = st.borrow_mut();
                                        let count = l.problems.len();
                                        if shows_step {
                                            ui.add(
                                                Segmented::new(&mut l.below, ["Step", "Problems", "Log"])
                                                    .counts([None, Some(count), None])
                                                    .disabled([None, None, off]),
                                            );
                                        } else {
                                            let mut at = l.below.saturating_sub(1);
                                            ui.add(
                                                Segmented::new(&mut at, ["Problems", "Log"])
                                                    .counts([Some(count), None])
                                                    .disabled([None, off]),
                                            );
                                            l.below = at + 1;
                                        }
                                    },
                                    meta.as_deref(),
                                    |ui| {
                                        if below == 2 && latest.is_some() {
                                            st.borrow_mut().open_run = ui
                                                .add(
                                                    Button::new("Open run")
                                                        .icon(Icon::History)
                                                        .variant(ButtonVariant::Ghost)
                                                        .size(ButtonSize::Sm),
                                                )
                                                .clicked();
                                        }
                                    },
                                    |ui| {
                                        let below = st.borrow().below;
                                        if below == 2
                                            && let Some(run) = &latest
                                        {
                                            crate::work::run_log_view(ui, run, &prefs);
                                        } else if below == 0 {
                                            let (shown_text, chosen) = {
                                                let l = st.borrow();
                                                (l.text.clone(), l.selected.clone())
                                            };
                                            if let Some(change) =
                                                step_settings(ui, &id, &shown_text, &pipeline, chosen.as_deref())
                                            {
                                                let mut l = st.borrow_mut();
                                                match change {
                                                    StepChange::Text(next) => l.replaced = Some(next),
                                                    StepChange::Renamed(from, to, next) => {
                                                        l.replaced = Some(next);
                                                        l.renamed = Some((from, to));
                                                    }
                                                }
                                            }
                                        } else {
                                            let problems = st.borrow().problems.clone();
                                            let mut tables = tables.borrow_mut();
                                            let state = tables.entry(format!("pipeline-problems:{id}")).or_default();
                                            let line = problems_list(
                                                ui,
                                                "pipeline-problems",
                                                &problems,
                                                "No problems: the pipeline is ready to run.",
                                                state,
                                            )
                                            .and_then(|i| problems.get(i))
                                            .map(|p| p.line);
                                            if line.is_some() {
                                                st.borrow_mut().jump = line;
                                            }
                                        }
                                    },
                                );
                            },
                        );
                },
            );
        let Local {
            text: typed,
            mut below,
            mut graph_tab,
            mode,
            save,
            run,
            cancel,
            mut edited,
            open_run,
            problems,
            jump,
            replaced,
            mut selected,
            renamed,
            moved,
            relayout,
        } = st.into_inner();
        let mut text = typed;
        // the graph's undo, and the keys, serve the text too
        let history = canvas.history.get_or_insert_with(|| GraphHistory::new(text.clone()));
        if let Some(next) = replaced {
            text = next;
            history.set(text.clone(), None);
            edited = true;
        } else if edited {
            history.set(text.clone(), Some("type"));
        }
        let typing = ui.ctx().memory(|m| m.focused().is_some());
        let undo_key = history_keys(ui.ctx(), mode != 0 && !typing);
        let (undo, redo) = std::mem::take(&mut canvas.asked);
        if (undo || undo_key == Some(HistoryKey::Undo)) && history.undo() {
            text = history.value().clone();
            canvas.notice = None;
            edited = true;
        }
        if (redo || undo_key == Some(HistoryKey::Redo)) && history.redo() {
            text = history.value().clone();
            canvas.notice = None;
            edited = true;
        }
        if edited {
            self.backend.edit(&id, &text);
        }
        if let Some((from, to)) = renamed {
            let mut kept = layout_of(&self.backend, &id);
            if let Some(at) = kept.remove(&from) {
                kept.insert(to.clone(), at);
                keep_layout(&mut self.backend, &id, &kept);
            }
            selected = Some(to);
        }
        if let Some((node, at)) = moved {
            let mut kept = layout_of(&self.backend, &id);
            kept.insert(node, at);
            keep_layout(&mut self.backend, &id, &kept);
        }
        if relayout {
            keep_layout(&mut self.backend, &id, &HashMap::new());
        }
        if save {
            self.backend.save(&id, now);
        }
        if cancel && let Some(run) = &latest {
            self.backend.cancel(&run.id, now);
        }
        if run {
            match self.backend.run(&id, now) {
                Some(Ok(_)) => {
                    graph_tab = 1;
                    below = 2;
                }
                Some(Err(notice)) => {
                    sokkel_egui::Toast::error(notice.title)
                        .description(notice.description)
                        .show(ui.ctx());
                }
                None => {}
            }
        }
        if open_run && let Some(run) = &latest {
            self.go(&format!("runs/{}", run.id));
        }
        let next_mode = match mode {
            0 => PipelineView::Text,
            1 => PipelineView::Split,
            _ => PipelineView::Graph,
        };
        if next_mode != view_mode {
            self.backend.set_prefs(|p| p.pipeline_view = next_mode);
        }
        view.popped = popped;
        view.below = below;
        view.graph = graph_tab;
        view.selected = selected;
        view.reveal = jump;
        view.problems = problems;
        self.views.insert(id.clone(), view);
        self.canvases.insert(id, canvas);
    }
}

/// What the pipeline editor's panes change while they are drawn.
#[derive(Default)]
struct Local {
    text: String,
    below: usize,
    graph_tab: usize,
    mode: usize,
    save: bool,
    run: bool,
    cancel: bool,
    edited: bool,
    open_run: bool,
    problems: Vec<sokkel_egui::CodeProblem>,
    jump: Option<usize>,
    replaced: Option<String>,
    selected: Option<String>,
    renamed: Option<(String, String)>,
    moved: Option<(String, Pos2)>,
    relayout: bool,
}

/// What the canvas asked of the editor this frame.
#[derive(Default)]
struct CanvasOut {
    /// The pipeline's text after a gesture.
    text: Option<String>,
    /// A step's settings shown, from its menu.
    settings: bool,
    /// A step shown in the text, from its menu.
    show_text: bool,
    /// A box moved, with its new top-left corner.
    moved: Option<(String, Pos2)>,
    /// The boxes laid out again, forgetting where they were put.
    relayout: bool,
}

/// The pipeline's steps on a canvas that edits them: steps added from the picker (the add button,
/// a step's plus or menu, an edge's middle, a connection let go on the ground, or the ground
/// right-clicked), connected port to port with loops refused, removed with Delete or from the
/// menu, and moved, where they stay for this pipeline.
#[allow(clippy::too_many_arguments)]
fn pipeline_canvas(
    ui: &mut Ui,
    id: &str,
    text: &str,
    pipeline: &Pipeline,
    issues: &HashMap<String, Vec<NodeIssue>>,
    positions: &HashMap<String, Pos2>,
    selected: &mut Option<String>,
    canvas: &mut CanvasState,
) -> CanvasOut {
    let mut out = CanvasOut::default();
    let ids: Vec<&str> = pipeline.steps.iter().map(|s| s.id.as_str()).collect();
    let steps: Vec<PipelineStep> = pipeline
        .steps
        .iter()
        .map(|step| {
            PipelineStep::new(&step.id)
                .kind(&step.kind)
                .needs(
                    step.needs
                        .iter()
                        .filter(|n| ids.contains(&n.as_str()) && **n != step.id)
                        .cloned()
                        .collect::<Vec<_>>(),
                )
                .icon(kind_icon(&step.kind))
                .issues(issues.get(&step.id).cloned().unwrap_or_default())
        })
        .collect();
    let can_connect = |source: &str, target: &str| -> Option<String> {
        let into = pipeline.steps.iter().find(|s| s.id == target)?;
        if into.needs.iter().any(|n| n == source) {
            return Some(format!("{target} already comes after {source}."));
        }
        cycle_path(&steps, source, target).map(|loop_path| {
            let mut path = loop_path;
            path.push(target.to_owned());
            format!("That makes a loop: {}.", path.join(" \u{2192} "))
        })
    };
    let step_menu = |node: &str| {
        Some(
            GraphMenu::new(vec![
                MenuItem::action("Add a step after").id("after"),
                MenuItem::action("Settings").id("settings"),
                MenuItem::action("Show in the text").id("text"),
                MenuItem::Separator,
                MenuItem::action("Remove step").id("remove").destructive(),
            ])
            .head(node),
        )
    };
    let placed = positions.keys().any(|k| ids.contains(&k.as_str()));
    let add = [GraphControlButton::new(Icon::Plus, "Add a step")];
    let issue_count = (!pipeline.steps.is_empty()).then_some(issues.len());
    let first_issue = pipeline
        .steps
        .iter()
        .find(|s| issues.contains_key(&s.id))
        .map(|s| s.id.clone());
    let history = canvas.history.as_ref();
    let toolbar = GraphToolbar::new()
        .history(
            history.is_some_and(|h| h.can_undo()),
            history.is_some_and(|h| h.can_redo()),
        )
        .zoom(false)
        .layout(placed)
        .issues(issue_count, first_issue.is_some());
    let label = if pipeline.name.is_empty() {
        "Pipeline".to_owned()
    } else {
        pipeline.name.clone()
    };
    let response = PipelineGraph::new(("pipeline-graph", id), &steps)
        .selected(selected.as_deref())
        .positions(Some(positions))
        .editing(Some(
            GraphEditing {
                append: true,
                insert: true,
                moves: true,
                drop: true,
                node_menu: true,
                pane_menu: true,
                ..GraphEditing::new()
            }
            .can_connect(&can_connect),
        ))
        .node_menu(&step_menu)
        .extra_controls(&add)
        .toolbar(Some(toolbar))
        .empty_action("Add step", Some(Icon::Plus))
        .words(GraphWords {
            label,
            empty: "No steps yet".into(),
            ..Default::default()
        })
        .show(ui);
    if let Some(pressed) = &response.pressed {
        *selected = Some(pressed.clone());
    }
    if response.pane_pressed {
        *selected = None;
    }
    let say = |canvas: &mut CanvasState, critical: bool, text: String| canvas.notice = Some((critical, text));
    for event in &response.events {
        match event {
            GraphEvent::Connect { source, target } => match can_connect(source, target) {
                Some(reason) => say(canvas, true, reason),
                None => {
                    out.text = Some(connect(text, source, target));
                    canvas.notice = None;
                }
            },
            GraphEvent::Refuse { reason, .. } => say(canvas, true, reason.clone()),
            GraphEvent::Disconnect(edge) => {
                out.text = Some(disconnect(text, &edge.source, &edge.target));
                say(
                    canvas,
                    false,
                    format!("{} no longer comes after {}.", edge.target, edge.source),
                );
            }
            GraphEvent::Remove(node) => {
                out.text = Some(remove_steps(text, &[node.as_str()]));
                if selected.as_deref() == Some(node.as_str()) {
                    *selected = None;
                }
                canvas.picker = None;
                say(canvas, false, format!("Removed {node}. Undo brings it back."));
            }
            GraphEvent::Move { node, at } => out.moved = Some((node.clone(), *at)),
            GraphEvent::Append { node, .. } => {
                *selected = Some(node.clone());
                canvas.picker = Some(Picker::After(node.clone()));
            }
            GraphEvent::Insert { edge, .. } => {
                canvas.picker = Some(Picker::Insert {
                    source: edge.source.clone(),
                    target: edge.target.clone(),
                    edge: edge.id.clone(),
                });
            }
            GraphEvent::Drop { source, at } => {
                *selected = Some(source.clone());
                canvas.picker = Some(Picker::Drop {
                    source: source.clone(),
                    at: at.at,
                });
            }
            GraphEvent::PaneMenu { at } => canvas.picker = Some(Picker::Here(at.at)),
            GraphEvent::NodeMenu { node, .. } => {
                *selected = Some(node.clone());
                canvas.picker = None;
            }
            GraphEvent::MenuPick { node: Some(node), item } => match item.as_str() {
                "after" => canvas.picker = Some(Picker::After(node.clone())),
                "settings" => {
                    *selected = Some(node.clone());
                    out.settings = true;
                }
                "text" => {
                    *selected = Some(node.clone());
                    out.show_text = true;
                }
                "remove" => {
                    out.text = Some(remove_steps(text, &[node.as_str()]));
                    if selected.as_deref() == Some(node.as_str()) {
                        *selected = None;
                    }
                    say(canvas, false, format!("Removed {node}. Undo brings it back."));
                }
                _ => {}
            },
            _ => {}
        }
    }
    canvas.asked = (response.toolbar.undo, response.toolbar.redo);
    if response.toolbar.layout {
        out.relayout = true;
    }
    if response.toolbar.issues
        && let Some(first) = first_issue
    {
        *selected = Some(first);
    }
    // the add button, or the empty canvas's: the picker after the chosen step
    let button = response
        .extra
        .and_then(|i| response.extra_rects.get(i).copied())
        .or(response.empty_action.then_some(response.empty_action_rect).flatten());
    if let Some(rect) = button {
        canvas.picker = match canvas.picker {
            Some(Picker::Button { .. }) => None,
            _ => Some(Picker::Button {
                rect,
                after: selected.clone(),
            }),
        };
    }
    let controls = &response.controls;
    // where a box after `id` goes: on the side its edges leave from
    let beside = |node: &str| -> Option<Pos2> {
        let b = controls.node_box(node)?;
        Some(match controls.direction {
            sokkel_egui_graph::GraphDirection::Down => Pos2::new(b.min.x, b.max.y + GAP_DOWN),
            _ => Pos2::new(b.max.x + GAP_ACROSS, b.min.y),
        })
    };
    // past every box, for a step added on its own to a graph placed by hand
    let past_all = || -> Option<Pos2> {
        let boxes: Vec<Rect> = pipeline.steps.iter().filter_map(|s| controls.node_box(&s.id)).collect();
        if boxes.is_empty() {
            return None;
        }
        Some(match controls.direction {
            sokkel_egui_graph::GraphDirection::Down => Pos2::new(
                boxes.iter().map(|b| b.min.x).fold(f32::INFINITY, f32::min),
                boxes.iter().map(|b| b.max.y).fold(f32::NEG_INFINITY, f32::max) + GAP_DOWN,
            ),
            _ => Pos2::new(
                boxes.iter().map(|b| b.max.x).fold(f32::NEG_INFINITY, f32::max) + GAP_ACROSS,
                boxes.iter().map(|b| b.min.y).fold(f32::INFINITY, f32::min),
            ),
        })
    };
    if let Some(picker) = canvas.picker.clone() {
        let (anchor, link, title) = match &picker {
            Picker::After(source) => (
                GraphAnchor::Node {
                    node: source.clone(),
                    dx: 34.0,
                    dy: 0.0,
                },
                None,
                format!("After {source}"),
            ),
            Picker::Insert { source, target, edge } => (
                GraphAnchor::Edge(edge.clone()),
                None,
                format!("Between {source} and {target}"),
            ),
            Picker::Drop { source, at } => (GraphAnchor::Point(*at), Some(source.clone()), format!("After {source}")),
            Picker::Here(at) => (GraphAnchor::Point(*at), None, "A step on its own".to_owned()),
            Picker::Button { rect, after } => (
                GraphAnchor::Rect(*rect),
                None,
                after
                    .as_ref()
                    .map_or("A step on its own".to_owned(), |a| format!("After {a}")),
            ),
        };
        let blocks: Vec<PipelineBlock> = STEP_KINDS
            .iter()
            .map(|kind| {
                let mut block = PipelineBlock::new(*kind, group_of(kind)).description(kind_docs(kind).unwrap_or(""));
                if let Some(icon) = kind_icon(kind) {
                    block = block.icon(icon);
                }
                block
            })
            .collect();
        let groups = [
            BlockGroup::new("read", "Read"),
            BlockGroup::new("shape", "Shape"),
            BlockGroup::new("write", "Write"),
        ];
        let shown =
            GraphOverlay::new(("pipeline-picker", id), anchor)
                .link(link)
                .show(ui, controls, |ui, room, width| {
                    BlockPicker::new(("picker", id), &blocks, &groups)
                        .title(Some(title))
                        .room(room, width)
                        .words(BlockPickerWords {
                            search: "Find a kind of step".into(),
                            list: "Kinds of step".into(),
                            ..Default::default()
                        })
                        .show(ui)
                });
        let picked = shown.inner.as_ref().and_then(|p| p.picked);
        if shown.closed || shown.inner.as_ref().is_some_and(|p| p.closed) {
            canvas.picker = None;
        }
        if let Some(index) = picked {
            let kind = STEP_KINDS[index];
            let at = |point: Pos2| point - DROP_OFFSET;
            let (needs, before, place): (Vec<String>, Option<String>, Option<Pos2>) = match &picker {
                Picker::After(source) => (vec![source.clone()], None, if placed { beside(source) } else { None }),
                Picker::Insert { source, target, .. } => (vec![source.clone()], Some(target.clone()), None),
                Picker::Drop { source, at: point } => (vec![source.clone()], None, Some(at(*point))),
                Picker::Here(point) => (Vec::new(), None, Some(at(*point))),
                Picker::Button { after, .. } => (
                    after.iter().cloned().collect(),
                    None,
                    if placed {
                        match after {
                            Some(a) => beside(a),
                            None => past_all(),
                        }
                    } else {
                        None
                    },
                ),
            };
            let (added, next) = add_step(text, kind, &needs, before.as_deref());
            if !added.is_empty() {
                out.text = Some(next);
                if let Some(point) = place {
                    out.moved = Some((added.clone(), point));
                }
                *selected = Some(added);
                canvas.picker = None;
                canvas.notice = None;
            }
        }
    }
    if let Some((critical, words)) = canvas.notice.clone() {
        let closed = GraphNotice::new(words)
            .tone(if critical {
                NoticeTone::Critical
            } else {
                NoticeTone::Info
            })
            .show(ui, controls);
        if closed {
            canvas.notice = None;
        }
    }
    out
}

/// What a step's settings changed.
enum StepChange {
    Text(String),
    /// The step renamed from, to, and the text after.
    Renamed(String, String, String),
}

/// The chosen step: its kind, its id, and its settings as a form drawn from the pipeline schema.
fn step_settings(
    ui: &mut Ui,
    file: &str,
    text: &str,
    pipeline: &Pipeline,
    selected: Option<&str>,
) -> Option<StepChange> {
    let t = tokens(ui.ctx());
    let step = selected.and_then(|s| pipeline.steps.iter().find(|st| st.id == s));
    let Some(step) = step.filter(|s| STEP_KINDS.contains(&s.kind.as_str())) else {
        ui.add_space(48.0);
        Empty::new("No step chosen")
            .description("Choose a step on the graph to set it here.")
            .show(ui, |_| {});
        return None;
    };
    let mut change = None;
    egui::ScrollArea::vertical()
        .id_salt(("step-settings", file))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::new().inner_margin(egui::Margin::same(16)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.horizontal_top(|ui| {
                    if let Some(icon) = kind_icon(&step.kind) {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 20.0), egui::Sense::hover());
                        sokkel_egui::icons::paint(
                            ui.painter(),
                            egui::pos2(rect.center().x, rect.top() + 2.0 + 8.0),
                            icon,
                            16.0,
                            t.color("muted-foreground"),
                        );
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        let spec = t.text(TextRole::Body.key());
                        ui.label(egui::RichText::new(&step.kind).font(sokkel_egui::mono_font(
                            ui.ctx(),
                            sokkel_egui::mono_size(&t, spec.size),
                            spec.weight,
                        )));
                        ui.label(
                            TextRole::Help
                                .rich(&t, kind_docs(&step.kind).unwrap_or(""))
                                .color(t.color("muted-foreground")),
                        );
                    });
                });
                // the step's id, taken when the field is left or Enter is pressed
                let draft_id = egui::Id::unique(("step-id", file, &step.id));
                let mut draft = ui
                    .data(|d| d.get_temp::<String>(draft_id))
                    .unwrap_or_else(|| step.id.clone());
                let problem = if !is_step_id(&draft) {
                    Some("Lowercase letters, digits and dashes, starting with a letter.".to_owned())
                } else if draft != step.id && pipeline.steps.iter().any(|s| s.id == draft) {
                    Some(format!("Another step is named {draft}."))
                } else {
                    None
                };
                let response = ui.add(
                    TextField::new("Step id", &mut draft)
                        .help("How other steps and references name it.")
                        .error(problem.clone()),
                );
                let taken = response.lost_focus();
                ui.data_mut(|d| d.insert_temp(draft_id, draft.clone()));
                if taken && draft != step.id && problem.is_none() {
                    change = Some(StepChange::Renamed(
                        step.id.clone(),
                        draft.clone(),
                        rename_step(text, &step.id, &draft),
                    ));
                    ui.data_mut(|d| d.remove::<String>(draft_id));
                }
                let schema = settings_schema(&step.kind);
                let mut value = Value::Object(step_fields(text, &step.id).unwrap_or_default());
                let before = value.clone();
                SchemaForm::new(("step-form", file, &step.id), &schema, &mut value)
                    .deferred(&holds_reference)
                    .submitted(true)
                    .show(ui);
                if value != before && change.is_none() {
                    let fields: Map<String, Value> = value.as_object().cloned().unwrap_or_default();
                    change = Some(StepChange::Text(set_step_fields(text, &step.id, &fields)));
                }
            });
        });
    change
}

/// Where a pipeline's boxes were moved to, by step.
fn layout_of(backend: &crate::backend::Backend, id: &str) -> HashMap<String, Pos2> {
    backend
        .layouts
        .iter()
        .find(|l| l.id == id)
        .map(|l| {
            l.positions
                .iter()
                .map(|(k, (x, y))| (k.clone(), Pos2::new(*x as f32, *y as f32)))
                .collect()
        })
        .unwrap_or_default()
}

/// Keeps where a pipeline's boxes were moved to.
fn keep_layout(backend: &mut crate::backend::Backend, id: &str, positions: &HashMap<String, Pos2>) {
    backend.set_layout(
        id,
        positions
            .iter()
            .map(|(k, p)| (k.clone(), (f64::from(p.x), f64::from(p.y))))
            .collect(),
    );
}
