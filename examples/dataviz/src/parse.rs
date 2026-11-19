//! Reading a table from a file or pasted text, as the web's `parse.ts` reads it: CSV, TSV or text
//! split by semicolons, or JSON; the columns' names made unique and their types guessed from the
//! values; each value kept as its type writes it; and a table written back as CSV under a file
//! name made from a title.

use serde::Serialize;

use crate::js::{self, Json};
use crate::model::{Column, ColumnType, DataRow};
use crate::seed::REGION_NAMES;
use sokkel_egui_charts::data::Cell;

/// A table read from a file or pasted text, before it becomes a dataset: its columns' names and
/// each row's values as text.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ParsedTable {
    pub names: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Csv,
    Tsv,
    Json,
}

/// What reading gave: the table and its format, or why it could not be read.
pub type ParseResult = Result<(ParsedTable, Format), String>;

/// The first line of a text, as `text.split(/\r?\n/, 1)[0]` cuts it.
fn first_line(text: &str) -> &str {
    let end = text.find('\n').unwrap_or(text.len());
    let line = &text[..end];
    if end < text.len() {
        line.strip_suffix('\r').unwrap_or(line)
    } else {
        line
    }
}

/// The delimiter a text of rows uses: the one that splits its first line the most, the first of
/// a tie.
pub fn delimiter_of(text: &str) -> char {
    let first = first_line(text);
    let mut best = (',', first.split(',').count());
    for mark in [';', '\t'] {
        let count = first.split(mark).count();
        if count > best.1 {
            best = (mark, count);
        }
    }
    best.0
}

/// Rows of delimited text, with quoted values that may hold the delimiter, quotes doubled and line
/// breaks; rows of nothing but spaces are left out.
pub fn read_delimited(text: &str, delimiter: Option<char>) -> Vec<Vec<String>> {
    let delimiter = delimiter.unwrap_or_else(|| delimiter_of(text));
    let chars: Vec<char> = text.chars().collect();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut value = String::new();
    let mut quoted = false;
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        if quoted {
            if c == '"' && chars.get(at + 1) == Some(&'"') {
                value.push('"');
                at += 1;
            } else if c == '"' {
                quoted = false;
            } else {
                value.push(c);
            }
            at += 1;
            continue;
        }
        if c == '"' && value.is_empty() {
            quoted = true;
        } else if c == delimiter {
            row.push(std::mem::take(&mut value));
        } else if c == '\n' || c == '\r' {
            if c == '\r' && chars.get(at + 1) == Some(&'\n') {
                at += 1;
            }
            row.push(std::mem::take(&mut value));
            rows.push(std::mem::take(&mut row));
        } else {
            value.push(c);
        }
        at += 1;
    }
    if !value.is_empty() || !row.is_empty() {
        row.push(value);
        rows.push(row);
    }
    rows.retain(|one| one.iter().any(|cell| !js::trim(cell).is_empty()));
    rows
}

/// Text of a JSON value as a cell holds it: none as empty, an object or a list as JSON.
fn cell_text(value: Option<&Json>) -> String {
    match value {
        None | Some(Json::Null) => String::new(),
        Some(Json::Bool(value)) => value.to_string(),
        Some(Json::Number(value)) => js::number_to_string(*value),
        Some(Json::Text(text)) => text.clone(),
        Some(other) => other.stringify(),
    }
}

