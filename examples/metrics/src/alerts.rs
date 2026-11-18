//! Alert rules on thresholds and what they fired, as the web's `Alerts.tsx` and `RuleDialog.tsx`.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, Cell, Chip, ChipTone, Column, ConfirmAnswer, ConfirmDialog, DataTable, Dialog,
    Field, Gap, Icon, MenuItem, NumberField, PageHeader, Segmented, Select, Stack, SwitchField, TableState,
    TableVariant, TextField, Toast, toolbar,
};
use sokkel_egui_charts::{Fill as ChartFill, Series, TimeSeriesPanel};

use crate::MetricsApp;
use crate::alerting::{AlertRule, Condition, RuleState, Severity, condition_of, crosses, written};
use crate::query::{Query, Range, clock_of, read_series, span_of, time_label};
use crate::source::{ALL, Aggregation, HOSTS, Label, METRICS, MetricId, SERVICES, metric_of};

/// Which list the Alerts page shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlertsTab {
    #[default]
    Rules,
    Fired,
}

/// Which fired alerts the list shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FiredFilter {
    #[default]
    All,
    Firing,
    Resolved,
}

/// A question the Alerts page is asking over it.
#[derive(Clone, Debug, PartialEq)]
pub enum AlertsAsking {
    DeleteRule(String),
    ClearResolved,
}

/// The Alerts page's own state between frames.
#[derive(Default)]
pub struct AlertsPage {
    pub tab: AlertsTab,
    pub filter: FiredFilter,
    pub asking: Option<AlertsAsking>,
    /// A rule being made or changed.
    pub rule: Option<RuleForm>,
    pub(crate) rules_table: TableState,
    pub(crate) fired_table: TableState,
}

/// A rule in the rule dialog: the one it changes, by its id, or a new one.
#[derive(Clone, Debug)]
pub struct RuleForm {
    pub editing: Option<String>,
    pub draft: AlertRule,
}

/// A new rule as the dialog opens on it, as the web's `FRESH`.
pub fn fresh_rule() -> AlertRule {
    AlertRule {
        id: String::new(),
        name: String::new(),
        metric: MetricId::LatencyP95,
        aggregation: Aggregation::Avg,
        filter: vec![(Label::Service, "api".into())],
        condition: Condition::Above,
        threshold: 200.0,
        for_seconds: 15.0,
        severity: Severity::Warning,
        enabled: true,
    }
}

const HOLDS: [(f64, &str); 6] = [
    (0.0, "At once"),
    (10.0, "10s"),
    (15.0, "15s"),
    (30.0, "30s"),
    (60.0, "1m"),
    (300.0, "5m"),
];

fn aggregation_label(aggregation: Aggregation) -> &'static str {
    match aggregation {
        Aggregation::Sum => "Sum",
        Aggregation::Avg => "Mean",
        Aggregation::Max => "Max",
        Aggregation::Min => "Min",
    }
}

/// A rule's or an alert's state as its chip, in the colors the web's `ALERT_STATUSES` give it.
fn state_chip(word: &str) -> Cell {
    let tone = match word {
        "firing" | "critical" => ChipTone::Critical,
        "pending" | "warning" => ChipTone::Warning,
        "normal" => ChipTone::Good,
        _ => ChipTone::Neutral,
    };
    Cell::Chip(word.to_owned(), tone)
}

/// How long an alert lasted, or has so far.
fn lasted(fired: f64, resolved: Option<f64>, now: f64) -> String {
    let seconds = (resolved.unwrap_or(now) - fired).max(0.0) as u64;
    if seconds < 60 {
        format!("{seconds}s")
    } else {
        format!("{}m {:02}s", seconds / 60, seconds % 60)
    }
}

fn filter_value(rule: &AlertRule, label: Label) -> Option<&str> {
    rule.filter.iter().find(|(l, _)| *l == label).map(|(_, v)| v.as_str())
}

fn set_filter(rule: &mut AlertRule, label: Label, value: Option<&str>) {
    rule.filter.retain(|(l, _)| *l != label);
    if let Some(value) = value {
        rule.filter.push((label, value.to_owned()));
    }
}

