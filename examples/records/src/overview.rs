//! What waits for review, as the web's `pages/Overview.tsx`: the responses by state, the forms a
//! record starts from in the person's order, and the server behind both.

use eframe::egui::Ui;
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, Chip, ChipTone, DescriptionList, Icon, ItemCard, PageHeader, Section,
    ServerState, SortableGrid, Stat, StatTiles, tokens,
};

use crate::app::{Foot, RecordsApp};
use crate::backend::{ReceiptState, SERVER, STATES};
use crate::forms::{forms, question_count};
use crate::ui::{count, kind_chip, state_dot};

impl RecordsApp {
    pub fn overview(&mut self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let all = &self.backend.receipts;
        let waiting = self.waiting();
        self.foot = Foot {
            note: format!(
                "{} · {} served",
                count(all.len(), "response"),
                count(forms().len(), "form")
            ),
            items: Vec::new(),
        };
        let mut send = false;
        let opened: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
        {
            PageHeader::new("Overview")
                .description(
                    "What the server holds right now: the responses not yet sent on to the register, the forms a record starts from, and the server behind both.",
                )
                .show(ui, |ui| {
                    let label = if waiting == 0 {
                        "Send on".to_owned()
                    } else {
                        format!("Send on {waiting}")
                    };
                    let button = Button::new(label)
                        .icon(Icon::Send)
                        .size(ButtonSize::Sm)
                        .enabled(waiting > 0);
                    send = ui.add(button).clicked();
                });
            Section::new("Responses")
                .flat(true)
                .description(
                    "Every response the server stored, and where each one is now. Each tile opens the list of them.",
                )
                .show(ui, |ui| {
                    let tiles = STATES
                        .iter()
                        .map(|state| {
                            let lit = *state == ReceiptState::Received && waiting > 0;
                            Stat::new(
                                state.label(),
                                self.backend.receipts.iter().filter(|r| r.state == *state).count(),
                            )
                            .dot(state_dot(&t, *state))
                            .note(state.note())
                            .emphasis(lit)
                            .large(lit)
                        })
                        .collect();
                    if let Some(at) = StatTiles::new("overview-states", 4, 120.0, 12.0).show(ui, tiles) {
                        *opened.borrow_mut() = Some(format!("responses/in/{}", STATES[at].key()));
                    }
                });
            let order = self.backend.in_order();
            Section::new("Start a record")
                .flat(true)
                .meta(forms().len().to_string())
                .description("Every form the server serves is one you can fill in and file. A registration registers a household for the follow-ups under it. Drag a form to put the ones you file most first.")
                .actions(|ui| {
                    let all = Button::new("All forms")
                        .trailing(Icon::ArrowRight)
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm);
                    if ui.add(all).clicked() {
                        *opened.borrow_mut() = Some("forms".into());
                    }
                })
                .show(ui, |ui| {
                    let outcome = SortableGrid::new("start-a-record", order.len(), "Forms to start a record from")
                        .min_width(224.0)
                        .show(ui, |ui, index, width, height| {
                            let form = order[index];
                            ItemCard::new(form.title)
                                .code(form.id)
                                .status(|ui| {
                                    ui.add(kind_chip(form));
                                    ui.label(
                                        sokkel_egui::TextRole::Help
                                            .rich(&t, count(question_count(form), "question"))
                                            .color(t.color("muted-foreground")),
                                    );
                                })
                                .show(ui, width, height)
                        });
                    if let Some(index) = outcome.pressed {
                        *opened.borrow_mut() = Some(format!("forms/{}", order[index].id));
                    }
                    if let Some((from, to)) = outcome.moved {
                        let mut ids: Vec<String> = order.iter().map(|f| f.id.to_owned()).collect();
                        let id = ids.remove(from);
                        ids.insert(to, id);
                        self.backend.form_order = ids;
                    }
                });
            let state = self.server_state();
            Section::new("This server")
                .actions(|ui| {
                    let offers = Button::new("What this server offers")
                        .trailing(Icon::ArrowRight)
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm);
                    if ui.add(offers).clicked() {
                        *opened.borrow_mut() = Some("server".into());
                    }
                })
                .show(ui, |ui| {
                    let (status, words) = match state {
                        ServerState::Up => ("succeeded", "answering"),
                        ServerState::Down => ("failed", "not answering"),
                        ServerState::Checking => ("queued", "checking"),
                    };
                    DescriptionList::new()
                        .columns(2)
                        .text("Name", SERVER.name)
                        .code("Version", SERVER.version)
                        .code("Address", SERVER.base)
                        .item("State", move |ui| {
                            ui.add(Chip::new(words).tone(ChipTone::Status(status)));
                        })
                        .show(ui);
                });
        }
        if send {
            self.send_on(ui.ctx());
        }
        if let Some(path) = opened.into_inner() {
            self.go(&path);
        }
    }
}
