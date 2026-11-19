//! What opens over the page, as the web's `dialogs.tsx`, `ImportDialog.tsx` and `pages/Settings.tsx`:
//! importing data, making a board or changing its details, adding a saved chart to a board, sharing
//! a chart as a link, the questions asked before what cannot be undone, and the settings.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Appearance, Button, ButtonSize, ButtonVariant, Cell, Choice, ChoiceList, Column, ConfirmAnswer, ConfirmDialog,
    DataTable, DialogShell, Editor, Field, Icon, Input, Segmented, Select, SettingRow, SettingsDialog, SettingsSection,
    Switch, TableState, TableVariant, TextField, TextRole, ThemeWords, theme_settings_of, tokens,
};

use crate::act::{Act, Asking};
use crate::app::{Dialog, VisualizerApp};
use crate::backend::{Backend, ImportSource, PrefsPatch};
use crate::dialogs::{SPANS, add_description, add_why, board_confirm, board_description, board_title, board_why};
use crate::import_dialog::{ImportDialog, SAMPLE};
use crate::inputs::palette_options;
use crate::model::{Aggregate, ColumnType, Dataset, Preferences};
use crate::pages::settings::{DECIMALS, NUMBER_STYLES, reset_description, style_description};
use crate::themes::CARRIED;

impl VisualizerApp {
    /// The dialog open over the page, if any.
    pub fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.dialog.take() else { return };
        let mut open = true;
        let mut acts: Vec<Act> = Vec::new();
        match &mut dialog {
            Dialog::Import(import) => open = import_dialog(ctx, import, &mut self.backend),
            Dialog::Board {
                editing,
                adding,
                name,
                description,
                then_open,
            } => {
                let answer = DialogShell::new(
                    "board-dialog",
                    board_title(editing.is_some()),
                    board_confirm(editing.is_some()),
                )
                .description(board_description(adding.as_deref()))
                .why(board_why(name))
                .show(ctx, |ui| {
                    let t = tokens(ui.ctx());
                    ui.spacing_mut().item_spacing.y = t.density("field-gap").max(12.0);
                    ui.add(TextField::new("Name", name));
                    ui.add(TextField::new("Description", description).multiline(true));
                });
                match answer {
                    Some(true) => {
                        open = false;
                        match editing {
                            Some(board) => {
                                let (id, name, description) = (board.id.clone(), name.clone(), description.clone());
                                acts.push(Act::change(move |backend| {
                                    backend.save_board_details(&id, &name, &description);
                                }));
                            }
                            None => {
                                let (name, description, adding, go) = (
                                    name.clone(),
                                    description.clone(),
                                    adding.as_deref().cloned(),
                                    *then_open,
                                );
                                acts.push(Act::change(move |backend| {
                                    backend.create_board(&name, &description, adding.as_ref(), go);
                                }));
                            }
                        }
                    }
                    Some(false) => open = false,
                    None => {}
                }
            }
            Dialog::AddChart { board, picked, span } => {
                let charts = self.backend.charts.seen().to_vec();
                let datasets = self.backend.datasets.seen().to_vec();
                // the board with its items as the page shows them, Edit's draft too
                let Some(mut shown) = self.backend.boards.seen_get(board).cloned() else {
                    return;
                };
                if let Some(page) = &self.views.board
                    && page.board == shown.id
                {
                    shown.items = page.items(&shown);
                }
                let offered = crate::dialogs::offered(&shown, &charts, &datasets);
                let chosen = crate::dialogs::chosen(picked.as_deref(), &shown, &charts, &datasets);
                let answer = DialogShell::new("add-chart", "Add a chart", "Add")
                    .description(add_description(&shown))
                    .width(512.0)
                    .why(add_why(&chosen))
                    .show(ctx, |ui| {
                        Field::new("Width")
                            .description("Of the board\u{2019}s width on a wide screen; charts stack on a phone.")
                            .show(ui, |ui| {
                                ui.add(Segmented::new(span, SPANS.iter().map(|one| one.label())).fill(true));
                            });
                        ui.add_space(8.0);
                        if !offered.is_empty() {
                            let choices: Vec<Choice> = offered
                                .iter()
                                .map(|one| Choice {
                                    value: &one.value,
                                    label: &one.label,
                                    description: Some(&one.description),
                                })
                                .collect();
                            let mut value = chosen.clone();
                            if ChoiceList::new("Saved charts", &choices).show(ui, &mut value) {
                                *picked = Some(value);
                            }
                        }
                    });
                match answer {
                    Some(true) => {
                        open = false;
                        let span = SPANS[(*span).min(SPANS.len() - 1)];
                        acts.push(Act::change(move |backend| {
                            backend.add_to_board(&shown, &chosen, span, &charts)
                        }));
                        if let Some(page) = &mut self.views.board
                            && page.arranging()
                        {
                            page.cancel();
                        }
                    }
                    Some(false) => open = false,
                    None => {}
                }
            }
            Dialog::Share {
                name,
                link,
                size,
                dataset,
            } => {
                let description = crate::dialogs::share_description(name, dataset);
                let closed = sokkel_egui::Dialog::new("share", "Share")
                    .description(description)
                    .show(
                        ctx,
                        |ui| {
                            Field::new("Link").show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let mut shown = link.clone();
                                    let width = ui.available_width() - 112.0;
                                    ui.add(Input::new(&mut shown).mono(true).read_only(true).width(width));
                                    if ui
                                        .add(
                                            Button::new("Copy link")
                                                .icon(Icon::Copy)
                                                .variant(ButtonVariant::Outline)
                                                .size(ButtonSize::Sm),
                                        )
                                        .clicked()
                                    {
                                        ui.ctx().copy_text(link.clone());
                                    }
                                });
                            });
                        },
                        |ui| {
                            let t = tokens(ui.ctx());
                            ui.label(
                                TextRole::Help
                                    .rich(&t, size.as_str())
                                    .color(t.color("muted-foreground")),
                            );
                        },
                    );
                if closed {
                    open = false;
                }
            }
        }
        if open {
            self.dialog = Some(dialog);
        }
        for act in acts {
            self.apply(act);
        }
    }

    /// The questions asked before what cannot be undone.
    pub fn questions(&mut self, ctx: &egui::Context) {
        let Some(asking) = self.asking.clone() else { return };
        let charts = self.backend.charts.seen().to_vec();
        let boards = self.backend.boards.seen().to_vec();
        let (question, detail, label, act): (String, String, &str, Option<Act>) = match &asking {
            Asking::DeleteDataset(id) => {
                let Some(dataset) = self.backend.datasets.seen_get(id).cloned() else {
                    self.asking = None;
                    return;
                };
                let (question, detail) = crate::pages::datasets::delete_question(&dataset, &charts);
                (
                    question,
                    detail,
                    "Delete",
                    Some(Act::change(move |backend| backend.delete_dataset(&dataset, &charts))),
                )
            }
            Asking::DropColumn { dataset, column } => {
                let Some(one) = self.backend.datasets.seen_get(dataset).cloned() else {
                    self.asking = None;
                    return;
                };
                let Some(column) = one.columns.iter().find(|one| one.id == *column).cloned() else {
                    self.asking = None;
                    return;
                };
                let (question, detail) = crate::pages::dataset::drop_question(&one, &charts, &column);
                (
                    question,
                    detail,
                    "Delete",
                    Some(Act::change(move |backend| backend.drop_column(&one.id, &column))),
                )
            }
            Asking::DeleteChart(id) => {
                let Some(chart) = self.backend.charts.seen_get(id).cloned() else {
                    self.asking = None;
                    return;
                };
                let (question, detail) = crate::pages::charts::delete_question(&chart, &boards);
                (
                    question,
                    detail,
                    "Delete",
                    Some(Act::change(move |backend| backend.delete_chart(&chart, &boards))),
                )
            }
            Asking::DeleteBoard(id) => {
                let board = self.backend.boards.seen_get(id).cloned();
                let (question, detail) = crate::pages::board::delete_question(board.as_ref());
                (
                    question,
                    detail.to_owned(),
                    "Delete",
                    board.map(|board| Act::change(move |backend| backend.delete_board(&board))),
                )
            }
            Asking::Reset => (
                "Reset the demo data?".to_owned(),
                "Every dataset, chart and board goes back to how the demo started, and what was imported goes. This cannot be undone."
                    .to_owned(),
                "Reset",
                Some(Act::change(Backend::reset_all)),
            ),
        };
        let answer = ConfirmDialog::new(("question", format!("{asking:?}")), &question, label)
            .detail(detail)
            .destructive(true)
            .show(ctx);
        match answer {
            Some(ConfirmAnswer::Confirm) => {
                self.asking = None;
                if let Some(act) = act {
                    self.apply(act);
                }
            }
            Some(_) => self.asking = None,
            None => {}
        }
    }

    /// The settings over the page while they are open, each change kept as it is made.
    pub fn settings_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut active) = self.settings.take() else { return };
        let sections = [
            SettingsSection::new("charts", "Charts", Icon::ChartColumn).group("Preferences"),
            SettingsSection::new("numbers", "Numbers", Icon::Hash).group("Preferences"),
            SettingsSection::new("appearance", "Appearance", Icon::Palette).group("Preferences"),
            SettingsSection::new("data", "Data", Icon::Database)
                .group("This demo")
                .reset(false),
        ];
        let prefs = self.backend.prefs();
        let datasets = self.backend.datasets.seen().to_vec();
        let mut patch: Option<PrefsPatch> = None;
        let mut ask_reset = false;
        let appearance = &mut self.appearance;
        let outcome = SettingsDialog::new("settings", &sections, &mut active).show(ctx, |section, ui| {
            settings_rows(section, ui, &prefs, &datasets, appearance, &mut patch, &mut ask_reset);
        });
        if let Some(patch) = patch {
            let id = prefs.id.clone();
            self.backend.set_prefs(&id, patch);
        }
        if ask_reset {
            self.asking = Some(Asking::Reset);
        }
        if outcome.reset {
            if active == "appearance" {
                self.appearance = Appearance::default();
            } else {
                self.backend.reset_section(&prefs.id, &active);
            }
        }
        if !outcome.close {
            self.settings = Some(active);
        }
    }
}

