//! The pages, as the web's `pages/*.tsx` draw them over each page's own logic in `pages`: the
//! indicators, data elements, validation rules and functions as tables, an indicator and a
//! validation rule edited beside where they are tried, the program's rules beside the form they
//! act on, and the organization units in their tree. Each page says what the status line holds
//! (`foot`); what it asks of the app comes back as acts.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    ApiLink, Button, ButtonSize, ButtonVariant, Cell, CellRun, ChipTone, Column, DataTable, DescriptionList,
    DetailSheet, FilterChip, Icon, NotFound, Page, PageHeader, PageState, Section, Stack, TableVariant, TableWords,
    TextLink, TextRole, tokens,
};

use crate::metadata::{latest_period, root};
use crate::pages::{data_elements, functions, indicators, org_units, program_rules, validation_rules};
use crate::ui::{Verdict, describe_items};
use crate::values::data_value;
use crate::workbench::{Asking, Views, WorkbenchApp};

/// Something a page asks of the app, done once the frame is drawn.
pub struct Act(Box<dyn FnOnce(&mut WorkbenchApp)>);

impl Act {
    pub fn new(does: impl FnOnce(&mut WorkbenchApp) + 'static) -> Self {
        Act(Box::new(does))
    }

    pub fn go(path: impl Into<String>) -> Self {
        let path = path.into();
        Act::new(move |app| app.go(&path))
    }

    pub fn ask(asking: Asking) -> Self {
        Act::new(move |app| app.asking = Some(asking))
    }
}

impl WorkbenchApp {
    /// Does what a page asked.
    pub fn apply(&mut self, act: Act) {
        (act.0)(self)
    }
}

/// A small button, outlined, ghost or the primary one, with an icon when given.
pub fn small(label: &str, icon: Option<Icon>, variant: ButtonVariant) -> Button {
    let button = Button::new(label).size(ButtonSize::Sm).variant(variant);
    match icon {
        Some(icon) => button.icon(icon),
        None => button,
    }
}

/// A state's chip color, as the web's `STATUSES` map it.
pub fn tone_of(state: &str) -> ChipTone {
    match crate::ui::STATUSES
        .iter()
        .find(|(key, _)| *key == state)
        .map(|(_, tone)| *tone)
    {
        Some("good") => ChipTone::Good,
        Some("warning") => ChipTone::Warning,
        Some("critical") => ChipTone::Critical,
        _ => ChipTone::Neutral,
    }
}

/// A count of rows in a table's bar: `9 rows`.
fn words_empty(empty: Option<&str>) -> TableWords {
    match empty {
        Some(empty) => TableWords {
            empty: empty.to_owned(),
            ..Default::default()
        },
        None => TableWords::default(),
    }
}

/// The uids of the indicators the server holds, which an `N{uid}` names.
pub fn indicator_ids(app: &WorkbenchApp) -> Vec<String> {
    app.backend.indicators.seen().iter().map(|one| one.id.clone()).collect()
}

/// What the status line says at its start for the page shown, as each page's `useFoot`.
pub fn foot(app: &WorkbenchApp) -> String {
    let version = app.version();
    let views = &app.views;
    let first = app.route.first().map(String::as_str);
    let second = app.route.get(1).map(String::as_str);
    match (crate::app::page_of(&app.route), first, second) {
        (crate::app::Page::Indicators, ..) => {
            let worked = indicators::worked(app.backend.indicators.seen(), version, &indicator_ids(app));
            let view = &views.indicators;
            let shown = indicators::shown(&worked, &view.query, &view.first, &view.second);
            indicators::foot(shown.len(), worked.len())
        }
        (crate::app::Page::Indicator(_), ..) => match &views.indicator {
            Some(editor) => editor.foot(&editor.checks(version, &indicator_ids(app))),
            None => String::new(),
        },
        (crate::app::Page::ValidationRules, ..) => {
            let worked = validation_rules::worked(app.backend.validation_rules.seen(), version, &indicator_ids(app));
            validation_rules::foot(&worked)
        }
        (crate::app::Page::ValidationRule(_), ..) => views.rule.as_ref().map_or_else(String::new, |one| one.foot()),
        (crate::app::Page::ProgramRules(selected), ..) => {
            let rows = app.backend.program_rules.seen();
            let rules = views.program.rules(rows, selected.as_deref());
            program_rules::TryForm::foot(&views.program.form.run(&rules, version))
        }
        (crate::app::Page::DataElements(_), ..) => {
            let shown = data_elements::rows(
                &views.elements.query,
                app.backend.indicators.seen(),
                app.backend.validation_rules.seen(),
            );
            data_elements::foot(shown.len())
        }
        (crate::app::Page::OrgUnits(selected), ..) => org_units::foot(org_units::unit_of(selected.as_deref())),
        (crate::app::Page::Functions, ..) => functions::foot(version, &functions::rows(version)),
        (crate::app::Page::NotFound, ..) => String::new(),
    }
}