impl MetricsApp {
    /// The Alerts page: its head, the toolbar of lists, and the rules or what fired.
    pub(crate) fn alerts_page(&mut self, ui: &mut Ui) {
        let now = self.at;
        let firing_now = self.alerting.events.iter().filter(|e| e.resolved_at.is_none()).count();
        let resolved = self.alerting.events.len() - firing_now;
        let mut new_rule = false;
        PageHeader::new("Alerts")
            .count(if firing_now == 0 {
                "nothing firing".to_owned()
            } else {
                format!("{firing_now} firing")
            })
            .description(
                "Each rule is read every second from the newest scrape. One that holds past its threshold for long enough fires, and resolves once it is back.",
            )
            .show(ui, |ui| {
                new_rule = ui
                    .add(Button::new("New rule").icon(Icon::Plus).size(ButtonSize::Sm))
                    .clicked();
            });
        if new_rule {
            self.alerts.rule = Some(RuleForm {
                editing: None,
                draft: fresh_rule(),
            });
        }
        let t = sokkel_egui::tokens(ui.ctx());
        let (rules_count, fired_count) = (self.alerting.rules.len(), self.alerting.events.len());
        let page = &mut self.alerts;
        let tab = page.tab;
        let mut clear = false;
        toolbar(
            ui,
            |ui| {
                let mut at = usize::from(page.tab == AlertsTab::Fired);
                if ui
                    .add(Segmented::new(&mut at, ["Rules", "Fired"]).counts([Some(rules_count), Some(fired_count)]))
                    .changed()
                {
                    page.tab = if at == 0 { AlertsTab::Rules } else { AlertsTab::Fired };
                }
                if page.tab == AlertsTab::Fired {
                    let mut at = match page.filter {
                        FiredFilter::All => 0,
                        FiredFilter::Firing => 1,
                        FiredFilter::Resolved => 2,
                    };
                    let which = Segmented::new(&mut at, ["All", "Firing", "Resolved"])
                        .counts([None, Some(firing_now), Some(resolved)])
                        .dots([None, Some(t.color("critical")), None]);
                    if ui.add(which).changed() {
                        page.filter = [FiredFilter::All, FiredFilter::Firing, FiredFilter::Resolved][at];
                    }
                }
            },
            |ui| {
                if tab == AlertsTab::Fired {
                    let mut button = Button::new("Clear resolved")
                        .icon(Icon::Eraser)
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm);
                    if resolved == 0 {
                        button = button.reason("Nothing has resolved yet.");
                    }
                    if ui.add(button).clicked() && resolved > 0 {
                        clear = true;
                    }
                }
            },
        );
        if clear {
            self.alerts.asking = Some(AlertsAsking::ClearResolved);
        }
        match self.alerts.tab {
            AlertsTab::Rules => self.rules_list(ui),
            AlertsTab::Fired => self.fired_list(ui, now),
        }
    }

    fn rules_list(&mut self, ui: &mut Ui) {
        // the web's 7rem, `minmax(10rem,1fr)`, `minmax(16rem,2fr)`, 7rem, 7rem and 4rem, less the
        // 12 between columns
        let columns = [
            Column::new("State", 100.0),
            Column::new("Name", 148.0).fill(),
            Column::new("Condition", 244.0).fill_share(2.0).sortable(false),
            Column::new("Now", 100.0).numeric(),
            Column::new("Severity", 100.0),
            Column::new("On", 52.0).sortable(false),
        ];
        let rows: Vec<Vec<Cell>> = self
            .alerting
            .rules
            .iter()
            .map(|rule| {
                let reading = self.alerting.readings.get(&rule.id);
                let state = reading.map_or(
                    if rule.enabled {
                        RuleState::Normal
                    } else {
                        RuleState::Paused
                    },
                    |r| r.state,
                );
                let value = reading.and_then(|r| r.value);
                vec![
                    state_chip(state.word()),
                    Cell::Strong(rule.name.clone()),
                    Cell::Muted(condition_of(rule)),
                    Cell::Number {
                        value: value.unwrap_or(0.0),
                        text: written(rule.metric, value),
                    },
                    state_chip(rule.severity.word()),
                    Cell::Switch {
                        on: rule.enabled,
                        label: if rule.enabled {
                            format!("Turn {} off", rule.name)
                        } else {
                            format!("Turn {} on", rule.name)
                        },
                    },
                ]
            })
            .collect();
        let enabled: Vec<bool> = self.alerting.rules.iter().map(|r| r.enabled).collect();
        let out = DataTable::new("alert-rules", &columns, &rows, &mut self.alerts.rules_table)
            .fits(true)
            .variant(TableVariant::List)
            .stretch(true)
            .footer(false)
            .actions(|at| {
                let on = enabled.get(at).copied().unwrap_or(true);
                vec![
                    MenuItem::action("Edit").icon(Icon::PencilLine),
                    if on {
                        MenuItem::action("Turn off").icon(Icon::BellOff)
                    } else {
                        MenuItem::action("Turn on").icon(Icon::BellRing)
                    },
                    MenuItem::action("Duplicate").icon(Icon::Copy),
                    MenuItem::Separator,
                    MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
                ]
            })
            .show(ui);
        if let Some(at) = out.switched {
            self.toggle_rule(at);
        }
        let opened = self.alerts.rules_table.opened.take();
        match out.action {
            Some((at, 0)) => self.edit_rule(at),
            Some((at, 1)) => self.toggle_rule(at),
            Some((at, 2)) => {
                if let Some(rule) = self.alerting.rules.get(at).cloned() {
                    let id = self.alerting.next_id("rule");
                    self.alerting.rules.push(AlertRule {
                        id,
                        name: format!("{} copy", rule.name),
                        ..rule.clone()
                    });
                    self.said.push(Toast::success(format!("{} duplicated", rule.name)));
                }
            }
            Some((at, 4)) => {
                if let Some(rule) = self.alerting.rules.get(at) {
                    self.alerts.asking = Some(AlertsAsking::DeleteRule(rule.id.clone()));
                }
            }
            _ => {
                if let Some(at) = opened {
                    self.edit_rule(at);
                }
            }
        }
    }

    fn toggle_rule(&mut self, at: usize) {
        if let Some(rule) = self.alerting.rules.get_mut(at) {
            rule.enabled = !rule.enabled;
        }
        self.evaluate_rules();
    }

    fn edit_rule(&mut self, at: usize) {
        if let Some(rule) = self.alerting.rules.get(at) {
            self.alerts.rule = Some(RuleForm {
                editing: Some(rule.id.clone()),
                draft: rule.clone(),
            });
        }
    }

    fn fired_list(&mut self, ui: &mut Ui, now: f64) {
        let offset = self.offset();
        // the web's 7rem, `minmax(10rem,1fr)`, 7rem, 7rem, 12rem and 7rem, less the 12 between columns
        let columns = [
            Column::new("State", 100.0),
            Column::new("Rule", 148.0).fill(),
            Column::new("Severity", 100.0),
            Column::new("Fired at", 100.0).numeric(),
            Column::new("When", 180.0),
            Column::new("Lasted", 100.0).numeric(),
        ];
        let filter = self.alerts.filter;
        let rows: Vec<Vec<Cell>> = self
            .alerting
            .events
            .iter()
            .filter(|e| match filter {
                FiredFilter::All => true,
                FiredFilter::Firing => e.resolved_at.is_none(),
                FiredFilter::Resolved => e.resolved_at.is_some(),
            })
            .map(|e| {
                let took = lasted(e.fired_at, e.resolved_at, now);
                vec![
                    state_chip(if e.resolved_at.is_none() { "firing" } else { "resolved" }),
                    Cell::Strong(e.name.clone()),
                    state_chip(e.severity.word()),
                    Cell::Number {
                        value: e.value,
                        text: written(e.metric, Some(e.value)),
                    },
                    Cell::Clock {
                        clock: clock_of(e.fired_at + offset),
                        at: e.fired_at * 1000.0,
                        now: now * 1000.0,
                    },
                    Cell::Number {
                        value: e.resolved_at.unwrap_or(now) - e.fired_at,
                        text: if e.resolved_at.is_none() {
                            format!("{took}, still")
                        } else {
                            took
                        },
                    },
                ]
            })
            .collect();
        DataTable::new("alerts-fired", &columns, &rows, &mut self.alerts.fired_table)
            .fits(true)
            .variant(TableVariant::List)
            .stretch(true)
            .footer(false)
            .show(ui);
    }

    /// Reads every rule at the newest scrape, saying what fires and resolves as the preferences ask.
    pub(crate) fn evaluate_rules(&mut self) {
        let said = self
            .alerting
            .evaluate(self.at, self.prefs.evaluate, self.prefs.notify, self.prefs.keep);
        self.said.extend(said);
    }

    /// The Alerts page's dialogs: a rule made or changed, and the questions before deleting.
    pub(crate) fn alerts_dialogs(&mut self, ctx: &egui::Context) {
        if let Some(mut form) = self.alerts.rule.take() {
            let offset = self.offset();
            let close = rule_dialog(ctx, &mut form, self.at, offset);
            match close {
                Some(true) => {
                    let name = form.draft.name.trim().to_owned();
                    let mut saved = AlertRule {
                        name: name.clone(),
                        ..form.draft.clone()
                    };
                    match &form.editing {
                        Some(id) => {
                            if let Some(rule) = self.alerting.rules.iter_mut().find(|r| &r.id == id) {
                                saved.id = id.clone();
                                *rule = saved;
                            }
                            self.said.push(Toast::success(format!("{name} saved")));
                        }
                        None => {
                            saved.id = self.alerting.next_id("rule");
                            self.alerting.rules.push(saved);
                            self.said.push(Toast::success(format!("{name} added")));
                            self.place = crate::app::ALERTS.into();
                            self.editor = None;
                        }
                    }
                    self.evaluate_rules();
                }
                Some(false) => {}
                None => self.alerts.rule = Some(form),
            }
        }
        match self.alerts.asking.clone() {
            Some(AlertsAsking::DeleteRule(id)) => {
                let Some(rule) = self.alerting.rules.iter().find(|r| r.id == id).cloned() else {
                    self.alerts.asking = None;
                    return;
                };
                let dialog = ConfirmDialog::new("delete-rule", format!("Delete {}?", rule.name), "Delete")
                    .detail("What it fired stays in the list until it is cleared.")
                    .destructive(true);
                match dialog.show(ctx) {
                    Some(ConfirmAnswer::Confirm) => {
                        self.alerts.asking = None;
                        self.alerting.rules.retain(|r| r.id != id);
                        self.evaluate_rules();
                        self.said.push(Toast::success(format!("{} deleted", rule.name)));
                    }
                    Some(_) => self.alerts.asking = None,
                    None => {}
                }
            }
            Some(AlertsAsking::ClearResolved) => {
                let resolved = self.alerting.events.iter().filter(|e| e.resolved_at.is_some()).count();
                let dialog = ConfirmDialog::new("clear-resolved", "Clear the resolved alerts?", "Clear")
                    .detail(format!(
                        "{resolved} resolved {} the list. Those still firing stay.",
                        if resolved == 1 { "alert leaves" } else { "alerts leave" }
                    ))
                    .destructive(true);
                match dialog.show(ctx) {
                    Some(ConfirmAnswer::Confirm) => {
                        self.alerts.asking = None;
                        self.alerting.events.retain(|e| e.resolved_at.is_none());
                        self.said.push(Toast::success("Resolved alerts cleared"));
                    }
                    Some(_) => self.alerts.asking = None,
                    None => {}
                }
            }
            None => {}
        }
    }
}

