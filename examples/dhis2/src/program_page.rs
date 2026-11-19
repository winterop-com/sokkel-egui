//! The program's rules, as the web's `pages/ProgramRules.tsx` draws them: the rules in the order they
//! run in a column, dragged into another order to number their priorities again; the chosen one's
//! name, priority, condition and actions edited; and the program's form beside them, filled in by
//! hand and reacting to every rule as it is filled, with what the rules did and those that cannot run.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    ButtonVariant, Cell, Column, ComputedField, DataTable, DateField, Field, Gap, Icon, Input, IssueTone, NumberField,
    Page, PageHeader, PageState, RepeatedGroup, SaveBar, Section, Select, SplitPane, Stack, SubNav, TableVariant,
    TableWords, Tree, TreeNode, YesNoField, group_label, tokens,
};

use crate::editor::ExpressionField;
use crate::expression::{Language, Value};
use crate::pages::program_rules::{self, ProgramRules, TryForm, action_fields, action_options, field_options};
use crate::program::{ACTION_TYPES, fields, program, variables};
use crate::screens::{Act, small, today};
use crate::workbench::{Asking, Views, WorkbenchApp};

/// The issues the rules show on a field, in the field's tones.
fn tones(issues: Vec<crate::ui::Issue>) -> Vec<(IssueTone, String)> {
    issues
        .into_iter()
        .map(|one| {
            (
                if one.tone == "critical" {
                    IssueTone::Critical
                } else {
                    IssueTone::Warning
                },
                one.text,
            )
        })
        .collect()
}

pub fn program_rules_page(
    ui: &mut Ui,
    app: &WorkbenchApp,
    selected: Option<&str>,
    views: &mut Views,
    acts: &mut Vec<Act>,
) {
    let rows = app.backend.program_rules.seen();
    let loading = app.backend.program_rules.seen.is_none();
    let version = app.version();
    let page = &mut views.program;
    let saved = page.saved(rows, selected);
    // the panes' pages are inset as the shell's page is
    let container = ui.available_width();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let mut column = views.column;
        let nodes: Vec<TreeNode> = ProgramRules::nodes(rows)
            .into_iter()
            .map(|one| TreeNode::new(one.id, one.label).meta(one.meta))
            .collect();
        let mut chosen = None;
        let mut moved = None;
        SubNav::new("program-rules-column", &mut column).show(
            ui,
            |ui| {
                ui.horizontal(|ui| {
                    ui.set_min_height(tokens(ui.ctx()).density("control-sm"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(small("New", Some(Icon::Plus), ButtonVariant::Ghost)).clicked() {
                            let rows = rows.to_vec();
                            acts.push(Act::new(move |app| {
                                app.views.program.create(&rows, &mut app.backend);
                            }));
                        }
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            group_label(ui, program().name);
                        });
                    });
                });
            },
            |ui| {
                let mut picked = saved.as_ref().map(|one| one.id.clone());
                let can_move = |_: &str, into: Option<&str>| into.is_none();
                let out = Tree::new("program-rules-tree", &nodes, &mut picked, &mut views.program_open)
                    .movable(&can_move)
                    .show(ui);
                chosen = out.chosen;
                moved = out.moved;
            },
        );
        views.column = column;
        if let Some(id) = chosen.filter(|id| Some(id) != saved.as_ref().map(|one| &one.id)) {
            acts.push(Act::go(format!("program-rules/{id}")));
        }
        if let Some((id, _, index)) = moved {
            let rows = rows.to_vec();
            acts.push(Act::new(move |app| {
                app.views.program.reorder(&rows, &mut app.backend, &id, index);
            }));
        }
        let room = ui.available_rect_before_wrap();
        let mut fill = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(room)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let ui = &mut fill;
        let Some(saved) = saved else {
            Page::show(ui, |ui| {
                if loading {
                    PageState::loading().show(ui);
                } else {
                    PageState::empty(program_rules::EMPTY).show(ui);
                }
            });
            return;
        };
        let rules = page.rules(rows, Some(&saved.id));
        let mut form = std::mem::take(&mut page.form);
        SplitPane::new("program-rules-split")
            .default_share(0.58)
            .min_share(0.4)
            .max_share(0.75)
            .show(
                ui,
                |ui| editor(ui, container, page, &saved, version, acts),
                |ui| {
                    try_form(
                        ui,
                        container,
                        &mut form,
                        &rules,
                        version,
                        &mut views.effects,
                        &mut views.broken,
                    )
                },
            );
        page.form = form;
    });
}

