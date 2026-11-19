//! A chart shared as a link that carries its whole dataset, as the web's `share.ts` packs it: the
//! chart's name, its view and its data as JSON, deflated, in base64url after a `z` (or plain
//! after a `j`), read back with every part checked so nothing drawn from it meets a value it
//! cannot read.

use serde::Serialize;

use crate::js::{self, Json};
use crate::model::{
    Aggregate, Column, ColumnType, DataRow, Dataset, DatasetSource, DateBin, SortOrder, ViewKind, ViewSpec,
};

/// What a shared link carries: the chart's name, its view, and its whole dataset, so a fresh load
/// needs nothing else.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SharedChart {
    pub name: String,
    pub spec: ViewSpec,
    pub dataset: SharedData,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SharedData {
    pub name: String,
    pub description: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Serialize)]
struct Packed<'a> {
    v: u8,
    n: &'a str,
    s: &'a ViewSpec,
    d: PackedData<'a>,
}

#[derive(Serialize)]
struct PackedData<'a> {
    n: &'a str,
    t: &'a str,
    c: &'a [Column],
    r: Vec<Vec<&'a str>>,
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Bytes in base64url, without padding.
pub fn to_base64_url(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let word = (u32::from(chunk[0]) << 16)
            | (u32::from(chunk.get(1).copied().unwrap_or(0)) << 8)
            | u32::from(chunk.get(2).copied().unwrap_or(0));
        for at in 0..=chunk.len() {
            out.push(ALPHABET[(word >> (18 - 6 * at) & 63) as usize] as char);
        }
    }
    out.replace('+', "-").replace('/', "_")
}

