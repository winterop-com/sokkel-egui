//! The figures every panel draws, as the web's `metrics.ts` works them out from the world for a
//! span and a segment: revenue, MRR, teams, conversion and churn, as levels at moments and as flows
//! over a stretch of time; funnels; revenue by country and region; the top customers; the MRR
//! movements; retention by cohort; and a team's usage.

use std::collections::{BTreeMap, HashMap};

use crate::days::{DAY, HOUR, civil, day_from, day_of, month_name, short_day, start_of, weekday};
use crate::model::{COUNTRIES, ChangeKind, Invoice, InvoiceStatus, PAID, Plan, REGIONS, Region, Step, Team};
use crate::seed::{first, hash, round};
use crate::view::{Bucket, Span};
use crate::world::World;

/// Which teams the figures count: a plan and a region, or every one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Segment {
    pub plan: Option<Plan>,
    pub region: Option<Region>,
}

pub const EVERYONE: Segment = Segment {
    plan: None,
    region: None,
};

/// What a team is between two changes, as the figures count it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct State {
    plan: Plan,
    mrr: i64,
    active: i64,
    paying: i64,
}

const NONE: State = State {
    plan: Plan::Free,
    mrr: 0,
    active: 0,
    paying: 0,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntryKind {
    Change(ChangeKind),
    Dormant,
}

/// One change to a team, with what it was before and after.
struct Entry {
    at: i64,
    team: usize,
    kind: EntryKind,
    before: State,
    after: State,
    /// Whether the team paid for the first time with it.
    first_paid: bool,
}

/// Every change of every team, oldest first, each with the state before and after it.
pub struct Ledger {
    entries: Vec<Entry>,
}

impl Ledger {
    pub fn of(world: &World) -> Self {
        let mut entries = Vec::new();
        for (at_team, team) in world.teams.iter().enumerate() {
            let mut state = NONE;
            let mut paid = false;
            let mut last = 0;
            for change in &team.changes {
                let after = if change.kind == ChangeKind::Churn {
                    State {
                        plan: change.plan,
                        mrr: 0,
                        active: 0,
                        paying: 0,
                    }
                } else {
                    State {
                        plan: change.plan,
                        mrr: change.plan.price() * change.seats,
                        active: 1,
                        paying: i64::from(change.plan != Plan::Free),
                    }
                };
                let first_paid = !paid && after.paying == 1;
                if after.paying == 1 {
                    paid = true;
                }
                entries.push(Entry {
                    at: change.at,
                    team: at_team,
                    kind: EntryKind::Change(change.kind),
                    before: state,
                    after,
                    first_paid,
                });
                state = after;
                last = change.at;
            }
            if let Some(dormant) = team.dormant
                && state.plan == Plan::Free
                && state.active == 1
            {
                let at = start_of(dormant);
                if at > last && at <= world.now {
                    entries.push(Entry {
                        at,
                        team: at_team,
                        kind: EntryKind::Dormant,
                        before: state,
                        after: State { active: 0, ..state },
                        first_paid: false,
                    });
                }
            }
        }
        entries.sort_by_key(|entry| entry.at);
        Self { entries }
    }

    /// The first entry at or after a moment.
    fn first_at(&self, at: i64) -> usize {
        self.entries.partition_point(|entry| entry.at < at)
    }
}

/// The world with its ledger, which every figure reads.
pub struct Figures<'a> {
    pub world: &'a World,
    ledger: Ledger,
}

fn in_region(region: Region, segment: Segment) -> bool {
    segment.region.is_none_or(|r| r == region)
}

fn counts(state: &State, segment: Segment) -> bool {
    segment.plan.is_none_or(|p| p == state.plan)
}

fn share(part: f64, whole: f64) -> f64 {
    if whole == 0.0 { 0.0 } else { part / whole * 100.0 }
}

/// MRR, paying teams and active teams at moments.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Levels {
    pub mrr: Vec<i64>,
    pub paying: Vec<i64>,
    pub active: Vec<i64>,
}

