//! The pages, as the web's `pages/*.tsx` draw them over each page's own state in `pages`: the
//! datasets, a dataset with its rows, columns and charts, the builder, the saved charts, the
//! boards, a board, a chart opened from a shared link, and a page for an address that leads
//! nowhere.

use eframe::egui::{Ui, vec2};
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, Cell, Column, CommandGroup, DataTable, DropdownMenu, Editor, Empty, Field,
    FilterBar, FormBar, Icon, IconButton, ItemCard, MenuItem, Page, PageHeader, PageState, PanelAsked, Section,
    Segmented, Select, SplitPane, SwitchField, TableState, TableVariant, Tabs, TextRole, WeekStart, tokens,
};

use crate::act::{Act, Asking};
use crate::app::{Views, VisualizerApp, view_icon};
use crate::draw::{Drawn, Mode, chart_view, thumb};
use crate::inputs::palette_options;
use crate::model::{
    Aggregate, Board, BoardItem, ColumnType, DateBin, Field as SpecField, LIMITS, PanelSpan, SortOrder, counted,
};
use crate::pages::builder::Builder;
use crate::pages::{board, charts, dataset, datasets, shared};
use crate::shape::{filter_fields, filter_fields_of, typed_rows};
use crate::views::{VIEWS, view_label};
use crate::visualizer::{Frame, Page as Route};

/// An ISO time as milliseconds since 1970.
pub fn ms_of(iso: &str) -> f64 {
    let days = crate::js::day_number(iso.get(..10).unwrap_or("")).unwrap_or(0);
    let part = |from: usize, to: usize| iso.get(from..to).and_then(|one| one.parse::<f64>().ok()).unwrap_or(0.0);
    days as f64 * 86_400_000.0 + part(11, 13) * 3_600_000.0 + part(14, 16) * 60_000.0 + part(17, 23) * 1000.0
}

/// When a row last changed, said against now in the foreground, sorted by the moment.
fn changed(at: f64, now: f64) -> Cell {
    Cell::Number {
        value: at,
        text: sokkel_egui::relative_time(at - now),
    }
}

/// A small button, outlined unless `primary`, with an icon when given.
pub fn small(label: &str, icon: Option<Icon>, primary: bool) -> Button {
    let button = Button::new(label).size(ButtonSize::Sm).variant(if primary {
        ButtonVariant::Default
    } else {
        ButtonVariant::Outline
    });
    match icon {
        Some(icon) => button.icon(icon),
        None => button,
    }
}

/// The page the address names.
pub fn page(ui: &mut Ui, app: &VisualizerApp, views: &mut Views, acts: &mut Vec<Act>) {
    let frame = Frame::of(&app.backend, &app.route);
    match frame.page() {
        Route::Loading => {
            Page::show(ui, |ui| {
                PageState::loading().show(ui);
            });
        }
        Route::Datasets => datasets_page(ui, app, &frame, views, acts),
        Route::Dataset { id } => dataset_page(ui, app, &frame, &id, views, acts),
        Route::Builder { .. } | Route::Chart { .. } => builder_page(ui, app, &frame, views, acts),
        Route::Charts => charts_page(ui, app, &frame, views, acts),
        Route::Boards => boards_page(ui, app, &frame, views, acts),
        Route::Board { id, presenting } => board_page(ui, app, &frame, &id, presenting, views, acts),
        Route::Shared { payload } => shared_page(ui, app, &payload, views, acts),
        Route::Nothing { message } => {
            Page::show(ui, |ui| {
                PageState::empty(message).show(ui);
            });
        }
    }
}

// --- Datasets --------------------------------------------------------------------------------------

fn datasets_page(ui: &mut Ui, app: &VisualizerApp, frame: &Frame, views: &mut Views, acts: &mut Vec<Act>) {
    let now = app.now() as f64;
    Page::show(ui, |ui| {
        PageHeader::new("Datasets")
            .count(datasets::count(frame.datasets))
            .description("Tables of data to chart. Import a file or paste rows.")
            .show(ui, |ui| {
                if ui.add(small("Import", Some(Icon::Upload), true)).clicked() {
                    acts.push(Act::Import);
                }
            });
        let shown = datasets::search(frame.datasets, &views.datasets_query);
        let columns = [
            Column::new("Name", 224.0).fill_share(2.0),
            Column::new("Columns", 192.0).fill_share(2.0).sortable(false),
            Column::new("Rows", 96.0).numeric(),
            Column::new("Charts", 96.0).numeric(),
            Column::new("Source", 128.0),
            Column::new("Changed", 128.0),
        ];
        let rows: Vec<Vec<Cell>> = shown
            .iter()
            .map(|one| {
                let names: Vec<&str> = one.columns.iter().map(|column| column.name.as_str()).collect();
                let drawn = datasets::charts_of(one, frame.charts);
                vec![
                    Cell::Text(one.name.clone()),
                    Cell::Muted(names.join(", ")),
                    Cell::Number {
                        value: one.rows.len() as f64,
                        text: crate::js::locale_count(one.rows.len()),
                    },
                    Cell::Number {
                        value: drawn as f64,
                        text: drawn.to_string(),
                    },
                    Cell::Text(datasets::source_label(one.source).to_owned()),
                    changed(ms_of(&one.updated_at), now),
                ]
            })
            .collect();
        let state = views.datasets_table.get_or_insert_with(|| TableState::sorted(0, true));
        let keys: Vec<&str> = shown.iter().map(|one| one.id.as_str()).collect();
        let out = DataTable::new("datasets", &columns, &rows, state)
            .keys(keys)
            .search(&mut views.datasets_query)
            .search_placeholder("Search")
            .chevron(true)
            .stretch(true)
            .footer(false)
            .words(rows_words("dataset", "datasets"))
            .actions(|_| {
                vec![
                    MenuItem::action("New chart").icon(Icon::ChartColumn),
                    MenuItem::action("Download CSV").icon(Icon::Download),
                    MenuItem::Separator,
                    MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
                ]
            })
            .empty(|ui| empty_words(ui, "No datasets. Import a file or paste rows to start."))
            .show(ui);
        if let Some(index) = state.opened.take()
            && let Some(one) = shown.get(index)
        {
            acts.push(Act::Go(format!("datasets/{}", one.id)));
        }
        if let Some((index, item)) = out.action
            && let Some(one) = shown.get(index)
        {
            let one = (*one).clone();
            match item {
                0 => acts.push(Act::Go(format!("build/{}", one.id))),
                1 => acts.push(Act::change(move |backend| datasets::download(backend, &one))),
                _ => acts.push(Act::Ask(Asking::DeleteDataset(one.id))),
            }
        }
    });
}

/// A table's count in its bar: `4 datasets`.
fn rows_words(one: &str, many: &str) -> sokkel_egui::TableWords {
    sokkel_egui::TableWords {
        row: format!("{{count}} {one}"),
        rows: format!("{{count}} {many}"),
        ..Default::default()
    }
}

/// What a table says with no rows to show.
fn empty_words(ui: &mut Ui, words: &str) {
    let t = tokens(ui.ctx());
    ui.add_space(24.0);
    ui.vertical_centered(|ui| {
        ui.label(TextRole::Body.rich(&t, words).color(t.color("muted-foreground")));
    });
    ui.add_space(24.0);
}

// --- A dataset ---------------------------------------------------------------------------------------

