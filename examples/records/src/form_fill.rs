//! A form opened to fill in, as the web's `pages/FormFill.tsx`: where and when it is for, its
//! sections of questions or its grid of counts, and Submit in the bar at its foot.

use std::collections::HashMap;

use eframe::egui::{self, Ui};
use sokkel_egui::{
    ApiLink, Button, ButtonSize, ButtonVariant, Columns, ComputedField, DateField, Day, Empty, Field, FormBar, Icon,
    Input, IssueCount, IssueEntry, NumberField, PageHeader, PeriodPicker, RepeatedGroup, RepeatedGroupWords, Section,
    Select, TimeField, Toast, ToastKind, TreeNode, TreeSelect, YesNoField,
};

use crate::app::{Foot, RecordsApp};
use crate::backend::{Failure, Submission, arranged};
use crate::days::day_of;
use crate::forms::{
    AnswerValue, Answers, FillContext, Form, FormKind, Question, QuestionType, answer_issue, form_of, is_answered,
    last_whole_period, question_count, questions_of, range_hint, test_answers, test_place,
};
use crate::geo::{COUNTRY, COUNTRY_NAME, Level, children_of, path_of, place_of};
use crate::ui::{count, count_as};

/// What a form being filled holds, kept while its page is open.
#[derive(Clone, Debug, Default)]
pub struct Fill {
    pub form: String,
    pub household_given: Option<String>,
    pub answers: Answers,
    pub place: Option<String>,
    pub period: Option<String>,
    pub period_issue: Option<String>,
    pub visit_date: Option<Day>,
    pub household: Option<String>,
    pub server_issues: HashMap<String, String>,
    pub tried: bool,
    pub seed_text: String,
    pub seed: Option<u32>,
    /// Counts each fill and clear, so a repeated question's rows start again from the answers set.
    pub round: u32,
}

/// Where a kind of form is filed from, said under the place's label.
fn place_meaning(kind: FormKind) -> &'static str {
    match kind {
        FormKind::Periodic => "The place the counts are for. The report is filed under it and its period.",
        FormKind::Event => "The site the visit was made at. The record is filed under it.",
        FormKind::Registration => "Where the household lives. Its visits are filed under the same place.",
        FormKind::Followup => "The site the visit was made from, usually where the household was registered.",
    }
}

/// The hierarchy as tree nodes, each with its level beside its name.
pub fn place_tree() -> Vec<TreeNode> {
    vec![
        TreeNode::new(COUNTRY, COUNTRY_NAME)
            .meta(Level::Country.label())
            .children(
                children_of(COUNTRY)
                    .into_iter()
                    .map(|region| {
                        TreeNode::new(&region.id, &region.name)
                            .meta(Level::Region.label())
                            .children(
                                children_of(&region.id)
                                    .into_iter()
                                    .map(|site| TreeNode::new(&site.id, &site.name).meta(Level::Site.label()))
                                    .collect(),
                            )
                    })
                    .collect(),
            ),
    ]
}

/// Why a place cannot be chosen for a form assigned to other levels.
fn outside(form: &Form, id: &str) -> Option<String> {
    let level = place_of(id)?.level;
    if form.levels.is_empty() || form.levels.contains(&level) {
        return None;
    }
    let levels: Vec<String> = form.levels.iter().map(|l| l.label().to_lowercase()).collect();
    Some(format!("Filed per {}", levels.join(" or ")))
}

/// One thing that stops a submit: the field, its label, and what is wrong.
struct Issue {
    id: String,
    label: String,
    text: String,
}

impl RecordsApp {
    /// The fill of a form, started afresh when the page is for another form or household.
    fn fill_for(&mut self, form: &Form, household: Option<&str>) -> Fill {
        if let Some(fill) = self.fill.take()
            && fill.form == form.id
            && fill.household_given.as_deref() == household
        {
            return fill;
        }
        let today = Day(day_of(self.now()) as i32);
        let start_place = household.is_none()
            && self.keep_place
            && self
                .last_place
                .as_deref()
                .is_some_and(|p| place_of(p).is_some() && outside(form, p).is_none());
        let household_place = household
            .and_then(|h| self.backend.households.iter().find(|x| x.id == h))
            .map(|h| h.site.clone());
        Fill {
            form: form.id.into(),
            household_given: household.map(str::to_owned),
            place: if start_place {
                self.last_place.clone()
            } else {
                household_place
            },
            period: form.period.map(|p| last_whole_period(p, today)),
            visit_date: (form.kind != FormKind::Periodic).then_some(today),
            household: household.map(str::to_owned),
            ..Fill::default()
        }
    }

