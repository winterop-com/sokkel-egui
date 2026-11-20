use eframe::egui::Ui;
use sokkel_egui::{
    Cell, ChipTone, Column, DataTable, Icon, ItemCard, MenuItem, ReadOutcome, StackLayout, StackPlace, TableState,
    TableVariant, TextRole, tokens,
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

/// Runs under their jobs under their projects, as the web's `makeRunTree`: each row with its
/// parent's index, the projects and their jobs counting the runs under them.
fn run_tree() -> (Vec<Vec<Cell>>, Vec<Option<usize>>) {
    let mut rows = Vec::new();
    let mut parents = Vec::new();
    let mut run = 4120;
    for (p, project) in ["harbor", "juniper", "larkspur"].into_iter().enumerate() {
        let project_at = rows.len();
        rows.push(Vec::new());
        parents.push(None);
        let (mut runs_in, mut failed_in) = (0, 0);
        for (j, job) in JOBS[..3].iter().enumerate() {
            let job_at = rows.len();
            rows.push(Vec::new());
            parents.push(Some(project_at));
            let count = 2 + (p + j) % 3;
            let mut failed = 0;
            for r in 0..count {
                let (state, tone) = STATES[(p * 5 + j * 3 + r * 2) % 4];
                failed += usize::from(state == "failed");
                let ago = (p * 7 + j * 3 + r) * 37 + 4;
                rows.push(vec![
                    Cell::MutedCode(format!("run_{run:04}")),
                    Cell::Empty,
                    Cell::Empty,
                    Cell::Chip(state.into(), tone),
                    Cell::Text(format!("Nov 5, {:02}:{:02}", 13 - ago / 60 % 12, 59 - ago % 60)),
                    Cell::Text(format!("{}m {:02}s", (run * 13) % 9 + 1, (run * 37) % 60)),
                ]);
                parents.push(Some(job_at));
                run -= 1;
            }
            rows[job_at] = vec![
                Cell::Mono((*job).into()),
                Cell::Text(count.to_string()),
                Cell::Text(failed.to_string()),
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
            ];
            runs_in += count;
            failed_in += failed;
        }
        rows[project_at] = vec![
            Cell::Strong(project.into()),
            Cell::Text(runs_in.to_string()),
            Cell::Text(failed_in.to_string()),
            Cell::Empty,
            Cell::Empty,
            Cell::Empty,
        ];
    }
    (rows, parents)
}

/// Each person's tokens, loaded as their row first opens: the people, and the tokens brought so far.
#[derive(Clone)]
struct Tokens {
    rows: Vec<Vec<Cell>>,
    parents: Vec<Option<usize>>,
    /// When each person's tokens were asked for, by their row, until they come.
    asked: Vec<(usize, f64)>,
    state: TableState,
}

impl Tokens {
    fn new() -> Self {
        let rows = PEOPLE
            .iter()
            .map(|person| vec![Cell::Person((*person).into()), Cell::Empty, Cell::Empty])
            .collect();
        let mut state = TableState::default();
        // linus's tokens refused, and margaret's on their way
        state.expanded.extend([2, 3]);
        state.refuse_children(2);
        Self {
            rows,
            parents: vec![None; PEOPLE.len()],
            asked: Vec::new(),
            state,
        }
    }

    /// The token of the person at `row`, under their row.
    fn bring(&mut self, row: usize) {
        let person = PEOPLE[row];
        let used = ["2 hours ago", "yesterday", "3 days ago", "last week"][row % 4];
        self.rows.push(vec![
            Cell::Mono(format!("{person}-deploy")),
            Cell::Muted(format!("tvn_{}…", &format!("{:x}", (row + 7) * 48_271)[..4])),
            Cell::Text(used.into()),
        ]);
        self.parents.push(Some(row));
    }
}

fn tree_columns() -> Vec<Column> {
    vec![
        Column::new("Name", 260.0).fill().sortable(true),
        Column::new("Runs", 88.0).numeric().sortable(true),
        Column::new("Failed", 88.0).numeric().sortable(true),
        Column::new("State", 120.0),
        Column::new("Started", 120.0),
        Column::new("Took", 88.0).numeric(),
    ]
}

pub fn data_table_tree(_: &mut State, ui: &mut Ui) {
    let (rows, parents) = run_tree();
    section(ui, "Runs under their jobs and projects");
    let id = eframe::egui::Id::unique(("story-table-tree", "state"));
    let mut state = ui.data_mut(|d| d.get_temp::<TableState>(id)).unwrap_or_else(|| {
        let mut state = TableState::default();
        // harbor open, the keys on its nightly-export, one of whose runs is chosen
        state.expanded.insert(0);
        state.focus_row(1);
        state.selected.insert(2);
        state
    });
    DataTable::new("story-table-tree", &tree_columns(), &rows, &mut state)
        .tree(parents)
        .selectable(true)
        .footer(false)
        .show(ui);
    ui.data_mut(|d| d.insert_temp(id, state));

    section(ui, "Tokens loaded as each person opens");
    let id = eframe::egui::Id::unique(("story-table-tokens", "state"));
    let mut tokens = ui.data_mut(|d| d.get_temp::<Tokens>(id)).unwrap_or_else(Tokens::new);
    let now = ui.input(|i| i.time);
    // a person's tokens come two seconds after they were asked for
    let came: Vec<usize> = tokens
        .asked
        .iter()
        .filter(|(_, at)| now - at > 2.0)
        .map(|(row, _)| *row)
        .collect();
    tokens.asked.retain(|(row, _)| !came.contains(row));
    for row in came {
        tokens.bring(row);
    }
    if !tokens.asked.is_empty() {
        ui.ctx().request_repaint();
    }
    let given: Vec<usize> = tokens.parents.iter().flatten().copied().collect();
    let Tokens {
        rows, parents, state, ..
    } = &mut tokens;
    let out = DataTable::new(
        "story-table-tokens",
        &[
            Column::new("Name", 220.0).fill(),
            Column::new("Token", 240.0),
            Column::new("Last used", 140.0),
        ],
        rows,
        state,
    )
    .tree(parents.clone())
    .lazy(|row| row < PEOPLE.len() && !given.contains(&row))
    .expand_label(|row, open| {
        let name = PEOPLE.get(row).copied().unwrap_or_default();
        format!("{} the tokens of {name}", if open { "Hide" } else { "Show" })
    })
    .footer(false)
    .show(ui);
    if let Some(row) = out.load_children {
        tokens.asked.push((row, now));
    }
    ui.data_mut(|d| d.insert_temp(id, tokens));
}

/// A small tree of runs under their jobs under harbor, sync-inventory's runs refused, for the
/// stacked layouts: the rows, their parents, and the state with every branch open.
fn stacked_tree() -> (Vec<Vec<Cell>>, Vec<Option<usize>>, TableState) {
    let run = |id: &str, state: &str, tone: ChipTone, started: &str| {
        vec![
            Cell::MutedCode(id.into()),
            Cell::Chip(state.into(), tone),
            Cell::Text(started.into()),
        ]
    };
    let rows = vec![
        vec![Cell::Mono("harbor".into()), Cell::Empty, Cell::Empty],
        vec![
            Cell::Mono("nightly-export".into()),
            Cell::Empty,
            Cell::Text("Nov 5, 02:00".into()),
        ],
        run("run_4120", "succeeded", ChipTone::Good, "Nov 5, 02:00"),
        run("run_4119", "failed", ChipTone::Critical, "Nov 4, 02:00"),
        vec![
            Cell::Mono("sync-inventory".into()),
            Cell::Empty,
            Cell::Text("Nov 5, 06:30".into()),
        ],
        vec![Cell::Mono("juniper".into()), Cell::Empty, Cell::Empty],
    ];
    let parents = vec![None, Some(0), Some(1), Some(1), Some(0), None];
    let mut state = TableState::default();
    state.expanded.extend([0, 1, 4]);
    state.refuse_children(4);
    state.selected.insert(3);
    (rows, parents, state)
}

pub fn data_table_tree_stacked(_: &mut State, ui: &mut Ui) {
    let columns = [
        Column::new("Name", 220.0).fill(),
        Column::new("State", 120.0),
        Column::new("Started", 140.0),
    ];
    let compact = [
        Column::new("Name", 220.0).fill(),
        Column::new("State", 120.0).stack(StackPlace::Secondary),
        Column::new("Started", 140.0).stack(StackPlace::Secondary),
    ];
    let looks: [(&str, &str, TableVariant, StackLayout); 3] = [
        ("story-tree-cards", "Cards", TableVariant::Table, StackLayout::Labeled),
        ("story-tree-listed", "Listed", TableVariant::List, StackLayout::Labeled),
        (
            "story-tree-compact",
            "Compact",
            TableVariant::List,
            StackLayout::Compact,
        ),
    ];
    ui.columns(3, |thirds| {
        for (third, (key, title, variant, layout)) in thirds.iter_mut().zip(looks) {
            section(third, title);
            let id = eframe::egui::Id::unique((key, "state"));
            let (rows, parents, fresh) = stacked_tree();
            let mut state = third.data_mut(|d| d.get_temp::<TableState>(id)).unwrap_or(fresh);
            let heads: &[Column] = if layout == StackLayout::Compact {
                &compact
            } else {
                &columns
            };
            DataTable::new(key, heads, &rows, &mut state)
                .tree(parents)
                .lazy(|row| row == 4)
                .selectable(true)
                .variant(variant)
                .stack_layout(layout)
                .stack_below(10_000.0)
                .footer(false)
                .height(f32::INFINITY)
                .show(third);
            third.data_mut(|d| d.insert_temp(id, state));
        }
    });
}

pub fn data_table_loading(_: &mut State, ui: &mut Ui) {
    let columns = columns();
    let narrow = &columns[..3];
    ui.columns(2, |halves| {
        section(&mut halves[0], "The first rows on their way");
        let mut search = String::new();
        let mut loading = TableState::default();
        DataTable::new("story-table-loading", narrow, &[], &mut loading)
            .search(&mut search)
            .selectable(true)
            .loading(true)
            .height(160.0)
            .show(&mut halves[0]);

        section(&mut halves[1], "More rows on their way after the last");
        let runs: Vec<Vec<Cell>> = runs().into_iter().take(2).map(|run| run[..3].to_vec()).collect();
        let mut more = TableState::default();
        DataTable::new("story-table-more", narrow, &runs, &mut more)
            .loading(true)
            .footer(false)
            .show(&mut halves[1]);
    });

    section(ui, "A failed read, read again by Try again");
    let id = eframe::egui::Id::unique(("story-table-failed", "since"));
    // Try again reads for 1.2 seconds, placeholder rows over the failure, then fails the same way
    let since: Option<f64> = ui.data(|d| d.get_temp(id));
    let now = ui.input(|i| i.time);
    let reading = since.is_some_and(|at| now - at < 1.2);
    if reading {
        ui.ctx().request_repaint();
    }
    let mut search = String::new();
    let mut failed = TableState::default();
    let out = DataTable::new("story-table-failed", &columns, &[], &mut failed)
        .search(&mut search)
        .selectable(true)
        .loading(reading)
        .failed(
            "The scheduler for harbor is restarting, so its runs cannot be listed.",
            ReadOutcome::Status(503),
        )
        .problems(["harbor: the scheduler answered 503 Service Unavailable"])
        .retry(true)
        .height(200.0)
        .show(ui);
    if out.retry {
        ui.data_mut(|d| d.insert_temp(id, now));
    }
}