/// Base64url read back as `atob` reads base64: spaces left out, up to two `=` at the end of a
/// whole, and anything else outside the alphabet refused.
pub fn from_base64_url(text: &str) -> Option<Vec<u8>> {
    let mut text: Vec<u8> = text
        .bytes()
        .map(|byte| match byte {
            b'-' => b'+',
            b'_' => b'/',
            other => other,
        })
        .filter(|byte| !matches!(byte, b'\t' | b'\n' | b'\x0c' | b'\r' | b' '))
        .collect();
    if text.len().is_multiple_of(4) {
        for _ in 0..2 {
            if text.last() == Some(&b'=') {
                text.pop();
            }
        }
    }
    if text.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for byte in text {
        let value = ALPHABET.iter().position(|one| *one == byte)? as u32;
        buffer = (buffer << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// A chart and its data as the JSON a link packs.
pub fn packed_json(name: &str, spec: &ViewSpec, dataset: &Dataset) -> String {
    let packed = Packed {
        v: 1,
        n: name,
        s: spec,
        d: PackedData {
            n: &dataset.name,
            t: &dataset.description,
            c: &dataset.columns,
            r: dataset
                .rows
                .iter()
                .map(|row| dataset.columns.iter().map(|column| row.text(&column.id)).collect())
                .collect(),
        },
    };
    serde_json::to_string(&packed).unwrap_or_default()
}

/// A chart and its data as one piece of a link's hash: JSON, deflated, in base64url after a `z`;
/// or, where `deflate` is false (a browser without compression streams), the JSON itself after a
/// `j`.
pub fn pack_chart(name: &str, spec: &ViewSpec, dataset: &Dataset, deflate: bool) -> String {
    let bytes = packed_json(name, spec, dataset).into_bytes();
    if !deflate {
        return format!("j{}", to_base64_url(&bytes));
    }
    format!("z{}", to_base64_url(&miniz_oxide::deflate::compress_to_vec(&bytes, 6)))
}

const CUT: &str = "The link is cut short or changed: its chart cannot be read.";

/// Reads a chart packed into a link back, or says why it cannot.
pub fn unpack_chart(text: &str) -> Result<SharedChart, String> {
    let mut chars = text.chars();
    let kind = chars.next();
    let mut bytes = from_base64_url(chars.as_str()).ok_or(CUT)?;
    match kind {
        Some('z') => {
            bytes = miniz_oxide::inflate::decompress_to_vec(&bytes).map_err(|_| CUT)?;
        }
        Some('j') => {}
        _ => return Err("This is not a link to a shared chart.".to_owned()),
    }
    let read = js::json_parse(&String::from_utf8_lossy(&bytes)).map_err(|_| CUT)?;
    packed_of(&read).ok_or_else(|| "The link was made by a version that cannot be read here.".to_owned())
}

fn text_of(value: Option<&Json>) -> Option<String> {
    match value {
        Some(Json::Text(text)) => Some(text.clone()),
        _ => None,
    }
}

/// A column read from what a link carried: an id, a name and a known type.
fn column_of(value: &Json) -> Option<Column> {
    if !matches!(value, Json::Object(_)) {
        return None;
    }
    let kind = ColumnType::from_id(&text_of(value.get("type"))?)?;
    Some(Column {
        id: text_of(value.get("id"))?,
        name: text_of(value.get("name"))?,
        kind,
    })
}

/// A view read from what a link carried, every field of the shape the builder draws from.
fn spec_of(value: &Json) -> Option<ViewSpec> {
    if !matches!(value, Json::Object(_)) {
        return None;
    }
    let field = |key: &str| -> Option<Option<String>> {
        match value.get(key) {
            None => Some(None),
            Some(Json::Text(text)) => Some(Some(text.clone())),
            Some(_) => None,
        }
    };
    let limit = match value.get("limit") {
        Some(Json::Number(limit)) if limit.is_finite() => *limit,
        _ => return None,
    };
    let flag = |key: &str| match value.get(key) {
        Some(Json::Bool(flag)) => Some(*flag),
        _ => None,
    };
    Some(ViewSpec {
        view: ViewKind::from_id(&text_of(value.get("view"))?)?,
        x: field("x")?,
        y: field("y")?,
        series: field("series")?,
        size: field("size")?,
        color: field("color")?,
        aggregate: Aggregate::from_id(&text_of(value.get("aggregate"))?)?,
        bin: DateBin::from_id(&text_of(value.get("bin"))?)?,
        sort: SortOrder::from_id(&text_of(value.get("sort"))?)?,
        limit,
        stack: flag("stack")?,
        horizontal: flag("horizontal")?,
        palette: text_of(value.get("palette"))?,
    })
}

/// What a link carried, where it has the whole shape of a packed chart.
fn packed_of(value: &Json) -> Option<SharedChart> {
    if !matches!(value, Json::Object(_)) || value.get("v") != Some(&Json::Number(1.0)) {
        return None;
    }
    let name = text_of(value.get("n"))?;
    let spec = spec_of(value.get("s")?)?;
    let data = value.get("d").filter(|data| matches!(data, Json::Object(_)))?;
    let data_name = text_of(data.get("n"))?;
    let description = text_of(data.get("t"))?;
    let Some(Json::Array(columns)) = data.get("c") else {
        return None;
    };
    let columns = columns.iter().map(column_of).collect::<Option<Vec<_>>>()?;
    let Some(Json::Array(rows)) = data.get("r") else {
        return None;
    };
    let rows = rows
        .iter()
        .map(|row| match row {
            Json::Array(cells) => cells.iter().map(|cell| text_of(Some(cell))).collect::<Option<Vec<_>>>(),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some(SharedChart {
        name,
        spec,
        dataset: SharedData {
            name: data_name,
            description,
            columns,
            rows,
        },
    })
}

/// The dataset a shared chart carries, as a dataset of its own under `id`.
pub fn shared_dataset(shared: &SharedChart, id: &str, updated_at: &str) -> Dataset {
    Dataset {
        id: id.to_owned(),
        name: shared.dataset.name.clone(),
        description: shared.dataset.description.clone(),
        source: DatasetSource::Link,
        columns: shared.dataset.columns.clone(),
        rows: shared
            .dataset
            .rows
            .iter()
            .enumerate()
            .map(|(index, values)| {
                let mut row = DataRow::new(format!("{id}-r{}", index + 1));
                for (at, column) in shared.dataset.columns.iter().enumerate() {
                    row.set(&column.id, values.get(at).cloned().unwrap_or_default());
                }
                row
            })
            .collect(),
        updated_at: updated_at.to_owned(),
    }
}

/// The address of the app with a hash, as a link someone else opens: the page's own address, or
/// `file://` and its path where the page has no origin (a desktop shell's file).
pub fn link_to(hash: &str, origin: &str, pathname: &str) -> String {
    let origin = if origin == "null" { "file://" } else { origin };
    format!("{origin}{pathname}{hash}")
}
