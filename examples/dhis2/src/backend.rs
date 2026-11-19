//! The simulated server, as the web's `data/backend.ts` keeps it on the shared fake backend
//! (`tidevann-shared/backend`): its indicators, validation rules and program rules as collections
//! the pages edit, and the analytics that run an expression for an organization unit and a month.
//!
//! Each collection is the web's `collection`: a change is made and kept at once, its answer comes
//! `LATENCY` milliseconds later, and every change is told to those listening that much later with
//! the rows as they were when it was made. A page reads a collection's rows as they were last told
//! or listed (`Collection::seen`), none until the first answer, so what a page shows lags a change
//! by the same moment the web's does. `Backend::advance` moves the clock; where the changes stand
//! (`saving`) follows the web's `savingNow`. Carried from the dataviz example's backend
//! (`examples/dataviz/src/backend.rs`).

use serde::{Deserialize, Serialize};

use crate::expression::{ItemKind, Node, Value, evaluate};
use crate::js::trim;
use crate::metadata::{age_combo, constant, constants, data_set, days_in, de, org_unit, shift_period, uid};
use crate::program::{ProgramRule, rule_seed};
use crate::values::{data_value, reports};

/// The server the workbench talks to, by its name and address.
pub const SERVER_NAME: &str = "tidevann-metadata";
pub const SERVER_ADDRESS: &str = "https://metadata.example.org";
/// The one signed in.
pub const ME: &str = "ada";

// --- Indicators -----------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndicatorType {
    #[serde(rename = "percent")]
    Percent,
    #[serde(rename = "perThousand")]
    PerThousand,
    #[serde(rename = "number")]
    Number,
}

pub const INDICATOR_TYPES: [IndicatorType; 3] = [
    IndicatorType::Percent,
    IndicatorType::PerThousand,
    IndicatorType::Number,
];

