//! The editor pages, as the web's `pages/Resource.tsx`, `ResourcePanel.tsx`, `FhirPath.tsx` and
//! `Cql.tsx` draw them: a resource as its JSON, checked against its type as it is typed and saved
//! as a new version, with the side panel beside it (its problems, its versions with the one chosen
//! drawn against the editor, its references and a patient's vital signs); FHIRPath evaluated
//! against a resource or the whole server; and a CQL library in the add-on's language beside its
//! versions and the diff of the one chosen.

use eframe::egui::Ui;
use serde_json::{Value, json};
use sokkel_egui::{
    ApiLink, Banner, BannerTone, ButtonVariant, Cell, CodeCompletion, CodeEditor, CodeLanguage, CodeMark, CodeVariant,
    Column, DataTable, DiffView, DiffViewWords, DropdownMenu, Field, Gap, Icon, MarkTone, MenuItem, Page, PageHeader,
    PageState, Select, SidePanel, SplitPane, Stack, StackLayout, StackPlace, TableState, TableVariant, panel_section,
};
use sokkel_egui_charts::{ChartKind, ChartPanel, ChartSpec};
use sokkel_egui_fhir::{CQL, FHIRPATH};

use crate::app::App;
use crate::evaluate::{Completion, describe_word};
use crate::js::{number_to_string, pretty};
use crate::model::{Part, field, js_string, last_updated};
use crate::pages::{self, FHIRPATH_EXAMPLES, Mark, PageState as Held};
use crate::screens::{Act, Later, chip, chip_of, code_size, looked, pane, small, words_empty};
use crate::ui::{state_of, when};
use crate::workbench::{Asking, Views};

/// A page's mark as the editor's: a whole line, the columns on a line, or the value at a path.
fn code_mark(mark: &Mark, text: &str) -> CodeMark {
    let tone = match mark.tone {
        "error" => MarkTone::Error,
        "warning" => MarkTone::Warning,
        _ => MarkTone::Info,
    };
    let path = mark.path.as_ref().map(|path| {
        path.iter()
            .map(|part| match part {
                Part::Name(name) => name.clone(),
                Part::Index(index) => index.to_string(),
            })
            .collect()
    });
    // columns on a line, from 1, as byte offsets into the text
    let range = match (mark.line, mark.from, mark.to) {
        (Some(line), Some(from), Some(to)) => {
            let start: usize = text.split('\n').take(line - 1).map(|one| one.len() + 1).sum();
            let length = text.split('\n').nth(line - 1).map_or(0, str::len);
            Some((start + (from - 1).min(length), start + (to - 1).min(length)))
        }
        _ => None,
    };
    CodeMark {
        line: if range.is_some() { None } else { mark.line },
        range,
        path,
        tone,
        message: Some(mark.message.clone()),
    }
}

/// The words an editor offers, as the core's completions.
fn completions(words: &[Completion]) -> Vec<CodeCompletion> {
    words
        .iter()
        .map(|one| CodeCompletion {
            label: one.label.clone(),
            kind: Some(one.kind.to_owned()),
            detail: one.detail.clone(),
            info: one.docs.clone(),
            apply: None,
        })
        .collect()
}

/// A header's chip and facts for a stored thing: its state, its version and when it was saved.
fn header_meta(mut header: PageHeader, state: Option<String>, version: f64, at: Option<&str>) -> PageHeader {
    if let Some(state) = state {
        let (label, tone) = chip_of(&state);
        header = header.chip(label, tone);
    }
    header
        .meta(format!("Version {}", number_to_string(version)))
        .meta(when(at))
}

// --- A resource -----------------------------------------------------------------------------------