/// The import dialog: its source, the file or the rows pasted, then the name and the columns read;
/// whether it stays open.
fn import_dialog(ctx: &egui::Context, import: &mut ImportDialog, backend: &mut Backend) -> bool {
    let why = import.why();
    let mut shell = DialogShell::new("import", "Import data", "Import")
        .description("A CSV, TSV or JSON file, or rows pasted.")
        .width(640.0)
        .why(why);
    if import.table.is_none() {
        shell = shell.cancel("Cancel");
    }
    // a file dropped on the window is read as the one picked
    let dropped = ctx.input(|i| i.raw.dropped_files.first().cloned());
    if let Some(file) = dropped
        && import.source == ImportSource::File
    {
        let content = file
            .bytes()
            .ok()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
        if let Some(content) = content {
            let name = file
                .path()
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            import.pick(Some(crate::import_dialog::Picked { name, content }));
        }
    }
    let answer = shell.show(ctx, |ui| {
        let t = tokens(ui.ctx());
        ui.spacing_mut().item_spacing.y = 16.0;
        let mut at = usize::from(import.source == ImportSource::Paste);
        if ui.add(Segmented::new(&mut at, ["File", "Paste"])).changed() {
            import.set_source(if at == 0 {
                ImportSource::File
            } else {
                ImportSource::Paste
            });
        }
        if import.source == ImportSource::File {
            Field::new("File")
                .description(
                    "The first row names the columns. JSON is a list of objects, or of arrays under a header row.",
                )
                .error(import.error.clone())
                .show(ui, |ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(ui.available_width(), 96.0), egui::Sense::hover());
                    ui.painter().rect_stroke(
                        rect,
                        t.radius("control"),
                        egui::Stroke::new(1.0, t.color("border")),
                        egui::StrokeKind::Inside,
                    );
                    let words = match &import.file {
                        Some(file) => file.name.clone(),
                        None => "Drop a CSV, TSV or JSON file here".to_owned(),
                    };
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        words,
                        TextRole::Body.font(&t),
                        t.color("muted-foreground"),
                    );
                });
        } else {
            let mut text = import.text.clone();
            Field::new("Rows")
                .description("Comma, semicolon or tab separated, the first row naming the columns; or JSON.")
                .error(import.error.clone())
                .show(ui, |ui| {
                    ui.add(
                        Input::new(&mut text)
                            .multiline(true)
                            .mono(true)
                            .placeholder(SAMPLE)
                            .name("Rows"),
                    );
                });
            if text != import.text {
                import.paste(&text);
            }
        }
        if import.table.is_some() {
            ui.add(TextField::new("Name", &mut import.name));
            let lines = import.lines();
            Field::new("Columns")
                .description("Types are guessed from the values; press a name or a type to change it.")
                .show(ui, |ui| {
                    let columns = [
                        Column::new("Column", 144.0).fill().editable(Editor::Text),
                        Column::new("Type", 128.0).editable(Editor::Select(
                            ColumnType::ALL.iter().map(|kind| kind.label().to_owned()).collect(),
                        )),
                        Column::new("Values", 160.0).fill_share(1.4),
                    ];
                    let rows: Vec<Vec<Cell>> = lines
                        .iter()
                        .map(|line| {
                            vec![
                                Cell::Text(line.column.name.clone()),
                                Cell::Text(line.column.kind.label().to_owned()),
                                Cell::Muted(line.sample.clone()),
                            ]
                        })
                        .collect();
                    let state_id = ui.scope_id().with("import-columns");
                    let mut state: TableState = ui.data_mut(|d| d.get_temp(state_id)).unwrap_or_default();
                    let out = DataTable::new("import-columns", &columns, &rows, &mut state)
                        .variant(TableVariant::List)
                        .footer(false)
                        .height(200.0)
                        .show(ui);
                    ui.data_mut(|d| d.insert_temp(state_id, state));
                    if let Some(edit) = out.edited {
                        if edit.column == 0 {
                            if !crate::js::trim(&edit.value).is_empty() {
                                import.set_column(edit.row, Some(&edit.value), None);
                            }
                        } else if let Some(kind) = ColumnType::ALL.into_iter().find(|kind| kind.label() == edit.value) {
                            import.set_column(edit.row, None, Some(kind));
                        }
                    }
                });
            if let Some(footer) = import.footer() {
                ui.label(TextRole::Help.rich(&t, footer).color(t.color("muted-foreground")));
            }
        }
    });
    match answer {
        Some(true) => {
            import.save(backend);
            false
        }
        Some(false) => false,
        None => true,
    }
}

