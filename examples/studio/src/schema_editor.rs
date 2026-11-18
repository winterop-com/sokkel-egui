//! A JSON Schema beside a value tried against it, as the web's `editors/SchemaEditor.tsx`.

use std::cell::{Cell, RefCell};

use eframe::egui::Ui;
use sokkel_egui::{
    ActiveLine, Button, ButtonSize, ButtonVariant, Chip, ChipTone, CodeEditor, CodeLanguage, CodeVariant, Icon,
    Segmented, SplitPane, WindowedPaneWhere, json_schema_2020_12,
};

use crate::app::StudioApp;
use crate::model::WorkFile;
use crate::work::{code_size, editor_pane, pane, problems_list, titled};

/// The schema a document holds, once it reads as a JSON object the meta-schema finds no error in.
fn read_schema(text: &str) -> Option<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    if !value.is_object() {
        return None;
    }
    let issues = sokkel_egui::schema_issues(json_schema_2020_12(), &value);
    issues
        .iter()
        .all(|issue| issue.tone != sokkel_egui::IssueTone::Critical)
        .then_some(value)
}

impl StudioApp {
    /// A JSON Schema: edited against the 2020-12 meta-schema, every keyword completed and checked
    /// as it is typed, beside a value tried against the schema as it stands.
    pub fn schema_editor(&mut self, ui: &mut Ui, file: &WorkFile) {
        let id = file.id.clone();
        let mut text = self.backend.doc_for(&id).unwrap_or_default().to_owned();
        let schema = read_schema(&text);
        let mut value = self.backend.sample(&id).unwrap_or("{}\n").to_owned();
        let unsaved = self.backend.dirty().contains(&id);
        let prefs = self.backend.prefs.clone();
        let mut view = self.views.remove(&id).unwrap_or_default();
        let (save, edited, tried) = (Cell::new(false), Cell::new(false), Cell::new(false));
        let problems = RefCell::new(view.problems.clone());
        let value_problems = RefCell::new(view.value_problems.clone());
        let reveal = view.reveal.take();
        let reveal_value = view.reveal_value.take();
        let mut popped = view.popped;
        let mut below = view.below.min(1);
        let mut jump = None;
        let mut jump_value = None;
        let tables = &mut self.tables;
        SplitPane::new("studio-schema")
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
                        None,
                        |ui| {
                            let pressed = ui
                                .add_enabled(
                                    unsaved,
                                    Button::new("Save")
                                        .icon(Icon::Save)
                                        .variant(ButtonVariant::Ghost)
                                        .size(ButtonSize::Sm),
                                )
                                .clicked();
                            save.set(save.get() | pressed);
                        },
                        |ui, place| {
                            let out = CodeEditor::new(("schema", &id, place == WindowedPaneWhere::Window), &mut text)
                                .label(&file.name)
                                .language(CodeLanguage::Json)
                                .variant(CodeVariant::Pane)
                                .schema(json_schema_2020_12())
                                .reveal(reveal)
                                .wrap(prefs.wrap)
                                .line_numbers(prefs.line_numbers)
                                .size(code_size(prefs.size))
                                .folding(prefs.folding)
                                .indent_guides(prefs.indent_guides)
                                .bracket_colors(prefs.bracket_colors)
                                .active_line(if prefs.line_wash {
                                    ActiveLine::Always
                                } else {
                                    ActiveLine::Focused
                                })
                                .search(true)
                                .show(ui);
                            edited.set(edited.get() | out.changed);
                            save.set(save.get() | out.saved);
                            if place == WindowedPaneWhere::Pane {
                                *problems.borrow_mut() = out.problems;
                            }
                        },
                    );
                },
                |ui| {
                    let shown_below = below;
                    SplitPane::new("studio-schema-side")
                        .down(true)
                        .default_share(0.55)
                        .show(
                            ui,
                            |ui| {
                                let failed = value_problems
                                    .borrow()
                                    .iter()
                                    .any(|p| p.tone == sokkel_egui::MarkTone::Error);
                                let (label, tone) = match (&schema, failed) {
                                    (None, _) => ("Schema not ready", "skipped"),
                                    (Some(_), true) => ("Refused", "failed"),
                                    (Some(_), false) => ("Passes", "succeeded"),
                                };
                                pane(
                                    ui,
                                    |ui| {
                                        titled(ui, "Try a value");
                                        ui.add(Chip::new(label).tone(ChipTone::Status(tone)));
                                    },
                                    None,
                                    |_| {},
                                    |ui| {
                                        let mut editor = CodeEditor::new(("schema-value", &id), &mut value)
                                            .label(format!("A value tried against {}", file.name))
                                            .language(CodeLanguage::Json)
                                            .variant(CodeVariant::Pane)
                                            .reveal(reveal_value)
                                            .wrap(prefs.wrap)
                                            .line_numbers(prefs.line_numbers)
                                            .size(code_size(prefs.size))
                                            .bracket_colors(prefs.bracket_colors)
                                            .active_line(if prefs.line_wash {
                                                ActiveLine::Always
                                            } else {
                                                ActiveLine::Focused
                                            });
                                        if let Some(schema) = &schema {
                                            editor = editor.schema(schema);
                                        }
                                        let out = editor.show(ui);
                                        tried.set(tried.get() | out.changed);
                                        *value_problems.borrow_mut() = out.problems;
                                    },
                                );
                            },
                            |ui| {
                                pane(
                                    ui,
                                    |ui| {
                                        let counts =
                                            [Some(value_problems.borrow().len()), Some(problems.borrow().len())];
                                        ui.add(Segmented::new(&mut below, ["Value", "Schema"]).counts(counts));
                                    },
                                    None,
                                    |_| {},
                                    |ui| {
                                        if shown_below == 1 {
                                            let state = tables.entry(format!("schema-problems:{id}")).or_default();
                                            jump = problems_list(
                                                ui,
                                                "schema-problems",
                                                &problems.borrow(),
                                                "The schema is valid JSON Schema 2020-12.",
                                                state,
                                            );
                                        } else {
                                            let empty = if schema.is_none() {
                                                "Fix the schema first; the value is tried once it is valid."
                                            } else {
                                                "The value passes the schema."
                                            };
                                            let state = tables.entry(format!("value-problems:{id}")).or_default();
                                            jump_value = problems_list(
                                                ui,
                                                "value-problems",
                                                &value_problems.borrow(),
                                                empty,
                                                state,
                                            );
                                        }
                                    },
                                );
                            },
                        );
                },
            );
        let now = self.now();
        let (problems, value_problems) = (problems.into_inner(), value_problems.into_inner());
        if edited.get() {
            self.backend.edit(&id, &text);
        }
        if tried.get() {
            self.backend.set_sample(&id, &value);
        }
        if save.get() {
            self.backend.save(&id, now);
        }
        view.popped = popped;
        view.below = below;
        view.reveal = jump.and_then(|i| problems.get(i)).map(|p| p.line);
        view.reveal_value = jump_value.and_then(|i| value_problems.get(i)).map(|p| p.line);
        view.problems = problems;
        view.value_problems = value_problems;
        self.views.insert(id, view);
    }
}
