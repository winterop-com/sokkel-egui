//! The indicators the server holds, as the web's `pages/Indicators.tsx` lists them: each checked
//! against the server's version and worked out for the whole country in the latest month,
//! searched and filtered by type and by how its check went, and a new one made.

use serde::Serialize;

use crate::backend::{Backend, INDICATOR_TYPES, Indicator, IndicatorType, Then, indicator_value};
use crate::check::{CheckContext, check};
use crate::expression::Language;
use crate::js::{base36, trim};
use crate::metadata::{de, latest_period, period_label, root, uid};
use crate::ui::{Verdict, count, counted, describe_items, figure, verdict};

/// An indicator as the table lists it: how its check went, its value for the country in the latest
/// month, and how many findings its two expressions have.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Worked {
    pub id: String,
    pub name: String,
    pub code: String,
    #[serde(rename = "type")]
    pub kind: IndicatorType,
    pub state: Verdict,
    pub value: Option<f64>,
    pub findings: usize,
    /// Its formula in words: the numerator over the denominator.
    pub formula: String,
    /// What its check chip says.
    pub check: String,
    /// Its value as the table writes it.
    pub shown: String,
}

/// Every indicator checked against `version` and worked out for the country in the latest month.
pub fn worked(rows: &[Indicator], version: &str, ids: &[String]) -> Vec<Worked> {
    let context = CheckContext {
        version: version.to_owned(),
        indicators: ids.to_vec(),
        ..CheckContext::default()
    };
    rows.iter()
        .map(|one| {
            let numerator = check(&one.numerator, Language::Expression, &context);
            let denominator = check(&one.denominator, Language::Expression, &context);
            let state = verdict(&[&numerator, &denominator]);
            let value = match (&numerator.node, &denominator.node) {
                (Some(top), Some(bottom)) if state != Verdict::Invalid => {
                    indicator_value(top, bottom, one.kind, root(), latest_period()).value
                }
                _ => None,
            };
            let findings = numerator.findings.len() + denominator.findings.len();
            Worked {
                id: one.id.clone(),
                name: one.name.clone(),
                code: one.code.clone(),
                kind: one.kind,
                state,
                value,
                findings,
                formula: format!(
                    "{} over {}",
                    describe_items(&one.numerator),
                    describe_items(&one.denominator)
                ),
                check: if findings == 0 {
                    "valid".to_owned()
                } else {
                    counted(findings, "finding")
                },
                shown: figure(value, one.kind.unit()),
            }
        })
        .collect()
}

/// The indicators the search and the filters let through.
pub fn shown<'a>(worked: &'a [Worked], query: &str, types: &[String], states: &[String]) -> Vec<&'a Worked> {
    let asked = trim(query).to_lowercase();
    worked
        .iter()
        .filter(|one| {
            (asked.is_empty()
                || format!("{} {} {}", one.name, one.code, one.id)
                    .to_lowercase()
                    .contains(&asked))
                && (types.is_empty() || types.iter().any(|kind| kind == one.kind.key()))
                && (states.is_empty() || states.iter().any(|state| state == one.state.key()))
        })
        .collect()
}

/// What the status line says.
pub fn foot(shown: usize, total: usize) -> String {
    format!("Showing {shown} of {}", counted(total, "indicator"))
}

/// What the table's foot says: how many cannot run on the version, when any cannot.
pub fn table_foot(worked: &[Worked], version: &str) -> Option<String> {
    let invalid = worked.iter().filter(|one| one.state == Verdict::Invalid).count();
    (invalid > 0).then(|| format!("{} cannot run on {version}.", count(invalid as f64, "indicator", None)))
}

/// What the table says when nothing is shown, when the search or a filter is the reason.
pub fn empty_words(query: &str, types: &[String], states: &[String]) -> Option<&'static str> {
    (!trim(query).is_empty() || !types.is_empty() || !states.is_empty())
        .then_some("No indicator matches the search and the filters.")
}

/// The header's description.
pub fn description(version: &str) -> String {
    format!(
        "A numerator over a denominator, each an expression over data elements, constants and reporting rates, times the type's factor. Each is checked against the server's version, {version}, and worked out for the country in the latest month."
    )
}

/// The value column's header: the country in the latest month.
pub fn value_header() -> String {
    format!("Norway, {}", period_label(latest_period()))
}

/// The type filter's options.
pub fn type_options() -> Vec<(&'static str, &'static str)> {
    INDICATOR_TYPES.iter().map(|one| (one.key(), one.label())).collect()
}

/// A new indicator, numbered after those there are, made and opened.
pub fn create(backend: &mut Backend, count: usize) {
    let made = format!("{:03}", count + 1);
    let code = format!("IND_NEW_{made}");
    let id = uid(&format!("ind:{code}:{}", base36(backend.now())));
    let row = Indicator {
        id: id.clone(),
        code,
        name: format!("New indicator {made}"),
        description: String::new(),
        kind: IndicatorType::Percent,
        numerator: format!("#{{{}}}", de().referrals.id),
        denominator: format!("#{{{}}}", de().visits.id),
    };
    backend.create_indicator(
        row,
        vec![
            Then::Go(format!("/indicators/{id}")),
            Then::Refused("Not created".to_owned()),
        ],
    );
}
