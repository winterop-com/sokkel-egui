//! Every team, as the web's `Customers.tsx`: where it stands, what it pays and paid in the range,
//! and who looks after it; a press opens its page.

use eframe::egui::Ui;
use sokkel_egui::{
    Cell, ChipTone, Column, DataTable, Filter, FilterBar, FilterField, FilterOperator, FilterType, FilterValue, Icon,
    PageHeader, StackLayout, TableState, TableWords,
};

use crate::app::AnalyticsApp;
use crate::days::{day_of, iso_day, short_day, thousands};
use crate::model::{COUNTRIES, PEOPLE, PLANS, REGIONS, Standing, TeamStatus};
use crate::panels::plan_chip;
use crate::view::range_name;
use crate::world::standing;

/// A team with where it stands now and what it paid in the range.
pub struct CustomerRow {
    pub team: usize,
    pub now: Standing,
    pub paid: i64,
}

fn status_word(status: TeamStatus) -> (&'static str, &'static str) {
    match status {
        TeamStatus::Active => ("active", "Active"),
        TeamStatus::Dormant => ("dormant", "Dormant"),
        TeamStatus::Churned => ("churned", "Churned"),
    }
}

/// A team's state as a chip, as the web's `StatusChip` says it.
pub fn team_chip(status: TeamStatus) -> Cell {
    let tone = match status {
        TeamStatus::Active => ChipTone::Good,
        TeamStatus::Dormant => ChipTone::Neutral,
        TeamStatus::Churned => ChipTone::Critical,
    };
    Cell::Chip(status_word(status).1.into(), tone)
}

/// The customers page's own setup: its filters and search.
#[derive(Default)]
pub struct CustomersPage {
    pub filters: Vec<Filter>,
    pub query: String,
}

