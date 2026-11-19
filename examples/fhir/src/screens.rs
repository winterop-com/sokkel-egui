//! The list pages, as the web's `pages/*.tsx` draw them over each page's own words and logic in
//! `pages`: the overview with its counts, the vital signs measured and the latest changes; a
//! type's resources as a table; the CQL libraries; and search by parameters beside the searches
//! kept and the type's parameters. A resource, FHIRPath and a library's editor are in `detail`.
//! What the status line says is `foot`; what a page asks of the app comes back as acts.

use eframe::egui::{self, Ui};
use serde_json::json;
use sokkel_egui::{
    ApiLink, Banner, BannerTone, Button, ButtonSize, ButtonVariant, Cell, ChipTone, CodeEditor, CodeLanguage, CodeSize,
    CodeVariant, Column, Columns, DataTable, DialogShell, DropdownMenu, Gap, Icon, IconButton, MenuItem, NotFound,
    Page, PageHeader, PageState, PaneHeader, Section, Segmented, SplitPane, Stack, StackLayout, StackPlace, Stat,
    TableState, TableVariant, TableWords, TextField,
};
use sokkel_egui_charts::{ChartKind, ChartPanel, ChartSpec, Stack as ChartStack};

use crate::app::{App, Page as Route};
use crate::js::parse_instant;
use crate::model::{field, js_string, last_updated};
use crate::pages::{self, PageState as Held, SEARCH_EXAMPLES};
use crate::server::Stored;
use crate::ui::{state_of, statuses, title_of, when};
use crate::workbench::{Views, WorkbenchApp};

/// Something a page asks of the app, done once the frame is drawn.
pub struct Act(Box<dyn FnOnce(&mut WorkbenchApp)>);

impl Act {
    pub fn new(does: impl FnOnce(&mut WorkbenchApp) + 'static) -> Self {
        Act(Box::new(does))
    }

    pub fn go(path: impl Into<String>) -> Self {
        let path = path.into();
        Act::new(move |app| app.go(&path))
    }
}

/// Acts asked from several parts of one page drawn side by side, such as the panes of a split.
#[derive(Default)]
pub struct Later(std::cell::RefCell<Vec<Act>>);

impl Later {
    pub fn push(&self, act: Act) {
        self.0.borrow_mut().push(act);
    }

    pub fn take(self) -> Vec<Act> {
        self.0.into_inner()
    }
}

impl WorkbenchApp {
    /// Does what a page asked.
    pub fn apply(&mut self, act: Act) {
        (act.0)(self)
    }
}

/// A small button, outlined or the primary one, with an icon when given.
pub fn small(label: &str, icon: Option<Icon>, variant: ButtonVariant) -> Button {
    let button = Button::new(label).size(ButtonSize::Sm).variant(variant);
    match icon {
        Some(icon) => button.icon(icon),
        None => button,
    }
}

/// A button as a page's look says it: why it cannot be pressed, and whether it is busy.
pub fn looked(button: Button, look: Option<&pages::ButtonLook>) -> Button {
    match look {
        Some(look) => {
            let button = button.busy(look.busy);
            match &look.reason {
                Some(reason) => button.reason(reason),
                None => button,
            }
        }
        None => button,
    }
}

/// A state's chip: its label and color, as the web's `STATUSES` map them.
pub fn chip(state: &str) -> Cell {
    let (label, tone) = chip_of(state);
    Cell::Chip(label, tone)
}

pub fn chip_of(state: &str) -> (String, ChipTone) {
    let style = statuses()
        .into_iter()
        .find(|(key, _)| *key == state)
        .map(|(_, style)| style);
    let tone = match style.map(|one| one.color) {
        Some("good") => ChipTone::Good,
        Some("info") => ChipTone::Info,
        Some("warning") => ChipTone::Warning,
        Some("critical") => ChipTone::Critical,
        _ => ChipTone::Neutral,
    };
    (style.and_then(|one| one.label).unwrap_or(state).to_owned(), tone)
}

/// An instant as its cell: written as the pages write it, sorted by when it is.
pub fn instant(iso: Option<&str>) -> Cell {
    Cell::Number {
        value: iso.and_then(parse_instant).map_or(0.0, |at| at as f64),
        text: when(iso),
    }
}

pub fn words_empty(empty: Option<&str>) -> TableWords {
    TableWords {
        empty: empty.unwrap_or_default().to_owned(),
        ..Default::default()
    }
}

