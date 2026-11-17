//! What Tidevann's analytics are made of, as the web's `model.ts`: its plans and prices, the world
//! it sells in, its team, and the shapes the backend keeps.

use serde::{Deserialize, Serialize};
use sokkel_egui::PanelSpan;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Person {
    Ada,
    Grace,
    Linus,
    Margaret,
}

pub const PEOPLE: [Person; 4] = [Person::Ada, Person::Grace, Person::Linus, Person::Margaret];

impl Person {
    pub fn name(self) -> &'static str {
        match self {
            Person::Ada => "ada",
            Person::Grace => "grace",
            Person::Linus => "linus",
            Person::Margaret => "margaret",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Plan {
    Free,
    Team,
    Business,
}

pub const PLANS: [Plan; 3] = [Plan::Free, Plan::Team, Plan::Business];
pub const PAID: [Plan; 2] = [Plan::Team, Plan::Business];

impl Plan {
    pub fn name(self) -> &'static str {
        match self {
            Plan::Free => "Free",
            Plan::Team => "Team",
            Plan::Business => "Business",
        }
    }

    /// As the address writes it.
    pub fn id(self) -> &'static str {
        match self {
            Plan::Free => "free",
            Plan::Team => "team",
            Plan::Business => "business",
        }
    }

    /// Dollars a seat costs each month.
    pub fn price(self) -> i64 {
        match self {
            Plan::Free => 0,
            Plan::Team => 12,
            Plan::Business => 30,
        }
    }

    /// The chart color token the plan is drawn in.
    pub fn color(self) -> &'static str {
        match self {
            Plan::Free => "chart-3",
            Plan::Team => "chart-2",
            Plan::Business => "chart-4",
        }
    }
}

/// The color token the period before is drawn in, beside the period itself.
pub const BEFORE_COLOR: &str = "muted-foreground";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Region {
    Europe,
    NorthAmerica,
    SouthAmerica,
    AsiaPacific,
    MiddleEastAfrica,
}

pub const REGIONS: [Region; 5] = [
    Region::Europe,
    Region::NorthAmerica,
    Region::SouthAmerica,
    Region::AsiaPacific,
    Region::MiddleEastAfrica,
];

impl Region {
    pub fn name(self) -> &'static str {
        match self {
            Region::Europe => "Europe",
            Region::NorthAmerica => "North America",
            Region::SouthAmerica => "South America",
            Region::AsiaPacific => "Asia Pacific",
            Region::MiddleEastAfrica => "Middle East and Africa",
        }
    }

    /// As the address writes it.
    pub fn id(self) -> &'static str {
        match self {
            Region::Europe => "europe",
            Region::NorthAmerica => "north-america",
            Region::SouthAmerica => "south-america",
            Region::AsiaPacific => "asia-pacific",
            Region::MiddleEastAfrica => "middle-east-africa",
        }
    }

    /// Who looks after the paying teams of the region.
    pub fn owner(self) -> Person {
        match self {
            Region::Europe => Person::Ada,
            Region::NorthAmerica => Person::Grace,
            Region::SouthAmerica => Person::Margaret,
            Region::AsiaPacific => Person::Linus,
            Region::MiddleEastAfrica => Person::Margaret,
        }
    }

    /// Its place in `REGIONS`.
    pub fn index(self) -> usize {
        REGIONS.iter().position(|r| *r == self).unwrap_or(0)
    }
}

pub struct Country {
    pub name: &'static str,
    pub region: Region,
    /// Where its point stands on the map: longitude, latitude.
    pub at: (f64, f64),
    /// Its share of its region's sign-ups.
    pub weight: f64,
}

const fn country(name: &'static str, region: Region, lon: f64, lat: f64, weight: f64) -> Country {
    Country {
        name,
        region,
        at: (lon, lat),
        weight,
    }
}

use Region::{AsiaPacific as Ap, Europe as Eu, MiddleEastAfrica as Mea, NorthAmerica as Na, SouthAmerica as Sa};