fn dataset_page(ui: &mut Ui, app: &VisualizerApp, frame: &Frame, id: &str, views: &mut Views, acts: &mut Vec<Act>) {
    let prefs = app.backend.prefs();
    let Some(one) = frame.datasets.iter().find(|one| one.id == id) else {
        Page::show(ui, |ui| {
            PageState::missing(dataset::NO_DATASET).show(ui);
        });
        return;
    };
    let Some(view) = views.dataset.as_mut() else { return };
    let drawn = dataset::drawn(one, frame.charts);
    Page::show(ui, |ui| {
        let header = PageHeader::new(&one.name)
            .count(dataset::count(one))
            .description(&one.description)
            .edit_description("Description", "Add a description")
            .edit_title("Name", |value| {
                crate::js::trim(value)
                    .is_empty()
                    .then(|| "A dataset needs a name.".to_owned())
            })
            .show(ui, |ui| {
                if ui.add(small("New chart", Some(Icon::ChartColumn), true)).clicked() {
                    acts.push(Act::Go(format!("build/{}", one.id)));
                }
                let more = ui.add(IconButton::new(Icon::Ellipsis, "More").outline());
                let items = [
                    MenuItem::action("Download CSV").icon(Icon::Download),
                    MenuItem::action("Duplicate").icon(Icon::Copy),
                    MenuItem::Separator,
                    MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
                ];
                match DropdownMenu::new(&items).end(true).width(208.0).show(&more) {
                    Some(0) => {
                        let one = one.clone();
                        acts.push(Act::change(move |backend| datasets::download(backend, &one)));
                    }
                    Some(1) => {
                        let one = one.clone();
                        acts.push(Act::change(move |backend| backend.duplicate_dataset(&one)));
                    }
                    Some(3) => acts.push(Act::Ask(Asking::DeleteDataset(one.id.clone()))),
                    _ => {}
                }
            });
        if let Some(name) = header.renamed {
            let id = one.id.clone();
            acts.push(Act::change(move |backend| backend.rename_dataset(&id, &name)));
        }
        if let Some(words) = header.described {
            let id = one.id.clone();
            acts.push(Act::change(move |backend| backend.describe_dataset(&id, &words)));
        }
        let tabs = [
            "Rows".to_owned(),
            "Columns".to_owned(),
            dataset::charts_tab(one, frame.charts).trim().to_owned(),
        ];
        ui.add(Tabs::new(&mut view.tab, tabs));
        match view.tab {
            0 => dataset_rows(ui, one, &prefs, view, acts),
            1 => dataset_columns(ui, one, &prefs, view, frame, acts),
            _ => {
                let columns = [Column::new("Chart", 192.0).fill(), Column::new("View", 128.0)];
                let rows: Vec<Vec<Cell>> = drawn
                    .iter()
                    .map(|chart| {
                        vec![
                            Cell::Text(chart.name.clone()),
                            Cell::Text(view_label(chart.spec.view).to_owned()),
                        ]
                    })
                    .collect();
                let id = one.id.clone();
                DataTable::new("dataset-charts", &columns, &rows, &mut view.charts)
                    .variant(TableVariant::List)
                    .chevron(true)
                    .footer(false)
                    .empty(|ui| {
                        let t = tokens(ui.ctx());
                        ui.horizontal(|ui| {
                            ui.add_space(16.0);
                            ui.label(
                                TextRole::Body
                                    .rich(&t, "No chart is drawn from this dataset yet.")
                                    .color(t.color("muted-foreground")),
                            );
                            if ui.add(small("New chart", None, false)).clicked() {
                                acts.push(Act::Go(format!("build/{id}")));
                            }
                        });
                    })
                    .show(ui);
                if let Some(index) = view.charts.opened.take()
                    && let Some(chart) = drawn.get(index)
                {
                    acts.push(Act::Go(format!("charts/{}", chart.id)));
                }
            }
        }
    });
}

fn dataset_rows(
    ui: &mut Ui,
    one: &crate::model::Dataset,
    prefs: &crate::model::Preferences,
    view: &mut crate::app::DatasetView,
    acts: &mut Vec<Act>,
) {
    let typed = typed_rows(one);
    let shown = dataset::shown(one, &typed, &view.filters, &view.query);
    let specs = filter_fields(one, &typed);
    let fields = filter_fields_of(&specs);
    let columns: Vec<Column> = one
        .columns
        .iter()
        .map(|column| {
            let base = match column.kind {
                ColumnType::Number => Column::new(dataset::header(column), 144.0).numeric(),
                ColumnType::Date => Column::new(dataset::header(column), 144.0),
                _ => Column::new(dataset::header(column), 144.0).fill(),
            };
            base.editable(match column.kind {
                ColumnType::Date => Editor::Date(WeekStart::Monday),
                ColumnType::Region => {
                    Editor::Select(crate::seed::REGION_NAMES.iter().map(|one| (*one).to_owned()).collect())
                }
                _ => Editor::Text,
            })
        })
        .collect();
    let rows: Vec<Vec<Cell>> = shown
        .iter()
        .map(|line| {
            let row = one.rows.iter().find(|row| row.id == line.id);
            one.columns
                .iter()
                .map(|column| {
                    let text = row.map_or("", |row| row.text(&column.id));
                    let value = line.get(&column.id);
                    match dataset::cell(text, value, prefs) {
                        dataset::Shown::Empty => Cell::Muted("-".to_owned()),
                        dataset::Shown::Unread(text) => Cell::Toned(text, "critical-ink"),
                        dataset::Shown::Value(written) => match value.as_number() {
                            Some(number) => Cell::Number {
                                value: number,
                                text: written,
                            },
                            None => Cell::Text(written),
                        },
                    }
                })
                .collect()
        })
        .collect();
    let keys: Vec<&str> = shown.iter().map(|line| line.id.as_str()).collect();
    let mut filters = view.filters.clone();
    let mut add_row = false;
    let empty = dataset::empty_rows(!view.filters.is_empty() || !view.query.trim().is_empty());
    let out = DataTable::new("dataset-rows", &columns, &rows, &mut view.rows)
        .keys(keys)
        .search(&mut view.query)
        .search_placeholder("Search")
        .selectable(true)
        .actions_on_hover(true)
        .stretch(true)
        .footer(false)
        .words(rows_words("row", "rows"))
        .toolbar(|ui| {
            FilterBar::new("dataset-filters", &fields, &mut filters).show(ui);
            if ui.add(small("Row", Some(Icon::Plus), false)).clicked() {
                add_row = true;
            }
        })
        .actions(|_| {
            vec![
                MenuItem::action("Duplicate row").icon(Icon::Copy),
                MenuItem::action("Delete row").icon(Icon::Trash2).destructive(),
            ]
        })
        .empty(|ui| empty_words(ui, empty))
        .show(ui);
    view.filters = filters;
    let id = one.id.clone();
    if add_row {
        acts.push(Act::change(move |backend| backend.add_row(&id)));
    }
    if let Some(edit) = out.edited
        && let (Some(line), Some(column)) = (shown.get(edit.row), one.columns.get(edit.column))
    {
        let (id, row, column, value) = (one.id.clone(), line.id.clone(), column.clone(), edit.value);
        acts.push(Act::change(move |backend| {
            backend.save_cell(&id, &row, &column, &value)
        }));
    }
    if let Some((index, item)) = out.action
        && let Some(line) = shown.get(index)
        && let Some(row) = one.rows.iter().find(|row| row.id == line.id).cloned()
    {
        let id = one.id.clone();
        acts.push(Act::change(move |backend| {
            if item == 0 {
                backend.duplicate_row(&id, &row);
            } else {
                backend.remove_rows(&id, std::slice::from_ref(&row.id));
            }
        }));
    }
}

