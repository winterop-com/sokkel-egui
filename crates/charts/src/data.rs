//! What a chart reads, as sokkel-ui's charts data module (`@winterop-com/sokkel-ui-charts/data`):
//! rows of named fields, a field read as a number or a label, the ways the values that fall in one
//! place are combined into one figure, and the pivot that groups rows into categories and series.
//! A chart of rows ([`crate::Chart`]) reads its rows through it, and an app can shape its own
//! figures with it before it draws them.

use std::collections::{HashMap, HashSet};

use crate::Series;

/// A value a row's field holds: text, a number, or none.
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
            Cell::Number(value) => number_to_string(*value),
            Cell::Null => String::new(),
        }
    }
}

impl From<Option<f64>> for Cell {
    fn from(value: Option<f64>) -> Self {
        value.map_or(Cell::Null, Cell::Number)
    }
}

impl From<f64> for Cell {
    fn from(value: f64) -> Self {
        Cell::Number(value)
    }
}

impl From<&str> for Cell {
    fn from(value: &str) -> Self {
        Cell::Text(value.to_owned())
    }
}

impl From<String> for Cell {
    fn from(value: String) -> Self {
        Cell::Text(value)
    }
}

/// One row of the data a chart draws: named fields in the order they were set, as an object keeps
/// its keys; setting a field it has changes it in its place.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row(pub Vec<(String, Cell)>);

impl Row {
    /// A row of these fields, in this order.
    pub fn of<K: Into<String>, V: Into<Cell>>(fields: impl IntoIterator<Item = (K, V)>) -> Self {
        let mut row = Row::default();
        for (name, value) in fields {
            row.set(&name.into(), value.into());
        }
        row
    }

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

/// How the values that fall in one place are combined into one figure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Aggregate {
    #[default]
    Sum,
    Count,
    Avg,
    Min,
    Max,
    Median,
    P90,
}

/// A field's value as a number, or none where it is missing or not a number; a text is read as
/// `Number(text)` reads it.
pub fn number_of(value: Option<&Cell>) -> Option<f64> {
    match value? {
        Cell::Number(value) => value.is_finite().then_some(*value),
        Cell::Text(text) if !trim(text).is_empty() => {
            let parsed = number_of_text(text);
            parsed.is_finite().then_some(parsed)
        }
        _ => None,
    }
}