/// What happened over a stretch of time.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Flows {
    pub signups: i64,
    /// Teams that paid for the first time.
    pub converted: i64,
    /// Paying teams that left.
    pub churned: i64,
    /// MRR from teams that started paying, from leaving, and from teams that pay more or less.
    pub new_mrr: i64,
    pub expansion: i64,
    pub contraction: i64,
    pub churn_mrr: i64,
}

/// The key figures of a span, each with its trend over the span's buckets.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Kpis {
    pub revenue: i64,
    pub mrr: i64,
    pub active: i64,
    pub paying: i64,
    pub signups: i64,
    /// Teams that paid for the first time, as a share of the teams that signed up, in percent.
    pub conversion: f64,
    /// Paying teams that left, as a share of those paying at the start, in percent.
    pub churn: f64,
    pub trend: Trend,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Trend {
    pub revenue: Vec<f64>,
    pub mrr: Vec<f64>,
    pub active: Vec<f64>,
    pub signups: Vec<f64>,
    pub conversion: Vec<f64>,
    pub churn: Vec<f64>,
}

/// One stage of a funnel: its step, how many reached it, and its share of the first and of the
/// stage before.
#[derive(Clone, Debug, PartialEq)]
pub struct Stage {
    pub step: Step,
    pub name: &'static str,
    pub count: i64,
    pub of_first: f64,
    pub of_previous: f64,
}

/// The steps of the sign-up funnel on the overview.
pub const SIGNUP_FUNNEL: [Step; 5] = [
    Step::Visited,
    Step::SignedUp,
    Step::Project,
    Step::Invited,
    Step::Upgraded,
];

/// One of the teams paying most.
#[derive(Clone, Debug, PartialEq)]
pub struct TopCustomer {
    pub team: usize,
    pub plan: Plan,
    pub seats: i64,
    pub mrr: i64,
    /// What it paid a month at the span's start.
    pub before: i64,
}

/// Each country's point, with its revenue now and before, for the map.
#[derive(Clone, Debug, PartialEq)]
pub struct CountryFigure {
    pub name: &'static str,
    pub region: Region,
    pub at: (f64, f64),
    pub revenue: i64,
    pub before: i64,
}

/// One cell of the retention grid: a cohort, the months since it first paid, and the share still
/// paying.
#[derive(Clone, Debug, PartialEq)]
pub struct CohortCell {
    pub cohort: String,
    pub month: String,
    pub retained: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cohorts {
    pub cells: Vec<CohortCell>,
    pub order: Vec<String>,
    pub months: Vec<String>,
}

/// A team's members at work each day of a span, and its seats.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Usage {
    pub labels: Vec<String>,
    pub members: Vec<Option<i64>>,
    pub seats: Vec<Option<i64>>,
}

/// A bucket that has not begun yet draws no figure.
fn later(bucket: &Bucket, now: i64) -> bool {
    bucket.from > now
}

/// How many people saw the sign-up page for every team that signed up, on a day in a region.
fn visits_per_signup(day: i64, region: usize) -> f64 {
    2.5 + 1.5 * first(hash(&[day, region as i64, 17]))
}

/// The moment a team took a funnel step, or none for a step it has not taken.
fn step_at(team: &Team, step: Step, plan: Option<Plan>) -> Option<i64> {
    match step {
        Step::Visited | Step::SignedUp => Some(team.created),
        Step::Project => team.project.map(|p| team.created.max(start_of(p) + 12 * HOUR)),
        Step::Invited => team.invited.map(|i| team.created.max(start_of(i) + 14 * HOUR)),
        Step::Upgraded => match plan {
            Some(Plan::Business) => team.firsts.business,
            Some(Plan::Team) => team
                .changes
                .iter()
                .find(|c| c.plan == Plan::Team && c.kind != ChangeKind::Churn)
                .map(|c| c.at),
            _ => team.firsts.paid,
        },
        Step::Seats => team.firsts.seats,
        Step::Business => team.firsts.business,
    }
}