fn dataset_columns(
    ui: &mut Ui,
    one: &crate::model::Dataset,
    prefs: &crate::model::Preferences,
    view: &mut crate::app::DatasetView,
    frame: &Frame,
    acts: &mut Vec<Act>,
) {
    let typed = typed_rows(one);
    let columns = [
        Column::new("Column", 160.0).fill().editable(Editor::Text),
        Column::new("Type", 128.0).editable(Editor::Select(
            ColumnType::ALL.iter().map(|kind| kind.label().to_owned()).collect(),
        )),
        Column::new("Values", 160.0).fill_share(1.2),
        Column::new("Empty", 80.0).numeric(),
        Column::new("Unread", 88.0).numeric(),
    ];
    let rows: Vec<Vec<Cell>> = one
        .columns
        .iter()
        .map(|column| {
            let stats = dataset::stats(one, &typed, column, prefs);
            vec![
                Cell::Text(column.name.clone()),
                Cell::Text(column.kind.label().to_owned()),
                Cell::Text(stats.range),
                Cell::Number {
                    value: stats.empty as f64,
                    text: stats.empty.to_string(),
                },
                if stats.unread > 0 {
                    Cell::Toned(stats.unread.to_string(), "critical-ink")
                } else {
                    Cell::Number {
                        value: 0.0,
                        text: "0".to_owned(),
                    }
                },
            ]
        })
        .collect();
    let mut add = false;
    let out = DataTable::new("dataset-columns", &columns, &rows, &mut view.columns)
        .toolbar(|ui| {
            if ui.add(small("Column", Some(Icon::Plus), false)).clicked() {
                add = true;
            }
        })
        .actions(|_| vec![MenuItem::action("Delete column").icon(Icon::Trash2).destructive()])
        .words(rows_words("column", "columns"))
        .foot("A name or a type changes in place; a type change rewrites the values it can read, and keeps the rest as typed.")
        .footer(false)
        .show(ui);
    let _ = frame;
    if add {
        let one = one.clone();
        acts.push(Act::change(move |backend| backend.add_column(&one)));
    }
    if let Some(edit) = out.edited
        && let Some(column) = one.columns.get(edit.row)
    {
        let id = one.id.clone();
        let at = edit.row;
        if edit.column == 0 {
            if dataset::validate_name(one, column, &edit.value).is_none() {
                let name = edit.value.clone();
                acts.push(Act::change(move |backend| {
                    backend.set_column(&id, at, Some(&name), None)
                }));
            }
        } else if let Some(kind) = ColumnType::ALL.into_iter().find(|kind| kind.label() == edit.value) {
            acts.push(Act::change(move |backend| {
                backend.set_column(&id, at, None, Some(kind))
            }));
        }
    }
    if let Some((index, _)) = out.action
        && let Some(column) = one.columns.get(index)
    {
        acts.push(Act::Ask(Asking::DropColumn {
            dataset: one.id.clone(),
            column: column.id.clone(),
        }));
    }
}

// --- The builder -------------------------------------------------------------------------------------

/// How a builder's change is made: to its draft, now, while the page draws.
fn builder_page(ui: &mut Ui, app: &VisualizerApp, frame: &Frame, views: &mut Views, acts: &mut Vec<Act>) {
    let prefs = app.backend.prefs();
    let Some((_, builder)) = views.builder.as_mut() else {
        Page::show(ui, |ui| {
            PageState::loading().show(ui);
        });
        return;
    };
    let datasets = frame.datasets;
    let Some(dataset) = builder.dataset(datasets).cloned() else {
        Page::show(ui, |ui| {
            PageHeader::new(builder.header()).show(ui, |_| {});
            PageState::empty("There is no dataset to chart. Import one, or reset the demo data, and come back.")
                .show(ui);
        });
        return;
    };
    let maps = &mut views.maps;
    Page::show(ui, |ui| {
        let charted = builder.charted();
        let header = PageHeader::new(builder.header()).back("Back").show(ui, |ui| {
            // drawn from the row's end
            let others: Vec<&Board> = builder.other_boards(&frame.boards);
            let mut add = small("Add to board", Some(Icon::LayoutDashboard), false).trailing(Icon::ChevronDown);
            if builder.chart.is_none() {
                add = add.reason("Save the chart first.");
            }
            let trigger = ui.add(add);
            let mut items: Vec<MenuItem> = Vec::new();
            if !others.is_empty() {
                items.push(MenuItem::Label("Boards".to_owned()));
                items.extend(
                    others
                        .iter()
                        .map(|board| MenuItem::action(&board.name).icon(Icon::LayoutDashboard)),
                );
                items.push(MenuItem::Separator);
            }
            items.push(MenuItem::action("New board").icon(Icon::Plus));
            if builder.chart.is_some()
                && let Some(index) = DropdownMenu::new(&items).end(true).width(224.0).show(&trigger)
            {
                let chart = builder.chart.clone();
                if index == items.len() - 1 {
                    acts.push(Act::NewBoard(chart));
                } else if let (Some(chart), Some(board)) = (chart, others.get(index.saturating_sub(1))) {
                    let board = (*board).clone();
                    acts.push(Act::change(move |backend| backend.add_chart_to(&chart, &board)));
                }
            }
            if ui.add(small("Share", Some(Icon::Share2), false)).clicked()
                && let Some(draft) = &builder.draft
            {
                acts.push(Act::Share(
                    builder.title(),
                    draft.spec.clone(),
                    Box::new(dataset.clone()),
                ));
            }
            let download = ui.add(small("Download", Some(Icon::Download), false).trailing(Icon::ChevronDown));
            let items = [
                MenuItem::action("Chart as PNG").icon(Icon::FileImage).disabled(true),
                MenuItem::action("Data as CSV").icon(Icon::FileSpreadsheet),
            ];
            if DropdownMenu::new(&items).end(true).width(208.0).show(&download) == Some(1) {
                let (builder, datasets) = (builder.clone(), datasets.to_vec());
                acts.push(Act::change(move |backend| builder.download_csv(backend, &datasets)));
            }
            let _ = charted;
        });
        if header.back {
            let back = match &builder.chart {
                None => format!("datasets/{}", dataset.id),
                Some(_) => "charts".to_owned(),
            };
            acts.push(Act::Go(back));
        }
        let gap = tokens(ui.ctx()).density("page-gap");
        let room = (ui.available_height() - FormBar::height(ui.ctx()) - gap).max(200.0);
        let width = ui.available_width();
        // both panes change the draft, one after the other
        let shared = std::cell::RefCell::new(&mut *builder);
        ui.allocate_ui(vec2(width, room), |ui| {
            ui.set_min_height(room);
            SplitPane::new("dataviz-builder")
                .default_share(0.62)
                .min_share(0.4)
                .max_share(0.75)
                .scroll(true)
                .show(
                    ui,
                    |ui| {
                        padded(ui, |ui| {
                            builder_preview(ui, &mut shared.borrow_mut(), &dataset, &prefs, maps)
                        });
                    },
                    |ui| {
                        padded(ui, |ui| {
                            builder_options(ui, &mut shared.borrow_mut(), datasets, &dataset, &prefs)
                        });
                    },
                );
        });
        FormBar::new().note(builder.note()).show(ui, |ui| {
            let mut save = small("Save", None, true);
            if let Some(reason) = builder.save_held() {
                save = save.reason(reason);
            }
            if ui.add(save).clicked() {
                let mut next = builder.clone();
                acts.push(Act::change(move |backend| next.save(backend, false)));
            }
            if builder.chart.is_some() {
                let mut again = small("Save as new", None, false);
                if let Some(reason) = builder.save_as_new_held() {
                    again = again.reason(reason);
                }
                if ui.add(again).clicked() {
                    let mut next = builder.clone();
                    acts.push(Act::change(move |backend| next.save(backend, true)));
                }
                let mut discard = small("Discard", None, false);
                if !builder.dirty() {
                    discard = discard.reason("Nothing has changed yet.");
                }
                if ui.add(discard).clicked() {
                    builder.discard();
                }
            }
        });
    });
}

