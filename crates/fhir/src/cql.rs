//! CQL, as sokkel-ui-fhir's `cql.ts` reads it: a single-quoted text is a string, a double-quoted or
//! backticked one is a name, a name followed by `(` is a function, a name after `.` is a member
//! even when it is spelled like a keyword, and `@2026-01-01` is a date.

use sokkel_egui::CodeToken;
use sokkel_egui::plugins::PluginLanguage;

use crate::program::{Syntax, code_tokens, digits, quoted};

/// What `CodeLanguage::Plugin` calls the language.
pub const CQL: &str = "cql";

/// What `cql` is written as.
pub const CQL_MEDIA_TYPES: [&str; 3] = ["text/cql", "text/cql-expression", "application/cql"];

/// The words CQL's grammar spells, lower case as the language is.
const KEYWORDS: [&str; 118] = [
    "after",
    "aggregate",
    "all",
    "and",
    "as",
    "asc",
    "ascending",
    "before",
    "between",
    "by",
    "called",
    "case",
    "cast",
    "code",
    "codesystem",
    "codesystems",
    "collapse",
    "concept",
    "contains",
    "context",
    "convert",
    "date",
    "day",
    "days",
    "default",
    "define",
    "desc",
    "descending",
    "difference",
    "display",
    "distinct",
    "div",
    "duration",
    "during",
    "else",
    "end",
    "ends",
    "except",
    "exists",
    "expand",
    "external",
    "false",
    "flatten",
    "fluent",
    "from",
    "function",
    "hour",
    "hours",
    "if",
    "implies",
    "in",
    "include",
    "included",
    "includes",
    "intersect",
    "is",
    "less",
    "let",
    "library",
    "maximum",
    "meets",
    "millisecond",
    "milliseconds",
    "minimum",
    "minute",
    "minutes",
    "mod",
    "month",
    "months",
    "more",
    "not",
    "null",
    "occurs",
    "of",
    "on",
    "or",
    "overlaps",
    "parameter",
    "per",
    "point",
    "predecessor",
    "private",
    "properly",
    "public",
    "return",
    "returns",
    "same",
    "second",
    "seconds",
    "singleton",
    "sort",
    "start",
    "starting",
    "starts",
    "successor",
    "such",
    "than",
    "that",
    "then",
    "time",
    "timezoneoffset",
    "to",
    "true",
    "union",
    "using",
    "valueset",
    "version",
    "week",
    "weeks",
    "when",
    "where",
    "width",
    "with",
    "within",
    "without",
    "xor",
    "year",
    "years",
];

/// `!=`, `!~`, `<=`, `>=`, then one of `=~<>+-*/&|^`.
fn operator(s: &str) -> usize {
    if ["!=", "!~", "<=", ">="].iter().any(|one| s.starts_with(one)) {
        return 2;
    }
    usize::from(s.starts_with(['=', '~', '<', '>', '+', '-', '*', '/', '&', '|', '^']))
}

fn punctuation(s: &str) -> usize {
    usize::from(s.starts_with([',', ':']))
}

/// `12`, `4.5` or the long integer `100L`.
fn number(s: &str) -> usize {
    let whole = digits(s);
    if whole == 0 {
        return 0;
    }
    let rest = &s[whole..];
    if let Some(fraction) = rest.strip_prefix('.').map(digits)
        && fraction > 0
    {
        return whole + 1 + fraction;
    }
    whole + usize::from(rest.starts_with('L'))
}

/// A double-quoted or backticked name.
fn quoted_name(s: &str) -> usize {
    quoted(s, '"').or_else(|| quoted(s, '`')).unwrap_or(0)
}

/// CQL's spelling.
pub fn syntax() -> Syntax {
    Syntax {
        keywords: &KEYWORDS,
        operator,
        punctuation,
        number,
        quoted_name,
        binding: None,
    }
}

/// A document's tokens as the code editor colors them.
pub fn cql_tokens(text: &str) -> Vec<CodeToken> {
    code_tokens(text, &syntax())
}

/// CQL as a plugin's language: `CodeLanguage::Plugin("cql")`, written as `text/cql`,
/// `text/cql-expression` or `application/cql`.
pub fn cql_language() -> PluginLanguage {
    PluginLanguage::new(CQL, cql_tokens).media_types(CQL_MEDIA_TYPES)
}
