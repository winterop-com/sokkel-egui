//! The expression language of program rules and program indicators, as sokkel-ui-dhis2's
//! `rule.ts` gives it: program rule variables `#{name}` and `A{name}` (whose names may hold spaces,
//! dashes and dots), program data `#{stage.dataElement}`, program variables `V{event_date}`,
//! `PS_EVENTDATE:`, program functions (`d2:hasValue`, `d2:daysBetween`), the functions every
//! expression takes, operators, numbers, strings, booleans and block comments. It knows the
//! functions and variables it is given, the latest server's by default; any other name is read as
//! a plain name.

use sokkel_egui::CodeToken;
use sokkel_egui::plugins::PluginLanguage;

use crate::names::Dhis2Names;
use crate::tokens::{Known, code_tokens};

/// What `CodeLanguage::Plugin` calls the language.
pub const DHIS2_RULE: &str = "dhis2-rule";

/// What `dhis2-rule` is written as.
pub const RULE_MEDIA_TYPES: [&str; 2] = ["text/x-dhis2-rule", "application/x-dhis2-rule"];

fn known(names: &Dhis2Names) -> Known {
    Known::new(&names.functions, &names.variables)
}

/// A document's tokens in the program rule language, knowing those names.
pub fn dhis2_rule_tokens(text: &str, names: &Dhis2Names) -> Vec<CodeToken> {
    code_tokens(text, &known(names))
}

/// The program language as a plugin's language: `CodeLanguage::Plugin("dhis2-rule")`, written as
/// `text/x-dhis2-rule`.
pub fn dhis2_rule_language(names: &Dhis2Names) -> PluginLanguage {
    let known = known(names);
    PluginLanguage::new(DHIS2_RULE, move |text| code_tokens(text, &known)).media_types(RULE_MEDIA_TYPES)
}