pub fn resource_page(ui: &mut Ui, app: &App, kind: &str, id: &str, views: &mut Views, acts: &mut Vec<Act>) {
    let look = pages::resource_look(app, kind, id);
    let view = pages::resource_view(app, kind, id);
    let prefs = app.prefs();
    let button = |label: &str| look.buttons.iter().find(|one| one.label == label);
    Page::show(ui, |ui| {
        let mut header = PageHeader::new(&look.header.title).back(format!("All {kind} resources"));
        if let Some(code) = &look.header.code {
            header = header.code(code);
        }
        header = match &view.stored {
            None => match &look.header.meta {
                Some(about) => header.meta(about),
                None => header,
            },
            Some(row) => header_meta(
                header,
                state_of(&row.resource),
                view.version,
                last_updated(&row.resource).as_deref(),
            ),
        };
        let out = header.show(ui, |ui| {
            if view.missing {
                return;
            }
            let label = if view.making { "Create" } else { "Save" };
            if ui
                .add(looked(
                    small(label, Some(Icon::Save), ButtonVariant::Default),
                    button(label),
                ))
                .clicked()
            {
                let (kind, id) = (kind.to_owned(), id.to_owned());
                acts.push(Act::new(move |app| pages::resource_save(&mut app.app, &kind, &id)));
            }
            if ui
                .add(looked(
                    small("Revert", Some(Icon::RotateCcw), ButtonVariant::Outline),
                    button("Revert"),
                ))
                .clicked()
            {
                let key = view.key.clone();
                acts.push(Act::new(move |app| app.app.editing.set_draft(&key, None)));
            }
            if !view.making {
                if ui
                    .add(small("Delete", Some(Icon::Trash2), ButtonVariant::Outline))
                    .clicked()
                {
                    let (kind, id) = (kind.to_owned(), id.to_owned());
                    acts.push(Act::new(move |app| app.asking = Some(Asking::DeleteResource(kind, id))));
                }
                if ui
                    .add(small("FHIRPath", Some(Icon::Braces), ButtonVariant::Outline))
                    .clicked()
                {
                    acts.push(Act::go(format!("fhirpath/{}", view.key)));
                }
                if let Some(api) = &look.api {
                    ui.add(ApiLink::new(api).method("PUT").copy_value(pages::api_address(api)));
                }
            }
        });
        if out.back {
            acts.push(Act::go(format!("resources/{kind}")));
        }
        if !app.backend.resources.loaded() {
            PageState::loading().show(ui);
            return;
        }
        if view.missing {
            PageState::missing(format!(
                "The server holds no {}. It may have been deleted; its versions stay in the history.",
                view.key
            ))
            .show(ui);
            return;
        }
        let schema = crate::types::schema_for(kind).unwrap_or(Value::Null);
        let mut text = view.text.clone();
        let marks: Vec<CodeMark> = look.editor.marks.iter().map(|one| code_mark(one, &text)).collect();
        let mut editor = CodeEditor::new(("resource", &view.key), &mut text)
            .label(&look.editor.label)
            .language(CodeLanguage::Json)
            .marks(&marks)
            .reveal(views.reveal.take())
            .line_numbers(prefs.line_numbers)
            .wrap(prefs.wrap)
            .size(code_size(&prefs))
            .folding(true)
            .bracket_colors(true)
            .search(true)
            .height(ui.available_height());
        if !schema.is_null() {
            editor = editor.schema(&schema);
        }
        let out = editor.show(ui);
        views.problems = out.problems;
        if out.changed {
            let (kind, id) = (kind.to_owned(), id.to_owned());
            acts.push(Act::new(move |app| {
                pages::resource_edit(&mut app.app, &kind, &id, &text)
            }));
        }
        if out.saved {
            let (kind, id) = (kind.to_owned(), id.to_owned());
            acts.push(Act::new(move |app| pages::resource_save(&mut app.app, &kind, &id)));
        }
    });
}

/// A list in the panel: rows without heads or a bar, each opening what it names.
fn panel_list(
    ui: &mut Ui,
    id: &str,
    columns: &[Column],
    cells: &[Vec<Cell>],
    state: &mut TableState,
    empty: &str,
) -> Option<usize> {
    DataTable::new(("panel-list", id), columns, cells, state)
        .variant(TableVariant::List)
        .framed(false)
        .heads(false)
        .stack_layout(StackLayout::Compact)
        .fits(true)
        .footer(false)
        .height(f32::INFINITY)
        .words(words_empty(Some(empty)))
        .show(ui);
    state.opened.take()
}

