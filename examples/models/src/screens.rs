//! The pages, as the web's `pages/*.tsx` draw them over each page's own words and logic in
//! `pages`: the library and a model, the disk, discover and the downloads, and the runner. The chat
//! and the comparison are in `talk`. What a page asks of the app comes back as acts, done once the
//! frame is drawn.

use std::cell::RefCell;

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AsideWidth, Button, ButtonSize, ButtonVariant, Cell, Check, CheckList, CheckStatus, ChipTone, Column, Columns,
    DataTable, DescriptionList, DetailSheet, DownloadAsked, DownloadButton, DownloadButtonWords, DownloadState, Gap,
    Icon, IconButton, ItemCard, ListViewState, LogLevel, LogLine, LogView, Markdown, MarkdownScale, MenuItem, NotFound,
    NumberField, Page, PageHeader, PageState, Progress, SavedView, SavedViews, SavedViewsAsked, Section, Segmented,
    Split, Stack, StackPlace, Stat, TableVariant, TableWords, TagChips, TextField, TextRole, tokens,
};
use sokkel_egui_charts::data::{Cell as Value, Row};
use sokkel_egui_charts::{ChartKind, ChartPanel, ChartSpec, Stack as ChartStack};

use crate::data::catalog::{CatalogEntry, CatalogQuery, entry_of, model_id, search_catalog, search_delay};
use crate::data::format::{GB, bytes};
use crate::data::model::{Format, LogSource, Model, SOURCE_IDS, Sampling, SamplingPatch, Source};
use crate::pages::{discover, disk, downloads, library, model_page, pull, runner};
use crate::workbench::{AfterDelete, ModelsApp, Views};

/// Something a page asks of the app, done once the frame is drawn.
pub struct Act(Box<dyn FnOnce(&mut ModelsApp)>);

impl Act {
    pub fn new(does: impl FnOnce(&mut ModelsApp) + 'static) -> Self {
        Act(Box::new(does))
    }

    pub fn go(path: impl Into<String>) -> Self {
        let path = path.into();
        Act::new(move |app| app.go(&path))
    }
}

/// Acts asked from inside a table's cells, which are drawn while the table holds the page's acts.
#[derive(Default)]
pub struct Later(RefCell<Vec<Act>>);

impl Later {
    pub fn push(&self, act: Act) {
        self.0.borrow_mut().push(act);
    }

    pub fn take(self) -> Vec<Act> {
        self.0.into_inner()
    }
}