/// A JSON document as a table: an array of objects, an array of arrays under a header row, or
/// either under `rows` or `data`.
fn read_json(text: &str) -> ParseResult {
    let parsed = js::json_parse(text).map_err(|error| format!("This is not JSON that can be read: {error}"))?;
    let list = match &parsed {
        Json::Array(items) => Some(items),
        _ => match (parsed.get("rows"), parsed.get("data")) {
            (Some(Json::Array(items)), _) => Some(items),
            (_, Some(Json::Array(items))) => Some(items),
            _ => None,
        },
    };
    let list = match list {
        Some(list) if !list.is_empty() => list,
        _ => {
            return Err("The JSON holds no list of rows: give an array of objects, or one under \"rows\".".to_owned());
        }
    };
    if list.iter().all(|row| matches!(row, Json::Array(_))) {
        let cells = |row: &Json| match row {
            Json::Array(items) => items.iter().map(|item| cell_text(Some(item))).collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        return Ok((
            ParsedTable {
                names: cells(&list[0]),
                rows: list[1..].iter().map(cells).collect(),
            },
            Format::Json,
        ));
    }
    if !list.iter().all(Json::is_object) {
        return Err("Every row of the JSON must be an object of named values.".to_owned());
    }
    let mut names: Vec<String> = Vec::new();
    for row in list {
        for key in row.keys() {
            if !names.contains(&key) {
                names.push(key);
            }
        }
    }
    let rows = list
        .iter()
        .map(|row| names.iter().map(|name| cell_text(row.get(name))).collect())
        .collect();
    Ok((ParsedTable { names, rows }, Format::Json))
}

/// Reads text as a table: JSON when it opens like JSON, otherwise rows split by commas,
/// semicolons or tabs, the first row naming the columns.
pub fn parse_table(text: &str) -> ParseResult {
    let trimmed = js::trim(text);
    if trimmed.is_empty() {
        return Err("There is nothing to read.".to_owned());
    }
    if trimmed.starts_with('[') || trimmed.starts_with('{') {
        return read_json(trimmed);
    }
    let delimiter = delimiter_of(trimmed);
    let mut rows = read_delimited(trimmed, Some(delimiter));
    if rows.len() < 2 {
        return Err("A table needs a row of column names and at least one row of values.".to_owned());
    }
    let head = rows.remove(0);
    let width = head.len();
    Ok((
        ParsedTable {
            names: head.iter().map(|name| js::trim(name).to_owned()).collect(),
            rows: rows
                .iter()
                .map(|row| {
                    (0..width)
                        .map(|at| js::trim(row.get(at).map_or("", String::as_str)).to_owned())
                        .collect()
                })
                .collect(),
        },
        if delimiter == '\t' { Format::Tsv } else { Format::Csv },
    ))
}

/// Names made unique and never empty: a blank becomes `Column 3`, a repeat `Name 2`.
pub fn unique_names(names: &[String]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    names
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            let trimmed = js::trim(raw);
            let base = if trimmed.is_empty() {
                format!("Column {}", index + 1)
            } else {
                trimmed.to_owned()
            };
            let mut name = base.clone();
            let mut count = 2;
            while seen.contains(&name.to_lowercase()) {
                name = format!("{base} {count}");
                count += 1;
            }
            seen.push(name.to_lowercase());
            name
        })
        .collect()
}

/// Whether a text reads as a number: a sign, digits or digits grouped by commas in threes, a
/// fraction and an exponent, as the web's pattern takes them.
fn is_number_text(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut at = 0;
    if matches!(bytes.first(), Some(b'-' | b'+')) {
        at += 1;
    }
    let digits_from = |at: usize| bytes[at..].iter().take_while(|b| b.is_ascii_digit()).count();
    let first = digits_from(at);
    if first == 0 {
        return false;
    }
    // Digits grouped by commas: one to three, then threes.
    let mut grouped_end = None;
    if first <= 3 && bytes.get(at + first) == Some(&b',') {
        let mut end = at + first;
        let mut groups = 0;
        while bytes.get(end) == Some(&b',') && digits_from(end + 1) >= 3 {
            end += 4;
            groups += 1;
        }
        if groups > 0 {
            grouped_end = Some(end);
        }
    }
    let rest = |mut at: usize| {
        if bytes.get(at) == Some(&b'.') {
            let fraction = digits_from(at + 1);
            if fraction == 0 {
                return false;
            }
            at += 1 + fraction;
        }
        if matches!(bytes.get(at), Some(b'e' | b'E')) {
            at += 1;
            if matches!(bytes.get(at), Some(b'-' | b'+')) {
                at += 1;
            }
            let exponent = digits_from(at);
            if exponent == 0 {
                return false;
            }
            at += exponent;
        }
        at == bytes.len()
    };
    rest(at + first) || grouped_end.is_some_and(rest)
}

/// A number from a cell's text, with thousands commas allowed, or none.
pub fn number_of(text: &str) -> Option<f64> {
    let trimmed = js::trim(text);
    if !is_number_text(trimmed) {
        return None;
    }
    let value = js::number_of_text(&trimmed.replace(',', ""));
    value.is_finite().then_some(value)
}

fn digits_at(text: &str, ranges: &[std::ops::Range<usize>]) -> bool {
    let bytes = text.as_bytes();
    ranges.iter().all(|range| {
        bytes
            .get(range.clone())
            .is_some_and(|part| part.iter().all(u8::is_ascii_digit))
    })
}