    pub fn form_fill(&mut self, ui: &mut Ui, form_id: &str, household: Option<&str>) {
        let Some(served) = form_of(form_id) else {
            PageHeader::new("No such form").show(ui, |_| {});
            Empty::new("No such form")
                .description(format!(
                    "The server serves no form called {form_id}. The form list names every one it serves."
                ))
                .show(ui, |_| {});
            return;
        };
        let form = arranged(served, self.backend.layout_kept(served.id));
        let mut fill = self.fill_for(&form, household);
        let today = Day(day_of(self.now()) as i32);
        let household_size = fill
            .household
            .as_deref()
            .and_then(|h| self.backend.households.iter().find(|x| x.id == h))
            .map(|h| h.size as f64);
        let context = FillContext {
            place: fill.place.clone(),
            period: fill.period.clone(),
            visit_date: fill.visit_date.map(|d| d.to_string()),
            household_size,
        };
        let asked: Vec<&Question> = questions_of(&form)
            .filter(|q| q.kind != QuestionType::Computed)
            .collect();
        // what stops a submit: the context, each answer's own issue, a required question left empty
        // once a submit was tried, and the server's refusals
        let mut issues: Vec<Issue> = Vec::new();
        let mut issue = |id: &str, label: &str, text: String| {
            issues.push(Issue {
                id: id.into(),
                label: label.into(),
                text,
            })
        };
        if let Some(why) = fill.place.as_deref().and_then(|p| outside(&form, p)) {
            issue("place", "Reporting from", format!("{why}."));
        } else if fill.place.is_none() && fill.tried {
            issue("place", "Reporting from", "Say where the record is from.".into());
        }
        if form.period.is_some() && (fill.period.is_none() || fill.period_issue.is_some()) {
            issue(
                "period",
                "Reporting period",
                fill.period_issue
                    .clone()
                    .unwrap_or_else(|| "Say which period the report is for.".into()),
            );
        }
        if form.kind != FormKind::Periodic && fill.visit_date.is_none() && fill.tried {
            issue("visit-date", "Visit date", "Say when the visit was.".into());
        }
        if form.kind == FormKind::Followup && fill.household.is_none() && fill.tried {
            issue(
                "household",
                "Answering for",
                "Say which household the visit was to.".into(),
            );
        }
        for question in &asked {
            let value = fill.answers.get(question.id);
            let own = answer_issue(question, value).or_else(|| fill.server_issues.get(question.id).cloned());
            if let Some(text) = own {
                issue(question.id, question.label, text);
            } else if fill.tried && question.required && !is_answered(value) {
                issue(question.id, question.label, "Answer this question.".into());
            }
        }
        let issue_of = |id: &str| issues.iter().find(|i| i.id == id).map(|i| i.text.clone());
        let answered = if form.grid.is_none() {
            asked.iter().filter(|q| is_answered(fill.answers.get(q.id))).count()
        } else {
            fill.answers.0.len()
        };
        let total = if form.grid.is_none() {
            asked.len()
        } else {
            question_count(&form)
        };
        self.foot = Foot {
            note: format!("{answered} of {total} questions answered"),
            items: Vec::new(),
        };
        let mut layout = false;
        let page = ui.available_rect_before_wrap();
        let bar = FormBar::height(ui.ctx());
        ui.spacing_mut().item_spacing.y = 0.0;
        egui::ScrollArea::vertical()
            .id_salt(("fill", form.id))
            .max_height((page.height() - bar).max(0.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                sokkel_egui::Page::show(ui, |ui| {
                    let mut header = PageHeader::new(form.title)
                        .code(form.id)
                        .description(form.description)
                        .chip(form.kind.label(), sokkel_egui::ChipTone::Kind(form.kind.color()))
                        .meta(count(total, "question"));
                    if form.repeats {
                        header = header.meta("Each visit is its own record");
                    }
                    header.show(ui, |ui| {
                        let mut button = Button::new("Layout")
                            .icon(Icon::ListOrdered)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm);
                        if form.sections.is_empty() {
                            button = button.reason("A grid of counts has no sections to order.");
                        }
                        layout = ui.add(button).clicked();
                        ui.add(ApiLink::new(format!("/forms/{}", form.id)));
                    });
                    self.filed_for(ui, &form, &mut fill, &issue_of, today);
                    if form.grid.is_some() {
                        self.count_grid(ui, &form, &mut fill);
                    }
                    for section in form.sections.iter().filter(|s| !s.questions.is_empty()) {
                        let mut box_ = Section::new(section.title).meta(count(section.questions.len(), "question"));
                        if let Some(description) = section.description {
                            box_ = box_.description(description);
                        }
                        box_.show(ui, |ui| {
                            let wide = section.questions.iter().any(|q| q.kind == QuestionType::Multiline);
                            Columns::new(if wide { 2 } else { 3 })
                                .min(160.0)
                                .gap(16.0)
                                .fields(true)
                                .show(ui, section.questions.len(), |ui, index, _| {
                                    let question = &section.questions[index];
                                    question_field(ui, question, &mut fill, &context, issue_of(question.id), today);
                                });
                        });
                    }
                });
            });
        // the bar at the foot: Clear, Fill with test data and the seed, the note, the issues, Submit
        let (mut cleared, mut filled, mut sent, mut went) = (false, false, false, None);
        let entries: Vec<IssueEntry> = issues
            .iter()
            .map(|i| IssueEntry {
                label: i.label.clone(),
                text: i.text.clone(),
                warning: false,
            })
            .collect();
        let note = match fill.seed {
            Some(seed) => format!("Test data from seed {seed}"),
            None => format!("{answered} of {total} answered"),
        };
        FormBar::new().note(note).show_with(
            ui,
            |ui| {
                let clear = Button::new("Clear")
                    .icon(Icon::Eraser)
                    .variant(ButtonVariant::Ghost)
                    .size(ButtonSize::Sm)
                    .enabled(!fill.answers.0.is_empty());
                cleared = ui.add(clear).clicked();
                let test = Button::new("Fill with test data")
                    .icon(Icon::Sparkles)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Sm);
                filled = ui.add(test).clicked();
                let before = fill.seed_text.clone();
                ui.add(
                    Input::new(&mut fill.seed_text)
                        .name("Seed")
                        .placeholder("Seed")
                        .width(96.0),
                );
                if fill.seed_text != before {
                    fill.seed_text.retain(|c| c.is_ascii_digit());
                }
            },
            |ui| {
                sent = ui
                    .add(Button::new("Submit").icon(Icon::Send).size(ButtonSize::Sm))
                    .clicked();
                went = IssueCount::new(("fill-issues", form.id), &entries).show(ui);
            },
        );
        let _ = went;
        if filled {
            let used = fill
                .seed_text
                .parse::<u32>()
                .unwrap_or_else(|_| 1000 + (self.now() % 9000) as u32);
            fill.answers = test_answers(&form, used);
            fill.round += 1;
            fill.seed = Some(used);
            fill.seed_text = used.to_string();
            if fill.place.is_none() {
                let site = test_place(used);
                fill.place = Some(if form.id == "FRM-STOCK" {
                    path_of(&site).get(1).map_or(site.clone(), |p| p.id.clone())
                } else {
                    site
                });
            }
            fill.server_issues.clear();
            Toast::success("Filled with test data")
                .description(format!("Seed {used}. Change anything you like before submitting."))
                .show(ui.ctx());
        }
        if cleared {
            fill.answers = Answers::default();
            fill.round += 1;
            fill.server_issues.clear();
            fill.tried = false;
            fill.seed = None;
            Toast::new("Cleared")
                .description("The answers are empty again. Where and when the record is for stay as they were.")
                .show(ui.ctx());
        }
        if sent {
            fill.tried = true;
            let missing = asked.iter().any(|q| q.required && !is_answered(fill.answers.get(q.id)));
            let blocked = !issues.is_empty()
                || missing
                || fill.place.is_none()
                || (form.kind != FormKind::Periodic && fill.visit_date.is_none())
                || (form.kind == FormKind::Followup && fill.household.is_none());
            if blocked {
                Toast::new("Not filed yet")
                    .kind(ToastKind::Error)
                    .description("Some answers need attention; the count beside Submit lists them.")
                    .show(ui.ctx());
            } else {
                let mut answers = fill.answers.clone();
                for question in questions_of(&form) {
                    if let Some(formula) = question.formula {
                        answers.set(
                            question.id,
                            (formula.compute)(&fill.answers, &context).map(AnswerValue::Number),
                        );
                    }
                }
                let submission = Submission {
                    form_id: form.id.into(),
                    place: fill.place.clone(),
                    period: fill.period.clone(),
                    visit_date: fill.visit_date.map(|d| d.to_string()),
                    household_id: fill.household.clone(),
                    answers,
                    seed: fill.seed,
                };
                match self.backend.submit(submission, self.now()) {
                    Ok(code) => {
                        if self.keep_place {
                            self.last_place = fill.place.clone();
                        }
                        Toast::success("Filed")
                            .description(format!(
                                "Stored as receipt {}, not yet sent on to the register.",
                                &code[..12]
                            ))
                            .show(ui.ctx());
                        self.go(&format!("responses/{code}"));
                        return;
                    }
                    Err(Failure::Refused(found)) => {
                        let said = found.len();
                        fill.server_issues = found.into_iter().map(|i| (i.question, i.text)).collect();
                        Toast::new("The server refused the response")
                            .kind(ToastKind::Error)
                            .description(format!("{}; nothing was stored.", count(said, "issue")))
                            .show(ui.ctx());
                    }
                    Err(Failure::Unreachable) => {
                        Toast::new("Not filed")
                            .kind(ToastKind::Error)
                            .description(
                                "The server is not answering, so nothing was stored. Try again once it answers.",
                            )
                            .show(ui.ctx());
                    }
                }
            }
        }
        self.fill = Some(fill);
        if layout {
            self.go(&format!("forms/{}/layout", form.id));
        }
    }

    /// Where and when the record is for: the place, the period or the visit's day, the household.
    fn filed_for(
        &mut self,
        ui: &mut Ui,
        form: &Form,
        fill: &mut Fill,
        issue_of: &dyn Fn(&str) -> Option<String>,
        today: Day,
    ) {
        let households = self.backend.households.clone();
        let keep = self.keep_place;
        Section::new("Filed for")
            .description("Where and when the record is for. It is context, not an answer: the server files the whole record under it.")
            .show(ui, |ui| {
                let mut parts: Vec<&str> = vec!["place"];
                if form.period.is_some() {
                    parts.push("period");
                }
                if form.kind != FormKind::Periodic {
                    parts.push("visit");
                }
                if form.kind == FormKind::Followup {
                    parts.push("household");
                }
                if form.kind == FormKind::Registration {
                    parts.push("identifier");
                }
                let tree = place_tree();
                Columns::new(2).min(224.0).gap(16.0).fields(true).show(ui, parts.len(), |ui, index, _| {
                    match parts[index] {
                        "place" => {
                            let mut field = Field::new(if form.kind == FormKind::Registration {
                                "Registered at"
                            } else {
                                "Reporting from"
                            })
                            .required(true)
                            .description(place_meaning(form.kind))
                            .error(issue_of("place"));
                            if keep {
                                field = field.help("The place chosen is kept for the next form.");
                            }
                            field.show(ui, |ui| {
                                let unfolded: Vec<String> = match &fill.place {
                                    None => vec![COUNTRY.into()],
                                    Some(p) => path_of(p).iter().map(|x| x.id.clone()).collect(),
                                };
                                TreeSelect::new(("place", form.id), &tree, &mut fill.place)
                                    .expanded(unfolded)
                                    .invalid(issue_of("place").is_some())
                                    .show(ui);
                            });
                        }
                        "period" => {
                            let kind = form.period.expect("a period form");
                            Field::new("Reporting period")
                                .required(true)
                                .hint(Some(kind.label().to_lowercase()))
                                .description("The period the counts are for. A report for another period is a record of its own.")
                                .error(if fill.period_issue.is_none() { issue_of("period") } else { None })
                                .show(ui, |ui| {
                                    fill.period_issue =
                                        PeriodPicker::new(("period", form.id), kind, &mut fill.period, today).show(ui);
                                });
                        }
                        "visit" => {
                            DateField::new(("visit-date", form.id), "Visit date", &mut fill.visit_date, today)
                                .required(true)
                                .description("The day the visit was made. The record is filed under it.")
                                .error(issue_of("visit-date"))
                                .show(ui);
                        }
                        "household" => {
                            let offered: Vec<_> = households
                                .iter()
                                .filter(|h| {
                                    h.status == crate::backend::HouseholdStatus::Active
                                        || Some(&h.id) == fill.household.as_ref()
                                })
                                .collect();
                            let chosen = fill
                                .household
                                .as_deref()
                                .and_then(|id| households.iter().find(|h| h.id == id));
                            let help = if offered.is_empty() {
                                Some("No household is registered yet. Register one first.".to_owned())
                            } else {
                                chosen.map(|h| {
                                    format!(
                                        "{}, registered at {}.",
                                        count_as(h.size as usize, "person", "people"),
                                        crate::geo::place_name(&h.site)
                                    )
                                })
                            };
                            Field::new("Answering for")
                                .required(true)
                                .description("The registered household the visit was to.")
                                .help_if(help)
                                .error(issue_of("household"))
                                .show(ui, |ui| {
                                    let mut labels = vec!["Not chosen".to_owned()];
                                    labels.extend(offered.iter().map(|h| {
                                        format!("{}, {}", h.identifier, crate::geo::place_name(&h.site))
                                    }));
                                    let mut at = fill
                                        .household
                                        .as_ref()
                                        .and_then(|id| offered.iter().position(|h| h.id == *id))
                                        .map_or(0, |i| i + 1);
                                    let width = ui.available_width();
                                    if ui
                                        .add(Select::new(&mut at, labels, "Answering for").width(width))
                                        .changed()
                                    {
                                        fill.household = at.checked_sub(1).map(|i| offered[i].id.clone());
                                        if fill.place.is_none()
                                            && let Some(h) = at.checked_sub(1).map(|i| offered[i])
                                        {
                                            fill.place = Some(h.site.clone());
                                        }
                                    }
                                });
                        }
                        _ => {
                            ComputedField::new(
                                "Household identifier",
                                None,
                                "The server gives the household its identifier when the record is filed, and every visit names it.",
                            )
                            .words(sokkel_egui::ComputedFieldWords {
                                pending: "Given when the record is filed".into(),
                                computed: "from the server".into(),
                            })
                            .show(ui);
                        }
                    }
                });
            });
    }

    /// A periodic report's grid of counts.
    fn count_grid(&mut self, ui: &mut Ui, form: &Form, fill: &mut Fill) {
        crate::count_grid::show(self, ui, form, fill);
    }
}

