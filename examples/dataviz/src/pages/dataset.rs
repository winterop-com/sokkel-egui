//! A dataset on its own page, as the web's `pages/Dataset.tsx` keeps it: its rows searched,
//! filtered and edited in place, its columns renamed, retyped, added and deleted, each column's
//! values summed up, and the charts drawn from it.

use serde::Serialize;
use sokkel_egui::{Filter, filter_rows};

use crate::chart_data::Cell;
use crate::model::{Column, Dataset, Preferences, SavedChart, counted};
use crate::shape::{TypedRow, filter_fields, filter_fields_of, format_figure, typed_rows};

/// A type as the rows' heads say it after the column's name: `Rainfall · number`.
pub fn header(column: &Column) -> String {
    format!("{} · {}", column.name, column.kind.id())
}

/// The header's count: rows and columns.
pub fn count(dataset: &Dataset) -> String {
    format!(
        "{}, {}",
        counted(dataset.rows.len(), "row"),
        counted(dataset.columns.len(), "column")
    )
}

/// The rows shown: those that pass the filters, then those a search finds in any column's text.
pub fn shown<'a>(dataset: &'a Dataset, typed: &'a [TypedRow], filters: &[Filter], query: &str) -> Vec<&'a TypedRow> {
    let fields = filter_fields_of(&filter_fields(dataset, typed));
    let filtered = filter_rows(typed, filters, &fields);
    let wanted = crate::js::trim(query).to_lowercase();
    filtered
        .into_iter()
        .filter(|line| {
            wanted.is_empty() || {
                let row = dataset.rows.iter().find(|row| row.id == line.id);
                dataset
                    .columns
                    .iter()
                    .any(|column| row.is_some_and(|row| row.text(&column.id).to_lowercase().contains(&wanted)))
            }
        })
        .collect()
}

/// What a row's cell shows: `-` for nothing, the text kept where it is not of the column's type,
/// a number in the settings' format, or the value.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", content = "text", rename_all = "camelCase")]
pub enum Shown {
    Empty,
    Unread(String),
    Value(String),
}

pub fn cell(text: &str, value: &Cell, prefs: &Preferences) -> Shown {
    if text.is_empty() {
        return Shown::Empty;
    }
    match value {
        Cell::Null => Shown::Unread(text.to_owned()),
        Cell::Number(number) => Shown::Value(format_figure(prefs.numbers, prefs.decimals, *number)),
        Cell::Text(value) => Shown::Value(value.clone()),
    }
}

/// A column summed up: its range or how many values it holds, how many cells are empty and how
/// many hold what its type cannot read.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ColumnStats {
    pub range: String,
    pub empty: usize,
    pub unread: usize,
}

pub fn stats(dataset: &Dataset, typed: &[TypedRow], column: &Column, prefs: &Preferences) -> ColumnStats {
    let values: Vec<&Cell> = typed.iter().map(|line| line.get(&column.id)).collect();
    let filled: Vec<&Cell> = values.iter().copied().filter(|value| !value.is_null()).collect();
    let unread = dataset
        .rows
        .iter()
        .zip(typed)
        .filter(|(row, line)| !row.text(&column.id).is_empty() && line.get(&column.id).is_null())
        .count();
    let numbers: Vec<f64> = filled.iter().filter_map(|value| value.as_number()).collect();
    let mut days: Vec<&str> = if column.kind == crate::model::ColumnType::Date {
        filled.iter().filter_map(|value| value.as_text()).collect()
    } else {
        Vec::new()
    };
    days.sort();
    let format = |value: f64| format_figure(prefs.numbers, prefs.decimals, value);
    let range = if !numbers.is_empty() {
        let low = numbers.iter().copied().fold(f64::INFINITY, f64::min);
        let high = numbers.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        format!("{} to {}", format(low), format(high))
    } else if let (Some(first), Some(last)) = (days.first(), days.last()) {
        format!("{first} to {last}")
    } else {
        let mut distinct: Vec<&Cell> = Vec::new();
        for value in &filled {
            if !distinct.contains(value) {
                distinct.push(value);
            }
        }
        counted(distinct.len(), "value")
    };
    ColumnStats {
        range,
        empty: values.len() - filled.len() - unread,
        unread,
    }
}

/// Why a column's new name cannot be kept.
pub fn validate_name(dataset: &Dataset, column: &Column, value: &str) -> Option<&'static str> {
    let trimmed = crate::js::trim(value);
    if trimmed.is_empty() {
        return Some("A column needs a name.");
    }
    dataset
        .columns
        .iter()
        .any(|one| one.id != column.id && one.name.to_lowercase() == trimmed.to_lowercase())
        .then_some("Another column has this name.")
}

/// The charts drawn from a dataset.
pub fn drawn<'a>(dataset: &Dataset, charts: &'a [SavedChart]) -> Vec<&'a SavedChart> {
    charts.iter().filter(|chart| chart.dataset_id == dataset.id).collect()
}

/// The Charts tab's label, with how many there are.
pub fn charts_tab(dataset: &Dataset, charts: &[SavedChart]) -> String {
    let count = drawn(dataset, charts).len();
    format!(
        "Charts {}",
        if count > 0 { format!("({count})") } else { String::new() }
    )
}

/// The question and its detail before a column is deleted, naming the charts that read it.
pub fn drop_question(dataset: &Dataset, charts: &[SavedChart], column: &Column) -> (String, String) {
    let readers: Vec<&str> = drawn(dataset, charts)
        .into_iter()
        .filter(|chart| chart.spec.names_column(&column.id))
        .map(|chart| chart.name.as_str())
        .collect();
    (
        format!("Delete the column {}?", column.name),
        if readers.is_empty() {
            "Its values go from every row. No saved chart reads it.".to_owned()
        } else {
            format!(
                "Its values go from every row, and {} will need another column.",
                readers.join(", ")
            )
        },
    )
}

/// What the rows' table says when it shows none.
pub fn empty_rows(filtered: bool) -> &'static str {
    if filtered {
        "No rows match the search and filters."
    } else {
        "No rows yet. Add one, or import data."
    }
}

/// Every row read by type, as the page holds them.
pub fn lines(dataset: &Dataset) -> Vec<TypedRow> {
    typed_rows(dataset)
}

/// The page's message where the dataset is gone.
pub const NO_DATASET: &str = "There is no such dataset here. It may have been deleted.";