/// Two controls side by side, each half the row, 8 apart, as the web's `Row` of two `flex-1` fields:
/// `side` draws the left one, then the right (`true`).
fn halves(ui: &mut Ui, mut side: impl FnMut(&mut Ui, bool)) {
    let half = (ui.available_width() - 8.0) / 2.0;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.allocate_ui(egui::vec2(half, 0.0), |ui| {
            ui.set_width(half);
            side(ui, false);
        });
        ui.allocate_ui(egui::vec2(half, 0.0), |ui| {
            ui.set_width(half);
            side(ui, true);
        });
    });
}

/// The rule dialog, as the web's `RuleDialog`: the rule's fields and a live chart of what it reads
/// against its threshold. True to save, false to close, `None` while it stays.
fn rule_dialog(ctx: &egui::Context, form: &mut RuleForm, at: f64, offset: f64) -> Option<bool> {
    let title = match &form.editing {
        None => "New alert rule".to_owned(),
        Some(_) => format!("Edit {}", form.draft.name),
    };
    let new = form.editing.is_none();
    let missing = form.draft.name.trim().is_empty();
    let mut answer = None;
    let draft = &mut form.draft;
    let close = Dialog::new("rule-dialog", title)
        .description(
            "A rule fires when its value crosses the threshold for as long as it says, and resolves once it is back.",
        )
        .width(672.0)
        .show(
            ctx,
            |ui| {
                Stack::new(Gap::Md).show(ui, |ui| {
                    ui.add(TextField::new("Name", &mut draft.name));
                    let metric = metric_of(draft.metric);
                    halves(ui, |ui, right| {
                        if right {
                            Field::new("Across series").show(ui, |ui| {
                                let mut at = metric
                                    .aggregations
                                    .iter()
                                    .position(|a| *a == draft.aggregation)
                                    .unwrap_or(0);
                                let labels = metric.aggregations.iter().map(|a| aggregation_label(*a));
                                if ui.add(Segmented::new(&mut at, labels).fill(true)).changed() {
                                    draft.aggregation = metric.aggregations[at];
                                }
                            });
                        } else {
                            Field::new("Metric").show(ui, |ui| {
                                let mut at = METRICS.iter().position(|m| m.id == draft.metric).unwrap_or(0);
                                let width = ui.available_width();
                                if ui
                                    .add(Select::new(&mut at, METRICS.iter().map(|m| m.label), "Metric").width(width))
                                    .changed()
                                {
                                    let next = &METRICS[at];
                                    draft.metric = next.id;
                                    if !next.aggregations.contains(&draft.aggregation) {
                                        draft.aggregation =
                                            next.aggregations.first().copied().unwrap_or(Aggregation::Avg);
                                    }
                                }
                            });
                        }
                    });
                    let service = filter_value(draft, Label::Service).unwrap_or(ALL).to_owned();
                    halves(ui, |ui, right| {
                        if right {
                            Field::new("Host").show(ui, |ui| {
                                let hosts: Vec<&str> = HOSTS
                                    .iter()
                                    .filter(|h| service == ALL || h.service == service)
                                    .map(|h| h.id)
                                    .collect();
                                let options: Vec<&str> =
                                    std::iter::once("Any host").chain(hosts.iter().copied()).collect();
                                let host = filter_value(draft, Label::Host).map(str::to_owned);
                                let mut at = host
                                    .and_then(|h| hosts.iter().position(|one| *one == h))
                                    .map_or(0, |i| i + 1);
                                let width = ui.available_width();
                                if ui
                                    .add(Select::new(&mut at, options.iter().copied(), "Host").width(width))
                                    .changed()
                                {
                                    set_filter(draft, Label::Host, (at > 0).then(|| options[at]));
                                }
                            });
                        } else {
                            Field::new("Service").show(ui, |ui| {
                                let options: Vec<&str> = std::iter::once("Any service").chain(SERVICES).collect();
                                let mut at = SERVICES.iter().position(|s| *s == service).map_or(0, |i| i + 1);
                                let width = ui.available_width();
                                if ui
                                    .add(Select::new(&mut at, options.iter().copied(), "Service").width(width))
                                    .changed()
                                {
                                    set_filter(draft, Label::Service, (at > 0).then(|| options[at]));
                                    set_filter(draft, Label::Host, None);
                                }
                            });
                        }
                    });
                    halves(ui, |ui, right| {
                        if right {
                            let width = ui.available_width();
                            ui.add(
                                NumberField::new("rule-threshold", "Threshold", &mut draft.threshold)
                                    .suffix(metric.unit.trim())
                                    .width(width),
                            );
                        } else {
                            Field::new("Fires when the value is").show(ui, |ui| {
                                let mut at = usize::from(draft.condition == Condition::Below);
                                if ui.add(Segmented::new(&mut at, ["Above", "Below"]).fill(true)).changed() {
                                    draft.condition = if at == 0 { Condition::Above } else { Condition::Below };
                                }
                            });
                        }
                    });
                    Field::new("For")
                        .description("How long the value must stay over before the rule fires.")
                        .show(ui, |ui| {
                            let mut at = HOLDS.iter().position(|(s, _)| *s == draft.for_seconds).unwrap_or(2);
                            if ui
                                .add(Segmented::new(&mut at, HOLDS.iter().map(|(_, l)| *l)).fill(true))
                                .changed()
                            {
                                draft.for_seconds = HOLDS[at].0;
                            }
                        });
                    halves(ui, |ui, right| {
                        if right {
                            ui.add_space(22.0);
                            ui.add(SwitchField::new(&mut draft.enabled, "Evaluate this rule"));
                        } else {
                            Field::new("Severity").show(ui, |ui| {
                                let mut at = usize::from(draft.severity == Severity::Critical);
                                if ui
                                    .add(Segmented::new(&mut at, ["Warning", "Critical"]).fill(true))
                                    .changed()
                                {
                                    draft.severity = if at == 0 { Severity::Warning } else { Severity::Critical };
                                }
                            });
                        }
                    });
                    rule_preview(ui, draft, at, offset);
                });
            },
            |ui| {
                let mut save = Button::new(if new { "Add rule" } else { "Save" });
                if missing {
                    save = save.reason("A rule needs a name.");
                }
                if ui.add(save).clicked() && !missing {
                    answer = Some(true);
                }
                if ui.add(Button::new("Cancel").variant(ButtonVariant::Ghost)).clicked() {
                    answer = Some(false);
                }
            },
        );
    answer.or(close.then_some(false))
}