fn builder_preview(
    ui: &mut Ui,
    builder: &mut Builder,
    dataset: &crate::model::Dataset,
    prefs: &crate::model::Preferences,
    maps: &mut std::collections::HashMap<String, sokkel_egui_maps::MapView>,
) {
    let Some(draft) = builder.draft.clone() else { return };
    let t = tokens(ui.ctx());
    ui.spacing_mut().item_spacing.y = t.density("page-gap") - 8.0;
    let title = builder.title();
    let map = maps.entry("builder".to_owned()).or_insert_with(new_map);
    let mut table = TableState::default();
    let drawn = Drawn {
        dataset,
        spec: &draft.spec,
        title: &title,
        subtitle: Some(&dataset.name),
        prefs,
        filters: &[],
        mode: Mode::Page(320.0),
        large: false,
    };
    chart_view(ui, &drawn, Vec::new(), map, &mut table);
    if let Some(note) = builder.preview_note(std::slice::from_ref(dataset)) {
        ui.label(TextRole::Help.rich(&t, note).color(t.color("muted-foreground")));
    }
    let suggestions = builder.suggestions(std::slice::from_ref(dataset), prefs);
    if suggestions.is_empty() {
        return;
    }
    Section::new("Suggested views")
        .description("Other ways to draw the columns picked. Pick one to try it; Save keeps it.")
        .show(ui, |ui| {
            let mut picked = None;
            sokkel_egui::Columns::new(4)
                .min(192.0)
                .gap(12.0)
                .show(ui, suggestions.len(), |ui, index, width| {
                    let one = &suggestions[index];
                    let mut card = ItemCard::new(&one.title)
                        .selectable(true)
                        .description(&one.reason)
                        .icon(view_icon(one.spec.view));
                    if crate::chart_view::is_charted(one.spec.view) {
                        let spec = one.spec.clone();
                        card = card.media(move |ui| {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                ui.vertical(|ui| {
                                    ui.set_width(ui.available_width() - 8.0);
                                    thumb(ui, dataset, &spec, prefs, 96.0);
                                });
                            });
                        });
                    }
                    if card.show(ui, width, 0.0).clicked() {
                        picked = Some(one.spec.clone());
                    }
                });
            if let Some(spec) = picked {
                builder.set_spec(|draft| *draft = spec);
            }
        });
}

fn builder_options(
    ui: &mut Ui,
    builder: &mut Builder,
    datasets: &[crate::model::Dataset],
    dataset: &crate::model::Dataset,
    prefs: &crate::model::Preferences,
) {
    let Some(draft) = builder.draft.clone() else { return };
    let t = tokens(ui.ctx());
    ui.spacing_mut().item_spacing.y = t.density("page-gap") - 8.0;
    let spec = draft.spec.clone();
    Section::new("Chart").show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 12.0;
        let mut name = draft.name.clone();
        let response = Field::new("Name").show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            ui.add(sokkel_egui::Input::new(&mut name).placeholder("Untitled chart"))
        });
        if name != draft.name {
            builder.set_name(&name);
        }
        let _ = response;
        Field::new("Dataset")
            .description("Another dataset starts the view over from its columns.")
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                let mut at = datasets.iter().position(|one| one.id == draft.dataset_id).unwrap_or(0);
                let before = at;
                let width = ui.available_width();
                ui.add(Select::new(&mut at, datasets.iter().map(|one| one.name.clone()), "Dataset").width(width));
                if at != before {
                    let id = datasets[at].id.clone();
                    builder.set_dataset(datasets, prefs, &id);
                }
            });
    });
    Section::new("View").show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 12.0;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for view in VIEWS {
                let pressed = spec.view == view.value;
                let button = small(view.label, Some(view_icon(view.value)), false).pressed(pressed);
                let response = sokkel_egui::tooltip(ui.add(button), view.description);
                if response.clicked() && !pressed {
                    builder.set_view(std::slice::from_ref(dataset), view.value);
                }
            }
        });
    });
    Section::new("Columns").show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 12.0;
        for slot in builder.slot_fields(std::slice::from_ref(dataset)) {
            let mut field = Field::new(slot.label).hint(slot.hint);
            if let Some(description) = slot.description {
                field = field.description(description);
            }
            field.show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                let mut at = slot.options.iter().position(|one| one.value == slot.value).unwrap_or(0);
                let before = at;
                let width = ui.available_width();
                let mut select =
                    Select::new(&mut at, slot.options.iter().map(|one| one.label.clone()), slot.label).width(width);
                if let Some(placeholder) = &slot.placeholder {
                    select = select.placeholder(placeholder);
                }
                ui.add_enabled(!slot.disabled, select);
                if at != before
                    && let Some(option) = slot.options.get(at)
                {
                    let field = match slot.field {
                        "x" => SpecField::X,
                        "y" => SpecField::Y,
                        "series" => SpecField::Series,
                        "size" => SpecField::Size,
                        _ => SpecField::Color,
                    };
                    builder.set_slot(field, &option.value);
                }
            });
        }
    });
    Section::new("Figures").show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 12.0;
        if let Some((aggregate, numbers)) = builder.combine(std::slice::from_ref(dataset)) {
            Field::new("Combine")
                .description("How the rows that fall in one place make one figure.")
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 12.0;
                    let mut at = Aggregate::ALL.iter().position(|one| *one == aggregate).unwrap_or(0);
                    let before = at;
                    let off = Aggregate::ALL.iter().map(|one| {
                        (!numbers && *one != Aggregate::Count)
                            .then(|| "The dataset has no number column to combine.".to_owned())
                    });
                    ui.add(
                        Segmented::new(&mut at, Aggregate::ALL.iter().map(|one| one.label()))
                            .disabled(off)
                            .fill(true),
                    );
                    if at != before {
                        builder.set_spec(|spec| spec.aggregate = Aggregate::ALL[at]);
                    }
                });
        }
        if builder.shows_bin(std::slice::from_ref(dataset)) {
            Field::new("Group dates by").show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                let mut at = DateBin::ALL.iter().position(|one| *one == spec.bin).unwrap_or(0);
                let before = at;
                ui.add(Segmented::new(&mut at, DateBin::ALL.iter().map(|one| one.label())).fill(true));
                if at != before {
                    builder.set_spec(|spec| spec.bin = DateBin::ALL[at]);
                }
            });
        }
        if let Some(labels) = builder.sort_labels(std::slice::from_ref(dataset)) {
            Field::new("Sort").show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                let mut at = SortOrder::ALL.iter().position(|one| *one == spec.sort).unwrap_or(0);
                let before = at;
                ui.add(Segmented::new(&mut at, labels).fill(true));
                if at != before {
                    builder.set_spec(|spec| spec.sort = SortOrder::ALL[at]);
                }
            });
            Field::new("Show").show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                let current = crate::js::number_to_string(spec.limit);
                let mut at = LIMITS.iter().position(|(value, _)| *value == current).unwrap_or(0);
                let before = at;
                ui.add(Select::new(&mut at, LIMITS.iter().map(|(_, label)| *label), "Show").width(128.0));
                if at != before {
                    let limit: f64 = LIMITS[at].0.parse().unwrap_or(0.0);
                    builder.set_spec(|spec| spec.limit = limit);
                }
            });
        }
    });
    Section::new("Look").show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 12.0;
        let options = palette_options(Some((
            Builder::default_palette_label(prefs),
            crate::model::PALETTES
                .iter()
                .find(|one| one.id == prefs.palette)
                .and_then(|one| one.tokens),
        )));
        Field::new("Colors").show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            let mut at = options.iter().position(|one| one.value == spec.palette).unwrap_or(0);
            let before = at;
            let width = ui.available_width();
            ui.add(
                Select::new(&mut at, options.iter().map(|one| one.label.clone()), "Colors")
                    .width(width)
                    .dot_rows(
                        options
                            .iter()
                            .map(|one| one.swatches.iter().map(|token| t.color(token)).collect()),
                    ),
            );
            if at != before {
                let palette = options[at].value.clone();
                builder.set_spec(|spec| spec.palette = palette);
            }
        });
        if let Some((stack, held)) = builder.stack_switch() {
            let mut on = stack;
            ui.add(
                SwitchField::new(&mut on, "Stack the series")
                    .explain("Each series on top of the last, so the stack shows the total.")
                    .enabled(!held),
            );
            if on != stack {
                builder.set_spec(|spec| spec.stack = on);
            }
        }
        if let Some(horizontal) = builder.horizontal_switch() {
            let mut on = horizontal;
            ui.add(SwitchField::new(&mut on, "Bars lying down"));
            if on != horizontal {
                builder.set_spec(|spec| spec.horizontal = on);
            }
        }
    });
}

