//! sokkel-ui's charts data module (`@winterop-com/sokkel-ui-charts/data`) as far as the
//! visualizer leans on it, carried here until the charts add-on has it: a chart's rows of named
//! fields, a field read as a number or a label, the quantile and the ways figures are combined, and
//! the pivot that groups rows into categories and series.

use std::collections::{HashMap, HashSet};

use serde::{Serialize, Serializer};

use crate::js;

/// A value a chart's row or a table's cell holds: text, a number, or none.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Cell {
    Text(String),
    Number(f64),
    #[default]
    Null,
}

impl Cell {
    pub fn text(value: impl Into<String>) -> Self {
        Cell::Text(value.into())
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Cell::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Cell::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Cell::Null)
    }

    /// The cell as `String(value)` writes it, none as empty: what a CSV holds for it.
    pub fn written(&self) -> String {
        match self {
            Cell::Text(text) => text.clone(),
            Cell::Number(value) => js::number_to_string(*value),
            Cell::Null => String::new(),
        }
    }
}

impl From<Option<f64>> for Cell {
    fn from(value: Option<f64>) -> Self {
        value.map_or(Cell::Null, Cell::Number)
    }
}

impl Serialize for Cell {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Cell::Text(text) => serializer.serialize_str(text),
            Cell::Number(value) if value.is_finite() => serializer.serialize_f64(*value),
            _ => serializer.serialize_none(),
        }
    }
}

/// One row of the data a chart draws: named fields in the order they were set, as an object keeps
/// its keys; setting a field it has changes it in its place.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row(pub Vec<(String, Cell)>);

impl Row {
    pub fn get(&self, field: &str) -> Option<&Cell> {
        self.0.iter().find(|(name, _)| name == field).map(|(_, value)| value)
    }

    pub fn set(&mut self, field: &str, value: Cell) {
        match self.0.iter_mut().find(|(name, _)| name == field) {
            Some(entry) => entry.1 = value,
            None => self.0.push((field.to_owned(), value)),
        }
    }
}

impl Serialize for Row {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

/// How the values that fall in one place are combined into one figure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Combine {
    Sum,
    Count,
    Avg,
    Min,
    Max,
    Median,
    P90,
}

impl From<crate::model::Aggregate> for Combine {
    fn from(how: crate::model::Aggregate) -> Self {
        use crate::model::Aggregate;
        match how {
            Aggregate::Sum => Combine::Sum,
            Aggregate::Avg => Combine::Avg,
            Aggregate::Count => Combine::Count,
            Aggregate::Min => Combine::Min,
            Aggregate::Max => Combine::Max,
        }
    }
}

/// A field's value as a number, or none where it is missing or not a number.
pub fn number_of(value: Option<&Cell>) -> Option<f64> {
    match value? {
        Cell::Number(value) => value.is_finite().then_some(*value),
        Cell::Text(text) if !js::trim(text).is_empty() => {
            let parsed = js::number_of_text(text);
            parsed.is_finite().then_some(parsed)
        }
        _ => None,
    }
}

/// A field's value as a label: what a category, a series or a slice is called.
pub fn label_of(value: Option<&Cell>) -> String {
    match value {
        Some(Cell::Text(text)) => text.clone(),
        Some(Cell::Number(value)) => js::number_to_string(*value),
        _ => String::new(),
    }
}

/// The least and greatest of some numbers, or none when there are none.
pub fn extent(values: &[f64]) -> Option<(f64, f64)> {
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for &value in values {
        if value < low {
            low = value;
        }
        if value > high {
            high = value;
        }
    }
    (low <= high).then_some((low, high))
}

/// The value at a fraction of the way through sorted numbers, interpolated between neighbors.
pub fn quantile(sorted: &[f64], at: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let index = (sorted.len() - 1) as f64 * at;
    let low = index.floor() as usize;
    let high = index.ceil() as usize;
    Some(sorted[low] + (sorted[high] - sorted[low]) * (index - low as f64))
}

