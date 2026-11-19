//! An indicator and a validation rule edited beside where they are tried, as the web's
//! `pages/Indicator.tsx` and `pages/ValidationRule.tsx` draw them: the definition with its
//! expressions checked as typed on one side, the indicator worked out (or the rule run) for a unit
//! and months on the other, and the save bar while anything is unsaved.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    ApiLink, ButtonVariant, Cell, Column, Columns, DataTable, Field, Gap, Icon, Input, Page, PageHeader, PageState,
    SaveBar, Section, Select, Stack, StackLayout, Stat, StatTiles, TableVariant, TableWords, TreeSelect, tokens,
};
use sokkel_egui_charts::{ChartKind, ChartPanel, PanelWords};

use crate::backend::{INDICATOR_TYPES, OPERATORS, STRATEGIES};
use crate::editor::ExpressionField;
use crate::expression::Language;
use crate::metadata::{org_unit, org_unit_tree, root};
use crate::pages::{indicator, validation_rule};
use crate::screens::{Act, indicator_ids, line_spec, small, today, tree_nodes};
use crate::ui::figure;
use crate::workbench::{Asking, Views, WorkbenchApp};

/// The two columns of an editor page: the definition and where it is tried, `inspector-gap` apart.
fn two_columns(ui: &mut Ui, mut column: impl FnMut(&mut Ui, usize)) {
    let gap = tokens(ui.ctx()).density("inspector-gap");
    Columns::new(2).min(288.0).gap(gap).show(ui, 2, |ui, index, width| {
        ui.set_width(width);
        column(ui, index);
    });
}

/// Fields side by side, `md` apart.
fn pair(ui: &mut Ui, mut field: impl FnMut(&mut Ui, usize)) {
    Columns::new(2)
        .min(120.0)
        .gap(12.0)
        .fields(true)
        .show(ui, 2, |ui, index, width| {
            ui.set_width(width);
            field(ui, index);
        });
}

/// A select field over `options`, its value one of their keys.
fn select_field<K: PartialEq + Copy>(
    ui: &mut Ui,
    label: &str,
    value: &mut K,
    options: &[(K, String)],
    help: Option<&str>,
) -> bool {
    let mut at = options.iter().position(|(key, _)| key == value).unwrap_or(0);
    let before = at;
    Field::new(label).help_if(help).show(ui, |ui| {
        let width = ui.available_width();
        ui.add(Select::new(&mut at, options.iter().map(|(_, label)| label.clone()), label).width(width));
    });
    if at != before {
        *value = options[at].0;
    }
    at != before
}

/// A text field, required or not, one line or several.
fn text_field(ui: &mut Ui, label: &str, value: &mut String, required: bool, multiline: bool) {
    Field::new(label).required(required).show(ui, |ui| {
        ui.add(Input::new(value).multiline(multiline).name(label));
    });
}