/// One question as its field: a number, a yes or no, a choice, a time, a day, text, a value worked
/// out from the others, or a text answered more than once.
fn question_field(
    ui: &mut Ui,
    question: &Question,
    fill: &mut Fill,
    context: &FillContext,
    error: Option<String>,
    today: Day,
) {
    let id = (fill.form.clone(), question.id);
    if question.repeats {
        let mut rows: Vec<String> = match fill.answers.get(question.id) {
            Some(AnswerValue::List(list)) if !list.is_empty() => list.clone(),
            _ => vec![String::new()],
        };
        let item = |index: usize| format!("Answer {}", index + 1);
        let out = RepeatedGroup::new(question.label, rows.len())
            .description(question.description)
            .required(question.required)
            .min(1)
            .words(RepeatedGroupWords {
                item: |index| format!("Answer {}", index + 1),
                ..Default::default()
            })
            .show(ui, |ui, index| {
                let name = format!("{}, {}", question.label, item(index));
                ui.add(Input::new(&mut rows[index]).name(name));
            });
        if out.added {
            rows.push(String::new());
        }
        if let Some(at) = out.removed {
            rows.remove(at);
        }
        let keep = if rows.iter().any(|r| !r.trim().is_empty()) || rows.len() > 1 {
            Some(AnswerValue::List(rows))
        } else {
            None
        };
        fill.answers.set(question.id, keep);
        return;
    }
    match question.kind {
        QuestionType::Computed => {
            let formula = question.formula.expect("a worked-out question");
            let worked = (formula.compute)(&fill.answers, context).map(crate::forms::js_number);
            ComputedField::new(question.label, worked, formula.why)
                .unit(question.unit)
                .formula(formula.expression)
                .description(question.description)
                .show(ui);
        }
        QuestionType::Integer | QuestionType::Decimal => {
            let mut value = fill.answers.number(question.id);
            let before = value;
            ui.add(
                NumberField::optional(id, question.label, &mut value)
                    .required(question.required)
                    .hint(question.unit)
                    .description(question.description)
                    .help(question.help.map(str::to_owned).or_else(|| range_hint(question)))
                    .integer(question.kind == QuestionType::Integer)
                    .error(error),
            );
            if value != before {
                fill.answers.set(question.id, value.map(AnswerValue::Number));
                fill.server_issues.remove(question.id);
            }
        }
        QuestionType::YesNo => {
            let mut value = match fill.answers.get(question.id) {
                Some(AnswerValue::Bool(b)) => Some(*b),
                _ => None,
            };
            let changed = YesNoField::new(question.label, &mut value)
                .required(question.required)
                .yes_only(question.yes_only)
                .description(question.description)
                .help(question.help)
                .error(error)
                .show(ui);
            if changed {
                let value = value.filter(|v| !(question.yes_only && !*v));
                fill.answers.set(question.id, value.map(AnswerValue::Bool));
            }
        }
        QuestionType::Choice => {
            let mut field = Field::new(question.label)
                .required(question.required)
                .help_if(question.help)
                .error(error);
            if let Some(description) = question.description {
                field = field.description(description);
            }
            field.show(ui, |ui| {
                let mut labels = vec!["Not answered"];
                labels.extend(question.options.iter().map(|o| o.label));
                let current = fill
                    .answers
                    .get(question.id)
                    .and_then(AnswerValue::text)
                    .map(str::to_owned);
                let mut at = current
                    .and_then(|v| question.options.iter().position(|o| o.value == v))
                    .map_or(0, |i| i + 1);
                let width = ui.available_width();
                if ui
                    .add(Select::new(&mut at, labels, question.label).width(width))
                    .changed()
                {
                    let value = at
                        .checked_sub(1)
                        .map(|i| AnswerValue::Text(question.options[i].value.into()));
                    fill.answers.set(question.id, value);
                }
            });
        }
        QuestionType::Time => {
            let mut value = fill
                .answers
                .get(question.id)
                .and_then(AnswerValue::text)
                .map(str::to_owned);
            let changed = TimeField::new(id, question.label, &mut value)
                .required(question.required)
                .description(question.description)
                .help(question.help)
                .error(error)
                .show(ui);
            if changed {
                fill.answers.set(question.id, value.map(AnswerValue::Text));
            }
        }
        QuestionType::Date => {
            let mut value = fill
                .answers
                .get(question.id)
                .and_then(AnswerValue::text)
                .and_then(Day::parse);
            let mut field = DateField::new(id, question.label, &mut value, today)
                .required(question.required)
                .error(error);
            if let Some(description) = question.description {
                field = field.description(description);
            }
            if field.show(ui) {
                fill.answers
                    .set(question.id, value.map(|d| AnswerValue::Text(d.to_string())));
            }
        }
        QuestionType::Text | QuestionType::Multiline => {
            let mut text = fill
                .answers
                .get(question.id)
                .and_then(AnswerValue::text)
                .unwrap_or_default()
                .to_owned();
            let before = text.clone();
            let mut field = Field::new(question.label)
                .required(question.required)
                .help_if(question.help)
                .error(error);
            if let Some(description) = question.description {
                field = field.description(description);
            }
            field.show(ui, |ui| {
                ui.add(
                    Input::new(&mut text)
                        .name(question.label)
                        .multiline(question.kind == QuestionType::Multiline),
                );
            });
            if text != before {
                fill.answers
                    .set(question.id, (!text.is_empty()).then(|| AnswerValue::Text(text.clone())));
            }
        }
    }
}
