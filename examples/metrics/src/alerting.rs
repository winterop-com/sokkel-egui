//! Alert rules on thresholds and what they fired, as the web's `alerting.ts` and the rules and
//! events of its `data.ts`: each rule is read every second from the newest scrape, fires once it
//! has crossed its threshold for long enough, and resolves once the value is back.

use std::collections::HashMap;

use sokkel_egui::{Toast, ToastKind};

use crate::query::{Query, read_now};
use crate::source::{Aggregation, Label, MetricId, metric_of};

/// Which way a rule's value crosses its threshold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Condition {
    Above,
    Below,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Severity {
    Warning,
    Critical,
}

impl Severity {
    pub fn word(self) -> &'static str {
        match self {
            Severity::Warning => "warning",
            Severity::Critical => "critical",
        }
    }
}

/// An alert rule: a metric's value against a threshold, held for a while before it fires.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub metric: MetricId,
    pub aggregation: Aggregation,
    pub filter: Vec<(Label, String)>,
    pub condition: Condition,
    pub threshold: f64,
    /// How long the value must cross the threshold before the rule fires, in seconds.
    pub for_seconds: f64,
    pub severity: Severity,
    pub enabled: bool,
}

/// A rule that fired: when, at what value, and when it resolved, if it has.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AlertEvent {
    pub id: String,
    pub rule: String,
    pub name: String,
    pub severity: Severity,
    pub metric: MetricId,
    /// The value when it fired.
    pub value: f64,
    pub fired_at: f64,
    pub resolved_at: Option<f64>,
}

/// Where a rule stands now: firing, crossing its threshold but not yet for long enough, normal, or
/// turned off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleState {
    Firing,
    Pending,
    Normal,
    Paused,
}

impl RuleState {
    pub fn word(self) -> &'static str {
        match self {
            RuleState::Firing => "firing",
            RuleState::Pending => "pending",
            RuleState::Normal => "normal",
            RuleState::Paused => "paused",
        }
    }
}

/// A rule's reading at the newest second.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuleReading {
    pub state: RuleState,
    pub value: Option<f64>,
}

/// A rule of `metric` combined by `aggregation`, of the service named, as the seeds fill one.
fn rule(id: &str, name: &str, metric: MetricId, aggregation: Aggregation, service: Option<&str>) -> AlertRule {
    AlertRule {
        id: id.into(),
        name: name.into(),
        metric,
        aggregation,
        filter: service
            .map(|s| vec![(Label::Service, s.to_owned())])
            .unwrap_or_default(),
        condition: Condition::Above,
        threshold: 0.0,
        for_seconds: 0.0,
        severity: Severity::Warning,
        enabled: true,
    }
}

/// The demo's rules, as the web's `seedRules`.
pub fn seed_rules() -> Vec<AlertRule> {
    use Aggregation::{Avg, Max, Sum};
    vec![
        AlertRule {
            threshold: 220.0,
            for_seconds: 15.0,
            severity: Severity::Critical,
            ..rule(
                "rule-latency",
                "api latency high",
                MetricId::LatencyP95,
                Avg,
                Some("api"),
            )
        },
        AlertRule {
            threshold: 2.0,
            for_seconds: 10.0,
            severity: Severity::Critical,
            ..rule("rule-errors", "api errors above 2%", MetricId::Errors, Avg, Some("api"))
        },
        AlertRule {
            threshold: 110.0,
            for_seconds: 30.0,
            ..rule("rule-queue", "Queue backing up", MetricId::QueueDepth, Sum, None)
        },
        AlertRule {
            threshold: 90.0,
            for_seconds: 20.0,
            ..rule("rule-worker-cpu", "Worker CPU hot", MetricId::Cpu, Max, Some("worker"))
        },
        AlertRule {
            condition: Condition::Below,
            threshold: 120.0,
            for_seconds: 60.0,
            enabled: false,
            ..rule(
                "rule-requests-low",
                "Requests dropped",
                MetricId::Requests,
                Sum,
                Some("api"),
            )
        },
    ]
}

/// A rule as a query of one value.
fn query_of(rule: &AlertRule) -> Query {
    Query {
        metric: rule.metric,
        aggregation: rule.aggregation,
        group_by: None,
        filters: vec![rule.filter.clone()],
    }
}

/// A rule's value at a second.
pub fn value_of(rule: &AlertRule, second: f64) -> Option<f64> {
    read_now(&query_of(rule), second)
}

/// Whether a value crosses a rule's threshold.
pub fn crosses(rule: &AlertRule, value: Option<f64>) -> bool {
    match (value, rule.condition) {
        (Some(v), Condition::Above) => v > rule.threshold,
        (Some(v), Condition::Below) => v < rule.threshold,
        (None, _) => false,
    }
}