/// The indicator's page, an editor once its row is in.
pub fn indicator_page(ui: &mut Ui, app: &WorkbenchApp, id: &str, views: &mut Views, acts: &mut Vec<Act>) {
    let rows = app.backend.indicators.seen.as_ref();
    let Some(editor) = views.indicator.as_mut().filter(|one| one.saved.id == id) else {
        Page::show(ui, |ui| {
            if rows.is_none() {
                PageState::loading().show(ui);
            } else {
                PageState::missing(indicator::EMPTY).show(ui);
            }
        });
        return;
    };
    let version = app.version();
    let ids = indicator_ids(app);
    let checks = editor.checks(version, &ids);
    let t = tokens(ui.ctx());
    let changed = editor.changed();
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
    Page::scroll(&mut body, |ui| {
        let saved = editor.saved.clone();
        let mut header = PageHeader::new(&saved.name).code(&saved.id);
        if !saved.description.is_empty() {
            header = header.description(&saved.description);
        }
        header.show(ui, |ui| {
            if ui
                .add(small("Delete", Some(Icon::Trash2), ButtonVariant::Outline))
                .clicked()
            {
                acts.push(Act::ask(Asking::DeleteIndicator));
            }
            ui.add(ApiLink::new(format!("/api/indicators/{}", saved.id)));
        });
        two_columns(ui, |ui, index| {
            if index == 0 {
                Section::new("Definition")
                    .description("Typed in DHIS2's expression language. Type #{ and a name to put in a data element, a dot after it for one of its category option combos.")
                    .show(ui, |ui| {
                        Stack::new(Gap::Lg).show(ui, |ui| {
                            let types: Vec<_> = indicator::type_options()
                                .into_iter()
                                .zip(INDICATOR_TYPES)
                                .map(|((_, label), kind)| (kind, label))
                                .collect();
                            pair(ui, |ui, at| {
                                if at == 0 {
                                    text_field(ui, "Name", &mut editor.draft.name, true, false);
                                } else {
                                    select_field(ui, "Type", &mut editor.draft.kind, &types, None);
                                }
                            });
                            text_field(ui, "Description", &mut editor.draft.description, false, true);
                            ExpressionField {
                                id: "indicator-numerator",
                                label: "Numerator",
                                language: Language::Expression,
                                checked: &checks.numerator,
                                version,
                                variables: &[],
                                description: None,
                            }
                            .show(ui, &mut editor.draft.numerator);
                            ExpressionField {
                                id: "indicator-denominator",
                                label: "Denominator",
                                language: Language::Expression,
                                checked: &checks.denominator,
                                version,
                                variables: &[],
                                description: None,
                            }
                            .show(ui, &mut editor.draft.denominator);
                        });
                    });
            } else {
                try_indicator(ui, editor, &checks, &mut views.below);
            }
        });
    });
    if changed > 0 {
        let mut foot = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_max(
                    egui::pos2(room.left(), room.bottom() - bar),
                    room.max,
                ))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        let out = SaveBar::new(changed).busy(editor.busy).show(&mut foot);
        if out.discard {
            editor.discard();
        }
        if out.save {
            acts.push(Act::new(|app| {
                if let Some(editor) = &mut app.views.indicator {
                    editor.save(&mut app.backend);
                }
            }));
        }
    }
    let _ = t;
}

/// The indicator's Try it: a unit and a month, its parts and value there, over the year, and for
/// each unit under it.
fn try_indicator(
    ui: &mut Ui,
    editor: &mut indicator::Editor,
    checks: &indicator::Checks,
    below_table: &mut sokkel_egui::TableState,
) {
    Section::new("Try it")
        .description("The expressions as they stand, unsaved edits included, run on the server's data values.")
        .show(ui, |ui| {
            Stack::new(Gap::Lg).show(ui, |ui| {
                let tree = tree_nodes(&org_unit_tree());
                pair(ui, |ui, at| {
                    if at == 0 {
                        Field::new("Organization unit").show(ui, |ui| {
                            let mut value = Some(editor.unit.clone());
                            if TreeSelect::new("indicator-unit", &tree, &mut value)
                                .expanded([root()])
                                .show(ui)
                            {
                                editor.unit = value.unwrap_or_else(|| root().to_owned());
                            }
                        });
                    } else {
                        Field::new("Period").show(ui, |ui| {
                            let mut value = Some(editor.period.clone());
                            sokkel_egui::PeriodPicker::new(
                                "indicator-period",
                                sokkel_egui::periods::PeriodType::Monthly,
                                &mut value,
                                today(),
                            )
                            .show(ui);
                            if let Some(value) = value {
                                editor.period = value;
                            }
                        });
                    }
                });
                let stats: Vec<Stat> = editor
                    .stats(checks)
                    .into_iter()
                    .map(|one| {
                        let stat = Stat::new(&one.label, &one.value).emphasis(one.emphasis);
                        match &one.note {
                            Some(note) => stat.note(note),
                            None => stat,
                        }
                    })
                    .collect();
                StatTiles::new("indicator-stats", 3, 120.0, 12.0).show(ui, stats);
                let here = org_unit(&editor.unit).map_or(String::new(), |one| one.name.clone());
                let year = editor.year(checks);
                let spec = line_spec(&year, "value", editor.unit_sign(), &editor.draft.name);
                ChartPanel::new("Over the year", ChartKind::Line, &spec)
                    .subtitle(here)
                    .empty(!checks.runnable())
                    .words(PanelWords::default())
                    .legend(false)
                    .height(214.0)
                    .show(ui);
                let below = editor.below(checks);
                if !below.is_empty() {
                    let sign = editor.unit_sign();
                    let columns = [
                        Column::new(editor.below_header(), 128.0).fill(),
                        Column::new("Numerator", 112.0).numeric(),
                        Column::new("Denominator", 112.0).numeric(),
                        Column::new("Value", 96.0).numeric(),
                    ];
                    let number = |value: Option<f64>, unit: &str| Cell::Number {
                        value: value.unwrap_or(f64::NEG_INFINITY),
                        text: figure(value, unit),
                    };
                    let cells: Vec<Vec<Cell>> = below
                        .iter()
                        .map(|one| {
                            vec![
                                Cell::Text(one.name.clone()),
                                number(one.numerator, ""),
                                number(one.denominator, ""),
                                number(one.value, sign),
                            ]
                        })
                        .collect();
                    DataTable::new("indicator-below", &columns, &cells, below_table)
                        .keys(below.iter().map(|one| one.id.clone()))
                        .variant(TableVariant::List)
                        .stack_layout(StackLayout::Labeled)
                        .chevron(true)
                        .fits(true)
                        .footer(false)
                        .height(f32::INFINITY)
                        .show(ui);
                    if let Some(index) = below_table.opened.take()
                        && let Some(one) = below.get(index)
                    {
                        editor.unit = one.id.clone();
                    }
                }
            });
        });
}

