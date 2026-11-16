//! Reading the source over a range, as the web's `query.ts`: the ranges a dashboard offers, the
//! steps a range is read at, a query's series grouped by a label, and a series read as one value.

use crate::source::{
    ALL, Aggregation, HOSTS, JOBS, Label, MetricId, PROJECTS, SERVICES, SeriesLabels, combine, matches, metric_of,
    series_of, value,
};

/// A range a dashboard offers, by how far back it reaches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    pub seconds: f64,
}

pub const PRESETS: [Preset; 5] = [
    Preset {
        id: "5m",
        label: "Last 5 minutes",
        seconds: 300.0,
    },
    Preset {
        id: "15m",
        label: "Last 15 minutes",
        seconds: 900.0,
    },
    Preset {
        id: "1h",
        label: "Last hour",
        seconds: 3600.0,
    },
    Preset {
        id: "6h",
        label: "Last 6 hours",
        seconds: 6.0 * 3600.0,
    },
    Preset {
        id: "24h",
        label: "Last 24 hours",
        seconds: 24.0 * 3600.0,
    },
];

/// The intervals a dashboard refreshes by itself at, in seconds; 0 is never.
pub const INTERVALS: [u32; 4] = [0, 5, 10, 30];

/// A dashboard's range: a preset reaching back from now, or the seconds from one moment to another.
#[derive(Clone, Debug, PartialEq)]
pub enum Range {
    Preset(&'static str),
    Between { from: f64, to: f64 },
}

impl Default for Range {
    fn default() -> Self {
        Range::Preset("15m")
    }
}

/// The seconds a dashboard covers and the steps it reads them at.
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub start: f64,
    pub end: f64,
    pub step: f64,
    pub times: Vec<f64>,
}

const NICE: [f64; 18] = [
    1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 900.0, 1800.0, 3600.0, 7200.0, 10_800.0, 21_600.0,
    43_200.0, 86_400.0,
];

/// The seconds a range covers at a moment: a preset reaches back from it, a custom range runs from
/// its start to its end, never past now.
pub fn bounds(range: &Range, now: f64) -> (f64, f64) {
    match range {
        Range::Preset(id) => {
            let preset = PRESETS.iter().find(|p| p.id == *id).unwrap_or(&PRESETS[1]);
            (now - preset.seconds, now)
        }
        Range::Between { from, to } => {
            let start = from.floor();
            (start, (start + 60.0).max(now.min(to.floor())))
        }
    }
}

fn steps(start: f64, end: f64, points: f64) -> (f64, Vec<f64>) {
    let raw = (end - start) / points;
    let step = NICE.iter().copied().find(|nice| *nice >= raw).unwrap_or(86_400.0);
    let mut times = Vec::new();
    let mut time = (start / step).ceil() * step;
    while time <= end {
        times.push(time);
        time += step;
    }
    (step, times)
}

/// The span of a range at a moment, read in about `points` steps of a round length, each on a
/// multiple of it so a moving range keeps its steps.
pub fn span_of(range: &Range, now: f64, points: f64) -> Span {
    let (start, end) = bounds(range, now);
    let (step, times) = steps(start, end, points);
    Span {
        start,
        end,
        step,
        times,
    }
}

/// The same span read in fewer, wider steps, for bars and heatmap cells.
pub fn coarse(span: &Span, points: f64) -> Span {
    let (step, times) = steps(span.start, span.end, points);
    Span {
        step,
        times,
        ..span.clone()
    }
}

/// A second written for an axis, as the web's in UTC: with seconds for a short range, the day for
/// one past two days.
pub fn time_label(second: f64, span: &Span) -> String {
    let at = second.rem_euclid(86_400.0) as u64;
    let (h, m, s) = (at / 3600, at / 60 % 60, at % 60);
    if span.end - span.start > 2.0 * 86_400.0 {
        let (month, day) = month_day(second);
        format!("{month} {day}, {h:02}:{m:02}")
    } else if span.end - span.start <= 600.0 {
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        format!("{h:02}:{m:02}")
    }
}

/// A moment written for a reader, with its seconds.
pub fn clock_of(second: f64) -> String {
    let at = second.rem_euclid(86_400.0) as u64;
    format!("{:02}:{:02}:{:02}", at / 3600, at / 60 % 60, at % 60)
}

/// The short month and the day of a second since 1970, in UTC.
fn month_day(second: f64) -> (&'static str, u32) {
    let days = (second / 86_400.0).floor() as i64;
    // civil from days, after Howard Hinnant
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as usize;
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    (MONTHS[month - 1], day)
}

/// What a group of series is split by, or none for one series.
pub type GroupBy = Option<Label>;

/// What a panel or a rule asks the source for.
#[derive(Clone, Debug, PartialEq)]
pub struct Query {
    pub metric: MetricId,
    pub aggregation: Aggregation,
    pub group_by: GroupBy,
    /// Filters a series must pass, each: the panel's own, the dashboard's variables.
    pub filters: Vec<Vec<(Label, String)>>,
}

/// One series of a query: its name and its value at each step.
#[derive(Clone, Debug, PartialEq)]
pub struct SeriesData {
    pub name: String,
    pub values: Vec<f64>,
}

fn order(label: Label) -> Vec<&'static str> {
    match label {
        Label::Service => SERVICES.to_vec(),
        Label::Host => HOSTS.iter().map(|h| h.id).collect(),
        Label::Project => PROJECTS.to_vec(),
        Label::Job => JOBS.to_vec(),
    }
}

