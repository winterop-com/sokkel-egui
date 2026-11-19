//! The validation rules the server holds, as the web's `pages/ValidationRules.tsx` lists them:
//! each checked against the server's version and run over every site and month, searched, and a
//! new one made.

use serde::Serialize;

use crate::backend::{Backend, Importance, MissingValueStrategy, Operator, Then, ValidationRule, run_rule};
use crate::check::{CheckContext, check};
use crate::expression::Language;
use crate::js::{base36, trim};
use crate::metadata::{de, periods, root, sites_under, uid};
use crate::ui::{Verdict, counted, describe_items, verdict};

/// Every site's uid, where the rules are run.
pub fn sites() -> Vec<String> {
    sites_under(root()).into_iter().map(|site| site.id.clone()).collect()
}

/// A rule as the table lists it: its importance, how its check went and how many violations it
/// finds over every site and month, none when it cannot run.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Worked {
    pub id: String,
    pub name: String,
    pub code: String,
    pub importance: Importance,
    pub state: Verdict,
    pub violations: Option<usize>,
    /// The rule in words: its two sides with the operator's sign between.
    pub rule: String,
}

/// Every rule checked against `version` and run over every site and month.
pub fn worked(rows: &[ValidationRule], version: &str, ids: &[String]) -> Vec<Worked> {
    let context = CheckContext {
        version: version.to_owned(),
        indicators: ids.to_vec(),
        ..CheckContext::default()
    };
    let sites = sites();
    rows.iter()
        .map(|one| {
            let left = check(&one.left, Language::Expression, &context);
            let right = check(&one.right, Language::Expression, &context);
            let state = verdict(&[&left, &right]);
            let violations = match (&left.node, &right.node) {
                (Some(a), Some(b)) if state != Verdict::Invalid => Some(
                    run_rule(
                        &one.id,
                        one.operator,
                        one.left_strategy,
                        one.right_strategy,
                        a,
                        b,
                        &sites,
                        periods(),
                    )
                    .violations
                    .len(),
                ),
                _ => None,
            };
            Worked {
                id: one.id.clone(),
                name: one.name.clone(),
                code: one.code.clone(),
                importance: one.importance,
                state,
                violations,
                rule: format!(
                    "{} {} {}",
                    describe_items(&one.left),
                    one.operator.sign(),
                    describe_items(&one.right)
                ),
            }
        })
        .collect()
}

/// The rules the search lets through.
pub fn shown<'a>(worked: &'a [Worked], query: &str) -> Vec<&'a Worked> {
    let asked = trim(query).to_lowercase();
    worked
        .iter()
        .filter(|one| {
            asked.is_empty()
                || format!("{} {} {}", one.name, one.code, one.id)
                    .to_lowercase()
                    .contains(&asked)
        })
        .collect()
}

/// What the status line says: every violation, across how many sites and months.
pub fn foot(worked: &[Worked]) -> String {
    let total: usize = worked.iter().map(|one| one.violations.unwrap_or(0)).sum();
    format!(
        "{} across {} and {}",
        counted(total, "violation"),
        counted(sites().len(), "site"),
        counted(periods().len(), "month")
    )
}

/// What the table says when the search finds nothing.
pub fn empty_words(query: &str) -> Option<&'static str> {
    (!trim(query).is_empty()).then_some("No rule matches the search.")
}

/// The header's description.
pub fn description(version: &str) -> String {
    format!(
        "A left side compared with a right side, each an expression over the data. Every rule is checked against the server's version, {version}, and run for each site and month the server holds."
    )
}

/// A new rule, numbered after those there are, made and opened.
pub fn create(backend: &mut Backend, count: usize) {
    let made = format!("{:03}", count + 1);
    let code = format!("VR_NEW_{made}");
    let id = uid(&format!("vr:{code}:{}", base36(backend.now())));
    let row = ValidationRule {
        id: id.clone(),
        code,
        name: format!("New rule {made}"),
        instruction: String::new(),
        importance: Importance::Medium,
        left: format!("#{{{}}}", de().referrals.id),
        operator: Operator::LessThanOrEqualTo,
        right: format!("#{{{}}}", de().visits.id),
        left_strategy: MissingValueStrategy::SkipIfAnyValueMissing,
        right_strategy: MissingValueStrategy::SkipIfAnyValueMissing,
    };
    backend.create_rule(
        row,
        vec![
            Then::Go(format!("/validation-rules/{id}")),
            Then::Refused("Not created".to_owned()),
        ],
    );
}