/// The last quarter hour of what a rule reads with its threshold across it, and where the value
/// stands now, as the web's `RulePreview`.
fn rule_preview(ui: &mut Ui, rule: &AlertRule, at: f64, offset: f64) {
    let t = sokkel_egui::tokens(ui.ctx());
    let span = span_of(&Range::Preset("15m"), at, 90.0);
    let query = Query {
        metric: rule.metric,
        aggregation: rule.aggregation,
        group_by: None,
        filters: vec![rule.filter.clone()],
    };
    let series = read_series(&query, &span);
    let metric = metric_of(rule.metric);
    let tone = match rule.severity {
        Severity::Critical => "critical",
        Severity::Warning => "warning",
    };
    let x: Vec<String> = span
        .times
        .iter()
        .map(|time| time_label(*time + offset, &span))
        .collect();
    let mut chart = TimeSeriesPanel::new("The last 15 minutes", x)
        .subtitle(metric.label)
        .unit(metric.unit)
        .decimals(metric.decimals)
        .fill(ChartFill::Gradient)
        .legend(false)
        .height(160.0)
        .threshold(
            rule.threshold,
            t.color(match rule.condition {
                Condition::Above => tone,
                Condition::Below => "good",
            }),
        );
    let now = series.first().and_then(|s| s.values.last().copied());
    for one in series {
        chart = chart.series(Series::new(one.name, one.values).color("chart-2"));
    }
    chart.show(ui);
    let over = crosses(rule, now);
    let state = match (over, rule.for_seconds == 0.0) {
        (true, true) => "firing",
        (true, false) => "pending",
        (false, _) => "normal",
    };
    let said = if !over {
        "within the threshold.".to_owned()
    } else if rule.for_seconds == 0.0 {
        "so the rule would fire.".to_owned()
    } else {
        format!("so it fires if it stays over for {}s.", rule.for_seconds)
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let Cell::Chip(word, chip_tone) = state_chip(state) else {
            return;
        };
        ui.add(Chip::new(word).tone(chip_tone));
        ui.label(
            sokkel_egui::TextRole::Body
                .rich(&t, format!("Now {}, {said}", written(rule.metric, now)))
                .color(t.color("muted-foreground")),
        );
    });
}
