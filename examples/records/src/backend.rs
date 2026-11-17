//! The simulated server, as the web's `data/backend.ts`: the responses it holds and where each is
//! now, the households it registered, the forms' layouts, and whether it answers at all.

use crate::days::{DAY, HOUR, civil, day_from, day_of, start_of};
use crate::forms::{
    AnswerValue, Answers, Form, FormKind, FormSection, answer_issue, form_of, forms, questions_of, test_answers,
    test_place,
};
use crate::geo::{COUNTRY, Level, place_of, places};

/// Where a response is now: received here, sent on and taken by the register, refused by it, or
/// taken back out of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReceiptState {
    Received,
    Forwarded,
    Rejected,
    Withdrawn,
}

pub const STATES: [ReceiptState; 4] = [
    ReceiptState::Received,
    ReceiptState::Forwarded,
    ReceiptState::Rejected,
    ReceiptState::Withdrawn,
];

impl ReceiptState {
    pub fn key(self) -> &'static str {
        match self {
            ReceiptState::Received => "received",
            ReceiptState::Forwarded => "forwarded",
            ReceiptState::Rejected => "rejected",
            ReceiptState::Withdrawn => "withdrawn",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ReceiptState::Received => "Received",
            ReceiptState::Forwarded => "Forwarded",
            ReceiptState::Rejected => "Rejected",
            ReceiptState::Withdrawn => "Withdrawn",
        }
    }

    pub fn note(self) -> &'static str {
        match self {
            ReceiptState::Received => "not yet sent on to the register",
            ReceiptState::Forwarded => "taken by the register",
            ReceiptState::Rejected => "refused by the register",
            ReceiptState::Withdrawn => "taken back out of the register",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        STATES.into_iter().find(|s| s.key() == key)
    }
}

pub const PEOPLE: [&str; 4] = ["ada", "grace", "linus", "margaret"];

/// Who is using the app.
pub const ME: &str = "grace";

#[derive(Clone, Debug, PartialEq)]
pub struct HistoryEntry {
    pub at: i64,
    pub state: ReceiptState,
    pub by: String,
    pub note: Option<String>,
}