/// The page the address names.
pub fn page(ui: &mut Ui, app: &WorkbenchApp, views: &mut Views, acts: &mut Vec<Act>) {
    match crate::app::page_of(&app.route) {
        crate::app::Page::Indicators => indicators_page(ui, app, views, acts),
        crate::app::Page::Indicator(id) => crate::detail::indicator_page(ui, app, &id, views, acts),
        crate::app::Page::ValidationRules => rules_page(ui, app, views, acts),
        crate::app::Page::ValidationRule(id) => crate::detail::rule_page(ui, app, &id, views, acts),
        crate::app::Page::ProgramRules(selected) => {
            crate::program_page::program_rules_page(ui, app, selected.as_deref(), views, acts)
        }
        crate::app::Page::DataElements(selected) => elements_page(ui, app, selected.as_deref(), views, acts),
        crate::app::Page::OrgUnits(selected) => units_page(ui, app, selected.as_deref(), views, acts),
        crate::app::Page::Functions => functions_page(ui, app, views),
        crate::app::Page::NotFound => {
            Page::show(ui, |ui| {
                NotFound::new(app.route.join("/")).show(ui);
            });
        }
    }
}

// --- Indicators ------------------------------------------------------------------------------------

fn indicators_page(ui: &mut Ui, app: &WorkbenchApp, views: &mut Views, acts: &mut Vec<Act>) {
    let version = app.version();
    let loading = app.backend.indicators.seen.is_none();
    let worked = indicators::worked(app.backend.indicators.seen(), version, &indicator_ids(app));
    let view = &mut views.indicators;
    Page::show(ui, |ui| {
        PageHeader::new("Indicators")
            .description(indicators::description(version))
            .show(ui, |ui| {
                if ui.add(small("New", Some(Icon::Plus), ButtonVariant::Default)).clicked() {
                    let count = worked.len();
                    acts.push(Act::new(move |app| indicators::create(&mut app.backend, count)));
                }
                ui.add(ApiLink::new("/api/indicators"));
            });
        if loading {
            PageState::loading().show(ui);
            return;
        }
        let shown: Vec<indicators::Worked> = indicators::shown(&worked, &view.query, &view.first, &view.second)
            .into_iter()
            .cloned()
            .collect();
        let columns = [
            Column::new("Name", 224.0).fill_share(1.2),
            Column::new("Type", 128.0),
            Column::new("Formula", 256.0).fill_share(2.0).sortable(false),
            Column::new("Check", 128.0),
            Column::new(indicators::value_header(), 160.0).numeric(),
        ];
        let saved = app.backend.indicators.seen();
        let cells: Vec<Vec<Cell>> = shown
            .iter()
            .map(|one| {
                let row = saved.iter().find(|row| row.id == one.id);
                let (top, bottom) = row.map_or((String::new(), String::new()), |row| {
                    (describe_items(&row.numerator), describe_items(&row.denominator))
                });
                vec![
                    Cell::NamedCode {
                        name: one.name.clone(),
                        code: one.id.clone(),
                        strong: true,
                    },
                    Cell::Text(one.kind.label().to_owned()),
                    Cell::Runs(vec![
                        CellRun::new(format!("{top} "), "muted-foreground"),
                        CellRun::new("over", "faint"),
                        CellRun::new(format!(" {bottom}"), "muted-foreground"),
                    ]),
                    Cell::Chip(one.check.clone(), tone_of(one.state.key())),
                    Cell::Number {
                        value: one.value.unwrap_or(f64::NEG_INFINITY),
                        text: one.shown.clone(),
                    },
                ]
            })
            .collect();
        let type_options: Vec<(String, String)> = indicators::type_options()
            .into_iter()
            .map(|(value, label)| (value.to_owned(), label.to_owned()))
            .collect();
        let check_options: Vec<(String, String)> = ["valid", "warnings", "invalid"]
            .iter()
            .map(|one| ((*one).to_owned(), (*one).to_owned()))
            .collect();
        let empty = indicators::empty_words(&view.query, &view.first, &view.second);
        let foot = indicators::table_foot(&worked, version);
        let (first, second) = (&mut view.first, &mut view.second);
        let mut table = DataTable::new("indicators", &columns, &cells, &mut view.table)
            .keys(shown.iter().map(|one| one.id.clone()))
            .search(&mut view.query)
            .search_placeholder("Search")
            .toolbar(|ui| {
                FilterChip::new("indicators-type", "Type", &type_options, first).show(ui);
                FilterChip::new("indicators-check", "Check", &check_options, second).show(ui);
            })
            .chevron(true)
            .stretch(true)
            .auto_rows()
            .footer(false)
            .words(words_empty(empty));
        if let Some(foot) = foot {
            table = table.foot(foot);
        }
        table.show(ui);
        if let Some(index) = view.table.opened.take()
            && let Some(one) = shown.get(index)
        {
            acts.push(Act::go(format!("indicators/{}", one.id)));
        }
    });
}