/// The rule being edited: its name, priority and description, its condition and actions, and the
/// save bar while anything is unsaved.
fn editor(
    ui: &mut Ui,
    container: f32,
    page: &mut ProgramRules,
    saved: &crate::program::ProgramRule,
    version: &str,
    acts: &mut Vec<Act>,
) {
    let changed = page.changed(saved);
    let bar = if changed > 0 {
        sokkel_egui::FormBar::height(ui.ctx())
    } else {
        0.0
    };
    let room = ui.available_rect_before_wrap();
    let mut body = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(egui::Rect::from_min_max(
                room.min,
                egui::pos2(room.right(), room.bottom() - bar),
            ))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let mut draft = page.editing(saved);
    let condition = page.condition(saved, version);
    let context = variables();
    Page::scroll_in(&mut body, container, |ui| {
        let mut header = PageHeader::new(&saved.name).code(&saved.id);
        if !saved.description.is_empty() {
            header = header.description(&saved.description);
        }
        header.show(ui, |ui| {
            if ui
                .add(small("Delete", Some(Icon::Trash2), ButtonVariant::Outline))
                .clicked()
            {
                acts.push(Act::ask(Asking::DeleteProgramRule(saved.id.clone())));
            }
        });
        Section::new("Rule").show(ui, |ui| {
            Stack::new(Gap::Lg).show(ui, |ui| {
                sokkel_egui::Columns::new(2)
                    .min(120.0)
                    .gap(12.0)
                    .fields(true)
                    .show(ui, 2, |ui, at, width| {
                        ui.set_width(width);
                        if at == 0 {
                            Field::new("Name").required(true).show(ui, |ui| {
                                ui.add(Input::new(&mut draft.name).name("Name"));
                            });
                        } else {
                            ui.add_enabled_ui(true, |ui| {
                                ui.add(
                                    NumberField::new("program-rule-priority", "Priority", &mut draft.priority)
                                        .integer(true)
                                        .help(Some(
                                            "Rules run from the lowest priority up; a later rule reads what an earlier one assigned.",
                                        )),
                                );
                            });
                        }
                    });
                Field::new("Description").show(ui, |ui| {
                    ui.add(Input::new(&mut draft.description).name("Description"));
                });
            });
        });
        Section::new("Condition and actions")
            .description("The condition is written in the program rule language: #{} names the form's fields by their rule variables, V{} the program's variables and d2: its functions.")
            .show(ui, |ui| {
                Stack::new(Gap::Lg).show(ui, |ui| {
                    ExpressionField {
                        id: "program-rule-condition",
                        label: "Condition",
                        language: Language::Rule,
                        checked: &condition,
                        version,
                        variables: &context,
                        description: None,
                    }
                    .show(ui, &mut draft.condition);
                    let options = action_options();
                    let field_choices = field_options();
                    let out = RepeatedGroup::new("Actions", draft.actions.len())
                        .description(Some("What happens on the form while the condition holds."))
                        .min(1)
                        .show(ui, |ui, index| {
                            let action = &mut draft.actions[index];
                            let shape = action_fields(action, version);
                            Stack::new(Gap::Md).show(ui, |ui| {
                                sokkel_egui::Columns::new(2)
                                    .min(120.0)
                                    .gap(12.0)
                                    .fields(true)
                                    .show(ui, 2, |ui, at, width| {
                                        ui.set_width(width);
                                        if at == 0 {
                                            let mut chosen =
                                                ACTION_TYPES.iter().position(|one| *one == action.kind).unwrap_or(0);
                                            Field::new("Action").show(ui, |ui| {
                                                let width = ui.available_width();
                                                ui.add(
                                                    Select::new(&mut chosen, options.iter().map(|(_, label)| *label), "Action")
                                                        .width(width),
                                                );
                                            });
                                            action.kind = ACTION_TYPES[chosen];
                                        } else if shape.field {
                                            let mut chosen = field_choices
                                                .iter()
                                                .position(|(variable, _)| *variable == action.field)
                                                .unwrap_or(usize::MAX);
                                            Field::new("Field").show(ui, |ui| {
                                                let width = ui.available_width();
                                                ui.add(
                                                    Select::new(
                                                        &mut chosen,
                                                        field_choices.iter().map(|(_, label)| *label),
                                                        "Field",
                                                    )
                                                    .width(width),
                                                );
                                            });
                                            if let Some((variable, _)) = field_choices.get(chosen) {
                                                action.field = (*variable).to_owned();
                                            }
                                        }
                                    });
                                if shape.content {
                                    Field::new("Message").show(ui, |ui| {
                                        ui.add(Input::new(&mut action.content).name("Message"));
                                    });
                                }
                                if shape.data {
                                    ExpressionField {
                                        id: &format!("{}-data", action.key),
                                        label: shape.data_label,
                                        language: Language::Rule,
                                        checked: &shape.checked,
                                        version,
                                        variables: &context,
                                        description: None,
                                    }
                                    .show(ui, &mut action.data);
                                }
                            });
                        });
                    if out.added {
                        let made = saved.clone();
                        acts.push(Act::new(move |app| app.views.program.add_action(&made, &mut app.backend)));
                    }
                    if let Some(index) = out.removed
                        && index < draft.actions.len()
                    {
                        draft.actions.remove(index);
                    }
                });
            });
    });
    if draft != page.editing(saved) {
        page.set(saved, |now| *now = draft.clone());
    }
    if changed > 0 {
        let mut foot = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_max(
                    egui::pos2(room.left(), room.bottom() - bar),
                    room.max,
                ))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let out = SaveBar::new(changed).busy(page.busy).show(&mut foot);
        if out.discard {
            page.discard();
        }
        if out.save {
            let saved = saved.clone();
            acts.push(Act::new(move |app| app.views.program.save(&saved, &mut app.backend)));
        }
    }
}

