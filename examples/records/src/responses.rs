//! Every response the server stored, as the web's `pages/Responses.tsx`, and one response as its
//! receipt, as `pages/Receipt.tsx`: on a page of its own, or in a sheet beside the list.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    ApiLink, Banner, BannerTone, Button, ButtonSize, ButtonVariant, Cell, ChipTone, Column, ConfirmAnswer,
    ConfirmDialog, DataTable, DescriptionList, DetailSheet, EntryColumn, EntryGrid, EntryRow, FilterChip, Icon,
    MenuItem, PageHeader, PageState, Section, Segmented, TableState, TableVariant, TextLink, Timeline, TimelineEvent,
    Toast, ToastKind, tokens,
};

use crate::app::{Foot, RecordsApp};
use crate::backend::{Failure, Receipt, ReceiptState, STATES};
use crate::forms::{Form, cell_key, form_of, forms, grid_cells, is_answered};
use crate::geo::place_name;
use crate::ui::{answer_text, count, kind_chip, on_day, state_chip, state_dot, when};

/// What the list holds open: the form filter, the search, the sheet's receipt and its width.
#[derive(Clone, Debug)]
pub struct ResponsesView {
    pub forms: Vec<String>,
    pub query: String,
    pub opened: Option<String>,
    pub sheet: f32,
    /// A withdraw asked, by its receipt.
    pub withdrawing: Option<String>,
}

impl Default for ResponsesView {
    fn default() -> Self {
        Self {
            forms: Vec::new(),
            query: String::new(),
            opened: None,
            sheet: 576.0,
            withdrawing: None,
        }
    }
}

fn answers_of(receipt: &Receipt) -> usize {
    receipt.answers.0.iter().filter(|(_, v)| is_answered(Some(v))).count()
}

/// The answers in the order the form asks them; one left unanswered is not in the record.
fn answer_rows<'a>(form: &'a Form, receipt: &Receipt) -> Vec<(&'a str, &'a crate::forms::Question, String)> {
    form.sections
        .iter()
        .flat_map(|section| {
            section.questions.iter().filter_map(move |q| {
                let text = answer_text(Some(q), receipt.answers.get(q.id));
                (!text.is_empty()).then_some((section.title, q, text))
            })
        })
        .collect()
}

/// How many questions a receipt answers, as its answers list counts them.
fn answered_in(form: Option<&Form>, receipt: &Receipt) -> usize {
    match form {
        None => 0,
        Some(form) if form.grid.is_none() => answer_rows(form, receipt).len(),
        Some(_) => receipt.answers.0.len(),
    }
}

/// What a receipt asked of the app this frame.
#[derive(Default)]
pub struct Asked {
    pub go: Option<String>,
    pub send: Option<String>,
    pub withdraw: Option<String>,
}