/// The validation rule's page, an editor once its row is in.
pub fn rule_page(ui: &mut Ui, app: &WorkbenchApp, id: &str, views: &mut Views, acts: &mut Vec<Act>) {
    let rows = app.backend.validation_rules.seen.as_ref();
    let Some(editor) = views.rule.as_mut().filter(|one| one.saved.id == id) else {
        Page::show(ui, |ui| {
            if rows.is_none() {
                PageState::loading().show(ui);
            } else {
                PageState::missing(validation_rule::EMPTY).show(ui);
            }
        });
        return;
    };
    let version = app.version();
    let ids = indicator_ids(app);
    let checks = editor.checks(version, &ids);
    let changed = editor.changed();
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
    let mut toast = None;
    Page::scroll(&mut body, |ui| {
        let saved = editor.saved.clone();
        let mut header = PageHeader::new(&saved.name).code(&saved.id);
        if !saved.instruction.is_empty() {
            header = header.description(&saved.instruction);
        }
        header.show(ui, |ui| {
            if ui
                .add(small("Delete", Some(Icon::Trash2), ButtonVariant::Outline))
                .clicked()
            {
                acts.push(Act::ask(Asking::DeleteRule));
            }
            ui.add(ApiLink::new(format!("/api/validationRules/{}", saved.id)));
        });
        two_columns(ui, |ui, index| {
            if index == 0 {
                rule_definition(ui, editor, &checks, version);
            } else {
                let runnable = checks.runnable();
                let mut run = false;
                Section::new("Run")
                    .description("The rule as it stands, unsaved edits included, tried for each site under the unit in each month.")
                    .actions(|ui| {
                        let button = small("Run", Some(Icon::Play), ButtonVariant::Default).enabled(runnable);
                        run = ui.add(button).clicked();
                    })
                    .show(ui, |ui| rule_run(ui, editor, &mut views.violations));
                if run {
                    toast = editor.start(&checks);
                }
            }
        });
    });
    if let Some(toast) = toast {
        acts.push(Act::new(move |app| app.backend.toast(toast)));
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
        let out = SaveBar::new(changed).busy(editor.busy).show(&mut foot);
        if out.discard {
            editor.discard();
        }
        if out.save {
            acts.push(Act::new(|app| {
                if let Some(editor) = &mut app.views.rule {
                    editor.save(&mut app.backend);
                }
            }));
        }
    }
}

