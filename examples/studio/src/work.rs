//! The open files and tables as tabs over the one shown, as the web's `pages/WorkArea.tsx`, and
//! the parts every editor page draws with (`editors/parts.tsx`, `results.tsx`, `Problems.tsx`,
//! `TableView.tsx`, `QueryEditor.tsx`).

use std::collections::HashMap;

use eframe::egui::{self, Ui};
use sokkel_egui::{
    ActiveLine, Banner, BannerTone, Button, ButtonSize, ButtonVariant, Cell, CodeCompletion, CodeEditor, CodeLanguage,
    CodeMark, CodeProblem, CodeSize, CodeVariant, Column, DataTable, DocumentTab, DocumentTabs, DocumentTabsWords,
    DropdownMenu, Empty, Icon, IconButton, MarkTone, MenuItem, PaneHeader, SplitPane, StackLayout, TableState,
    TableVariant, TableWords, WindowedPane, WindowedPaneWhere, WindowedPaneWords, windowed_pane_button,
};

use crate::app::{StudioApp, kind_icon};
use crate::model::{FileKind, Run, TextSize, counted, say_duration};
use crate::sql::{QueryResult, Table, Value, run_query};
use crate::tables::tables_with;

/// What a query's last run gave: its rows, or its refusal and where it is.
#[derive(Clone, Debug)]
pub enum Outcome {
    Done(QueryResult, u64),
    Failed(String, CodeMark),
}

/// What an editor page holds between frames, by its file.
#[derive(Default)]
pub struct FileView {
    /// The editor's pane popped out into a window.
    pub popped: bool,
    /// The query's last run.
    pub outcome: Option<Outcome>,
    /// What the editor's checks found, and the try value's.
    pub problems: Vec<CodeProblem>,
    pub value_problems: Vec<CodeProblem>,
    /// A line to scroll the editor to, and the try value's.
    pub reveal: Option<usize>,
    pub reveal_value: Option<usize>,
    /// Which list stands under the side: 0, 1, 2 in each editor's own order.
    pub below: usize,
    /// The pipeline's graph shows its steps (0) or its last run (1).
    pub graph: usize,
    /// The step chosen on the graph.
    pub selected: Option<String>,
    /// How many times the query ran, to key its results table.
    pub runs: u64,
}

/// A pane: its head with a title, a short note and its actions, and its content filling the rest.
pub fn pane(
    ui: &mut Ui,
    title: impl FnOnce(&mut Ui),
    meta: Option<&str>,
    actions: impl FnOnce(&mut Ui),
    content: impl FnOnce(&mut Ui),
) {
    let spacing = ui.spacing().item_spacing.y;
    ui.spacing_mut().item_spacing.y = 0.0;
    let mut header = PaneHeader::new().title_with(title);
    if let Some(meta) = meta {
        header = header.meta(meta);
    }
    header.show(ui, actions);
    ui.spacing_mut().item_spacing.y = spacing;
    content(ui);
}

/// A pane titled in words.
pub fn titled(ui: &mut Ui, title: &str) {
    let t = sokkel_egui::tokens(ui.ctx());
    ui.label(
        sokkel_egui::TextRole::SectionTitle
            .rich(&t, title)
            .color(t.color_or("section-head-ink", t.color("foreground"))),
    );
}

/// The editor's text size, as the settings give it.
pub fn code_size(size: TextSize) -> CodeSize {
    match size {
        TextSize::Sm => CodeSize::Sm,
        TextSize::Md => CodeSize::Md,
        TextSize::Lg => CodeSize::Lg,
    }
}

/// A cell's value as the results show it: null quiet, a number in figures.
pub fn value_cell(value: &Value, numeric: bool) -> Cell {
    match value {
        Value::Null => Cell::MutedCode("null".into()),
        Value::Num(n) => Cell::Number {
            value: *n,
            text: crate::js::locale_number(*n),
        },
        Value::Bool(b) if numeric => Cell::Number {
            value: f64::from(u8::from(*b)),
            text: b.to_string(),
        },
        other => Cell::Text(other.say()),
    }
}

