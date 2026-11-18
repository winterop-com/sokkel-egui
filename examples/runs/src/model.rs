//! What Tidevann's jobs console holds, as the web's `model.ts`: jobs, their runs, the saved views of
//! the runs and a person's preferences, and the words times, durations and counts are said in.
//!
//! Every time here is on the UTC clock, as the web's model reads it. The local zone is kept for the
//! parts of the console that read the local clock: `set_local_offset` sets it, UTC until then.

use std::sync::LazyLock;
use std::sync::atomic::{AtomicI64, Ordering};

use serde::{Deserialize, Serialize};
use sokkel_egui::{DashboardLayoutItem, ListViewState};

/// A minute in milliseconds.
pub const MINUTE: i64 = 60_000;
/// An hour in milliseconds.
pub const HOUR: i64 = 60 * MINUTE;
/// A day in milliseconds.
pub const DAY: i64 = 24 * HOUR;

/// How far the local zone is ahead of UTC, in milliseconds: UTC until the app sets it.
static OFFSET: AtomicI64 = AtomicI64::new(0);

/// Sets the local zone, `ms` milliseconds ahead of UTC, as the app starts.
pub fn set_local_offset(ms: i64) {
    OFFSET.store(ms, Ordering::Relaxed);
}

/// How far the local zone is ahead of UTC, in milliseconds.
pub fn local_offset() -> i64 {
    OFFSET.load(Ordering::Relaxed)
}

/// The people of Tidevann's workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Person {
    Ada,
    Grace,
    Linus,
    Margaret,
}

pub const PEOPLE: [Person; 4] = [Person::Ada, Person::Grace, Person::Linus, Person::Margaret];

impl Person {
    pub fn word(self) -> &'static str {
        match self {
            Self::Ada => "ada",
            Self::Grace => "grace",
            Self::Linus => "linus",
            Self::Margaret => "margaret",
        }
    }

    /// The person a word names.
    pub fn from_word(word: &str) -> Option<Self> {
        PEOPLE.into_iter().find(|one| one.word() == word)
    }
}

/// The projects of Tidevann's workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Project {
    Harbor,
    Juniper,
    Larkspur,
}

pub const PROJECTS: [Project; 3] = [Project::Harbor, Project::Juniper, Project::Larkspur];

impl Project {
    pub fn word(self) -> &'static str {
        match self {
            Self::Harbor => "harbor",
            Self::Juniper => "juniper",
            Self::Larkspur => "larkspur",
        }
    }
}

/// The states a run or a step stands at, as sokkel-ui's run `Status`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Succeeded,
    Failed,
    Running,
    Queued,
    Cancelled,
    Skipped,
}

/// The run states a run can stand at, in the order the console lists them.
pub const STATES: [Status; 6] = [
    Status::Succeeded,
    Status::Failed,
    Status::Running,
    Status::Queued,
    Status::Cancelled,
    Status::Skipped,
];

impl Status {
    pub fn word(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Running => "running",
            Self::Queued => "queued",
            Self::Cancelled => "cancelled",
            Self::Skipped => "skipped",
        }
    }
}

/// How often a schedule fires; every time is on the UTC clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Cadence {
    #[serde(rename = "15m")]
    Quarter,
    #[serde(rename = "hourly")]
    Hourly,
    #[serde(rename = "daily")]
    Daily,
    #[serde(rename = "weekdays")]
    Weekdays,
    #[serde(rename = "weekly")]
    Weekly,
}