pub const COUNTRIES: [Country; 30] = [
    country("United Kingdom", Eu, -2.0, 53.5, 16.0),
    country("Germany", Eu, 10.4, 51.2, 18.0),
    country("France", Eu, 2.5, 46.6, 12.0),
    country("Netherlands", Eu, 5.3, 52.1, 8.0),
    country("Norway", Eu, 9.0, 61.0, 7.0),
    country("Sweden", Eu, 16.0, 62.5, 8.0),
    country("Denmark", Eu, 9.5, 56.0, 5.0),
    country("Finland", Eu, 26.0, 63.5, 4.0),
    country("Spain", Eu, -3.7, 40.2, 8.0),
    country("Italy", Eu, 12.5, 42.8, 7.0),
    country("Poland", Eu, 19.4, 52.0, 5.0),
    country("Ireland", Eu, -8.0, 53.2, 2.0),
    country("United States", Na, -98.5, 39.8, 74.0),
    country("Canada", Na, -106.0, 56.0, 16.0),
    country("Mexico", Na, -102.5, 23.6, 10.0),
    country("Brazil", Sa, -51.9, -10.8, 48.0),
    country("Argentina", Sa, -64.0, -34.0, 20.0),
    country("Chile", Sa, -71.2, -35.7, 14.0),
    country("Colombia", Sa, -74.3, 4.6, 18.0),
    country("Japan", Ap, 138.3, 36.2, 22.0),
    country("Australia", Ap, 134.5, -25.7, 24.0),
    country("India", Ap, 78.9, 21.0, 22.0),
    country("Singapore", Ap, 103.8, 1.35, 10.0),
    country("South Korea", Ap, 127.8, 36.5, 12.0),
    country("New Zealand", Ap, 172.5, -41.5, 10.0),
    country("South Africa", Mea, 24.0, -29.0, 30.0),
    country("Nigeria", Mea, 8.7, 9.1, 18.0),
    country("Kenya", Mea, 37.9, 0.2, 16.0),
    country("Egypt", Mea, 30.8, 26.8, 14.0),
    country("United Arab Emirates", Mea, 54.0, 24.0, 22.0),
];

/// What a change to a team's plan was.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChangeKind {
    Signup,
    Upgrade,
    Downgrade,
    Seats,
    Churn,
}

/// One change to a team's plan: its sign-up, a move between plans, seats bought or let go, or its
/// leaving.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Change {
    pub id: String,
    pub team: String,
    pub kind: ChangeKind,
    /// The moment, in milliseconds.
    pub at: i64,
    /// The plan after the change; a team that left keeps the plan it left.
    pub plan: Plan,
    /// Seats after the change; none once the team has left.
    pub seats: i64,
    /// Who made it from the app, or none for the team itself.
    pub by: Option<Person>,
    /// Whether the live stream made it, rather than the history or a person.
    pub live: bool,
}

/// A team as the backend keeps one: who it is and the steps it took.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TeamRecord {
    pub id: String,
    pub name: String,
    pub country: String,
    pub region: Region,
    pub owner: Person,
    /// When it signed up, in milliseconds.
    pub created: i64,
    /// The day it made its first project.
    pub project: Option<i64>,
    /// The day it first invited a teammate.
    pub invited: Option<i64>,
    /// The day a free team stopped using it.
    pub dormant: Option<i64>,
    pub live: bool,
}

/// The moments a team first paid, first bought more seats and first moved to Business.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Firsts {
    pub paid: Option<i64>,
    pub seats: Option<i64>,
    pub business: Option<i64>,
}

/// A team with its changes in order, and when it first took the steps a funnel counts.
#[derive(Clone, Debug, PartialEq)]
pub struct Team {
    pub record: TeamRecord,
    pub changes: Vec<Change>,
    pub firsts: Firsts,
}

impl std::ops::Deref for Team {
    type Target = TeamRecord;
    fn deref(&self) -> &TeamRecord {
        &self.record
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeamStatus {
    Active,
    Dormant,
    Churned,
}

/// What a team is now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Standing {
    pub plan: Plan,
    pub seats: i64,
    /// Dollars a month.
    pub mrr: i64,
    pub status: TeamStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InvoiceStatus {
    Paid,
    Open,
    Failed,
    Refunded,
}

impl InvoiceStatus {
    pub fn id(self) -> &'static str {
        match self {
            InvoiceStatus::Paid => "paid",
            InvoiceStatus::Open => "open",
            InvoiceStatus::Failed => "failed",
            InvoiceStatus::Refunded => "refunded",
        }
    }
}

/// What an invoice bills: a month of the plan, a move to a plan, or seats added.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BillKind {
    Renewal,
    Upgrade,
    Seats,
}