fn rule_definition(ui: &mut Ui, editor: &mut validation_rule::Editor, checks: &validation_rule::Checks, version: &str) {
    Section::new("Definition")
        .description("Each side is an expression over the data; the rule holds where the left side stands to the right as the operator says.")
        .show(ui, |ui| {
            Stack::new(Gap::Lg).show(ui, |ui| {
                use crate::backend::Importance;
                let importances = [
                    (Importance::High, "High".to_owned()),
                    (Importance::Medium, "Medium".to_owned()),
                    (Importance::Low, "Low".to_owned()),
                ];
                pair(ui, |ui, at| {
                    if at == 0 {
                        text_field(ui, "Name", &mut editor.draft.name, true, false);
                    } else {
                        select_field(ui, "Importance", &mut editor.draft.importance, &importances, None);
                    }
                });
                text_field(ui, "Instruction", &mut editor.draft.instruction, false, true);
                let strategies: Vec<_> = STRATEGIES
                    .iter()
                    .map(|one| (*one, one.label().to_owned()))
                    .collect();
                let operators: Vec<_> = validation_rule::operator_options()
                    .into_iter()
                    .zip(OPERATORS)
                    .map(|((_, label), one)| (one, label))
                    .collect();
                ExpressionField {
                    id: "rule-left",
                    label: "Left side",
                    language: Language::Expression,
                    checked: &checks.left,
                    version,
                    variables: &[],
                    description: None,
                }
                .show(ui, &mut editor.draft.left);
                let help = editor.draft.left_strategy.description();
                select_field(
                    ui,
                    "When the left side has missing values",
                    &mut editor.draft.left_strategy,
                    &strategies,
                    Some(help),
                );
                select_field(ui, "Operator", &mut editor.draft.operator, &operators, None);
                ExpressionField {
                    id: "rule-right",
                    label: "Right side",
                    language: Language::Expression,
                    checked: &checks.right,
                    version,
                    variables: &[],
                    description: None,
                }
                .show(ui, &mut editor.draft.right);
                let help = editor.draft.right_strategy.description();
                select_field(
                    ui,
                    "When the right side has missing values",
                    &mut editor.draft.right_strategy,
                    &strategies,
                    Some(help),
                );
            });
        });
}

fn rule_run(ui: &mut Ui, editor: &mut validation_rule::Editor, violations: &mut sokkel_egui::TableState) {
    Stack::new(Gap::Lg).show(ui, |ui| {
        let tree = tree_nodes(&org_unit_tree());
        Field::new("Organization unit").show(ui, |ui| {
            let mut value = Some(editor.unit.clone());
            if TreeSelect::new("rule-unit", &tree, &mut value)
                .expanded([root()])
                .show(ui)
            {
                editor.unit = value.unwrap_or_else(|| root().to_owned());
            }
        });
        let periods: Vec<(usize, String)> = validation_rule::period_options()
            .into_iter()
            .enumerate()
            .map(|(at, (_, label))| (at, label))
            .collect();
        let ids: Vec<String> = validation_rule::period_options()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        pair(ui, |ui, at| {
            let (label, value) = if at == 0 {
                ("From", &mut editor.from)
            } else {
                ("To", &mut editor.to)
            };
            let mut index = ids.iter().position(|one| one == value).unwrap_or(0);
            if select_field(ui, label, &mut index, &periods, None) {
                *value = ids[index].clone();
            }
        });
        if editor.run.is_some() {
            let stats: Vec<Stat> = editor
                .stats()
                .into_iter()
                .map(|one| Stat::new(one.label, one.value).note(&one.note).emphasis(one.emphasis))
                .collect();
            StatTiles::new("rule-stats", 3, 120.0, 12.0).show(ui, stats);
            let rows = editor.rows();
            let columns = [
                Column::new("Site", 160.0).fill(),
                Column::new("Month", 96.0),
                Column::new("Left side", 88.0).numeric(),
                Column::new("Must be", 72.0).sortable(false),
                Column::new("Right side", 88.0).numeric(),
            ];
            let cells: Vec<Vec<Cell>> = rows
                .iter()
                .map(|one| {
                    vec![
                        Cell::Lines {
                            title: one.site.clone(),
                            detail: one.path.clone(),
                            over: false,
                        },
                        Cell::Text(one.month.clone()),
                        Cell::Text(one.left.clone()),
                        Cell::Code(one.sign.clone()),
                        Cell::Text(one.right.clone()),
                    ]
                })
                .collect();
            DataTable::new("rule-violations", &columns, &cells, violations)
                .keys(rows.iter().map(|one| one.id.clone()))
                .variant(TableVariant::List)
                .auto_rows()
                .fits(true)
                .footer(false)
                .height(f32::INFINITY)
                .words(TableWords {
                    empty: "The rule holds for every site and month it was tried on.".to_owned(),
                    ..Default::default()
                })
                .show(ui);
        }
    });
}