impl ModelsApp {
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

/// A column's width as the web's grid gives it, less the 12 its cell pads, which the table here
/// keeps as the gap between columns.
pub fn cell(web: f32) -> f32 {
    web - 12.0
}

/// A state's chip tone, as the web's `STATUSES` color the app's states beside the run states.
pub fn tone_of(status: &str) -> ChipTone {
    match status {
        "ready" | "up" => ChipTone::Good,
        "down" => ChipTone::Critical,
        "paused" | "stopping" => ChipTone::Warning,
        "done" => ChipTone::Status("succeeded"),
        "loading" | "starting" => ChipTone::Status("running"),
        "succeeded" | "failed" | "running" | "queued" | "pending" | "waiting" | "cancelled" | "skipped" => {
            ChipTone::Status(match status {
                "succeeded" => "succeeded",
                "failed" => "failed",
                "running" => "running",
                "queued" => "queued",
                "pending" => "pending",
                "waiting" => "waiting",
                "cancelled" => "cancelled",
                _ => "skipped",
            })
        }
        _ => ChipTone::Neutral,
    }
}

/// A chip the pages write as `status|label`: its words (the status's own when there are none) and
/// tone, or the muted words alone.
pub fn chip_cell(said: &str) -> Cell {
    match said.split_once('|') {
        Some((status, label)) => {
            let label = if label.is_empty() { status } else { label };
            Cell::Chip(label.to_owned(), tone_of(status))
        }
        None => Cell::Muted(said.to_owned()),
    }
}

/// A chip as a widget, written as `chip_cell` reads it.
pub fn chip(ui: &mut Ui, said: &str) {
    let t = tokens(ui.ctx());
    match said.split_once('|') {
        Some((status, label)) => {
            let label = if label.is_empty() { status } else { label };
            ui.add(sokkel_egui::Chip::new(label).tone(tone_of(status)));
        }
        None => {
            ui.label(TextRole::Body.rich(&t, said).color(t.color("muted-foreground")));
        }
    }
}

/// A figure's cell, written as `text` and sorted by `value`.
fn number(value: f64, text: String) -> Cell {
    Cell::Number { value, text }
}

pub fn words(row: &str, rows: &str, empty: &str) -> TableWords {
    TableWords {
        row: format!("{{count}} {row}"),
        rows: format!("{{count}} {rows}"),
        empty: empty.to_owned(),
        ..TableWords::default()
    }
}

/// The page the address names.
pub fn page(ui: &mut Ui, app: &ModelsApp, page: &str, second: Option<&str>, views: &mut Views, acts: &mut Vec<Act>) {
    match (page, second) {
        ("library", None) => library_page(ui, app, views, acts),
        ("library", Some(id)) => model_page(ui, app, id, views, acts),
        ("disk", _) => disk_page(ui, app, views, acts),
        ("discover", _) => discover_page(ui, app, views, acts),
        ("downloads", _) => downloads_page(ui, app, views, acts),
        ("runner", _) => runner_page(ui, app, views, acts),
        ("chat", _) => crate::talk::chat_page(ui, app, second, views, acts),
        ("compare", _) => crate::talk::compare_page(ui, app, second, views, acts),
        _ => {
            Page::show(ui, |ui| {
                let route = app.route.clone();
                let gone = NotFound::new(route).show(ui);
                if gone {
                    acts.push(Act::go("library"));
                }
            });
        }
    }
}

// --- Library ----------------------------------------------------------------------------------------

/// The library's columns, as the web's grid sizes them.
fn library_columns() -> Vec<Column> {
    vec![
        Column::new("Name", cell(192.0)).fill_share(2.0),
        Column::new("Rounding", cell(104.0)),
        Column::new("Format", cell(112.0)),
        Column::new("Params", cell(80.0)).numeric(),
        Column::new("Size", cell(96.0)).numeric(),
        Column::new("Context", cell(80.0)).numeric(),
        Column::new("Source", cell(128.0)),
        Column::new("Added", cell(112.0)).stack(StackPlace::Secondary),
        Column::new("Runner", cell(120.0)),
    ]
}

/// A model's row in the library.
fn library_row(model: &Model, loaded: &[crate::data::model::Loaded]) -> Vec<Cell> {
    let said = library::cells(model, loaded);
    vec![
        Cell::Mono(said[0].clone()),
        Cell::Mono(said[1].clone()),
        Cell::Badge(said[2].clone()),
        number(model.params, said[3].clone()),
        number(model.size, said[4].clone()),
        number(model.context, said[5].clone()),
        Cell::Text(said[6].clone()),
        number(model.added as f64, said[7].clone()),
        chip_cell(&said[8]),
    ]
}

/// The listing's setup now, by the columns' ids, as the saved views keep it.
fn library_setup(views: &Views) -> ListViewState {
    ListViewState {
        filters: views.filters.clone(),
        sort: views
            .table
            .sort
            .map(|(column, rises)| (library::COLUMN_IDS[column].to_owned(), rises)),
        hidden: views
            .table
            .hidden
            .iter()
            .map(|at| library::COLUMN_IDS[*at].to_owned())
            .collect(),
        ..ListViewState::default()
    }
}

/// Sets the listing to a saved view's setup.
fn apply_view(views: &mut Views, state: &ListViewState) {
    views.filters = state.filters.clone();
    views.table.sort = state.sort.as_ref().and_then(|(column, rises)| {
        library::COLUMN_IDS
            .iter()
            .position(|one| one == column)
            .map(|at| (at, *rises))
    });
    views.table.hidden = state
        .hidden
        .iter()
        .filter_map(|id| library::COLUMN_IDS.iter().position(|one| one == id))
        .collect();
}

fn library_page(ui: &mut Ui, app: &ModelsApp, views: &mut Views, acts: &mut Vec<Act>) {
    let models = &app.backend.models.seen;
    let loaded = &app.backend.loaded.seen;
    let saved: Vec<SavedView> = app
        .backend
        .views
        .seen
        .iter()
        .map(|one| SavedView {
            id: one.id.clone(),
            name: one.name.clone(),
            state: one.state.clone(),
            locked: one.locked == Some(true),
        })
        .collect();
    let current = library_setup(views);
    Page::show(ui, |ui| {
        let mut asked = None;
        PageHeader::new("Models").count(library::count(models)).show(ui, |ui| {
            if ui
                .add(small("Pull", Some(Icon::ArrowDownToLine), ButtonVariant::Default))
                .clicked()
            {
                acts.push(Act::go("discover"));
            }
            asked = SavedViews::new("library-views", &saved, views.view.as_deref(), &current).show(ui);
        });
        match asked {
            Some(SavedViewsAsked::Picked(id)) => {
                if let Some(view) = saved.iter().find(|one| one.id == id) {
                    apply_view(views, &view.state);
                }
                views.view = Some(id);
            }
            Some(SavedViewsAsked::Save) => {
                if let Some(id) = views.view.clone() {
                    let state = current.clone();
                    acts.push(Act::new(move |app| {
                        let promise = app.backend.update(|b| &mut b.views, &id, move |row| row.state = state);
                        app.failed(promise, "Could not save the view".to_owned());
                    }));
                }
            }
            Some(SavedViewsAsked::SaveNew(name)) => {
                let state = current.clone();
                acts.push(Act::new(move |app| {
                    let promise = app.backend.create(
                        |b| &mut b.views,
                        move |id| crate::data::backend::LibraryView {
                            id,
                            name,
                            state,
                            locked: None,
                        },
                        None,
                    );
                    app.then(promise, |app, outcome| match outcome {
                        Ok(made) => {
                            if let Some(made) = made.downcast_ref::<crate::data::backend::LibraryView>() {
                                app.views.view = Some(made.id.clone());
                            }
                        }
                        Err(why) => app.notes.push(
                            sokkel_egui::Toast::error("Could not save the view")
                                .description(why)
                                .timeout(Some(4.0)),
                        ),
                    });
                }));
            }
            Some(SavedViewsAsked::Rename(name)) => {
                if let Some(id) = views.view.clone() {
                    acts.push(Act::new(move |app| {
                        let promise = app.backend.update(|b| &mut b.views, &id, move |row| row.name = name);
                        app.failed(promise, "Could not rename the view".to_owned());
                    }));
                }
            }
            Some(SavedViewsAsked::Delete) => {
                if let Some(id) = views.view.clone() {
                    acts.push(Act::new(move |app| {
                        let promise = app.backend.remove(|b| &mut b.views, &id);
                        app.then(promise, |app, outcome| match outcome {
                            Ok(_) => {
                                app.views.view = Some("all".to_owned());
                                apply_view(&mut app.views, &ListViewState::default());
                            }
                            Err(why) => app.notes.push(
                                sokkel_egui::Toast::error("Could not delete the view")
                                    .description(why)
                                    .timeout(Some(4.0)),
                            ),
                        });
                    }));
                }
            }
            None => {}
        }

        let filters_now = views.filters.clone();
        let shown = library::shown(models, &filters_now, &views.query);
        let fields = library::fields_of(models);
        let columns = library_columns();
        let rows: Vec<Vec<Cell>> = shown.iter().map(|model| library_row(model, loaded)).collect();
        let keys: Vec<&str> = shown.iter().map(|model| model.id.as_str()).collect();
        let mut filters = filters_now;
        let mut bulk = None;
        let Views { table, query, .. } = views;
        let out = DataTable::new("library", &columns, &rows, table)
            .keys(keys)
            .search(query)
            .search_placeholder("Search")
            .toolbar(|ui| {
                sokkel_egui::FilterBar::new("library-filters", &fields, &mut filters).show(ui);
            })
            .selectable(true)
            .bulk(|ui, chosen| {
                let said = format!("Delete {}", chosen.len());
                if ui
                    .add(small(&said, Some(Icon::Trash2), ButtonVariant::Outline))
                    .clicked()
                {
                    bulk = Some(chosen.clone());
                }
            })
            .actions(|row| {
                let held = loaded.iter().any(|one| one.id == shown[row].id);
                vec![
                    MenuItem::action("Chat with it").icon(Icon::MessageSquarePlus),
                    if held {
                        MenuItem::action("Unload").icon(Icon::PowerOff)
                    } else {
                        MenuItem::action("Load in the runner").icon(Icon::Cpu)
                    },
                    MenuItem::Separator,
                    MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
                ]
            })
            .cards(256.0, 176.0, |ui, index, width| {
                let model = shown[index];
                let mut tags = vec![model.format.key().to_owned()];
                for one in model
                    .capabilities
                    .iter()
                    .map(|one| one.key().to_owned())
                    .chain(model.tags.clone())
                {
                    if !tags.contains(&one) {
                        tags.push(one);
                    }
                }
                let held = library::memory_chip(loaded.iter().find(|one| one.id == model.id));
                ItemCard::new(&model.name)
                    .code(format!("{} · {}", model.publisher, model.quant))
                    .description(tags.join(" · "))
                    .status(move |ui| chip(ui, &held))
                    .fact("Size", bytes(model.size))
                    .fact("Context", crate::data::format::context(model.context))
                    .show(ui, width, 0.0)
            })
            .stretch(true)
            .footer(false)
            .words(words("model", "models", "Nothing to show"))
            .show(ui);
        views.filters = filters;
        if let Some(index) = views.table.opened.take()
            && let Some(model) = shown.get(index)
        {
            acts.push(Act::go(format!("library/{}", model.id)));
        }
        if let Some(chosen) = bulk {
            let gone: Vec<Model> = chosen
                .iter()
                .filter_map(|at| shown.get(*at))
                .map(|one| (*one).clone())
                .collect();
            acts.push(Act::new(move |app| app.ask_delete(gone, AfterDelete::ClearSelection)));
        }
        if let Some((row, action)) = out.action
            && let Some(model) = shown.get(row).map(|one| (*one).clone())
        {
            let held = loaded.iter().any(|one| one.id == model.id);
            acts.push(Act::new(move |app| match action {
                0 => app.chat_with(&model),
                1 if held => app.unload(&model.id),
                1 => app.load(&model),
                _ => app.ask_delete(vec![model], AfterDelete::Stay),
            }));
        }
    });
}

// --- A model ----------------------------------------------------------------------------------------

fn model_page(ui: &mut Ui, app: &ModelsApp, id: &str, views: &mut Views, acts: &mut Vec<Act>) {
    let prefs = app.prefs();
    let Some(model) = app.backend.models.seen.iter().find(|one| one.id == id).cloned() else {
        Page::show(ui, |ui| {
            PageState::empty(format!("No model {id} is in the library. It may have been deleted.")).show(ui);
        });
        return;
    };
    let said = model_page::model_page(
        &model,
        &app.backend.loaded.seen,
        &app.backend.conversations.seen,
        &prefs.root,
    );
    let Views {
        second,
        system,
        max_tokens,
        ..
    } = views;
    Page::scroll(ui, |ui| {
        PageHeader::new(&said.title)
            .code(&said.code)
            .description(&said.description)
            .show(ui, |ui| {
                if ui
                    .add(small("Chat", Some(Icon::MessageSquarePlus), ButtonVariant::Default))
                    .clicked()
                {
                    let model = model.clone();
                    acts.push(Act::new(move |app| app.chat_with(&model)));
                }
                if said.load == "load" {
                    if ui.add(small("Load", Some(Icon::Cpu), ButtonVariant::Outline)).clicked() {
                        let model = model.clone();
                        acts.push(Act::new(move |app| app.load(&model)));
                    }
                } else if ui
                    .add(small("Unload", Some(Icon::PowerOff), ButtonVariant::Outline))
                    .clicked()
                {
                    let id = model.id.clone();
                    acts.push(Act::new(move |app| app.unload(&id)));
                }
                if ui
                    .add(small("Delete", Some(Icon::Trash2), ButtonVariant::Outline))
                    .clicked()
                {
                    let model = model.clone();
                    acts.push(Act::new(move |app| app.ask_delete(vec![model], AfterDelete::Library)));
                }
            });
        Split::new(AsideWidth::Lg).show(
            ui,
            |ui| {
                Stack::new(Gap::Page).show(ui, |ui| {
                    Section::new("Details").show(ui, |ui| {
                        Stack::new(Gap::Lg).show(ui, |ui| {
                            let mut tags: Vec<String> = Vec::new();
                            for one in model
                                .capabilities
                                .iter()
                                .map(|one| one.key().to_owned())
                                .chain(model.tags.clone())
                            {
                                if !tags.contains(&one) {
                                    tags.push(one);
                                }
                            }
                            ui.add(TagChips::new(&tags));
                            let mut list = DescriptionList::new().side(128.0);
                            for (term, value) in &said.details {
                                if term == "Path" {
                                    let path = value.clone();
                                    list = list.item(term.clone(), move |ui| {
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 2.0;
                                            let t = tokens(ui.ctx());
                                            let mono = egui::FontId::monospace(TextRole::Help.font(&t).size);
                                            ui.label(
                                                egui::RichText::new(&path).font(mono).color(t.color("foreground")),
                                            );
                                            if ui.add(IconButton::new(Icon::Copy, "Copy the path").xs()).clicked() {
                                                ui.ctx().copy_text(path.clone());
                                            }
                                        });
                                    });
                                } else {
                                    list = list.text(term.clone(), value.clone());
                                }
                            }
                            list.show(ui);
                        });
                    });
                    Section::new("Files").meta(&said.files_meta).flush(true).show(ui, |ui| {
                        let columns = [
                            Column::new("File", cell(192.0)).fill(),
                            Column::new("Role", cell(112.0)),
                            Column::new("Size", cell(104.0)).numeric(),
                        ];
                        let mut files: Vec<_> = model.files.iter().collect();
                        files.sort_by(|a, b| b.size.partial_cmp(&a.size).unwrap_or(std::cmp::Ordering::Equal));
                        let rows: Vec<Vec<Cell>> = said
                            .files
                            .iter()
                            .zip(&files)
                            .map(|((name, role, size), file)| {
                                vec![Cell::Mono(name.clone()), chip_cell(role), number(file.size, size.clone())]
                            })
                            .collect();
                        if second.sort.is_none() {
                            second.sort = Some((2, false));
                        }
                        DataTable::new("model-files", &columns, &rows, second)
                            .variant(TableVariant::List)
                            .framed(false)
                            .footer(false)
                            .fits(true)
                            .height(f32::INFINITY)
                            .show(ui);
                    });
                    Section::new("Card").show(ui, |ui| {
                        Markdown::new(&model.card).scale(MarkdownScale::Document).show(ui);
                    });
                });
            },
            |ui| {
                Stack::new(Gap::Page).show(ui, |ui| {
                    Section::new("Defaults for new chats")
                        .description(
                            "Each new conversation with it starts from these; a conversation's own settings change only that one.",
                        )
                        .show(ui, |ui| defaults(ui, &model, &said.max_hint, system, max_tokens, acts));
                    Section::new("In use").show(ui, |ui| {
                        let mut list = DescriptionList::new().side(128.0);
                        for (term, value) in &said.in_use {
                            if term == "Runner" {
                                let value = value.clone();
                                list = list.item(term.clone(), move |ui| chip(ui, &value));
                            } else {
                                list = list.text(term.clone(), value.clone());
                            }
                        }
                        list.show(ui);
                    });
                });
            },
        );
    });
}