// --- Validation rules -------------------------------------------------------------------------------

fn rules_page(ui: &mut Ui, app: &WorkbenchApp, views: &mut Views, acts: &mut Vec<Act>) {
    let version = app.version();
    let loading = app.backend.validation_rules.seen.is_none();
    let rows = app.backend.validation_rules.seen();
    let worked = validation_rules::worked(rows, version, &indicator_ids(app));
    let view = &mut views.rules;
    Page::show(ui, |ui| {
        PageHeader::new("Validation rules")
            .description(validation_rules::description(version))
            .show(ui, |ui| {
                if ui.add(small("New", Some(Icon::Plus), ButtonVariant::Default)).clicked() {
                    let count = worked.len();
                    acts.push(Act::new(move |app| validation_rules::create(&mut app.backend, count)));
                }
                ui.add(ApiLink::new("/api/validationRules"));
            });
        if loading {
            PageState::loading().show(ui);
            return;
        }
        let shown: Vec<validation_rules::Worked> = validation_rules::shown(&worked, &view.query)
            .into_iter()
            .cloned()
            .collect();
        let columns = [
            Column::new("Name", 208.0).fill(),
            Column::new("Importance", 112.0),
            Column::new("Rule", 256.0).fill_share(2.0).sortable(false),
            Column::new("Check", 112.0),
            Column::new("Violations", 112.0).numeric(),
        ];
        let cells: Vec<Vec<Cell>> = shown
            .iter()
            .map(|one| {
                let row = rows.iter().find(|row| row.id == one.id);
                let rule = match row {
                    Some(row) => vec![
                        CellRun::new(format!("{} ", describe_items(&row.left)), "muted-foreground"),
                        CellRun::new(row.operator.sign(), "foreground").mono(),
                        CellRun::new(format!(" {}", describe_items(&row.right)), "muted-foreground"),
                    ],
                    None => vec![CellRun::new(one.rule.clone(), "muted-foreground")],
                };
                vec![
                    Cell::NamedCode {
                        name: one.name.clone(),
                        code: one.id.clone(),
                        strong: true,
                    },
                    Cell::Chip(one.importance.key().to_owned(), tone_of(one.importance.key())),
                    Cell::Runs(rule),
                    Cell::Chip(one.state.key().to_owned(), tone_of(one.state.key())),
                    match one.violations {
                        None => Cell::Number {
                            value: -1.0,
                            text: "\u{2013}".to_owned(),
                        },
                        Some(count) if count > 0 => Cell::Toned(count.to_string(), "critical-ink"),
                        Some(count) => Cell::Number {
                            value: count as f64,
                            text: count.to_string(),
                        },
                    },
                ]
            })
            .collect();
        let empty = validation_rules::empty_words(&view.query);
        DataTable::new("validation-rules", &columns, &cells, &mut view.table)
            .keys(shown.iter().map(|one| one.id.clone()))
            .search(&mut view.query)
            .search_placeholder("Search")
            .chevron(true)
            .stretch(true)
            .auto_rows()
            .footer(false)
            .words(words_empty(empty))
            .show(ui);
        if let Some(index) = view.table.opened.take()
            && let Some(one) = shown.get(index)
        {
            acts.push(Act::go(format!("validation-rules/{}", one.id)));
        }
    });
}

// --- Data elements ----------------------------------------------------------------------------------

