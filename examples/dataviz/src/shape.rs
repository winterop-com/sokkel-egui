//! How a chart's figures are read from its dataset, as the web's `shape.ts` reads them: each value
//! read by its column's type, the board's filters applied, dates binned by day, week or month,
//! categories combined, sorted and cut to the limit, and the same figures as a table, a map's
//! regions and one reading with its course and its change on the step before. With the filter
//! fields a dataset offers, a board's filters moved onto one dataset's columns, the view a new
//! chart starts from, the views suggested for the columns picked, and a map's breaks.

use std::cmp::Ordering;
use std::collections::HashSet;

use serde::Serialize;
use sokkel_egui::{Filter, FilterField, FilterType, FilterValue, filter_rows};

use crate::chart_data::{self, Cell, Combine, PivotSpec, Row, aggregate, pivot, quantile};
use crate::js;
use crate::model::{
    Aggregate, Column, ColumnType, Dataset, DateBin, Decimals, NumberStyle, Preferences, SortOrder, ViewKind, ViewSpec,
};
use crate::parse::{day_of, number_of};

/// A cell's text read as its column's type: a number, a day as `YYYY-MM-DD`, text, or none.
pub fn typed(text: Option<&str>, column: &Column) -> Cell {
    let Some(text) = text else { return Cell::Null };
    if js::trim(text).is_empty() {
        return Cell::Null;
    }
    match column.kind {
        ColumnType::Number => number_of(text).into(),
        ColumnType::Date => day_of(text).map_or(Cell::Null, Cell::Text),
        _ => Cell::Text(text.to_owned()),
    }
}

/// A row's values read by type, keyed by column id.
#[derive(Clone, Debug, PartialEq)]
pub struct TypedRow {
    pub id: String,
    pub cells: Vec<(String, Cell)>,
}

impl TypedRow {
    /// A column's value: none where it has none.
    pub fn get(&self, column: &str) -> &Cell {
        self.cells
            .iter()
            .find(|(key, _)| key == column)
            .map_or(&Cell::Null, |(_, value)| value)
    }
}

impl Serialize for TypedRow {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.cells.len() + 1))?;
        map.serialize_entry("id", &self.id)?;
        for (key, value) in &self.cells {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

/// Every row of a dataset with its values read by type.
pub fn typed_rows(dataset: &Dataset) -> Vec<TypedRow> {
    dataset
        .rows
        .iter()
        .map(|row| TypedRow {
            id: row.id.clone(),
            cells: dataset
                .columns
                .iter()
                .map(|column| (column.id.clone(), typed(row.get(&column.id), column)))
                .collect(),
        })
        .collect()
}

/// A column by its id, or none where the dataset has none.
pub fn column_of<'a>(dataset: &'a Dataset, id: Option<&str>) -> Option<&'a Column> {
    dataset.column(id)
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The key a day falls under in a bin: the day, the Monday starting its week, or its month.
pub fn bin_key(day: &str, bin: DateBin) -> String {
    match bin {
        DateBin::Month => day.chars().take(7).collect(),
        DateBin::Day => day.to_owned(),
        DateBin::Week => {
            let days = js::day_number(day).unwrap_or(0);
            js::iso_day(days - (js::weekday(days) + 6) % 7)
        }
    }
}

/// How a bin is written along an axis: `Mar`, or `Mar 3` for a day or the week starting on it,
/// with the year where the data spans several.
pub fn bin_label(key: &str, bin: DateBin, with_year: bool) -> String {
    let parts: Vec<&str> = key.split('-').collect();
    let year = parts.first().copied().unwrap_or("undefined");
    let month = parts.get(1).copied();
    let name = month
        .map(js::number_of_text)
        .filter(|number| number.fract() == 0.0 && (1.0..=12.0).contains(number))
        .map_or_else(
            || month.unwrap_or("undefined").to_owned(),
            |number| MONTHS[number as usize - 1].to_owned(),
        );
    if bin == DateBin::Month {
        return if with_year { format!("{name} {year}") } else { name };
    }
    let day = js::number_to_string(parts.get(2).map_or(f64::NAN, |day| js::number_of_text(day)));
    if with_year {
        format!("{name} {day}, {year}")
    } else {
        format!("{name} {day}")
    }
}

