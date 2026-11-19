use eframe::egui::Ui;
use sokkel_egui::{
    Cell, ChipTone, Column, DataTable, Icon, ItemCard, MenuItem, TableState, TableVariant, TextRole, tokens,
};

use super::{State, section};

const JOBS: [&str; 5] = [
    "nightly-export",
    "sync-inventory",
    "rebuild-index",
    "send-digest",
    "import-orders",
];
const STATES: [(&str, ChipTone); 6] = [
    ("succeeded", ChipTone::Good),
    ("failed", ChipTone::Critical),
    ("running", ChipTone::Info),
    ("queued", ChipTone::Neutral),
    ("cancelled", ChipTone::Warning),
    ("skipped", ChipTone::Outline),
];
const PEOPLE: [&str; 4] = ["ada", "grace", "linus", "margaret"];

/// Forty runs, the same every time.
fn runs() -> Vec<Vec<Cell>> {
    (0..40)
        .map(|i: usize| {
            let (state, tone) = STATES[(i * 7 + i / 3) % STATES.len()];
            vec![
                Cell::Mono(format!("run_{:04}", 4120 - i)),
                Cell::Text(JOBS[(i * 3) % JOBS.len()].into()),
                Cell::Chip(state.into(), tone),
                Cell::Text(PEOPLE[i % PEOPLE.len()].into()),
                Cell::Text(format!("{}m {:02}s", (i * 13) % 9 + 1, (i * 37) % 60)),
                Cell::Text(format!("{}", ((i * 7919) % 20000) + 120)),
            ]
        })
        .collect()
}

fn columns() -> Vec<Column> {
    vec![
        Column::new("Run", 120.0),
        Column::new("Job", 180.0),
        Column::new("State", 130.0),
        Column::new("By", 110.0),
        Column::new("Took", 110.0).numeric(),
        Column::new("Rows", 120.0).numeric(),
    ]
}

pub fn data_table(state: &mut State, ui: &mut Ui) {
    let runs = runs();
    section(ui, "Runs, sorted and selectable");
    DataTable::new("runs", &columns(), &runs, &mut state.runs)
        .selectable(true)
        .height(320.0)
        .show(ui);
    section(ui, "A list");
    let jobs: Vec<Vec<Cell>> = JOBS
        .iter()
        .enumerate()
        .map(|(i, job)| {
            vec![
                Cell::Text((*job).into()),
                Cell::Text(
                    [
                        "every day at 02:00",
                        "every hour",
                        "on demand",
                        "every day at 07:00",
                        "every 15 minutes",
                    ][i]
                        .into(),
                ),
            ]
        })
        .collect();
    DataTable::new(
        "jobs",
        &[Column::new("Job", 220.0), Column::new("Schedule", 200.0)],
        &jobs,
        &mut state.jobs,
    )
    .variant(TableVariant::List)
    .show(ui);
    section(ui, "Empty");
    let mut empty = TableState::default();
    DataTable::new("empty", &columns(), &[], &mut empty).show(ui);
}

/// A card's picture: a few bars in the chart colors, as a chart drawn small.
fn bars(ui: &mut Ui, heights: &[f32]) {
    let t = tokens(ui.ctx());
    let room = ui.max_rect().shrink(16.0);
    let width = room.width() / heights.len() as f32;
    for (at, height) in heights.iter().enumerate() {
        let left = room.left() + at as f32 * width + width * 0.2;
        let rect = eframe::egui::Rect::from_min_max(
            eframe::egui::pos2(left, room.bottom() - room.height() * height),
            eframe::egui::pos2(left + width * 0.6, room.bottom()),
        );
        ui.painter().rect_filled(rect, 2.0, t.color("chart-2"));
    }
}

/// The saved charts of the stories, with the view each is drawn as.
const CHARTS: [(&str, &str, Icon); 5] = [
    ("Runs by job and state", "Runs by job", Icon::ChartColumn),
    ("Duration through October", "Runs by job", Icon::ChartLine),
    ("Rows moved this month", "Runs by job", Icon::SquareSigma),
    (
        "Rows moved by project, every project of the workspace",
        "Runs by job",
        Icon::ChartPie,
    ),
    ("Arrivals by week", "Harbor arrivals", Icon::ChartColumn),
];