/// Where a rule would have fired and resolved between two seconds, read every `step` seconds, as
/// the web's `firings`; one still open at the end is left to the live evaluator.
pub fn firings(rule: &AlertRule, from: f64, to: f64, step: f64) -> Vec<AlertEvent> {
    let mut found: Vec<AlertEvent> = Vec::new();
    let mut since: Option<f64> = None;
    let mut open: Option<usize> = None;
    let mut second = from;
    while second <= to {
        let value = value_of(rule, second);
        if crosses(rule, value) {
            let start = *since.get_or_insert(second);
            if open.is_none() && second - start >= rule.for_seconds {
                found.push(AlertEvent {
                    id: String::new(),
                    rule: rule.id.clone(),
                    name: rule.name.clone(),
                    severity: rule.severity,
                    metric: rule.metric,
                    value: value.unwrap_or(0.0),
                    fired_at: second,
                    resolved_at: None,
                });
                open = Some(found.len() - 1);
            }
        } else {
            since = None;
            if let Some(at) = open.take() {
                found[at].resolved_at = Some(second);
            }
        }
        second += step;
    }
    found.retain(|event| event.resolved_at.is_some());
    found
}

/// What the rules fired over the six hours before `now`, newest first, as the web's `seedEvents`.
pub fn seed_events(rules: &[AlertRule], now: f64) -> Vec<AlertEvent> {
    let mut events: Vec<AlertEvent> = rules
        .iter()
        .filter(|rule| rule.enabled)
        .flat_map(|rule| firings(rule, now - 6.0 * 3600.0, now - 60.0, 5.0))
        .collect();
    events.sort_by(|a, b| b.fired_at.total_cmp(&a.fired_at));
    for (index, event) in events.iter_mut().enumerate() {
        event.id = format!("event-seed-{}", index + 1);
    }
    events
}

/// A number written as `toLocaleString('en-US')` writes it with at most `decimals` places: thousands
/// parted by commas, no trailing zeros.
fn locale_number(value: f64, decimals: usize) -> String {
    let fixed = format!("{:.*}", decimals, value.abs());
    let (whole, fraction) = fixed.split_once('.').unwrap_or((&fixed, ""));
    let fraction = fraction.trim_end_matches('0');
    let mut grouped = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let sign = if value < 0.0 && fixed.chars().any(|c| c.is_ascii_digit() && c != '0') {
        "-"
    } else {
        ""
    };
    if fraction.is_empty() {
        format!("{sign}{grouped}")
    } else {
        format!("{sign}{grouped}.{fraction}")
    }
}

/// A value written with its metric's unit, or "No data".
pub fn written(metric: MetricId, value: Option<f64>) -> String {
    let metric = metric_of(metric);
    match value {
        Some(v) => format!("{}{}", locale_number(v, metric.decimals), metric.unit),
        None => "No data".into(),
    }
}

fn aggregation_key(aggregation: Aggregation) -> &'static str {
    match aggregation {
        Aggregation::Sum => "sum",
        Aggregation::Avg => "avg",
        Aggregation::Max => "max",
        Aggregation::Min => "min",
    }
}

/// A rule's condition in words: `Latency p95 (avg) of api above 220 ms for 15s`.
pub fn condition_of(rule: &AlertRule) -> String {
    let metric = metric_of(rule.metric);
    let scope: Vec<&str> = rule
        .filter
        .iter()
        .map(|(_, v)| v.as_str())
        .filter(|v| !v.is_empty() && *v != "all")
        .collect();
    let of = if scope.is_empty() {
        String::new()
    } else {
        format!(" of {}", scope.join(", "))
    };
    let held = if rule.for_seconds == 0.0 {
        String::new()
    } else if rule.for_seconds < 60.0 {
        format!(" for {}s", rule.for_seconds)
    } else {
        format!(" for {}m", rule.for_seconds / 60.0)
    };
    let way = match rule.condition {
        Condition::Above => "above",
        Condition::Below => "below",
    };
    format!(
        "{} ({}){of} {way} {}{held}",
        metric.label,
        aggregation_key(rule.aggregation),
        written(rule.metric, Some(rule.threshold))
    )
}

/// The rules, what they fired, and where each stands, evaluated every second.
#[derive(Clone, Debug)]
pub struct Alerting {
    pub rules: Vec<AlertRule>,
    pub events: Vec<AlertEvent>,
    pub readings: HashMap<String, RuleReading>,
    /// Since when each rule has crossed its threshold.
    since: HashMap<String, f64>,
    made: u32,
}

impl Alerting {
    /// The demo's rules and what they fired before `now`, evaluated at `now`.
    pub fn seeded(now: f64) -> Self {
        let rules = seed_rules();
        let events = seed_events(&rules, now);
        let mut alerting = Self {
            rules,
            events,
            readings: HashMap::new(),
            since: HashMap::new(),
            made: 0,
        };
        alerting.evaluate(now, true, false, 24.0 * 3600.0);
        alerting
    }