/// The columns of a result, one per name: a numeric one (every value a number or null) read from
/// its end, at least 112 wide, the others at least 144, sharing the rest.
pub fn result_columns(names: &[String], rows: &[Vec<Value>]) -> Vec<Column> {
    names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let numeric = !rows.is_empty()
                && rows
                    .iter()
                    .all(|row| matches!(row.get(i), Some(Value::Null) | Some(Value::Num(_)) | None));
            if numeric {
                Column::new(name, 112.0 - 12.0).numeric().fill_share(0.6).sortable(true)
            } else {
                Column::new(name, 144.0 - 12.0).fill().sortable(true)
            }
        })
        .collect()
}

/// What the editor's checks found, a row each, as the web's `Problems`: pressing one goes to it.
pub fn problems_list(
    ui: &mut Ui,
    id: &str,
    problems: &[CodeProblem],
    empty: &str,
    state: &mut TableState,
) -> Option<usize> {
    let t = sokkel_egui::tokens(ui.ctx());
    if problems.is_empty() {
        egui::Frame::new()
            .inner_margin(egui::Margin::same(t.density("panel-pad") as i8))
            .show(ui, |ui| {
                ui.label(
                    sokkel_egui::TextRole::Body
                        .rich(&t, empty)
                        .color(t.color("muted-foreground")),
                );
            });
        return None;
    }
    let columns = vec![
        Column::new("Kind", 40.0),
        Column::new("Problem", 192.0).fill(),
        Column::new("Line", 96.0).numeric(),
    ];
    let rows: Vec<Vec<Cell>> = problems
        .iter()
        .map(|p| {
            let (word, tone) = match p.tone {
                MarkTone::Error => ("Error", "critical"),
                MarkTone::Warning => ("Warning", "warning"),
                MarkTone::Info => ("Note", "info"),
            };
            vec![
                Cell::Dot(tone.into(), word.into()),
                Cell::Text(p.message.clone()),
                Cell::Text(format!("line {}", p.line)),
            ]
        })
        .collect();
    DataTable::new(id, &columns, &rows, state)
        .variant(TableVariant::List)
        .framed(false)
        .bar(false)
        .heads(false)
        .fits(true)
        .stack_layout(StackLayout::Compact)
        .height(f32::INFINITY)
        .footer(false)
        .show(ui);
    state.opened.take()
}

/// The editor's pane, which opens into a window over the page: the pane stays as it is under the
/// window, whose own editor goes on in the same document.
/// What stands beside the editor in its window, such as a reference.
pub type Reference<'a> = Option<Box<dyn FnOnce(&mut Ui) + 'a>>;

#[allow(clippy::too_many_arguments)]
pub fn editor_pane(
    ui: &mut Ui,
    id: &str,
    name: &str,
    unsaved: bool,
    popped: &mut bool,
    reference: Reference<'_>,
    mut actions: impl FnMut(&mut Ui),
    mut editor: impl FnMut(&mut Ui, WindowedPaneWhere),
) {
    let words = WindowedPaneWords {
        expand: "Pop out".into(),
        ..Default::default()
    };
    let mut pane_box = WindowedPane::new(("editor", id), name)
        .keep_pane(true)
        .words(words.clone());
    if let Some(reference) = reference {
        pane_box = pane_box.aside(reference);
    }
    let mut pop = false;
    pane_box.show(ui, popped, |ui, place| {
        if place == WindowedPaneWhere::Window {
            editor(ui, place);
            return;
        }
        pane(
            ui,
            |ui| titled(ui, name),
            Some(if unsaved { "Unsaved changes" } else { "Saved" }),
            |ui| {
                pop = windowed_pane_button(ui, &words);
                actions(ui);
            },
            |ui| editor(ui, place),
        );
    });
    if pop {
        *popped = true;
    }
}

