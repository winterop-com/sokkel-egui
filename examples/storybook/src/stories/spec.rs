//! Stories for the parts sokkel-design specifies that had none: the API link, the issue count, the
//! server menu, the unfoldable, the sortable grid, the secret field and the prompt.

use eframe::egui::Ui;
use eframe::egui::{Stroke, vec2};
use sokkel_egui::{
    ApiLink, Button, ButtonSize, Cell, CellFocus, Chip, ChipTone, Column, DataTable, Editor, Icon, IssueCount,
    IssueEntry, ItemCard, MenuItem, PromptDialog, SecretField, ServerCheck, ServerMenu, ServerOwnState, ServerState,
    SortableGrid, SortableHandle, TableState, TextRole, Unfoldable, sum_aggregate, tokens,
};

use super::{State, section};

pub fn api_link(_: &mut State, ui: &mut Ui) {
    section(ui, "Methods");
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        ui.add(ApiLink::new("/api/jobs"));
        ui.add(ApiLink::new("/api/jobs/nightly-export/runs").method("POST"));
        ui.add(ApiLink::new("/api/jobs/sync-inventory").method("PATCH"));
        ui.add(ApiLink::new("/api/jobs/rebuild-index").method("DELETE"));
    });
    section(ui, "Linked");
    ui.add(
        ApiLink::new("/api/runs?job=import-orders&state=failed")
            .href("https://api.example.org/api/runs?job=import-orders&state=failed")
            .copy_value("curl https://api.example.org/api/runs?job=import-orders&state=failed"),
    );
    section(ui, "Opens");
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        ui.add(
            ApiLink::new("/api/jobs/nightly-export")
                .href("https://api.example.org/api/jobs/nightly-export")
                .open(true),
        );
        ui.add(
            ApiLink::new("/api/jobs/sync-inventory/runs")
                .method("POST")
                .href("https://api.example.org/api/jobs/sync-inventory/runs")
                .open(true),
        );
    });
    section(ui, "Cut to its room");
    ui.allocate_ui(eframe::egui::vec2(320.0, 24.0), |ui| {
        ui.add(ApiLink::new(
            "/api/projects/harbor/jobs/nightly-export/runs?state=failed&since=2026-11-01",
        ));
    });
}

pub fn unfoldable(_: &mut State, ui: &mut Ui) {
    ui.allocate_ui(vec2(520.0, 420.0), unfoldables);
}

fn unfoldables(ui: &mut Ui) {
    let t = tokens(ui.ctx());
    // the column's edge, so the row reaching 4 past it shows
    let edge = ui.available_rect_before_wrap();
    ui.painter().vline(
        edge.left() - 0.5,
        edge.top()..=edge.top() + 400.0,
        Stroke::new(1.0, t.color("border")),
    );
    ui.add_space(4.0);
    ui.label(
        TextRole::Body
            .rich(&t, "Runs of the harbor project, by job.")
            .color(t.color("foreground")),
    );
    ui.add_space(8.0);
    Unfoldable::new("story-unfold-shut", "nightly-export")
        .summary("12 runs, 1 failed")
        .aside(|ui| {
            ui.add(Chip::new("failed").tone(ChipTone::Status("failed")));
        })
        .show(ui, |ui| {
            ui.label("rebuild-index failed at validate.");
        });
    Unfoldable::new("story-unfold-open", "sync-inventory")
        .default_open(true)
        .summary("8 runs")
        .show(ui, |ui| {
            ui.label(
                TextRole::Body
                    .rich(&t, "Every run succeeded this week.")
                    .color(t.color("foreground")),
            );
        });
    Unfoldable::new("story-unfold-wrap", "send-digest")
        .summary("Sends the digest to ada, grace, linus and margaret every morning at 07:00, once nightly-export has run and the rows have been validated.")
        .wrap_summary(true)
        .show(ui, |ui| {
            ui.label("The digest.");
        });
    Unfoldable::new("story-unfold-cut", "import-orders")
        .summary(
            "Imports the orders of every site in the harbor, juniper and larkspur projects, then notifies the team.",
        )
        .show(ui, |ui| {
            ui.label("The orders.");
        });
    Unfoldable::new("story-unfold-forced", "rebuild-index")
        .forced(true)
        .show(ui, |ui| {
            ui.label(
                TextRole::Body
                    .rich(&t, "Held open while the filter is typed.")
                    .color(t.color("foreground")),
            );
        });
    Unfoldable::new("story-unfold-static", "notify").show_static(ui);
}