/// The defaults a new conversation with the model starts with, kept as each is changed.
fn defaults(
    ui: &mut Ui,
    model: &Model,
    max_hint: &str,
    typed: &mut Option<String>,
    max_typed: &mut Option<f64>,
    acts: &mut Vec<Act>,
) {
    let save = |patch: SamplingPatch, acts: &mut Vec<Act>| {
        let id = model.id.clone();
        acts.push(Act::new(move |app| {
            let promise = app.backend.update(
                |b| &mut b.models,
                &id,
                move |row| row.defaults = patch.apply(&row.defaults),
            );
            app.failed(promise, "Could not save".to_owned());
        }));
    };
    let fixed = |value: f64| crate::js::to_fixed(value, 2);
    Stack::new(Gap::Lg).show(ui, |ui| {
        let mut temperature = model.defaults.temperature;
        ui.add(
            sokkel_egui::SliderField::new("Temperature", &mut temperature, 2.0)
                .step(0.05)
                .format(fixed)
                .help("Higher is more varied, lower more literal. 0 always answers alike."),
        );
        if temperature != model.defaults.temperature {
            save(
                SamplingPatch {
                    temperature: Some(temperature),
                    ..SamplingPatch::default()
                },
                acts,
            );
        }
        let mut top_p = model.defaults.top_p;
        ui.add(
            sokkel_egui::SliderField::new("Top p", &mut top_p, 1.0)
                .step(0.05)
                .format(fixed)
                .help("Only the likeliest tokens up to this share are drawn from."),
        );
        if top_p != model.defaults.top_p {
            save(
                SamplingPatch {
                    top_p: Some(top_p),
                    ..SamplingPatch::default()
                },
                acts,
            );
        }
        let mut max = max_typed.unwrap_or(model.defaults.max_tokens);
        ui.add(
            NumberField::new(("default-max-tokens", &model.id), "Max tokens", &mut max)
                .integer(true)
                .step(64.0)
                .hint(Some(max_hint)),
        );
        if max != max_typed.unwrap_or(model.defaults.max_tokens) {
            *max_typed = Some(max);
            if model_page::max_tokens_kept(Some(max), model) {
                save(
                    SamplingPatch {
                        max_tokens: Some(max),
                        ..SamplingPatch::default()
                    },
                    acts,
                );
            }
        }
        let mut system = typed.clone().unwrap_or_else(|| model.defaults.system.clone());
        let field = ui.add(
            TextField::new("System prompt", &mut system)
                .multiline(true)
                .help("Said to the model before every conversation with it starts."),
        );
        if field.lost_focus() && system != model.defaults.system {
            save(
                SamplingPatch {
                    system: Some(system.clone()),
                    ..SamplingPatch::default()
                },
                acts,
            );
        }
        *typed = Some(system);
        let reset = ui.add(small(
            "Reset to the app's defaults",
            Some(Icon::RotateCcw),
            ButtonVariant::Outline,
        ));
        if reset.clicked() {
            let back: Sampling = crate::data::model::default_sampling();
            *typed = Some(back.system.clone());
            *max_typed = None;
            save(model_page::reset_defaults(), acts);
        }
    });
}