impl IndicatorType {
    pub fn key(self) -> &'static str {
        match self {
            IndicatorType::Percent => "percent",
            IndicatorType::PerThousand => "perThousand",
            IndicatorType::Number => "number",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            IndicatorType::Percent => "Percent",
            IndicatorType::PerThousand => "Per thousand",
            IndicatorType::Number => "Number",
        }
    }

    pub fn factor(self) -> f64 {
        match self {
            IndicatorType::Percent => 100.0,
            IndicatorType::PerThousand => 1000.0,
            IndicatorType::Number => 1.0,
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            IndicatorType::Percent => "%",
            IndicatorType::PerThousand => "\u{2030}",
            IndicatorType::Number => "",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Indicator {
    pub id: String,
    pub code: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub kind: IndicatorType,
    pub numerator: String,
    pub denominator: String,
}

fn indicator(
    code: &str,
    name: &str,
    kind: IndicatorType,
    numerator: String,
    denominator: String,
    description: &str,
) -> Indicator {
    Indicator {
        id: uid(&format!("ind:{code}")),
        code: code.to_owned(),
        name: name.to_owned(),
        description: description.to_owned(),
        kind,
        numerator,
        denominator,
    }
}

/// The indicators the demo starts with.
pub fn indicator_seed() -> Vec<Indicator> {
    let de = de();
    let under5 = &age_combo().combos[0].id;
    let target = &constants()[0].id;
    vec![
        indicator(
            "IND_VACC_COVERAGE",
            "Vaccination coverage",
            IndicatorType::Percent,
            format!("#{{{}}}", de.vaccinated.id),
            format!("#{{{}}}", de.due.id),
            "Children given their last dose, of those it fell due for.",
        ),
        indicator(
            "IND_VACC_TARGET",
            "Coverage against target",
            IndicatorType::Percent,
            format!(
                "/* coverage in percent */\n#{{{}}} / #{{{}}} * 100",
                de.vaccinated.id, de.due.id
            ),
            format!("C{{{target}}}"),
            "Vaccination coverage as a share of the target the constant holds.",
        ),
        indicator(
            "IND_UNDER5_SHARE",
            "Under-five share of visits",
            IndicatorType::Percent,
            format!("#{{{}.{under5}}}", de.visits.id),
            format!("#{{{}}}", de.visits.id),
            "Outpatient visits by children under five, of all visits.",
        ),
        indicator(
            "IND_REFERRAL_RATE",
            "Referrals per 1,000 visits",
            IndicatorType::PerThousand,
            format!("#{{{}}}", de.referrals.id),
            format!("#{{{}.*}}", de.visits.id),
            "People sent on to another site, per thousand visits.",
        ),
        indicator(
            "IND_VISITS_PER_DAY",
            "Visits per day",
            IndicatorType::Number,
            format!("#{{{}}}", de.visits.id),
            "[days]".to_owned(),
            "Outpatient visits over the days in the month.",
        ),
        indicator(
            "IND_WATER_PASS",
            "Water samples passing",
            IndicatorType::Percent,
            format!("#{{{}}} - #{{{}}}", de.samples_taken.id, de.samples_failed.id),
            format!("greatest(#{{{}}}, 1)", de.samples_taken.id),
            "Samples that passed the test, of those taken.",
        ),
        indicator(
            "IND_STOCK_CHANGE",
            "Stock change on last month",
            IndicatorType::Number,
            format!("#{{{0}}}\n  - #{{{0}}}.periodOffset(-1)", de.stock_on_hand.id),
            "1".to_owned(),
            "Kits on hand at the end of the month, against the end of the month before.",
        ),
        indicator(
            "IND_REPORTING",
            "Reporting rate",
            IndicatorType::Number,
            format!("R{{{}.REPORTING_RATE}}", data_set().id),
            "1".to_owned(),
            "Sites that sent the monthly report, in percent of those that should.",
        ),
        indicator(
            "IND_REFERRALS_NONZERO",
            "Referrals, months with any",
            IndicatorType::Number,
            format!("removeZeros(#{{{}}})", de.referrals.id),
            "1".to_owned(),
            "Referrals out, leaving out months with none so they count as missing.",
        ),
    ]
}

/// What an item stands for in a unit and a month, `offset` months away.
pub fn value_of(kind: ItemKind, raw: &str, unit: &str, period: &str, offset: f64) -> Value {
    let at = if offset == 0.0 {
        period.to_owned()
    } else {
        shift_period(period, offset)
    };
    let body = trim(raw);
    let mut parts = body.split('.');
    let first = parts.next().unwrap_or_default();
    let second = parts.next();
    match kind {
        ItemKind::Hash => {
            let combo = second.filter(|one| *one != "*");
            data_value(first, combo, unit, &at).map_or(Value::Null, Value::Number)
        }
        ItemKind::C => constant(body).map_or(Value::Null, |one| Value::Number(one.value)),
        ItemKind::Days => Value::Number(days_in(&at)),
        ItemKind::R => {
            let (actual, expected) = reports(unit, &at);
            if expected == 0 {
                return Value::Null;
            }
            Value::Number(match second {
                Some("ACTUAL_REPORTS") => actual as f64,
                Some("EXPECTED_REPORTS") => expected as f64,
                _ => (actual as f64 / expected as f64) * 100.0,
            })
        }
        _ => Value::Null,
    }
}

/// An aggregate expression's value for a unit and a month, and the items it read and found missing.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Aggregate {
    pub value: Option<f64>,
    pub items: usize,
    pub missing: usize,
}

/// An aggregate expression's value for a unit and a month: none when every item it reads is missing.
pub fn aggregate(node: &Node, unit: &str, period: &str) -> Aggregate {
    let scope = |kind: ItemKind, body: &str, offset: f64| value_of(kind, body, unit, period, offset);
    let result = evaluate(node, &scope);
    let value = match result.value {
        Value::Number(number) => Some(number),
        Value::Bool(flag) => Some(f64::from(u8::from(flag))),
        _ => None,
    };
    Aggregate {
        value: if result.items > 0 && result.missing == result.items {
            None
        } else {
            value
        },
        items: result.items,
        missing: result.missing,
    }
}

/// An indicator's parts and value for a unit and a month.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct IndicatorValue {
    pub numerator: Option<f64>,
    pub denominator: Option<f64>,
    pub value: Option<f64>,
}