pub fn issue_count(_: &mut State, ui: &mut Ui) {
    section(ui, "Counts with nothing to open");
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        IssueCount::counts("story-issues-errors", 2, 0).show(ui);
        IssueCount::counts("story-issues-warnings", 0, 1).show(ui);
        IssueCount::counts("story-issues-both", 3, 1_200).show(ui);
    });
    section(ui, "A list open above the face");
    ui.add_space(220.0);
    let list = [
        IssueEntry {
            label: "Job name".into(),
            text: "A job named nightly-export already exists.".into(),
            warning: false,
        },
        IssueEntry {
            label: "Runs every".into(),
            text: "Pick how often the job runs.".into(),
            warning: false,
        },
        IssueEntry {
            label: "Description".into(),
            text: "Names over 24 characters are cut short in the rail.".into(),
            warning: true,
        },
    ];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        IssueCount::new("story-issues-list", &list).default_open(true).show(ui);
        ui.add(Button::new("Save").icon(Icon::Check).size(ButtonSize::Sm));
    });
}

/// Tidevann's own states for its services, beside up, down and checking.
const SERVER_STATES: [ServerOwnState<'static>; 3] = [
    ServerOwnState {
        name: "healthy",
        words: "Healthy",
        color: "good",
    },
    ServerOwnState {
        name: "degraded",
        words: "Degraded",
        color: "warning",
    },
    ServerOwnState {
        name: "unhealthy",
        words: "Unhealthy",
        color: "critical",
    },
];

const SERVER_FACTS: [&str; 3] = [
    "records.example.org",
    "Serves 8 kinds of record and 3 operations",
    "Keeps what it receives on disk",
];

pub fn server_menu(_: &mut State, ui: &mut Ui) {
    let degraded = [
        ServerCheck {
            name: "api",
            state: ServerState::Own("healthy"),
            detail: Some("48 ms"),
        },
        ServerCheck {
            name: "worker",
            state: ServerState::Own("degraded"),
            detail: Some("214 jobs queued"),
        },
        ServerCheck {
            name: "scheduler",
            state: ServerState::Own("healthy"),
            detail: Some("next run at 02:00"),
        },
    ];
    let items = [MenuItem::action("What this server offers").icon(Icon::ServerCog)];
    section(ui, "Up, down and checking");
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 24.0;
        for (id, state) in [
            ("up", ServerState::Up),
            ("down", ServerState::Down),
            ("checking", ServerState::Checking),
        ] {
            ui.push_id(id, |ui| {
                ServerMenu::new("Tidevann records", state)
                    .version(Some("2.4.0"))
                    .show(ui);
            });
        }
    });
    section(ui, "Quiet, well and wrong, and a face that opens the app's own pane");
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 24.0;
        ui.push_id("quiet", |ui| {
            ServerMenu::new("Tidevann", ServerState::Own("healthy"))
                .version(Some("3.1.0"))
                .states(&SERVER_STATES)
                .quiet(true)
                .show(ui);
        });
        ui.push_id("quiet-wrong", |ui| {
            ServerMenu::new("Tidevann", ServerState::Own("unhealthy"))
                .version(Some("3.1.0"))
                .states(&SERVER_STATES)
                .quiet(true)
                .show(ui);
        });
        ui.push_id("pane", |ui| {
            ServerMenu::new("Tidevann records", ServerState::Up)
                .version(Some("2.4.0"))
                .on_press(true)
                .show(ui);
        });
    });
    section(ui, "The menu open on a state of the app's own");
    ui.horizontal(|ui| {
        ui.add_space(240.0);
        ui.push_id("degraded", |ui| {
            ServerMenu::new("Tidevann", ServerState::Own("degraded"))
                .version(Some("3.1.0"))
                .environment("production")
                .states(&SERVER_STATES)
                .checks(&degraded)
                .facts(&SERVER_FACTS)
                .check(true)
                .items(&items)
                .default_open(true)
                .show(ui);
        });
    });
}