/// One response as the server stored it: what it answers, for where and when, and where it is now.
#[derive(Clone, Debug, PartialEq)]
pub struct Receipt {
    pub id: String,
    /// The id the server gave the response: 32 hex digits, which the address and the API name it by.
    pub receipt: String,
    pub form_id: String,
    pub received_at: i64,
    pub state: ReceiptState,
    pub place: String,
    /// The period a periodic report is for.
    pub period: Option<String>,
    /// The day an event or a visit happened, `YYYY-MM-DD`.
    pub visit_date: Option<String>,
    /// The registered household a registration or a follow-up is about.
    pub household_id: Option<String>,
    pub answers: Answers,
    pub by: String,
    /// The seed test data was made from, when it was.
    pub seed: Option<u32>,
    /// Why the register refused it.
    pub refusal: Option<String>,
    pub history: Vec<HistoryEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HouseholdStatus {
    Active,
    Completed,
}

/// A registered household, named by the identifier the server gave it.
#[derive(Clone, Debug, PartialEq)]
pub struct Household {
    pub id: String,
    pub identifier: String,
    pub region: String,
    pub site: String,
    pub born: i64,
    pub size: i64,
    pub source: String,
    pub treats: Option<bool>,
    pub registered_at: i64,
    pub status: HouseholdStatus,
}

/// The instant `offset` days and `hour` hours from today's local midnight.
fn instant(now: i64, offset: i64, hour: i64, minute: i64) -> i64 {
    start_of(day_of(now) + offset) + hour * HOUR + minute * 60_000
}

/// A receipt id: 32 hex digits from a seed, the same every time.
pub fn receipt_id(seed: u32) -> String {
    let mut out = String::new();
    let mut state = (u64::from(seed) * 2_654_435_761) as u32;
    while out.len() < 32 {
        state = (state ^ (state >> 15))
            .wrapping_mul(2_246_822_507)
            .wrapping_add(3_266_489_909);
        out.push_str(&format!("{state:08x}"));
    }
    out.truncate(32);
    out
}

/// A household identifier: `H-` and two groups of four digits.
fn household_identifier(seed: u64) -> String {
    let digits = format!("{:08}", 10_000_000 + (seed * 7_919) % 89_999_999);
    format!("H-{}-{}", &digits[..4], &digits[4..8])
}

/// A seeded household: region, site, birth year, size, source, treats, days ago, status.
type SeedHousehold = (
    &'static str,
    &'static str,
    i64,
    i64,
    &'static str,
    Option<bool>,
    i64,
    HouseholdStatus,
);

fn seed_households(now: i64) -> Vec<Household> {
    let rows: [SeedHousehold; 7] = [
        (
            "bo",
            "bo/baomahun",
            1985,
            4,
            "well",
            Some(true),
            -40,
            HouseholdStatus::Active,
        ),
        (
            "bo",
            "bo/bumpe",
            1991,
            6,
            "river",
            Some(false),
            -38,
            HouseholdStatus::Active,
        ),
        (
            "kenema",
            "kenema/baama",
            1978,
            3,
            "standpipe",
            None,
            -35,
            HouseholdStatus::Completed,
        ),
        (
            "kono",
            "kono/baiama",
            1969,
            8,
            "well",
            Some(true),
            -30,
            HouseholdStatus::Active,
        ),
        (
            "bombali",
            "bombali/kamabai",
            1994,
            2,
            "rain",
            Some(false),
            -21,
            HouseholdStatus::Active,
        ),
        (
            "port-loko",
            "port-loko/lunsar",
            1988,
            5,
            "tap",
            Some(true),
            -12,
            HouseholdStatus::Active,
        ),
        // the first household registered a second time by mistake, which the metadata check finds
        (
            "bo",
            "bo/bumpe",
            1985,
            4,
            "well",
            Some(true),
            -6,
            HouseholdStatus::Active,
        ),
    ];
    rows.into_iter()
        .enumerate()
        .map(|(index, (region, site, born, size, source, treats, offset, status))| {
            let site = if place_of(site).is_some() {
                site.to_owned()
            } else {
                places()
                    .iter()
                    .find(|p| p.parent.as_deref() == Some(region))
                    .map_or_else(|| region.to_owned(), |p| p.id.clone())
            };
            Household {
                id: format!("household-{}", index + 1),
                identifier: household_identifier(if index == 6 { 11 } else { index as u64 + 11 }),
                region: region.into(),
                site,
                born,
                size,
                source: source.into(),
                treats,
                registered_at: instant(now, offset, 10, 0),
                status,
            }
        })
        .collect()
}

/// `YYYYMM` of the month `back` months before today's.
fn previous_month(now: i64, back: i64) -> String {
    let (year, month, _) = civil(day_of(now));
    let (_, m, _) = civil(day_from(year, month - back, 1));
    let (y, _, _) = civil(day_from(year, month - back, 1));
    format!("{y}{m:02}")
}

fn previous_quarter(now: i64) -> String {
    let (year, month, _) = civil(day_of(now));
    let quarter = (month - 1) / 3;
    if quarter == 0 {
        format!("{}Q4", year - 1)
    } else {
        format!("{year}Q{quarter}")
    }
}

fn seed_receipts(now: i64, households: &[Household]) -> Vec<Receipt> {
    struct Seed {
        form: &'static str,
        offset: i64,
        state: ReceiptState,
        by: &'static str,
        seed: u32,
        household: Option<usize>,
        refusal: Option<&'static str>,
        place: Option<&'static str>,
    }
    let row = |form, offset, state, by, seed| Seed {
        form,
        offset,
        state,
        by,
        seed,
        household: None,
        refusal: None,
        place: None,
    };
    use ReceiptState::*;
    let mut rows = vec![
        row("FRM-SITE", 0, Received, "grace", 9104),
        row("FRM-SITE", 0, Received, "linus", 9105),
        Seed {
            household: Some(0),
            ..row("FRM-HOME", -1, Received, "margaret", 9106)
        },
        Seed {
            place: Some("bo/baomahun"),
            ..row("FRM-MONTH", -2, Forwarded, "ada", 9101)
        },
        row("FRM-SAMPLE", -2, Forwarded, "grace", 9107),
        Seed {
            refusal: Some("The register takes a water level over 5 m only with a photo of the gauge."),
            ..row("FRM-SITE", -3, Rejected, "linus", 9108)
        },
        Seed {
            household: Some(1),
            ..row("FRM-HOME", -4, Forwarded, "margaret", 9109)
        },
        Seed {
            place: Some("kenema"),
            ..row("FRM-STOCK", -5, Forwarded, "ada", 9110)
        },
        row("FRM-SITE", -6, Withdrawn, "grace", 9111),
        Seed {
            household: Some(0),
            ..row("FRM-HOME", -8, Forwarded, "margaret", 9112)
        },
        Seed {
            household: Some(2),
            ..row("FRM-EXIT", -9, Forwarded, "linus", 9113)
        },
        row("FRM-SAMPLE", -11, Forwarded, "ada", 9114),
        Seed {
            place: Some("kono/baiama"),
            ..row("FRM-MONTH", -33, Forwarded, "grace", 9115)
        },
    ];
    for (index, household) in households.iter().enumerate() {
        let offset = ((household.registered_at - now) as f64 / DAY as f64 + 0.5).floor() as i64;
        rows.push(Seed {
            household: Some(index),
            ..row(
                "FRM-ENROLL",
                offset,
                Forwarded,
                PEOPLE[index % PEOPLE.len()],
                9200 + index as u32,
            )
        });
    }
    let mut out: Vec<Receipt> = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let form = form_of(row.form).expect("a seeded form");
            let household = row.household.map(|h| &households[h]);
            let mut answers = test_answers(form, row.seed);
            if row.form == "FRM-SITE" && row.state == Rejected {
                answers.set("level", Some(AnswerValue::Number(6.4)));
            }
            if let (Some(h), "FRM-ENROLL") = (household, row.form) {
                answers.set("born", Some(AnswerValue::Number(h.born as f64)));
                answers.set("size", Some(AnswerValue::Number(h.size as f64)));
                answers.set("source", Some(AnswerValue::Text(h.source.clone())));
                answers.set("treats", h.treats.map(AnswerValue::Bool));
            }
            let at = instant(now, row.offset, 9 + (index as i64 % 8), (index as i64 * 13) % 60);
            let place = row
                .place
                .map(str::to_owned)
                .or_else(|| household.map(|h| h.site.clone()))
                .unwrap_or_else(|| test_place(row.seed));
            let mut history = vec![HistoryEntry {
                at,
                state: Received,
                by: row.by.into(),
                note: None,
            }];
            if row.state != Received {
                history.push(HistoryEntry {
                    at: instant(now, row.offset, 18, 0),
                    state: if row.state == Withdrawn { Forwarded } else { row.state },
                    by: "server".into(),
                    note: row.refusal.map(str::to_owned),
                });
            }
            if row.state == Withdrawn {
                history.push(HistoryEntry {
                    at: instant(now, row.offset + 1, 9, 0),
                    state: Withdrawn,
                    by: "ada".into(),
                    note: Some("Filed twice by mistake.".into()),
                });
            }
            let period = match form.period {
                Some(sokkel_egui::periods::PeriodType::Monthly) => {
                    Some(previous_month(now, if row.offset < -30 { 2 } else { 1 }))
                }
                Some(sokkel_egui::periods::PeriodType::Quarterly) => Some(previous_quarter(now)),
                _ => None,
            };
            Receipt {
                id: format!("receipt-{}", index + 1),
                receipt: receipt_id(row.seed),
                form_id: row.form.into(),
                received_at: at,
                state: row.state,
                place,
                period,
                visit_date: (form.kind != FormKind::Periodic).then(|| iso_utc_day(at)),
                household_id: household.map(|h| h.id.clone()),
                answers,
                by: row.by.into(),
                seed: Some(row.seed),
                refusal: row.refusal.map(str::to_owned),
                history,
            }
        })
        .collect();
    out.sort_by_key(|r| std::cmp::Reverse(r.received_at));
    out
}