impl<'a> Figures<'a> {
    pub fn new(world: &'a World) -> Self {
        Self {
            ledger: Ledger::of(world),
            world,
        }
    }

    /// MRR, paying teams and active teams at each of the moments, which come in order.
    pub fn levels(&self, segment: Segment, moments: &[i64]) -> Levels {
        let entries = &self.ledger.entries;
        let mut out = Levels::default();
        let (mut mrr, mut paying, mut active) = (0, 0, 0);
        let mut index = 0;
        for moment in moments {
            while index < entries.len() && entries[index].at <= *moment {
                let entry = &entries[index];
                index += 1;
                if !in_region(self.world.teams[entry.team].region, segment) {
                    continue;
                }
                let was = counts(&entry.before, segment);
                let now = counts(&entry.after, segment);
                let pick = |cond: bool, value: i64| if cond { value } else { 0 };
                mrr += pick(now, entry.after.mrr) - pick(was, entry.before.mrr);
                paying += pick(now, entry.after.paying) - pick(was, entry.before.paying);
                active += pick(now, entry.after.active) - pick(was, entry.before.active);
            }
            out.mrr.push(mrr);
            out.paying.push(paying);
            out.active.push(active);
        }
        out
    }

    pub fn flows(&self, segment: Segment, from: i64, to: i64) -> Flows {
        let entries = &self.ledger.entries;
        let mut out = Flows::default();
        for entry in &entries[self.ledger.first_at(from)..] {
            if entry.at >= to {
                break;
            }
            if !in_region(self.world.teams[entry.team].region, segment) {
                continue;
            }
            if entry.kind == EntryKind::Change(ChangeKind::Signup) {
                out.signups += 1;
                continue;
            }
            let was = if counts(&entry.before, segment) {
                entry.before.mrr
            } else {
                0
            };
            let now = if counts(&entry.after, segment) {
                entry.after.mrr
            } else {
                0
            };
            if entry.first_paid && counts(&entry.after, segment) {
                out.converted += 1;
            }
            if entry.kind == EntryKind::Change(ChangeKind::Churn) && was > 0 {
                out.churned += 1;
                out.churn_mrr += was;
            } else if was == 0 && now > 0 {
                out.new_mrr += now;
            } else if now > was {
                out.expansion += now - was;
            } else if now < was && now > 0 {
                out.contraction += was - now;
            } else if now == 0 && was > 0 {
                out.churn_mrr += was;
            }
        }
        out
    }

    /// The paid invoices of a segment between two moments.
    fn paid_in(&self, segment: Segment, from: i64, to: i64, mut each: impl FnMut(&Invoice)) {
        let list = &self.world.invoices;
        let low = list.partition_point(|invoice| invoice.at < from);
        for invoice in &list[low..] {
            if invoice.at >= to {
                break;
            }
            if invoice.status != InvoiceStatus::Paid
                || !in_region(invoice.region, segment)
                || segment.plan.is_some_and(|p| p != invoice.plan)
            {
                continue;
            }
            each(invoice);
        }
    }

    pub fn revenue(&self, segment: Segment, from: i64, to: i64) -> i64 {
        let mut sum = 0;
        self.paid_in(segment, from, to, |invoice| sum += invoice.amount);
        sum
    }

    /// Refunds between two moments: how many, and how much.
    pub fn refunds(&self, segment: Segment, from: i64, to: i64) -> (i64, i64) {
        let mut count = 0;
        let mut amount = 0;
        for invoice in &self.world.invoices {
            if invoice.at < from || invoice.at >= to || invoice.status != InvoiceStatus::Refunded {
                continue;
            }
            if !in_region(invoice.region, segment) || segment.plan.is_some_and(|p| p != invoice.plan) {
                continue;
            }
            count += 1;
            amount += invoice.amount;
        }
        (count, amount)
    }