const PROJECTS: [(&str, &str); 4] = [
    ("nightly-export", "Exports the day's orders to harbor"),
    ("sync-inventory", "Keeps juniper's stock in step"),
    ("send-digest", "Mails larkspur's digest at 07:00"),
    ("rebuild-index", "Rebuilds the search index"),
];

pub fn sortable_grid(_: &mut State, ui: &mut Ui) {
    section(ui, "Each card the grip");
    ui.allocate_ui(vec2(720.0, 0.0), |ui| {
        SortableGrid::new("story-sortable", PROJECTS.len(), "Jobs")
            .item_label(|index| PROJECTS[index].0.to_owned())
            .show(ui, |ui, index, width, height| {
                let (name, about) = PROJECTS[index];
                ItemCard::new(name).description(about).show(ui, width, height)
            });
    });
    section(ui, "Moved by a grip in each card");
    ui.allocate_ui(vec2(720.0, 0.0), |ui| {
        SortableGrid::new("story-sortable-grip", 3, "Jobs by grip")
            .handle(SortableHandle::Grip)
            .item_label(|index| PROJECTS[index].0.to_owned())
            .show(ui, |ui, index, width, height| {
                let (name, about) = PROJECTS[index];
                let card = ItemCard::new(name).description(about).show(ui, width, height);
                let grip = eframe::egui::Rect::from_min_size(
                    card.rect.right_top() + vec2(-16.0 - 24.0, 14.0),
                    vec2(24.0, 24.0),
                );
                let mut child = ui.new_child(eframe::egui::UiBuilder::new().max_rect(grip));
                SortableGrid::grip(&mut child, "story-sortable-grip", index, &format!("Move {name}"));
                card
            });
    });
}

pub fn table_editing(_: &mut State, ui: &mut Ui) {
    let columns = [
        Column::new("Job", 160.0).fill().editable(Editor::Text),
        Column::new("Runs", 96.0).numeric().editable(Editor::Number),
        Column::new("Failed", 96.0).numeric().editable(Editor::Number),
    ];
    let rows: Vec<Vec<Cell>> = [("nightly-export", "1,284", "3"), ("sync-inventory", "642", "0")]
        .into_iter()
        .map(|(job, runs, failed)| {
            vec![
                Cell::Text(job.into()),
                Cell::Text(runs.into()),
                Cell::Text(failed.into()),
            ]
        })
        .collect();
    let modes = [
        ("Underline", CellFocus::Underline),
        ("Ring", CellFocus::Ring),
        ("Soft", CellFocus::Soft),
        ("Row", CellFocus::Row),
    ];
    eframe::egui::Grid::new("story-cell-focus")
        .num_columns(2)
        .spacing(vec2(24.0, 8.0))
        .show(ui, |ui| {
            for (index, (title, focus)) in modes.into_iter().enumerate() {
                ui.vertical(|ui| {
                    ui.set_width(440.0);
                    section(ui, title);
                    let id = eframe::egui::Id::unique(("story-cell-focus", index));
                    let mut state = ui.data_mut(|d| d.get_temp::<TableState>(id)).unwrap_or_else(|| {
                        let mut state = TableState::default();
                        state.focus_cell(0, 1);
                        state
                    });
                    DataTable::new(("story-cell-focus", index), &columns, &rows, &mut state)
                        .cell_focus(focus)
                        .footer(false)
                        .height(140.0)
                        .show(ui);
                    ui.data_mut(|d| d.insert_temp(id, state));
                });
                if index % 2 == 1 {
                    ui.end_row();
                }
            }
        });
}