/// A column's width as the web's grid gives it, less the 12 its cell pads, which the table here
/// keeps as the gap between columns.
pub fn cell(web: f32) -> f32 {
    web - 12.0
}

/// What the status line says for the page shown: a note at its start, and the edit's state.
pub fn foot(app: &App) -> (String, Option<String>) {
    let foot = match app.page_of() {
        Route::Overview => pages::overview_foot(app.rows()),
        Route::Resources(kind) => {
            let query = match &app.page {
                Held::Resources { query } => query.clone(),
                _ => String::new(),
            };
            pages::resources_look(&kind, app.rows(), &query).foot
        }
        Route::Resource(kind, id) => pages::resource_look(app, &kind, &id).foot,
        Route::FhirPath(subject) => pages::fhirpath_look(app, subject.as_deref()).foot,
        Route::CqlLibraries => pages::Foot {
            note: Some(crate::ui::count(
                app.rows().iter().filter(|row| row.kind == "Library").count(),
                "library",
                Some("libraries"),
            )),
            items: None,
        },
        Route::CqlEditor(id) => pages::cql_look(app, &id).foot,
        Route::Search => pages::search_look(app).foot,
        Route::NotFound => pages::Foot::default(),
    };
    (foot.note.unwrap_or_default(), foot.items)
}

/// The page the address names.
pub fn page(ui: &mut Ui, app: &App, views: &mut Views, acts: &mut Vec<Act>) {
    match app.page_of() {
        Route::Overview => overview(ui, app, views, acts),
        Route::Resources(kind) => resources(ui, app, &kind, views, acts),
        Route::Resource(kind, id) => crate::detail::resource_page(ui, app, &kind, &id, views, acts),
        Route::FhirPath(subject) => crate::detail::fhirpath_page(ui, app, subject.as_deref(), acts),
        Route::CqlLibraries => libraries(ui, app, views, acts),
        Route::CqlEditor(id) => crate::detail::cql_page(ui, app, &id, views, acts),
        Route::Search => search_page(ui, app, views, acts),
        Route::NotFound => {
            Page::show(ui, |ui| {
                NotFound::new(app.route().join("/")).show(ui);
            });
        }
    }
}

// --- Overview --------------------------------------------------------------------------------------

fn overview(ui: &mut Ui, app: &App, views: &mut Views, acts: &mut Vec<Act>) {
    let header = pages::overview_header();
    let rows = app.rows();
    Page::scroll(ui, |ui| {
        PageHeader::new(&header.title)
            .description(header.description.unwrap_or_default())
            .show(ui, |_| {});
        if !app.backend.resources.loaded() {
            PageState::loading().show(ui);
            return;
        }
        Stack::new(Gap::Section).show(ui, |ui| {
            let stats = pages::overview_stats(rows);
            Columns::new(4)
                .min(160.0)
                .gap(24.0)
                .show(ui, stats.len(), |ui, index, width| {
                    let (kind, value) = &stats[index];
                    if Stat::new(kind, value).tile(ui, width).clicked() {
                        acts.push(Act::go(pages::stat_route(kind)));
                    }
                });
            let measured = pages::measured(rows);
            let chart_rows = measured
                .iter()
                .map(|one| {
                    use sokkel_egui_charts::data::{Cell as Value, Row};
                    Row::of([
                        ("month", Value::from(one.month.as_str())),
                        ("vital", Value::from(one.vital.as_str())),
                        ("count", Value::Number(one.count as f64)),
                    ])
                })
                .collect();
            let spec = ChartSpec::new(chart_rows)
                .x("month")
                .y("count")
                .series("vital")
                .stack(ChartStack::Stacked);
            ChartPanel::new("Vital signs measured", ChartKind::Bar, &spec)
                .subtitle("per month, by measurement")
                .height(256.0)
                .show(ui);
            let recent = pages::recent(app.backend.history.seen());
            Section::new("Latest changes").show(ui, |ui| {
                let columns = [
                    Column::new("Resource", 192.0).fill(),
                    Column::new("Version", 80.0).numeric(),
                    Column::new("Change", 112.0),
                    Column::new("By", 96.0),
                    Column::new("When", 176.0),
                ];
                let cells: Vec<Vec<Cell>> = recent
                    .iter()
                    .map(|one| {
                        vec![
                            Cell::Mono(one.key.clone()),
                            Cell::Number {
                                value: one.version_id as f64,
                                text: one.version_id.to_string(),
                            },
                            chip(one.change.key()),
                            Cell::Text(one.by.clone()),
                            Cell::Text(when(Some(&one.at))),
                        ]
                    })
                    .collect();
                DataTable::new("changes", &columns, &cells, &mut views.changes)
                    .keys(recent.iter().map(|one| one.id.clone()))
                    .chevron(true)
                    .fits(true)
                    .footer(false)
                    .height(f32::INFINITY)
                    .show(ui);
                if let Some(index) = views.changes.opened.take()
                    && let Some(one) = recent.get(index)
                {
                    acts.push(Act::go(pages::change_route(one)));
                }
            });
        });
    });
}