impl AnalyticsApp {
    pub fn customers_page(&mut self, ui: &mut Ui) {
        let segment = self.segment();
        let (span, _) = self.spans();
        let money = self.money();
        let world = &self.world;
        let mut paid = vec![0i64; world.teams.len()];
        for invoice in &world.invoices {
            if invoice.at >= span.from && invoice.at < span.to && invoice.status == crate::model::InvoiceStatus::Paid {
                paid[invoice.team] += invoice.amount;
            }
        }
        let all: Vec<CustomerRow> = world
            .teams
            .iter()
            .enumerate()
            .map(|(at, team)| CustomerRow {
                team: at,
                now: standing(team, world.now),
                paid: paid[at],
            })
            .filter(|row| {
                segment.region.is_none_or(|r| world.teams[row.team].region == r)
                    && segment.plan.is_none_or(|p| row.now.plan == p)
            })
            .collect();
        let paying = all.iter().filter(|r| r.now.mrr > 0).count();
        PageHeader::new("Customers")
            .count(format!(
                "{} teams, {} paying",
                thousands(all.len() as i64),
                thousands(paying as i64)
            ))
            .show(ui, |_| {});
        self.view_bar(ui);
        let teams = &self.world.teams;
        let fields: Vec<FilterField<'_, CustomerRow>> = vec![
            FilterField::new("plan", "Plan", FilterType::Set, |r: &CustomerRow| {
                FilterValue::Text(r.now.plan.id().into())
            })
            .options(PLANS.iter().map(|p| (p.id(), p.name())))
            .icon(Icon::Layers),
            FilterField::new("status", "State", FilterType::Set, |r: &CustomerRow| {
                FilterValue::Text(status_word(r.now.status).0.into())
            })
            .options([TeamStatus::Active, TeamStatus::Dormant, TeamStatus::Churned].map(status_word))
            .icon(Icon::CircleDot),
            FilterField::new("region", "Region", FilterType::Set, move |r: &CustomerRow| {
                FilterValue::Text(teams[r.team].region.id().into())
            })
            .options(REGIONS.iter().map(|r| (r.id(), r.name())))
            .icon(Icon::Earth),
            FilterField::new("country", "Country", FilterType::Set, move |r: &CustomerRow| {
                FilterValue::Text(teams[r.team].country.clone())
            })
            .options(COUNTRIES.iter().map(|c| (c.name, c.name)))
            .icon(Icon::Flag),
            FilterField::new("owner", "Looked after by", FilterType::Set, move |r: &CustomerRow| {
                FilterValue::Text(teams[r.team].owner.name().into())
            })
            .options(PEOPLE.iter().map(|p| (p.name(), p.name())))
            .icon(Icon::UserRound),
            FilterField::new("seats", "Seats", FilterType::Number, |r: &CustomerRow| {
                FilterValue::Number(r.now.seats as f64)
            })
            .operators([FilterOperator::AtLeast, FilterOperator::AtMost, FilterOperator::Between])
            .icon(Icon::Users),
            FilterField::new("mrr", "MRR", FilterType::Number, |r: &CustomerRow| {
                FilterValue::Number(r.now.mrr as f64)
            })
            .operators([FilterOperator::AtLeast, FilterOperator::AtMost, FilterOperator::Between])
            .icon(Icon::Banknote)
            .format(move |v| v.parse::<f64>().map_or_else(|_| v.to_owned(), |n| money.full(n))),
            FilterField::new("signedUp", "Signed up", FilterType::Date, move |r: &CustomerRow| {
                FilterValue::Day(iso_day(day_of(teams[r.team].created)))
            })
            .icon(Icon::CalendarDays),
        ];
        let kept = sokkel_egui::filter_rows(all.iter(), &self.customers.filters, &fields);
        let wanted = self.customers.query.trim().to_lowercase();
        let rows: Vec<&CustomerRow> = kept
            .into_iter()
            .filter(|r| {
                wanted.is_empty()
                    || format!("{} {}", teams[r.team].name, teams[r.team].country)
                        .to_lowercase()
                        .contains(&wanted)
            })
            .collect();
        // the web's widths less the 12 between two columns, which the web's cells hold inside
        let columns = vec![
            Column::new("Team", 164.0).fill(),
            Column::new("Plan", 92.0),
            Column::new("State", 100.0),
            Column::new("Seats", 60.0).numeric(),
            Column::new("MRR", 84.0).numeric(),
            Column::new("Paid in range", 108.0).numeric(),
            Column::new("Country", 132.0),
            Column::new("Looked after by", 124.0),
            Column::new("Signed up", 92.0),
        ];
        let cells: Vec<Vec<Cell>> = rows
            .iter()
            .map(|r| {
                let team = &teams[r.team];
                vec![
                    Cell::Text(team.name.clone()),
                    plan_chip(r.now.plan),
                    team_chip(r.now.status),
                    Cell::Number {
                        value: r.now.seats as f64,
                        text: r.now.seats.to_string(),
                    },
                    Cell::Number {
                        value: r.now.mrr as f64,
                        text: money.full(r.now.mrr as f64),
                    },
                    Cell::Toned(money.full(r.paid as f64), "muted-foreground"),
                    Cell::Text(team.country.clone()),
                    Cell::Person(team.owner.name().into()),
                    Cell::Number {
                        value: team.created as f64,
                        text: short_day(day_of(team.created)),
                    },
                ]
            })
            .collect();
        let ids: Vec<String> = rows.iter().map(|r| teams[r.team].id.clone()).collect();
        let foot = format!("Paid in range: {}", range_name(&self.view.range).to_lowercase());
        let mut filters = std::mem::take(&mut self.customers.filters);
        let mut query = std::mem::take(&mut self.customers.query);
        // the most paying first, until a column head sorts them
        let state = self
            .tables
            .entry("customers".into())
            .or_insert_with(|| TableState::sorted(4, false));
        DataTable::new("customers", &columns, &cells, state)
            .search(&mut query)
            .toolbar(|ui| {
                FilterBar::new("customer-filters", &fields, &mut filters).show(ui);
            })
            .keys(ids.iter())
            .foot(foot)
            .footer(false)
            .stretch(true)
            .stack_layout(StackLayout::Compact)
            .words(TableWords {
                rows: "{count} teams".into(),
                row: "{count} team".into(),
                empty: "No team matches.".into(),
                ..Default::default()
            })
            .show(ui);
        let opened = state.opened.take().and_then(|row| ids.get(row).cloned());
        drop(fields);
        self.customers.filters = filters;
        self.customers.query = query;
        if let Some(id) = opened {
            self.go(&format!("customers/{id}"));
        }
    }
}
