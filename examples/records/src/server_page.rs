//! What the demo server is and what it offers, as the web's `Server.tsx`: its kinds of record, the
//! parameters each reads, and its operations.

use eframe::egui::{self, Ui};
use sokkel_egui::{ApiLink, Cell, ChipTone, Column, DataTable, DescriptionList, PageHeader, Section, TableVariant};

use crate::app::{Foot, RecordsApp};
use crate::backend::SERVER;

struct Operation {
    name: &'static str,
    on: &'static str,
    does: &'static str,
}

/// A search parameter a kind reads: its name, the type of its value, and what it does.
struct Param {
    name: &'static str,
    kind: &'static str,
    does: &'static str,
}

struct Kind {
    name: &'static str,
    interactions: &'static str,
    params: &'static [Param],
    detail: &'static str,
}

const COUNT: Param = Param {
    name: "count",
    kind: "number",
    does: "How many to answer with at most, 50 when left out.",
};

const OPERATIONS: &[Operation] = &[
    Operation {
        name: "draft",
        on: "forms",
        does: "Answers a form with test answers drawn from a seed, the same again for the same seed. What Fill with test data asks for.",
    },
    Operation {
        name: "forward",
        on: "responses",
        does: "Sends one received response on to the register, which takes it or refuses it with a reason.",
    },
    Operation {
        name: "health",
        on: "the server",
        does: "Runs every check over the metadata and the households, and answers with what it finds.",
    },
];

const KINDS: &[Kind] = &[
    Kind {
        name: "forms",
        interactions: "read, list",
        params: &[
            Param {
                name: "kind",
                kind: "token",
                does: "Only forms of this kind: periodic, event, registration or followup.",
            },
            COUNT,
        ],
        detail: "The forms the server serves, each with its sections and questions.",
    },
    Kind {
        name: "places",
        interactions: "read, list",
        params: &[
            Param {
                name: "level",
                kind: "token",
                does: "Only places at this level: country, region or site.",
            },
            Param {
                name: "parent",
                kind: "reference",
                does: "Only the places directly under this one, by its id.",
            },
            COUNT,
        ],
        detail: "The country, its regions and their sites, each with its parent.",
    },
    Kind {
        name: "households",
        interactions: "list, create by registration",
        params: &[
            Param {
                name: "identifier",
                kind: "token",
                does: "The household with this identifier, matched exactly.",
            },
            COUNT,
        ],
        detail: "The registered households, named by their identifiers.",
    },
    Kind {
        name: "responses",
        interactions: "create, read, list",
        params: &[
            Param {
                name: "state",
                kind: "token",
                does: "Only records in this state: received, forwarded or rejected.",
            },
            Param {
                name: "form",
                kind: "reference",
                does: "Only records filed on this form, by its id.",
            },
            COUNT,
        ],
        detail: "Every record filed, with where it is now.",
    },
];

impl RecordsApp {
    pub fn server_page(&mut self, ui: &mut Ui) {
        self.foot = Foot {
            note: format!(
                "{} kinds served · {} operations declared",
                KINDS.len(),
                OPERATIONS.len()
            ),
            items: Vec::new(),
        };
        PageHeader::new("Server")
            .description("What the server says about itself at /metadata: the whole of what a caller can ask of it.")
            .show(ui, |ui| {
                ui.add(ApiLink::new("/metadata"));
            });
        Section::new(SERVER.name)
            .description("The server the app reads and files through.")
            .show(ui, |ui| {
                DescriptionList::new()
                    .stacked()
                    .columns(2)
                    .code("Version", SERVER.version)
                    .code("Base", SERVER.base)
                    .text("Authentication", "None: this demo sends no credentials")
                    .text("Storage", "This browser, through the demo backend")
                    .show(ui);
            });

        // the web's widths: a list's cells have no padding of their own
        let kind_columns = vec![
            Column::new("Kind", 160.0).fill(),
            Column::new("Interactions", 192.0),
            Column::new("Parameters", 224.0),
        ];
        let kind_rows: Vec<Vec<Cell>> = KINDS
            .iter()
            .map(|kind| {
                vec![
                    Cell::CodeLines {
                        code: kind.name.into(),
                        detail: kind.detail.into(),
                    },
                    Cell::Mono(kind.interactions.into()),
                    Cell::OutlineTags(kind.params.iter().map(|p| p.name.to_owned()).collect()),
                ]
            })
            .collect();
        Section::new("Kinds of record")
            .flat(true)
            .meta(KINDS.len().to_string())
            .description(
                "What a caller can read from the server, and how. Open a kind for the search parameters it reads.",
            )
            .show(ui, |ui| {
                let state = self.tables.entry("server-kinds".into()).or_default();
                DataTable::new("server-kinds", &kind_columns, &kind_rows, state)
                    .variant(TableVariant::List)
                    .auto_rows()
                    .height(f32::INFINITY)
                    .footer(false)
                    .detail(|index, ui| parameters(ui, KINDS[index].params))
                    .show(ui);
            });

        let operation_columns = vec![
            Column::new("Operation", 128.0),
            Column::new("Declared on", 144.0),
            Column::new("What it does", 256.0).fill(),
        ];
        let operation_rows: Vec<Vec<Cell>> = OPERATIONS
            .iter()
            .map(|operation| {
                vec![
                    Cell::Code(operation.name.into()),
                    Cell::Chip(operation.on.into(), ChipTone::Outline),
                    Cell::Text(operation.does.into()),
                ]
            })
            .collect();
        Section::new("Declared operations")
            .flat(true)
            .meta(OPERATIONS.len().to_string())
            .description("What a caller can ask the server to do beyond reading.")
            .show(ui, |ui| {
                let state = self.tables.entry("server-operations".into()).or_default();
                DataTable::new("server-operations", &operation_columns, &operation_rows, state)
                    .variant(TableVariant::List)
                    .auto_rows()
                    .height(f32::INFINITY)
                    .footer(false)
                    .show(ui);
            });
    }
}

/// A kind's search parameters: each name beside what it does and the type of its value.
fn parameters(ui: &mut Ui, params: &[Param]) {
    let mut list = DescriptionList::new().side(112.0).mono_terms();
    for param in params {
        list = list.item(param.name, move |ui| {
            let t = sokkel_egui::tokens(ui.ctx());
            let mut job = egui::text::LayoutJob::default();
            sokkel_egui::TextRole::Body
                .rich(&t, format!("{} ", param.does))
                .color(t.color("foreground"))
                .append_to(&mut job, ui.style(), egui::FontSelection::Default, egui::Align::Center);
            sokkel_egui::TextRole::Help
                .rich(&t, param.kind)
                .color(t.color("muted-foreground"))
                .append_to(&mut job, ui.style(), egui::FontSelection::Default, egui::Align::Center);
            ui.add(egui::Label::new(job).wrap());
        });
    }
    list.show(ui);
}