// --- Resources --------------------------------------------------------------------------------------

/// A type's resources as cells: the title, the id, the type's own columns, the state, the version
/// and when it was last changed.
fn resource_cells(kind: &str, shown: &[&Stored], has_state: bool) -> Vec<Vec<Cell>> {
    let columns = pages::type_columns(kind);
    shown
        .iter()
        .map(|row| {
            let resource = &row.resource;
            let mut line = vec![
                Cell::Text(title_of(resource)),
                Cell::Mono(crate::model::id_of(resource)),
            ];
            line.extend(columns.iter().map(|(_, _, read)| Cell::Text(read(resource))));
            if has_state {
                line.push(state_of(resource).map_or(Cell::Empty, |one| chip(&one)));
            }
            let version = crate::model::version_id(resource).unwrap_or_default();
            line.push(Cell::Number {
                value: crate::js::number_of_text(&version),
                text: version,
            });
            line.push(instant(last_updated(resource).as_deref()));
            line
        })
        .collect()
}

fn resources(ui: &mut Ui, app: &App, kind: &str, views: &mut Views, acts: &mut Vec<Act>) {
    let rows = app.rows();
    let mut query = match &app.page {
        Held::Resources { query } => query.clone(),
        _ => String::new(),
    };
    let look = pages::resources_look(kind, rows, &query);
    let header = pages::resources_header();
    let shown: Vec<&Stored> = look
        .table
        .ids
        .iter()
        .filter_map(|id| rows.iter().find(|row| row.id == *id))
        .collect();
    let has_state = look.table.headers.iter().any(|one| one == "State");
    Page::show(ui, |ui| {
        PageHeader::new(&header.title)
            .description(header.description.unwrap_or_default())
            .show(ui, |ui| {
                if ui.add(small("New", Some(Icon::Plus), ButtonVariant::Default)).clicked() {
                    acts.push(Act::go(format!("resources/{kind}/new")));
                }
                ui.add(ApiLink::new(format!("/{kind}")).copy_value(pages::api_address(&format!("/{kind}"))));
            });
        if !app.backend.resources.loaded() {
            PageState::loading().show(ui);
            return;
        }
        let type_columns = pages::type_columns(kind);
        let mut columns = vec![
            Column::new(&look.table.headers[0], cell(192.0)).fill_share(2.0),
            Column::new("Id", cell(144.0)),
        ];
        // the web's widths: a fixed one where `typeColumns` names it, else at least 8rem sharing the rest
        for (id, head, _) in &type_columns {
            columns.push(match (kind, *id) {
                ("Patient", "gender") => Column::new(*head, cell(112.0)),
                ("Patient", "birthDate" | "city") | (_, "value" | "effective" | "onset" | "class") => {
                    Column::new(*head, cell(128.0))
                }
                (_, "version") => Column::new(*head, cell(96.0)),
                _ => Column::new(*head, cell(128.0)).fill(),
            });
        }
        if has_state {
            columns.push(Column::new("State", cell(128.0)));
        }
        columns.push(Column::new("Version", cell(80.0)).numeric());
        let updated = columns.len();
        columns.push(Column::new("Updated", cell(176.0)));
        if views.resources.sort.is_none() {
            views.resources.sort = Some((updated, false));
        }
        let cells = resource_cells(kind, &shown, has_state);
        let types: Vec<&str> = look.types.iter().map(|(name, _, _)| name.as_str()).collect();
        let mut chosen = types.iter().position(|one| *one == kind).unwrap_or(0);
        let counted: Vec<Option<usize>> = look.types.iter().map(|(_, _, n)| Some(*n)).collect();
        DataTable::new(("resources", kind), &columns, &cells, &mut views.resources)
            .keys(shown.iter().map(|row| row.id.clone()))
            .search(&mut query)
            .search_placeholder("Search")
            .toolbar(|ui| {
                if ui
                    .add(Segmented::new(&mut chosen, types.iter().copied()).counts(counted))
                    .changed()
                {
                    acts.push(Act::go(format!("resources/{}", types[chosen])));
                }
            })
            .chevron(true)
            .stretch(true)
            .auto_rows()
            .footer(false)
            .words(words_empty(look.table.empty.as_deref()))
            .show(ui);
        if let Some(index) = views.resources.opened.take()
            && let Some(row) = shown.get(index)
        {
            acts.push(Act::go(format!("resources/{}", row.id)));
        }
    });
    let was = match &app.page {
        Held::Resources { query } => query.clone(),
        _ => String::new(),
    };
    if query != was {
        acts.push(Act::new(move |app| {
            if let Held::Resources { query: held } = &mut app.app.page {
                *held = query;
            }
        }));
    }
}