impl Cadence {
    /// The cadence as the web writes it: `15m`, `hourly`, ...
    pub fn value(self) -> &'static str {
        match self {
            Self::Quarter => "15m",
            Self::Hourly => "hourly",
            Self::Daily => "daily",
            Self::Weekdays => "weekdays",
            Self::Weekly => "weekly",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schedule {
    pub cadence: Cadence,
    /// The time it fires at, `HH:mm` UTC: its minute alone for `15m` and `hourly`.
    pub at: String,
    /// The first day it may fire, `YYYY-MM-DD`; a weekly schedule fires on this day's weekday.
    pub starts: String,
}

impl Schedule {
    pub fn new(cadence: Cadence, at: &str, starts: &str) -> Self {
        Self {
            cadence,
            at: at.to_owned(),
            starts: starts.to_owned(),
        }
    }
}

/// One step of a job: its code, what it runs on, and the steps it waits for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobStep {
    pub code: String,
    pub kind: String,
    pub needs: Vec<String>,
}

impl JobStep {
    pub fn new(code: &str, kind: &str, needs: &[&str]) -> Self {
        Self {
            code: code.to_owned(),
            kind: kind.to_owned(),
            needs: needs.iter().map(|one| (*one).to_owned()).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub description: String,
    pub project: Project,
    pub owner: Person,
    pub steps: Vec<JobStep>,
    /// When it runs by itself; none for a job only started by hand.
    pub schedule: Option<Schedule>,
    pub paused: bool,
    pub retries: i64,
    /// Minutes a run may take before it is stopped; none for no limit.
    pub timeout: Option<i64>,
    /// Seconds a step of a scheduled run takes, about.
    pub pace: f64,
    /// The share of runs that fail.
    pub risk: f64,
}

/// One step of a run as it was planned: how long it takes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunStepPlan {
    pub code: String,
    pub kind: String,
    pub needs: Vec<String>,
    pub ms: i64,
}

/// Who started a run: the schedule, or a person.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum By {
    Schedule,
    Person(Person),
}

impl By {
    pub fn word(self) -> &'static str {
        match self {
            Self::Schedule => "schedule",
            Self::Person(person) => person.word(),
        }
    }
}

impl From<By> for String {
    fn from(by: By) -> Self {
        by.word().to_owned()
    }
}

impl TryFrom<String> for By {
    type Error = String;

    fn try_from(word: String) -> Result<Self, Self::Error> {
        if word == "schedule" {
            return Ok(Self::Schedule);
        }
        Person::from_word(&word)
            .map(Self::Person)
            .ok_or_else(|| format!("{word} is neither the schedule nor a person"))
    }
}

/// How a run was started.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Trigger {
    Schedule,
    Manual,
    Retry,
}

impl Trigger {
    pub fn word(self) -> &'static str {
        match self {
            Self::Schedule => "schedule",
            Self::Manual => "manual",
            Self::Retry => "retry",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub id: String,
    pub job: String,
    pub project: Project,
    pub by: By,
    pub trigger: Trigger,
    /// The run this one retries.
    pub retry_of: Option<String>,
    /// When it was asked for, in milliseconds since the epoch.
    pub created: i64,
    /// How long it waits for a worker before its first step.
    pub wait: i64,
    pub steps: Vec<RunStepPlan>,
    /// The step that fails, and what it says.
    pub fail_at: Option<String>,
    pub error: Option<String>,
    /// Why the scheduler never started it, such as the run before still going.
    pub skipped: Option<String>,
    pub cancelled_at: Option<i64>,
    pub cancelled_by: Option<Person>,
    pub records: i64,
    /// What it is given: text, numbers and flags by name, in the order the web writes them.
    pub inputs: serde_json::Map<String, serde_json::Value>,
    /// The state it last stood at, kept as it changes so a listing filters without a clock.
    pub state: Status,
    pub ended: Option<i64>,
}

/// A saved view of the runs listing, kept for the person.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunView {
    pub id: String,
    pub name: String,
    pub state: ListViewState,
    #[serde(default)]
    pub locked: bool,
}

/// Where a run opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenRuns {
    Panel,
    Page,
}