/// The side panel beside a resource: its problems, its versions, its references and, for a
/// patient, the vital signs.
pub fn resource_panel(ui: &mut Ui, app: &App, kind: &str, id: &str, views: &mut Views, acts: &mut Vec<Act>) {
    let look = pages::panel_look(app, kind, id);
    let key = format!("{kind}/{id}");
    let issues = views.problems.iter().filter(|one| one.tone != MarkTone::Info).count();
    let labels: Vec<String> = look
        .tabs
        .iter()
        .map(|tab| {
            if tab.id == "problems" && issues > 0 {
                format!("Problems {issues}")
            } else {
                tab.label.clone()
            }
        })
        .collect();
    let words: Vec<&str> = labels.iter().map(String::as_str).collect();
    let chosen = views.tab.unwrap_or(look.default);
    let mut index = look.tabs.iter().position(|tab| tab.id == chosen).unwrap_or(0);
    let rows = app.rows();
    SidePanel::new("resource-panel", &words, &mut index).show(ui, |index, ui| {
        let Some(tab) = look.tabs.get(index) else { return };
        match tab.id {
            "problems" => panel_section(ui, "Problems", |ui| {
                let columns = [
                    Column::new("Kind", 32.0).stack(StackPlace::Lead),
                    Column::new("Problem", 160.0).fill().stack(StackPlace::Primary),
                    Column::new("Where", 128.0).stack(StackPlace::Secondary),
                ];
                let cells: Vec<Vec<Cell>> = views
                    .problems
                    .iter()
                    .map(|one| {
                        let (token, word) = match one.tone {
                            MarkTone::Error => ("critical", "Error"),
                            MarkTone::Warning => ("warning", "Warning"),
                            MarkTone::Info => ("info", "Note"),
                        };
                        vec![
                            Cell::Dot(token.to_owned(), word.to_owned()),
                            Cell::Text(one.message.clone()),
                            Cell::Mono(format!("line {}", one.line)),
                        ]
                    })
                    .collect();
                let empty = "The resource checks. The server checks it again, references included, as it is saved.";
                if let Some(at) = panel_list(ui, "problems", &columns, &cells, &mut views.panel[0], empty)
                    && let Some(one) = views.problems.get(at)
                {
                    views.reveal = Some(one.line);
                }
            }),
            "history" => {
                let versions = pages::versions_of(app.backend.history.seen(), &key);
                let chosen = app.panel.as_ref().and_then(|panel| panel.chosen);
                let picked = versions
                    .iter()
                    .find(|one| Some(one.version_id) == chosen)
                    .or(versions.get(1))
                    .or(versions.first())
                    .cloned();
                panel_section(ui, "Versions", |ui| {
                    let columns = [
                        Column::new("Version", 48.0).stack(StackPlace::Lead),
                        Column::new("Change", 96.0).stack(StackPlace::Primary),
                        Column::new("By", 128.0).fill().stack(StackPlace::Secondary),
                    ];
                    let cells: Vec<Vec<Cell>> = versions
                        .iter()
                        .map(|one| {
                            vec![
                                Cell::Mono(format!("v{}", one.version_id)),
                                chip(one.change.key()),
                                Cell::Text(format!("{}, {}", one.by, when(Some(&one.at)))),
                            ]
                        })
                        .collect();
                    let empty = "No versions yet: it is saved as version 1.";
                    if let Some(at) = panel_list(ui, "versions", &columns, &cells, &mut views.panel[1], empty)
                        && let Some(one) = versions.get(at)
                    {
                        let version = one.version_id;
                        acts.push(Act::new(move |app| {
                            if let Some(panel) = &mut app.app.panel {
                                panel.chosen = Some(version);
                            }
                        }));
                    }
                });
                if let Some(one) = picked {
                    let before = one.resource.as_ref().map(pretty).unwrap_or_default();
                    let after = pages::panel_current(app, &key);
                    panel_section(ui, &format!("Version {} against the editor", one.version_id), |ui| {
                        DiffView::new(("resource-diff", &key), &before, &after)
                            .small(true)
                            .words(DiffViewWords {
                                unchanged: "The editor holds this version.".to_owned(),
                                ..Default::default()
                            })
                            .show(ui);
                    });
                }
            }
            "references" => {
                let columns = [
                    Column::new("Resource", 128.0).fill().stack(StackPlace::Primary),
                    Column::new("Key", 160.0).stack(StackPlace::Secondary),
                ];
                for (at, section) in tab.sections.iter().enumerate() {
                    let Some(table) = &section.table else { continue };
                    panel_section(ui, &section.title, |ui| {
                        let cells: Vec<Vec<Cell>> = table
                            .cells
                            .iter()
                            .map(|one| vec![Cell::Text(one[0].clone()), Cell::Mono(one[1].clone())])
                            .collect();
                        let empty = table.empty.clone().unwrap_or_default();
                        if let Some(row) =
                            panel_list(ui, &section.title, &columns, &cells, &mut views.panel[2 + at], &empty)
                            && let Some(key) = table.ids.get(row)
                        {
                            acts.push(Act::go(format!("resources/{key}")));
                        }
                    });
                }
            }
            _ => {
                let _ = rows;
                Stack::new(Gap::Md).show(ui, |ui| {
                    for chart in &tab.charts {
                        use sokkel_egui_charts::data::{Cell as Figure, Row};
                        let points: Vec<Row> = chart
                            .rows
                            .iter()
                            .map(|one| {
                                Row::of([
                                    ("day", Figure::from(one["day"].as_str().unwrap_or_default())),
                                    ("value", Figure::Number(one["value"].as_f64().unwrap_or(0.0))),
                                ])
                            })
                            .collect();
                        let spec = ChartSpec::new(points)
                            .x("day")
                            .y("value")
                            .unit(&chart.subtitle)
                            .fit(true);
                        ChartPanel::new(&chart.title, ChartKind::Line, &spec)
                            .subtitle(&chart.subtitle)
                            .empty(chart.empty)
                            .legend(false)
                            .height(160.0)
                            .show(ui);
                    }
                });
            }
        }
    });
    if look.tabs.get(index).map(|tab| tab.id) != Some(chosen) {
        views.tab = look.tabs.get(index).map(|tab| tab.id);
    }
}