/// An indicator's parts and value for a unit and a month: the numerator over the denominator, times
/// its type's factor.
pub fn indicator_value(
    numerator: &Node,
    denominator: &Node,
    kind: IndicatorType,
    unit: &str,
    period: &str,
) -> IndicatorValue {
    let top = aggregate(numerator, unit, period).value;
    let bottom = aggregate(denominator, unit, period).value;
    let value = match (top, bottom) {
        (Some(top), Some(bottom)) if bottom != 0.0 => Some((top / bottom) * kind.factor()),
        _ => None,
    };
    IndicatorValue {
        numerator: top,
        denominator: bottom,
        value,
    }
}

// --- Validation rules -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    EqualTo,
    NotEqualTo,
    GreaterThan,
    GreaterThanOrEqualTo,
    LessThan,
    LessThanOrEqualTo,
    CompulsoryPair,
    ExclusivePair,
}

pub const OPERATORS: [Operator; 8] = [
    Operator::EqualTo,
    Operator::NotEqualTo,
    Operator::GreaterThan,
    Operator::GreaterThanOrEqualTo,
    Operator::LessThan,
    Operator::LessThanOrEqualTo,
    Operator::CompulsoryPair,
    Operator::ExclusivePair,
];

impl Operator {
    pub fn key(self) -> &'static str {
        match self {
            Operator::EqualTo => "equal_to",
            Operator::NotEqualTo => "not_equal_to",
            Operator::GreaterThan => "greater_than",
            Operator::GreaterThanOrEqualTo => "greater_than_or_equal_to",
            Operator::LessThan => "less_than",
            Operator::LessThanOrEqualTo => "less_than_or_equal_to",
            Operator::CompulsoryPair => "compulsory_pair",
            Operator::ExclusivePair => "exclusive_pair",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Operator::EqualTo => "Equal to",
            Operator::NotEqualTo => "Not equal to",
            Operator::GreaterThan => "Greater than",
            Operator::GreaterThanOrEqualTo => "Greater than or equal to",
            Operator::LessThan => "Less than",
            Operator::LessThanOrEqualTo => "Less than or equal to",
            Operator::CompulsoryPair => "Compulsory pair",
            Operator::ExclusivePair => "Exclusive pair",
        }
    }

    pub fn sign(self) -> &'static str {
        match self {
            Operator::EqualTo => "==",
            Operator::NotEqualTo => "!=",
            Operator::GreaterThan => ">",
            Operator::GreaterThanOrEqualTo => ">=",
            Operator::LessThan => "<",
            Operator::LessThanOrEqualTo => "<=",
            Operator::CompulsoryPair => "both or neither",
            Operator::ExclusivePair => "not both",
        }
    }

    fn pair(self) -> bool {
        matches!(self, Operator::CompulsoryPair | Operator::ExclusivePair)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MissingValueStrategy {
    SkipIfAnyValueMissing,
    SkipIfAllValuesMissing,
    NeverSkip,
}

pub const STRATEGIES: [MissingValueStrategy; 3] = [
    MissingValueStrategy::SkipIfAnyValueMissing,
    MissingValueStrategy::SkipIfAllValuesMissing,
    MissingValueStrategy::NeverSkip,
];

impl MissingValueStrategy {
    pub fn key(self) -> &'static str {
        match self {
            MissingValueStrategy::SkipIfAnyValueMissing => "SKIP_IF_ANY_VALUE_MISSING",
            MissingValueStrategy::SkipIfAllValuesMissing => "SKIP_IF_ALL_VALUES_MISSING",
            MissingValueStrategy::NeverSkip => "NEVER_SKIP",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            MissingValueStrategy::SkipIfAnyValueMissing => "Skip if any value is missing",
            MissingValueStrategy::SkipIfAllValuesMissing => "Skip if all values are missing",
            MissingValueStrategy::NeverSkip => "Never skip",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            MissingValueStrategy::SkipIfAnyValueMissing => "The side counts only when every item it reads has a value.",
            MissingValueStrategy::SkipIfAllValuesMissing => {
                "A missing item counts as zero, unless every one is missing."
            }
            MissingValueStrategy::NeverSkip => "A missing item counts as zero, and so does a side with none.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Importance {
    High,
    Medium,
    Low,
}

impl Importance {
    pub fn key(self) -> &'static str {
        match self {
            Importance::High => "high",
            Importance::Medium => "medium",
            Importance::Low => "low",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationRule {
    pub id: String,
    pub code: String,
    pub name: String,
    pub instruction: String,
    pub importance: Importance,
    pub left: String,
    pub operator: Operator,
    pub right: String,
    pub left_strategy: MissingValueStrategy,
    pub right_strategy: MissingValueStrategy,
}

fn rule(
    code: &str,
    name: &str,
    importance: Importance,
    left: String,
    operator: Operator,
    right: String,
    instruction: &str,
) -> ValidationRule {
    ValidationRule {
        id: uid(&format!("vr:{code}")),
        code: code.to_owned(),
        name: name.to_owned(),
        instruction: instruction.to_owned(),
        importance,
        left,
        operator,
        right,
        left_strategy: MissingValueStrategy::SkipIfAnyValueMissing,
        right_strategy: MissingValueStrategy::SkipIfAnyValueMissing,
    }
}

/// The validation rules the demo starts with.
pub fn rules_seed() -> Vec<ValidationRule> {
    let de = de();
    vec![
        rule(
            "VR_VACCINATED_DUE",
            "Vaccinated within due",
            Importance::High,
            format!("#{{{}}}", de.vaccinated.id),
            Operator::LessThanOrEqualTo,
            format!("#{{{}}}", de.due.id),
            "No more children can be vaccinated than were due.",
        ),
        rule(
            "VR_SAMPLES",
            "Failed samples within taken",
            Importance::Medium,
            format!("#{{{}}}", de.samples_failed.id),
            Operator::LessThanOrEqualTo,
            format!("#{{{}}}", de.samples_taken.id),
            "A sample can only fail if it was taken.",
        ),
        rule(
            "VR_STOCK",
            "Stock used within stock held",
            Importance::High,
            format!("#{{{}}}", de.stock_used.id),
            Operator::LessThanOrEqualTo,
            format!(
                "#{{{}}}.periodOffset(-1)\n  + #{{{}}}",
                de.stock_on_hand.id, de.stock_received.id
            ),
            "Kits used cannot be more than those left last month and those received.",
        ),
        rule(
            "VR_REFERRALS",
            "Referrals within visits",
            Importance::Low,
            format!("#{{{}}}", de.referrals.id),
            Operator::LessThanOrEqualTo,
            format!("#{{{}}} * 0.1", de.visits.id),
            "More than one visit in ten referred on is worth a second look.",
        ),
    ]
}

/// A side's value under its missing value strategy: `Err` when the strategy skips it.
fn side(node: &Node, strategy: MissingValueStrategy, unit: &str, period: &str) -> Result<Option<f64>, ()> {
    let result = aggregate(node, unit, period);
    if strategy == MissingValueStrategy::SkipIfAnyValueMissing && result.missing > 0 {
        return Err(());
    }
    if strategy == MissingValueStrategy::SkipIfAllValuesMissing && result.items > 0 && result.missing == result.items {
        return Err(());
    }
    match result.value {
        Some(value) => Ok(Some(value)),
        None if strategy == MissingValueStrategy::NeverSkip => Ok(Some(0.0)),
        None => Err(()),
    }
}

/// Whether a rule holds for two sides.
pub fn holds(operator: Operator, left: Option<f64>, right: Option<f64>) -> bool {
    match operator {
        Operator::CompulsoryPair => return left.is_none() == right.is_none(),
        Operator::ExclusivePair => return left.is_none() || right.is_none(),
        _ => {}
    }
    let a = left.unwrap_or(0.0);
    let b = right.unwrap_or(0.0);
    match operator {
        Operator::EqualTo => (a - b).abs() < 1e-9,
        Operator::NotEqualTo => (a - b).abs() >= 1e-9,
        Operator::GreaterThan => a > b,
        Operator::GreaterThanOrEqualTo => a >= b,
        Operator::LessThan => a < b,
        _ => a <= b,
    }
}

/// A site and month a rule does not hold for, with its two sides.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Violation {
    pub id: String,
    pub rule: String,
    pub unit: String,
    pub period: String,
    pub left: Option<f64>,
    pub right: Option<f64>,
}

/// A rule's run: the pairs it does not hold for, and how many it was tried on and skipped.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RuleOutcome {
    pub violations: Vec<Violation>,
    pub tried: usize,
    pub skipped: usize,
}

/// A rule run for each unit and month given: the pairs it does not hold for, and how many it was
/// tried on.
#[allow(clippy::too_many_arguments)]
pub fn run_rule(
    id: &str,
    operator: Operator,
    left_strategy: MissingValueStrategy,
    right_strategy: MissingValueStrategy,
    left: &Node,
    right: &Node,
    units: &[String],
    periods: &[String],
) -> RuleOutcome {
    let mut out = RuleOutcome {
        violations: Vec::new(),
        tried: 0,
        skipped: 0,
    };
    for unit in units {
        for period in periods {
            let a = side(left, left_strategy, unit, period);
            let b = side(right, right_strategy, unit, period);
            if !operator.pair() && (a.is_err() || b.is_err()) {
                out.skipped += 1;
                continue;
            }
            out.tried += 1;
            let (a, b) = (a.ok().flatten(), b.ok().flatten());
            if !holds(operator, a, b) {
                out.violations.push(Violation {
                    id: format!("{id}.{unit}.{period}"),
                    rule: id.to_owned(),
                    unit: unit.clone(),
                    period: period.clone(),
                    left: a,
                    right: b,
                });
            }
        }
    }
    out
}

/// A unit's name, or its uid when there is none.
pub fn unit_name(id: &str) -> String {
    org_unit(id).map_or_else(|| id.to_owned(), |one| one.name.clone())
}

// --- The collections ------------------------------------------------------------------------------

/// How long each call to the backend takes, in milliseconds.
pub const LATENCY: i64 = 150;

/// A row of a collection, kept under its id.
pub trait Row: Clone {
    fn id(&self) -> &str;
}

impl Row for Indicator {
    fn id(&self) -> &str {
        &self.id
    }
}

impl Row for ValidationRule {
    fn id(&self) -> &str {
        &self.id
    }
}

impl Row for ProgramRule {
    fn id(&self) -> &str {
        &self.id
    }
}

/// The collections, by the name the backend keeps each under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Name {
    Indicators,
    ValidationRules,
    ProgramRules,
}

impl Name {
    pub fn key(self) -> &'static str {
        match self {
            Name::Indicators => "indicators",
            Name::ValidationRules => "validation-rules",
            Name::ProgramRules => "program-rules",
        }
    }
}