impl RecordsApp {
    pub fn responses_page(&mut self, ui: &mut Ui, state: Option<&str>) {
        let all = self.backend.receipts.clone();
        let chosen = state.and_then(ReceiptState::from_key);
        let view = &mut self.responses;
        let shown: Vec<&Receipt> = all
            .iter()
            .filter(|r| {
                chosen.is_none_or(|c| r.state == c) && (view.forms.is_empty() || view.forms.contains(&r.form_id))
            })
            .collect();
        let waiting = all.iter().filter(|r| r.state == ReceiptState::Received).count();
        let mut items = Vec::new();
        if let Some(c) = chosen {
            items.push(("state".to_owned(), c.label().to_owned()));
        }
        self.foot = Foot {
            note: format!("Showing {} of {}", shown.len(), count(all.len(), "response")),
            items,
        };
        let mut asked = Asked::default();
        let (mut reload, mut send_all) = (false, false);
        PageHeader::new("Responses")
            .description("Every record the server stored, and where each one is now.")
            .show(ui, |ui| {
                let mut send = Button::new("Send on").icon(Icon::Send).size(ButtonSize::Sm);
                if waiting == 0 {
                    send = send.reason("Nothing waits to be sent on.");
                }
                send_all = ui.add(send).clicked();
                reload = ui
                    .add(
                        Button::new("Reload")
                            .icon(Icon::RefreshCw)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm),
                    )
                    .clicked();
                ui.add(ApiLink::new(match chosen {
                    None => "/responses".to_owned(),
                    Some(c) => format!("/responses?state={}", c.key()),
                }));
            });
        let t = tokens(ui.ctx());
        let mut at = chosen.map_or(0, |c| STATES.iter().position(|s| *s == c).map_or(0, |i| i + 1));
        let mut labels = vec!["All".to_owned()];
        labels.extend(STATES.iter().map(|s| s.label().to_owned()));
        let mut counts = vec![Some(all.len())];
        counts.extend(
            STATES
                .iter()
                .map(|s| Some(all.iter().filter(|r| r.state == *s).count())),
        );
        let mut dots = vec![None];
        dots.extend(STATES.iter().map(|s| Some(state_dot(&t, *s))));
        if ui
            .add(Segmented::new(&mut at, labels).counts(counts).dots(dots))
            .changed()
        {
            self.go(&match at {
                0 => "responses".to_owned(),
                i => format!("responses/in/{}", STATES[i - 1].key()),
            });
            return;
        }
        let columns = vec![
            Column::new("Form", 160.0).fill(),
            // the web's widths, less the 12 between columns
            Column::new("Received", 188.0),
            Column::new("Kind", 132.0),
            Column::new("Place", 132.0),
            Column::new("State", 100.0),
            Column::new("Answers", 76.0).numeric(),
            Column::new("Receipt", 132.0),
        ];
        let rows: Vec<Vec<Cell>> = shown
            .iter()
            .map(|r| {
                let form = form_of(&r.form_id);
                vec![
                    Cell::Strong(form.map_or(r.form_id.clone(), |f| f.title.to_owned())),
                    Cell::Number {
                        value: r.received_at as f64,
                        text: when(r.received_at),
                    },
                    form.map_or(Cell::Empty, |f| {
                        Cell::Chip(f.kind.label().into(), ChipTone::Kind(f.kind.color()))
                    }),
                    Cell::Text(place_name(&r.place)),
                    Cell::Chip(r.state.key().into(), state_tone(r.state)),
                    Cell::Number {
                        value: answers_of(r) as f64,
                        text: answers_of(r).to_string(),
                    },
                    Cell::Mono(r.receipt[..12].to_owned()),
                ]
            })
            .collect();
        let options: Vec<(String, String)> = forms().iter().map(|f| (f.id.to_owned(), f.title.to_owned())).collect();
        let view = &mut self.responses;
        let mut forms_chosen = view.forms.clone();
        let state_key = "responses".to_owned();
        let table = self
            .tables
            .entry(state_key)
            .or_insert_with(|| TableState::sorted(1, false));
        let shown_ids: Vec<String> = shown.iter().map(|r| r.receipt.clone()).collect();
        let states: Vec<ReceiptState> = shown.iter().map(|r| r.state).collect();
        let out = DataTable::new("responses", &columns, &rows, table)
            .search(&mut view.query)
            .toolbar(|ui| {
                FilterChip::new("responses-form", "Form", &options, &mut forms_chosen).show(ui);
            })
            .actions(move |row| {
                let mut items = vec![
                    MenuItem::action("Open the full page").icon(Icon::ArrowUpRight),
                    MenuItem::action("Copy the receipt ID").icon(Icon::Copy),
                ];
                match states[row] {
                    ReceiptState::Received | ReceiptState::Rejected => {
                        items.push(MenuItem::Separator);
                        items.push(MenuItem::action("Send on").icon(Icon::Send));
                    }
                    ReceiptState::Forwarded => {
                        items.push(MenuItem::Separator);
                        items.push(MenuItem::action("Withdraw").icon(Icon::Undo2).destructive());
                    }
                    ReceiptState::Withdrawn => {}
                }
                items
            })
            .stretch(true)
            .footer(false)
            .show(ui);
        let view = &mut self.responses;
        view.forms = forms_chosen;
        if let Some(index) = self.tables.get_mut("responses").and_then(|s| s.opened.take()) {
            view.opened = shown_ids.get(index).cloned();
        }
        if let Some((row, item)) = out.action
            && let Some(code) = shown_ids.get(row)
        {
            match item {
                0 => asked.go = Some(format!("responses/{code}")),
                1 => {
                    ui.ctx().copy_text(code.clone());
                    Toast::new("Copied").description(code.clone()).show(ui.ctx());
                }
                _ => match states_of(&all, code) {
                    Some(ReceiptState::Forwarded) => asked.withdraw = Some(code.clone()),
                    _ => asked.send = Some(code.clone()),
                },
            }
        }
        // the receipt opened beside the list
        if let Some(code) = self.responses.opened.clone() {
            match all.iter().find(|r| r.receipt == code) {
                None => self.responses.opened = None,
                Some(receipt) => {
                    let form = form_of(&receipt.form_id);
                    let mut size = self.responses.sheet;
                    let mut sheet_asked = Asked::default();
                    let mut body_go = None;
                    let mut full = false;
                    let closed = DetailSheet::new(
                        "response-sheet",
                        form.map_or(receipt.form_id.as_str(), |f| f.title),
                        &mut size,
                    )
                    .code(&receipt.receipt)
                    .line(|ui| {
                        if let Some(form) = form {
                            ui.add(kind_chip(form));
                        }
                        ui.add(state_chip(receipt.state));
                        full = ui
                            .add(
                                Button::new("Open the full page")
                                    .trailing(Icon::ArrowUpRight)
                                    .variant(ButtonVariant::Outline)
                                    .size(ButtonSize::Sm),
                            )
                            .clicked();
                        ui.add(ApiLink::new(format!("/responses/{}", receipt.receipt)));
                    })
                    .foot(|ui| receipt_actions(ui, receipt, &mut sheet_asked))
                    .show(ui.ctx(), |ui| {
                        body_go = self.receipt_body(ui, receipt);
                    });
                    self.responses.sheet = size;
                    if closed {
                        self.responses.opened = None;
                    }
                    if full {
                        asked.go = Some(format!("responses/{}", receipt.receipt));
                    }
                    asked.go = asked.go.or(sheet_asked.go).or(body_go);
                    asked.send = asked.send.or(sheet_asked.send);
                    asked.withdraw = asked.withdraw.or(sheet_asked.withdraw);
                }
            }
        }
        if reload {
            Toast::new("Read again")
                .description(format!("{} on the server.", count(all.len(), "response")))
                .show(ui.ctx());
        }
        if send_all {
            self.send_on(ui.ctx());
        }
        self.answer(ui.ctx(), asked);
    }

    /// Does what a receipt asked: go somewhere, send it on, or ask before withdrawing it.
    pub fn answer(&mut self, ctx: &egui::Context, asked: Asked) {
        if let Some(code) = asked.send {
            match self.backend.forward(&code, self.now()) {
                Ok(ReceiptState::Rejected) => {
                    let why = self.backend.receipt_of(&code).and_then(|r| r.refusal.clone());
                    let mut toast = Toast::new("Refused by the register").kind(ToastKind::Error);
                    if let Some(why) = why {
                        toast = toast.description(why);
                    }
                    toast.show(ctx);
                }
                Ok(_) => {
                    Toast::success("Sent on")
                        .description("The register took the record.")
                        .show(ctx);
                }
                Err(failure) => {
                    Toast::new("Not sent on")
                        .kind(ToastKind::Error)
                        .description(failure.to_string())
                        .show(ctx);
                }
            }
        }
        if let Some(code) = asked.withdraw {
            self.responses.withdrawing = Some(code);
        }
        if let Some(code) = self.responses.withdrawing.clone() {
            let answer = ConfirmDialog::new("withdraw", "Withdraw this record from the register?", "Withdraw")
                .detail("The register keeps it, no longer active, and its counts leave every report. This cannot be sent on again.")
                .destructive(true)
                .show(ctx);
            match answer {
                Some(ConfirmAnswer::Confirm) => {
                    self.responses.withdrawing = None;
                    match self.backend.withdraw(&code, "Taken back from the receipt.", self.now()) {
                        Ok(()) => {
                            Toast::success("Withdrawn")
                                .description("The register keeps the record, no longer active.")
                                .show(ctx);
                        }
                        Err(failure @ Failure::Unreachable) | Err(failure @ Failure::Refused(_)) => {
                            Toast::new("Not withdrawn")
                                .kind(ToastKind::Error)
                                .description(failure.to_string())
                                .show(ctx);
                        }
                    }
                }
                Some(_) => self.responses.withdrawing = None,
                None => {}
            }
        }
        if let Some(path) = asked.go {
            self.go(&path);
        }
    }

    /// A receipt's body, the same on its page and in the sheet over the list; answers with a page
    /// a link in it asked for.
    pub fn receipt_body(&self, ui: &mut Ui, receipt: &Receipt) -> Option<String> {
        let form = form_of(&receipt.form_id);
        let household = receipt
            .household_id
            .as_deref()
            .and_then(|h| self.backend.households.iter().find(|x| x.id == h));
        let answered = answered_in(form, receipt);
        let cell: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
        let go = &cell;
        let t = tokens(ui.ctx());
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = t.density("page-gap");
            if receipt.state == ReceiptState::Rejected
                && let Some(refusal) = &receipt.refusal
            {
                Banner::new(
                    BannerTone::Critical,
                    format!(
                        "Refused by the register: {refusal} Change nothing here; file the record again once it is put right, or send this one on again."
                    ),
                ).show(ui);
            }
            Section::new("Received")
                .description("The record as it arrived, not a view of what the register holds now.")
                .show(ui, |ui| {
                    DescriptionList::new()
                        .columns(2)
                        .text("Received", when(receipt.received_at))
                        .code("Receipt ID", &receipt.receipt)
                        .item("Form", move |ui| match form {
                            None => {
                                ui.label(&receipt.form_id);
                            }
                            Some(form) => {
                                if ui.add(TextLink::new(form.title)).clicked() {
                                    *go.borrow_mut() = Some(format!("forms/{}", form.id));
                                }
                            }
                        })
                        .text("Answers", answered.to_string())
                        .text("Filed by", &receipt.by)
                        .text(
                            "Identifier",
                            receipt
                                .seed
                                .map_or("Entered by hand".to_owned(), |s| format!("Test data, seed {s}")),
                        )
                        .show(ui);
                });
            Section::new("Filed for")
                .description("Where and when the record is for, read off the stored record.")
                .show(ui, |ui| {
                    let mut list = DescriptionList::new().columns(2).item("Place", move |ui| {
                        if ui.add(TextLink::new(place_name(&receipt.place))).clicked() {
                            *go.borrow_mut() = Some(format!("places/{}", receipt.place));
                        }
                    });
                    if let Some(period) = &receipt.period {
                        list = list
                            .code("Period", period)
                            .text("Period type", form.and_then(|f| f.period).map_or("", |p| p.label()));
                    }
                    if let Some(day) = &receipt.visit_date {
                        list = list.text("Visit date", on_day(day));
                    }
                    if let Some(id) = &receipt.household_id {
                        let name = household.map_or(id.clone(), |h| h.identifier.clone());
                        let id = id.clone();
                        list = list.item("Household", move |ui| {
                            if ui.add(TextLink::new(name)).clicked() {
                                *go.borrow_mut() = Some(format!("people/{id}"));
                            }
                        });
                    }
                    list.item("State", |ui| {
                        ui.add(state_chip(receipt.state));
                    })
                    .show(ui);
                });
            Section::new("Answers")
                .flat(true)
                .meta(format!("{answered} answered"))
                .description("Every question the record answers, in the order the form asks them. A question left unanswered is not in the record.")
                .show(ui, |ui| match form {
                    None => {
                        ui.label(
                            sokkel_egui::TextRole::Help
                                .rich(&t, "The server no longer serves the form this record answers, so there are no questions to show.")
                                .color(t.color("muted-foreground")),
                        );
                    }
                    Some(form) if form.grid.is_none() => {
                        let columns = vec![
                            Column::new("Question", 192.0).fill_share(2.0),
                            Column::new("Key", 144.0),
                            Column::new("Answer", 128.0).fill(),
                        ];
                        let rows: Vec<Vec<Cell>> = answer_rows(form, receipt)
                            .into_iter()
                            .map(|(part, q, text)| {
                                vec![
                                    Cell::Lines {
                                        title: q.label.into(),
                                        detail: part.into(),
                                        over: true,
                                    },
                                    Cell::Mono(q.id.into()),
                                    Cell::Text(text),
                                ]
                            })
                            .collect();
                        let mut state = TableState::default();
                        DataTable::new(("answers", &receipt.receipt), &columns, &rows, &mut state)
                            .variant(TableVariant::List)
                            .bar(false)
                            .footer(false)
                            .row_height(56.0)
                            .height(f32::INFINITY)
                            .show(ui);
                    }
                    Some(form) => {
                        let grid = form.grid.as_ref().expect("a grid form");
                        let rows: Vec<EntryRow> = grid
                            .rows
                            .iter()
                            .map(|r| EntryRow {
                                id: r.id.into(),
                                label: r.label.into(),
                                group: Some(r.part.into()),
                            })
                            .collect();
                        let columns: Vec<EntryColumn> = grid_cells(grid)
                            .into_iter()
                            .map(|c| EntryColumn {
                                id: c.value,
                                label: c.label,
                                band: c.band,
                            })
                            .collect();
                        let value = |row: &str, column: &str| receipt.answers.number(&cell_key(row, column));
                        EntryGrid::new(("grid-answers", &receipt.receipt), &rows, &columns, &value)
                            .widths(192.0, 96.0)
                            .total(false)
                            .read_only(true)
                            .show(ui, |_| {});
                    }
                });
            Section::new("History")
                .flat(true)
                .description("Where the record has been since it arrived.")
                .show(ui, |ui| {
                    let events: Vec<TimelineEvent> = receipt
                        .history
                        .iter()
                        .map(|entry| TimelineEvent {
                            icon: None,
                            tone: Some(t.color(match entry.state {
                                ReceiptState::Rejected => "status-failed",
                                ReceiptState::Forwarded => "status-succeeded",
                                ReceiptState::Received => "status-running",
                                ReceiptState::Withdrawn => "faint",
                            })),
                            title: format!("{} by {}", entry.state.label(), entry.by),
                            at: when(entry.at),
                            detail: entry.note.clone(),
                            person: None,
                            content: None,
                        })
                        .collect();
                    Timeline::new(events).show(ui);
                });
        });
        cell.into_inner()
    }

    pub fn receipt_page(&mut self, ui: &mut Ui, code: &str) {
        let Some(receipt) = self.backend.receipt_of(code).cloned() else {
            PageHeader::new("Response").code(code).show(ui, |_| {});
            PageState::missing(format!("The server holds no response under the receipt {code}.")).show(ui);
            return;
        };
        let form = form_of(&receipt.form_id);
        self.foot = Foot {
            note: format!(
                "{} · received {}",
                count(answered_in(form, &receipt), "answer"),
                when(receipt.received_at)
            ),
            items: Vec::new(),
        };
        let mut asked = Asked::default();
        let mut header = PageHeader::new(form.map_or("Response", |f| f.title)).code(code);
        if let Some(form) = form {
            header = header
                .chip(form.kind.label(), ChipTone::Kind(form.kind.color()))
                .chip(receipt.state.key(), state_tone(receipt.state));
        }
        header.show(ui, |ui| {
            receipt_actions(ui, &receipt, &mut asked);
            ui.add(ApiLink::new(format!("/responses/{}", receipt.receipt)));
        });
        if let Some(go) = self.receipt_body(ui, &receipt) {
            asked.go = Some(go);
        }
        self.answer(ui.ctx(), asked);
    }
}

