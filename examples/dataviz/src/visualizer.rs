//! The visualizer's frame, as the web's `Visualizer.tsx` lays it out: the rail with its places and
//! the boards, the palette's commands, the trail along the top, the status line's counts, and the
//! page the route names.

use serde::Serialize;

use crate::backend::{Backend, saving_note};
use crate::js;
use crate::model::{Board, Dataset, SavedChart, order_boards};

/// One place in the rail.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RailEntry {
    pub id: String,
    pub label: String,
    pub href: String,
    pub active: bool,
    pub count: Option<usize>,
}

/// The rail's groups: the places (`main`), the boards (`board-list`) and Settings (`system`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RailGroup {
    pub id: &'static str,
    pub title: Option<&'static str>,
    pub entries: Vec<RailEntry>,
}

/// One command in the palette.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Command {
    pub id: String,
    pub label: String,
    pub hint: Option<String>,
    pub keywords: Vec<String>,
    /// Where it goes, or what it opens: `import`, `board-new`.
    pub does: String,
}

/// The palette's own groups, by title, after the rail's places.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CommandGroup {
    pub title: &'static str,
    pub rank: u8,
    pub commands: Vec<Command>,
}

/// One step of the trail along the top.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Crumb {
    pub label: String,
    pub href: Option<String>,
}

/// The page a route names.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "page", rename_all = "camelCase")]
pub enum Page {
    Shared {
        payload: String,
    },
    Loading,
    Datasets,
    Dataset {
        id: String,
    },
    Builder {
        dataset: Option<String>,
    },
    Charts,
    Chart {
        id: String,
    },
    Boards,
    Board {
        id: String,
        presenting: bool,
    },
    /// Nothing is there: a chart since deleted, or a place the rail does not name.
    Nothing {
        message: String,
    },
}

/// What the frame reads from the backend: the rows the pages show, the boards in their order.
pub struct Frame<'a> {
    pub datasets: &'a [Dataset],
    pub charts: &'a [SavedChart],
    pub boards: Vec<Board>,
    /// The route's parts, as the hash names them; Settings open over the page they came from.
    pub route: Vec<String>,
    pub loading: bool,
}

impl<'a> Frame<'a> {
    pub fn of(backend: &'a Backend, route: &[String]) -> Self {
        let prefs = backend.prefs();
        Frame {
            datasets: backend.datasets.seen(),
            charts: backend.charts.seen(),
            boards: order_boards(backend.boards.seen(), prefs.board_order.as_deref()),
            route: route.to_vec(),
            loading: !backend.loaded(),
        }
    }

    fn part(&self, at: usize) -> Option<&str> {
        self.route.get(at).map(String::as_str)
    }

    pub fn rail(&self) -> Vec<RailGroup> {
        let (section, id) = (self.part(0), self.part(1));
        let count = |n: usize| (n > 0).then_some(n);
        let entry = |id: &str, label: &str, active: bool, count: Option<usize>| RailEntry {
            id: id.to_owned(),
            label: label.to_owned(),
            href: format!("#/{id}"),
            active,
            count,
        };
        vec![
            RailGroup {
                id: "main",
                title: None,
                entries: vec![
                    entry(
                        "datasets",
                        "Datasets",
                        matches!(section, Some("datasets" | "build")),
                        count(self.datasets.len()),
                    ),
                    entry("charts", "Charts", section == Some("charts"), count(self.charts.len())),
                    entry("boards", "Boards", section == Some("boards") && id.is_none(), None),
                ],
            },
            RailGroup {
                id: "board-list",
                title: Some("Boards"),
                entries: self
                    .boards
                    .iter()
                    .map(|board| RailEntry {
                        id: format!("board-{}", board.id),
                        label: board.name.clone(),
                        href: format!("#/boards/{}", board.id),
                        active: section == Some("boards") && id == Some(board.id.as_str()),
                        count: None,
                    })
                    .collect(),
            },
            RailGroup {
                id: "system",
                title: Some("System"),
                entries: vec![entry("settings", "Settings", false, None)],
            },
        ]
    }