    /// Revenue in each bucket for each paid plan: Team's, then Business's.
    pub fn revenue_by_plan(&self, segment: Segment, span: &Span) -> (Vec<Option<i64>>, Vec<Option<i64>>) {
        let now = self.world.now;
        let mut team = Vec::new();
        let mut business = Vec::new();
        for bucket in &span.buckets {
            if later(bucket, now) {
                team.push(None);
                business.push(None);
                continue;
            }
            let (mut on_team, mut on_business) = (0, 0);
            self.paid_in(segment, bucket.from, bucket.to.min(now + 1), |invoice| {
                if invoice.plan == Plan::Business {
                    on_business += invoice.amount;
                } else {
                    on_team += invoice.amount;
                }
            });
            team.push(Some(on_team));
            business.push(Some(on_business));
        }
        (team, business)
    }

    /// Revenue in each bucket, the last one up to the span's end.
    pub fn revenue_series(&self, segment: Segment, span: &Span) -> Vec<Option<i64>> {
        span.buckets
            .iter()
            .map(|bucket| {
                (!later(bucket, self.world.now)).then(|| self.revenue(segment, bucket.from, bucket.to.min(span.to)))
            })
            .collect()
    }

    pub fn kpis(&self, segment: Segment, span: &Span) -> Kpis {
        let shown: Vec<&Bucket> = span.buckets.iter().filter(|b| !later(b, self.world.now)).collect();
        let ends: Vec<i64> = shown.iter().map(|b| b.to.min(span.to)).collect();
        let mut moments = vec![span.from];
        moments.extend(&ends);
        moments.push(span.to);
        let level = self.levels(segment, &moments);
        let start_paying = level.paying[0];
        let flow = self.flows(segment, span.from, span.to);
        let per_bucket: Vec<Flows> = shown
            .iter()
            .map(|b| self.flows(segment, b.from, b.to.min(span.to)))
            .collect();
        let inner = |values: &[i64]| values[1..values.len() - 1].iter().map(|v| *v as f64).collect();
        Kpis {
            revenue: self.revenue(segment, span.from, span.to),
            mrr: *level.mrr.last().unwrap_or(&0),
            active: *level.active.last().unwrap_or(&0),
            paying: *level.paying.last().unwrap_or(&0),
            signups: flow.signups,
            conversion: share(flow.converted as f64, flow.signups as f64),
            churn: share(flow.churned as f64, start_paying as f64),
            trend: Trend {
                revenue: shown
                    .iter()
                    .map(|b| self.revenue(segment, b.from, b.to.min(span.to)) as f64)
                    .collect(),
                mrr: inner(&level.mrr),
                active: inner(&level.active),
                signups: per_bucket.iter().map(|f| f.signups as f64).collect(),
                conversion: per_bucket
                    .iter()
                    .map(|f| share(f.converted as f64, f.signups as f64))
                    .collect(),
                churn: per_bucket
                    .iter()
                    .enumerate()
                    .map(|(i, f)| share(f.churned as f64, level.paying.get(i).copied().unwrap_or(0) as f64))
                    .collect(),
            },
        }
    }