/// The UTC day of a moment, as `toISOString().slice(0, 10)` writes it.
pub fn iso_utc_day(at: i64) -> String {
    crate::days::iso_day(at.div_euclid(DAY))
}

/// A moment as `toISOString` writes it: `2026-11-18T09:00:00.000Z`.
pub fn iso_moment(at: i64) -> String {
    let ms = at.rem_euclid(DAY);
    format!(
        "{}T{:02}:{:02}:{:02}.{:03}Z",
        iso_utc_day(at),
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

pub struct ServerInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub base: &'static str,
    pub address: &'static str,
}

pub const SERVER: ServerInfo = ServerInfo {
    name: "Tidevann records",
    version: "2.4.0",
    base: "https://records.example.org/api",
    address: "records.example.org",
};

/// One section of a form's layout: the section, and its questions in the order they are asked.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutSection {
    pub id: String,
    pub questions: Vec<String>,
}

/// What the app files: a form's answers, for where, when and whom.
#[derive(Clone, Debug, Default)]
pub struct Submission {
    pub form_id: String,
    pub place: Option<String>,
    pub period: Option<String>,
    pub visit_date: Option<String>,
    pub household_id: Option<String>,
    pub answers: Answers,
    pub seed: Option<u32>,
}

/// One thing the server refuses in a submission: the question, and what is wrong.
#[derive(Clone, Debug, PartialEq)]
pub struct Issue {
    pub question: String,
    pub text: String,
}

