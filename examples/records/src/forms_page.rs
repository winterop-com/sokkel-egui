//! Every form the server serves, as the web's `pages/Forms.tsx`: grouped by how each is filed,
//! periodic reports and single events as cards, and programs as lists of their forms.

use eframe::egui::Ui;
use sokkel_egui::{ApiLink, Cell, Column, Columns, DataTable, ItemCard, PageHeader, Section, TableVariant, tokens};

use crate::app::{Foot, RecordsApp};
use crate::forms::{Form, FormKind, PROGRAM_WATER, forms, question_count};
use crate::ui::{count, kind_chip};

impl RecordsApp {
    pub fn forms_page(&mut self, ui: &mut Ui) {
        let of = |kind: FormKind| forms().iter().filter(move |f| f.kind == kind);
        self.foot = Foot {
            note: format!(
                "{} · {} · {}",
                count(of(FormKind::Periodic).count(), "periodic report"),
                count(of(FormKind::Event).count(), "single event"),
                count(1, "program")
            ),
            items: Vec::new(),
        };
        let mut opened: Option<String> = None;
        PageHeader::new("Forms")
            .description(
                "The forms the server serves, grouped by how each is filed: counts for a place over a period, single events, and programs that register a household once and visit it after.",
            )
            .show(ui, |ui| {
                ui.add(ApiLink::new("/forms"));
            });
        for kind in [FormKind::Periodic, FormKind::Event] {
            let listed: Vec<&Form> = of(kind).collect();
            Section::new(kind.group())
                .flat(true)
                .meta(listed.len().to_string())
                .description(kind.note())
                .show(ui, |ui| {
                    let t = tokens(ui.ctx());
                    Columns::new(3)
                        .min(224.0)
                        .gap(12.0)
                        .show(ui, listed.len(), |ui, index, width| {
                            let form = listed[index];
                            let card = ItemCard::new(form.title)
                                .code(form.id)
                                .description(form.description)
                                .status(|ui| {
                                    ui.add(kind_chip(form));
                                    let quiet = |text: String| {
                                        sokkel_egui::TextRole::Help
                                            .rich(&t, text)
                                            .color(t.color("muted-foreground"))
                                    };
                                    ui.label(quiet(count(question_count(form), "question")));
                                    if let Some(period) = form.period {
                                        ui.label(quiet(period.label().to_lowercase()));
                                    }
                                })
                                .show(ui, width, 0.0);
                            if card.clicked() {
                                opened = Some(format!("forms/{}", form.id));
                            }
                        });
                });
        }
        Section::new("Programs")
            .flat(true)
            .meta("1")
            .description(FormKind::Registration.note())
            .show(ui, |ui| {
                let listed: Vec<&Form> = forms().iter().filter(|f| f.program == Some(PROGRAM_WATER.0)).collect();
                Section::new(PROGRAM_WATER.1)
                    .description(PROGRAM_WATER.2)
                    .flush(true)
                    .show(ui, |ui| {
                        let columns = vec![
                            Column::new("Form", 224.0).fill(),
                            Column::new("Kind", 144.0),
                            Column::new("Questions", 96.0).numeric(),
                            Column::new("Code", 128.0),
                        ];
                        let rows: Vec<Vec<Cell>> = listed
                            .iter()
                            .map(|form| {
                                let detail = if form.kind == FormKind::Registration {
                                    "Registers the household once"
                                } else if form.repeats {
                                    "Each visit is its own record"
                                } else {
                                    "One record per household"
                                };
                                vec![
                                    Cell::Lines {
                                        title: form.title.into(),
                                        detail: detail.into(),
                                    },
                                    Cell::Chip(
                                        form.kind.label().into(),
                                        sokkel_egui::ChipTone::Kind(form.kind.color()),
                                    ),
                                    Cell::Number {
                                        value: question_count(form) as f64,
                                        text: question_count(form).to_string(),
                                    },
                                    Cell::Mono(form.id.into()),
                                ]
                            })
                            .collect();
                        let state = self.tables.entry("program-forms".into()).or_default();
                        let out = DataTable::new("program-forms", &columns, &rows, state)
                            .variant(TableVariant::List)
                            .framed(false)
                            .bar(false)
                            .footer(false)
                            .chevron(true)
                            .row_height(56.0)
                            .show(ui);
                        let _ = out;
                        if let Some(index) = state.opened.take() {
                            opened = Some(format!("forms/{}", listed[index].id));
                        }
                    });
            });
        if let Some(path) = opened {
            self.go(&path);
        }
    }
}
