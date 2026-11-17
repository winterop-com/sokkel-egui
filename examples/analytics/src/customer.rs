//! One team's page, as the web's `Customer.tsx`: its plan and what it pays, its members at work,
//! what is known of it, its invoices with what can be done about them, and its plan history.

use std::cell::RefCell;

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Button, ButtonVariant, Cell, Column, ConfirmAnswer, ConfirmDialog, DashboardGrid, DashboardPanel, DataTable,
    DescriptionList, Dialog, Empty, Icon, InlineEdit, MenuItem, NumberField, PageHeader, PanelSpan, Segmented,
    StatPanel, StatWords, TableVariant, TimeRangePicker, Timeline, TimelineEvent, Toast,
};
use sokkel_egui_charts::{Series, TimeSeriesPanel};

use crate::app::AnalyticsApp;
use crate::customers::team_chip;
use crate::days::{DAY, counted, day_of, long_day, short_day};
use crate::invoices::status_chip;
use crate::metrics::usage;
use crate::model::{ChangeKind, Invoice, InvoiceStatus, PEOPLE, PLANS, Plan, TeamStatus};
use crate::panels::{describe, plan_chip};
use crate::view::{PRESETS, Range};
use crate::world::standing;

/// What the customer page has open: the plan dialog and its draft, or a question.
#[derive(Default)]
pub struct CustomerPage {
    /// The plan dialog's draft: the plan and the seats.
    pub plan: Option<(Plan, f64)>,
    pub cancelling: bool,
    pub refunding: Option<String>,
}

/// What was asked on the page this frame.
enum Asked {
    Back,
    Cancel,
    ChangePlan,
    Owner(String),
    Invoice(Invoice, usize),
    Range(Range),
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map_or_else(String::new, |c| c.to_uppercase().collect::<String>() + chars.as_str())
}