/// Why a request failed: the server did not answer, or refused what was sent.
#[derive(Clone, Debug, PartialEq)]
pub enum Failure {
    Unreachable,
    Refused(Vec<Issue>),
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Unreachable => f.write_str("The server is not answering."),
            Failure::Refused(issues) => write!(
                f,
                "The server refused the response: {}",
                issues.iter().map(|i| i.text.as_str()).collect::<Vec<_>>().join(" ")
            ),
        }
    }
}

/// The server's state: everything it holds, and whether it answers.
#[derive(Clone, Debug)]
pub struct Backend {
    pub receipts: Vec<Receipt>,
    pub households: Vec<Household>,
    /// Forms' layouts by form id.
    pub layouts: Vec<(String, Vec<LayoutSection>)>,
    /// Form ids in the order Start a record lists them.
    pub form_order: Vec<String>,
    /// Whether the demo server has been told not to answer.
    pub down: bool,
    made: u32,
    households_made: u64,
}

impl Backend {
    pub fn seeded(now: i64) -> Self {
        let households = seed_households(now);
        let receipts = seed_receipts(now, &households);
        Self {
            receipts,
            households,
            layouts: Vec::new(),
            form_order: Vec::new(),
            down: false,
            made: (now % 100_000) as u32,
            households_made: 0,
        }
    }

    fn guard(&self) -> Result<(), Failure> {
        if self.down { Err(Failure::Unreachable) } else { Ok(()) }
    }

    /// Checks a submission as the server does, answering with each issue.
    pub fn check(submission: &Submission) -> Vec<Issue> {
        let issue = |question: &str, text: String| Issue {
            question: question.into(),
            text,
        };
        let Some(form) = form_of(&submission.form_id) else {
            return vec![issue("form", format!("No form is called {}.", submission.form_id))];
        };
        let mut issues = Vec::new();
        if submission.place.as_deref().and_then(place_of).is_none() {
            issues.push(issue("place", "Say where the record is from.".into()));
        }
        if form.period.is_some() && submission.period.is_none() {
            issues.push(issue("period", "Say which period the report is for.".into()));
        }
        if matches!(form.kind, FormKind::Followup | FormKind::Event) && submission.visit_date.is_none() {
            issues.push(issue("visitDate", "Say when the visit was.".into()));
        }
        if form.kind == FormKind::Followup && submission.household_id.is_none() {
            issues.push(issue("household", "Say which household the visit was to.".into()));
        }
        for question in questions_of(form) {
            if let Some(text) = answer_issue(question, submission.answers.get(question.id)) {
                issues.push(issue(question.id, format!("{}: {text}", question.label)));
            }
        }
        issues
    }

