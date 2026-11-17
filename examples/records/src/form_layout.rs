//! The order a form asks its questions in, as the web's `pages/FormLayout.tsx`: its sections and
//! their questions in a tree, dragged into the order every later fill follows and kept on the server.

use std::collections::HashSet;

use eframe::egui::Ui;
use sokkel_egui::{
    ApiLink, Button, ButtonSize, ButtonVariant, Icon, PageHeader, PageState, Section, Toast, ToastKind, TreeNode,
    move_tree_node,
};

use crate::app::{Foot, RecordsApp};
use crate::backend::{LayoutSection, arranged};
use crate::forms::{Form, QuestionType, form_of, questions_of};
use crate::ui::count;

const SECTION: &str = "section:";
const QUESTION: &str = "question:";

fn type_words(kind: QuestionType) -> &'static str {
    match kind {
        QuestionType::Text => "Text",
        QuestionType::Multiline => "Long text",
        QuestionType::Integer => "Whole number",
        QuestionType::Decimal => "Number",
        QuestionType::Date => "Date",
        QuestionType::Time => "Time",
        QuestionType::YesNo => "Yes or no",
        QuestionType::Choice => "Choice",
        QuestionType::Computed => "Worked out",
    }
}

/// The form's sections as branches and their questions as rows, in the order it asks them.
fn nodes_of(form: &Form) -> Vec<TreeNode> {
    form.sections
        .iter()
        .map(|section| {
            TreeNode::new(format!("{SECTION}{}", section.id), section.title)
                .meta(count(section.questions.len(), "question"))
                .icon(Icon::Folder)
                .open_icon(Icon::FolderOpen)
                .children(
                    section
                        .questions
                        .iter()
                        .map(|q| {
                            TreeNode::new(format!("{QUESTION}{}", q.id), q.label)
                                .meta(type_words(q.kind))
                                .icon(Icon::FileText)
                        })
                        .collect(),
                )
        })
        .collect()
}

/// A tree of sections and questions read back as a layout.
fn layout_from(nodes: &[TreeNode]) -> Vec<LayoutSection> {
    nodes
        .iter()
        .map(|n| LayoutSection {
            id: n.id[SECTION.len()..].to_owned(),
            questions: n.children.iter().map(|c| c.id[QUESTION.len()..].to_owned()).collect(),
        })
        .collect()
}

impl RecordsApp {
    pub fn form_layout(&mut self, ui: &mut Ui, form_id: &str) {
        let served = form_of(form_id);
        let kept = self.backend.layout_kept(form_id).cloned();
        let shown = served.map(|f| arranged(f, kept.as_ref()));
        self.foot = Foot {
            note: shown.as_ref().map_or_else(String::new, |f| {
                format!(
                    "{} · {}",
                    count(f.sections.len(), "section"),
                    count(questions_of(f).count(), "question")
                )
            }),
            items: Vec::new(),
        };
        let (mut reset, mut fill) = (false, false);
        let answer = PageHeader::new("Layout")
            .code(form_id)
            .back(served.map_or("The form", |f| f.title))
            .description(
                "The order the form asks its questions in. Drag a question within its section or into another, or a section among the sections; from the keyboard, Alt (Option) and the arrows move the focused row. Every fill of the form follows the order kept here.",
            )
            .show(ui, |ui| {
                if served.is_none() {
                    return;
                }
                fill = ui
                    .add(Button::new("Fill in").icon(Icon::PenLine).size(ButtonSize::Sm))
                    .clicked();
                let mut button = Button::new("Reset")
                    .icon(Icon::RotateCcw)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Sm);
                if kept.is_none() {
                    button = button.reason("The form is in its own order.");
                }
                reset = ui.add(button).clicked();
                ui.add(ApiLink::new(format!("/forms/{form_id}/layout")));
            });
        match &shown {
            None => {
                PageState::missing(format!("The server serves no form called {form_id}.")).show(ui);
            }
            Some(form) if form.sections.is_empty() => {
                PageState::refused(
                    "The form is a grid of counts, laid out by its rows and cells; it has no sections to order.",
                )
                .show(ui);
            }
            Some(form) => {
                let nodes = nodes_of(form);
                let mut moved = None;
                Section::new(form.title)
                    .meta(if kept.is_none() {
                        "The form’s own order"
                    } else {
                        "Arranged"
                    })
                    .description("Sections in the order they are filled, each with its questions.")
                    .flush(true)
                    .show(ui, |ui| {
                        let mut open: HashSet<String> = ui
                            .data(|d| d.get_temp(egui_id(form_id)))
                            .unwrap_or_else(|| nodes.iter().map(|n| n.id.clone()).collect());
                        let mut none = None;
                        let can_move = |id: &str, into: Option<&str>| {
                            if id.starts_with(SECTION) {
                                into.is_none()
                            } else {
                                into.is_some_and(|i| i.starts_with(SECTION))
                            }
                        };
                        let out = sokkel_egui::Tree::new(("layout", form_id), &nodes, &mut none, &mut open)
                            .choosing(false)
                            .movable(&can_move)
                            .show(ui);
                        ui.data_mut(|d| d.insert_temp(egui_id(form_id), open));
                        moved = out.moved;
                    });
                if let Some((id, into, index)) = moved {
                    let next = layout_from(&move_tree_node(&nodes, &id, into.as_deref(), index));
                    if let Err(failure) = self.backend.save_layout(form_id, next) {
                        Toast::new("The order was not kept")
                            .kind(ToastKind::Error)
                            .description(failure.to_string())
                            .show(ui.ctx());
                    }
                }
            }
        }
        if reset {
            match self.backend.reset_layout(form_id) {
                Ok(()) => {
                    Toast::success("Back in the form’s own order").show(ui.ctx());
                }
                Err(failure) => {
                    Toast::new("Not reset")
                        .kind(ToastKind::Error)
                        .description(failure.to_string())
                        .show(ui.ctx());
                }
            }
        }
        if answer.back || fill {
            self.go(&format!("forms/{form_id}"));
        }
    }
}

fn egui_id(form: &str) -> eframe::egui::Id {
    eframe::egui::Id::unique(("layout-open", form))
}