/// A pane's content 4 in from its edges, as the web's `p-1`.
fn padded(ui: &mut Ui, content: impl FnOnce(&mut Ui)) {
    eframe::egui::Frame::new().inner_margin(4).show(ui, |ui| {
        ui.set_width(ui.available_width());
        content(ui);
    });
}

/// A map's view as the visualizer draws one: no basemap, framed on the regions as it is first drawn.
pub fn new_map() -> sokkel_egui_maps::MapView {
    sokkel_egui_maps::MapView::new(6.4, -9.4, 6.0).framed(false)
}

// --- The saved charts --------------------------------------------------------------------------------

fn charts_page(ui: &mut Ui, app: &VisualizerApp, frame: &Frame, views: &mut Views, acts: &mut Vec<Act>) {
    let prefs = app.backend.prefs();
    let now = app.now() as f64;
    Page::show(ui, |ui| {
        PageHeader::new("Charts")
            .count(charts::count(frame.charts))
            .description(
                "Charts saved from the datasets. Open one to change it, share it as a link, or place it on a board.",
            )
            .show(ui, |ui| {
                let mut new = small("New", Some(Icon::Plus), true).trailing(Icon::ChevronDown);
                if frame.datasets.is_empty() {
                    new = new.enabled(false);
                }
                let trigger = ui.add(new);
                let mut items = vec![MenuItem::Label("From the dataset".to_owned())];
                items.extend(
                    frame
                        .datasets
                        .iter()
                        .map(|one| MenuItem::action(&one.name).icon(Icon::Database)),
                );
                if let Some(index) = DropdownMenu::new(&items).end(true).width(240.0).show(&trigger)
                    && let Some(one) = frame.datasets.get(index.saturating_sub(1))
                {
                    acts.push(Act::Go(format!("build/{}", one.id)));
                }
            });
        let shown = charts::search(frame.charts, frame.datasets, &views.charts_query);
        let columns = [
            Column::new("Name", 224.0).fill_share(2.0),
            Column::new("View", 112.0),
            Column::new("Dataset", 160.0).fill(),
            Column::new("Boards", 128.0).fill().sortable(false),
            Column::new("Saved", 128.0),
        ];
        let rows: Vec<Vec<Cell>> = shown
            .iter()
            .map(|chart| {
                vec![
                    Cell::IconName {
                        icon: view_icon(chart.spec.view),
                        name: chart.name.clone(),
                    },
                    Cell::Text(view_label(chart.spec.view).to_owned()),
                    Cell::Text(
                        frame
                            .datasets
                            .iter()
                            .find(|one| one.id == chart.dataset_id)
                            .map_or_else(|| "Deleted".to_owned(), |one| one.name.clone()),
                    ),
                    Cell::Text(charts::boards_cell(chart, &frame.boards)),
                    changed(ms_of(&chart.updated_at), now),
                ]
            })
            .collect();
        let state = views.charts_table.get_or_insert_with(|| {
            let mut state = TableState::sorted(4, false);
            state.grid = true;
            state
        });
        let keys: Vec<&str> = shown.iter().map(|chart| chart.id.as_str()).collect();
        let datasets = frame.datasets;
        let out = DataTable::new("charts", &columns, &rows, state)
            .keys(keys)
            .search(&mut views.charts_query)
            .search_placeholder("Search")
            .stretch(true)
            .footer(false)
            .words(rows_words("chart", "charts"))
            .cards(256.0, 248.0, |ui, index, width| {
                let chart = shown[index];
                let dataset = datasets.iter().find(|one| one.id == chart.dataset_id);
                let icon = view_icon(chart.spec.view);
                ItemCard::new(&chart.name)
                    .selectable(true)
                    .title_lines(1)
                    .description(charts::card_description(chart, datasets))
                    .icon(icon)
                    .media(|ui| match dataset {
                        Some(dataset) if crate::chart_view::is_charted(chart.spec.view) => {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                ui.vertical(|ui| {
                                    ui.set_width(ui.available_width() - 8.0);
                                    thumb(ui, dataset, &chart.spec, &prefs, 128.0);
                                });
                            });
                        }
                        _ => {
                            let t = tokens(ui.ctx());
                            let rect = ui.max_rect();
                            sokkel_egui::icons::paint(
                                ui.painter(),
                                rect.center(),
                                icon,
                                40.0,
                                t.color("muted-foreground"),
                            );
                        }
                    })
                    .show(ui, width, 0.0)
            })
            .actions(|index| {
                let chart = shown[index];
                let has = datasets.iter().any(|one| one.id == chart.dataset_id);
                vec![
                    MenuItem::action("Edit").icon(Icon::PencilLine),
                    MenuItem::action("Share").icon(Icon::Share2),
                    MenuItem::action("Duplicate").icon(Icon::Copy),
                    MenuItem::action("Open its dataset").icon(Icon::Database).disabled(!has),
                    MenuItem::Separator,
                    MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
                ]
            })
            .empty(|ui| empty_words(ui, "No saved charts. Open a dataset and make one."))
            .show(ui);
        if let Some(index) = state.opened.take()
            && let Some(chart) = shown.get(index)
        {
            acts.push(Act::Go(format!("charts/{}", chart.id)));
        }
        if let Some((index, item)) = out.action
            && let Some(chart) = shown.get(index)
        {
            let chart = (*chart).clone();
            match item {
                0 => acts.push(Act::Go(format!("charts/{}", chart.id))),
                1 => {
                    if let Some(dataset) = datasets.iter().find(|one| one.id == chart.dataset_id) {
                        acts.push(Act::Share(
                            chart.name.clone(),
                            chart.spec.clone(),
                            Box::new(dataset.clone()),
                        ));
                    }
                }
                2 => acts.push(Act::change(move |backend| backend.duplicate_chart(&chart))),
                3 => acts.push(Act::Go(format!("datasets/{}", chart.dataset_id))),
                _ => acts.push(Act::Ask(Asking::DeleteChart(chart.id))),
            }
        }
    });
}