    /// Files a response, and for a registration the household it registers; answers with the
    /// receipt's id.
    pub fn submit(&mut self, submission: Submission, now: i64) -> Result<String, Failure> {
        self.guard()?;
        let issues = Self::check(&submission);
        if !issues.is_empty() {
            return Err(Failure::Refused(issues));
        }
        let form = form_of(&submission.form_id).expect("checked");
        let mut household_id = submission.household_id.clone();
        if form.kind == FormKind::Registration {
            let place = submission.place.as_deref().and_then(place_of);
            self.households_made += 1;
            let number = self.households.len() as u64 + 11 + (now as u64 % 997) + self.households_made;
            let id = format!(
                "household-{}",
                self.households.len() + 1 + self.households_made as usize
            );
            let answer = |key: &str| submission.answers.number(key).unwrap_or(0.0) as i64;
            self.households.push(Household {
                id: id.clone(),
                identifier: household_identifier(number),
                region: match place {
                    Some(p) if p.level == Level::Site => p.parent.clone().unwrap_or_else(|| COUNTRY.into()),
                    Some(p) => p.id.clone(),
                    None => COUNTRY.into(),
                },
                site: place.map_or_else(|| COUNTRY.into(), |p| p.id.clone()),
                born: answer("born"),
                size: answer("size"),
                source: submission
                    .answers
                    .get("source")
                    .map(AnswerValue::written)
                    .unwrap_or_default(),
                treats: match submission.answers.get("treats") {
                    Some(AnswerValue::Bool(b)) => Some(*b),
                    _ => None,
                },
                registered_at: now,
                status: HouseholdStatus::Active,
            });
            household_id = Some(id);
        }
        if form.id == "FRM-EXIT"
            && let Some(id) = &household_id
            && let Some(h) = self.households.iter_mut().find(|h| h.id == *id)
        {
            h.status = HouseholdStatus::Completed;
        }
        self.made += 1;
        let code = receipt_id(self.made);
        let receipt = Receipt {
            id: format!("receipt-{}", self.receipts.len() + 1 + self.made as usize),
            receipt: code.clone(),
            form_id: submission.form_id,
            received_at: now,
            state: ReceiptState::Received,
            place: submission.place.unwrap_or_else(|| COUNTRY.into()),
            period: submission.period,
            visit_date: submission.visit_date,
            household_id,
            answers: submission.answers,
            by: ME.into(),
            seed: submission.seed,
            refusal: None,
            history: vec![HistoryEntry {
                at: now,
                state: ReceiptState::Received,
                by: ME.into(),
                note: None,
            }],
        };
        self.receipts.insert(0, receipt);
        Ok(code)
    }

    /// Why the register refuses a response, or `None` when it takes it.
    pub fn register_refusal(receipt: &Receipt) -> Option<String> {
        if receipt.answers.number("level").is_some_and(|l| l > 5.0) {
            return Some("The register takes a water level over 5 m only with a photo of the gauge.".into());
        }
        if receipt.answers.number("stored").is_some_and(|s| s > 1500.0) {
            return Some("The register holds at most 1,500 liters stored in one home; check the count.".into());
        }
        None
    }

    /// Sends a received or refused response on to the register, which takes or refuses it.
    pub fn forward(&mut self, receipt: &str, now: i64) -> Result<ReceiptState, Failure> {
        self.guard()?;
        let Some(r) = self.receipts.iter_mut().find(|r| r.receipt == receipt) else {
            return Err(Failure::Unreachable);
        };
        let refusal = Self::register_refusal(r);
        let state = if refusal.is_none() {
            ReceiptState::Forwarded
        } else {
            ReceiptState::Rejected
        };
        r.state = state;
        r.refusal = refusal.clone();
        r.history.push(HistoryEntry {
            at: now,
            state,
            by: "server".into(),
            note: refusal,
        });
        Ok(state)
    }

    /// Sends every received response on, answering with how many were taken and refused.
    pub fn forward_all(&mut self, now: i64) -> Result<(usize, usize), Failure> {
        self.guard()?;
        let waiting: Vec<String> = self
            .receipts
            .iter()
            .filter(|r| r.state == ReceiptState::Received)
            .map(|r| r.receipt.clone())
            .collect();
        let mut refused = 0;
        for code in &waiting {
            if self.forward(code, now)? == ReceiptState::Rejected {
                refused += 1;
            }
        }
        Ok((waiting.len() - refused, refused))
    }

    /// Takes a forwarded response back out of the register.
    pub fn withdraw(&mut self, receipt: &str, note: &str, now: i64) -> Result<(), Failure> {
        self.guard()?;
        if let Some(r) = self.receipts.iter_mut().find(|r| r.receipt == receipt) {
            r.state = ReceiptState::Withdrawn;
            r.history.push(HistoryEntry {
                at: now,
                state: ReceiptState::Withdrawn,
                by: ME.into(),
                note: (!note.trim().is_empty()).then(|| note.trim().to_owned()),
            });
        }
        Ok(())
    }

    /// Takes a household out of the register; its records stay.
    pub fn remove_household(&mut self, id: &str) -> Result<(), Failure> {
        self.guard()?;
        self.households.retain(|h| h.id != id);
        Ok(())
    }

    pub fn receipt_of(&self, code: &str) -> Option<&Receipt> {
        self.receipts.iter().find(|r| r.receipt == code)
    }

    /// The receipts of a household, newest first.
    pub fn events_of(&self, household: &str) -> Vec<&Receipt> {
        let mut out: Vec<&Receipt> = self
            .receipts
            .iter()
            .filter(|r| r.household_id.as_deref() == Some(household))
            .collect();
        out.sort_by_key(|r| std::cmp::Reverse(r.received_at));
        out
    }