fn elements_page(ui: &mut Ui, app: &WorkbenchApp, selected: Option<&str>, views: &mut Views, acts: &mut Vec<Act>) {
    let indicators = app.backend.indicators.seen();
    let rules = app.backend.validation_rules.seen();
    let view = &mut views.elements;
    let shown = data_elements::rows(&view.query, indicators, rules);
    Page::show(ui, |ui| {
        PageHeader::new("Data elements")
            .description(data_elements::description())
            .show(ui, |ui| {
                ui.add(ApiLink::new("/api/dataElements"));
            });
        let columns = [
            Column::new("Name", 208.0).fill_share(1.4),
            Column::new("Code", 176.0),
            Column::new("Disaggregation", 144.0),
            Column::new("Value type", 192.0).sortable(false),
            Column::new("Read by", 144.0).sortable(false),
            Column::new(data_elements::value_header(), 144.0).numeric(),
        ];
        let cells: Vec<Vec<Cell>> = shown
            .iter()
            .map(|one| {
                let value = data_value(&one.id, None, root(), latest_period());
                vec![
                    Cell::NamedCode {
                        name: one.name.clone(),
                        code: one.id.clone(),
                        strong: true,
                    },
                    Cell::Mono(one.code.clone()),
                    match &one.disaggregation {
                        Some(name) => Cell::Chip(name.clone(), ChipTone::Kind("violet")),
                        None => Cell::Muted("none".to_owned()),
                    },
                    Cell::Mono(one.value_type.to_owned()),
                    match &one.read_by {
                        Some(read) => Cell::Text(read.clone()),
                        None => Cell::Muted("nothing".to_owned()),
                    },
                    Cell::Number {
                        value: value.unwrap_or(-1.0),
                        text: one.value.clone(),
                    },
                ]
            })
            .collect();
        let opened = selected.and_then(|id| shown.iter().position(|one| one.id == id));
        view.table.opened = None;
        DataTable::new("data-elements", &columns, &cells, &mut view.table)
            .keys(shown.iter().map(|one| one.id.clone()))
            .opened(opened)
            .search(&mut view.query)
            .search_placeholder("Search")
            .chevron(true)
            .stretch(true)
            .auto_rows()
            .footer(false)
            .words(words_empty(Some(data_elements::EMPTY)))
            .show(ui);
        if let Some(one) = view
            .table
            .opened
            .take()
            .filter(|index| Some(*index) != opened)
            .and_then(|index| shown.get(index))
        {
            acts.push(Act::go(format!("data-elements/{}", one.id)));
        }
    });
    let Some(sheet) = data_elements::sheet(selected, indicators, rules) else {
        return;
    };
    let ctx = ui.ctx().clone();
    let closed = DetailSheet::new("data-element-sheet", &sheet.title, &mut views.sheet)
        .code(&sheet.code)
        .description(&sheet.description)
        .show(&ctx, |ui| {
            let t = tokens(ui.ctx());
            Stack::new(sokkel_egui::Gap::Lg).show(ui, |ui| {
                let mut facts = DescriptionList::new().side(144.0);
                for term in &sheet.facts {
                    facts = if ["Code", "Value type", "Aggregation"].contains(&term.term.as_str()) {
                        facts.code(&term.term, &term.value)
                    } else {
                        facts.text(&term.term, &term.value)
                    };
                }
                facts.show(ui);
                Section::new("Category option combos").flat(true).show(ui, |ui| {
                    let mut combos = DescriptionList::new().side(144.0);
                    for term in &sheet.combos {
                        combos = combos.code(&term.term, &term.value);
                    }
                    combos.show(ui);
                });
                Section::new(&sheet.read_title).flat(true).show(ui, |ui| {
                    if let Some(none) = sheet.none {
                        ui.add(
                            egui::Label::new(TextRole::Help.rich(&t, none).color(t.color("muted-foreground"))).wrap(),
                        );
                    }
                    ui.spacing_mut().item_spacing.y = 4.0;
                    for reader in &sheet.readers {
                        ui.horizontal(|ui| {
                            if ui.add(TextLink::new(&reader.name)).clicked() {
                                acts.push(Act::go(reader.href.clone()));
                            }
                            ui.label(TextRole::Help.rich(&t, reader.kind).color(t.color("muted-foreground")));
                        });
                    }
                });
            });
        });
    if closed {
        acts.push(Act::go("data-elements"));
    }
}

// --- Organization units -----------------------------------------------------------------------------