impl AnalyticsApp {
    pub fn customer_page(&mut self, ui: &mut Ui, id: &str) {
        let Some(&at) = self.world.by_id.get(id) else {
            let back = Empty::new("No such team")
                .description(format!(
                    "No team has the id {id}. It may have been made before the demo data were reset."
                ))
                .show(ui, |ui| {
                    ui.add(Button::new("All customers").variant(ButtonVariant::Outline))
                        .clicked()
                });
            if back {
                self.go("customers");
            }
            return;
        };
        let money = self.money();
        let (span, _) = self.spans();
        let world = &self.world;
        let team = &world.teams[at];
        let now = standing(team, world.now);
        let invoices: Vec<Invoice> = world
            .invoices_of
            .get(&at)
            .map(|list| list.iter().rev().map(|i| world.invoices[*i].clone()).collect())
            .unwrap_or_default();
        let work = usage(team, &span, world.now);
        let lifetime: i64 = invoices
            .iter()
            .filter(|i| i.status == InvoiceStatus::Paid)
            .map(|i| i.amount)
            .sum();
        let months = team.firsts.paid.map(|p| (world.now - p) / (30 * DAY));
        let history: Vec<TimelineEvent> = team
            .changes
            .iter()
            .rev()
            .map(|change| {
                let said = describe(change, Some(team));
                let t = sokkel_egui::tokens(ui.ctx());
                let tone = match said.tone {
                    sokkel_egui::ActivityTone::Critical => Some(t.color("critical")),
                    sokkel_egui::ActivityTone::Good => Some(t.color("good")),
                    sokkel_egui::ActivityTone::Warning => Some(t.color("warning")),
                    _ => None,
                };
                TimelineEvent {
                    title: capitalized(&said.text.replacen(&format!("{} ", team.name), "", 1)),
                    detail: Some(if change.kind == ChangeKind::Churn {
                        "Left".to_owned()
                    } else {
                        format!(
                            "{}, {} {}, {} a month",
                            change.plan.name(),
                            change.seats,
                            if change.seats == 1 { "seat" } else { "seats" },
                            money.full((change.plan.price() * change.seats) as f64)
                        )
                    }),
                    at: long_day(day_of(change.at)),
                    tone,
                    ..Default::default()
                }
            })
            .collect();
        let name = team.name.clone();
        let paying = now.status != TeamStatus::Churned && now.plan != Plan::Free;
        let asked: RefCell<Option<Asked>> = RefCell::new(None);
        let say = |what: Asked| *asked.borrow_mut() = Some(what);
        let header = PageHeader::new(&name)
            .back("Customers")
            .meta_row(true)
            .chip(
                now.plan.name(),
                match plan_chip(now.plan) {
                    Cell::Chip(_, tone) => tone,
                    _ => sokkel_egui::ChipTone::Neutral,
                },
            )
            .chip(
                match now.status {
                    TeamStatus::Active => "Active",
                    TeamStatus::Dormant => "Dormant",
                    TeamStatus::Churned => "Churned",
                },
                match team_chip(now.status) {
                    Cell::Chip(_, tone) => tone,
                    _ => sokkel_egui::ChipTone::Neutral,
                },
            )
            .show(ui, |ui| {
                let restart = now.status == TeamStatus::Churned;
                if ui
                    .add(Button::new(if restart { "Restart" } else { "Change plan" }).icon(Icon::ArrowUpDown))
                    .clicked()
                {
                    say(Asked::ChangePlan);
                }
                if paying
                    && ui
                        .add(Button::new("Cancel plan").icon(Icon::X).variant(ButtonVariant::Outline))
                        .clicked()
                {
                    say(Asked::Cancel);
                }
            });
        if header.back {
            say(Asked::Back);
        }
        let today = day_of(world.now);
        let mut preset = match self.view.range {
            Range::Preset(id) => id.to_owned(),
            Range::Days { .. } => String::new(),
        };
        let mut custom = match self.view.range {
            Range::Days { from, to } => Some((sokkel_egui::Day(from as i32), sokkel_egui::Day(to as i32))),
            Range::Preset(_) => None,
        };
        let presets: Vec<(&str, &str)> = PRESETS
            .iter()
            .filter(|p| p.id != "today")
            .map(|p| (p.id, p.label))
            .collect();
        let about = (
            format!("{}, {}", team.country, team.region.name()),
            long_day(day_of(team.created)),
            team.project.map_or("-".to_owned(), long_day),
            team.invited.map_or("-".to_owned(), long_day),
            team.owner.name().to_owned(),
            team.id.clone(),
        );
        let changes = counted(team.changes.len(), "change");
        let columns = vec![
            Column::new("Invoice", 148.0),
            Column::new("Date", 92.0),
            Column::new("Plan", 92.0),
            Column::new("State", 100.0),
            Column::new("Amount", 84.0).numeric().fill(),
        ];
        let cells: Vec<Vec<Cell>> = invoices
            .iter()
            .map(|i| {
                vec![
                    Cell::Mono(i.id.clone()),
                    Cell::Text(short_day(i.day)),
                    plan_chip(i.plan),
                    status_chip(i.status),
                    Cell::Number {
                        value: i.amount as f64,
                        text: money.full(i.amount as f64),
                    },
                ]
            })
            .collect();
        let state = self.tables.entry(format!("customer-{id}")).or_default();
        DashboardGrid::new(("customer", id)).show(ui, |grid| {
            grid.cell(PanelSpan::Quarter, move |ui| {
                StatPanel::new("MRR", Some(now.mrr as f64))
                    .format(&move |v| money.full(v))
                    .color("chart-1")
                    .show(ui);
            });
            grid.cell(PanelSpan::Quarter, move |ui| {
                StatPanel::new("Seats", Some(now.seats as f64))
                    .color("chart-2")
                    .show(ui);
            });
            grid.cell(PanelSpan::Quarter, move |ui| {
                StatPanel::new("Paid in all", Some(lifetime as f64))
                    .format(&move |v| money.short(v))
                    .color("chart-3")
                    .show(ui);
            });
            grid.cell(PanelSpan::Quarter, move |ui| {
                StatPanel::new("Paying for", months.map(|m| m as f64))
                    .unit(if months == Some(1) { " month" } else { " months" })
                    .color("chart-4")
                    .words(StatWords {
                        no_data: "Never paid".into(),
                        ..Default::default()
                    })
                    .show(ui);
            });
            let (labels, members, seats) = (work.labels.clone(), work.members.clone(), work.seats.clone());
            let as_f64 = |values: Vec<Option<i64>>| values.into_iter().map(|v| v.map(|v| v as f64)).collect::<Vec<_>>();
            let (asked, presets, preset, custom) = (&asked, &presets, &mut preset, &mut custom);
            grid.cell(PanelSpan::TwoThirds, move |ui| {
                TimeSeriesPanel::new("Members at work", labels)
                    .subtitle("each day, against its seats")
                    .series(Series::gappy("Members at work", as_f64(members)).color("chart-2"))
                    .series(Series::gappy("Seats", as_f64(seats)).color("muted-foreground").dashed())
                    .height(224.0)
                    .show_with(
                        ui,
                        Some(|ui: &mut Ui| {
                            if TimeRangePicker::new(preset, presets)
                                .custom(custom, sokkel_egui::Day(today as i32))
                                .show(ui)
                                .changed()
                            {
                                let range = match *custom {
                                    Some((from, to)) if preset.is_empty() => Range::Days {
                                        from: i64::from(from.0),
                                        to: i64::from(to.0),
                                    },
                                    _ => Range::preset(preset),
                                };
                                *asked.borrow_mut() = Some(Asked::Range(range));
                            }
                        }),
                    );
            });
            let about = &about;
            grid.cell(PanelSpan::Third, move |ui| {
                DashboardPanel::new("About").show(ui, |ui| {
                    DescriptionList::new()
                        .side(128.0)
                        .text("Country", about.0.clone())
                        .text("Signed up", about.1.clone())
                        .text("First project", about.2.clone())
                        .text("Invited a teammate", about.3.clone())
                        .item("Looked after by", |ui| {
                            if let Some(owner) =
                                InlineEdit::new(("owner", &about.5), "Looked after by", about.4.clone())
                                    .select(PEOPLE.iter().map(|p| p.name()))
                                    .show(ui)
                            {
                                *asked.borrow_mut() = Some(Asked::Owner(owner));
                            }
                        })
                        .code("Team code", about.5.clone())
                        .show(ui);
                });
            });
            let (columns, cells, invoices) = (&columns, &cells, &invoices);
            let count = invoices.len();
            grid.cell(PanelSpan::TwoThirds, move |ui| {
                DashboardPanel::new("Invoices")
                    .subtitle(format!("{count} in all"))
                    .flush(true)
                    .show(ui, |ui| {
                        if invoices.is_empty() {
                            ui.add_space(32.0);
                            ui.vertical_centered(|ui| ui.label("It has never been billed: it is on Free."));
                            ui.add_space(32.0);
                            return;
                        }
                        let statuses: Vec<InvoiceStatus> = invoices.iter().map(|i| i.status).collect();
                        let out = DataTable::new(("customer-invoices", count), columns, cells, state)
                            .variant(TableVariant::List)
                            .framed(false)
                            .bar(false)
                            .footer(false)
                            .paging(&[10, 25, 50])
                            .actions(move |row| {
                                let status = statuses[row];
                                vec![
                                    MenuItem::action("Charge again")
                                        .icon(Icon::RotateCcw)
                                        .disabled(!matches!(status, InvoiceStatus::Failed | InvoiceStatus::Open)),
                                    MenuItem::action("Mark paid")
                                        .disabled(matches!(status, InvoiceStatus::Paid | InvoiceStatus::Refunded)),
                                    MenuItem::action("Refund")
                                        .icon(Icon::Undo2)
                                        .destructive()
                                        .disabled(status != InvoiceStatus::Paid),
                                ]
                            })
                            .show(ui);
                        if let Some((row, action)) = out.action {
                            *asked.borrow_mut() = Some(Asked::Invoice(invoices[row].clone(), action));
                        }
                    });
            });
            grid.cell(PanelSpan::Third, move |ui| {
                DashboardPanel::new("Plan history").subtitle(changes).show(ui, |ui| {
                    Timeline::new(history).show(ui);
                });
            });
        });
        let asked = asked.into_inner();
        let id = id.to_owned();
        match asked {
            Some(Asked::Back) => self.go("customers"),
            Some(Asked::Cancel) => self.customer.cancelling = true,
            Some(Asked::ChangePlan) => {
                let plan = if now.status == TeamStatus::Churned || now.plan == Plan::Free {
                    Plan::Team
                } else {
                    now.plan
                };
                self.customer.plan = Some((plan, now.seats.max(if now.seats == 0 { 3 } else { 1 }) as f64));
            }
            Some(Asked::Owner(owner)) => {
                if let Some(person) = PEOPLE.iter().find(|p| p.name() == owner) {
                    match self.kept.team_edits.iter_mut().find(|e| e.id == id) {
                        Some(edit) => edit.owner = *person,
                        None => self.kept.team_edits.push(crate::model::TeamEdit {
                            id: id.clone(),
                            owner: *person,
                        }),
                    }
                    self.rebuild();
                    Toast::new(format!("{owner} looks after {name}")).show(ui.ctx());
                }
            }
            Some(Asked::Invoice(invoice, 0)) => self.charge_again(ui.ctx(), &invoice),
            Some(Asked::Invoice(invoice, 1)) => {
                self.set_status(&invoice, InvoiceStatus::Paid);
                Toast::new(format!("{} paid", invoice.id))
                    .description(money.full(invoice.amount as f64))
                    .show(ui.ctx());
            }
            Some(Asked::Invoice(invoice, _)) => self.customer.refunding = Some(invoice.id.clone()),
            Some(Asked::Range(range)) => self.view.range = range,
            None => {}
        }
        self.customer_dialogs(ui.ctx(), &id);
    }