    pub fn layout_kept(&self, form: &str) -> Option<&Vec<LayoutSection>> {
        self.layouts.iter().find(|(id, _)| id == form).map(|(_, l)| l)
    }

    /// Keeps a form's layout on the server, the order every later fill asks it in.
    pub fn save_layout(&mut self, form: &str, sections: Vec<LayoutSection>) -> Result<(), Failure> {
        self.guard()?;
        match self.layouts.iter_mut().find(|(id, _)| id == form) {
            Some((_, kept)) => *kept = sections,
            None => self.layouts.push((form.to_owned(), sections)),
        }
        Ok(())
    }

    /// Drops a form's layout, so it asks in the order the form was made with.
    pub fn reset_layout(&mut self, form: &str) -> Result<(), Failure> {
        self.guard()?;
        self.layouts.retain(|(id, _)| id != form);
        Ok(())
    }

    /// The forms in the person's order.
    pub fn in_order(&self) -> Vec<&'static Form> {
        let rank = |id: &str| {
            self.form_order
                .iter()
                .position(|o| o == id)
                .unwrap_or(self.form_order.len())
        };
        let mut all: Vec<(usize, &'static Form)> = forms().iter().enumerate().collect();
        all.sort_by_key(|(at, f)| (rank(f.id), *at));
        all.into_iter().map(|(_, f)| f).collect()
    }
}

/// A form's sections and questions in the order it asks them now.
pub fn layout_of(form: &Form) -> Vec<LayoutSection> {
    form.sections
        .iter()
        .map(|s| LayoutSection {
            id: s.id.into(),
            questions: s.questions.iter().map(|q| q.id.to_owned()).collect(),
        })
        .collect()
}

/// The form with its sections and questions in the layout's order. A section or a question the
/// layout does not place keeps its place in the form's own order, so a form changed since stays
/// whole.
pub fn arranged(form: &Form, layout: Option<&Vec<LayoutSection>>) -> Form {
    let Some(layout) = layout else { return form.clone() };
    let mut placed: Vec<&str> = Vec::new();
    let mut sections: Vec<FormSection> = Vec::new();
    for entry in layout {
        let Some(section) = form.sections.iter().find(|s| s.id == entry.id) else {
            continue;
        };
        if sections.iter().any(|s| s.id == section.id) {
            continue;
        }
        let mut asked = Vec::new();
        for id in &entry.questions {
            if placed.contains(&id.as_str()) {
                continue;
            }
            if let Some(q) = questions_of(form).find(|q| q.id == id) {
                placed.push(q.id);
                asked.push(q.clone());
            }
        }
        sections.push(FormSection {
            questions: asked,
            ..section.clone()
        });
    }
    for section in &form.sections {
        if !sections.iter().any(|s| s.id == section.id) {
            sections.push(FormSection {
                questions: Vec::new(),
                ..section.clone()
            });
        }
    }
    for section in &form.sections {
        for question in &section.questions {
            if placed.contains(&question.id) {
                continue;
            }
            if let Some(home) = sections.iter_mut().find(|s| s.id == section.id) {
                home.questions.push(question.clone());
            }
        }
    }
    Form {
        sections,
        ..form.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seed_reads_as_the_web_seeds_it() {
        // 2026-11-18 09:00 UTC, the parity shots' moment
        let backend = Backend::seeded(1_794_992_400_000);
        assert_eq!(backend.receipts.len(), 20);
        let count = |s| backend.receipts.iter().filter(|r| r.state == s).count();
        assert_eq!(
            [
                count(ReceiptState::Received),
                count(ReceiptState::Forwarded),
                count(ReceiptState::Rejected),
                count(ReceiptState::Withdrawn)
            ],
            [3, 15, 1, 1]
        );
        assert_eq!(receipt_id(9104).len(), 32);
        assert_eq!(backend.households[0].identifier.len(), 11);
    }
}

#[cfg(test)]
mod ids {
    #[test]
    fn receipt_ids_are_the_webs() {
        // from the web's `receiptId`, run with bun
        assert_eq!(super::receipt_id(9104), "7f3d438ab4b07185d993c7ecf06aad0e");
        assert_eq!(super::receipt_id(9101), "9832602a1b1ccacffb3c8507bd2d5fdf");
    }
}