// --- Disk -------------------------------------------------------------------------------------------

/// A stat's tile in a row of them, `count` to a row and 24 apart, as the web's `Columns`.
fn tiles(ui: &mut Ui, stats: Vec<Stat>, count: usize) {
    Columns::new(count)
        .min(160.0)
        .gap(24.0)
        .show(ui, stats.len(), |ui, index, width| {
            stats[index].clone().tile(ui, width);
        });
}

fn stat_of(one: &disk::Stat) -> Stat {
    let mut stat = Stat::new(&one.label, &one.value);
    if let Some(note) = &one.note {
        stat = stat.note(note);
    }
    stat
}

/// Gigabytes written back as the disk is sold.
fn write_gb(value: f64) -> String {
    bytes(value * GB)
}

fn disk_page(ui: &mut Ui, app: &ModelsApp, views: &mut Views, acts: &mut Vec<Act>) {
    let prefs = app.prefs();
    let models = &app.backend.models.seen;
    let said = disk::disk_page(&prefs.root, models, &app.backend.downloads.seen);
    Page::scroll(ui, |ui| {
        PageHeader::new("Disk usage")
            .description(&said.description)
            .show(ui, |ui| {
                if ui
                    .add(small("Change the root", Some(Icon::Settings2), ButtonVariant::Outline))
                    .clicked()
                {
                    acts.push(Act::go("settings"));
                }
            });
        Stack::new(Gap::Page).show(ui, |ui| {
            tiles(ui, said.stats.iter().map(stat_of).collect(), 4);
            let whole: Vec<Row> = said
                .whole
                .iter()
                .map(|one| {
                    Row::of([
                        ("disk", Value::from(one.disk.as_str())),
                        ("part", Value::from(one.part)),
                        ("size", Value::Number(one.size)),
                    ])
                })
                .collect();
            let spec = ChartSpec::new(whole)
                .x("disk")
                .y("size")
                .series("part")
                .stack(ChartStack::Stacked)
                .horizontal(true)
                .series_order(["Models", "Downloads under way", "Other files", "Free"])
                .color("Models", "primary")
                .color("Downloads under way", "status-running")
                .color("Other files", "muted-foreground")
                .color("Free", "border")
                .format(write_gb);
            ChartPanel::new("The disk", ChartKind::Bar, &spec)
                .subtitle(&said.subtitle)
                .height(99.0)
                .show(ui);
            let rows: Vec<Row> = said
                .rows
                .iter()
                .map(|one| {
                    Row::of([
                        ("source", Value::from(one.source)),
                        ("format", Value::from(one.format)),
                        ("name", Value::from(one.name.as_str())),
                        ("size", Value::Number(one.size)),
                    ])
                })
                .collect();
            let by_source = ChartSpec::new(rows.clone())
                .x("source")
                .y("size")
                .format(write_gb)
                .total("on disk");
            let by_format = ChartSpec::new(rows.clone())
                .x("format")
                .y("size")
                .order(["packed", "unified", "sharded"])
                .format(write_gb);
            Columns::new(2).min(400.0).gap(24.0).show(ui, 2, |ui, index, _| {
                if index == 0 {
                    ChartPanel::new("By source", ChartKind::Donut, &by_source)
                        .subtitle("where the models came from")
                        .height(243.0)
                        .show(ui);
                } else {
                    ChartPanel::new("By format", ChartKind::Bar, &by_format)
                        .subtitle("how the weights are written")
                        .legend(false)
                        .height(277.0)
                        .show(ui);
                }
            });
            sokkel_egui_charts::TreemapPanel::new("Every model", &rows, &["source", "format", "name"], "size")
                .subtitle("by source, then format, sized by what it takes")
                .format(write_gb)
                .height(307.0)
                .show(ui);
            let mut section = Section::new("Kept more than once")
                .description(
                    "Models on the disk in more than one rounding or format. Keeping only the smallest copy frees the rest.",
                )
                .flush(true);
            if let Some(meta) = &said.copies_meta {
                section = section.meta(meta);
            }
            section.show(ui, |ui| {
                let columns = [
                    Column::new("Model", cell(160.0)).fill(),
                    Column::new("Copies", cell(192.0)).fill_share(1.4),
                    Column::new("Frees", cell(112.0)).numeric(),
                ];
                let rows: Vec<Vec<Cell>> = said
                    .copies
                    .iter()
                    .map(|one| {
                        vec![
                            Cell::Mono(one.name.clone()),
                            Cell::Text(one.said.clone()),
                            Cell::Text(one.reclaim.clone()),
                        ]
                    })
                    .collect();
                let out = DataTable::new("disk-copies", &columns, &rows, &mut views.table)
                    .variant(TableVariant::List)
                    .framed(false)
                    .footer(false)
                    .fits(true)
                    .height(f32::INFINITY)
                    .actions(|_| {
                        vec![
                            MenuItem::action("Delete all but the smallest")
                                .icon(Icon::Trash2)
                                .destructive(),
                        ]
                    })
                    .words(words("model", "models", "Every model is kept once."))
                    .show(ui);
                if let Some(index) = views.table.opened.take()
                    && let Some(one) = said.copies.get(index)
                {
                    acts.push(Act::go(format!("library/{}", one.kept[0])));
                }
                if let Some((index, _)) = out.action
                    && let Some(one) = said.copies.get(index)
                {
                    let gone: Vec<Model> = one.kept[1..]
                        .iter()
                        .filter_map(|id| models.iter().find(|model| model.id == *id).cloned())
                        .collect();
                    acts.push(Act::new(move |app| app.ask_delete(gone, AfterDelete::Stay)));
                }
            });
            let t = tokens(ui.ctx());
            let path = crate::data::disk::path_of(&prefs.root, Source::Hub, "<publisher>", "<name>", "<rounding>");
            let mut job = egui::text::LayoutJob::default();
            for (words, mono) in [("Models are kept under ", false), (path.as_str(), true), (".", false)] {
                let mut rich = TextRole::Help.rich(&t, words).color(t.color("muted-foreground"));
                if mono {
                    rich = rich.font(egui::FontId::monospace(TextRole::Help.font(&t).size));
                }
                rich.append_to(&mut job, ui.style(), egui::FontSelection::Default, egui::Align::Center);
            }
            ui.label(job);
        });
    });
}