fn units_page(ui: &mut Ui, _app: &WorkbenchApp, selected: Option<&str>, views: &mut Views, acts: &mut Vec<Act>) {
    let unit = org_units::unit_of(selected);
    if views.units_for.as_deref() != Some(unit.id.as_str()) {
        views.units_open.extend(org_units::default_expanded(unit));
        views.units_for = Some(unit.id.clone());
    }
    let tree = tree_nodes(&crate::metadata::org_unit_tree());
    let t = tokens(ui.ctx());
    let container = ui.available_width();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let mut column = views.column;
        let query = views.units.query.clone();
        let mut asked = query.clone();
        let mut chosen = None;
        sokkel_egui::SubNav::new("units-hierarchy", &mut column).show(
            ui,
            |ui| {
                sokkel_egui::SearchField::new("units-search", &mut asked)
                    .keys("/")
                    .show(ui);
            },
            |ui| {
                let mut picked = Some(unit.id.clone());
                let out = sokkel_egui::Tree::new("units-tree", &tree, &mut picked, &mut views.units_open)
                    .query(&query)
                    .show(ui);
                chosen = out.chosen;
            },
        );
        views.column = column;
        views.units.query = asked;
        if let Some(id) = chosen.filter(|id| *id != unit.id) {
            acts.push(Act::go(org_units::route_of(Some(&id))));
        }
        let room = ui.available_rect_before_wrap();
        let mut page = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(room)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        Page::scroll_in(&mut page, container, |ui| {
            PageHeader::new(&unit.name)
                .code(&unit.id)
                .description(org_units::description(unit))
                .show(ui, |ui| {
                    ui.add(ApiLink::new(format!("/api/organisationUnits/{}", unit.id)));
                });
            let gap = t.density("inspector-gap");
            let facts = views.units.facts(unit);
            sokkel_egui::Columns::new(2)
                .min(288.0)
                .gap(gap)
                .show(ui, 2, |ui, index, width| {
                    ui.set_width(width);
                    if index == 0 {
                        let cell = std::cell::RefCell::new(None::<String>);
                        let to = &cell;
                        Section::new("Facts").show(ui, |ui| {
                            let mut list = DescriptionList::new().side(124.0);
                            for fact in &facts {
                                list = match fact.term.as_str() {
                                    "Code" => list.code(&fact.term, &fact.value),
                                    "Path" => {
                                        let path = crate::metadata::path_of(&unit.id);
                                        let current = unit.id.clone();
                                        list.item(&fact.term, move |ui| {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.spacing_mut().item_spacing.x = 0.0;
                                                for (at, one) in path.iter().enumerate() {
                                                    if at > 0 {
                                                        ui.label(" / ");
                                                    }
                                                    if one.id == current {
                                                        ui.label(&one.name);
                                                    } else if ui.add(TextLink::new(&one.name)).clicked() {
                                                        *to.borrow_mut() = Some(one.id.clone());
                                                    }
                                                }
                                            });
                                        })
                                    }
                                    _ => list.text(&fact.term, &fact.value),
                                };
                            }
                            list.show(ui);
                        });
                        if let Some(id) = cell.into_inner() {
                            acts.push(Act::go(org_units::route_of(Some(&id))));
                        }
                    } else {
                        let year = org_units::year(unit);
                        let spec = line_spec(&year, "rate", "%", "Reporting rate");
                        sokkel_egui_charts::ChartPanel::new(
                            "Reporting rate",
                            sokkel_egui_charts::ChartKind::Line,
                            &spec,
                        )
                        .subtitle("sites that sent the monthly report")
                        .legend(false)
                        .height(214.0)
                        .show(ui);
                    }
                });
            let values = views.units.values(unit);
            let mut period = Some(views.units.period.clone());
            Section::new("Data values")
                .description(org_units::values_description(unit))
                .actions(|ui| {
                    sokkel_egui::PeriodPicker::new(
                        "org-unit-period",
                        sokkel_egui::periods::PeriodType::Monthly,
                        &mut period,
                        today(),
                    )
                    .width(188.0)
                    .show(ui);
                })
                .show(ui, |ui| {
                    let columns = [
                        Column::new("Data element", 192.0).fill_share(1.4),
                        Column::new("Category option combo", 144.0).fill().sortable(false),
                        Column::new("Value", 128.0).numeric(),
                    ];
                    let cells: Vec<Vec<Cell>> = values
                        .iter()
                        .map(|one| {
                            vec![
                                Cell::Text(one.element.clone()),
                                if one.combo == "default" {
                                    Cell::Muted(one.combo.clone())
                                } else {
                                    Cell::Text(one.combo.clone())
                                },
                                Cell::Number {
                                    value: one.value.unwrap_or(-1.0),
                                    text: one.shown.clone(),
                                },
                            ]
                        })
                        .collect();
                    DataTable::new("unit-values", &columns, &cells, &mut views.values)
                        .variant(TableVariant::List)
                        .fits(true)
                        .footer(false)
                        .height(f32::INFINITY)
                        .words(words_empty(Some("No data values.")))
                        .show(ui);
                });
            if let Some(period) = period
                && period != views.units.period
            {
                views.units.period = period;
            }
        });
    });
}