// --- The boards --------------------------------------------------------------------------------------

fn boards_page(ui: &mut Ui, app: &VisualizerApp, frame: &Frame, views: &mut Views, acts: &mut Vec<Act>) {
    let now = app.now() as f64;
    let prefs = app.backend.prefs();
    Page::show(ui, |ui| {
        PageHeader::new("Boards")
            .count(crate::pages::boards::count(&frame.boards))
            .description(
                "Saved charts arranged on a page, filtered together and presented full screen. Drag a board by its grip to order them.",
            )
            .show(ui, |ui| {
                if ui.add(small("New", Some(Icon::Plus), true)).clicked() {
                    acts.push(Act::NewBoard(None));
                }
            });
        let shown = views.boards.shown(&frame.boards);
        let columns = [
            Column::new("Name", 192.0).fill(),
            Column::new("Description", 224.0).fill_share(2.0).sortable(false),
            Column::new("Charts", 96.0).numeric(),
            Column::new("Changed", 128.0),
        ];
        let rows: Vec<Vec<Cell>> = shown
            .iter()
            .map(|board| {
                vec![
                    Cell::Text(board.name.clone()),
                    Cell::Muted(board.description.clone()),
                    Cell::Number {
                        value: board.items.len() as f64,
                        text: board.items.len().to_string(),
                    },
                    changed(ms_of(&board.updated_at), now),
                ]
            })
            .collect();
        let keys: Vec<&str> = shown.iter().map(|board| board.id.as_str()).collect();
        let out = DataTable::new("boards", &columns, &rows, &mut views.boards_table)
            .keys(keys)
            .reorder(true)
            .chevron(true)
            .stretch(true)
            .footer(false)
            .bar(true)
            .words(rows_words("board", "boards"))
            .actions(|_| {
                vec![
                    MenuItem::action("Present").icon(Icon::Presentation),
                    MenuItem::action("Edit details").icon(Icon::PencilLine),
                    MenuItem::Separator,
                    MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
                ]
            })
            .empty(|ui| empty_words(ui, "No boards yet. Make one and add saved charts to it."))
            .show(ui);
        if let Some(index) = views.boards_table.opened.take()
            && let Some(board) = shown.get(index)
        {
            acts.push(Act::Go(format!("boards/{}", board.id)));
        }
        if let Some((from, to)) = out.reordered {
            // the order is held here at once and kept in the preferences behind it
            let mut order = shown.clone();
            sokkel_egui::move_item(&mut order, from, to);
            views.boards.order = Some(order);
            let prefs = prefs.id.clone();
            let before = shown.clone();
            acts.push(Act::change(move |backend| {
                backend.reorder_boards(&before, from, to, &prefs);
            }));
        }
        if let Some((index, item)) = out.action
            && let Some(board) = shown.get(index)
        {
            match item {
                0 => acts.push(Act::Go(format!("boards/{}/present", board.id))),
                1 => acts.push(Act::EditBoard(board.id.clone())),
                _ => acts.push(Act::Ask(Asking::DeleteBoard(board.id.clone()))),
            }
        }
    });
}

// --- A board -----------------------------------------------------------------------------------------

/// A board's item as the arrangement places it.
impl sokkel_egui::ArrangeItem for BoardItem {
    fn id(&self) -> &str {
        &self.id
    }

    fn span(&self) -> sokkel_egui::PanelSpan {
        match self.span {
            PanelSpan::Third => sokkel_egui::PanelSpan::Third,
            PanelSpan::Half => sokkel_egui::PanelSpan::Half,
            PanelSpan::TwoThirds => sokkel_egui::PanelSpan::TwoThirds,
            PanelSpan::Full => sokkel_egui::PanelSpan::Full,
        }
    }

    fn set_span(&mut self, span: sokkel_egui::PanelSpan) {
        self.span = match span {
            sokkel_egui::PanelSpan::Sixth | sokkel_egui::PanelSpan::Quarter | sokkel_egui::PanelSpan::Third => {
                PanelSpan::Third
            }
            sokkel_egui::PanelSpan::Half => PanelSpan::Half,
            sokkel_egui::PanelSpan::TwoThirds => PanelSpan::TwoThirds,
            sokkel_egui::PanelSpan::Full => PanelSpan::Full,
        };
    }
}