/// A run's log as text, a line each, with the step that wrote it and its time from the start.
pub fn log_text(run: &Run) -> String {
    let width = run.steps.iter().map(|s| s.id.chars().count()).max().unwrap_or(0).max(4);
    run.log
        .iter()
        .map(|line| {
            let offset = format!("+{}s", crate::js::to_fixed(line.at as f64 / 1000.0, 3));
            let step = line.step.clone().unwrap_or_else(|| "run".into());
            format!("{offset:>9}  {step:<width$}  {}", line.text)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The log's marks: every line of the step that failed washed, its errors saying why, and
/// warnings.
pub fn log_marks(run: &Run) -> Vec<CodeMark> {
    run.log
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let at = index + 1;
            let mark = |tone, message: Option<String>| CodeMark {
                line: Some(at),
                range: None,
                path: None,
                tone,
                message,
            };
            if line.level == crate::model::Level::Error {
                return Some(mark(
                    MarkTone::Error,
                    Some(run.failed.as_ref().map_or(line.text.clone(), |f| f.reason.clone())),
                ));
            }
            if run
                .failed
                .as_ref()
                .is_some_and(|f| line.step.as_deref() == Some(f.step.as_str()))
            {
                return Some(mark(MarkTone::Error, None));
            }
            (line.level == crate::model::Level::Warn).then(|| mark(MarkTone::Warning, None))
        })
        .collect()
}

/// A run's log, read-only, with the failing step marked.
pub fn run_log_view(ui: &mut Ui, run: &Run, prefs: &crate::model::Prefs) {
    let mut text = log_text(run);
    let marks = log_marks(run);
    {
        CodeEditor::new(("run-log", &run.id), &mut text)
            .label(format!("Log of run {}", run.number))
            .variant(CodeVariant::Pane)
            .size(CodeSize::Sm)
            .marks(&marks)
            .read_only(true)
            .wrap(prefs.wrap)
            .line_numbers(prefs.line_numbers)
            .active_line(if prefs.line_wash {
                ActiveLine::Always
            } else {
                ActiveLine::Focused
            })
            .search(true)
            .show(ui);
    }
}

impl StudioApp {
    /// A run's log, read-only, with the failing step marked.
    pub fn run_log(&self, ui: &mut Ui, run: &Run) {
        run_log_view(ui, run, &self.backend.prefs);
    }

    /// The open files and tables as tabs over the one shown.
    pub fn work_area(&mut self, ui: &mut Ui, tab: &str) {
        let tables = tables_with(&self.backend.runs);
        let dirty = self.backend.dirty();
        let file = self.backend.file(tab).cloned();
        let table = tab
            .strip_prefix("table:")
            .and_then(|name| tables.iter().find(|t| t.name == name))
            .cloned();
        let exists = file.is_some() || table.is_some();
        let tabs: Vec<DocumentTab> = self
            .backend
            .prefs
            .tabs
            .iter()
            .filter_map(|id| {
                if let Some(name) = id.strip_prefix("table:") {
                    return tables
                        .iter()
                        .any(|t| t.name == name)
                        .then(|| DocumentTab::new(id, name).icon(Icon::Sheet));
                }
                let one = self.backend.file(id)?;
                Some(
                    DocumentTab::new(id, &one.name)
                        .icon(kind_icon(one.kind))
                        .dirty(dirty.contains(id)),
                )
            })
            .collect();
        let all_tabs = self.backend.prefs.tabs.clone();
        let spacing = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 0.0;
        let (mut new_file, mut menu) = (false, None);
        let out = DocumentTabs::new("work-tabs", &tabs, exists.then_some(tab))
            .closable(true)
            .words(DocumentTabsWords {
                label: "Open files".into(),
                ..Default::default()
            })
            .show(ui, |ui| {
                let trigger = ui.add(IconButton::new(Icon::Ellipsis, "Tabs").xs());
                let items = [
                    MenuItem::Label("Tabs".into()),
                    MenuItem::action("Save all").icon(Icon::Save).disabled(dirty.is_empty()),
                    MenuItem::Separator,
                    MenuItem::action("Close the others")
                        .icon(Icon::X)
                        .disabled(all_tabs.len() < 2),
                    MenuItem::action("Close the saved ones")
                        .icon(Icon::X)
                        .disabled(all_tabs.iter().all(|t| dirty.contains(t))),
                    MenuItem::action("Close all")
                        .icon(Icon::X)
                        .disabled(all_tabs.is_empty()),
                ];
                menu = DropdownMenu::new(&items).end(true).width(224.0).show(&trigger);
                new_file = ui.add(IconButton::new(Icon::FilePlus, "New file").xs()).clicked();
            });
        ui.spacing_mut().item_spacing.y = spacing;
        match (&file, &table) {
            (Some(file), _) if file.kind == FileKind::Pipeline => self.pipeline_editor(ui, file),
            (Some(file), _) if file.kind == FileKind::Schema => self.schema_editor(ui, file),
            (Some(file), _) => self.query_editor(ui, file, &tables),
            (None, Some(table)) => self.table_view(ui, table),
            _ => sokkel_egui::Page::show(ui, |ui| {
                let mut back = false;
                Empty::new("Nothing here")
                    .description("The file was deleted, or the address is wrong.")
                    .show(ui, |ui| {
                        back = ui
                            .add(
                                Button::new("Back to the workspace")
                                    .variant(ButtonVariant::Outline)
                                    .size(ButtonSize::Sm),
                            )
                            .clicked();
                    });
                if back {
                    self.go("");
                }
            }),
        }
        let now = self.now();
        if let Some(chosen) = out.chosen.filter(|c| c != tab) {
            self.open_tab(&chosen);
        }
        if let Some(closed) = out.closed {
            self.close_tabs(&[closed]);
        }
        if new_file {
            self.backend.asking = Some(crate::backend::Asking::New {
                project: file.as_ref().map_or(crate::model::Project::Harbor, |f| f.project),
                kind: file.as_ref().map_or(FileKind::Pipeline, |f| f.kind),
            });
        }
        match menu {
            Some(1) => self.backend.save_all(now),
            Some(3) => {
                let others: Vec<String> = all_tabs.iter().filter(|t| t.as_str() != tab).cloned().collect();
                self.close_tabs(&others);
            }
            Some(4) => {
                let saved: Vec<String> = all_tabs.iter().filter(|t| !dirty.contains(*t)).cloned().collect();
                if let Some(to) = self.backend.close_now(&saved) {
                    self.go(&to);
                }
            }
            Some(5) => self.close_tabs(&all_tabs),
            _ => {}
        }
    }

    /// Closes tabs, asking first about those with changes not saved, and follows the shown one.
    pub fn close_tabs(&mut self, ids: &[String]) {
        if let Some(to) = self.backend.close(ids) {
            self.go(&to);
        }
    }

    /// The view an editor page keeps, made the first time it is drawn.
    pub fn view(&mut self, id: &str) -> &mut FileView {
        self.views.entry(id.to_owned()).or_default()
    }

    /// A table a query reads: its columns and what each holds, over its rows, searched.
    fn table_view(&mut self, ui: &mut Ui, table: &Table) {
        let info_columns = vec![
            Column::new("Column", 128.0).fill_share(0.6),
            Column::new("Type", 96.0),
            Column::new("What it holds", 192.0).fill(),
        ];
        let info_rows: Vec<Vec<Cell>> = table
            .columns
            .iter()
            .map(|c| {
                vec![
                    Cell::Code(c.name.clone()),
                    Cell::Text(c.kind.name().into()),
                    Cell::Text(c.description.clone()),
                ]
            })
            .collect();
        let names: Vec<String> = table.columns.iter().map(|c| c.name.clone()).collect();
        let query_id = format!("table-query:{}", table.name);
        let mut query = self.queries.remove(&query_id).unwrap_or_default();
        let typed = query.trim().to_lowercase();
        let shown: Vec<&Vec<Value>> = table
            .rows
            .iter()
            .filter(|row| typed.is_empty() || row.iter().any(|v| v.say().to_lowercase().contains(&typed)))
            .collect();
        let columns = result_columns(&names, &table.rows);
        let rows: Vec<Vec<Cell>> = shown
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(i, v)| value_cell(v, columns[i].numeric))
                    .collect()
            })
            .collect();
        let tables = std::cell::RefCell::new(&mut self.tables);
        SplitPane::new("studio-table").down(true).default_share(0.3).show(
            ui,
            |ui| {
                pane(
                    ui,
                    |ui| titled(ui, &table.name),
                    Some(&table.description),
                    |_| {},
                    |ui| {
                        let mut tables = tables.borrow_mut();
                        let state = tables.entry(format!("info:{}", table.name)).or_default();
                        DataTable::new(("info", &table.name), &info_columns, &info_rows, state)
                            .variant(TableVariant::List)
                            .framed(false)
                            .bar(false)
                            .fits(true)
                            .height(f32::INFINITY)
                            .footer(false)
                            .show(ui);
                    },
                );
            },
            |ui| {
                let mut tables = tables.borrow_mut();
                let state = tables.entry(format!("rows:{}", table.name)).or_default();
                DataTable::new(("rows", &table.name), &columns, &rows, state)
                    .framed(false)
                    .fits(true)
                    .search(&mut query)
                    .words(TableWords {
                        rows: "{count} rows".into(),
                        row: "{count} row".into(),
                        ..Default::default()
                    })
                    .footer(false)
                    .empty(|ui| {
                        Empty::new("No row holds that.").show(ui, |_| {});
                    })
                    .height(f32::INFINITY)
                    .show(ui);
            },
        );
        self.queries.insert(query_id, query);
    }

    /// A query: SQL over the simulated tables, run with Mod-Enter or Run, its rows in a table under
    /// it and a refusal marked where it is.
    fn query_editor(&mut self, ui: &mut Ui, file: &crate::model::WorkFile, tables: &[Table]) {
        let id = file.id.clone();
        let mut text = self.backend.doc_for(&id).unwrap_or_default().to_owned();
        if self.view(&id).outcome.is_none() {
            let outcome = execute(&text, tables);
            self.view(&id).outcome = Some(outcome);
        }
        let words = table_words(tables);
        let unsaved = self.backend.dirty().contains(&id);
        let prefs = self.backend.prefs.clone();
        let describe = |word: &str| describe_word(tables, word);
        let mut view = self.views.remove(&id).unwrap_or_default();
        let (save, run, edited) = (
            std::cell::Cell::new(false),
            std::cell::Cell::new(false),
            std::cell::Cell::new(false),
        );
        let marks: Vec<CodeMark> = match &view.outcome {
            Some(Outcome::Failed(_, mark)) => vec![mark.clone()],
            _ => Vec::new(),
        };
        let reveal = view.reveal.take();
        let mut popped = view.popped;
        let tables_state = &mut self.tables;
        SplitPane::new("studio-query")
            .down(true)
            .default_share(0.45)
            .min_share(0.2)
            .max_share(0.8)
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
                                .add(Button::new("Run").icon(Icon::Play).size(ButtonSize::Sm))
                                .clicked();
                            run.set(run.get() | pressed);
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
                            let out = CodeEditor::new(("query", &id, place == WindowedPaneWhere::Window), &mut text)
                                .label(&file.name)
                                .language(CodeLanguage::Sql)
                                .variant(CodeVariant::Pane)
                                .completions(&words)
                                .hover(describe)
                                .marks(&marks)
                                .reveal(reveal)
                                .wrap(prefs.wrap)
                                .line_numbers(prefs.line_numbers)
                                .size(code_size(prefs.size))
                                .bracket_colors(prefs.bracket_colors)
                                .active_line(if prefs.line_wash {
                                    ActiveLine::Always
                                } else {
                                    ActiveLine::Focused
                                })
                                .search(true)
                                .show(ui);
                            edited.set(edited.get() | out.changed);
                            run.set(run.get() | out.run);
                            save.set(save.get() | out.saved);
                        },
                    );
                },
                |ui| {
                    let meta = match &view.outcome {
                        Some(Outcome::Done(result, _)) => Some(format!(
                            "{} in {}",
                            counted(result.rows.len() as f64, "row", None),
                            say_duration(result.ms)
                        )),
                        Some(Outcome::Failed(..)) => Some("Refused".to_owned()),
                        None => None,
                    };
                    pane(
                        ui,
                        |ui| titled(ui, "Results"),
                        meta.as_deref(),
                        |_| {},
                        |ui| match &view.outcome {
                            Some(Outcome::Failed(message, _)) => {
                                egui::Frame::new().inner_margin(egui::Margin::same(12)).show(ui, |ui| {
                                    Banner::new(BannerTone::Critical, message).show(ui);
                                });
                            }
                            Some(Outcome::Done(result, at)) => {
                                let columns = result_columns(&result.columns, &result.rows);
                                let rows: Vec<Vec<Cell>> = result
                                    .rows
                                    .iter()
                                    .map(|row| {
                                        row.iter()
                                            .enumerate()
                                            .map(|(i, v)| value_cell(v, columns[i].numeric))
                                            .collect()
                                    })
                                    .collect();
                                let state = tables_state.entry(format!("result:{id}:{at}")).or_default();
                                DataTable::new(("result", &id, *at), &columns, &rows, state)
                                    .framed(false)
                                    .fits(true)
                                    .words(TableWords {
                                        rows: "{count} rows".into(),
                                        row: "{count} row".into(),
                                        ..Default::default()
                                    })
                                    .bar(true)
                                    .footer(false)
                                    .empty(|ui| {
                                        Empty::new("The query found no rows.").show(ui, |_| {});
                                    })
                                    .height(f32::INFINITY)
                                    .show(ui);
                            }
                            None => {}
                        },
                    );
                },
            );
        view.popped = popped;
        let now = self.now();
        if edited.get() {
            self.backend.edit(&id, &text);
        }
        if run.get() {
            view.runs += 1;
            view.outcome = Some(match execute(&text, tables) {
                // a held clock (the tests, the screenshots) times every run as the web's shot does
                Outcome::Done(mut result, _) => {
                    if self.held {
                        result.ms = 1.0;
                    }
                    Outcome::Done(result, view.runs)
                }
                failed => failed,
            });
        }
        if save.get() {
            self.backend.save(&id, now);
        }
        self.views.insert(id, view);
    }
}