/// The metadata's tree as the core's tree nodes.
pub fn tree_nodes(nodes: &[crate::metadata::TreeNode]) -> Vec<sokkel_egui::TreeNode> {
    nodes
        .iter()
        .map(|one| {
            let mut node = sokkel_egui::TreeNode::new(&one.id, &one.label).meta(&one.meta);
            if let Some(children) = &one.children {
                node = node.children(tree_nodes(children));
            }
            node
        })
        .collect()
}

/// The day the period pickers count back from.
pub fn today() -> sokkel_egui::Day {
    sokkel_egui::Day::parse(crate::metadata::TODAY).unwrap_or_else(sokkel_egui::Day::today)
}

/// A line over the months: `month` along and `field` up, in `unit`, the field named `name`.
pub fn line_spec(year: &[(String, Option<f64>)], field: &str, unit: &str, name: &str) -> sokkel_egui_charts::ChartSpec {
    use sokkel_egui_charts::data::{Cell as Value, Row};
    let rows: Vec<Row> = year
        .iter()
        .map(|(month, value)| Row::of([("month", Value::from(month.as_str())), (field, Value::from(*value))]))
        .collect();
    sokkel_egui_charts::ChartSpec::new(rows)
        .x("month")
        .y(field)
        .unit(unit)
        .field_name(field, name)
        .field_name("month", "Month")
}

// --- Functions -------------------------------------------------------------------------------------

fn functions_page(ui: &mut Ui, app: &WorkbenchApp, views: &mut Views) {
    let version = app.version();
    let rows = functions::rows(version);
    let view = &mut views.functions;
    Page::show(ui, |ui| {
        PageHeader::new("Functions")
            .description(functions::description(version, &rows))
            .show(ui, |_| {});
        let shown: Vec<functions::Name> = functions::shown(&rows, &view.query, &view.first, &view.second)
            .into_iter()
            .cloned()
            .collect();
        let on = format!("On {version}");
        let columns = [
            Column::new("Name", 192.0).fill_share(1.4),
            Column::new("Kind", 112.0),
            Column::new("Language", 160.0),
            Column::new("Since", 144.0),
            Column::new(&on, 128.0),
            Column::new("Run here", 112.0),
        ];
        let cells: Vec<Vec<Cell>> = shown
            .iter()
            .map(|one| {
                vec![
                    Cell::Mono(one.name.clone()),
                    Cell::Text(one.kind.to_owned()),
                    if one.languages == "rule" {
                        Cell::Chip("dhis2-rule".to_owned(), ChipTone::Kind("pink"))
                    } else {
                        Cell::Chip("both".to_owned(), ChipTone::Kind("teal"))
                    },
                    Cell::Text(one.since.clone()),
                    if one.known {
                        Cell::Chip("known".to_owned(), tone_of(Verdict::Valid.key()))
                    } else {
                        Cell::Chip("unknown".to_owned(), tone_of(Verdict::Invalid.key()))
                    },
                    if one.evaluated {
                        Cell::Text("yes".to_owned())
                    } else {
                        Cell::Muted("no".to_owned())
                    },
                ]
            })
            .collect();
        let kinds: Vec<(String, String)> = ["function", "variable"]
            .iter()
            .map(|one| ((*one).to_owned(), (*one).to_owned()))
            .collect();
        let known: Vec<(String, String)> = ["known", "unknown"]
            .iter()
            .map(|one| ((*one).to_owned(), (*one).to_owned()))
            .collect();
        let (first, second) = (&mut view.first, &mut view.second);
        DataTable::new("functions", &columns, &cells, &mut view.table)
            .keys(shown.iter().map(|one| one.id.clone()))
            .search(&mut view.query)
            .search_placeholder("Search")
            .toolbar(|ui| {
                FilterChip::new("functions-kind", "Kind", &kinds, first).show(ui);
                FilterChip::new("functions-known", &on, &known, second).show(ui);
            })
            .stretch(true)
            .footer(false)
            .words(words_empty(Some(functions::EMPTY)))
            .show(ui);
    });
}