// --- FHIRPath ----------------------------------------------------------------------------------------

pub fn fhirpath_page(ui: &mut Ui, app: &App, subject_key: Option<&str>, acts: &mut Vec<Act>) {
    let look = pages::fhirpath_look(app, subject_key);
    let (subject_id, subject) = pages::fhirpath_subject(app, subject_key);
    let prefs = app.prefs();
    let rows = app.rows();
    let mut words = crate::evaluate::functions();
    words.extend(crate::evaluate::variables_offered());
    match &subject {
        Some(value) if value.get("resourceType").and_then(Value::as_str) == Some("Bundle") => {
            words.extend(crate::evaluate::element_names(&Value::Array(
                rows.iter().take(80).map(|row| row.resource.clone()).collect(),
            )));
        }
        Some(value) => words.extend(crate::evaluate::element_names(value)),
        None => {}
    }
    let words = completions(&words);
    let owned = subject_key.map(str::to_owned);
    Page::show(ui, |ui| {
        let header = &look.header;
        PageHeader::new(&header.title)
            .description(header.description.clone().unwrap_or_default())
            .show(ui, |ui| {
                if !prefs.live
                    && ui
                        .add(small("Evaluate", Some(Icon::Play), ButtonVariant::Default))
                        .clicked()
                {
                    let subject = owned.clone();
                    acts.push(Act::new(move |app| {
                        pages::fhirpath_run(&mut app.app, subject.as_deref())
                    }));
                }
                let trigger = ui.add(small("Examples", Some(Icon::BookOpen), ButtonVariant::Outline));
                let items: Vec<MenuItem> = FHIRPATH_EXAMPLES.iter().map(|one| MenuItem::action(one.0)).collect();
                if let Some(index) = DropdownMenu::new(&items).end(true).width(208.0).show(&trigger) {
                    let (label, subject) = (FHIRPATH_EXAMPLES[index].0, owned.clone());
                    acts.push(Act::new(move |app| {
                        pages::fhirpath_example(&mut app.app, subject.as_deref(), label);
                    }));
                }
            });
        let labels: Vec<String> = look
            .options
            .iter()
            .map(|one| one["label"].as_str().unwrap_or_default().to_owned())
            .collect();
        let values: Vec<String> = look
            .options
            .iter()
            .map(|one| one["value"].as_str().unwrap_or_default().to_owned())
            .collect();
        let mut at = values.iter().position(|one| *one == subject_id).unwrap_or(0);
        let before = at;
        Field::new("Evaluate against").show(ui, |ui| {
            let width = ui.available_width();
            ui.add(Select::new(&mut at, labels.iter().cloned(), "Evaluate against").width(width));
        });
        if at != before
            && let Some(key) = values.get(at).cloned()
        {
            let subject = owned.clone();
            acts.push(Act::new(move |app| {
                pages::fhirpath_choose(&mut app.app, subject.as_deref(), &key);
            }));
        }
        SplitPane::new("fhirpath-split")
            .default_share(0.55)
            .min_share(0.3)
            .max_share(0.75)
            .show(
                ui,
                |ui| {
                    SplitPane::new("fhirpath-expression")
                        .down(true)
                        .default_share(0.5)
                        .min_share(0.2)
                        .max_share(0.8)
                        .show(
                            ui,
                            |ui| {
                                let head = &look.panes[0];
                                pane(
                                    ui,
                                    &head.title,
                                    head.meta.as_deref(),
                                    |_| {},
                                    |ui| {
                                        let mut expression = look.expression.value.clone();
                                        let marks: Vec<CodeMark> = look
                                            .expression
                                            .marks
                                            .iter()
                                            .map(|one| code_mark(one, &expression))
                                            .collect();
                                        let out = CodeEditor::new("fhirpath-expression", &mut expression)
                                            .label(&look.expression.label)
                                            .language(CodeLanguage::Plugin(FHIRPATH))
                                            .variant(CodeVariant::Pane)
                                            .completions(&words)
                                            .hover(describe_word)
                                            .marks(&marks)
                                            .line_numbers(prefs.line_numbers)
                                            .wrap(prefs.wrap)
                                            .size(code_size(&prefs))
                                            .bracket_colors(true)
                                            .show(ui);
                                        if out.changed {
                                            acts.push(Act::new(move |app| {
                                                app.app.kept.set("fhirpath-expression", json!(expression));
                                            }));
                                        }
                                        if out.run {
                                            let subject = owned.clone();
                                            acts.push(Act::new(move |app| {
                                                pages::fhirpath_run(&mut app.app, subject.as_deref());
                                            }));
                                        }
                                    },
                                );
                            },
                            |ui| {
                                let head = &look.panes[1];
                                pane(
                                    ui,
                                    &head.title,
                                    head.meta.as_deref(),
                                    |_| {},
                                    |ui| {
                                        if let Some(banner) = look.result.get("banner").and_then(Value::as_str) {
                                            ui.add_space(12.0);
                                            Banner::new(BannerTone::Critical, banner).show(ui);
                                            return;
                                        }
                                        let mut value =
                                            look.result["editor"]["value"].as_str().unwrap_or_default().to_owned();
                                        CodeEditor::new("fhirpath-result", &mut value)
                                            .label("Result")
                                            .language(CodeLanguage::Json)
                                            .variant(CodeVariant::Pane)
                                            .read_only(true)
                                            .placeholder(if prefs.live {
                                                "Write an expression to see what it selects."
                                            } else {
                                                "Press Evaluate or Mod-Enter."
                                            })
                                            .line_numbers(prefs.line_numbers)
                                            .size(code_size(&prefs))
                                            .folding(true)
                                            .show(ui);
                                    },
                                );
                            },
                        );
                },
                |ui| {
                    let head = &look.panes[2];
                    pane(
                        ui,
                        &head.title,
                        head.meta.as_deref(),
                        |_| {},
                        |ui| {
                            let mut value = subject.as_ref().map(pretty).unwrap_or_default();
                            CodeEditor::new(("fhirpath-subject", &subject_id), &mut value)
                                .label("Subject")
                                .language(CodeLanguage::Json)
                                .variant(CodeVariant::Pane)
                                .read_only(true)
                                .line_numbers(prefs.line_numbers)
                                .size(code_size(&prefs))
                                .folding(true)
                                .search(true)
                                .show(ui);
                        },
                    );
                },
            );
    });
}

