//! Everything the analytics read, as it stands now, as the web's `world.ts` builds it: the seeded
//! history joined with what the backend keeps, each team with its changes, the invoices those
//! changes bill, and the latest changes for the live feed.

use std::collections::HashMap;

use crate::days::{DAY, civil, day_of};
use crate::model::{
    BillKind, Change, ChangeKind, Firsts, Invoice, InvoiceChange, InvoiceStatus, Plan, Standing, Team, TeamEdit,
    TeamRecord, TeamStatus,
};
use crate::seed::{History, Names, first, hash, history, round};

pub struct World {
    /// When it was built.
    pub now: i64,
    /// Every team, oldest first.
    pub teams: Vec<Team>,
    pub by_id: HashMap<String, usize>,
    /// Every invoice, oldest first.
    pub invoices: Vec<Invoice>,
    /// Each team's invoices, by their places in `invoices`.
    pub invoices_of: HashMap<usize, Vec<usize>>,
    /// The latest changes of all, newest first.
    pub latest: Vec<Change>,
    /// How many changes the live stream and people made, out of the history.
    pub live_count: usize,
    pub names: Names,
}

impl World {
    pub fn team(&self, id: &str) -> Option<&Team> {
        self.by_id.get(id).map(|at| &self.teams[*at])
    }
}

/// What a team is at a moment: its plan and seats then, what it paid a month, and whether it is
/// still with us.
pub fn standing(team: &Team, at: i64) -> Standing {
    let last = team.changes.iter().take_while(|change| change.at <= at).last();
    match last {
        None => Standing {
            plan: Plan::Free,
            seats: 1,
            mrr: 0,
            status: TeamStatus::Active,
        },
        Some(last) if last.kind == ChangeKind::Churn => Standing {
            plan: last.plan,
            seats: 0,
            mrr: 0,
            status: TeamStatus::Churned,
        },
        Some(last) => {
            let dormant = last.plan == Plan::Free && team.dormant.is_some_and(|d| day_of(at) >= d);
            Standing {
                plan: last.plan,
                seats: last.seats,
                mrr: last.plan.price() * last.seats,
                status: if dormant {
                    TeamStatus::Dormant
                } else {
                    TeamStatus::Active
                },
            }
        }
    }
}

const CYCLE: i64 = 30 * DAY;

/// A day's invoices numbered in the order they were made: `INV-261108-014`.
fn number_of(day: i64, index: usize) -> String {
    let (y, m, d) = civil(day);
    format!("INV-{:02}{m:02}{d:02}-{:03}", y % 100, index + 1)
}

struct Bill {
    key: String,
    kind: BillKind,
    plan: Plan,
    seats: i64,
    amount: i64,
    at: i64,
}

/// What a team's changes bill up to `now`: a month of its plan every thirty days, a move to a
/// dearer plan at once, and seats added for what is left of the month.
fn bills_of(team: &Team, now: i64) -> Vec<Bill> {
    let mut bills = Vec::new();
    let mut mrr = 0;
    let mut anchor: Option<i64> = None;
    let mut plan = Plan::Free;
    let mut seats = 0;
    let renew = |bills: &mut Vec<Bill>, anchor: &mut Option<i64>, mrr: i64, plan: Plan, seats: i64, until: i64| {
        let Some(at) = anchor.as_mut() else { return };
        if mrr == 0 {
            return;
        }
        while *at + CYCLE <= until {
            *at += CYCLE;
            bills.push(Bill {
                key: format!("{}/r{}", team.id, *at),
                kind: BillKind::Renewal,
                plan,
                seats,
                amount: mrr,
                at: *at,
            });
        }
    };
    for change in &team.changes {
        renew(&mut bills, &mut anchor, mrr, plan, seats, change.at);
        let next = if change.kind == ChangeKind::Churn {
            0
        } else {
            change.plan.price() * change.seats
        };
        if change.kind == ChangeKind::Churn {
            anchor = None;
        } else if next > mrr {
            match anchor {
                Some(start) if mrr != 0 => {
                    let left = (1.0 / 30.0f64).max((start + CYCLE - change.at) as f64 / CYCLE as f64);
                    bills.push(Bill {
                        key: format!("{}/{}", team.id, change.id),
                        kind: if change.plan == plan {
                            BillKind::Seats
                        } else {
                            BillKind::Upgrade
                        },
                        plan: change.plan,
                        seats: change.seats,
                        amount: round((next - mrr) as f64 * left) as i64,
                        at: change.at,
                    });
                }
                _ => {
                    anchor = Some(change.at);
                    bills.push(Bill {
                        key: format!("{}/{}", team.id, change.id),
                        kind: BillKind::Upgrade,
                        plan: change.plan,
                        seats: change.seats,
                        amount: next,
                        at: change.at,
                    });
                }
            }
        }
        mrr = next;
        plan = change.plan;
        seats = change.seats;
    }
    renew(&mut bills, &mut anchor, mrr, plan, seats, now);
    bills
}

