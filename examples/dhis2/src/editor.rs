//! An expression in its field, as the web's `ExpressionField` in `ui.tsx` draws it: the dhis2
//! add-on's language colors it, the server's findings are underlined and listed on the field's
//! issue mark, words are offered as it is typed, uids are named on hover, and an aggregate
//! expression is read back with the names it stands for.

use eframe::egui::Ui;
use sokkel_egui::{CodeCompletion, CodeEditor, CodeLanguage, CodeMark, CodeSize, Field, IssueTone, MarkTone};
use sokkel_egui_dhis2::{DHIS2_EXPRESSION, DHIS2_RULE};

use crate::check::{Checked, Tone};
use crate::expression::Language;
use crate::ui::{completions, field_help, hover_word, issues_of, marks_of};

/// A byte offset into `text` for an offset counted in UTF-16 units, as the web's strings count.
pub fn byte_of(text: &str, units: usize) -> usize {
    let mut seen = 0;
    for (at, c) in text.char_indices() {
        if seen >= units {
            return at;
        }
        seen += c.len_utf16();
    }
    text.len()
}

/// The add-on's language an expression is written in.
pub fn language_of(language: Language) -> CodeLanguage {
    CodeLanguage::Plugin(match language {
        Language::Expression => DHIS2_EXPRESSION,
        Language::Rule => DHIS2_RULE,
    })
}

/// What an expression field shows and reads.
pub struct ExpressionField<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub language: Language,
    pub checked: &'a Checked,
    pub version: &'a str,
    /// The program's rule variables a rule's `#{` offers.
    pub variables: &'a [String],
    pub description: Option<&'a str>,
}

impl ExpressionField<'_> {
    /// Draws the field over `value`; true when it changed.
    pub fn show(self, ui: &mut Ui, value: &mut String) -> bool {
        let marks: Vec<CodeMark> = marks_of(self.checked)
            .into_iter()
            .map(|mark| CodeMark {
                line: None,
                range: Some((byte_of(value, mark.from), byte_of(value, mark.to))),
                path: None,
                tone: if mark.tone == Tone::Error {
                    MarkTone::Error
                } else {
                    MarkTone::Warning
                },
                message: Some(mark.message),
            })
            .collect();
        let issues = issues_of(value, self.checked).into_iter().map(|issue| {
            (
                if issue.tone == "critical" {
                    IssueTone::Critical
                } else {
                    IssueTone::Warning
                },
                issue.text,
            )
        });
        let invalid = self.checked.findings.iter().any(|one| one.tone == Tone::Error);
        let mut field = Field::new(self.label)
            .help_if(field_help(self.language, value))
            .issues(issues);
        if let Some(description) = self.description {
            field = field.description(description);
        }
        let (language, version, variables) = (self.language, self.version, self.variables);
        field.show(ui, |ui| {
            CodeEditor::new(self.id, value)
                .label(self.label)
                .language(language_of(language))
                .marks(&marks)
                .complete(move |_, before| {
                    completions(language, version, variables, before)
                        .into_iter()
                        .map(|one| CodeCompletion {
                            label: one.label,
                            kind: Some(one.kind.to_owned()),
                            detail: one.detail,
                            info: None,
                            apply: Some(one.apply),
                        })
                        .collect()
                })
                .hover(hover_word)
                .line_numbers(false)
                .size(CodeSize::Sm)
                .wrap(true)
                .invalid(invalid)
                .show(ui)
                .changed
        })
    }
}