pub fn data_table_cards(state: &mut State, ui: &mut Ui) {
    let heads = [Column::new("Name", 200.0).fill(), Column::new("Dataset", 160.0)];
    let rows: Vec<Vec<Cell>> = CHARTS
        .iter()
        .map(|(name, dataset, icon)| {
            vec![
                Cell::IconName {
                    icon: *icon,
                    name: (*name).to_owned(),
                },
                Cell::Text((*dataset).to_owned()),
            ]
        })
        .collect();
    let menu = |_: usize| {
        vec![
            MenuItem::action("Edit").icon(Icon::PencilLine),
            MenuItem::Separator,
            MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
        ]
    };
    section(ui, "Cards, with the table a press away");
    let words = sokkel_egui::TableWords {
        rows: "{count} charts".into(),
        ..Default::default()
    };
    DataTable::new("chart-cards", &heads, &rows, &mut state.cards)
        .search(&mut state.empty)
        .words(words)
        .footer(false)
        .height(330.0)
        .cards(240.0, 236.0, |ui, index, width| {
            let (name, dataset, icon) = CHARTS[index];
            let card = ItemCard::new(name)
                .selectable(true)
                .title_lines(1)
                .description(dataset)
                .icon(icon);
            if matches!(icon, Icon::ChartColumn) {
                card.media(|ui| bars(ui, &[0.9, 0.55, 0.3, 0.25, 0.2]))
            } else {
                card.media(move |ui| {
                    let t = tokens(ui.ctx());
                    sokkel_egui::icons::paint(
                        ui.painter(),
                        ui.max_rect().center(),
                        icon,
                        40.0,
                        t.color("muted-foreground"),
                    );
                })
            }
            .show(ui, width, 0.0)
        })
        .actions(menu)
        .show(ui);
    section(ui, "Row actions shown on the row pointed at");
    let runs = runs();
    DataTable::new("hover-actions", &columns(), &runs[..4], &mut state.hovered)
        .actions(menu)
        .actions_on_hover(true)
        .footer(false)
        .height(200.0)
        .show(ui);
}

pub fn item_card(_: &mut State, ui: &mut Ui) {
    section(ui, "A picture over the words, an icon before the title");
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        ItemCard::new("Runs by job and state")
            .description("Runs by job")
            .icon(Icon::ChartColumn)
            .media(|ui| bars(ui, &[0.9, 0.55, 0.3, 0.25, 0.2]))
            .selectable(true)
            .show(ui, 256.0, 0.0);
        ItemCard::new("Rows moved by project, every project of the workspace")
            .title_lines(1)
            .description("A title held to one line, its whole text on hover")
            .icon(Icon::ChartPie)
            .show(ui, 256.0, 0.0);
        ItemCard::new("Rows moved by project, every project of the workspace")
            .description("A title of two lines, the default")
            .icon(Icon::SquareSigma)
            .show(ui, 256.0, 0.0);
    });
    section(
        ui,
        "A code, a status that wraps, facts in columns, chosen, a menu, a code alone",
    );
    let menu = [
        MenuItem::action("Rename").icon(Icon::Pencil),
        MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
    ];
    let mut chosen = None;
    let t = tokens(ui.ctx());
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        ItemCard::new("nightly-export")
            .code("job_01JCF8Q2")
            .status(|ui| {
                ui.add(sokkel_egui::Chip::new("succeeded").tone(ChipTone::Status("succeeded")));
                ui.label(TextRole::Help.rich(&t, "every night at 02:00").color(t.color("muted-foreground")));
                ui.label(TextRole::Help.rich(&t, "harbor").color(t.color("muted-foreground")));
                ui.label(TextRole::Help.rich(&t, "owned by grace").color(t.color("muted-foreground")));
            })
            .description(
                "Exports every order of the day to the harbor bucket, then hands the file to send-digest, which mails it to the team before the morning stand-up begins.",
            )
            .fact("Runs", "1,284")
            .fact("Failed", "3")
            .fact("Duration", "4.2 min")
            .selectable(true)
            .show(ui, 256.0, 0.0);
        ItemCard::new("sync-inventory")
            .description("Chosen from a set: the primary edge and the check")
            .selected(true)
            .selectable(true)
            .menu(&menu, &mut chosen)
            .show(ui, 256.0, 0.0);
        ItemCard::from_code("rebuild-index")
            .description("A card with no name: its code is its title")
            .fact("Documents", "48,210")
            .show(ui, 256.0, 0.0);
    });
}

pub fn data_table_runs(_: &mut State, ui: &mut Ui) {
    use sokkel_egui::CellRun;
    let heads = [
        Column::new("Name", 200.0).fill(),
        Column::new("Formula", 280.0).fill_share(2.0),
    ];
    let formula = |top: &str, joint: &str, bottom: &str, mono: bool| {
        let joint = CellRun::new(joint, if mono { "foreground" } else { "faint" });
        Cell::Runs(vec![
            CellRun::new(format!("{top} "), "muted-foreground"),
            if mono { joint.mono() } else { joint },
            CellRun::new(format!(" {bottom}"), "muted-foreground"),
        ])
    };
    let rows = vec![
        vec![
            Cell::Strong("Exports finished".into()),
            formula("{nightly-export succeeded}", "over", "{nightly-export runs}", false),
        ],
        vec![
            Cell::Strong("Retries within runs".into()),
            formula("{sync-inventory retries}", "<=", "{sync-inventory runs} * 0.1", true),
        ],
    ];
    section(ui, "Formulas in runs of their own inks");
    let id = eframe::egui::Id::unique("story-runs-table");
    let mut state: TableState = ui.data(|d| d.get_temp(id)).unwrap_or_default();
    DataTable::new("runs-table", &heads, &rows, &mut state)
        .auto_rows()
        .footer(false)
        .height(240.0)
        .show(ui);
    ui.data_mut(|d| d.insert_temp(id, state));
}
