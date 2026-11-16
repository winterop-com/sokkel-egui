//! What a panel draws, read from the source over its range, as the web's `panel-data.ts`.

use crate::model::{Panel, PanelKind};
use crate::query::{
    Query, Range, Reduce, SeriesData, before, change, clock_of, coarse, read_series, reduce, span_of, time_label,
    variables_filter,
};
use crate::source::{Label, Rising, Variables, metric_of};

/// One group's values over the range, read as one each way, for a table.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupRow {
    pub name: String,
    pub last: f64,
    pub mean: f64,
    pub max: f64,
    pub min: f64,
}

/// One line of a log panel.
#[derive(Clone, Debug, PartialEq)]
pub struct LogRow {
    pub at: String,
    pub level: &'static str,
    pub source: String,
    pub text: String,
}

/// What a panel draws, by its kind.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// A time series or bars: the time of each step and a series per group, with its color when
    /// it is the only one.
    Series {
        x: Vec<String>,
        series: Vec<(SeriesData, Option<&'static str>)>,
    },
    Stat {
        value: Option<f64>,
        sparkline: Vec<f64>,
        delta: Option<String>,
        good: Option<bool>,
    },
    Gauge {
        value: Option<f64>,
    },
    BarGauge {
        bars: Vec<(String, Option<f64>)>,
    },
    Heatmap {
        x: Vec<String>,
        y: Vec<String>,
        cells: Vec<(usize, usize, f64)>,
    },
    Table {
        rows: Vec<GroupRow>,
    },
    Logs {
        lines: Vec<LogRow>,
    },
}

/// What a panel draws, with the unit and decimals it writes figures in.
#[derive(Clone, Debug, PartialEq)]
pub struct PanelData {
    pub unit: String,
    pub decimals: usize,
    pub empty: bool,
    pub shape: Shape,
}

/// What a panel asks the source for, under the dashboard's variables.
pub fn query_of(panel: &Panel, vars: &Variables) -> Query {
    let one = matches!(panel.kind, PanelKind::Stat | PanelKind::Gauge);
    Query {
        metric: panel.metric,
        aggregation: panel.aggregation,
        group_by: if one { None } else { panel.group_by },
        filters: vec![panel.filter.clone(), variables_filter(vars)],
    }
}

/// The one filter a log panel reads by, its own and the variables', or `None` when they cannot
/// both hold.
pub fn log_filter(panel: &Panel, vars: &Variables) -> Option<Vec<(Label, String)>> {
    let mut filter = variables_filter(vars);
    for (label, value) in &panel.filter {
        if value.is_empty() || value == crate::source::ALL {
            continue;
        }
        match filter.iter_mut().find(|(l, _)| l == label) {
            Some((_, chosen)) if chosen != value => return None,
            Some(_) => {}
            None => filter.push((*label, value.clone())),
        }
    }
    Some(filter)
}

/// Reads what a panel draws over its range, up to the second `at`.
pub fn panel_data(panel: &Panel, vars: &Variables, range: &Range, at: f64) -> PanelData {
    let metric = metric_of(panel.metric);
    let unit = panel.unit.clone().unwrap_or_else(|| metric.unit.to_owned());
    let decimals = panel.decimals.unwrap_or(metric.decimals);
    let query = query_of(panel, vars);
    let span = span_of(range, at, 150.0);
    let colored = |series: Vec<SeriesData>| {
        let alone = series.len() == 1;
        series
            .into_iter()
            .map(|s| (s, alone.then_some(panel.color)))
            .collect::<Vec<_>>()
    };
    let (empty, shape) = match panel.kind {
        PanelKind::TimeSeries => {
            let series = read_series(&query, &span);
            (
                series.is_empty(),
                Shape::Series {
                    x: span.times.iter().map(|t| time_label(*t, &span)).collect(),
                    series: colored(series),
                },
            )
        }
        PanelKind::Bar => {
            let wide = coarse(&span, 20.0);
            let series = read_series(&query, &wide);
            (
                series.is_empty(),
                Shape::Series {
                    x: wide.times.iter().map(|t| time_label(*t, &wide)).collect(),
                    series: colored(series),
                },
            )
        }
        PanelKind::Stat => {
            let series = read_series(&query, &span).into_iter().next();
            let value = series.as_ref().and_then(|s| reduce(&s.values, panel.reduce));
            let earlier = read_series(&query, &before(&span)).into_iter().next();
            let delta = change(value, earlier.and_then(|s| reduce(&s.values, panel.reduce)));
            let good = match (&delta, metric.rising) {
                (Some(d), Some(rising)) if d != "0.0%" => Some(match rising {
                    Rising::Good => d.starts_with('+'),
                    Rising::Bad => d.starts_with('-'),
                }),
                _ => None,
            };
            (
                series.is_none(),
                Shape::Stat {
                    value,
                    sparkline: series.map(|s| s.values).unwrap_or_default(),
                    delta,
                    good,
                },
            )
        }
        PanelKind::Gauge => {
            let series = read_series(&query, &span).into_iter().next();
            (
                series.is_none(),
                Shape::Gauge {
                    value: series.and_then(|s| reduce(&s.values, panel.reduce)),
                },
            )
        }
        PanelKind::BarGauge => {
            let series = read_series(&query, &span);
            (
                series.is_empty(),
                Shape::BarGauge {
                    bars: series
                        .into_iter()
                        .map(|s| {
                            let value = reduce(&s.values, panel.reduce);
                            (s.name, value)
                        })
                        .collect(),
                },
            )
        }
        PanelKind::Heatmap => {
            let wide = coarse(&span, 30.0);
            let series = read_series(&query, &wide);
            let cells = series
                .iter()
                .enumerate()
                .flat_map(|(row, s)| s.values.iter().enumerate().map(move |(column, v)| (column, row, *v)))
                .collect();
            (
                series.is_empty(),
                Shape::Heatmap {
                    x: wide.times.iter().map(|t| time_label(*t, &wide)).collect(),
                    y: series.iter().map(|s| s.name.clone()).collect(),
                    cells,
                },
            )
        }
        PanelKind::Table => {
            let series = read_series(&query, &span);
            let rows: Vec<GroupRow> = series
                .into_iter()
                .map(|s| GroupRow {
                    last: reduce(&s.values, Reduce::Last).unwrap_or(0.0),
                    mean: reduce(&s.values, Reduce::Mean).unwrap_or(0.0),
                    max: reduce(&s.values, Reduce::Max).unwrap_or(0.0),
                    min: reduce(&s.values, Reduce::Min).unwrap_or(0.0),
                    name: s.name,
                })
                .collect();
            (rows.is_empty(), Shape::Table { rows })
        }
        PanelKind::Logs => {
            let lines: Vec<LogRow> = log_filter(panel, vars)
                .map(|filter| {
                    crate::source::logs_until(span.end, span.start, &filter, 200)
                        .into_iter()
                        .map(|line| LogRow {
                            at: clock_of(line.second),
                            level: line.level,
                            source: line.host.to_owned(),
                            text: line.text,
                        })
                        .collect()
                })
                .unwrap_or_default();
            (lines.is_empty(), Shape::Logs { lines })
        }
    };
    PanelData {
        unit,
        decimals,
        empty,
        shape,
    }
}