/// A note the app shows for a moment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Toast {
    /// `success`, or `error` for a failure; none for a plain note.
    #[serde(rename = "type")]
    pub kind: Option<&'static str>,
    pub title: String,
    pub description: Option<String>,
}

impl Toast {
    pub fn note(title: impl Into<String>) -> Self {
        Toast {
            kind: None,
            title: title.into(),
            description: None,
        }
    }

    pub fn success(title: impl Into<String>, description: impl Into<String>) -> Self {
        Toast {
            kind: Some("success"),
            title: title.into(),
            description: Some(description.into()),
        }
    }

    pub fn error(title: impl Into<String>, description: impl Into<String>) -> Self {
        Toast {
            kind: Some("error"),
            title: title.into(),
            description: Some(description.into()),
        }
    }
}

/// Where the example's changes stand: some still on their way, the last one refused, or all kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Saving {
    Saving,
    Failed(String),
    Saved,
}

/// The words a status line says for where the changes stand, its tone and whether it pulses.
pub fn saving_note(saving: &Saving) -> (String, &'static str, bool) {
    match saving {
        Saving::Saving => ("Saving".to_owned(), "quiet", true),
        Saving::Failed(reason) => (format!("Not saved: {reason}"), "error", false),
        Saving::Saved => ("All changes saved".to_owned(), "quiet", false),
    }
}

