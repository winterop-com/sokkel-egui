//! Every function and program variable the latest server knows, as the web's `pages/Functions.tsx`
//! lists them: the version each came in, whether the running version knows it, and whether the
//! workbench's own evaluator runs it; searched and filtered.

use serde::Serialize;

use crate::expression::evaluated;
use crate::js::trim;
use crate::program::EVALUATED_VARIABLES;
use crate::ui::counted;
use crate::versions::{LATEST, VERSIONS, added, names_of};

/// A name as the table lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Name {
    pub id: String,
    pub name: String,
    /// `function` or `variable`.
    pub kind: &'static str,
    /// `rule` for the program rule language alone, `both` for either.
    pub languages: &'static str,
    pub since: String,
    pub known: bool,
    pub evaluated: bool,
}

/// Every name the latest server knows, as the running version sees it.
pub fn rows(version: &str) -> Vec<Name> {
    let known = names_of(version);
    let all = names_of(LATEST);
    let since = |name: &str| added(name).map_or_else(|| format!("{} or before", VERSIONS[0]), str::to_owned);
    let mut out: Vec<Name> = all
        .functions
        .iter()
        .map(|name| Name {
            id: format!("f:{name}"),
            name: (*name).to_owned(),
            kind: "function",
            languages: if name.starts_with("d2:") { "rule" } else { "both" },
            since: since(name),
            known: known.functions.contains(name),
            evaluated: evaluated(name).is_some(),
        })
        .collect();
    out.extend(all.variables.iter().map(|name| Name {
        id: format!("v:{name}"),
        name: format!("V{{{name}}}"),
        kind: "variable",
        languages: "rule",
        since: since(name),
        known: known.variables.contains(name),
        evaluated: EVALUATED_VARIABLES.contains(name),
    }));
    out
}

/// The names the search and the filters let through.
pub fn shown<'a>(rows: &'a [Name], query: &str, kinds: &[String], known: &[String]) -> Vec<&'a Name> {
    let asked = trim(query).to_lowercase();
    rows.iter()
        .filter(|one| {
            (asked.is_empty() || one.name.to_lowercase().contains(&asked))
                && (kinds.is_empty() || kinds.iter().any(|kind| kind == one.kind))
                && (known.is_empty()
                    || known
                        .iter()
                        .any(|state| state == if one.known { "known" } else { "unknown" }))
        })
        .collect()
}

/// What the status line says: how many of the names the version knows.
pub fn foot(version: &str, rows: &[Name]) -> String {
    let missing = rows.iter().filter(|one| !one.known).count();
    format!(
        "{version} knows {} of {}",
        rows.len() - missing,
        counted(rows.len(), "name")
    )
}

/// The header's description.
pub fn description(version: &str, rows: &[Name]) -> String {
    let missing = rows.iter().filter(|one| !one.known).count();
    format!(
        "The names DHIS2's expression languages know, as the server lists them. Switching the version hands the editors another list: on {version}, {} of the latest are read as plain names and checked as unknown. \"Run here\" says which the workbench's own evaluator runs.",
        counted(missing, "name")
    )
}

/// What the table says when nothing is shown.
pub const EMPTY: &str = "No name matches the search and the filters.";