/// A series' value over one step, the mean of three readings across it so a wide step is not one
/// second's flicker.
fn sample(metric: MetricId, labels: &SeriesLabels, time: f64, step: f64) -> f64 {
    if step <= 10.0 {
        return value(metric, labels, time);
    }
    let third = step / 3.0;
    (value(metric, labels, time) + value(metric, labels, time - third) + value(metric, labels, time - 2.0 * third))
        / 3.0
}

/// The groups of series a query reads, by name, in the order the labels are known in.
pub fn groups_of(query: &Query) -> Vec<(String, Vec<SeriesLabels>)> {
    let kept: Vec<SeriesLabels> = series_of(query.metric)
        .into_iter()
        .filter(|s| query.filters.iter().all(|f| matches(s, f)))
        .collect();
    let Some(label) = query.group_by else {
        if kept.is_empty() {
            return Vec::new();
        }
        return vec![(metric_of(query.metric).label.to_owned(), kept)];
    };
    let mut names: Vec<&str> = Vec::new();
    for s in &kept {
        let name = s.get(label).unwrap_or("-");
        if !names.contains(&name) {
            names.push(name);
        }
    }
    let known = order(label);
    let rank = |name: &str| known.iter().position(|k| *k == name).map_or(-1, |i| i as i64);
    names.sort_by_key(|name| rank(name));
    names
        .into_iter()
        .map(|name| {
            let members = kept
                .iter()
                .filter(|s| s.get(label).unwrap_or("-") == name)
                .copied()
                .collect();
            (name.to_owned(), members)
        })
        .collect()
}

/// A query's series over a span: one per group, each step combining the group's series.
pub fn read_series(query: &Query, span: &Span) -> Vec<SeriesData> {
    groups_of(query)
        .into_iter()
        .map(|(name, members)| SeriesData {
            name,
            values: span
                .times
                .iter()
                .map(|time| {
                    let values: Vec<f64> = members
                        .iter()
                        .map(|m| sample(query.metric, m, *time, span.step))
                        .collect();
                    combine(&values, query.aggregation).unwrap_or(0.0)
                })
                .collect(),
        })
        .collect()
}

/// A query's one value at a second, as an alert rule reads it.
pub fn read_now(query: &Query, second: f64) -> Option<f64> {
    let values: Vec<f64> = series_of(query.metric)
        .iter()
        .filter(|s| query.filters.iter().all(|f| matches(s, f)))
        .map(|s| value(query.metric, s, second))
        .collect();
    combine(&values, query.aggregation)
}

/// How a series over the range is read as one value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reduce {
    Last,
    Mean,
    Max,
    Min,
}

pub const REDUCES: [(Reduce, &str); 4] = [
    (Reduce::Last, "Last"),
    (Reduce::Mean, "Mean"),
    (Reduce::Max, "Max"),
    (Reduce::Min, "Min"),
];

/// A series read as one value.
pub fn reduce(values: &[f64], how: Reduce) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(match how {
        Reduce::Last => values[values.len() - 1],
        Reduce::Max => values.iter().copied().fold(f64::MIN, f64::max),
        Reduce::Min => values.iter().copied().fold(f64::MAX, f64::min),
        Reduce::Mean => values.iter().sum::<f64>() / values.len() as f64,
    })
}

/// The change from one value to another, written for a reader: `+4.2%`.
pub fn change(now: Option<f64>, before: Option<f64>) -> Option<String> {
    let (now, before) = (now?, before?);
    if before == 0.0 {
        return None;
    }
    let share = (now - before) / before.abs() * 100.0;
    if share.abs() < 0.05 {
        return Some("0.0%".into());
    }
    Some(format!("{}{:.1}%", if share > 0.0 { "+" } else { "" }, share))
}

/// The span just before another, as long as it, for a change against the range before.
pub fn before(span: &Span) -> Span {
    let length = span.end - span.start;
    Span {
        start: span.start - length,
        end: span.end - length,
        step: span.step,
        times: span.times.iter().map(|t| t - length).collect(),
    }
}

/// A dashboard's variables as a filter.
pub fn variables_filter(variables: &crate::source::Variables) -> Vec<(Label, String)> {
    vec![
        (Label::Service, variables.service.clone()),
        (Label::Host, variables.host.clone()),
        (Label::Project, variables.project.clone()),
    ]
    .into_iter()
    .filter(|(_, v)| v != ALL)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quarter_hour_reads_ninety_steps_of_ten_seconds() {
        let span = span_of(&Range::Preset("15m"), DEMO_NOW, 150.0);
        assert_eq!(span.step, 10.0);
        assert_eq!(span.times.len(), 90);
        assert_eq!(time_label(DEMO_NOW - 5.0, &span), "09:00");
        assert_eq!(coarse(&span, 24.0).step, 60.0);
    }

    #[test]
    fn requests_by_service_are_the_web_s() {
        let query = Query {
            metric: MetricId::Requests,
            aggregation: Aggregation::Sum,
            group_by: Some(Label::Service),
            filters: Vec::new(),
        };
        let span = span_of(&Range::Preset("15m"), DEMO_NOW, 150.0);
        let series = read_series(&query, &span);
        let names: Vec<&str> = series.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["api", "worker", "scheduler"]);
        assert!(
            (series[0].values.last().unwrap() - WEB_API_LAST).abs() < 1e-9,
            "{:?}",
            series[0].values.last()
        );
    }

    /// `readSeries` of requests summed by service over the last 15 minutes at 09:00:05 on the
    /// demo's day, its api series' last step, as bun runs the web's `query.ts`.
    const WEB_API_LAST: f64 = 333.9608450423496;

    /// 09:00:05 UTC on the demo's day, 2026-11-18, as the web's shots are taken.
    const DEMO_NOW: f64 = 1_794_992_405.0;
}