// --- CQL libraries ----------------------------------------------------------------------------------

fn libraries(ui: &mut Ui, app: &App, views: &mut Views, acts: &mut Vec<Act>) {
    let header = pages::libraries_header();
    let busy = matches!(app.page, Held::CqlLibraries { busy: true });
    let rows: Vec<&Stored> = app.rows().iter().filter(|row| row.kind == "Library").collect();
    Page::show(ui, |ui| {
        PageHeader::new(&header.title)
            .description(header.description.unwrap_or_default())
            .show(ui, |ui| {
                if ui
                    .add(small("New", Some(Icon::Plus), ButtonVariant::Default).busy(busy))
                    .clicked()
                {
                    acts.push(Act::new(|app| pages::make_library(&mut app.app)));
                }
                ui.add(ApiLink::new("/Library").copy_value(pages::api_address("/Library")));
            });
        if !app.backend.resources.loaded() {
            PageState::loading().show(ui);
            return;
        }
        let text = |row: &Stored, name: &str| {
            field(&row.resource, name)
                .filter(|one| !one.is_null())
                .map(js_string)
                .unwrap_or_default()
        };
        let columns = [
            Column::new("Title", 192.0).fill_share(2.0),
            Column::new("Name", 160.0).fill(),
            Column::new("Version", 96.0),
            Column::new("State", 112.0),
            Column::new("Lines", 80.0).numeric(),
            Column::new("Updated", 176.0),
        ];
        let cells: Vec<Vec<Cell>> = rows
            .iter()
            .map(|row| {
                let title = [text(row, "title"), text(row, "name"), row.id.clone()]
                    .into_iter()
                    .find(|one| !one.is_empty())
                    .unwrap_or_default();
                let lines = pages::cql_of(Some(&row.resource))
                    .unwrap_or_default()
                    .split('\n')
                    .count();
                let status = text(row, "status");
                vec![
                    Cell::Text(title),
                    Cell::Mono(text(row, "name")),
                    Cell::Mono(text(row, "version")),
                    chip(if status.is_empty() { "unknown" } else { &status }),
                    Cell::Number {
                        value: lines as f64,
                        text: lines.to_string(),
                    },
                    instant(last_updated(&row.resource).as_deref()),
                ]
            })
            .collect();
        DataTable::new("libraries", &columns, &cells, &mut views.libraries)
            .keys(rows.iter().map(|row| row.id.clone()))
            .bar(true)
            .chevron(true)
            .stretch(true)
            .auto_rows()
            .footer(false)
            .words(words_empty(Some("The server holds no libraries. New makes one.")))
            .show(ui);
        if let Some(index) = views.libraries.opened.take()
            && let Some(row) = rows.get(index)
        {
            acts.push(Act::go(format!("cql/{}", crate::model::id_of(&row.resource))));
        }
    });
}

// --- Search -------------------------------------------------------------------------------------------

/// A pane: its head with a title, a short note and its actions, and its content filling the rest,
/// as the web's `pane.tsx`.
pub fn pane(
    ui: &mut Ui,
    title: &str,
    meta: Option<&str>,
    actions: impl FnOnce(&mut Ui),
    content: impl FnOnce(&mut Ui),
) {
    let mut head = PaneHeader::new().title(title);
    if let Some(meta) = meta {
        head = head.meta(meta);
    }
    head.show(ui, actions);
    let room = ui.available_rect_before_wrap();
    let mut body = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(room)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    body.set_clip_rect(room.intersect(ui.clip_rect()));
    content(&mut body);
    ui.allocate_rect(room, egui::Sense::hover());
}