/// Something a page hears once its change is answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A save was answered, kept or refused: the page is no longer busy.
    Settled,
    /// A program rule's save was kept: its draft goes.
    Discard,
}

/// What follows a change once it is answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Then {
    Toast(Toast),
    /// Goes to a route, as the web's `go` sets the hash.
    Go(String),
    /// Tells the page.
    Event(Event),
    /// On a refusal only: an error note with this title, the refusal's words its description.
    Refused(String),
}

#[derive(Clone, Debug)]
enum Job {
    /// A collection's change told, or its list answered, with the rows kept under the timer's seq.
    Heard(Name),
    /// A change answered: kept or refused, and what follows it.
    Answer { failure: Option<String>, then: Vec<Then> },
}

#[derive(Clone, Debug)]
struct Timer {
    at: i64,
    seq: u64,
    job: Job,
}

/// The web's clock and its timers.
#[derive(Clone, Debug, Default)]
pub struct Clock {
    pub now: i64,
    seq: u64,
    timers: Vec<Timer>,
}

impl Clock {
    fn after(&mut self, ms: i64, job: Job) -> u64 {
        self.seq += 1;
        self.timers.push(Timer {
            at: self.now + ms,
            seq: self.seq,
            job,
        });
        self.seq
    }
}

/// One collection: its rows as the backend keeps them, and as the pages last heard them.
#[derive(Clone, Debug)]
pub struct Collection<T: Row> {
    pub name: Name,
    /// The rows as kept, every change made at once.
    pub rows: Vec<T>,
    /// The rows as last told or listed, none until the first answer.
    pub seen: Option<Vec<T>>,
    pending: Vec<(u64, Vec<T>)>,
}