// --- A CQL library -------------------------------------------------------------------------------------

pub fn cql_page(ui: &mut Ui, app: &App, id: &str, views: &mut Views, out: &mut Vec<Act>) {
    let later = Later::default();
    cql_inner(ui, app, id, views, &later);
    out.extend(later.take());
}

fn cql_inner(ui: &mut Ui, app: &App, id: &str, views: &mut Views, acts: &Later) {
    let look = pages::cql_look(app, id);
    let view = pages::cql_view(app, id);
    let prefs = app.prefs();
    let button = |label: &str| look.buttons.iter().find(|one| one.label == label);
    Page::show(ui, |ui| {
        let mut header = PageHeader::new(&look.header.title).back("All libraries");
        if let Some(code) = &look.header.code {
            header = header.code(code);
        }
        if let Some(row) = &view.stored {
            header = header_meta(
                header,
                Some(view.status.clone()),
                view.version,
                last_updated(&row.resource).as_deref(),
            );
        }
        let out = header.show(ui, |ui| {
            if view.missing {
                return;
            }
            if ui
                .add(looked(
                    small("Save", Some(Icon::Save), ButtonVariant::Default),
                    button("Save"),
                ))
                .clicked()
            {
                let id = id.to_owned();
                acts.push(Act::new(move |app| pages::cql_save(&mut app.app, &id, None)));
            }
            if view.status == "draft"
                && ui
                    .add(looked(
                        small("Publish", Some(Icon::Upload), ButtonVariant::Outline),
                        button("Publish"),
                    ))
                    .clicked()
            {
                let id = id.to_owned();
                acts.push(Act::new(move |app| pages::cql_save(&mut app.app, &id, Some("active"))));
            }
            if ui
                .add(looked(
                    small("Revert", Some(Icon::RotateCcw), ButtonVariant::Outline),
                    button("Revert"),
                ))
                .clicked()
            {
                let key = view.draft_key.clone();
                acts.push(Act::new(move |app| app.app.editing.set_draft(&key, None)));
            }
            if ui
                .add(small("Library resource", None, ButtonVariant::Outline))
                .clicked()
            {
                acts.push(Act::go(format!("resources/{}", view.key)));
            }
            let api = format!("/{}", view.key);
            ui.add(ApiLink::new(&api).copy_value(pages::api_address(&api)));
        });
        if out.back {
            acts.push(Act::go("cql"));
        }
        if !app.backend.resources.loaded() {
            PageState::loading().show(ui);
            return;
        }
        if view.missing {
            PageState::missing(format!("The server holds no {}.", view.key)).show(ui);
            return;
        }
        SplitPane::new("cql-split")
            .default_share(0.6)
            .min_share(0.35)
            .max_share(0.8)
            .show(
                ui,
                |ui| {
                    let head = &look.panes[0];
                    pane(
                        ui,
                        &head.title,
                        head.meta.as_deref(),
                        |_| {},
                        |ui| {
                            let mut text = view.text.clone();
                            let marks: Vec<CodeMark> =
                                look.editor.marks.iter().map(|one| code_mark(one, &text)).collect();
                            let out = CodeEditor::new(("cql", &view.key), &mut text)
                                .label(&look.editor.label)
                                .language(CodeLanguage::Plugin(CQL))
                                .variant(CodeVariant::Pane)
                                .marks(&marks)
                                .line_numbers(prefs.line_numbers)
                                .wrap(prefs.wrap)
                                .size(code_size(&prefs))
                                .bracket_colors(true)
                                .search(true)
                                .show(ui);
                            if out.changed {
                                let id = id.to_owned();
                                acts.push(Act::new(move |app| pages::cql_edit(&mut app.app, &id, &text)));
                            }
                            if out.saved {
                                let id = id.to_owned();
                                acts.push(Act::new(move |app| pages::cql_save(&mut app.app, &id, None)));
                            }
                        },
                    );
                },
                |ui| {
                    SplitPane::new("cql-versions")
                        .down(true)
                        .default_share(0.35)
                        .min_share(0.2)
                        .max_share(0.7)
                        .show(
                            ui,
                            |ui| {
                                let head = &look.panes[1];
                                pane(
                                    ui,
                                    &head.title,
                                    head.meta.as_deref(),
                                    |_| {},
                                    |ui| {
                                        versions_list(ui, &view, views, acts);
                                    },
                                );
                            },
                            |ui| {
                                let head = &look.panes[2];
                                pane(
                                    ui,
                                    &head.title,
                                    head.meta.as_deref(),
                                    |_| {},
                                    |ui| {
                                        if let Some(against) = &view.against {
                                            let before = pages::cql_of(against.resource.as_ref()).unwrap_or_default();
                                            ui.add_space(0.0);
                                            DiffView::new(("cql-diff", &view.key), &before, &view.text)
                                                .small(true)
                                                .words(DiffViewWords {
                                                    unchanged: "The editor holds this version.".to_owned(),
                                                    ..Default::default()
                                                })
                                                .show(ui);
                                        }
                                    },
                                );
                            },
                        );
                },
            );
    });
}