/// How times are said: from now, or on the clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Times {
    Relative,
    Clock,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    pub id: String,
    /// Who is signed in; none after signing out.
    pub user: Option<Person>,
    pub open_runs: OpenRuns,
    pub times: Times,
    pub confirm_cancel: bool,
    pub notify_failures: bool,
    pub notify_mine: bool,
    pub follow_log: bool,
    pub wrap_log: bool,
    /// Whether the scheduler starts runs on their schedules.
    pub scheduler: bool,
    /// The overview's panels as the person arranged them, in order with their widths; none for the
    /// default. Not written when the prefs are saved.
    #[serde(skip)]
    pub overview: Option<Vec<DashboardLayoutItem>>,
}

pub static DEFAULT_PREFS: LazyLock<Prefs> = LazyLock::new(|| Prefs {
    id: "me".to_owned(),
    user: Some(Person::Ada),
    open_runs: OpenRuns::Panel,
    times: Times::Relative,
    confirm_cancel: true,
    notify_failures: true,
    notify_mine: true,
    follow_log: true,
    wrap_log: true,
    scheduler: true,
    overview: None,
});

impl Default for Prefs {
    fn default() -> Self {
        DEFAULT_PREFS.clone()
    }
}

/// `Math.round`: the nearest whole number, a half rounded up toward positive infinity.
pub fn js_round(value: f64) -> f64 {
    let floor = value.floor();
    if value - floor >= 0.5 { floor + 1.0 } else { floor }
}