/// A query run over the tables.
fn execute(text: &str, tables: &[Table]) -> Outcome {
    match run_query(text, tables) {
        Ok(result) => Outcome::Done(result, 0),
        Err(error) => {
            let mark = match error.at {
                Some((from, to)) => CodeMark {
                    line: None,
                    range: Some((from, to.max(from + 1))),
                    path: None,
                    tone: MarkTone::Error,
                    message: Some(error.message.clone()),
                },
                None => CodeMark {
                    line: Some(1),
                    range: None,
                    path: None,
                    tone: MarkTone::Error,
                    message: None,
                },
            };
            Outcome::Failed(error.message, mark)
        }
    }
}

/// The table and column names a query completes from, each with what it holds.
fn table_words(tables: &[Table]) -> Vec<CodeCompletion> {
    let mut columns: Vec<(String, CodeCompletion)> = Vec::new();
    for table in tables {
        for column in &table.columns {
            let detail = match columns.iter().find(|(name, _)| *name == column.name) {
                None => format!("{}, {}", table.name, column.kind.name()),
                Some((_, known)) => format!(
                    "{} and {}",
                    known.detail.as_deref().unwrap_or("").split(',').next().unwrap_or(""),
                    table.name
                ),
            };
            let word = CodeCompletion {
                label: column.name.clone(),
                kind: Some("property".into()),
                detail: Some(detail),
                info: Some(column.description.clone()),
                apply: None,
            };
            match columns.iter_mut().find(|(name, _)| *name == column.name) {
                Some(slot) => slot.1 = word,
                None => columns.push((column.name.clone(), word)),
            }
        }
    }
    tables
        .iter()
        .map(|table| CodeCompletion {
            label: table.name.clone(),
            kind: Some("type".into()),
            detail: Some("table".into()),
            info: Some(table.description.clone()),
            apply: None,
        })
        .chain(columns.into_iter().map(|(_, w)| w))
        .collect()
}

/// What a word of a query names: a table, or the columns by that name.
fn describe_word(tables: &[Table], word: &str) -> Option<String> {
    let lower = word.to_lowercase();
    if let Some(table) = tables.iter().find(|t| t.name == lower) {
        return Some(format!(
            "{}: {} {}.",
            table.name,
            table.description,
            counted(table.rows.len() as f64, "row", None)
        ));
    }
    let owners: Vec<String> = tables
        .iter()
        .filter_map(|t| {
            t.columns
                .iter()
                .find(|c| c.name == lower)
                .map(|c| format!("{}.{lower}: {}", t.name, c.description))
        })
        .collect();
    (!owners.is_empty()).then(|| owners.join("\n"))
}

/// The queries typed in each table's search, by table.
pub type Queries = HashMap<String, String>;
