//! Where the app is and what it shows, as the web's `view.ts`: the page, the range, the comparison
//! and the segment, and the span a range covers, cut into the steps its charts draw.

use crate::days::{DAY, HOUR, civil, day_from, day_of, short_day, start_of};
use crate::model::{Plan, Region};

pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
}

pub const PRESETS: [Preset; 7] = [
    Preset {
        id: "today",
        label: "Today",
    },
    Preset {
        id: "7d",
        label: "Last 7 days",
    },
    Preset {
        id: "30d",
        label: "Last 30 days",
    },
    Preset {
        id: "90d",
        label: "Last 90 days",
    },
    Preset {
        id: "mtd",
        label: "Month to date",
    },
    Preset {
        id: "ytd",
        label: "Year to date",
    },
    Preset {
        id: "12m",
        label: "Last 12 months",
    },
];

/// A range: a preset's id, or a span of days.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Range {
    Preset(&'static str),
    Days { from: i64, to: i64 },
}

impl Range {
    /// The preset `id` names, or the last 30 days when it names none.
    pub fn preset(id: &str) -> Self {
        Range::Preset(PRESETS.iter().find(|p| p.id == id).map_or("30d", |p| p.id))
    }
}

/// The page shown and what it shows.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub path: Vec<String>,
    pub range: Range,
    pub compare: bool,
    pub plan: Option<Plan>,
    pub region: Option<Region>,
}

/// One step along a chart's time axis: from and to in milliseconds, and its label.
#[derive(Clone, Debug, PartialEq)]
pub struct Bucket {
    pub from: i64,
    pub to: i64,
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Hour,
    Day,
    Week,
    Month,
}

/// A stretch of time the figures are read over, cut into the steps its charts draw.
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub from: i64,
    pub to: i64,
    /// Its first and last day.
    pub first: i64,
    pub last: i64,
    /// How many days it covers.
    pub days: i64,
    pub buckets: Vec<Bucket>,
    pub step: Step,
}

fn cut(from: i64, to: i64, first: i64, days: i64) -> (Vec<Bucket>, Step) {
    if days <= 1 {
        let buckets = (0..24)
            .map(|hour| {
                let start = from + hour * HOUR;
                Bucket {
                    from: start,
                    to: start + HOUR,
                    label: format!("{hour:02}:00"),
                }
            })
            .collect();
        return (buckets, Step::Hour);
    }
    let width = if days <= 92 {
        1
    } else if days <= 400 {
        7
    } else {
        30
    };
    let mut buckets = Vec::new();
    let mut day = first;
    while start_of(day) < to {
        buckets.push(Bucket {
            from: start_of(day),
            to: start_of(day + width).min(to),
            label: short_day(day),
        });
        day += width;
    }
    let step = match width {
        1 => Step::Day,
        7 => Step::Week,
        _ => Step::Month,
    };
    (buckets, step)
}

/// The span a range covers at `now`; a range that reaches today ends at the moment.
pub fn span_of(range: &Range, now: i64) -> Span {
    let today = day_of(now);
    let (first, last) = match range {
        Range::Days { from, to } => (*from, (*to).min(today)),
        Range::Preset(id) => {
            let (year, month, _) = civil(today);
            let first = match *id {
                "today" => today,
                "7d" => today - 6,
                "90d" => today - 89,
                "12m" => today - 364,
                "mtd" => day_from(year, month, 1),
                "ytd" => day_from(year, 1, 1),
                _ => today - 29,
            };
            (first, today)
        }
    };
    let days = last - first + 1;
    let from = start_of(first);
    let to = if last >= today { now } else { start_of(last + 1) };
    let (buckets, step) = cut(from, start_of(last + 1), first, days);
    Span {
        from,
        to,
        first,
        last,
        days,
        buckets,
        step,
    }
}

/// The span just before, as long, ending at the same moment of its day, for comparing with.
pub fn previous_of(span: &Span) -> Span {
    let shift = span.days * DAY;
    let first = span.first - span.days;
    Span {
        from: start_of(first),
        to: span.to - shift,
        first,
        last: span.last - span.days,
        days: span.days,
        buckets: span
            .buckets
            .iter()
            .map(|bucket| Bucket {
                from: bucket.from - shift,
                to: bucket.to - shift,
                label: bucket.label.clone(),
            })
            .collect(),
        step: span.step,
    }
}

/// The range said for a reader: `Last 30 days`, `Oct 3 to Oct 17`.
pub fn range_name(range: &Range) -> String {
    match range {
        Range::Preset(id) => PRESETS
            .iter()
            .find(|p| p.id == *id)
            .map_or_else(|| (*id).to_owned(), |p| p.label.to_owned()),
        Range::Days { from, to } => format!("{} to {}", short_day(*from), short_day(*to)),
    }
}