impl<T: Row> Collection<T> {
    fn new(name: Name, rows: Vec<T>) -> Self {
        Collection {
            name,
            rows,
            seen: None,
            pending: Vec::new(),
        }
    }

    /// The rows a page shows: none until the first answer.
    pub fn seen(&self) -> &[T] {
        self.seen.as_deref().unwrap_or_default()
    }

    /// The row under `id` as kept.
    pub fn get(&self, id: &str) -> Option<&T> {
        self.rows.iter().find(|row| row.id() == id)
    }

    fn heard(&mut self, seq: u64) {
        if let Some(at) = self.pending.iter().position(|(one, _)| *one == seq) {
            self.seen = Some(self.pending.remove(at).1);
        }
    }

    fn list(&mut self, clock: &mut Clock) {
        let seq = clock.after(LATENCY, Job::Heard(self.name));
        self.pending.push((seq, self.rows.clone()));
    }

    fn save(&mut self, clock: &mut Clock, next: Vec<T>) {
        self.rows = next;
        let seq = clock.after(LATENCY, Job::Heard(self.name));
        self.pending.push((seq, self.rows.clone()));
    }
}

/// The workbench's backend on the web's clock, and what the app said and where it went.
#[derive(Clone, Debug)]
pub struct Backend {
    pub clock: Clock,
    pub indicators: Collection<Indicator>,
    pub validation_rules: Collection<ValidationRule>,
    pub program_rules: Collection<ProgramRule>,
    /// Where the changes stand, and how many are on their way.
    pub saving: Saving,
    pending: usize,
    /// How many actions `new_action` made, whose count starts each one's key.
    pub actions_made: u64,
    /// The notes shown since they were last taken.
    pub toasts: Vec<Toast>,
    /// The routes gone to since they were last taken, as `go` sets them.
    pub went: Vec<String>,
    /// What the pages were told since they last heard.
    pub events: Vec<Event>,
}