// --- Discover ---------------------------------------------------------------------------------------

/// A variant's pull button, through every state its pull passes; what it asks comes back as acts.
fn pull_button(ui: &mut Ui, app: &ModelsApp, entry: &CatalogEntry, at: usize, compact: bool, later: &Later) {
    let variant = &entry.variants[at];
    let said = pull::pull_button(entry, variant, &app.backend.models.seen, &app.backend.downloads.seen);
    let id = ("pull", entry.id.as_str(), variant.quant.as_str(), variant.format.key());
    let (button, words) = match &said {
        pull::PullButton::Kept { detail, said, .. } => (
            DownloadButton::new(id, DownloadState::Kept).kept_detail(detail).opens(),
            DownloadButtonWords {
                kept: said.clone(),
                ..DownloadButtonWords::default()
            },
        ),
        pull::PullButton::Idle { said, error, .. } => {
            let mut button = DownloadButton::new(id, DownloadState::Idle);
            if let Some(error) = error {
                button = button.error(error);
            }
            (
                button,
                DownloadButtonWords {
                    download: said.clone(),
                    ..DownloadButtonWords::default()
                },
            )
        }
        pull::PullButton::Running {
            value, paused, queued, ..
        } => (
            DownloadButton::new(
                id,
                if *paused {
                    DownloadState::Paused
                } else {
                    DownloadState::Running
                },
            )
            .value(*value as f32)
            .pausable()
            .resumable()
            .cancellable(),
            DownloadButtonWords {
                share: if *queued {
                    "Queued".to_owned()
                } else {
                    "{percent}".to_owned()
                },
                ..DownloadButtonWords::default()
            },
        ),
    };
    let button = button.size(ButtonSize::Sm).words(words);
    let button = if compact { button.compact_on_phone() } else { button };
    let out = button.show(ui);
    let Some(asked) = out.asked else { return };
    let entry_id = entry.id.clone();
    let name = entry.name.clone();
    match (&said, asked) {
        (pull::PullButton::Kept { open, .. }, DownloadAsked::Open) => later.push(Act::go(open.clone())),
        (pull::PullButton::Idle { retry: Some(retry), .. }, DownloadAsked::Download) => {
            let retry = retry.clone();
            later.push(Act::new(move |app| {
                let promise = app.backend.retry_pull(&retry);
                app.failed(promise, format!("Could not pull {name}"));
            }));
        }
        (pull::PullButton::Idle { .. }, DownloadAsked::Download) => later.push(Act::new(move |app| {
            if let Some(entry) = entry_of(&entry_id) {
                let promise = app.backend.pull(entry, &entry.variants[at]);
                app.failed(promise, format!("Could not pull {name}"));
            }
        })),
        (pull::PullButton::Running { id, .. }, asked) => {
            let id = id.clone();
            later.push(Act::new(move |app| {
                match asked {
                    DownloadAsked::Pause => app.backend.pause_pull(&id),
                    DownloadAsked::Resume => app.backend.resume_pull(&id),
                    _ => app.backend.cancel_pull(&id),
                };
            }));
        }
        _ => {}
    }
}

/// The source and the format a search asks for, by the choices' ids.
fn query_of(app: &ModelsApp) -> CatalogQuery {
    CatalogQuery {
        text: app.kept.discover_query.clone(),
        source: Source::from_key(&app.kept.discover_source),
        format: Format::from_key(&app.kept.discover_format),
    }
}

/// Discover's search as the web runs it: 220 after the last change, then the sources' answer.
pub fn discover_search(app: &mut ModelsApp) {
    let asked = (
        app.kept.discover_query.clone(),
        app.kept.discover_source.clone(),
        app.kept.discover_format.clone(),
    );
    let now = app.now();
    let views = &mut app.views;
    if views.searched.as_ref() != Some(&asked) {
        views.searched = Some(asked);
        views.results = None;
        let query = query_of(app);
        app.views.search_due = Some(now + 220 + search_delay(&query));
    }
    if app.views.search_due.is_some_and(|due| now >= due) {
        app.views.search_due = None;
        let found = search_catalog(&query_of(app));
        app.views.results = Some(found.iter().map(|one| one.id.clone()).collect());
    }
}