/// The program's form, every rule run on it as it changes, and what the rules did.
fn try_form(
    ui: &mut Ui,
    container: f32,
    form: &mut TryForm,
    rules: &[crate::program::ProgramRule],
    version: &str,
    effects_table: &mut sokkel_egui::TableState,
    broken_table: &mut sokkel_egui::TableState,
) {
    let run = form.run(rules, version);
    let shown = form.shown(&run);
    let (stage, description) = program_rules::form_words();
    Page::scroll_in(ui, container, |ui| {
        let mut cleared = false;
        Section::new("Try the form")
            .meta(stage)
            .description(description)
            .actions(|ui| {
                cleared = ui
                    .add(small("Clear", Some(Icon::RotateCcw), ButtonVariant::Ghost))
                    .clicked();
            })
            .show(ui, |ui| {
                Stack::new(Gap::Md).show(ui, |ui| {
                    let mut date = sokkel_egui::Day::parse(&form.event_date);
                    if DateField::new("try-event-date", "Inspection date", &mut date, today()).show(ui) {
                        let written = date.map(|one| one.to_string());
                        form.set_event_date(written.as_deref());
                    }
                    for field in &shown {
                        let spec = fields().iter().find(|one| one.variable == field.variable);
                        let issues = tones(field.issues.clone());
                        match field.widget {
                            "computed" => ComputedField::new(
                                field.label,
                                field.value.as_str().map(str::to_owned),
                                "Assigned by a program rule from the other answers.",
                            )
                            .show(ui),
                            "option" => {
                                let options = spec.and_then(|one| one.options.clone()).unwrap_or_default();
                                let now = field.value.as_str().unwrap_or("");
                                let mut chosen = options.iter().position(|one| one.value == now).unwrap_or(usize::MAX);
                                let before = chosen;
                                Field::new(field.label).issues(issues).show(ui, |ui| {
                                    let width = ui.available_width();
                                    ui.add(
                                        Select::new(&mut chosen, options.iter().map(|one| one.label), field.label)
                                            .placeholder("Choose")
                                            .width(width),
                                    );
                                });
                                if chosen != before
                                    && let Some(one) = options.get(chosen)
                                {
                                    form.set(field.variable, Value::Text(one.value.to_owned()));
                                }
                            }
                            "boolean" => {
                                let mut value = field.value.as_bool();
                                if YesNoField::new(field.label, &mut value).issues(issues).show(ui) {
                                    form.set(field.variable, value.map_or(Value::Null, Value::Bool));
                                }
                            }
                            "number" => {
                                let mut value = field.value.as_f64();
                                if ui
                                    .add(
                                        NumberField::optional(&field.id, field.label, &mut value)
                                            .integer(true)
                                            .issues(issues),
                                    )
                                    .changed()
                                {
                                    form.set(field.variable, value.map_or(Value::Null, Value::Number));
                                }
                            }
                            "date" => {
                                let mut value = field.value.as_str().and_then(sokkel_egui::Day::parse);
                                if DateField::new(&field.id, field.label, &mut value, today()).show(ui) {
                                    form.set(
                                        field.variable,
                                        value.map_or(Value::Null, |one| Value::Text(one.to_string())),
                                    );
                                }
                            }
                            _ => {
                                let mut text = field.value.as_str().unwrap_or("").to_owned();
                                Field::new(field.label).issues(issues).show(ui, |ui| {
                                    ui.add(Input::new(&mut text).name(field.label));
                                });
                                if text != field.value.as_str().unwrap_or("") {
                                    form.set(
                                        field.variable,
                                        if text.is_empty() {
                                            Value::Null
                                        } else {
                                            Value::Text(text)
                                        },
                                    );
                                }
                            }
                        }
                    }
                });
            });
        if cleared {
            form.clear();
        }
        let effects = TryForm::effects(&run, rules);
        Section::new("What the rules did")
            .meta(effects.len().to_string())
            .flush(true)
            .show(ui, |ui| {
                let columns = [
                    Column::new("Rule", 128.0).fill(),
                    Column::new("Effect", 160.0).fill_share(1.4),
                ];
                let cells: Vec<Vec<Cell>> = effects
                    .iter()
                    .map(|one| {
                        let title = match &one.field {
                            Some(field) => format!("{} on {field}", one.action),
                            None => one.action.to_owned(),
                        };
                        vec![
                            Cell::Text(one.rule.clone()),
                            match &one.text {
                                Some(text) => Cell::Lines {
                                    title,
                                    detail: text.clone(),
                                    over: false,
                                },
                                None => Cell::Strong(title),
                            },
                        ]
                    })
                    .collect();
                DataTable::new("rule-effects", &columns, &cells, effects_table)
                    .keys(effects.iter().map(|one| one.id.clone()))
                    .variant(TableVariant::List)
                    .framed(false)
                    .heads(false)
                    .auto_rows()
                    .fits(true)
                    .footer(false)
                    .height(f32::INFINITY)
                    .words(TableWords {
                        empty: "No rule acts on the form as it is filled.".to_owned(),
                        ..Default::default()
                    })
                    .show(ui);
            });
        let broken = TryForm::broken(&run, rules);
        if !broken.is_empty() {
            Section::new("Cannot run")
                .meta(broken.len().to_string())
                .flush(true)
                .show(ui, |ui| {
                    let columns = [
                        Column::new("Rule", 128.0).fill(),
                        Column::new("Why", 160.0).fill_share(1.4),
                    ];
                    let cells: Vec<Vec<Cell>> = broken
                        .iter()
                        .map(|one| {
                            vec![
                                Cell::Text(one.rule.clone()),
                                Cell::Toned(one.why.clone(), "critical-ink"),
                            ]
                        })
                        .collect();
                    DataTable::new("rule-broken", &columns, &cells, broken_table)
                        .keys(broken.iter().map(|one| one.id.clone()))
                        .variant(TableVariant::List)
                        .framed(false)
                        .heads(false)
                        .auto_rows()
                        .fits(true)
                        .footer(false)
                        .height(f32::INFINITY)
                        .show(ui);
                });
        }
    });
}