/// The figure written for a reader as the settings say: grouped, compact or plain, with their
/// decimals.
pub fn format_figure(numbers: NumberStyle, decimals: Decimals, value: f64) -> String {
    let fixed = match decimals {
        Decimals::Auto => None,
        Decimals::Fixed(places) => Some(usize::from(places)),
    };
    let compact = numbers == NumberStyle::Compact;
    let max = fixed.unwrap_or(if compact { 1 } else { 2 });
    let min = fixed.unwrap_or(0);
    js::intl_number(value, min, max, numbers != NumberStyle::Plain, compact)
}

/// The writer of figures the settings make, as the web's `formatter`.
pub fn formatter(prefs: &Preferences) -> impl Fn(f64) -> String + use<> {
    let (numbers, decimals) = (prefs.numbers, prefs.decimals);
    move |value| format_figure(numbers, decimals, value)
}

/// How the measure is named: `Sum of Rainfall`, or `Rows` where rows are counted.
pub fn measure_name(dataset: &Dataset, spec: &ViewSpec) -> String {
    let y = column_of(dataset, spec.y.as_deref());
    if spec.view == ViewKind::Scatter {
        return y.map_or_else(|| "Rows".to_owned(), |y| y.name.clone());
    }
    match y {
        Some(y) if spec.aggregate != Aggregate::Count => format!("{} of {}", spec.aggregate.label(), y.name),
        _ => "Rows".to_owned(),
    }
}

/// What each view needs to be drawn, said when it is missing.
fn problem_of(dataset: &Dataset, spec: &ViewSpec) -> Option<&'static str> {
    let x = column_of(dataset, spec.x.as_deref());
    let y = column_of(dataset, spec.y.as_deref());
    let kind = |column: Option<&Column>| column.map(|one| one.kind);
    match spec.view {
        ViewKind::Scatter => (kind(x) != Some(ColumnType::Number) || kind(y) != Some(ColumnType::Number))
            .then_some("A scatter plots one number against another: pick a number column for each axis."),
        ViewKind::Map => (kind(x) != Some(ColumnType::Region))
            .then_some("A map colors regions: pick a column of region names, typed Region, for Region."),
        ViewKind::Stat => None,
        ViewKind::Heatmap => (x.is_none() || column_of(dataset, spec.series.as_deref()).is_none())
            .then_some("A heatmap needs a column along it and one down it."),
        _ => x.is_none().then_some("Pick the column the chart runs along."),
    }
}

/// The figures as a table: its column names and its rows, for the table view and a CSV.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Table {
    pub names: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
}

/// One reading of the measure over every row, with its course over the dates.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Stat {
    /// The reading: a number, or none; text where the measure's column holds text, as the web's
    /// arithmetic gives it there.
    pub value: Cell,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sparkline: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<String>,
}

/// What a view draws, read from its dataset: the chart's rows and fields, the same figures as a
/// table, and a single reading.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Shaped {
    /// The rows for a chart, keyed by column names, dates binned.
    pub rows: Vec<Row>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The categories in the order picked, after the limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
    pub aggregate: Aggregate,
    pub table: Table,
    /// Each region's figure, for the map.
    pub regions: Vec<(String, Option<f64>)>,
    pub stat: Stat,
    /// How many rows were read, after the filters.
    pub read: usize,
    /// Why the view cannot be drawn from these columns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
    pub empty: bool,
}

fn blank(problem: Option<&str>) -> Shaped {
    Shaped {
        rows: Vec::new(),
        x: None,
        y: None,
        series: None,
        value: None,
        order: None,
        aggregate: Aggregate::Sum,
        table: Table::default(),
        regions: Vec::new(),
        stat: Stat::default(),
        read: 0,
        problem: problem.map(str::to_owned),
        empty: true,
    }
}

