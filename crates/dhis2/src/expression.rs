//! The expression language of indicators, validation rules and predictors, as sokkel-ui-dhis2's
//! `expression.ts` gives it: data items in braces (`#{dataElement.categoryOptionCombo.attributeOptionCombo}`
//! with `*` wildcards, `I{}`, `R{dataSet.REPORTING_RATE}`, `D{}`, `A{}`, `C{}`, `N{}`, `OUG{}`),
//! `[days]`, functions (`if`, `isNull`, `orgUnit.group`, `sum`, `.periodOffset(-1)`), operators,
//! numbers, strings, booleans and block comments. It knows the functions it is given that are not
//! program functions (`d2:`), the latest server's by default; any other name is read as a plain
//! name.

use sokkel_egui::CodeToken;
use sokkel_egui::plugins::PluginLanguage;

use crate::names::Dhis2Names;
use crate::tokens::{Known, code_tokens};

/// What `CodeLanguage::Plugin` calls the language.
pub const DHIS2_EXPRESSION: &str = "dhis2-expression";

/// What `dhis2-expression` is written as.
pub const EXPRESSION_MEDIA_TYPES: [&str; 2] = ["text/x-dhis2-expression", "application/x-dhis2-expression"];

fn known(names: &Dhis2Names) -> Known {
    Known::new(
        names.functions.iter().filter(|name| !name.starts_with("d2:")),
        std::iter::empty::<&str>(),
    )
}

/// A document's tokens in the expression language, knowing those names.
pub fn dhis2_expression_tokens(text: &str, names: &Dhis2Names) -> Vec<CodeToken> {
    code_tokens(text, &known(names))
}

/// The expression language as a plugin's language: `CodeLanguage::Plugin("dhis2-expression")`,
/// written as `text/x-dhis2-expression`.
pub fn dhis2_expression_language(names: &Dhis2Names) -> PluginLanguage {
    let known = known(names);
    PluginLanguage::new(DHIS2_EXPRESSION, move |text| code_tokens(text, &known)).media_types(EXPRESSION_MEDIA_TYPES)
}