    /// Rules and events kept from an earlier run, ids counting on from `made`, evaluated at `now`
    /// without a word: what was firing when the app closed is still open.
    pub fn restored(
        rules: Vec<AlertRule>,
        events: Vec<AlertEvent>,
        made: u32,
        now: f64,
        evaluate: bool,
        keep: f64,
    ) -> Self {
        let mut alerting = Self {
            rules,
            events,
            readings: HashMap::new(),
            since: HashMap::new(),
            made,
        };
        alerting.evaluate(now, evaluate, false, keep);
        alerting
    }

    /// How many ids have been made, to count on from in the next run.
    pub fn made(&self) -> u32 {
        self.made
    }

    /// A fresh id for a rule or an event.
    pub fn next_id(&mut self, kind: &str) -> String {
        self.made += 1;
        format!("{kind}-{}", self.made)
    }

    /// How many rules fire now.
    pub fn firing(&self) -> usize {
        self.readings.values().filter(|r| r.state == RuleState::Firing).count()
    }

    /// Reads every rule at `second`, as the web's `useAlerting`: a rule crossing its threshold for
    /// its `for_seconds` fires, an event kept and, when `notify`, a toast said; once back, its event
    /// resolves. With `evaluate` off nothing fires and what fires resolves. Resolved alerts older
    /// than `keep` seconds are let go. The toasts to say come back.
    pub fn evaluate(&mut self, second: f64, evaluate: bool, notify: bool, keep: f64) -> Vec<Toast> {
        let mut said = Vec::new();
        let mut readings = HashMap::new();
        let rules = self.rules.clone();
        for rule in &rules {
            let value = value_of(rule, second);
            let open = self
                .events
                .iter()
                .position(|e| e.rule == rule.id && e.resolved_at.is_none());
            if !rule.enabled || !evaluate {
                self.since.remove(&rule.id);
                if let Some(at) = open {
                    self.events[at].resolved_at = Some(second);
                }
                readings.insert(
                    rule.id.clone(),
                    RuleReading {
                        state: RuleState::Paused,
                        value,
                    },
                );
                continue;
            }
            if crosses(rule, value) {
                let from = *self.since.entry(rule.id.clone()).or_insert(second);
                let due = second - from >= rule.for_seconds;
                if due && open.is_none() {
                    let id = self.next_id("event");
                    self.events.insert(
                        0,
                        AlertEvent {
                            id,
                            rule: rule.id.clone(),
                            name: rule.name.clone(),
                            severity: rule.severity,
                            metric: rule.metric,
                            value: value.unwrap_or(0.0),
                            fired_at: second,
                            resolved_at: None,
                        },
                    );
                    if notify {
                        let kind = match rule.severity {
                            Severity::Critical => ToastKind::Error,
                            Severity::Warning => ToastKind::Warning,
                        };
                        said.push(
                            Toast::new(format!("{} is firing", rule.name))
                                .kind(kind)
                                .description(format!("{}: {}", written(rule.metric, value), condition_of(rule))),
                        );
                    }
                }
                let state = if due || open.is_some() {
                    RuleState::Firing
                } else {
                    RuleState::Pending
                };
                readings.insert(rule.id.clone(), RuleReading { state, value });
            } else {
                self.since.remove(&rule.id);
                if let Some(at) = open {
                    self.events[at].resolved_at = Some(second);
                    if notify {
                        said.push(
                            Toast::success(format!("{} resolved", rule.name)).description(written(rule.metric, value)),
                        );
                    }
                }
                readings.insert(
                    rule.id.clone(),
                    RuleReading {
                        state: RuleState::Normal,
                        value,
                    },
                );
            }
        }
        // a rule removed while firing leaves its event open; it resolves with the rule
        for event in &mut self.events {
            if event.resolved_at.is_none() && !rules.iter().any(|r| r.id == event.rule) {
                event.resolved_at = Some(second);
            }
        }
        self.events
            .retain(|e| e.resolved_at.is_none_or(|resolved| resolved >= second - keep));
        self.readings = readings;
        said
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::DEMO_NOW;

    #[test]
    fn the_rules_read_as_the_web_s() {
        let rules = seed_rules();
        assert_eq!(condition_of(&rules[0]), "Latency p95 (avg) of api above 220 ms for 15s");
        assert_eq!(condition_of(&rules[4]), "Requests (sum) of api below 120 req/s for 1m");
        assert_eq!(locale_number(1234.5, 1), "1,234.5");
        assert_eq!(locale_number(0.4212, 2), "0.42");
        // the web's Alerts page counts what fired in the six hours before 09:00:05
        assert_eq!(seed_events(&rules, DEMO_NOW).len(), 69);
    }
}
