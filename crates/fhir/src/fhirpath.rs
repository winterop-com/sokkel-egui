//! FHIRPath, as sokkel-ui-fhir's `fhirpath.ts` reads it: a name followed by `(` is a function, a
//! name after `.` is a member even when it is spelled like a keyword, `%name` and `$this` are bound
//! from outside the expression, a backticked name is a name, a single-quoted text is a string, and
//! `@2026-01-01` is a date.

use sokkel_egui::CodeToken;
use sokkel_egui::plugins::PluginLanguage;

use crate::program::{Syntax, code_tokens};

/// What `CodeLanguage::Plugin` calls the language.
pub const FHIRPATH: &str = "fhirpath";

/// What `fhirpath` is written as.
pub const FHIRPATH_MEDIA_TYPES: [&str; 2] = ["text/fhirpath", "application/fhirpath"];

/// The words FHIRPath's grammar spells as operators and literals.
const KEYWORDS: [&str; 12] = [
    "and", "or", "xor", "implies", "is", "as", "div", "mod", "in", "contains", "true", "false",
];

/// `!=`, `!~`, `<=`, `>=`, then one of `=~<>+-*/&|`.
fn operator(s: &str) -> usize {
    if ["!=", "!~", "<=", ">="].iter().any(|one| s.starts_with(one)) {
        return 2;
    }
    usize::from(s.starts_with(['=', '~', '<', '>', '+', '-', '*', '/', '&', '|']))
}

fn punctuation(s: &str) -> usize {
    usize::from(s.starts_with(','))
}

/// `12` or `4.5`.
fn number(s: &str) -> usize {
    let whole = crate::program::digits(s);
    if whole == 0 {
        return 0;
    }
    let rest = &s[whole..];
    match rest.strip_prefix('.').map(crate::program::digits) {
        Some(fraction) if fraction > 0 => whole + 1 + fraction,
        _ => whole,
    }
}

/// A backticked name.
fn quoted_name(s: &str) -> usize {
    crate::program::quoted(s, '`').unwrap_or(0)
}

/// `%name`, ``%`name` ``, `%'name'`, or `$this`, `$index`, `$total` ending a word.
fn binding(s: &str) -> usize {
    if let Some(rest) = s.strip_prefix('%') {
        let name = crate::program::name_len(rest);
        let len = if name > 0 {
            name
        } else {
            crate::program::quoted(rest, '`')
                .or_else(|| crate::program::quoted(rest, '\''))
                .unwrap_or(0)
        };
        return if len > 0 { 1 + len } else { 0 };
    }
    if let Some(rest) = s.strip_prefix('$') {
        for word in ["this", "index", "total"] {
            if let Some(after) = rest.strip_prefix(word)
                && !after.chars().next().is_some_and(crate::program::word_char)
            {
                return 1 + word.len();
            }
        }
    }
    0
}

/// FHIRPath's spelling.
pub fn syntax() -> Syntax {
    Syntax {
        keywords: &KEYWORDS,
        operator,
        punctuation,
        number,
        quoted_name,
        binding: Some(binding),
    }
}

/// A document's tokens as the code editor colors them.
pub fn fhirpath_tokens(text: &str) -> Vec<CodeToken> {
    code_tokens(text, &syntax())
}

/// FHIRPath as a plugin's language: `CodeLanguage::Plugin("fhirpath")`, written as `text/fhirpath`
/// or `application/fhirpath`.
pub fn fhirpath_language() -> PluginLanguage {
    PluginLanguage::new(FHIRPATH, fhirpath_tokens).media_types(FHIRPATH_MEDIA_TYPES)
}