impl Backend {
    /// The backend as the app opens at `now`: every collection from its seed, listed once as the
    /// app lists them.
    pub fn new(now: i64) -> Self {
        let mut backend = Backend {
            clock: Clock {
                now,
                ..Clock::default()
            },
            indicators: Collection::new(Name::Indicators, indicator_seed()),
            validation_rules: Collection::new(Name::ValidationRules, rules_seed()),
            program_rules: Collection::new(Name::ProgramRules, rule_seed()),
            saving: Saving::Saved,
            pending: 0,
            actions_made: 0,
            toasts: Vec::new(),
            went: Vec::new(),
            events: Vec::new(),
        };
        backend.list_all();
        backend
    }

    /// The rows eframe's storage kept put back on a new visit, as the web's backend keeps its
    /// collections in the browser between visits; listed again as the app lists them.
    pub fn restore(&mut self, indicators: Vec<Indicator>, rules: Vec<ValidationRule>, program_rules: Vec<ProgramRule>) {
        self.indicators.rows = indicators;
        self.validation_rules.rows = rules;
        self.program_rules.rows = program_rules;
        self.list_all();
    }

    /// Every collection listed, as a page that reads them does when it is drawn.
    pub fn list_all(&mut self) {
        self.indicators.list(&mut self.clock);
        self.validation_rules.list(&mut self.clock);
        self.program_rules.list(&mut self.clock);
    }

    pub fn now(&self) -> i64 {
        self.clock.now
    }

    /// Fires every timer due by `to`, each at its time, in the order they were set.
    pub fn advance(&mut self, to: i64) {
        loop {
            let next = self
                .clock
                .timers
                .iter()
                .enumerate()
                .filter(|(_, timer)| timer.at <= to)
                .min_by_key(|(_, timer)| (timer.at, timer.seq))
                .map(|(index, _)| index);
            let Some(index) = next else { break };
            let timer = self.clock.timers.remove(index);
            self.clock.now = timer.at;
            self.fire(timer.seq, timer.job);
        }
        self.clock.now = self.clock.now.max(to);
    }

    /// The time of the next timer due, if any.
    pub fn next_due(&self) -> Option<i64> {
        self.clock.timers.iter().map(|timer| timer.at).min()
    }

    fn fire(&mut self, seq: u64, job: Job) {
        match job {
            Job::Heard(Name::Indicators) => self.indicators.heard(seq),
            Job::Heard(Name::ValidationRules) => self.validation_rules.heard(seq),
            Job::Heard(Name::ProgramRules) => self.program_rules.heard(seq),
            Job::Answer { failure, then } => {
                self.pending -= 1;
                match &failure {
                    Some(reason) => self.saving = Saving::Failed(reason.clone()),
                    None => {
                        if self.pending == 0 && self.saving == Saving::Saving {
                            self.saving = Saving::Saved;
                        }
                    }
                }
                for one in then {
                    match (one, &failure) {
                        (Then::Refused(title), Some(reason)) => self.toasts.push(Toast::error(title, reason.clone())),
                        (Then::Refused(_), None) => {}
                        (Then::Toast(toast), None) => self.toasts.push(toast),
                        (Then::Go(path), None) => self.went.push(path),
                        (Then::Event(Event::Settled), _) => self.events.push(Event::Settled),
                        (Then::Event(event), None) => self.events.push(event),
                        _ => {}
                    }
                }
            }
        }
    }

    /// Shows a note at once, as a page's own `toast.add` does.
    pub fn toast(&mut self, toast: Toast) {
        self.toasts.push(toast);
    }

    /// Goes to a route at once, as the web's `go` sets the hash.
    pub fn go(&mut self, path: impl Into<String>) {
        self.went.push(path.into());
    }