pub fn table_bands(_: &mut State, ui: &mut Ui) {
    let count = |title: &str, band: &str| {
        Column::new(title, 96.0)
            .numeric()
            .band(band)
            .aggregate(sum_aggregate)
            .editable(Editor::Number)
    };
    let columns = [
        Column::new("Job", 180.0).fill(),
        count("Succeeded", "This week"),
        count("Failed", "This week"),
        count("Succeeded", "Last week"),
        count("Failed", "Last week"),
    ];
    let jobs = [
        ("harbor", "nightly-export", ["7", "0", "6", "1"]),
        ("harbor", "send-digest", ["7", "0", "7", "0"]),
        ("juniper", "sync-inventory", ["1,204", "3", "1,180", "12"]),
        ("juniper", "rebuild-index", ["2", "1", "3", "0"]),
        ("larkspur", "import-orders", ["48", "0", "51", "2"]),
    ];
    let rows: Vec<Vec<Cell>> = jobs
        .iter()
        .map(|(_, job, counts)| {
            std::iter::once(Cell::Text((*job).into()))
                .chain(counts.iter().map(|c| Cell::Text((*c).into())))
                .collect()
        })
        .collect();
    let id = eframe::egui::Id::unique("story-bands");
    let mut state = ui.data_mut(|d| d.get_temp::<TableState>(id)).unwrap_or_default();
    DataTable::new("story-bands", &columns, &rows, &mut state)
        .groups(jobs.iter().map(|(project, ..)| *project))
        .footer(false)
        .height(420.0)
        .show(ui);
    ui.data_mut(|d| d.insert_temp(id, state));
}

pub fn secret_field(_: &mut State, ui: &mut Ui) {
    let held = |ui: &mut Ui, key: &str, start: &str| {
        let id = eframe::egui::Id::unique(("story-secret", key));
        let value: String = ui.data(|d| d.get_temp(id)).unwrap_or_else(|| start.to_owned());
        (id, value)
    };
    ui.allocate_ui(vec2(384.0, 0.0), |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 16.0;
            let (id, mut value) = held(ui, "not-set", "");
            ui.add(
                SecretField::new("story-secret-not-set", "API token", &mut value)
                    .placeholder("tv_live_...")
                    .help("Made under Settings, Tokens on the server."),
            );
            ui.data_mut(|d| d.insert_temp(id, value));
            let (id, mut value) = held(ui, "set", "");
            ui.add(
                SecretField::new("story-secret-set", "Webhook secret", &mut value)
                    .sealed(true)
                    .set_at("2 days ago"),
            );
            ui.data_mut(|d| d.insert_temp(id, value));
            let (id, mut value) = held(ui, "replacing", "tv_live_8f2c41");
            ui.add(
                SecretField::new("story-secret-replacing", "Signing key", &mut value)
                    .sealed(true)
                    .default_replacing(true),
            );
            ui.data_mut(|d| d.insert_temp(id, value));
            let (id, mut value) = held(ui, "refused", "short");
            ui.add(
                SecretField::new("story-secret-refused", "Password", &mut value)
                    .required(true)
                    .error(Some("At least 12 characters.")),
            );
            ui.data_mut(|d| d.insert_temp(id, value));
            let (id, mut value) = held(ui, "read-only", "");
            ui.add(
                SecretField::new("story-secret-read-only", "Deploy key", &mut value)
                    .sealed(true)
                    .set_at("in March")
                    .read_only(true),
            );
            ui.data_mut(|d| d.insert_temp(id, value));
        });
    });
}

/// A project's name the app refuses: one already in the workspace.
fn project_issue(value: &str) -> Option<String> {
    ["harbor", "juniper", "larkspur"]
        .contains(&value.trim())
        .then(|| format!("A project named {} already exists.", value.trim()))
}

pub fn prompt_dialog(_: &mut State, ui: &mut Ui) {
    section(ui, "New project");
    let check: &dyn Fn(&str) -> Option<String> = &project_issue;
    let _ = PromptDialog::new("story-prompt", "New project", "Name")
        .description("Projects hold jobs and the people who run them.")
        .help("Lowercase letters, digits and dashes.")
        .default_value("harbor")
        .validate(check)
        .error(Some(
            "The server refused the name: the workspace holds 12 projects, its most.",
        ))
        .show(ui.ctx());
}

pub fn prompt_secret(_: &mut State, ui: &mut Ui) {
    section(ui, "API token for api");
    let _ = PromptDialog::new("story-prompt-secret", "API token for api", "Token")
        .description("The token the scheduler signs its calls to api with.")
        .placeholder("tv_live_...")
        .secret(true)
        .show(ui.ctx());
}
