//! Revenue's invoices, as the web's `Invoices` in `Revenue.tsx`: the invoices of the range and
//! segment, filtered, sorted and saved as views, each with what can be done about it.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Cell, ChipTone, Column, ConfirmAnswer, ConfirmDialog, DataTable, Filter, FilterBar, FilterField, FilterOperator,
    FilterType, FilterValue, Icon, ListViewState, MenuItem, SavedView, SavedViews, SavedViewsAsked, StackLayout,
    TableWords, Toast, ToastKind,
};

use crate::app::AnalyticsApp;
use crate::days::{iso_day, short_day};
use crate::model::{BillKind, Invoice, InvoiceChange, InvoiceStatus, PLANS, Person, Plan, REGIONS};
use crate::panels::plan_chip;
use crate::seed::{first, hash};
use crate::view::range_name;

/// The columns' ids, in their order.
const COLUMNS: [&str; 8] = ["id", "team", "plan", "kind", "region", "date", "status", "amount"];

fn kind_name(kind: BillKind) -> &'static str {
    match kind {
        BillKind::Renewal => "A month",
        BillKind::Upgrade => "A new plan",
        BillKind::Seats => "Seats added",
    }
}

fn status_name(status: InvoiceStatus) -> &'static str {
    match status {
        InvoiceStatus::Paid => "Paid",
        InvoiceStatus::Open => "Open",
        InvoiceStatus::Failed => "Failed",
        InvoiceStatus::Refunded => "Refunded",
    }
}

/// An invoice's state as a chip, as the web's `StatusChip` says it.
pub fn status_chip(status: InvoiceStatus) -> Cell {
    let tone = match status {
        InvoiceStatus::Paid => ChipTone::Good,
        InvoiceStatus::Open => ChipTone::Info,
        InvoiceStatus::Failed => ChipTone::Critical,
        InvoiceStatus::Refunded => ChipTone::Warning,
    };
    Cell::Chip(status_name(status).into(), tone)
}

/// The views the demo starts with, as the web's `invoiceViews` seeds them.
pub fn seed_views() -> Vec<SavedView> {
    let view = |id: &str, name: &str, state: ListViewState, locked: bool| SavedView {
        id: id.into(),
        name: name.into(),
        state,
        locked,
    };
    vec![
        view("all", "All invoices", ListViewState::default(), true),
        view(
            "failed",
            "Failed payments",
            ListViewState {
                filters: vec![Filter::new("status", FilterOperator::OneOf, ["failed"])],
                sort: Some(("date".into(), false)),
                hidden: Vec::new(),
            },
            false,
        ),
        view(
            "refunds",
            "Refunds",
            ListViewState {
                filters: vec![Filter::new("status", FilterOperator::OneOf, ["refunded"])],
                sort: None,
                hidden: vec!["kind".into()],
            },
            false,
        ),
        view(
            "large-business",
            "Large Business invoices",
            ListViewState {
                filters: vec![
                    Filter::new("plan", FilterOperator::OneOf, ["business"]),
                    Filter::new("amount", FilterOperator::AtLeast, ["1500"]),
                ],
                sort: Some(("amount".into(), false)),
                hidden: Vec::new(),
            },
            false,
        ),
    ]
}

/// What the invoices page holds: its views, the one chosen and the listing's own setup.
pub struct InvoicesPage {
    pub views: Vec<SavedView>,
    pub chosen: String,
    pub filters: Vec<Filter>,
    pub query: String,
    /// The invoice asked to be refunded, by its number.
    pub refunding: Option<String>,
    made: u32,
}

impl Default for InvoicesPage {
    fn default() -> Self {
        Self {
            views: seed_views(),
            chosen: "all".into(),
            filters: Vec::new(),
            query: String::new(),
            refunding: None,
            made: 0,
        }
    }
}

impl AnalyticsApp {
    /// The listing's setup now: its filters, its sort and its hidden columns, by the columns' ids.
    fn invoices_setup(&mut self) -> ListViewState {
        let state = self.tables.entry("invoices".into()).or_default();
        ListViewState {
            filters: self.invoices.filters.clone(),
            sort: state.sort.map(|(c, up)| (COLUMNS[c].to_owned(), up)),
            hidden: state.hidden.iter().map(|c| COLUMNS[*c].to_owned()).collect(),
        }
    }

