//! The dhis2 add-on's stories: DHIS2's two expression languages in the core's code editor.

use eframe::egui::Ui;
use sokkel_egui::plugins::register_plugins;
use sokkel_egui::{CodeEditor, CodeLanguage, CodeMark, CodeSize, Field, IssueTone, MarkTone};
use sokkel_egui_dhis2::{DHIS2_EXPRESSION, DHIS2_RULE, dhis2};

use super::{State, section};

/// A document kept between frames in the story's own memory.
fn kept(ui: &Ui, key: &str, first: &str) -> String {
    ui.data(|d| d.get_temp::<String>(eframe::egui::Id::unique(("story-dhis2", key))))
        .unwrap_or_else(|| first.to_owned())
}

fn keep(ui: &Ui, key: &str, text: String) {
    ui.data_mut(|d| d.insert_temp(eframe::egui::Id::unique(("story-dhis2", key)), text));
}

/// An indicator's numerator in the expression language and a program rule's condition in the rule
/// language, each colored by the add-on as the latest server names its functions and variables; the
/// numerator carries a finding on its data element, underlined and listed on the field's mark.
pub fn expression_editor(_: &mut State, ui: &mut Ui) {
    if sokkel_egui::plugins::plugin_language(ui.ctx(), DHIS2_EXPRESSION).is_none() {
        register_plugins(ui.ctx(), [dhis2()]);
    }
    ui.set_max_width(560.0);
    section(ui, "Expression: an indicator's numerator");
    let mut numerator = kept(
        ui,
        "numerator",
        "/* coverage in percent */\nif(isNull(#{HUck6sTd5p0}), 0, #{HUck6sTd5p0} / #{q6chUqwpCut.*} * 100)\n  + C{nYRkC6Bmk1e} - #{unknownDE01}.periodOffset(-1)",
    );
    let from = numerator.find("#{unknownDE01}").unwrap_or(0);
    let marks = [CodeMark {
        line: None,
        range: Some((from, from + "#{unknownDE01}".len())),
        path: None,
        tone: MarkTone::Error,
        message: Some("No data element has the uid unknownDE01".into()),
    }];
    Field::new("Numerator")
        .help("Reads as: {Children vaccinated} / {Children due for vaccination} * 100")
        .issues([(
            IssueTone::Critical,
            "Line 3: No data element has the uid unknownDE01".to_owned(),
        )])
        .show(ui, |ui| {
            CodeEditor::new("story-numerator", &mut numerator)
                .language(CodeLanguage::Plugin(DHIS2_EXPRESSION))
                .marks(&marks)
                .line_numbers(false)
                .size(CodeSize::Sm)
                .wrap(true)
                .invalid(true)
                .show(ui);
        });
    keep(ui, "numerator", numerator);
    ui.add_space(16.0);
    section(ui, "Rule: a program rule's condition");
    let mut condition = kept(
        ui,
        "condition",
        "d2:hasValue(#{water source}) && #{treated} == false\n  && d2:daysBetween(V{event_date}, V{current_date}) <= 30",
    );
    Field::new("Condition").show(ui, |ui| {
        CodeEditor::new("story-condition", &mut condition)
            .language(CodeLanguage::Plugin(DHIS2_RULE))
            .line_numbers(false)
            .size(CodeSize::Sm)
            .wrap(true)
            .show(ui);
    });
    keep(ui, "condition", condition);
}