/// A category's own order: dates and numbers ascending, names alphabetically.
pub fn naturally(a: &str, b: &str) -> Ordering {
    js::locale_compare(a, b, true)
}

/// A comparison as a sort reads a comparator's number: below zero, above it, or alike (NaN too).
fn sign_of(diff: f64) -> Ordering {
    if diff < 0.0 {
        Ordering::Less
    } else if diff > 0.0 {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

/// The figures a view draws from its dataset, after the filters, with dates binned, categories
/// sorted and cut to the limit.
pub fn shape(dataset: &Dataset, spec: &ViewSpec, filters: &[Filter]) -> Shaped {
    if let Some(problem) = problem_of(dataset, spec) {
        return blank(Some(problem));
    }
    let all = typed_rows(dataset);
    let rows: Vec<&TypedRow> = if filters.is_empty() {
        all.iter().collect()
    } else {
        let fields = filter_fields_of(&filter_fields(dataset, &all));
        filter_rows(&all, filters, &fields)
    };
    let x = column_of(dataset, spec.x.as_deref());
    let y = if spec.aggregate == Aggregate::Count {
        None
    } else {
        column_of(dataset, spec.y.as_deref())
    };
    let how = if y.is_none() { Aggregate::Count } else { spec.aggregate };
    let of_y = |row: &TypedRow| -> Cell { y.map_or(Cell::Null, |y| row.get(&y.id).clone()) };

    if spec.view == ViewKind::Scatter {
        let (Some(xx), Some(yy)) = (x, column_of(dataset, spec.y.as_deref())) else {
            return blank(None);
        };
        let color = column_of(dataset, spec.color.as_deref());
        let size = column_of(dataset, spec.size.as_deref());
        let points: Vec<&&TypedRow> = rows
            .iter()
            .filter(|row| !row.get(&xx.id).is_null() && !row.get(&yy.id).is_null())
            .collect();
        let chart_rows: Vec<Row> = points
            .iter()
            .map(|row| {
                let mut out = Row::default();
                out.set(&xx.name, row.get(&xx.id).clone());
                out.set(&yy.name, row.get(&yy.id).clone());
                if let Some(color) = color {
                    let value = row.get(&color.id);
                    out.set(
                        &color.name,
                        if value.is_null() { Cell::text("") } else { value.clone() },
                    );
                }
                if let Some(size) = size {
                    out.set(&size.name, row.get(&size.id).clone());
                }
                out
            })
            .collect();
        let mut names = vec![xx.name.clone(), yy.name.clone()];
        names.extend(color.map(|one| one.name.clone()));
        names.extend(size.map(|one| one.name.clone()));
        let table_rows = chart_rows
            .iter()
            .map(|row| {
                names
                    .iter()
                    .map(|name| row.get(name).cloned().unwrap_or(Cell::Null))
                    .collect()
            })
            .collect();
        let empty = chart_rows.is_empty();
        return Shaped {
            rows: chart_rows,
            x: Some(xx.name.clone()),
            y: Some(yy.name.clone()),
            series: color.map(|one| one.name.clone()),
            value: size.map(|one| one.name.clone()),
            aggregate: Aggregate::Sum,
            table: Table {
                names,
                rows: table_rows,
            },
            read: rows.len(),
            empty,
            ..blank(None)
        };
    }

    // A reading of the measure over every row, and its course over the dates where a date column
    // leads.
    let values: Vec<Cell> = rows
        .iter()
        .map(|row| of_y(row))
        .filter(|value| !value.is_null())
        .collect();
    let reading = if how == Aggregate::Count {
        Cell::Number(rows.len() as f64)
    } else {
        reading_of(&values, how)
    };
    let mut sparkline: Option<Vec<f64>> = None;
    let mut delta: Option<String> = None;
    if let Some(date) = x.filter(|x| x.kind == ColumnType::Date) {
        let mut buckets: Vec<(String, Vec<f64>)> = Vec::new();
        for row in &rows {
            let Some(day) = row.get(&date.id).as_text() else {
                continue;
            };
            let key = bin_key(day, spec.bin);
            let value = if how == Aggregate::Count {
                1.0
            } else {
                of_y(row).as_number().unwrap_or(f64::NAN)
            };
            match buckets.iter_mut().find(|(one, _)| *one == key) {
                Some((_, bucket)) => bucket.push(value),
                None => buckets.push((key, vec![value])),
            }
        }
        buckets.sort_by(|a, b| a.0.cmp(&b.0));
        let combine: Combine = if how == Aggregate::Count {
            Combine::Sum
        } else {
            how.into()
        };
        let line: Vec<f64> = buckets
            .iter()
            .map(|(_, bucket)| {
                let finite: Vec<f64> = bucket.iter().copied().filter(|value| value.is_finite()).collect();
                aggregate(&finite, combine).unwrap_or(0.0)
            })
            .collect();
        if line.len() > 1 {
            let (before, last) = (line[line.len() - 2], line[line.len() - 1]);
            if before != 0.0 {
                let change = ((last - before) / before.abs()) * 100.0;
                let places = if change != 0.0 && change.abs() < 10.0 { 1 } else { 0 };
                delta = Some(format!(
                    "{}{}% on the {} before",
                    if change >= 0.0 { '+' } else { '−' },
                    js::to_fixed(change.abs(), places),
                    spec.bin.id()
                ));
            }
        }
        sparkline = Some(line);
    }
    if spec.view == ViewKind::Stat {
        let name = measure_name(dataset, spec);
        return Shaped {
            table: Table {
                names: vec![name],
                rows: vec![vec![reading.clone()]],
            },
            stat: Stat {
                value: reading,
                sparkline,
                delta,
            },
            read: rows.len(),
            empty: rows.is_empty(),
            ..blank(None)
        };
    }

    let Some(xx) = x else { return blank(None) };
    let series = if matches!(spec.view, ViewKind::Pie | ViewKind::Map) {
        None
    } else {
        column_of(dataset, spec.series.as_deref())
    };
    let years: HashSet<String> = rows
        .iter()
        .map(|row| match row.get(&xx.id) {
            Cell::Text(day) if xx.kind == ColumnType::Date => day.chars().take(4).collect(),
            _ => String::new(),
        })
        .collect();
    let with_year = years.len() > 1;
    let mut key_of: Vec<(String, String)> = Vec::new();
    let mut chart_rows: Vec<Row> = Vec::new();
    for row in &rows {
        let raw = row.get(&xx.id);
        if raw.is_null() {
            continue;
        }
        let label = if xx.kind == ColumnType::Date {
            let key = bin_key(raw.as_text().unwrap_or(""), spec.bin);
            let label = bin_label(&key, spec.bin, with_year);
            match key_of.iter_mut().find(|(one, _)| *one == label) {
                Some(entry) => entry.1 = key,
                None => key_of.push((label.clone(), key)),
            }
            label
        } else {
            raw.written()
        };
        let mut out = Row::default();
        out.set(&xx.name, Cell::Text(label));
        if let Some(y) = y {
            out.set(&y.name, of_y(row));
        }
        if let Some(series) = series {
            let value = row.get(&series.id);
            out.set(
                &series.name,
                Cell::Text(if value.is_null() {
                    "(none)".to_owned()
                } else {
                    value.written()
                }),
            );
        }
        chart_rows.push(out);
    }
    let combine: Combine = how.into();
    let totals = pivot(&PivotSpec {
        rows: &chart_rows,
        x: &xx.name,
        y: y.map(|y| y.name.as_str()),
        series: None,
        aggregate: combine,
        order: None,
    });
    let figure = |name: &str| -> Option<f64> {
        let at = totals.categories.iter().position(|one| one == name)?;
        totals.series.first().and_then(|series| series.values[at])
    };
    let sort_key = |label: &str| -> String {
        key_of
            .iter()
            .find(|(one, _)| one == label)
            .map_or_else(|| label.to_owned(), |(_, key)| key.clone())
    };
    let mut sorted = totals.categories.clone();
    sorted.sort_by(|a, b| {
        if spec.sort == SortOrder::Natural {
            return naturally(&sort_key(a), &sort_key(b));
        }
        let diff = figure(a).unwrap_or(f64::NEG_INFINITY) - figure(b).unwrap_or(f64::NEG_INFINITY);
        sign_of(if spec.sort == SortOrder::Desc { -diff } else { diff })
    });
    let kept: Vec<String> = if spec.limit > 0.0 {
        sorted.iter().take(spec.limit.trunc() as usize).cloned().collect()
    } else {
        sorted.clone()
    };
    let shown: Vec<Row> = if kept.len() == sorted.len() {
        chart_rows
    } else {
        chart_rows
            .into_iter()
            .filter(|row| match row.get(&xx.name) {
                Some(Cell::Text(label)) => kept.contains(label),
                _ => false,
            })
            .collect()
    };
    let split = pivot(&PivotSpec {
        rows: &shown,
        x: &xx.name,
        y: y.map(|y| y.name.as_str()),
        series: series.map(|series| series.name.as_str()),
        aggregate: combine,
        order: Some(&kept),
    });
    let measure = measure_name(dataset, spec);
    let mut names = vec![xx.name.clone()];
    if series.is_none() {
        names.push(measure);
    } else {
        names.extend(split.series.iter().map(|entry| entry.name.clone()));
    }
    let table_rows = split
        .categories
        .iter()
        .enumerate()
        .map(|(at, name)| {
            let mut cells = vec![Cell::Text(name.clone())];
            cells.extend(split.series.iter().map(|entry| entry.values[at].into()));
            cells
        })
        .collect();
    let regions = kept.iter().map(|name| (name.clone(), figure(name))).collect();
    let empty = shown.is_empty();
    Shaped {
        rows: shown,
        x: Some(xx.name.clone()),
        y: y.map(|y| y.name.clone()),
        series: series.map(|series| series.name.clone()),
        value: None,
        order: Some(kept),
        aggregate: how,
        table: Table {
            names,
            rows: table_rows,
        },
        regions,
        stat: Stat {
            value: reading,
            sparkline,
            delta,
        },
        read: rows.len(),
        problem: None,
        empty,
    }
}

/// A value as JavaScript's arithmetic reads it: text as the number it writes, NaN where none.
fn to_number(value: &Cell) -> f64 {
    match value {
        Cell::Number(value) => *value,
        Cell::Text(text) => js::number_of_text(text),
        Cell::Null => 0.0,
    }
}

/// Whether `a < b` as JavaScript compares them: texts by their UTF-16 units, the rest as numbers.
fn js_less(a: &Cell, b: &Cell) -> bool {
    match (a, b) {
        (Cell::Text(a), Cell::Text(b)) => a.encode_utf16().lt(b.encode_utf16()),
        _ => to_number(a) < to_number(b),
    }
}

/// The measure's values combined as the web's `aggregate` combines what it is given: numbers as
/// numbers, and texts (a measure whose column holds text) as JavaScript's `+`, `/` and `<` treat
/// them, joined, divided as the number they write, or compared as text.
fn reading_of(values: &[Cell], how: Aggregate) -> Cell {
    if values.iter().all(|value| matches!(value, Cell::Number(_))) {
        let numbers: Vec<f64> = values.iter().filter_map(Cell::as_number).collect();
        return aggregate(&numbers, how.into()).into();
    }
    if values.is_empty() {
        return Cell::Null;
    }
    let sum = || {
        values.iter().fold(Cell::Number(0.0), |sum, value| match (&sum, value) {
            (Cell::Number(a), Cell::Number(b)) => Cell::Number(a + b),
            _ => Cell::Text(format!("{}{}", sum.written(), value.written())),
        })
    };
    match how {
        Aggregate::Count => Cell::Number(values.len() as f64),
        Aggregate::Sum => sum(),
        Aggregate::Avg => Cell::Number(to_number(&sum()) / values.len() as f64),
        Aggregate::Min | Aggregate::Max => {
            let mut low = Cell::Number(f64::INFINITY);
            let mut high = Cell::Number(f64::NEG_INFINITY);
            for value in values {
                if js_less(value, &low) {
                    low = value.clone();
                }
                if js_less(&high, value) {
                    high = value.clone();
                }
            }
            if js_less(&high, &low) {
                Cell::Null
            } else if how == Aggregate::Min {
                low
            } else {
                high
            }
        }
    }
}

/// A filter field of a dataset's column: numbers and dates by range, a short list of names as a
/// set to tick (its options in their natural order), long ones as text.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FieldSpec {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
}

impl FieldSpec {
    pub fn filter_type(&self) -> FilterType {
        match self.kind {
            "number" => FilterType::Number,
            "date" => FilterType::Date,
            "set" => FilterType::Set,
            _ => FilterType::Text,
        }
    }
}

/// The filter fields of a dataset's columns: numbers and dates by range, a short list of names as
/// a set to tick, long ones as text.
pub fn filter_fields(dataset: &Dataset, rows: &[TypedRow]) -> Vec<FieldSpec> {
    dataset
        .columns
        .iter()
        .map(|column| {
            let (kind, options) = match column.kind {
                ColumnType::Number => ("number", None),
                ColumnType::Date => ("date", None),
                _ => {
                    let mut names: Vec<String> = Vec::new();
                    let mut seen: HashSet<&str> = HashSet::new();
                    for row in rows {
                        if let Cell::Text(value) = row.get(&column.id)
                            && seen.insert(value)
                        {
                            names.push(value.clone());
                        }
                    }
                    names.sort_by(|a, b| naturally(a, b));
                    if names.len() > 60 {
                        ("text", None)
                    } else {
                        ("set", Some(names))
                    }
                }
            };
            FieldSpec {
                id: column.id.clone(),
                label: column.name.clone(),
                kind,
                options,
            }
        })
        .collect()
}

/// The fields as the core's filter matching reads them, each reading its column's typed value.
pub fn filter_fields_of(specs: &[FieldSpec]) -> Vec<FilterField<'static, TypedRow>> {
    specs
        .iter()
        .map(|spec| {
            let id = spec.id.clone();
            let kind = spec.filter_type();
            let field = FilterField::new(&spec.id, &spec.label, kind, move |row: &TypedRow| {
                match (kind, row.get(&id)) {
                    (FilterType::Number, Cell::Number(value)) => FilterValue::Number(*value),
                    (FilterType::Date, Cell::Text(day)) => FilterValue::Day(day.clone()),
                    (FilterType::Text, Cell::Text(text)) => FilterValue::Text(text.clone()),
                    (FilterType::Text, _) => FilterValue::Text(String::new()),
                    (FilterType::Set, Cell::Text(text)) => FilterValue::Set(vec![text.clone()]),
                    (FilterType::Set, _) => FilterValue::Set(Vec::new()),
                    _ => FilterValue::None,
                }
            });
            match &spec.options {
                Some(options) => field.options(options.iter().map(|value| (value.clone(), value.clone()))),
                None => field,
            }
        })
        .collect()
}

