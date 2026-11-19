//! The parts the models example asked of the library: a progress bar, a health report, a stat
//! tile whose value is a state, a quiet select, a table with badges and cells the app draws, and
//! a treemap.

use eframe::egui::Ui;
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, Cell, Check, CheckList, CheckStatus, ChipTone, Column, Columns, DataTable,
    Progress, Select, Stat, TableState, TableVariant,
};
use sokkel_egui_charts::TreemapPanel;
use sokkel_egui_charts::data::{Cell as Value, Row};

use super::{State, section};

pub fn progress(_: &mut State, ui: &mut Ui) {
    section(ui, "Memory the loaded models take");
    ui.add(
        Progress::new(59.2)
            .label("tide-7b-instruct 4.99 GB, brine-coder-14b 9.25 GB")
            .quiet(true),
    );
    section(ui, "A pull, small in a table's cell");
    ui.add(Progress::new(42.0).label("2.10 GB of 4.39 GB").small(true));
    section(ui, "Begun and nearly done");
    ui.add(Progress::new(0.4).label("nightly-export"));
    ui.add_space(12.0);
    ui.add(Progress::new(99.6).label("sync-inventory"));
}

pub fn check_list(_: &mut State, ui: &mut Ui) {
    let checks = [
        Check::new("Runner", CheckStatus::Ok).detail("Answering on runner.example.org:8400 for 5 h 12 min"),
        Check::new("Memory", CheckStatus::Warn)
            .detail("22.1 GB of the 24.0 GB the runner may use (36.0 GB in this machine)")
            .hint(Some("Unload a model, or raise the limit in Settings.")),
        Check::new("Library root", CheckStatus::Fail)
            .detail("models on the system disk")
            .hint(Some("Set a folder path in Settings.")),
        Check::new("Workers", CheckStatus::Ok).detail("Two of two answering"),
        Check::new("worker-a", CheckStatus::Ok)
            .group("Workers")
            .detail("harbor"),
        Check::new("worker-b", CheckStatus::Warn)
            .group("Workers")
            .detail("juniper, slow to answer"),
    ];
    section(ui, "Health, a group opened by its worst check");
    CheckList::new("story-checks", &checks).show(ui);
}

pub fn stat_chip(_: &mut State, ui: &mut Ui) {
    section(ui, "A state where the number stands");
    let tiles = [
        Stat::new("State", "").chip("up", ChipTone::Good).note("for 5 h 12 min"),
        Stat::new("Loaded", 2).note("15 models on disk"),
        Stat::new("Memory", "14.2 GB").note("of 24.0 GB"),
    ];
    Columns::new(3)
        .min(160.0)
        .gap(24.0)
        .show(ui, tiles.len(), |ui, index, width| {
            tiles[index].clone().tile(ui, width);
        });
}

pub fn select_quiet(state: &mut State, ui: &mut Ui) {
    let models = ["tide-7b-instruct Q4_K_M", "kelp-3b Q4_K_M", "brine-coder-14b 4bit"];
    section(ui, "Small, as a field");
    ui.add(Select::new(&mut state.tab, models, "Model").small(true).width(232.0));
    section(ui, "Quiet, in a composer's tools");
    ui.add(
        Select::new(&mut state.period, models, "Model")
            .small(true)
            .quiet(true)
            .width(232.0),
    );
}

pub fn table_cells(state: &mut State, ui: &mut Ui) {
    section(ui, "A format as a badge, and a button the app draws in a cell");
    let columns = [
        Column::new("Model", 180.0).fill(),
        Column::new("Format", 100.0),
        Column::new("Size", 84.0).numeric(),
        Column::new("Pull", 140.0).sortable(false),
    ];
    let rows: Vec<Vec<Cell>> = [
        ("tide-7b-instruct", "packed", "4.39 GB"),
        ("brine-coder-14b", "unified", "8.29 GB"),
        ("reef-8b", "sharded", "16.1 GB"),
    ]
    .into_iter()
    .map(|(name, format, size)| {
        vec![
            Cell::Mono(name.into()),
            Cell::Badge(format.into()),
            Cell::Text(size.into()),
            Cell::Slot { words: String::new() },
        ]
    })
    .collect();
    let _ = state;
    let key = ui.scope_id().with("story-cells-state");
    let mut table: TableState = ui.data(|d| d.get_temp(key)).unwrap_or_default();
    DataTable::new("story-cells", &columns, &rows, &mut table)
        .variant(TableVariant::List)
        .footer(false)
        .height(f32::INFINITY)
        .slot(|ui, row, _| {
            let said = if row == 0 { "In library" } else { "Pull Q4_K_M" };
            ui.add(Button::new(said).variant(ButtonVariant::Outline).size(ButtonSize::Sm));
        })
        .show(ui);
    ui.data_mut(|d| d.insert_temp(key, table));
}

pub fn treemap(_: &mut State, ui: &mut Ui) {
    let rows: Vec<Row> = [
        ("Public hub", "packed", "tide-7b-instruct Q4_K_M", 4.4),
        ("Public hub", "packed", "tide-14b-instruct Q4_K_M", 8.6),
        ("Public hub", "packed", "reef-32b Q3_K_S", 14.0),
        ("Public hub", "sharded", "brine-coder-14b BF16", 29.4),
        ("Public hub", "unified", "eddy-22b 4bit", 12.5),
        ("Local runner", "packed", "kelp-3b Q4_K_M", 1.9),
        ("Local runner", "packed", "cove-embed-0.3b F16", 0.6),
        ("Desktop runner", "unified", "brine-coder-14b 4bit", 8.3),
        ("Desktop runner", "packed", "drift-vision-11b Q4_K_M", 6.5),
    ]
    .into_iter()
    .map(|(source, format, name, size)| {
        Row::of([
            ("source", Value::from(source)),
            ("format", Value::from(format)),
            ("name", Value::from(name)),
            ("size", Value::Number(size)),
        ])
    })
    .collect();
    TreemapPanel::new("Every model", &rows, &["source", "format", "name"], "size")
        .subtitle("by source, then format, sized by what it takes")
        .format(|value| format!("{value:.1} GB"))
        .height(320.0)
        .show(ui);
}