/// A library's versions, the newest first, the one compared marked; one chosen is compared.
fn versions_list(ui: &mut Ui, view: &pages::CqlView, views: &mut Views, acts: &Later) {
    let columns = [
        Column::new("Version", 40.0).stack(StackPlace::Lead),
        Column::new("Number", 64.0).stack(StackPlace::Lead),
        Column::new("State", 96.0).stack(StackPlace::Primary),
        Column::new("Saved", 160.0).fill().stack(StackPlace::Secondary),
    ];
    let text = |resource: &Value, name: &str| {
        field(resource, name)
            .filter(|one| !one.is_null())
            .map(js_string)
            .unwrap_or_default()
    };
    let cells: Vec<Vec<Cell>> = view
        .own
        .iter()
        .map(|one| match &one.resource {
            None => vec![
                Cell::Mono(format!("v{}", one.version_id)),
                Cell::Empty,
                chip("delete"),
                Cell::Text(format!("{}, {}", one.by, when(Some(&one.at)))),
            ],
            Some(resource) => vec![
                Cell::Mono(format!("v{}", one.version_id)),
                Cell::Code(text(resource, "version")),
                chip(&text(resource, "status")),
                Cell::Text(format!("{}, {}", one.by, when(Some(&one.at)))),
            ],
        })
        .collect();
    let marked = view.own.iter().position(|one| {
        view.against
            .as_ref()
            .is_some_and(|against| against.version_id == one.version_id)
    });
    views.versions.opened = marked;
    DataTable::new("cql-version-list", &columns, &cells, &mut views.versions)
        .keys(view.own.iter().map(|one| one.id.clone()))
        .variant(TableVariant::List)
        .framed(false)
        .heads(false)
        .stack_layout(StackLayout::Compact)
        .stretch(true)
        .footer(false)
        .show(ui);
    if views.versions.opened != marked
        && let Some(one) = views.versions.opened.and_then(|at| view.own.get(at))
    {
        let version = one.version_id;
        acts.push(Act::new(move |app| {
            if let Held::CqlEditor { compared, .. } = &mut app.app.page {
                *compared = Some(version);
            }
        }));
    }
}