    /// A funnel over the teams that signed up in a stretch of time: each step counts the teams that
    /// took it after the step before, within `window` days of signing up. Visiting the site, as a
    /// first step, counts the visitors of those days.
    pub fn funnel(&self, segment: Segment, from: i64, to: i64, steps: &[Step], window: i64) -> Vec<Stage> {
        let world = self.world;
        let cohort: Vec<&Team> = world
            .teams
            .iter()
            .filter(|team| team.created >= from && team.created < to && in_region(team.region, segment))
            .collect();
        let mut reach = vec![0i64; steps.len()];
        for team in &cohort {
            let mut previous = team.created;
            let limit = team.created + window * DAY;
            for (index, step) in steps.iter().enumerate() {
                if *step == Step::Visited {
                    if index == 0 {
                        continue;
                    }
                    break;
                }
                match step_at(team, *step, segment.plan) {
                    Some(at) if at >= previous && at <= limit && at <= world.now => {
                        reach[index] += 1;
                        previous = at;
                    }
                    _ => break,
                }
            }
        }
        if steps.first() == Some(&Step::Visited) {
            // in the order the web's map meets its keys: each day and region as first signed up
            let mut signed: Vec<((i64, usize), i64)> = Vec::new();
            let mut seen: HashMap<(i64, usize), usize> = HashMap::new();
            for team in &cohort {
                let key = (day_of(team.created), team.region.index());
                match seen.get(&key) {
                    Some(at) => signed[*at].1 += 1,
                    None => {
                        seen.insert(key, signed.len());
                        signed.push((key, 1));
                    }
                }
            }
            let mut visits = 0.0;
            for ((day, region), count) in signed {
                visits += round(count as f64 * visits_per_signup(day, region));
            }
            reach[0] = visits as i64;
        }
        let first = reach.first().copied().unwrap_or(0) as f64;
        steps
            .iter()
            .enumerate()
            .map(|(index, step)| {
                let count = reach[index];
                let before = if index == 0 { count } else { reach[index - 1] };
                Stage {
                    step: *step,
                    name: step.name(),
                    count,
                    of_first: share(count as f64, first),
                    of_previous: share(count as f64, before as f64),
                }
            })
            .collect()
    }