/// A day as `YYYY-MM-DD` from a cell's text (a date, a time, or a month), or none.
pub fn day_of(text: &str) -> Option<String> {
    let trimmed = js::trim(text);
    let bytes = trimmed.as_bytes();
    if bytes.len() == 7 && bytes[4] == b'-' && digits_at(trimmed, &[0..4, 5..7]) {
        return Some(format!("{trimmed}-01"));
    }
    if bytes.len() < 10 || bytes[4] != b'-' || bytes[7] != b'-' || !digits_at(trimmed, &[0..4, 5..7, 8..10]) {
        return None;
    }
    let day = &trimmed[..10];
    js::day_number(day).map(|_| day.to_owned())
}

/// Whether a text names one of the map's regions.
pub fn is_region(text: &str) -> bool {
    let wanted = js::trim(text).to_lowercase();
    REGION_NAMES.iter().any(|name| name.to_lowercase() == wanted)
}

/// The type a column's values read as best: every value a number or a date, most of them regions,
/// or text.
pub fn guess_type(values: &[&str]) -> ColumnType {
    let filled: Vec<&str> = values
        .iter()
        .copied()
        .filter(|value| !js::trim(value).is_empty())
        .collect();
    if filled.is_empty() {
        return ColumnType::Text;
    }
    if filled.iter().all(|value| number_of(value).is_some()) {
        return ColumnType::Number;
    }
    if filled.iter().all(|value| day_of(value).is_some()) {
        return ColumnType::Date;
    }
    if filled.iter().filter(|value| is_region(value)).count() as f64 >= filled.len() as f64 * 0.8 {
        return ColumnType::Region;
    }
    ColumnType::Text
}

/// Columns for a parsed table: unique names and guessed types, with ids `c1`, `c2` in order.
pub fn columns_of(table: &ParsedTable) -> Vec<Column> {
    unique_names(&table.names)
        .into_iter()
        .enumerate()
        .map(|(at, name)| {
            let values: Vec<&str> = table
                .rows
                .iter()
                .map(|row| row.get(at).map_or("", String::as_str))
                .collect();
            Column {
                id: format!("c{}", at + 1),
                name,
                kind: guess_type(&values),
            }
        })
        .collect()
}

/// A cell's text kept as its type writes it: numbers without thousands commas, dates as
/// `YYYY-MM-DD`.
pub fn normalize(text: &str, kind: ColumnType) -> String {
    match kind {
        ColumnType::Number => number_of(text).map_or_else(|| js::trim(text).to_owned(), js::number_to_string),
        ColumnType::Date => day_of(text).unwrap_or_else(|| js::trim(text).to_owned()),
        _ => js::trim(text).to_owned(),
    }
}

/// The rows of a parsed table, keyed by the columns' ids, each value kept as its column's type
/// writes it.
pub fn rows_of(table: &ParsedTable, columns: &[Column], prefix: &str) -> Vec<DataRow> {
    table
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let mut out = DataRow::new(format!("{prefix}-r{}", index + 1));
            for (at, column) in columns.iter().enumerate() {
                out.set(
                    &column.id,
                    normalize(row.get(at).map_or("", String::as_str), column.kind),
                );
            }
            out
        })
        .collect()
}

/// A value written for CSV: quoted when it holds a comma, a quote or a line break.
fn csv_cell(value: &str) -> String {
    if value.contains(['"', ',', '\r', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

/// A table as CSV text: a header row of names, then a row per record, each line ended by CRLF.
pub fn to_csv(names: &[String], rows: &[Vec<Cell>]) -> String {
    let mut out = names.iter().map(|name| csv_cell(name)).collect::<Vec<_>>().join(",");
    out.push_str("\r\n");
    for row in rows {
        out.push_str(
            &row.iter()
                .map(|cell| csv_cell(&cell.written()))
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push_str("\r\n");
    }
    out
}

/// A file name from a title: lowercase words joined by dashes, `chart` where none is left.
pub fn file_name(title: &str) -> String {
    let lower = title.to_lowercase();
    let mut slug = String::new();
    let mut gap = false;
    for c in lower.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if gap && !slug.is_empty() {
                slug.push('-');
                gap = false;
            }
            slug.push(c);
        } else {
            gap = true;
        }
    }
    // A run at the start becomes one dash, which the trim then takes; a run at the end, too.
    if slug.is_empty() { "chart".to_owned() } else { slug }
}