fn discover_page(ui: &mut Ui, app: &ModelsApp, views: &mut Views, acts: &mut Vec<Act>) {
    let models = &app.backend.models.seen;
    let (count, description, button) = discover::header(&app.backend.downloads.seen);
    acts.push(Act::new(discover_search));
    let results: Vec<&CatalogEntry> = views.results.iter().flatten().filter_map(|id| entry_of(id)).collect();
    let later = Later::default();
    Page::show(ui, |ui| {
        PageHeader::new("Discover")
            .count(count)
            .description(description)
            .show(ui, |ui| {
                if ui
                    .add(small(&button, Some(Icon::ArrowDownToLine), ButtonVariant::Outline))
                    .clicked()
                {
                    acts.push(Act::go("downloads"));
                }
            });
        let columns = [
            Column::new("Model", cell(208.0)).fill_share(2.0),
            Column::new("Publisher", cell(112.0)),
            Column::new("Source", cell(136.0)),
            Column::new("Params", cell(88.0)).numeric(),
            Column::new("Pulls", cell(112.0)).numeric().stack(StackPlace::Secondary),
            Column::new("Library", cell(128.0)),
            Column::new("Pull", cell(192.0)).sortable(false),
        ];
        let rows: Vec<Vec<Cell>> = results
            .iter()
            .map(|entry| {
                let said = discover::row(entry, models);
                vec![
                    Cell::Mono(said[0].clone()),
                    Cell::Text(said[1].clone()),
                    Cell::Text(said[2].clone()),
                    number(entry.params, said[3].clone()),
                    number(entry.downloads, said[4].clone()),
                    chip_cell(&said[5]),
                    Cell::Slot { words: String::new() },
                ]
            })
            .collect();
        let mut query = app.kept.discover_query.clone();
        let sources = ["all", "hub", "runner", "desktop"];
        let formats = ["all", "packed", "unified", "sharded"];
        let mut source = sources
            .iter()
            .position(|one| *one == app.kept.discover_source)
            .unwrap_or(0);
        let mut format = formats
            .iter()
            .position(|one| *one == app.kept.discover_format)
            .unwrap_or(0);
        let source_was = source;
        let format_was = format;
        let mut source_words = vec!["All".to_owned()];
        source_words.extend(SOURCE_IDS.iter().map(|one| one.label().to_owned()));
        let loading = views.results.is_none();
        DataTable::new("discover", &columns, &rows, &mut views.table)
            .keys(results.iter().map(|one| one.id.as_str()))
            .search(&mut query)
            .search_placeholder("Search")
            .toolbar(|ui| {
                ui.add(Segmented::new(&mut source, source_words));
                ui.add(Segmented::new(
                    &mut format,
                    ["Any format", "packed", "unified", "sharded"],
                ));
            })
            .slot(|ui, row, _| pull_button(ui, app, results[row], pull_variant(results[row]), true, &later))
            .stretch(true)
            .footer(false)
            .words(words(
                "result",
                "results",
                if loading {
                    ""
                } else {
                    "Nothing matches. Try fewer words, or another source."
                },
            ))
            .show(ui);
        if query != app.kept.discover_query || source != source_was || format != format_was {
            let (source, format) = (sources[source].to_owned(), formats[format].to_owned());
            acts.push(Act::new(move |app| {
                app.kept.discover_query = query;
                app.kept.discover_source = source;
                app.kept.discover_format = format;
            }));
        }
        if let Some(index) = views.table.opened.take()
            && let Some(entry) = results.get(index)
        {
            views.open = Some(entry.id.clone());
        }
    });
    if let Some(entry) = views.open.as_deref().and_then(entry_of) {
        let mut width = 576.0;
        let mut tags: Vec<String> = Vec::new();
        for one in entry
            .capabilities
            .iter()
            .map(|one| one.key().to_owned())
            .chain(entry.tags.clone())
        {
            if !tags.contains(&one) {
                tags.push(one);
            }
        }
        let memory = app.prefs().memory;
        let closed = DetailSheet::new("discover-sheet", &entry.name, &mut width)
            .code(format!("{} · {}", entry.publisher, entry.source.label()))
            .description(&entry.summary)
            .line(|ui| {
                ui.add(TagChips::new(&tags));
            })
            .show(ui.ctx(), |ui| {
                Stack::new(Gap::Page).show(ui, |ui| {
                    Section::new("Variants")
                        .meta(entry.variants.len().to_string())
                        .flush(true)
                        .show(ui, |ui| {
                            let columns = [
                                Column::new("Rounding", cell(96.0)).fill(),
                                Column::new("Format", cell(112.0)),
                                Column::new("Size", cell(96.0)).numeric(),
                                Column::new("Memory", cell(112.0)),
                                Column::new("Pull", cell(176.0)),
                            ];
                            let fits = discover::variant_memory(entry, memory);
                            let rows: Vec<Vec<Cell>> = entry
                                .variants
                                .iter()
                                .zip(&fits)
                                .map(|(variant, fits)| {
                                    vec![
                                        Cell::Mono(variant.quant.clone()),
                                        Cell::Badge(variant.format.key().to_owned()),
                                        number(variant.size, bytes(variant.size)),
                                        chip_cell(fits),
                                        Cell::Slot { words: String::new() },
                                    ]
                                })
                                .collect();
                            DataTable::new(("variants", &entry.id), &columns, &rows, &mut views.second)
                                .variant(TableVariant::List)
                                .framed(false)
                                .footer(false)
                                .fits(true)
                                .height(f32::INFINITY)
                                .slot(|ui, row, _| pull_button(ui, app, entry, row, false, &later))
                                .show(ui);
                        });
                    Section::new("About").show(ui, |ui| {
                        let t = tokens(ui.ctx());
                        let about = discover::about(entry);
                        ui.label(TextRole::Body.rich(&t, &about[0]).color(t.color("foreground")));
                        ui.label(TextRole::Help.rich(&t, &about[1]).color(t.color("muted-foreground")));
                        ui.label(TextRole::Help.rich(&t, &about[2]).color(t.color("muted-foreground")));
                    });
                });
            });
        if closed {
            views.open = None;
        }
    }
    acts.extend(later.take());
}

/// The variant a row offers to pull at once, by its place in the entry's variants.
fn pull_variant(entry: &CatalogEntry) -> usize {
    let usual = pull::usual_variant(entry);
    entry
        .variants
        .iter()
        .position(|one| std::ptr::eq(one, usual))
        .unwrap_or(0)
}

// --- Downloads --------------------------------------------------------------------------------------