    /// The palette's own groups: actions, datasets, charts and boards.
    pub fn commands(&self) -> Vec<CommandGroup> {
        let command = |id: String, label: String, hint: Option<String>, keywords: Vec<String>, does: String| Command {
            id,
            label,
            hint,
            keywords,
            does,
        };
        let mut actions = vec![command(
            "import".into(),
            "Import data".into(),
            Some("CSV, TSV or JSON".into()),
            Vec::new(),
            "import".into(),
        )];
        actions.extend(self.datasets.first().map(|one| {
            command(
                "chart-new".into(),
                "New chart".into(),
                Some(format!("From {}", one.name)),
                Vec::new(),
                format!("/build/{}", one.id),
            )
        }));
        actions.push(command(
            "board-new".into(),
            "New board".into(),
            None,
            Vec::new(),
            "board-new".into(),
        ));
        let datasets = self
            .datasets
            .iter()
            .map(|one| {
                command(
                    format!("dataset-{}", one.id),
                    one.name.clone(),
                    Some(format!("{} rows", js::locale_count(one.rows.len()))),
                    one.columns.iter().map(|column| column.name.clone()).collect(),
                    format!("/datasets/{}", one.id),
                )
            })
            .collect();
        let charts = self
            .charts
            .iter()
            .map(|chart| {
                command(
                    format!("chart-{}", chart.id),
                    chart.name.clone(),
                    self.datasets
                        .iter()
                        .find(|one| one.id == chart.dataset_id)
                        .map(|one| one.name.clone()),
                    vec![chart.spec.view.id().to_owned()],
                    format!("/charts/{}", chart.id),
                )
            })
            .collect();
        let boards = self
            .boards
            .iter()
            .flat_map(|board| {
                [
                    command(
                        format!("board-{}", board.id),
                        board.name.clone(),
                        Some(format!("{} charts", board.items.len())),
                        Vec::new(),
                        format!("/boards/{}", board.id),
                    ),
                    command(
                        format!("board-present-{}", board.id),
                        format!("Present {}", board.name),
                        None,
                        vec!["full screen".into(), "present".into()],
                        format!("/boards/{}/present", board.id),
                    ),
                ]
            })
            .collect();
        vec![
            CommandGroup {
                title: "Actions",
                rank: 5,
                commands: actions,
            },
            CommandGroup {
                title: "Datasets",
                rank: 6,
                commands: datasets,
            },
            CommandGroup {
                title: "Charts",
                rank: 7,
                commands: charts,
            },
            CommandGroup {
                title: "Boards",
                rank: 8,
                commands: boards,
            },
        ]
    }

    pub fn trail(&self) -> Vec<Crumb> {
        let crumb = |label: &str, href: Option<&str>| Crumb {
            label: label.to_owned(),
            href: href.map(str::to_owned),
        };
        let (section, id) = (self.part(0), self.part(1));
        match (section, id) {
            (Some("datasets"), None) => vec![crumb("Datasets", None)],
            (Some("datasets"), Some(id)) => vec![
                crumb("Datasets", Some("#/datasets")),
                crumb(
                    self.datasets
                        .iter()
                        .find(|one| one.id == id)
                        .map_or("Dataset", |one| one.name.as_str()),
                    None,
                ),
            ],
            (Some("build"), _) => vec![crumb("Datasets", Some("#/datasets")), crumb("New chart", None)],
            (Some("charts"), None) => vec![crumb("Charts", None)],
            (Some("charts"), Some(id)) => vec![
                crumb("Charts", Some("#/charts")),
                crumb(
                    self.charts
                        .iter()
                        .find(|one| one.id == id)
                        .map_or("Chart", |one| one.name.as_str()),
                    None,
                ),
            ],
            (Some("boards"), None) => vec![crumb("Boards", None)],
            (Some("boards"), Some(id)) => vec![
                crumb("Boards", Some("#/boards")),
                crumb(
                    self.boards
                        .iter()
                        .find(|one| one.id == id)
                        .map_or("Board", |one| one.name.as_str()),
                    None,
                ),
            ],
            (Some("shared"), _) => vec![crumb("Shared chart", None)],
            _ => Vec::new(),
        }
    }

    pub fn page(&self) -> Page {
        let (section, id, mode) = (self.part(0), self.part(1), self.part(2));
        if let (Some("shared"), Some(payload)) = (section, id) {
            return Page::Shared {
                payload: payload.to_owned(),
            };
        }
        if self.loading {
            return Page::Loading;
        }
        match (section, id) {
            (Some("datasets"), None) => Page::Datasets,
            (Some("datasets"), Some(id)) => Page::Dataset { id: id.to_owned() },
            (Some("build"), id) => Page::Builder {
                dataset: id.map(str::to_owned),
            },
            (Some("charts"), None) => Page::Charts,
            (Some("charts"), Some(id)) if self.charts.iter().any(|one| one.id == id) => {
                Page::Chart { id: id.to_owned() }
            }
            (Some("boards"), None) => Page::Boards,
            (Some("boards"), Some(id)) => Page::Board {
                id: id.to_owned(),
                presenting: mode == Some("present"),
            },
            _ => Page::Nothing {
                message: if section == Some("charts") {
                    "There is no such chart here. It may have been deleted."
                } else {
                    "Nothing is here. Pick a place in the rail."
                }
                .to_owned(),
            },
        }
    }
}

/// The status line: its note, its tone, whether it pulses, and the counts it shows.
pub fn status(backend: &Backend) -> (String, &'static str, bool, [usize; 3]) {
    let (note, tone, live) = saving_note(&backend.saving);
    (
        note,
        tone,
        live,
        [
            backend.datasets.seen().len(),
            backend.charts.seen().len(),
            backend.boards.seen().len(),
        ],
    )
}