impl BillKind {
    pub fn id(self) -> &'static str {
        match self {
            BillKind::Renewal => "renewal",
            BillKind::Upgrade => "upgrade",
            BillKind::Seats => "seats",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Invoice {
    /// Its number, given in the order the day's invoices were made.
    pub id: String,
    /// What it bills, which never changes as other invoices come and go: a person's change to it
    /// is kept under this.
    pub key: String,
    /// The team's place in the world's teams.
    pub team: usize,
    pub kind: BillKind,
    pub plan: Plan,
    pub seats: i64,
    pub amount: i64,
    pub at: i64,
    pub day: i64,
    pub status: InvoiceStatus,
    pub region: Region,
}

/// A person's change to an invoice, kept under the invoice's `key`: refunded, marked paid, a charge tried again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InvoiceChange {
    pub id: String,
    pub status: InvoiceStatus,
    pub by: Person,
    pub at: i64,
}

/// A person's change to a team itself: who looks after it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TeamEdit {
    pub id: String,
    pub owner: Person,
}

/// The steps a funnel is built from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Step {
    Visited,
    SignedUp,
    Project,
    Invited,
    Upgraded,
    Seats,
    Business,
}

pub const STEPS: [Step; 7] = [
    Step::Visited,
    Step::SignedUp,
    Step::Project,
    Step::Invited,
    Step::Upgraded,
    Step::Seats,
    Step::Business,
];

impl Step {
    pub fn name(self) -> &'static str {
        match self {
            Step::Visited => "Saw the sign-up page",
            Step::SignedUp => "Signed up",
            Step::Project => "Made a project",
            Step::Invited => "Invited a teammate",
            Step::Upgraded => "Moved to a paid plan",
            Step::Seats => "Bought more seats",
            Step::Business => "Moved to Business",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Funnel {
    pub id: String,
    pub name: String,
    pub steps: Vec<Step>,
    /// Days after signing up a team has to take every step.
    pub window: i64,
    pub by: Person,
    pub updated: i64,
}

/// The panels a report is composed from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PanelId {
    Kpis,
    Revenue,
    Funnel,
    Countries,
    Customers,
    Movements,
    Regions,
    Cohorts,
}

pub const PANELS: [PanelId; 8] = [
    PanelId::Kpis,
    PanelId::Revenue,
    PanelId::Funnel,
    PanelId::Countries,
    PanelId::Customers,
    PanelId::Movements,
    PanelId::Regions,
    PanelId::Cohorts,
];

impl PanelId {
    pub fn name(self) -> &'static str {
        match self {
            PanelId::Kpis => "Key figures",
            PanelId::Revenue => "Revenue over time",
            PanelId::Funnel => "Sign-up funnel",
            PanelId::Countries => "Revenue by country",
            PanelId::Customers => "Top customers",
            PanelId::Movements => "MRR movements",
            PanelId::Regions => "Revenue by region",
            PanelId::Cohorts => "Retention by cohort",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub name: String,
    pub panels: Vec<PanelId>,
    pub by: Person,
    pub updated: i64,
}

/// The widgets a dashboard can be arranged from: each key figure as a tile, and the panels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WidgetId {
    Revenue,
    Mrr,
    Arr,
    Active,
    Paying,
    Arpa,
    Signups,
    Conversion,
    Churn,
    Refunded,
    RevenueChart,
    Plans,
    Funnel,
    Countries,
    Feed,
    Customers,
    Movements,
    Regions,
    Cohorts,
}

pub const WIDGETS: [WidgetId; 19] = [
    WidgetId::Revenue,
    WidgetId::Mrr,
    WidgetId::Arr,
    WidgetId::Active,
    WidgetId::Paying,
    WidgetId::Arpa,
    WidgetId::Signups,
    WidgetId::Conversion,
    WidgetId::Churn,
    WidgetId::Refunded,
    WidgetId::RevenueChart,
    WidgetId::Plans,
    WidgetId::Funnel,
    WidgetId::Countries,
    WidgetId::Feed,
    WidgetId::Customers,
    WidgetId::Movements,
    WidgetId::Regions,
    WidgetId::Cohorts,
];

/// A widget's place on an arranged dashboard: which it is and how wide.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Placed {
    pub id: WidgetId,
    #[serde(with = "crate::model::SpanDef")]
    pub span: PanelSpan,
}