/// The rows of a settings section, saying the change asked.
fn settings_rows(
    section: &str,
    ui: &mut Ui,
    prefs: &Preferences,
    datasets: &[Dataset],
    appearance: &mut Appearance,
    patch: &mut Option<PrefsPatch>,
    ask_reset: &mut bool,
) {
    let t = tokens(ui.ctx());
    match section {
        "charts" => {
            SettingRow::new(
                "Colors",
                "The palette every chart takes unless it picks its own: series, slices, a heatmap\u{2019}s and a map\u{2019}s shades.",
            )
            .keywords(&["palette", "series", "swatches"])
            .show(ui, |ui| {
                let options = palette_options(None);
                let mut at = options.iter().position(|one| one.value == prefs.palette).unwrap_or(0);
                let before = at;
                ui.add(
                    Select::new(&mut at, options.iter().map(|one| one.label.clone()), "Colors")
                        .width(172.0)
                        .dot_rows(options.iter().map(|one| one.swatches.iter().map(|token| t.color(token)).collect())),
                );
                if at != before {
                    *patch = Some(PrefsPatch {
                        palette: Some(options[at].value.clone()),
                        ..Default::default()
                    });
                }
            });
            SettingRow::new(
                "Combine figures by",
                "How a new chart combines the rows that fall in one place, until it is changed.",
            )
            .keywords(&["sum", "mean", "average", "aggregate"])
            .show(ui, |ui| {
                let mut at = Aggregate::ALL
                    .iter()
                    .position(|one| *one == prefs.aggregate)
                    .unwrap_or(0);
                if ui
                    .add(Segmented::new(&mut at, Aggregate::ALL.iter().map(|one| one.label())))
                    .changed()
                {
                    *patch = Some(PrefsPatch {
                        aggregate: Some(Aggregate::ALL[at]),
                        ..Default::default()
                    });
                }
            });
            SettingRow::new(
                "Suggest views",
                "Show views that suit the columns picked under the builder\u{2019}s preview.",
            )
            .keywords(&["recommend", "builder"])
            .show(ui, |ui| {
                let mut on = prefs.suggestions;
                ui.add(Switch::new(&mut on, "Suggest views"));
                if on != prefs.suggestions {
                    *patch = Some(PrefsPatch {
                        suggestions: Some(on),
                        ..Default::default()
                    });
                }
            });
        }
        "numbers" => {
            let description = style_description(prefs);
            SettingRow::new("Number format", &description)
                .keywords(&["thousands", "compact", "format"])
                .show(ui, |ui| {
                    let mut at = NUMBER_STYLES.iter().position(|one| *one == prefs.numbers).unwrap_or(0);
                    if ui
                        .add(Segmented::new(&mut at, NUMBER_STYLES.iter().map(|one| one.label())))
                        .changed()
                    {
                        *patch = Some(PrefsPatch {
                            numbers: Some(NUMBER_STYLES[at]),
                            ..Default::default()
                        });
                    }
                });
            SettingRow::new(
                "Decimals",
                "How many decimals every figure keeps; Auto keeps up to two, and drops trailing zeros.",
            )
            .keywords(&["precision", "rounding"])
            .show(ui, |ui| {
                let mut at = DECIMALS.iter().position(|one| *one == prefs.decimals).unwrap_or(0);
                if ui
                    .add(Segmented::new(&mut at, DECIMALS.iter().map(|one| one.label())))
                    .changed()
                {
                    *patch = Some(PrefsPatch {
                        decimals: Some(DECIMALS[at]),
                        ..Default::default()
                    });
                }
            });
        }
        "data" => {
            let description = reset_description(datasets);
            SettingRow::new("Reset demo data", &description).show(ui, |ui| {
                if ui
                    .add(
                        Button::new("Reset")
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm),
                    )
                    .clicked()
                {
                    *ask_reset = true;
                }
            });
        }
        _ => theme_settings_of(ui, appearance, &CARRIED, true, &ThemeWords::default()),
    }
}