fn downloads_page(ui: &mut Ui, app: &ModelsApp, views: &mut Views, acts: &mut Vec<Act>) {
    let prefs = app.prefs();
    let pulls = &app.backend.downloads.seen;
    let said = downloads::downloads_page(pulls, &prefs);
    let later = Later::default();
    Page::show(ui, |ui| {
        PageHeader::new("Downloads")
            .count(&said.count)
            .description(&said.description)
            .show(ui, |ui| {
                if ui
                    .add(small("Discover", Some(Icon::Search), ButtonVariant::Default))
                    .clicked()
                {
                    acts.push(Act::go("discover"));
                }
                let clear = small("Clear ended", Some(Icon::Trash2), ButtonVariant::Outline);
                if ui.add_enabled(said.can_clear, clear).clicked() {
                    acts.push(Act::new(|app| {
                        app.backend.clear_ended();
                    }));
                }
            });
        Stack::new(Gap::Page).show(ui, |ui| {
            tiles(ui, said.stats.iter().map(stat_of).collect(), 3);
            let columns = [
                Column::new("Model", cell(192.0)).fill_share(1.6),
                Column::new("From", cell(128.0)),
                Column::new("State", cell(112.0)),
                Column::new("Progress", cell(176.0))
                    .fill_share(1.4)
                    .stack(StackPlace::Primary),
                Column::new("Speed", cell(112.0)).numeric().stack(StackPlace::Secondary),
                Column::new("Time left", cell(120.0))
                    .numeric()
                    .stack(StackPlace::Secondary),
                Column::new("", cell(192.0)).sortable(false),
            ];
            let rows: Vec<Vec<Cell>> = said
                .rows
                .iter()
                .map(|row| {
                    let pull = pulls.iter().find(|one| one.id == row.id);
                    vec![
                        Cell::Runs(vec![
                            sokkel_egui::CellRun::new(pull.map_or("", |one| one.name.as_str()), "foreground").mono(),
                            sokkel_egui::CellRun::new(" ", "foreground").mono(),
                            sokkel_egui::CellRun::new(pull.map_or("", |one| one.quant.as_str()), "muted-foreground")
                                .mono(),
                        ]),
                        Cell::Text(row.from.to_owned()),
                        chip_cell(&row.state),
                        Cell::Slot {
                            words: row.progress.clone(),
                        },
                        number(pull.map_or(0.0, |one| one.speed), row.speed.clone()),
                        Cell::Muted(row.left.clone()),
                        Cell::Slot { words: String::new() },
                    ]
                })
                .collect();
            let groups: Vec<&str> = said.rows.iter().map(|row| row.group).collect();
            let out = DataTable::new("downloads", &columns, &rows, &mut views.table)
                .keys(said.rows.iter().map(|row| row.id.as_str()))
                .groups(
                    groups
                        .iter()
                        .map(|group| if *group == "going" { "Under way" } else { "Ended" }),
                )
                .bar(true)
                .slot(|ui, index, column| {
                    let Some(row) = said.rows.get(index) else { return };
                    let Some(pull) = pulls.iter().find(|one| one.id == row.id) else {
                        return;
                    };
                    if column == 3 {
                        match row.value {
                            Some(value) => {
                                ui.add(Progress::new(value as f32).label(&row.progress).small(true));
                            }
                            None => {
                                let t = tokens(ui.ctx());
                                ui.label(TextRole::Body.rich(&t, &row.progress).color(t.color("critical-ink")));
                            }
                        }
                    } else {
                        pull_control(ui, pull, &later);
                    }
                })
                .actions(|index| {
                    said.rows[index]
                        .actions
                        .iter()
                        .map(|id| match *id {
                            "pause" => MenuItem::action("Pause").icon(Icon::Pause),
                            "resume" => MenuItem::action("Resume").icon(Icon::Play),
                            "cancel" => MenuItem::action("Cancel").icon(Icon::X).destructive(),
                            "retry" => MenuItem::action("Pull again").icon(Icon::RotateCcw),
                            "open" => MenuItem::action("Open in the library").icon(Icon::ExternalLink),
                            _ => MenuItem::action("Remove from the list").icon(Icon::Trash2),
                        })
                        .collect()
                })
                .auto_rows()
                .stretch(true)
                .footer(false)
                .words(words(
                    "pull",
                    "pulls",
                    "Nothing has been pulled yet. Find a model in Discover.",
                ))
                .show(ui);
            if let Some(index) = views.table.opened.take()
                && let Some(pull) = said
                    .rows
                    .get(index)
                    .and_then(|row| pulls.iter().find(|one| one.id == row.id))
                && pull.state == crate::data::model::DownloadState::Done
            {
                acts.push(Act::go(format!("library/{}", model_id(&pull.name, &pull.quant))));
            }
            if let Some((index, action)) = out.action
                && let Some(row) = said.rows.get(index)
                && let Some(pull) = pulls.iter().find(|one| one.id == row.id)
                && let Some(chosen) = row.actions.get(action)
            {
                let (id, chosen) = (pull.id.clone(), *chosen);
                let route = format!("library/{}", model_id(&pull.name, &pull.quant));
                acts.push(Act::new(move |app| {
                    match chosen {
                        "pause" => {
                            app.backend.pause_pull(&id);
                        }
                        "resume" => {
                            app.backend.resume_pull(&id);
                        }
                        "cancel" => {
                            app.backend.cancel_pull(&id);
                        }
                        "retry" => {
                            app.backend.retry_pull(&id);
                        }
                        "open" => app.go(&route),
                        _ => {
                            app.backend.remove_pull(&id);
                        }
                    };
                }));
            }
        });
    });
    acts.extend(later.take());
}

/// A pull's own control: its share with pause and cancel, paused with resume, or what to do once
/// it ended.
fn pull_control(ui: &mut Ui, pull: &crate::data::model::Download, later: &Later) {
    use crate::data::model::DownloadState as Pulled;
    let id = ("pull-control", pull.id.as_str());
    let button = match pull.state {
        Pulled::Done => DownloadButton::new(id, DownloadState::Kept)
            .opens()
            .words(DownloadButtonWords {
                kept: "Open".to_owned(),
                ..DownloadButtonWords::default()
            }),
        Pulled::Failed | Pulled::Cancelled => {
            let mut button = DownloadButton::new(id, DownloadState::Idle).words(DownloadButtonWords {
                download: "Pull again".to_owned(),
                ..DownloadButtonWords::default()
            });
            if let Some(error) = &pull.error {
                button = button.error(error);
            }
            button
        }
        state => DownloadButton::new(
            id,
            if state == Pulled::Paused {
                DownloadState::Paused
            } else {
                DownloadState::Running
            },
        )
        .value((pull.done / pull.size) as f32)
        .pausable()
        .resumable()
        .cancellable()
        .words(DownloadButtonWords {
            share: if state == Pulled::Queued {
                "Queued".to_owned()
            } else {
                "{percent}".to_owned()
            },
            ..DownloadButtonWords::default()
        }),
    };
    let out = button.size(ButtonSize::Sm).show(ui);
    let Some(asked) = out.asked else { return };
    let id = pull.id.clone();
    let route = format!("library/{}", model_id(&pull.name, &pull.quant));
    later.push(Act::new(move |app| {
        match asked {
            DownloadAsked::Open => app.go(&route),
            DownloadAsked::Download => {
                app.backend.retry_pull(&id);
            }
            DownloadAsked::Pause => {
                app.backend.pause_pull(&id);
            }
            DownloadAsked::Resume => {
                app.backend.resume_pull(&id);
            }
            _ => {
                app.backend.cancel_pull(&id);
            }
        };
    }));
}

// --- Runner -----------------------------------------------------------------------------------------