    /// The plan dialog, and the questions before a cancel and a refund.
    fn customer_dialogs(&mut self, ctx: &egui::Context, id: &str) {
        let Some(&at) = self.world.by_id.get(id) else { return };
        let money = self.money();
        let team = self.world.teams[at].clone();
        let now = standing(&team, self.world.now);
        let was = if now.status == TeamStatus::Churned { 0 } else { now.mrr };
        if let Some((mut plan, mut seats)) = self.customer.plan.take() {
            let next = plan.price() as f64 * seats.max(0.0);
            let same = now.status != TeamStatus::Churned && plan == now.plan && seats as i64 == now.seats;
            let why = if seats < 1.0 {
                Some("A plan needs at least one seat.")
            } else if same {
                Some("This is the plan it is on.")
            } else {
                None
            };
            let restart = now.status == TeamStatus::Churned;
            let mut done = false;
            let mut confirmed = false;
            let title = if restart {
                format!("Restart {}", team.name)
            } else {
                format!("Change {}'s plan", team.name)
            };
            let closed = Dialog::new("plan-dialog", title)
                .description(format!("It pays {} a month now.", money.full(was as f64)))
                .show(
                    ctx,
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 16.0;
                        let labels: Vec<String> = PLANS
                            .iter()
                            .map(|p| {
                                if p.price() > 0 {
                                    format!("{}, {} a seat", p.name(), money.full(p.price() as f64))
                                } else {
                                    p.name().to_owned()
                                }
                            })
                            .collect();
                        let mut at = PLANS.iter().position(|p| *p == plan).unwrap_or(0);
                        ui.label(sokkel_egui::TextRole::Label.rich(&sokkel_egui::tokens(ui.ctx()), "Plan"));
                        if ui.add(Segmented::new(&mut at, labels).fill(true)).changed() {
                            plan = PLANS[at];
                        }
                        ui.add(NumberField::new("plan-seats", "Seats", &mut seats).step(1.0));
                        let change = next - was as f64;
                        DescriptionList::new()
                            .side(128.0)
                            .text("Each month", money.full(next))
                            .text(
                                "Change",
                                format!("{}{}", if change >= 0.0 { "+" } else { "-" }, money.full(change.abs())),
                            )
                            .show(ui);
                    },
                    |ui| {
                        let label = if restart { "Restart" } else { "Change plan" };
                        let mut button = Button::new(label);
                        if let Some(why) = why {
                            button = button.reason(why);
                        }
                        if ui.add(button).clicked() {
                            confirmed = true;
                        }
                        if ui.add(Button::new("Cancel").variant(ButtonVariant::Outline)).clicked() {
                            done = true;
                        }
                    },
                );
            if confirmed && why.is_none() {
                let at_now = self.now();
                let change = self.stream.change_plan(
                    &team,
                    plan,
                    seats as i64,
                    Some(crate::model::Person::Ada),
                    at_now,
                    false,
                );
                self.kept.changes.push(change);
                self.rebuild();
                Toast::success(format!("{} is on {}", team.name, plan.name()))
                    .description(format!("{} seats, {} a month", seats as i64, money.full(next)))
                    .show(ctx);
            } else if !(closed || done) {
                self.customer.plan = Some((plan, seats));
            }
        }
        if self.customer.cancelling {
            let answer = ConfirmDialog::new("cancel-plan", format!("Cancel {}'s plan?", team.name), "Cancel plan")
                .detail(format!(
                    "It stops paying {} a month, and counts as a team that left.",
                    money.full(now.mrr as f64)
                ))
                .destructive(true)
                .show(ctx);
            match answer {
                Some(ConfirmAnswer::Confirm) => {
                    self.customer.cancelling = false;
                    let at_now = self.now();
                    let change = self
                        .stream
                        .cancel_plan(&team, Some(crate::model::Person::Ada), at_now, false);
                    self.kept.changes.push(change);
                    self.rebuild();
                    Toast::new(format!("{} has left", team.name))
                        .description(format!("{} a month less", money.full(now.mrr as f64)))
                        .show(ctx);
                }
                Some(_) => self.customer.cancelling = false,
                None => {}
            }
        }
        if let Some(number) = self.customer.refunding.clone() {
            let Some(invoice) = self.world.invoices.iter().find(|i| i.id == number).cloned() else {
                self.customer.refunding = None;
                return;
            };
            let answer = ConfirmDialog::new("customer-refund", format!("Refund {number}?"), "Refund")
                .detail(format!(
                    "{} goes back to {}, and leaves the revenue.",
                    money.full(invoice.amount as f64),
                    team.name
                ))
                .destructive(true)
                .show(ctx);
            match answer {
                Some(ConfirmAnswer::Confirm) => {
                    self.customer.refunding = None;
                    self.set_status(&invoice, InvoiceStatus::Refunded);
                    Toast::new(format!("{number} refunded"))
                        .description(money.full(invoice.amount as f64))
                        .show(ctx);
                }
                Some(_) => self.customer.refunding = None,
                None => {}
            }
        }
    }
}