/// A field's value as a label: what a category, a series or a slice is called.
pub fn label_of(value: Option<&Cell>) -> String {
    match value {
        Some(Cell::Text(text)) => text.clone(),
        Some(Cell::Number(value)) => number_to_string(*value),
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

pub use crate::panel::quantile;

/// The values combined into one figure; `Count` counts them, the rest need at least one.
pub fn aggregate(values: &[f64], how: Aggregate) -> Option<f64> {
    if how == Aggregate::Count {
        return Some(values.len() as f64);
    }
    if values.is_empty() {
        return None;
    }
    match how {
        Aggregate::Sum => Some(values.iter().fold(0.0, |sum, value| sum + value)),
        Aggregate::Avg => Some(values.iter().fold(0.0, |sum, value| sum + value) / values.len() as f64),
        Aggregate::Min => extent(values).map(|(low, _)| low),
        Aggregate::Max => extent(values).map(|(_, high)| high),
        _ => {
            let mut sorted = values.to_vec();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            quantile(&sorted, if how == Aggregate::Median { 0.5 } else { 0.9 })
        }
    }
}

/// How the `index`th `y` field is combined: its own way where `how` lists one per field, the sum
/// where none is given.
pub fn aggregate_at(how: &[Aggregate], index: usize) -> Aggregate {
    match how {
        [] => Aggregate::Sum,
        [one] => *one,
        many => many.get(index).copied().unwrap_or(Aggregate::Sum),
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

/// What a chart of categories draws: the categories along one axis and a series of figures for
/// each.
#[derive(Clone, Debug)]
pub struct Pivot {
    pub categories: Vec<String>,
    pub series: Vec<Series>,
}

/// How rows become categories and series.
#[derive(Clone, Copy, Debug)]
pub struct PivotSpec<'a> {
    pub rows: &'a [Row],
    /// The field whose labels are the categories.
    pub x: &'a str,
    /// The fields whose numbers are drawn, one series each; without one the rows are counted.
    pub y: &'a [String],
    /// The field whose labels split the rows into series.
    pub series: Option<&'a str>,
    /// How each group's values are combined, or one way for each `y` field in order; the sum when
    /// none is given.
    pub aggregate: &'a [Aggregate],
    /// Categories in this order first, then any others as the rows give them.
    pub order: Option<&'a [String]>,
    /// Series in this order first, then any others.
    pub series_order: Option<&'a [String]>,
    /// What the one series is called where rows are counted; `Count` by default.
    pub count: Option<&'a str>,
}

impl<'a> PivotSpec<'a> {
    /// The rows by `x`, counted, until more is said.
    pub fn new(rows: &'a [Row], x: &'a str) -> Self {
        Self {
            rows,
            x,
            y: &[],
            series: None,
            aggregate: &[],
            order: None,
            series_order: None,
            count: None,
        }
    }
}

/// The rows grouped by category, and by series where `series` names a field, each group's `y`
/// combined by `aggregate`: the sum where a `y` is given and the count where none is. Several `y`
/// fields make one series each, each combined its own way where `aggregate` lists one per field.
pub fn pivot(spec: &PivotSpec) -> Pivot {
    let categories = distinct(spec.rows, spec.x, spec.order);
    // A category the order names more than once has a place for each: its nth row goes to its
    // nth place, and any past the last join that one.
    let mut places: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, name) in categories.iter().enumerate() {
        places.entry(name.as_str()).or_default().push(index);
    }
    let collect = |pick: &dyn Fn(&Row) -> bool, field: Option<&str>, how: Aggregate| -> Vec<Option<f64>> {
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
            .map(|bucket| aggregate(bucket, if field.is_none() { Aggregate::Count } else { how }))
            .collect()
    };
    if let Some(series) = spec.series {
        let field = spec.y.first().map(String::as_str);
        let how = aggregate_at(spec.aggregate, 0);
        return Pivot {
            series: distinct(spec.rows, series, spec.series_order)
                .into_iter()
                .map(|name| {
                    let values = collect(&|row: &Row| label_of(row.get(series)) == name, field, how);
                    Series::gappy(name, values)
                })
                .collect(),
            categories,
        };
    }
    if spec.y.is_empty() {
        return Pivot {
            series: vec![Series::gappy(
                spec.count.unwrap_or("Count"),
                collect(&|_| true, None, Aggregate::Count),
            )],
            categories,
        };
    }
    Pivot {
        series: spec
            .y
            .iter()
            .enumerate()
            .map(|(index, field)| {
                Series::gappy(
                    field.clone(),
                    collect(&|_| true, Some(field), aggregate_at(spec.aggregate, index)),
                )
            })
            .collect(),
        categories,
    }
}

/// Each category's figures as shares of that category's total, in percent; a negative figure is a
/// negative share of the sizes added up, so no share passes 100.
pub fn as_shares(series: &[Series]) -> Vec<Series> {
    let totals: Vec<f64> = series.first().map_or_else(Vec::new, |first| {
        (0..first.values.len())
            .map(|index| {
                series
                    .iter()
                    .map(|entry| entry.values.get(index).copied().flatten().unwrap_or(0.0).abs())
                    .sum()
            })
            .collect()
    });
    series
        .iter()
        .map(|entry| Series {
            values: entry
                .values
                .iter()
                .enumerate()
                .map(|(index, value)| match (value, totals.get(index)) {
                    (Some(value), Some(total)) if *total != 0.0 => Some(value / total * 100.0),
                    _ => None,
                })
                .collect(),
            ..entry.clone()
        })
        .collect()
}

/// A figure written for a tooltip or a label, as the web's `formatNumber`: grouped thousands, at
/// most two decimals.
pub fn format_number(value: f64) -> String {
    let written = sokkel_egui::format_value(value, Some(2));
    match written.split_once('.') {
        Some((whole, fraction)) => {
            let fraction = fraction.trim_end_matches('0');
            if fraction.is_empty() {
                whole.to_owned()
            } else {
                format!("{whole}.{fraction}")
            }
        }
        None => written,
    }
}

/// A figure written for a value axis' tick, as the web's `formatTick`: short from ten up, `1.2K`
/// for 1,200, and with up to two decimals below, so the ticks of a small range never read the same.
pub fn format_tick(value: f64) -> String {
    if value.abs() < 10.0 {
        format_number(value)
    } else {
        sokkel_egui::format_compact(value)
    }
}

/// Whether a character is a space `String.prototype.trim` cuts.
fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

fn trim(text: &str) -> &str {
    text.trim_matches(is_space)
}

/// A number as `String(number)` writes it: the fewest digits that read back as it, in figures up
/// to 21 of them and with an exponent past that.
pub fn number_to_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    if value < 0.0 {
        return format!("-{}", number_to_string(-value));
    }
    let written = format!("{value:e}");
    let (mantissa, exponent) = written.split_once('e').unwrap_or((&written, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let k = digits.len() as i64;
    let n = exponent.parse::<i64>().unwrap_or(0) + 1;
    if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let sign = if n - 1 < 0 { '-' } else { '+' };
        let rest = if k > 1 {
            format!(".{}", &digits[1..])
        } else {
            String::new()
        };
        format!("{}{rest}e{sign}{}", &digits[..1], (n - 1).abs())
    }
}

/// A text read as `Number(text)` reads it: spaces at its ends left out, nothing as 0, a decimal
/// literal, `Infinity`, or a whole number in hexadecimal, octal or binary after `0x`, `0o` or
/// `0b`; anything else is NaN.
pub fn number_of_text(text: &str) -> f64 {
    let text = trim(text);
    if text.is_empty() {
        return 0.0;
    }
    let bytes = text.as_bytes();
    if bytes.len() > 2 && bytes[0] == b'0' {
        let radix = match bytes[1] {
            b'x' | b'X' => 16,
            b'o' | b'O' => 8,
            b'b' | b'B' => 2,
            _ => 0,
        };
        if radix > 0 {
            let mut value = 0.0_f64;
            for c in text[2..].chars() {
                match c.to_digit(radix) {
                    Some(digit) => value = value * f64::from(radix) + f64::from(digit),
                    None => return f64::NAN,
                }
            }
            return value;
        }
    }
    let (sign, body) = match bytes[0] {
        b'+' => (1.0, &text[1..]),
        b'-' => (-1.0, &text[1..]),
        _ => (1.0, text),
    };
    if body == "Infinity" {
        return sign * f64::INFINITY;
    }
    if !is_decimal_literal(body) {
        return f64::NAN;
    }
    body.parse::<f64>().map_or(f64::NAN, |value| sign * value)
}

/// Whether a text is a decimal literal without its sign: digits with a point among or after them,
/// or a point before digits, and an exponent after.
fn is_decimal_literal(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut at = 0;
    let digits = |at: &mut usize| {
        let start = *at;
        while *at < bytes.len() && bytes[*at].is_ascii_digit() {
            *at += 1;
        }
        *at - start
    };
    let whole = digits(&mut at);
    let mut fraction = 0;
    if at < bytes.len() && bytes[at] == b'.' {
        at += 1;
        fraction = digits(&mut at);
    }
    if whole == 0 && fraction == 0 {
        return false;
    }
    if at < bytes.len() && (bytes[at] == b'e' || bytes[at] == b'E') {
        at += 1;
        if at < bytes.len() && (bytes[at] == b'+' || bytes[at] == b'-') {
            at += 1;
        }
        if digits(&mut at) == 0 {
            return false;
        }
    }
    at == bytes.len()
}

#[cfg(feature = "serde")]
mod serialize {
    use serde::ser::{SerializeMap, SerializeStruct};
    use serde::{Serialize, Serializer};

    use super::{Cell, Pivot, Row};

    impl Serialize for Cell {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            match self {
                Cell::Text(text) => serializer.serialize_str(text),
                Cell::Number(value) if value.is_finite() => serializer.serialize_f64(*value),
                _ => serializer.serialize_none(),
            }
        }
    }

    impl Serialize for Row {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut map = serializer.serialize_map(Some(self.0.len()))?;
            for (key, value) in &self.0 {
                map.serialize_entry(key, value)?;
            }
            map.end()
        }
    }

    /// A series as the web's `{ name, values }`.
    struct Named<'a>(&'a crate::Series);

    impl Serialize for Named<'_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut out = serializer.serialize_struct("Series", 2)?;
            out.serialize_field("name", &self.0.name)?;
            out.serialize_field("values", &self.0.values)?;
            out.end()
        }
    }

    impl Serialize for Pivot {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let series: Vec<Named<'_>> = self.series.iter().map(Named).collect();
            let mut out = serializer.serialize_struct("Pivot", 2)?;
            out.serialize_field("categories", &self.categories)?;
            out.serialize_field("series", &series)?;
            out.end()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<Row> {
        let row = |job: &str, state: &str, duration: Cell| {
            Row::of([
                ("job", Cell::from(job)),
                ("state", Cell::from(state)),
                ("duration", duration),
            ])
        };
        vec![
            row("nightly-export", "succeeded", Cell::Number(100.0)),
            row("nightly-export", "failed", Cell::Number(40.0)),
            row("sync-inventory", "succeeded", Cell::Number(60.0)),
            row("nightly-export", "succeeded", Cell::Number(140.0)),
            row("sync-inventory", "succeeded", Cell::from("n/a")),
        ]
    }

    fn named(pivot: &Pivot) -> Vec<(String, Vec<Option<f64>>)> {
        pivot
            .series
            .iter()
            .map(|s| (s.name.clone(), s.values.clone()))
            .collect()
    }

    #[test]
    fn combines_values_each_way() {
        let four = [1.0, 2.0, 3.0, 10.0];
        assert_eq!(aggregate(&four, Aggregate::Sum), Some(16.0));
        assert_eq!(aggregate(&four, Aggregate::Count), Some(4.0));
        assert_eq!(aggregate(&four, Aggregate::Avg), Some(4.0));
        assert_eq!(aggregate(&four, Aggregate::Median), Some(2.5));
        assert_eq!(aggregate(&[], Aggregate::Sum), None);
        assert_eq!(aggregate(&[], Aggregate::Count), Some(0.0));
        assert_eq!(quantile(&[0.0, 10.0], 0.9), Some(9.0));
    }

    #[test]
    fn keeps_the_order_rows_give_with_a_named_order_first() {
        let rows = rows();
        assert_eq!(distinct(&rows, "job", None), ["nightly-export", "sync-inventory"]);
        assert_eq!(
            distinct(&rows, "job", Some(&["sync-inventory".to_owned()])),
            ["sync-inventory", "nightly-export"]
        );
    }

    #[test]
    fn counts_the_rows_when_no_y_is_given() {
        let rows = rows();
        let data = pivot(&PivotSpec::new(&rows, "job"));
        assert_eq!(data.categories, ["nightly-export", "sync-inventory"]);
        assert_eq!(named(&data), [("Count".to_owned(), vec![Some(3.0), Some(2.0)])]);
    }

    #[test]
    fn gives_a_category_the_order_names_twice_a_place_for_each() {
        let rows: Vec<Row> = [("19:32", 1.0), ("19:32", 2.0), ("19:33", 3.0)]
            .into_iter()
            .map(|(at, v)| Row::of([("at", Cell::from(at)), ("v", Cell::Number(v))]))
            .collect();
        let y = ["v".to_owned()];
        let order: Vec<String> = ["19:32", "19:32", "19:33"].map(String::from).to_vec();
        let data = pivot(&PivotSpec {
            y: &y,
            order: Some(&order),
            ..PivotSpec::new(&rows, "at")
        });
        assert_eq!(data.categories, ["19:32", "19:32", "19:33"]);
        assert_eq!(named(&data), [("v".to_owned(), vec![Some(1.0), Some(2.0), Some(3.0)])]);
        let data = pivot(&PivotSpec {
            y: &y,
            ..PivotSpec::new(&rows, "at")
        });
        assert_eq!(named(&data), [("v".to_owned(), vec![Some(3.0), Some(3.0)])]);
    }

    #[test]
    fn splits_by_series_and_skips_values_that_are_not_numbers() {
        let rows = rows();
        let y = ["duration".to_owned()];
        let data = pivot(&PivotSpec {
            y: &y,
            series: Some("state"),
            aggregate: &[Aggregate::Avg],
            ..PivotSpec::new(&rows, "job")
        });
        assert_eq!(
            named(&data),
            [
                ("succeeded".to_owned(), vec![Some(120.0), Some(60.0)]),
                ("failed".to_owned(), vec![Some(40.0), None]),
            ]
        );
    }

    #[test]
    fn several_y_fields_make_a_series_each_combined_their_own_way() {
        let rows: Vec<Row> = [(1.0, 5.0), (3.0, 7.0)]
            .into_iter()
            .map(|(a, b)| Row::of([("x", Cell::from("one")), ("a", a.into()), ("b", b.into())]))
            .collect();
        let y = ["a".to_owned(), "b".to_owned()];
        let data = pivot(&PivotSpec {
            y: &y,
            aggregate: &[Aggregate::Sum, Aggregate::Max],
            ..PivotSpec::new(&rows, "x")
        });
        assert_eq!(
            named(&data),
            [("a".to_owned(), vec![Some(4.0)]), ("b".to_owned(), vec![Some(7.0)])]
        );
    }

    #[test]
    fn turns_each_category_into_shares_of_its_total() {
        let shares = as_shares(&[Series::new("a", [1.0, 0.0]), Series::new("b", [3.0, 0.0])]);
        assert_eq!(shares[0].values, [Some(25.0), None]);
        assert_eq!(shares[1].values, [Some(75.0), None]);
        let signed = as_shares(&[Series::new("up", [100.0]), Series::new("down", [-50.0])]);
        assert!((signed[0].values[0].unwrap() - 66.67).abs() < 0.01);
        assert!((signed[1].values[0].unwrap() + 33.33).abs() < 0.01);
    }

    #[test]
    fn spans_many_values() {
        let many: Vec<f64> = (0..300_000).map(|i| f64::from(i) - 1000.0).collect();
        assert_eq!(extent(&many), Some((-1000.0, 298_999.0)));
        assert_eq!(extent(&[]), None);
        assert_eq!(aggregate(&many, Aggregate::Max), Some(298_999.0));
        assert_eq!(aggregate(&many, Aggregate::Min), Some(-1000.0));
    }

    #[test]
    fn keeps_the_ticks_of_a_small_range_apart() {
        let ticks: Vec<String> = [0.0, 0.05, 0.1, 0.15, 0.2].into_iter().map(format_tick).collect();
        assert_eq!(ticks, ["0", "0.05", "0.1", "0.15", "0.2"]);
        assert_eq!(
            [format_tick(1250.0), format_tick(42.0), format_tick(2.5)],
            ["1.3K", "42", "2.5"]
        );
        assert_eq!(format_number(38929.0), "38,929");
        assert_eq!(format_number(1.005), "1");
        assert_eq!(format_number(-2.5), "-2.5");
    }

    #[test]
    fn reads_fields_as_the_web_reads_them() {
        assert_eq!(number_of(Some(&Cell::from(" 42 "))), Some(42.0));
        assert_eq!(number_of(Some(&Cell::from("0x10"))), Some(16.0));
        assert_eq!(number_of(Some(&Cell::from("   "))), None);
        assert_eq!(number_of(Some(&Cell::from("n/a"))), None);
        assert_eq!(number_of(Some(&Cell::Number(f64::NAN))), None);
        assert_eq!(number_of(None), None);
        assert_eq!(label_of(Some(&Cell::Number(2025.0))), "2025");
        assert_eq!(label_of(Some(&Cell::Number(0.1 + 0.2))), "0.30000000000000004");
        assert_eq!(label_of(Some(&Cell::Number(1e21))), "1e+21");
        assert_eq!(label_of(Some(&Cell::Null)), "");
    }
}