fn board_page(
    ui: &mut Ui,
    app: &VisualizerApp,
    frame: &Frame,
    id: &str,
    presenting: bool,
    views: &mut Views,
    acts: &mut Vec<Act>,
) {
    let prefs = app.backend.prefs();
    let Some(one) = frame.boards.iter().find(|one| one.id == id) else {
        Page::show(ui, |ui| {
            PageState::missing(board::NO_BOARD).show(ui);
        });
        return;
    };
    let Some(page) = views.board.as_mut() else { return };
    let maps = &mut views.maps;
    let _ = presenting;
    Page::scroll(ui, |ui| {
        let arranging = page.arranging();
        let items = page.items(one);
        let mut header = PageHeader::new(&one.name)
            .count(page.count(one))
            .edit_title("Name", |value| {
                crate::js::trim(value)
                    .is_empty()
                    .then(|| "A board needs a name.".to_owned())
            });
        if let Some(description) = page.description(one) {
            header = header.description(description);
        }
        let said = header.show(ui, |ui| {
            if arranging {
                if ui.add(small("Done", Some(Icon::Check), true)).clicked() {
                    let board = one.clone();
                    let mut next = page.clone();
                    page.draft = None;
                    acts.push(Act::change(move |backend| next.done(backend, &board)));
                }
                if ui.add(small("Cancel", None, false)).clicked() {
                    page.cancel();
                }
                return;
            }
            if ui.add(small("Add chart", Some(Icon::Plus), true)).clicked() {
                acts.push(Act::AddChart(one.id.clone()));
            }
            if ui.add(small("Edit", Some(Icon::Pencil), false)).clicked() {
                page.edit(one);
            }
            let mut present = small("Present", Some(Icon::Presentation), false);
            if items.is_empty() {
                present = present.reason("Add a chart first.");
            }
            if ui.add(present).clicked() {
                acts.push(Act::Go(format!("boards/{}/present", one.id)));
            }
            let more = ui.add(IconButton::new(Icon::Ellipsis, "More").outline());
            let menu = [
                MenuItem::action("Edit details").icon(Icon::PencilLine),
                MenuItem::action("Duplicate").icon(Icon::CopyPlus),
                MenuItem::Separator,
                MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
            ];
            match DropdownMenu::new(&menu).end(true).width(208.0).show(&more) {
                Some(0) => acts.push(Act::EditBoard(one.id.clone())),
                Some(1) => {
                    let (page, board) = (page.clone(), one.clone());
                    acts.push(Act::change(move |backend| page.duplicate(backend, &board)));
                }
                Some(3) => acts.push(Act::Ask(Asking::DeleteBoard(one.id.clone()))),
                _ => {}
            }
        });
        if let Some(name) = said.renamed {
            let id = one.id.clone();
            acts.push(Act::change(move |backend| backend.rename_board(&id, &name)));
        }
        if items.is_empty() && !arranging {
            let edit = Empty::new("No charts yet")
                .icon(Icon::LayoutDashboard)
                .description(
                    "Drag saved charts onto the board to arrange them here, filter them together and present them.",
                )
                .show(ui, |ui| ui.add(small("Edit", Some(Icon::Pencil), true)).clicked());
            if edit {
                if frame.charts.is_empty() {
                    acts.push(Act::AddChart(one.id.clone()));
                } else {
                    page.edit(one);
                }
            }
            return;
        }
        let placed = page.placed(one, frame.charts, frame.datasets);
        if !items.is_empty() {
            let used = board::BoardPage::used(&placed);
            let specs = board::board_fields(&used);
            let fields = filter_fields_of(&specs);
            let mut filters = page.filters.clone();
            let t = tokens(ui.ctx());
            let mut changed = false;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                changed = FilterBar::new("board-filters", &fields, &mut filters).show(ui);
                ui.label(
                    TextRole::Help
                        .rich(&t, page.filter_note())
                        .color(t.color("muted-foreground")),
                );
            });
            // the grid 12 under the filters, as the web's `Stack gap="md"`
            ui.add_space(12.0 - t.density("page-gap"));
            if changed {
                page.filters = filters.clone();
                let id = one.id.clone();
                acts.push(Act::change(move |backend| backend.set_board_filters(&id, &filters)));
            }
        }
        let mut layout = items.clone();
        let label = |item: &BoardItem| {
            placed
                .iter()
                .find(|entry| entry.item.id == item.id)
                .and_then(|entry| entry.chart)
                .map_or_else(|| "A deleted chart".to_owned(), |chart| chart.name.clone())
        };
        let palette: Vec<sokkel_egui::DashboardWidget> = page
            .palette(one, frame.charts)
            .into_iter()
            .map(|chart| {
                sokkel_egui::DashboardWidget::new(&chart.id, &chart.name)
                    .icon(view_icon(chart.spec.view))
                    .span(sokkel_egui::PanelSpan::Half)
            })
            .collect();
        let spans = [
            sokkel_egui::PanelSpan::Third,
            sokkel_egui::PanelSpan::Half,
            sokkel_egui::PanelSpan::TwoThirds,
            sokkel_egui::PanelSpan::Full,
        ];
        let mut asked: Option<(String, &'static str)> = None;
        let mut tables: std::collections::HashMap<String, TableState> = std::collections::HashMap::new();
        let arranged = sokkel_egui::DashboardArrange::new(("board", &one.id), &mut layout, &label)
            .arranging(arranging)
            .palette(&palette)
            .spans(&spans)
            .words(sokkel_egui::DashboardArrangeWords {
                end: "Drop a chart here to add it at the end".to_owned(),
                ..Default::default()
            })
            .show(ui, |ui, item| {
                let Some(entry) = placed.iter().find(|entry| entry.item.id == item.id) else {
                    return;
                };
                let (menu, names) = panel_menu(entry, arranging);
                if let Some(PanelAsked::Item(index)) = board_panel(ui, page, entry, &prefs, menu, maps, &mut tables)
                    && let Some(name) = names.get(index)
                {
                    asked = Some((item.id.clone(), name));
                }
            });
        if arranging && (arranged.changed || layout != items) {
            page.change(layout.clone());
        }
        if let Some((chart, at)) = arranged.added {
            let (mut next, board) = (page.clone(), one.clone());
            acts.push(Act::change(move |backend| {
                next.add(backend, &board, &chart, PanelSpan::Half, at)
            }));
        }
        if let Some((item, name)) = asked
            && let Some(entry) = placed.iter().find(|entry| entry.item.id == item)
        {
            match Some(name) {
                Some("edit-chart") => acts.push(Act::Go(format!("charts/{}", entry.item.chart_id))),
                Some("csv") => {
                    let (page, entry) = (page.clone(), entry.clone());
                    let (chart, dataset) = (entry.chart.cloned(), entry.dataset.cloned());
                    let item = entry.item.clone();
                    acts.push(Act::change(move |backend| {
                        let placed = board::Placed {
                            item,
                            chart: chart.as_ref(),
                            dataset: dataset.as_ref(),
                        };
                        page.download_csv(backend, &placed);
                    }));
                }
                Some("edit-board") => page.edit(one),
                Some("remove") => {
                    if arranging {
                        page.change(items.iter().filter(|one| one.id != entry.item.id).cloned().collect());
                    } else {
                        let (board, items, item, chart) = (
                            one.clone(),
                            items.clone(),
                            entry.item.id.clone(),
                            entry.chart.map(|chart| chart.name.clone()),
                        );
                        acts.push(Act::change(move |backend| {
                            backend.take_off(&board, &items, &item, chart.as_deref());
                        }));
                    }
                }
                _ => {}
            }
        }
    });
}

/// One item of a board, drawn as its chart's view under the board's filters.
/// A board's panel's menu, as the web's: Edit chart, Download CSV, Edit the board, then Remove
/// from board; and what each entry is.
fn panel_menu(entry: &board::Placed, arranging: bool) -> (Vec<MenuItem>, Vec<&'static str>) {
    let mut menu = Vec::new();
    let mut names = Vec::new();
    // the chart's editor is a page of its own, which would leave the draft behind
    if entry.chart.is_some() && !arranging {
        menu.push(MenuItem::action("Edit chart").icon(Icon::PencilLine));
        names.push("edit-chart");
    }
    if entry.chart.is_some() && entry.dataset.is_some() {
        menu.push(MenuItem::action("Download CSV").icon(Icon::FileSpreadsheet));
        names.push("csv");
    }
    menu.push(
        MenuItem::action("Edit the board")
            .icon(Icon::Pencil)
            .disabled(arranging),
    );
    names.push("edit-board");
    menu.push(MenuItem::Separator);
    names.push("separator");
    menu.push(MenuItem::action("Remove from board").icon(Icon::Trash2).destructive());
    names.push("remove");
    (menu, names)
}

fn board_panel(
    ui: &mut Ui,
    page: &board::BoardPage,
    entry: &board::Placed,
    prefs: &crate::model::Preferences,
    menu: Vec<MenuItem>,
    maps: &mut std::collections::HashMap<String, sokkel_egui_maps::MapView>,
    tables: &mut std::collections::HashMap<String, TableState>,
) -> Option<PanelAsked> {
    match page.panel(entry) {
        board::Panel::Chart {
            title,
            subtitle,
            filters,
            ..
        } => {
            let (Some(chart), Some(dataset)) = (entry.chart, entry.dataset) else {
                return None;
            };
            let drawn = Drawn {
                dataset,
                spec: &chart.spec,
                title: &title,
                subtitle: Some(&subtitle),
                prefs,
                filters: &filters,
                mode: Mode::Panel,
                large: false,
            };
            let map = maps.entry(format!("item-{}", entry.item.id)).or_insert_with(new_map);
            let table = tables.entry(entry.item.id.clone()).or_default();
            chart_view(ui, &drawn, menu, map, table)
        }
        board::Panel::Missing { title, text } => missing_panel(ui, &title, &text, menu),
    }
}