/// `Number.prototype.toString(36)` of a whole number.
pub fn base36(value: i64) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".to_owned();
    }
    let mut left = value.unsigned_abs();
    let mut out = Vec::new();
    while left > 0 {
        out.push(DIGITS[(left % 36) as usize]);
        left /= 36;
    }
    if value < 0 {
        out.push(b'-');
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// A whole number as `toLocaleString('en-US')` writes it: `12,345`.
pub fn thousands(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    if value < 0 { format!("-{out}") } else { out }
}

/// `Number.prototype.toFixed(digits)` of a value from 0 up to a million, with up to a few digits:
/// the exact value of the double rounded, a tie going to the larger number.
fn to_fixed(value: f64, digits: usize) -> String {
    // Sixty decimals write a double of this size exactly, so a tie is seen as one.
    let exact = format!("{value:.60}");
    let (whole, fraction) = exact.split_once('.').unwrap_or((&exact, ""));
    let kept = format!("{whole}{}", &fraction[..digits]);
    let rest = &fraction[digits..];
    let up = rest.as_bytes().first().is_some_and(|first| *first >= b'5');
    let mut number: u128 = kept.parse().unwrap_or(0);
    if up {
        number += 1;
    }
    let text = format!("{number:0>width$}", width = digits + 1);
    if digits == 0 {
        return text;
    }
    let (head, tail) = text.split_at(text.len() - digits);
    format!("{head}.{tail}")
}

/// A duration as a reader says it: `840ms`, `4.2s`, `48s`, `4m 12s`, `1h 03m`.
pub fn took(ms: f64) -> String {
    if ms < 1000.0 {
        return format!("{}ms", js_round(ms).max(0.0));
    }
    let seconds = js_round(ms / 1000.0) as i64;
    if ms < 10_000.0 {
        return format!("{}s", to_fixed(ms / 1000.0, 1));
    }
    if seconds < 60 {
        return format!("{seconds}s");
    }
    if seconds < 3600 {
        return format!("{}m {:02}s", seconds / 60, seconds % 60);
    }
    format!("{}h {:02}m", seconds / 3600, (seconds % 3600) / 60)
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// The year, month (1 to 12) and day of a day number since 1970.
fn civil(day: i64) -> (i64, i64, i64) {
    // Howard Hinnant's civil_from_days
    let z = day + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// The day number since 1970 of a date.
fn day_from(year: i64, month: i64, date: i64) -> i64 {
    // Howard Hinnant's days_from_civil
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + date - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn days_in(year: i64, month: i64) -> i64 {
    day_from(year + i64::from(month == 12), month % 12 + 1, 1) - day_from(year, month, 1)
}

/// UTC midnight of a `YYYY-MM-DD` day, in milliseconds, as `new Date('<day>T00:00:00Z')` reads it;
/// none where the web's date is invalid.
pub fn start_of_day(day: &str) -> Option<i64> {
    let bytes = day.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let number = |range: std::ops::Range<usize>| -> Option<i64> {
        let part = &day[range];
        part.bytes().all(|b| b.is_ascii_digit()).then(|| part.parse().ok())?
    };
    let (year, month, date) = (number(0..4)?, number(5..7)?, number(8..10)?);
    if !(1..=12).contains(&month) || date < 1 || date > days_in(year, month) {
        return None;
    }
    Some(day_from(year, month, date) * DAY)
}

/// A moment's UTC parts: year, month (1 to 12), day, weekday (0 for Sunday), hours, minutes,
/// seconds and milliseconds, as `Date`'s `getUTC*` read them.
fn parts(at: i64) -> (i64, i64, i64, usize, i64, i64, i64, i64) {
    let day = at.div_euclid(DAY);
    let within = at.rem_euclid(DAY);
    let (year, month, date) = civil(day);
    (
        year,
        month,
        date,
        (day + 4).rem_euclid(7) as usize,
        within / HOUR,
        within % HOUR / MINUTE,
        within % MINUTE / 1000,
        within % 1000,
    )
}

/// The time of day on the UTC clock: `14:02`, or `14:02:07` with seconds.
pub fn clock(at: i64, seconds: bool) -> String {
    let (_, _, _, _, hours, minutes, secs, _) = parts(at);
    if seconds {
        format!("{hours:02}:{minutes:02}:{secs:02}")
    } else {
        format!("{hours:02}:{minutes:02}")
    }
}

/// A moment as a reader says it, on the UTC clock: `Nov 8, 14:02`.
pub fn stamp(at: i64) -> String {
    let (_, month, date, ..) = parts(at);
    format!("{} {date}, {}", MONTHS[(month - 1) as usize], clock(at, false))
}

/// The UTC day of a moment, `YYYY-MM-DD`.
pub fn day_of(at: i64) -> String {
    let (year, month, date, ..) = parts(at);
    format!("{year}-{month:02}-{date:02}")
}

/// A moment as `Date.prototype.toISOString` writes it: `2026-11-18T07:16:40.000Z`.
pub fn iso_string(at: i64) -> String {
    let (year, month, date, _, hours, minutes, seconds, ms) = parts(at);
    format!("{year:04}-{month:02}-{date:02}T{hours:02}:{minutes:02}:{seconds:02}.{ms:03}Z")
}

/// A day as a heading says it, from today's: Today, Tomorrow, or `Wednesday, Nov 12`. A day the
/// web cannot read is given back as it is.
pub fn say_day(day: &str, today: &str) -> String {
    if day == today {
        return "Today".to_owned();
    }
    let Some(start) = start_of_day(day) else {
        return day.to_owned();
    };
    if start_of_day(today).is_some_and(|today| day_of(today + DAY) == day) {
        return "Tomorrow".to_owned();
    }
    let (_, month, date, weekday, ..) = parts(start);
    format!("{}, {} {date}", WEEKDAYS[weekday], MONTHS[(month - 1) as usize])
}

/// The weekday of a `YYYY-MM-DD` day, `Wednesday`; empty for a day the web cannot read.
pub fn weekday_of(day: &str) -> String {
    start_of_day(day)
        .map(|start| WEEKDAYS[parts(start).3].to_owned())
        .unwrap_or_default()
}

/// A count with its noun: `1 run`, `12 runs`.
pub fn counted(count: i64, one: &str) -> String {
    counted_as(count, one, &format!("{one}s"))
}

/// A count with its noun, the plural given: `0 people`.
pub fn counted_as(count: i64, one: &str, many: &str) -> String {
    format!("{} {}", thousands(count), if count == 1 { one } else { many })
}

/// Who started a run, as a listing says it.
pub fn say_by(by: By, trigger: Trigger) -> String {
    match by {
        By::Schedule => "schedule".to_owned(),
        By::Person(person) if trigger == Trigger::Retry => format!("{}, retry", person.word()),
        By::Person(person) => person.word().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_794_992_400_000;

    #[test]
    fn durations_read_as_the_web_says_them() {
        let cases: [(f64, &str); 22] = [
            (0.0, "0ms"),
            (-5.0, "0ms"),
            (999.5, "1000ms"),
            (999.4, "999ms"),
            (1000.0, "1.0s"),
            (1050.0, "1.1s"),
            (1150.0, "1.1s"),
            (1250.0, "1.3s"),
            (1350.0, "1.4s"),
            (4249.0, "4.2s"),
            (9949.0, "9.9s"),
            (9950.0, "9.9s"),
            (9999.0, "10.0s"),
            (10_000.0, "10s"),
            (59_499.0, "59s"),
            (59_500.0, "1m 00s"),
            (61_000.0, "1m 01s"),
            (3_599_499.0, "59m 59s"),
            (3_599_500.0, "1h 00m"),
            (3_780_000.0, "1h 03m"),
            (86_400_000.0, "24h 00m"),
            (1_600.000_000_000_000_2, "1.6s"),
        ];
        for (ms, said) in cases {
            assert_eq!(took(ms), said, "took({ms})");
        }
    }

    #[test]
    fn moments_read_as_the_web_says_them() {
        assert_eq!(clock(NOW, false), "09:00");
        assert_eq!(clock(NOW + 7_000, true), "09:00:07");
        assert_eq!(stamp(NOW), "Nov 18, 09:00");
        assert_eq!(day_of(NOW), "2026-11-18");
        assert_eq!(
            [clock(0, false), clock(0, true), stamp(0), day_of(0)],
            ["00:00", "00:00:00", "Jan 1, 00:00", "1970-01-01"]
        );
        let late = 1_794_999_999_999;
        assert_eq!(
            [clock(late, false), clock(late, true), stamp(late), day_of(late)],
            ["11:06", "11:06:39", "Nov 18, 11:06", "2026-11-18"]
        );
        assert_eq!(iso_string(1_794_986_200_000), "2026-11-18T07:16:40.000Z");
    }

    #[test]
    fn days_read_as_the_web_says_them() {
        assert_eq!(say_day("2026-11-18", "2026-11-18"), "Today");
        assert_eq!(say_day("2026-11-19", "2026-11-18"), "Tomorrow");
        assert_eq!(say_day("2026-11-20", "2026-11-18"), "Friday, Nov 20");
        assert_eq!(say_day("2027-01-01", "2026-12-31"), "Tomorrow");
        assert_eq!(say_day("2026-11-17", "2026-11-18"), "Tuesday, Nov 17");
        assert_eq!(weekday_of("2026-11-18"), "Wednesday");
        assert_eq!(weekday_of("nope"), "");
    }

    #[test]
    fn counts_and_starters_read_as_the_web_says_them() {
        assert_eq!(counted(1, "run"), "1 run");
        assert_eq!(counted(12, "run"), "12 runs");
        assert_eq!(counted(1_234_567, "row"), "1,234,567 rows");
        assert_eq!(counted_as(0, "person", "people"), "0 people");
        assert_eq!(say_by(By::Schedule, Trigger::Schedule), "schedule");
        assert_eq!(say_by(By::Person(Person::Ada), Trigger::Manual), "ada");
        assert_eq!(say_by(By::Person(Person::Grace), Trigger::Retry), "grace, retry");
    }

    #[test]
    fn rounds_and_writes_numbers_as_javascript() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(0.499_999_999_999_999_94), 0.0);
        assert_eq!(base36(1_794_992_400), "toox00");
        assert_eq!(base36(272), "7k");
        assert_eq!(thousands(236_979), "236,979");
    }
}