/// A board's filters, which name columns, as filters on one dataset's columns by id; a filter on a
/// column it lacks is left out.
pub fn filters_for(dataset: &Dataset, filters: &[Filter]) -> Vec<Filter> {
    filters
        .iter()
        .filter_map(|filter| {
            let wanted = filter.field.to_lowercase();
            let column = dataset.columns.iter().find(|one| one.name.to_lowercase() == wanted)?;
            Some(Filter {
                field: column.id.clone(),
                ..filter.clone()
            })
        })
        .collect()
}

/// A view's starting point for a dataset: a line over its dates, a map of its regions, bars of its
/// names, or its rows as a table.
pub fn starting_spec(dataset: &Dataset, prefs: &Preferences) -> ViewSpec {
    let of = |kind: ColumnType| dataset.columns.iter().find(|column| column.kind == kind);
    let number = of(ColumnType::Number);
    let base = ViewSpec {
        view: ViewKind::Table,
        aggregate: if number.is_none() {
            Aggregate::Count
        } else {
            prefs.aggregate
        },
        y: number.map(|one| one.id.clone()),
        ..ViewSpec::base()
    };
    if let Some(date) = of(ColumnType::Date) {
        return ViewSpec {
            view: ViewKind::Line,
            x: Some(date.id.clone()),
            ..base
        };
    }
    if let Some(region) = of(ColumnType::Region) {
        return ViewSpec {
            view: ViewKind::Map,
            x: Some(region.id.clone()),
            ..base
        };
    }
    if let Some(text) = of(ColumnType::Text) {
        return ViewSpec {
            view: ViewKind::Bar,
            x: Some(text.id.clone()),
            sort: SortOrder::Desc,
            limit: 10.0,
            ..base
        };
    }
    ViewSpec {
        x: dataset.columns.first().map(|one| one.id.clone()),
        ..base
    }
}