fn search_page(ui: &mut Ui, app: &App, views: &mut Views, out: &mut Vec<Act>) {
    let later = Later::default();
    search_inner(ui, app, views, &later);
    out.extend(later.take());
}

fn search_inner(ui: &mut Ui, app: &App, views: &mut Views, acts: &Later) {
    let look = pages::search_look(app);
    let result = pages::search_result(app);
    let prefs = app.prefs();
    let mut query = pages::search_query(app);
    let side = pages::search_side(app);
    let view = match &app.page {
        Held::Search(state) => state.view.clone(),
        _ => "table".to_owned(),
    };
    let saved = pages::searches(app);
    let kind = crate::search::read_query(&query).0;
    let known = crate::types::TYPE_NAMES.contains(&kind.as_str());
    let button = |label: &str| look.buttons.iter().find(|one| one.label == label);
    Page::show(ui, |ui| {
        let header = &look.header;
        PageHeader::new(&header.title)
            .description(header.description.clone().unwrap_or_default())
            .show(ui, |ui| {
                let trigger = ui.add(small("Examples", Some(Icon::BookOpen), ButtonVariant::Outline));
                let items: Vec<MenuItem> = SEARCH_EXAMPLES
                    .iter()
                    .map(|(label, _)| MenuItem::action(*label))
                    .collect();
                if let Some(index) = DropdownMenu::new(&items).end(true).width(224.0).show(&trigger) {
                    let text = SEARCH_EXAMPLES[index].1;
                    acts.push(Act::new(move |app| pages::search_set(&mut app.app, text)));
                }
                let save = looked(
                    small("Save", Some(Icon::BookmarkPlus), ButtonVariant::Outline),
                    button("Save"),
                );
                if ui.add(save).clicked() {
                    acts.push(Act::new(|app| {
                        if let Held::Search(state) = &mut app.app.page {
                            state.saving = true;
                        }
                    }));
                }
                if let Some(api) = &look.api {
                    ui.add(ApiLink::new(api).copy_value(pages::api_address(api)));
                }
            });
        let mut ran = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let search = Button::new("Search").icon(Icon::Search);
            let width = ui.available_width() - 8.0 - search_width(ui);
            ui.allocate_ui(egui::vec2(width, ui.available_height()), |ui| {
                ui.set_width(width);
                let words: Vec<sokkel_egui::CodeCompletion> = query_words(&kind);
                let out = CodeEditor::new("search-query", &mut query)
                    .label("Query")
                    .variant(CodeVariant::Line)
                    .completions(&words)
                    .placeholder("Patient?name=ada")
                    .show(ui);
                ran |= out.run;
            });
            ran |= ui.add(search).clicked();
        });
        if query != pages::search_query(app) {
            let text = query.clone();
            acts.push(Act::new(move |app| app.app.kept.set("search-query", json!(text))));
        }
        if ran {
            let text = query.clone();
            acts.push(Act::new(move |app| pages::search_run(&mut app.app, Some(&text))));
        }
        if let Some(banner) = &look.banner {
            let tone = if banner["tone"] == "critical" {
                BannerTone::Critical
            } else {
                BannerTone::Warning
            };
            Banner::new(tone, banner["text"].as_str().unwrap_or_default()).show(ui);
        }
        let empty_matches = look.tables.first().and_then(|one| one.empty.clone());
        SplitPane::new("search-split")
            .default_share(0.62)
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
                        |ui| {
                            let mut chosen = usize::from(view == "bundle");
                            if ui.add(Segmented::new(&mut chosen, ["Table", "Bundle"])).changed() {
                                let next = if chosen == 1 { "bundle" } else { "table" };
                                acts.push(Act::new(move |app| {
                                    if let Held::Search(state) = &mut app.app.page {
                                        state.view = next.to_owned();
                                    }
                                }));
                            }
                            let next = ui.add_enabled(
                                button("Next page").is_some_and(|one| one.reason.is_none()),
                                IconButton::new(Icon::ChevronRight, "Next page"),
                            );
                            if next.clicked()
                                && let Some(result) = &result
                            {
                                let to = result.offset + result.count;
                                acts.push(Act::new(move |app| pages::search_page(&mut app.app, to)));
                            }
                            let previous = ui.add_enabled(
                                button("Previous page").is_some_and(|one| one.reason.is_none()),
                                IconButton::new(Icon::ChevronLeft, "Previous page"),
                            );
                            if previous.clicked()
                                && let Some(result) = &result
                            {
                                let to = (result.offset - result.count).max(0.0);
                                acts.push(Act::new(move |app| pages::search_page(&mut app.app, to)));
                            }
                        },
                        |ui| {
                            if view == "table" {
                                let matches: &[Stored] = result.as_ref().map_or(&[], |one| &one.matches);
                                let columns = [
                                    Column::new("Resource", 192.0).fill_share(2.0),
                                    Column::new("Key", 160.0).fill(),
                                    Column::new("State", 128.0),
                                    Column::new("Updated", 176.0),
                                ];
                                let cells: Vec<Vec<Cell>> = matches
                                    .iter()
                                    .map(|row| {
                                        vec![
                                            Cell::Text(title_of(&row.resource)),
                                            Cell::Mono(row.id.clone()),
                                            state_of(&row.resource).map_or(Cell::Empty, |one| chip(&one)),
                                            Cell::Text(when(last_updated(&row.resource).as_deref())),
                                        ]
                                    })
                                    .collect();
                                DataTable::new("matches", &columns, &cells, &mut views.matches)
                                    .keys(matches.iter().map(|row| row.id.clone()))
                                    .framed(false)
                                    .chevron(true)
                                    .stretch(true)
                                    .auto_rows()
                                    .footer(false)
                                    .words(words_empty(empty_matches.as_deref()))
                                    .show(ui);
                                if let Some(index) = views.matches.opened.take()
                                    && let Some(row) = matches.get(index)
                                {
                                    acts.push(Act::go(format!("resources/{}", row.id)));
                                }
                            } else {
                                let mut bundle = look.bundle.clone().unwrap_or_default();
                                CodeEditor::new("search-bundle", &mut bundle)
                                    .label("Search result")
                                    .language(CodeLanguage::Json)
                                    .variant(CodeVariant::Pane)
                                    .read_only(true)
                                    .line_numbers(prefs.line_numbers)
                                    .size(code_size(&prefs))
                                    .folding(true)
                                    .search(true)
                                    .show(ui);
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
                        |ui| {
                            let mut chosen = usize::from(side != "saved");
                            if ui.add(Segmented::new(&mut chosen, ["Saved", "Parameters"])).changed() {
                                let next = if chosen == 1 { "parameters" } else { "saved" };
                                acts.push(Act::new(move |app| app.app.kept.set("search-side", json!(next))));
                            }
                        },
                        |ui| {
                            if side == "saved" {
                                saved_list(ui, &saved, &mut views.side, acts);
                            } else {
                                let mut params = if known {
                                    crate::search::params_of(&kind)
                                } else {
                                    Vec::new()
                                };
                                if known {
                                    params.extend(crate::search::RESULT_PARAMS);
                                }
                                let columns = [
                                    Column::new("Parameter", 144.0).stack(StackPlace::Primary),
                                    Column::new("Type", 96.0).stack(StackPlace::Secondary),
                                    Column::new("What it matches", 160.0).fill().stack(StackPlace::Line),
                                    Column::new("FHIRPath", 160.0).fill().stack(StackPlace::Line),
                                ];
                                let cells: Vec<Vec<Cell>> = params
                                    .iter()
                                    .map(|one| {
                                        vec![
                                            Cell::Mono(one.name.to_owned()),
                                            Cell::Text(one.kind.key().to_owned()),
                                            Cell::Text(one.doc.to_owned()),
                                            Cell::Mono(one.expression.to_owned()),
                                        ]
                                    })
                                    .collect();
                                DataTable::new("parameters", &columns, &cells, &mut views.side)
                                    .variant(TableVariant::List)
                                    .framed(false)
                                    .heads(false)
                                    .stack_layout(StackLayout::Compact)
                                    .stretch(true)
                                    .footer(false)
                                    .words(words_empty(Some("Name a resource type to see its parameters.")))
                                    .show(ui);
                                if let Some(index) = views.side.opened.take()
                                    && let Some(one) = params.get(index)
                                {
                                    let name = one.name;
                                    acts.push(Act::new(move |app| pages::search_add_param(&mut app.app, name)));
                                }
                            }
                        },
                    );
                },
            );
    });
}