/// How an invoice stands before anyone touches it: mostly paid, a few failed or refunded, and some
/// of today's renewals still open.
fn status_of(bill: &Bill, today: i64) -> InvoiceStatus {
    let roll = first(hash(&[bill.at, bill.amount, bill.key.encode_utf16().count() as i64]));
    if bill.kind != BillKind::Renewal {
        return InvoiceStatus::Paid;
    }
    if day_of(bill.at) >= today && roll < 0.25 {
        InvoiceStatus::Open
    } else if roll < 0.018 {
        InvoiceStatus::Failed
    } else if roll < 0.028 {
        InvoiceStatus::Refunded
    } else {
        InvoiceStatus::Paid
    }
}

fn firsts_of(changes: &[Change]) -> Firsts {
    let mut firsts = Firsts::default();
    let mut seats = 0;
    for change in changes {
        if firsts.paid.is_none() && change.plan != Plan::Free && change.kind != ChangeKind::Churn {
            firsts.paid = Some(change.at);
        }
        if firsts.business.is_none() && change.plan == Plan::Business && change.kind != ChangeKind::Churn {
            firsts.business = Some(change.at);
        }
        if firsts.seats.is_none() && change.kind == ChangeKind::Seats && change.seats > seats {
            firsts.seats = Some(change.at);
        }
        seats = change.seats;
    }
    firsts
}

/// The seeded history, made once at the first build and kept, its changes in time order.
pub struct Base {
    history: History,
}

impl Base {
    pub fn new(now: i64) -> Self {
        let mut history = history(now);
        history.changes.sort_by_key(|change| change.at);
        Self { history }
    }
}

/// What the backend keeps after the history: teams that signed up while the app was open, changes
/// the stream and people made, invoices people changed, and teams handed to someone else.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Kept {
    pub teams: Vec<TeamRecord>,
    pub changes: Vec<Change>,
    pub invoice_changes: Vec<InvoiceChange>,
    pub team_edits: Vec<TeamEdit>,
}

/// The world at `now`, from the history and what the backend keeps.
pub fn build(base: &Base, kept: &Kept, now: i64) -> World {
    let mut names = base.history.names.clone();
    for record in &kept.teams {
        names.keep(&record.name);
    }
    let mut by_team: HashMap<&str, Vec<Change>> = HashMap::new();
    for change in &base.history.changes {
        by_team.entry(change.team.as_str()).or_default().push(change.clone());
    }
    let mut live = kept.changes.clone();
    live.sort_by_key(|change| change.at);
    for change in &live {
        by_team.entry(change.team.as_str()).or_default().push(change.clone());
    }
    let owners: HashMap<&str, _> = kept
        .team_edits
        .iter()
        .map(|edit| (edit.id.as_str(), edit.owner))
        .collect();
    let mut records: Vec<&TeamRecord> = kept.teams.iter().collect();
    records.sort_by_key(|record| record.created);
    let mut teams = Vec::with_capacity(base.history.teams.len() + records.len());
    for record in base.history.teams.iter().chain(records) {
        let changes = by_team.remove(record.id.as_str()).unwrap_or_default();
        let mut record = record.clone();
        if let Some(owner) = owners.get(record.id.as_str()) {
            record.owner = *owner;
        }
        teams.push(Team {
            firsts: firsts_of(&changes),
            record,
            changes,
        });
    }
    let by_id: HashMap<String, usize> = teams
        .iter()
        .enumerate()
        .map(|(at, team)| (team.id.clone(), at))
        .collect();

    let today = day_of(now);
    let status: HashMap<&str, InvoiceStatus> = kept
        .invoice_changes
        .iter()
        .map(|one| (one.id.as_str(), one.status))
        .collect();
    let mut bills: Vec<(Bill, usize)> = Vec::new();
    for (at, team) in teams.iter().enumerate() {
        if team.changes.len() < 2 {
            continue;
        }
        bills.extend(bills_of(team, now).into_iter().map(|bill| (bill, at)));
    }
    bills.sort_by_key(|(bill, _)| bill.at);
    let mut invoices = Vec::with_capacity(bills.len());
    let mut invoices_of: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut day = i64::MIN;
    let mut index = 0;
    for (bill, at) in bills {
        let on = day_of(bill.at);
        index = if on == day { index + 1 } else { 0 };
        day = on;
        let id = number_of(on, index);
        let state = status
            .get(bill.key.as_str())
            .copied()
            .unwrap_or_else(|| status_of(&bill, today));
        let invoice = Invoice {
            status: state,
            id,
            key: bill.key,
            team: at,
            kind: bill.kind,
            plan: bill.plan,
            seats: bill.seats,
            amount: bill.amount,
            at: bill.at,
            day: on,
            region: teams[at].region,
        };
        invoices_of.entry(at).or_default().push(invoices.len());
        invoices.push(invoice);
    }

    let changes = &base.history.changes;
    let mut merged: Vec<Change> = changes[changes.len().saturating_sub(200)..].to_vec();
    merged.extend(live);
    merged.sort_by_key(|change| std::cmp::Reverse(change.at));
    merged.truncate(80);
    World {
        now,
        teams,
        by_id,
        invoices,
        invoices_of,
        latest: merged,
        live_count: kept.changes.len(),
        names,
    }
}