    /// Paid revenue by country between two moments.
    pub fn revenue_by_country(&self, segment: Segment, from: i64, to: i64) -> HashMap<&'a str, i64> {
        let mut totals = HashMap::new();
        let teams = &self.world.teams;
        self.paid_in(segment, from, to, |invoice| {
            *totals.entry(teams[invoice.team].country.as_str()).or_insert(0) += invoice.amount;
        });
        totals
    }

    /// Paid revenue by region between two moments, in the order of `REGIONS`.
    pub fn revenue_by_region(&self, segment: Segment, from: i64, to: i64) -> BTreeMap<Region, i64> {
        let mut totals: BTreeMap<Region, i64> = REGIONS.iter().map(|r| (*r, 0)).collect();
        self.paid_in(segment, from, to, |invoice| {
            *totals.entry(invoice.region).or_insert(0) += invoice.amount;
        });
        totals
    }

    /// The teams paying most at the span's end, with what they paid at its start.
    pub fn top_customers(&self, segment: Segment, span: &Span, count: usize) -> Vec<TopCustomer> {
        let mut found = Vec::new();
        for (at, team) in self.world.teams.iter().enumerate() {
            if !in_region(team.region, segment) || team.firsts.paid.is_none_or(|p| p > span.to) {
                continue;
            }
            let mut now: Option<(Plan, i64, i64)> = None;
            let mut before = 0;
            for change in &team.changes {
                let churned = change.kind == ChangeKind::Churn;
                if change.at <= span.from {
                    before = if churned { 0 } else { change.plan.price() * change.seats };
                }
                if change.at <= span.to {
                    now = (!churned).then(|| (change.plan, change.seats, change.plan.price() * change.seats));
                }
            }
            let Some((plan, seats, mrr)) = now else { continue };
            if mrr == 0 || segment.plan.is_some_and(|p| p != plan) {
                continue;
            }
            found.push(TopCustomer {
                team: at,
                plan,
                seats,
                mrr,
                before,
            });
        }
        found.sort_by_key(|one| std::cmp::Reverse(one.mrr));
        found.truncate(count);
        found
    }

    pub fn countries(&self, segment: Segment, span: &Span, previous: &Span) -> Vec<CountryFigure> {
        let now = self.revenue_by_country(segment, span.from, span.to);
        let then = self.revenue_by_country(segment, previous.from, previous.to);
        COUNTRIES
            .iter()
            .filter(|c| in_region(c.region, segment))
            .map(|c| CountryFigure {
                name: c.name,
                region: c.region,
                at: c.at,
                revenue: now.get(c.name).copied().unwrap_or(0),
                before: then.get(c.name).copied().unwrap_or(0),
            })
            .collect()
    }

    /// Retention by cohort: the teams that first paid in each of the last twelve months, and the
    /// share of them still paying each month after. A month that has not come yet has no cell.
    pub fn cohorts(&self, segment: Segment, last: i64) -> Cohorts {
        let world = self.world;
        let (year, month, _) = civil(last);
        let months: Vec<String> = (0..12).map(|i| format!("Month {i}")).collect();
        let mut out = Cohorts {
            months: months.clone(),
            ..Default::default()
        };
        for back in (0..=11).rev() {
            let first = day_from(year, month - back, 1);
            let next = day_from(year, month - back + 1, 1);
            let name = month_name(first);
            out.order.push(name.clone());
            let members: Vec<&Team> = world
                .teams
                .iter()
                .filter(|team| {
                    let Some(paid) = team.firsts.paid else { return false };
                    if !in_region(team.region, segment) {
                        return false;
                    }
                    let day = day_of(paid);
                    if day < first || day >= next {
                        return false;
                    }
                    match segment.plan {
                        None => true,
                        Some(plan) => team.changes.iter().find(|c| c.at == paid).map(|c| c.plan) == Some(plan),
                    }
                })
                .collect();
            if members.is_empty() {
                continue;
            }
            for (index, label) in months.iter().enumerate() {
                let mut counted = 0;
                let mut still = 0;
                for team in &members {
                    let moment = team.firsts.paid.unwrap_or(0) + index as i64 * 30 * DAY;
                    if moment > world.now {
                        continue;
                    }
                    counted += 1;
                    let mut paying = false;
                    for change in &team.changes {
                        if change.at > moment {
                            break;
                        }
                        paying = change.kind != ChangeKind::Churn && change.plan != Plan::Free;
                    }
                    if paying {
                        still += 1;
                    }
                }
                if counted == 0 {
                    break;
                }
                out.cells.push(CohortCell {
                    cohort: name.clone(),
                    month: label.clone(),
                    retained: round(f64::from(still) / f64::from(counted) * 1000.0) / 10.0,
                });
            }
        }
        out
    }

    /// Every paid plan's MRR at a moment; Free's is none.
    pub fn mrr_by_plan(&self, segment: Segment, at: i64) -> BTreeMap<Plan, i64> {
        let mut out: BTreeMap<Plan, i64> = BTreeMap::from([(Plan::Free, 0)]);
        for plan in PAID {
            let level = self.levels(
                Segment {
                    plan: Some(plan),
                    ..segment
                },
                &[at],
            );
            out.insert(plan, level.mrr[0]);
        }
        out
    }
}

/// A team's members at work each day of a span: most of its seats on a weekday, fewer at weekends,
/// none once it has left or gone quiet.
pub fn usage(team: &Team, span: &Span, now: i64) -> Usage {
    let mut out = Usage::default();
    for day in span.first..=span.last {
        out.labels.push(short_day(day));
        let at = (start_of(day + 1) - 1).min(now);
        if start_of(day) > now || at < team.created {
            out.members.push(None);
            out.seats.push(None);
            continue;
        }
        let mut held = 0;
        let mut gone = false;
        for change in &team.changes {
            if change.at > at {
                break;
            }
            held = change.seats;
            gone = change.kind == ChangeKind::Churn;
        }
        let quiet = team.dormant.is_some_and(|d| day >= d);
        let weekend = matches!(weekday(day), 0 | 6);
        let busy = if weekend { 0.25 } else { 0.62 } + 0.25 * first(hash(&[day, team.created % 9973]));
        out.members.push(Some(if gone || quiet {
            0
        } else {
            i64::from(held > 0).max(round(held as f64 * busy) as i64)
        }));
        out.seats.push(Some(if gone { 0 } else { held }));
    }
    out
}