/// The search button's width, so the query takes the rest of the row.
fn search_width(ui: &Ui) -> f32 {
    let t = sokkel_egui::tokens(ui.ctx());
    let galley = ui.painter().layout_no_wrap(
        "Search".to_owned(),
        sokkel_egui::TextRole::Action.font(&t),
        egui::Color32::WHITE,
    );
    galley.size().x + 16.0 + 24.0
}

/// The searches kept, each running its query, dragged into another order, and removed.
fn saved_list(ui: &mut Ui, saved: &[crate::server::SavedSearch], state: &mut TableState, acts: &Later) {
    let columns = [
        Column::new("Name", 160.0).fill().stack(StackPlace::Primary),
        Column::new("Query", 224.0).fill_share(1.6).stack(StackPlace::Secondary),
    ];
    let cells: Vec<Vec<Cell>> = saved
        .iter()
        .map(|one| vec![Cell::Text(one.label.clone()), Cell::MutedCode(one.query.clone())])
        .collect();
    let before = saved.iter().map(|one| one.id.clone()).collect::<Vec<_>>();
    let out = DataTable::new("saved-searches", &columns, &cells, state)
        .keys(before.iter().cloned())
        .variant(TableVariant::List)
        .framed(false)
        .heads(false)
        .reorder(true)
        .stack_layout(StackLayout::Compact)
        .stretch(true)
        .footer(false)
        .actions(|_| vec![MenuItem::action("Remove").icon(Icon::Trash2).destructive()])
        .words(words_empty(Some(
            "No search is saved. Save keeps the query above here, to run again; drag a saved search by its grip to order them.",
        )))
        .show(ui);
    if let Some((from, to)) = out.reordered {
        acts.push(Act::new(move |app| pages::search_reorder(&mut app.app, from, to)));
    }
    if let Some((row, _)) = out.action
        && let Some(id) = before.get(row).cloned()
    {
        acts.push(Act::new(move |app| pages::search_remove(&mut app.app, &id)));
    }
    if let Some(index) = state.opened.take()
        && let Some(one) = saved.get(index)
    {
        let text = one.query.clone();
        acts.push(Act::new(move |app| pages::search_set(&mut app.app, &text)));
    }
}