/// A view suggested for the columns picked, with what it shows.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Suggestion {
    pub id: String,
    pub title: String,
    pub reason: String,
    pub spec: ViewSpec,
}

fn capital(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Up to four views that suit the columns picked now, none the view already shown.
pub fn suggest(dataset: &Dataset, spec: &ViewSpec) -> Vec<Suggestion> {
    let x = column_of(dataset, spec.x.as_deref());
    let y = column_of(dataset, spec.y.as_deref());
    let series = column_of(dataset, spec.series.as_deref());
    let of = |kinds: &[ColumnType]| -> Vec<&Column> {
        dataset
            .columns
            .iter()
            .filter(|column| kinds.contains(&column.kind))
            .collect()
    };
    let numbers = of(&[ColumnType::Number]);
    let dates = of(&[ColumnType::Date]);
    let regions = of(&[ColumnType::Region]);
    let texts = of(&[ColumnType::Text, ColumnType::Region]);
    let measure = y.filter(|y| y.kind == ColumnType::Number && spec.aggregate != Aggregate::Count);
    let what = measure.map_or_else(|| "rows".to_owned(), |measure| measure.name.to_lowercase());
    let mut out: Vec<Suggestion> = Vec::new();
    let mut add = |view: ViewKind, title: &str, reason: String, patch: &dyn Fn(&mut ViewSpec)| {
        if view == spec.view || out.iter().any(|one| one.spec.view == view) {
            return;
        }
        let mut next = spec.clone();
        patch(&mut next);
        next.view = view;
        out.push(Suggestion {
            id: view.id().to_owned(),
            title: title.to_owned(),
            reason,
            spec: next,
        });
    };
    let distinct = |column: &Column| {
        dataset
            .rows
            .iter()
            .map(|row| row.get(&column.id))
            .collect::<HashSet<_>>()
            .len()
    };
    let bin = spec.bin.id();
    if x.is_some_and(|x| x.kind == ColumnType::Date) {
        add(
            ViewKind::Line,
            "Line",
            format!("{} over {bin}s", capital(&what)),
            &|_| {},
        );
        let stacked = series.is_some();
        add(
            ViewKind::Area,
            "Area",
            match series {
                None => format!("The {what} filled under its line"),
                Some(series) => format!("{} stacked by {}", capital(&what), series.name.to_lowercase()),
            },
            &|next| next.stack = stacked,
        );
        add(ViewKind::Bar, "Bars", format!("A bar for each {bin}"), &|_| {});
        let other = texts
            .iter()
            .copied()
            .find(|column| Some(&column.id) != series.map(|one| &one.id))
            .or(series);
        if let Some(other) = other {
            let id = other.id.clone();
            add(
                ViewKind::Heatmap,
                "Heatmap",
                format!("{}s against {}", capital(bin), other.name.to_lowercase()),
                &|next| next.series = Some(id.clone()),
            );
        }
    }
    if x.is_some_and(|x| x.kind == ColumnType::Region) {
        add(
            ViewKind::Map,
            "Map",
            format!("{} shaded by region", capital(&what)),
            &|_| {},
        );
    }
    if let Some(x) = x.filter(|x| matches!(x.kind, ColumnType::Text | ColumnType::Region)) {
        add(
            ViewKind::Bar,
            "Bars",
            format!("{} by {}, largest first", capital(&what), x.name.to_lowercase()),
            &|next| next.sort = SortOrder::Desc,
        );
        if distinct(x) <= 8 && series.is_none() {
            add(
                ViewKind::Pie,
                "Pie",
                format!("Each {}’s share", x.name.to_lowercase()),
                &|_| {},
            );
        }
        if let Some(across) = texts.iter().copied().find(|column| column.id != x.id) {
            let id = series.map_or_else(|| across.id.clone(), |series| series.id.clone());
            add(
                ViewKind::Heatmap,
                "Heatmap",
                format!("{} against {}", x.name, across.name.to_lowercase()),
                &|next| next.series = Some(id.clone()),
            );
        }
    }
    if let Some(region) = regions
        .first()
        .filter(|_| x.is_none_or(|x| x.kind != ColumnType::Region))
    {
        let id = region.id.clone();
        add(
            ViewKind::Map,
            "Map",
            format!("{} by {}", capital(&what), region.name.to_lowercase()),
            &|next| next.x = Some(id.clone()),
        );
    }
    if numbers.len() >= 2 {
        let (first, second) = match (x, y) {
            (Some(x), Some(y)) if x.kind == ColumnType::Number && y.kind == ColumnType::Number && x.id != y.id => {
                (x, y)
            }
            _ => (numbers[0], numbers[1]),
        };
        let (first_id, second_id) = (first.id.clone(), second.id.clone());
        let color = texts.first().map(|one| one.id.clone());
        add(
            ViewKind::Scatter,
            "Scatter",
            format!("{} against {}", first.name, second.name.to_lowercase()),
            &|next| {
                next.x = Some(first_id.clone());
                next.y = Some(second_id.clone());
                next.color = color.clone();
            },
        );
    }
    if measure.is_some() || spec.aggregate == Aggregate::Count {
        let along = dates.first().map(|one| one.id.clone()).or_else(|| spec.x.clone());
        add(ViewKind::Stat, "Figure", format!("The {what} as one figure"), &|next| {
            next.x = along.clone()
        });
    }
    add(ViewKind::Table, "Table", "The same figures as rows".to_owned(), &|_| {});
    out.truncate(4);
    out
}

/// A figure rounded to two significant digits, so a scale's breaks read as round numbers: 2,474 is
/// 2,500.
fn roundly(value: f64) -> f64 {
    if value == 0.0 {
        return 0.0;
    }
    let step = 10f64.powi(value.abs().log10().floor() as i32 - 1);
    js::round(value / step) * step
}

/// Quantile breaks for a map's scale: four round steps through the figures, each once.
pub fn breaks_of(values: &[f64]) -> Vec<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let mut out: Vec<f64> = Vec::new();
    for at in [0.2, 0.4, 0.6, 0.8] {
        if let Some(value) = quantile(&sorted, at) {
            let round = roundly(value);
            if !out.iter().any(|one| *one == round || (one.is_nan() && round.is_nan())) {
                out.push(round);
            }
        }
    }
    out
}

/// A cell's value as the chart data reads it: a number, or none.
pub fn number_in(cell: &Cell) -> Option<f64> {
    chart_data::number_of(Some(cell))
}
