//! sokkel-egui's DHIS2 add-on, the counterpart of `@winterop-com/sokkel-ui-dhis2`: DHIS2's two
//! expression languages for the core's code editor, UI only. It never calls a server: what a
//! server version offers (its function and variable names) comes from the app as data.
//!
//! `dhis2-expression` is the language of indicators, validation rules and predictors;
//! `dhis2-rule` that of program rules and program indicators (`expression`, `rule`), both read
//! by one grammar (`tokens`). `names` lists the functions and program variables the latest server
//! knows. An app installs them with `register_plugins(ctx, [dhis2_plugin(&names)])` and draws a
//! `CodeEditor` with `CodeLanguage::Plugin(DHIS2_EXPRESSION)` or `CodeLanguage::Plugin(DHIS2_RULE)`;
//! installed again with another version's names, every open editor takes them.

pub mod expression;
pub mod names;
pub mod plugin;
pub mod rule;
pub mod tokens;

pub use expression::{DHIS2_EXPRESSION, EXPRESSION_MEDIA_TYPES, dhis2_expression_language, dhis2_expression_tokens};
pub use names::{DHIS2_FUNCTIONS, DHIS2_VARIABLES, Dhis2Names, dhis2_functions};
pub use plugin::{dhis2, dhis2_plugin};
pub use rule::{DHIS2_RULE, RULE_MEDIA_TYPES, dhis2_rule_language, dhis2_rule_tokens};