fn states_of(all: &[Receipt], code: &str) -> Option<ReceiptState> {
    all.iter().find(|r| r.receipt == code).map(|r| r.state)
}

fn state_tone(state: ReceiptState) -> ChipTone {
    match state {
        ReceiptState::Received => ChipTone::Info,
        ReceiptState::Forwarded => ChipTone::Good,
        ReceiptState::Rejected => ChipTone::Critical,
        ReceiptState::Withdrawn => ChipTone::Neutral,
    }
}

/// What can be done with a receipt where it is now: send it on, or take it back out of the
/// register; laid from the end.
fn receipt_actions(ui: &mut Ui, receipt: &Receipt, asked: &mut Asked) {
    match receipt.state {
        ReceiptState::Received | ReceiptState::Rejected => {
            let label = if receipt.state == ReceiptState::Rejected {
                "Send on again"
            } else {
                "Send on"
            };
            if ui
                .add(Button::new(label).icon(Icon::Send).size(ButtonSize::Sm))
                .clicked()
            {
                asked.send = Some(receipt.receipt.clone());
            }
        }
        ReceiptState::Forwarded => {
            if ui
                .add(
                    Button::new("Withdraw")
                        .icon(Icon::Undo2)
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm),
                )
                .clicked()
            {
                asked.withdraw = Some(receipt.receipt.clone());
            }
        }
        ReceiptState::Withdrawn => {}
    }
}