/// A panel's span, as the library names it.
#[derive(Serialize, Deserialize)]
#[serde(remote = "PanelSpan")]
pub enum SpanDef {
    Sixth,
    Quarter,
    Third,
    Half,
    TwoThirds,
    Full,
}

/// The dashboards a person arranges.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BoardId {
    Overview,
    Revenue,
}

const fn placed(id: WidgetId, span: PanelSpan) -> Placed {
    Placed { id, span }
}

/// Each dashboard as it comes, and as Reset layout puts it back.
pub fn default_layout(board: BoardId) -> Vec<Placed> {
    use PanelSpan::*;
    use WidgetId as W;
    match board {
        BoardId::Overview => vec![
            placed(W::Revenue, Sixth),
            placed(W::Mrr, Sixth),
            placed(W::Active, Sixth),
            placed(W::Signups, Sixth),
            placed(W::Conversion, Sixth),
            placed(W::Churn, Sixth),
            placed(W::RevenueChart, TwoThirds),
            placed(W::Funnel, Third),
            placed(W::Countries, TwoThirds),
            placed(W::Feed, Third),
            placed(W::Customers, Half),
            placed(W::Regions, Half),
        ],
        BoardId::Revenue => vec![
            placed(W::Mrr, Sixth),
            placed(W::Arr, Sixth),
            placed(W::Paying, Sixth),
            placed(W::Arpa, Sixth),
            placed(W::Revenue, Sixth),
            placed(W::Refunded, Sixth),
            placed(W::RevenueChart, TwoThirds),
            placed(W::Plans, Third),
            placed(W::Movements, Half),
            placed(W::Regions, Half),
            placed(W::Cohorts, Full),
        ],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Metric {
    Mrr,
    Revenue,
    Paying,
    Active,
    Signups,
}

pub const METRICS: [Metric; 5] = [
    Metric::Mrr,
    Metric::Revenue,
    Metric::Paying,
    Metric::Active,
    Metric::Signups,
];

impl Metric {
    pub fn name(self) -> &'static str {
        match self {
            Metric::Mrr => "MRR",
            Metric::Revenue => "Revenue",
            Metric::Paying => "Paying teams",
            Metric::Active => "Active teams",
            Metric::Signups => "Sign-ups",
        }
    }

    /// Whether a goal's figure is a sum since the goal started (revenue) rather than a level now.
    pub fn sums(self) -> bool {
        matches!(self, Metric::Revenue | Metric::Signups)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub name: String,
    pub metric: Metric,
    pub target: f64,
    /// The day it counts from and the day it is due.
    pub start: i64,
    pub due: i64,
    pub plan: Option<Plan>,
    pub region: Option<Region>,
    pub by: Person,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    Usd,
    Eur,
    Nok,
}

impl Currency {
    pub fn code(self) -> &'static str {
        match self {
            Currency::Usd => "USD",
            Currency::Eur => "EUR",
            Currency::Nok => "NOK",
        }
    }

    /// What a dollar is in the currency.
    pub fn rate(self) -> f64 {
        match self {
            Currency::Usd => 1.0,
            Currency::Eur => 0.92,
            Currency::Nok => 10.8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pace {
    Fast,
    Slow,
    Off,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Prefs {
    /// How often the live stream sends an event.
    pub pace: Pace,
    /// The pace the stream resumes at after a pause.
    pub resume: Option<Pace>,
    pub currency: Currency,
    /// The range a dashboard opens on, by its preset's id.
    pub range: String,
    /// Whether a dashboard compares with the period before.
    pub compare: bool,
    /// A note in the corner for each order of at least this many dollars; 0 for none.
    pub big_order: i64,
    /// A note in the corner for each team that leaves.
    pub notify_churn: bool,
    /// The name the workspace goes by in the rail.
    pub workspace: String,
    /// When the live feed was last marked read; what came after is new.
    pub feed_seen: i64,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            pace: Pace::Fast,
            resume: None,
            currency: Currency::Usd,
            range: "30d".into(),
            compare: true,
            big_order: 500,
            notify_churn: false,
            workspace: "Tidevann".into(),
            feed_seen: 0,
        }
    }
}
