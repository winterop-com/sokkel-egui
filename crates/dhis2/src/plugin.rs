//! DHIS2's expression languages as a plugin, as sokkel-ui-dhis2's `plugin.ts` gives them.

use sokkel_egui::plugins::SokkelPlugin;

use crate::expression::dhis2_expression_language;
use crate::names::Dhis2Names;
use crate::rule::dhis2_rule_language;

/// DHIS2's expression languages as a plugin that knows the functions and program variables given:
/// a `CodeEditor` takes `CodeLanguage::Plugin("dhis2-expression")` (indicators, validation rules,
/// predictors) and `CodeLanguage::Plugin("dhis2-rule")` (program rules, program indicators), and
/// `language_for_media_type` reads `text/x-dhis2-expression` and `text/x-dhis2-rule` as them. An
/// app passes the names its server knows, from its own data layer.
pub fn dhis2_plugin(names: &Dhis2Names) -> SokkelPlugin {
    SokkelPlugin {
        name: "dhis2",
        languages: vec![dhis2_expression_language(names), dhis2_rule_language(names)],
    }
}

/// DHIS2's expression languages as a plugin, knowing the latest server's functions and variables.
pub fn dhis2() -> SokkelPlugin {
    dhis2_plugin(&Dhis2Names::default())
}