    /// A change made: followed by `saving` until it is answered, with what follows it.
    fn answered(&mut self, failure: Option<String>, then: Vec<Then>) {
        self.pending += 1;
        if self.saving != Saving::Saving {
            self.saving = Saving::Saving;
        }
        self.clock.after(LATENCY, Job::Answer { failure, then });
    }

    // --- Indicators ---

    pub fn create_indicator(&mut self, row: Indicator, then: Vec<Then>) {
        let mut next = vec![row];
        next.extend(self.indicators.rows.iter().cloned());
        self.indicators.save(&mut self.clock, next);
        self.answered(None, then);
    }

    /// Changes the indicator under `id`; there being none is refused.
    pub fn update_indicator(&mut self, id: &str, change: impl FnOnce(&mut Indicator), then: Vec<Then>) {
        match self.indicators.get(id).cloned() {
            None => self.answered(Some(format!("No indicators with the id {id}.")), then),
            Some(mut row) => {
                change(&mut row);
                let next = self
                    .indicators
                    .rows
                    .iter()
                    .map(|one| if one.id == id { row.clone() } else { one.clone() })
                    .collect();
                self.indicators.save(&mut self.clock, next);
                self.answered(None, then);
            }
        }
    }

    pub fn remove_indicator(&mut self, id: &str, then: Vec<Then>) {
        let next = self
            .indicators
            .rows
            .iter()
            .filter(|one| one.id != id)
            .cloned()
            .collect();
        self.indicators.save(&mut self.clock, next);
        self.answered(None, then);
    }

    // --- Validation rules ---

    pub fn create_rule(&mut self, row: ValidationRule, then: Vec<Then>) {
        let mut next = vec![row];
        next.extend(self.validation_rules.rows.iter().cloned());
        self.validation_rules.save(&mut self.clock, next);
        self.answered(None, then);
    }

    /// Changes the validation rule under `id`; there being none is refused.
    pub fn update_rule(&mut self, id: &str, change: impl FnOnce(&mut ValidationRule), then: Vec<Then>) {
        match self.validation_rules.get(id).cloned() {
            None => self.answered(Some(format!("No validation-rules with the id {id}.")), then),
            Some(mut row) => {
                change(&mut row);
                let next = self
                    .validation_rules
                    .rows
                    .iter()
                    .map(|one| if one.id == id { row.clone() } else { one.clone() })
                    .collect();
                self.validation_rules.save(&mut self.clock, next);
                self.answered(None, then);
            }
        }
    }

    pub fn remove_rule(&mut self, id: &str, then: Vec<Then>) {
        let next = self
            .validation_rules
            .rows
            .iter()
            .filter(|one| one.id != id)
            .cloned()
            .collect();
        self.validation_rules.save(&mut self.clock, next);
        self.answered(None, then);
    }

    // --- Program rules ---

    pub fn create_program_rule(&mut self, row: ProgramRule, then: Vec<Then>) {
        let mut next = vec![row];
        next.extend(self.program_rules.rows.iter().cloned());
        self.program_rules.save(&mut self.clock, next);
        self.answered(None, then);
    }

    /// Changes the program rule under `id`; there being none is refused.
    pub fn update_program_rule(&mut self, id: &str, change: impl FnOnce(&mut ProgramRule), then: Vec<Then>) {
        match self.program_rules.get(id).cloned() {
            None => self.answered(Some(format!("No program-rules with the id {id}.")), then),
            Some(mut row) => {
                change(&mut row);
                let next = self
                    .program_rules
                    .rows
                    .iter()
                    .map(|one| if one.id == id { row.clone() } else { one.clone() })
                    .collect();
                self.program_rules.save(&mut self.clock, next);
                self.answered(None, then);
            }
        }
    }

    pub fn remove_program_rule(&mut self, id: &str, then: Vec<Then>) {
        let next = self
            .program_rules
            .rows
            .iter()
            .filter(|one| one.id != id)
            .cloned()
            .collect();
        self.program_rules.save(&mut self.clock, next);
        self.answered(None, then);
    }

    /// The pages' events since they last heard.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
}