/// The values combined into one figure; `Count` counts them, the rest need at least one.
pub fn aggregate(values: &[f64], how: Combine) -> Option<f64> {
    if how == Combine::Count {
        return Some(values.len() as f64);
    }
    if values.is_empty() {
        return None;
    }
    match how {
        Combine::Sum => Some(values.iter().fold(0.0, |sum, value| sum + value)),
        Combine::Avg => Some(values.iter().fold(0.0, |sum, value| sum + value) / values.len() as f64),
        Combine::Min => extent(values).map(|(low, _)| low),
        Combine::Max => extent(values).map(|(_, high)| high),
        _ => {
            let mut sorted = values.to_vec();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            quantile(&sorted, if how == Combine::Median { 0.5 } else { 0.9 })
        }
    }
}

/// The distinct labels of a field, in the order the rows first give them, or in `order` where it
/// names them.
pub fn distinct(rows: &[Row], field: &str, order: Option<&[String]>) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for row in rows {
        let label = label_of(row.get(field));
        if seen.insert(label.clone()) {
            found.push(label);
        }
    }
    let Some(order) = order else { return found };
    let mut out: Vec<String> = order.iter().filter(|name| seen.contains(*name)).cloned().collect();
    out.extend(found.into_iter().filter(|name| !order.contains(name)));
    out
}

/// One named run of figures, one per category.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Series {
    pub name: String,
    pub values: Vec<Option<f64>>,
}

/// What a chart of categories draws: the categories along one axis and a series of figures for
/// each.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Pivot {
    pub categories: Vec<String>,
    pub series: Vec<Series>,
}

/// How rows become categories and series, for the one measure the visualizer draws.
pub struct PivotSpec<'a> {
    pub rows: &'a [Row],
    /// The field whose labels are the categories.
    pub x: &'a str,
    /// The field whose numbers are drawn; without one the rows are counted.
    pub y: Option<&'a str>,
    /// The field whose labels split the rows into series.
    pub series: Option<&'a str>,
    pub aggregate: Combine,
    /// Categories in this order first, then any others as the rows give them.
    pub order: Option<&'a [String]>,
}

/// The rows grouped by category, and by series where `series` names a field, each group's `y`
/// combined by `aggregate`; without a `y` the rows are counted, in one series called `Count`.
pub fn pivot(spec: &PivotSpec) -> Pivot {
    let categories = distinct(spec.rows, spec.x, spec.order);
    // A category the order names more than once has a place for each: its nth row goes to its
    // nth place, and any past the last join that one.
    let mut places: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, name) in categories.iter().enumerate() {
        places.entry(name.as_str()).or_default().push(index);
    }
    let collect = |pick: &dyn Fn(&Row) -> bool, field: Option<&str>, how: Combine| -> Vec<Option<f64>> {
        let mut buckets: Vec<Vec<f64>> = vec![Vec::new(); categories.len()];
        let mut seen: HashMap<String, usize> = HashMap::new();
        for row in spec.rows {
            if !pick(row) {
                continue;
            }
            let name = label_of(row.get(spec.x));
            let Some(at) = places.get(name.as_str()) else {
                continue;
            };
            let times = seen.entry(name).or_insert(0);
            let index = at[(*times).min(at.len() - 1)];
            *times += 1;
            match field {
                None => buckets[index].push(1.0),
                Some(field) => {
                    if let Some(value) = number_of(row.get(field)) {
                        buckets[index].push(value);
                    }
                }
            }
        }
        buckets
            .iter()
            .map(|bucket| aggregate(bucket, if field.is_none() { Combine::Count } else { how }))
            .collect()
    };
    if let Some(series) = spec.series {
        return Pivot {
            series: distinct(spec.rows, series, None)
                .into_iter()
                .map(|name| {
                    let values = collect(&|row: &Row| label_of(row.get(series)) == name, spec.y, spec.aggregate);
                    Series { name, values }
                })
                .collect(),
            categories,
        };
    }
    match spec.y {
        None => Pivot {
            series: vec![Series {
                name: "Count".to_owned(),
                values: collect(&|_| true, None, Combine::Count),
            }],
            categories,
        },
        Some(field) => Pivot {
            series: vec![Series {
                name: field.to_owned(),
                values: collect(&|_| true, Some(field), spec.aggregate),
            }],
            categories,
        },
    }
}