/// A board's item whose chart or dataset is gone.
fn missing_panel(ui: &mut Ui, title: &str, text: &str, menu: Vec<MenuItem>) -> Option<PanelAsked> {
    sokkel_egui::PanelMenu::new(menu).show(ui, |ui| {
        sokkel_egui::DashboardPanel::new(title).show(ui, |ui| {
            let t = tokens(ui.ctx());
            ui.label(TextRole::Body.rich(&t, text).color(t.color("muted-foreground")));
        });
    })
}

// --- A shared chart ----------------------------------------------------------------------------------

fn shared_page(ui: &mut Ui, app: &VisualizerApp, payload: &str, views: &mut Views, acts: &mut Vec<Act>) {
    let prefs = app.backend.prefs();
    let read = crate::share::unpack_chart(payload);
    Page::scroll(ui, |ui| match read {
        Err(error) => {
            PageHeader::new("Shared chart").show(ui, |_| {});
            PageState::refused(error).show(ui);
        }
        Ok(shared) => {
            let dataset = crate::share::shared_dataset(&shared, "shared", &app.backend.now_iso());
            PageHeader::new(shared::title(&shared))
                .description(shared::description(&dataset))
                .show(ui, |ui| {
                    if ui.add(small("Keep a copy", Some(Icon::Save), true)).clicked() {
                        let shared = shared.clone();
                        acts.push(Act::change(move |backend| backend.keep_shared(&shared)));
                    }
                    if ui.add(small("CSV", Some(Icon::Download), false)).clicked() {
                        let dataset = dataset.clone();
                        acts.push(Act::change(move |backend| datasets::download(backend, &dataset)));
                    }
                });
            let map = views.maps.entry("shared".to_owned()).or_insert_with(new_map);
            let mut table = TableState::default();
            let drawn = Drawn {
                dataset: &dataset,
                spec: &shared.spec,
                title: &shared.name,
                subtitle: Some(&dataset.name),
                prefs: &prefs,
                filters: &[],
                mode: Mode::Page(384.0),
                large: false,
            };
            chart_view(ui, &drawn, Vec::new(), map, &mut table);
            Section::new("Data")
                .meta(counted(dataset.rows.len(), "row"))
                .description(&dataset.description)
                .flush(true)
                .show(ui, |ui| {
                    let columns: Vec<Column> = dataset
                        .columns
                        .iter()
                        .map(|column| {
                            if column.kind == ColumnType::Number {
                                Column::new(&column.name, 128.0).numeric()
                            } else {
                                Column::new(&column.name, 144.0).fill()
                            }
                        })
                        .collect();
                    let rows: Vec<Vec<Cell>> = dataset
                        .rows
                        .iter()
                        .map(|row| {
                            dataset
                                .columns
                                .iter()
                                .map(|column| Cell::Text(row.text(&column.id).to_owned()))
                                .collect()
                        })
                        .collect();
                    let state = views.datasets_table.get_or_insert_with(TableState::default);
                    DataTable::new("shared-rows", &columns, &rows, state)
                        .framed(false)
                        .footer(false)
                        .height(320.0)
                        .show(ui);
                });
        }
    });
}

// --- The page's own commands -------------------------------------------------------------------------

/// The palette's commands of the page shown, as the web's pages add them with `useCommands`.
pub fn page_commands(app: &VisualizerApp) -> Vec<CommandGroup> {
    let section = app.route.first().map(String::as_str);
    match section {
        Some("build") | Some("charts") if app.views.builder.is_some() => {
            let Some((_, builder)) = &app.views.builder else {
                return Vec::new();
            };
            if builder.draft.is_none() {
                return Vec::new();
            }
            let mut items = Vec::new();
            if builder.save_held().is_none() {
                items.push(MenuItem::action("Save chart").icon(Icon::Save).id("page:chart-save"));
            }
            items.push(
                MenuItem::action("Share chart")
                    .icon(Icon::Share2)
                    .hint("A link with its data")
                    .id("page:chart-share"),
            );
            items.push(
                MenuItem::action("Download chart data as CSV")
                    .icon(Icon::FileSpreadsheet)
                    .id("page:chart-csv"),
            );
            vec![CommandGroup::new("This chart", items)]
        }
        Some("boards") if app.route.len() > 1 => {
            let Some(id) = app.route.get(1) else { return Vec::new() };
            let Some(board) = app.backend.boards.seen_get(id) else {
                return Vec::new();
            };
            let presenting = app.route.get(2).map(String::as_str) == Some("present");
            let arranging = app.views.board.as_ref().is_some_and(|page| page.arranging());
            let filtered = app.views.board.as_ref().is_some_and(|page| !page.filters.is_empty());
            let mut items = vec![
                MenuItem::action(if presenting { "Stop presenting" } else { "Present board" })
                    .icon(Icon::Presentation)
                    .id("page:board-present"),
                MenuItem::action(if arranging {
                    "Keep the edits to the board"
                } else {
                    "Edit the board"
                })
                .icon(Icon::Pencil)
                .id("page:board-arrange"),
            ];
            if arranging {
                items.push(
                    MenuItem::action("Throw the edits to the board away")
                        .icon(Icon::X)
                        .id("page:board-cancel"),
                );
            } else {
                items.push(
                    MenuItem::action("Add a chart to the board")
                        .icon(Icon::Plus)
                        .id("page:board-add"),
                );
            }
            if filtered {
                items.push(
                    MenuItem::action("Clear the board\u{2019}s filters")
                        .icon(Icon::X)
                        .id("page:board-clear"),
                );
            }
            vec![CommandGroup::new(board.name.clone(), items)]
        }
        _ => Vec::new(),
    }
}

/// Runs one of the page's own commands.
pub fn run_page_command(app: &mut VisualizerApp, command: &str) {
    match command {
        "chart-save" | "chart-csv" | "chart-share" => {
            let Some((_, builder)) = app.views.builder.clone() else {
                return;
            };
            let datasets = app.backend.datasets.seen().to_vec();
            match command {
                "chart-save" => {
                    let mut builder = builder;
                    builder.save(&mut app.backend, false);
                }
                "chart-csv" => builder.download_csv(&mut app.backend, &datasets),
                _ => {
                    if let (Some(draft), Some(dataset)) = (&builder.draft, builder.dataset(&datasets)) {
                        app.apply(Act::Share(
                            builder.title(),
                            draft.spec.clone(),
                            Box::new(dataset.clone()),
                        ));
                    }
                }
            }
        }
        board_command => {
            let Some(id) = app.route.get(1).cloned() else { return };
            let Some(board) = app.backend.boards.seen_get(&id).cloned() else {
                return;
            };
            let presenting = app.route.get(2).map(String::as_str) == Some("present");
            match board_command {
                "board-present" => app.go(&if presenting {
                    format!("boards/{id}")
                } else {
                    format!("boards/{id}/present")
                }),
                "board-arrange" => {
                    if let Some(page) = &mut app.views.board {
                        if page.arranging() {
                            page.done(&mut app.backend, &board);
                        } else {
                            page.edit(&board);
                        }
                    }
                }
                "board-cancel" => {
                    if let Some(page) = &mut app.views.board {
                        page.cancel();
                    }
                }
                "board-add" => app.apply(Act::AddChart(id)),
                "board-clear" => {
                    if let Some(page) = &mut app.views.board {
                        page.set_filters(&mut app.backend, Vec::new());
                    }
                }
                _ => {}
            }
        }
    }
}