    /// Sets the listing to a saved view's setup.
    fn apply_view(&mut self, id: &str) {
        let Some(view) = self.invoices.views.iter().find(|v| v.id == id).cloned() else {
            return;
        };
        self.invoices.chosen = view.id.clone();
        self.invoices.filters = view.state.filters.clone();
        let state = self.tables.entry("invoices".into()).or_default();
        state.sort = view
            .state
            .sort
            .as_ref()
            .and_then(|(c, up)| COLUMNS.iter().position(|one| one == c).map(|c| (c, *up)));
        state.hidden = view
            .state
            .hidden
            .iter()
            .filter_map(|c| COLUMNS.iter().position(|one| one == c))
            .collect();
    }

    /// Sets an invoice's state by hand and builds the world again, as the backend's
    /// `setInvoiceStatus`.
    fn set_invoice_status(&mut self, invoice: &Invoice, status: InvoiceStatus) {
        let now = self.now();
        let change = InvoiceChange {
            id: invoice.key.clone(),
            status,
            by: Person::Ada,
            at: now,
        };
        match self.kept.invoice_changes.iter_mut().find(|c| c.id == invoice.key) {
            Some(known) => *known = change,
            None => self.kept.invoice_changes.push(change),
        }
        self.rebuild();
    }

    pub fn invoices_page(&mut self, ui: &mut Ui) {
        let current = self.invoices_setup();
        let segment = self.segment();
        let (span, _) = self.spans();
        let money = self.money();
        let in_range: Vec<&Invoice> = self
            .world
            .invoices
            .iter()
            .rev()
            .filter(|i| {
                i.at >= span.from
                    && i.at < span.to
                    && segment.region.is_none_or(|r| i.region == r)
                    && segment.plan.is_none_or(|p| i.plan == p)
            })
            .collect();
        let teams = &self.world.teams;
        let fields: Vec<FilterField<'_, Invoice>> = vec![
            FilterField::new("status", "State", FilterType::Set, |i: &Invoice| {
                FilterValue::Text(i.status.id().into())
            })
            .options(
                [
                    InvoiceStatus::Paid,
                    InvoiceStatus::Open,
                    InvoiceStatus::Failed,
                    InvoiceStatus::Refunded,
                ]
                .map(|s| (s.id(), status_name(s))),
            )
            .icon(Icon::CircleDot),
            FilterField::new("plan", "Plan", FilterType::Set, |i: &Invoice| {
                FilterValue::Text(i.plan.id().into())
            })
            .options(PLANS.iter().filter(|p| **p != Plan::Free).map(|p| (p.id(), p.name())))
            .icon(Icon::Layers),
            FilterField::new("region", "Region", FilterType::Set, |i: &Invoice| {
                FilterValue::Text(i.region.id().into())
            })
            .options(REGIONS.iter().map(|r| (r.id(), r.name())))
            .icon(Icon::Earth),
            FilterField::new("kind", "For", FilterType::Set, |i: &Invoice| {
                FilterValue::Text(i.kind.id().into())
            })
            .options([BillKind::Renewal, BillKind::Upgrade, BillKind::Seats].map(|k| (k.id(), kind_name(k))))
            .icon(Icon::Layers),
            FilterField::new("amount", "Amount", FilterType::Number, |i: &Invoice| {
                FilterValue::Number(i.amount as f64)
            })
            .operators([FilterOperator::AtLeast, FilterOperator::AtMost, FilterOperator::Between])
            .icon(Icon::Banknote)
            .format(move |v| v.parse::<f64>().map_or_else(|_| v.to_owned(), |n| money.full(n))),
            FilterField::new("date", "Date", FilterType::Date, |i: &Invoice| {
                FilterValue::Day(iso_day(i.day))
            })
            .icon(Icon::CalendarDays),
        ];
        let kept = sokkel_egui::filter_rows(in_range.iter().copied(), &self.invoices.filters, &fields);
        let wanted = self.invoices.query.trim().to_lowercase();
        let rows_of: Vec<&Invoice> = kept
            .into_iter()
            .filter(|i| {
                wanted.is_empty()
                    || format!("{} {}", i.id, teams[i.team].name)
                        .to_lowercase()
                        .contains(&wanted)
            })
            .collect();
        let asked = SavedViews::new(
            "invoice-views",
            &self.invoices.views,
            Some(&self.invoices.chosen),
            &current,
        )
        .show(ui);
        // the web's widths less the 12 between two columns, which the web's cells hold inside
        let columns = vec![
            Column::new("Invoice", 148.0),
            Column::new("Team", 148.0).fill(),
            Column::new("Plan", 92.0),
            Column::new("For", 108.0),
            Column::new("Region", 132.0),
            Column::new("Date", 92.0),
            Column::new("State", 100.0),
            Column::new("Amount", 100.0).numeric(),
        ];
        let cells: Vec<Vec<Cell>> = rows_of
            .iter()
            .map(|i| {
                vec![
                    Cell::Mono(i.id.clone()),
                    Cell::Text(teams[i.team].name.clone()),
                    plan_chip(i.plan),
                    Cell::Muted(kind_name(i.kind).into()),
                    Cell::Text(i.region.name().into()),
                    Cell::Number {
                        value: i.at as f64,
                        text: short_day(i.day),
                    },
                    status_chip(i.status),
                    Cell::Number {
                        value: i.amount as f64,
                        text: money.full(i.amount as f64),
                    },
                ]
            })
            .collect();
        let paid: i64 = rows_of
            .iter()
            .filter(|i| i.status == InvoiceStatus::Paid)
            .map(|i| i.amount)
            .sum();
        let foot = format!(
            "{} paid of these, {}",
            money.full(paid as f64),
            range_name(&self.view.range).to_lowercase()
        );
        let statuses: Vec<InvoiceStatus> = rows_of.iter().map(|i| i.status).collect();
        let keys: Vec<String> = rows_of.iter().map(|i| i.id.clone()).collect();
        let actions = |row: usize| {
            let status = statuses[row];
            vec![
                MenuItem::action("Open team").icon(Icon::ExternalLink),
                MenuItem::action("Copy number").icon(Icon::Copy),
                MenuItem::Separator,
                MenuItem::action("Charge again")
                    .icon(Icon::RotateCcw)
                    .disabled(!matches!(status, InvoiceStatus::Failed | InvoiceStatus::Open)),
                MenuItem::action("Mark paid").disabled(matches!(status, InvoiceStatus::Paid | InvoiceStatus::Refunded)),
                MenuItem::action("Refund")
                    .icon(Icon::Undo2)
                    .destructive()
                    .disabled(status != InvoiceStatus::Paid),
            ]
        };
        let mut filters = std::mem::take(&mut self.invoices.filters);
        let mut query = std::mem::take(&mut self.invoices.query);
        let state = self.tables.entry("invoices".into()).or_default();
        let out = DataTable::new("invoices", &columns, &cells, state)
            .search(&mut query)
            .toolbar(|ui| {
                FilterBar::new("invoice-filters", &fields, &mut filters).show(ui);
            })
            .keys(keys.iter())
            .actions(actions)
            .foot(foot)
            .footer(false)
            .stretch(true)
            .stack_layout(StackLayout::Compact)
            .words(TableWords {
                rows: "{count} invoices".into(),
                row: "{count} invoice".into(),
                empty: "No invoice in this range matches.".into(),
                ..Default::default()
            })
            .show(ui);
        let opened = state.opened.take();
        drop(fields);
        self.invoices.filters = filters;
        self.invoices.query = query;
        let chosen_row = |row: usize| rows_of.get(row).map(|i| (*i).clone());
        let opened = opened.and_then(chosen_row);
        let acted = out
            .action
            .and_then(|(row, action)| chosen_row(row).map(|i| (i, action)));
        if let Some(invoice) = opened {
            let path = format!("customers/{}", self.world.teams[invoice.team].id);
            self.go(&path);
        }
        if let Some((invoice, action)) = acted {
            self.invoice_action(ui.ctx(), &invoice, action);
        }
        match asked {
            Some(SavedViewsAsked::Picked(id)) => self.apply_view(&id),
            Some(SavedViewsAsked::Save) => {
                let chosen = self.invoices.chosen.clone();
                if let Some(view) = self.invoices.views.iter_mut().find(|v| v.id == chosen) {
                    view.state = current;
                }
            }
            Some(SavedViewsAsked::SaveNew(name)) => {
                self.invoices.made += 1;
                let id = format!("view-{}", self.invoices.made);
                self.invoices.views.push(SavedView {
                    id: id.clone(),
                    name,
                    state: current,
                    locked: false,
                });
                self.invoices.chosen = id;
            }
            Some(SavedViewsAsked::Rename(name)) => {
                let chosen = self.invoices.chosen.clone();
                if let Some(view) = self.invoices.views.iter_mut().find(|v| v.id == chosen) {
                    view.name = name;
                }
            }
            Some(SavedViewsAsked::Delete) => {
                let chosen = self.invoices.chosen.clone();
                self.invoices.views.retain(|v| v.id != chosen);
                self.invoices.filters.clear();
                let state = self.tables.entry("invoices".into()).or_default();
                state.sort = None;
                state.hidden.clear();
                self.invoices.chosen = "all".into();
            }
            None => {}
        }
        self.refund_dialog(ui.ctx());
    }

    /// What a row's menu does, by the action's place in it.
    fn invoice_action(&mut self, ctx: &egui::Context, invoice: &Invoice, action: usize) {
        let money = self.money();
        let name = self.world.teams[invoice.team].name.clone();
        match action {
            0 => {
                let path = format!("customers/{}", self.world.teams[invoice.team].id);
                self.go(&path);
            }
            1 => {
                ctx.copy_text(invoice.id.clone());
                Toast::new("Copied").description(invoice.id.clone()).show(ctx);
            }
            3 => {
                // most cards go through, and about one in five is declined again
                let status = if first(hash(&[self.now(), 5])) < 0.8 {
                    InvoiceStatus::Paid
                } else {
                    InvoiceStatus::Failed
                };
                self.set_invoice_status(invoice, status);
                let toast = if status == InvoiceStatus::Paid {
                    Toast::success(format!("{} paid", invoice.id)).description(format!(
                        "The card went through for {}.",
                        money.full(invoice.amount as f64)
                    ))
                } else {
                    Toast::new(format!("{} declined again", invoice.id))
                        .kind(ToastKind::Error)
                        .description("The card was declined. Try again later.")
                };
                toast.show(ctx);
            }
            4 => {
                self.set_invoice_status(invoice, InvoiceStatus::Paid);
                Toast::new(format!("{} paid", invoice.id))
                    .description(format!("{name}, {}", money.full(invoice.amount as f64)))
                    .show(ctx);
            }
            5 => self.invoices.refunding = Some(invoice.id.clone()),
            _ => {}
        }
    }

    /// The question before a refund.
    fn refund_dialog(&mut self, ctx: &egui::Context) {
        let Some(id) = self.invoices.refunding.clone() else {
            return;
        };
        let Some(invoice) = self.world.invoices.iter().find(|i| i.id == id).cloned() else {
            self.invoices.refunding = None;
            return;
        };
        let money = self.money();
        let name = self.world.teams[invoice.team].name.clone();
        let answer = ConfirmDialog::new("refund", format!("Refund {id}?"), "Refund")
            .detail(format!(
                "{} goes back to {name}, and leaves the revenue.",
                money.full(invoice.amount as f64)
            ))
            .destructive(true)
            .show(ctx);
        match answer {
            Some(ConfirmAnswer::Confirm) => {
                self.invoices.refunding = None;
                self.set_invoice_status(&invoice, InvoiceStatus::Refunded);
                Toast::new(format!("{id} refunded"))
                    .description(format!("{name}, {}", money.full(invoice.amount as f64)))
                    .show(ctx);
            }
            Some(_) => self.invoices.refunding = None,
            None => {}
        }
    }
}