fn runner_page(ui: &mut Ui, app: &ModelsApp, views: &mut Views, acts: &mut Vec<Act>) {
    let prefs = app.prefs();
    let now = app.now();
    let runner_row = app.backend.runner.seen.first();
    let loaded = &app.backend.loaded.seen;
    let models = &app.backend.models.seen;
    let sources = [
        None,
        Some(LogSource::Runner),
        Some(LogSource::Loader),
        Some(LogSource::Chat),
        Some(LogSource::Pull),
    ];
    let said = runner::runner_page(
        runner_row,
        loaded,
        models,
        &prefs,
        &app.backend.events.seen,
        &app.backend.downloads.seen,
        now,
        sources[views.log_source.min(4)],
        &views.log_query,
    );
    let status = runner_row.map_or(crate::data::model::RunnerStatus::Down, |one| one.status);
    Page::scroll(ui, |ui| {
        PageHeader::new("Runner")
            .code(&said.code)
            .description("Loads models into memory and answers with them. Chats and comparisons load what they need.")
            .show(ui, |ui| {
                let (label, on) = &said.button;
                if label == "Stop" {
                    if ui
                        .add(small("Stop", Some(Icon::PowerOff), ButtonVariant::Outline))
                        .clicked()
                    {
                        acts.push(Act::new(|app| {
                            app.backend.stop_runner();
                        }));
                    }
                } else if ui
                    .add_enabled(*on, small(label, Some(Icon::Power), ButtonVariant::Default))
                    .clicked()
                {
                    acts.push(Act::new(|app| {
                        app.backend.start_runner();
                    }));
                }
            });
        Stack::new(Gap::Page).show(ui, |ui| {
            let stats: Vec<Stat> = said
                .stats
                .iter()
                .map(|one| match one.value.split_once('|') {
                    Some((state, _)) => {
                        let mut stat = Stat::new(&one.label, "").chip(state, tone_of(state));
                        if let Some(note) = &one.note {
                            stat = stat.note(note);
                        }
                        stat
                    }
                    None => stat_of(one),
                })
                .collect();
            tiles(ui, stats, 4);
            Section::new("Memory").meta(&said.memory_meta).show(ui, |ui| {
                ui.add(
                    Progress::new(said.memory_value as f32)
                        .label(&said.memory_label)
                        .quiet(true),
                );
            });
            let mut pick = views.pick.clone().unwrap_or_else(|| prefs.default_model.clone());
            let mut load = false;
            let can_load = status == crate::data::model::RunnerStatus::Up
                && models.iter().any(|one| one.id == pick)
                && !loaded.iter().any(|one| one.id == pick);
            Section::new("Loaded models")
                .meta(loaded.len().to_string())
                .flush(true)
                .actions(|ui| {
                    load = ui
                        .add_enabled(can_load, small("Load", Some(Icon::Cpu), ButtonVariant::Outline))
                        .clicked();
                    if let Some(next) = crate::talk::model_select(ui, "Model to load", models, &pick, false) {
                        pick = next;
                    }
                })
                .show(ui, |ui| {
                    let columns = [
                        Column::new("Model", cell(176.0)).fill_share(1.4),
                        Column::new("State", cell(136.0)),
                        Column::new("Memory", cell(104.0)).numeric(),
                        Column::new("Context", cell(96.0)).numeric(),
                        Column::new("Answers", cell(88.0))
                            .numeric()
                            .stack(StackPlace::Secondary),
                        Column::new("Last used", cell(120.0)),
                        Column::new("Unloads in", cell(112.0)).stack(StackPlace::Secondary),
                    ];
                    let rows: Vec<Vec<Cell>> = loaded
                        .iter()
                        .zip(&said.loaded)
                        .map(|(one, row)| {
                            vec![
                                Cell::Mono(row[0].clone()),
                                chip_cell(&row[1]),
                                number(one.memory, row[2].clone()),
                                number(one.context, row[3].clone()),
                                number(one.answers as f64, row[4].clone()),
                                Cell::Relative {
                                    at: one.used_at as f64,
                                    now: now as f64,
                                },
                                Cell::Muted(row[6].clone()),
                            ]
                        })
                        .collect();
                    let out = DataTable::new("runner-loaded", &columns, &rows, &mut views.second)
                        .keys(loaded.iter().map(|one| one.id.as_str()))
                        .variant(TableVariant::List)
                        .framed(false)
                        .footer(false)
                        .height(f32::INFINITY)
                        .actions(|_| {
                            vec![
                                MenuItem::action("Chat with it").icon(Icon::MessageSquarePlus),
                                MenuItem::action("Unload").icon(Icon::PowerOff),
                            ]
                        })
                        .words(words("model", "models", said.empty))
                        .show(ui);
                    if let Some((index, action)) = out.action
                        && let Some(one) = loaded.get(index)
                    {
                        let id = one.id.clone();
                        acts.push(Act::new(move |app| {
                            if action == 0 {
                                if let Some(model) = app.backend.models.seen.iter().find(|each| each.id == id).cloned()
                                {
                                    app.chat_with(&model);
                                }
                            } else {
                                app.unload(&id);
                            }
                        }));
                    }
                });
            if load && let Some(model) = models.iter().find(|one| one.id == pick).cloned() {
                acts.push(Act::new(move |app| app.load(&model)));
            }
            views.pick = Some(pick);
            Section::new("Health").show(ui, |ui| {
                let checks: Vec<Check> = said
                    .checks
                    .iter()
                    .map(|one| {
                        Check::new(
                            one.name,
                            match one.status {
                                "ok" => CheckStatus::Ok,
                                "warn" => CheckStatus::Warn,
                                _ => CheckStatus::Fail,
                            },
                        )
                        .detail(&one.detail)
                        .hint(one.hint)
                    })
                    .collect();
                CheckList::new("runner-health", &checks).show(ui);
            });
            let log_words = ["All", "Runner", "Loads", "Answers", "Pulls"];
            let mut source = views.log_source;
            Section::new("Log")
                .meta(said.lines.len().to_string())
                .actions(|ui| {
                    ui.add(Segmented::new(&mut source, log_words));
                })
                .show(ui, |ui| {
                    Stack::new(Gap::Md).show(ui, |ui| {
                        sokkel_egui::SearchField::new("runner-log-search", &mut views.log_query)
                            .placeholder("Search")
                            .show(ui);
                        let lines: Vec<LogLine> = said
                            .lines
                            .iter()
                            .map(|one| LogLine {
                                at: Some(one.at.clone()),
                                level: Some(match one.level {
                                    "warning" => LogLevel::Warning,
                                    "error" => LogLevel::Error,
                                    _ => LogLevel::Info,
                                }),
                                source: Some(one.source.to_owned()),
                                text: one.text.clone(),
                                fields: Vec::new(),
                            })
                            .collect();
                        LogView::new("runner-log", &lines).follow(true).height(320.0).show(ui);
                    });
                });
            views.log_source = source;
        });
    });
}