/// The names a query completes from: the types before the `?`, the type's parameters after it.
fn query_words(kind: &str) -> Vec<sokkel_egui::CodeCompletion> {
    let mut words: Vec<sokkel_egui::CodeCompletion> = crate::types::TYPE_NAMES
        .iter()
        .map(|name| sokkel_egui::CodeCompletion {
            label: (*name).to_owned(),
            kind: Some("type".to_owned()),
            detail: Some("resource type".to_owned()),
            ..Default::default()
        })
        .collect();
    let mut params = crate::search::params_of(kind);
    params.extend(crate::search::RESULT_PARAMS);
    words.extend(params.iter().map(|param| sokkel_egui::CodeCompletion {
        label: param.name.to_owned(),
        kind: Some("property".to_owned()),
        detail: Some(param.kind.key().to_owned()),
        info: Some(param.doc.to_owned()),
        apply: None,
    }));
    words
}

/// The editors' text size, as the settings choose it.
pub fn code_size(prefs: &crate::ui::Prefs) -> CodeSize {
    if prefs.size == "md" { CodeSize::Md } else { CodeSize::Sm }
}

/// The search's save dialog while it is open: the query named, kept at the end of Saved.
pub fn save_dialog(ctx: &egui::Context, workbench: &mut WorkbenchApp) {
    let Held::Search(state) = &workbench.app.page else {
        return;
    };
    if !state.saving {
        return;
    }
    let mut label = state.label.clone();
    let busy = state.busy;
    let query = pages::search_query(&workbench.app);
    let why = crate::js::trim(&label).is_empty().then_some("Name the search first.");
    let answer = DialogShell::new("save-search", "Save search", if busy { "Saving" } else { "Save" })
        .description(query)
        .width(384.0)
        .why(why)
        .show(ctx, |ui| {
            ui.add(TextField::new("Name", &mut label).placeholder("Women born before 1970"));
        });
    if let Held::Search(state) = &mut workbench.app.page {
        state.label = label;
        match answer {
            Some(true) => pages::search_save(&mut workbench.app),
            Some(false) => state.saving = false,
            None => {}
        }
    }
}
