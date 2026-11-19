//! What the visualizer keeps, as the web's `model.ts` names it: datasets of typed columns and rows
//! of text, the views a chart is drawn as and the spec it reads its columns by, saved charts,
//! boards of them with their filters, and the preferences; with the lists the builder and the
//! settings offer, the palettes, the boards' order, fresh ids and counted nouns.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sokkel_egui::{Filter, FilterOperator};

use crate::js;

/// What a column holds, which decides how its values are read, edited, filtered and charted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColumnType {
    Text,
    Number,
    Date,
    Region,
}

impl ColumnType {
    pub const ALL: [ColumnType; 4] = [
        ColumnType::Text,
        ColumnType::Number,
        ColumnType::Date,
        ColumnType::Region,
    ];

    pub fn id(self) -> &'static str {
        match self {
            ColumnType::Text => "text",
            ColumnType::Number => "number",
            ColumnType::Date => "date",
            ColumnType::Region => "region",
        }
    }

    pub fn from_id(id: &str) -> Option<ColumnType> {
        ColumnType::ALL.into_iter().find(|one| one.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            ColumnType::Text => "Text",
            ColumnType::Number => "Number",
            ColumnType::Date => "Date",
            ColumnType::Region => "Region",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            ColumnType::Text => "Names and labels, charted as categories",
            ColumnType::Number => "Figures that are added up, averaged and plotted",
            ColumnType::Date => "Days as YYYY-MM-DD, binned by day, week or month",
            ColumnType::Region => "Region names, drawn on the map",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Column {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ColumnType,
}

impl Column {
    pub fn new(id: &str, name: &str, kind: ColumnType) -> Self {
        Column {
            id: id.to_owned(),
            name: name.to_owned(),
            kind,
        }
    }
}

/// A row: its id and each column's value as text by the column's id, empty for none. The values
/// keep the order they were set in, as the web's object keeps its keys.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DataRow {
    pub id: String,
    pub values: Vec<(String, String)>,
}

impl DataRow {
    pub fn new(id: impl Into<String>) -> Self {
        DataRow {
            id: id.into(),
            values: Vec::new(),
        }
    }

    /// A column's value, as `row[column.id]` reads it: none where the row has no such key.
    pub fn get(&self, column: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(key, _)| key == column)
            .map(|(_, value)| value.as_str())
    }

    /// A column's value, empty where it has none, as `row[column.id] ?? ''`.
    pub fn text(&self, column: &str) -> &str {
        self.get(column).unwrap_or("")
    }

    /// Sets a column's value, in its place where the row has it, after the rest where not.
    pub fn set(&mut self, column: &str, value: impl Into<String>) {
        let value = value.into();
        match self.values.iter_mut().find(|(key, _)| key == column) {
            Some(entry) => entry.1 = value,
            None => self.values.push((column.to_owned(), value)),
        }
    }

    /// Leaves a column's key out.
    pub fn drop_key(&mut self, column: &str) {
        self.values.retain(|(key, _)| key != column);
    }
}

impl Serialize for DataRow {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.values.len() + 1))?;
        map.serialize_entry("id", &self.id)?;
        for (key, value) in &self.values {
            if key != "id" {
                map.serialize_entry(key, value)?;
            }
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for DataRow {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = serde_json::Map::<String, serde_json::Value>::deserialize(deserializer)?;
        let mut row = DataRow::default();
        for (key, value) in map {
            let text = value
                .as_str()
                .ok_or_else(|| D::Error::custom("a row's values are texts"))?
                .to_owned();
            if key == "id" {
                row.id = text;
            } else {
                row.values.push((key, text));
            }
        }
        Ok(row)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatasetSource {
    Seed,
    File,
    Paste,
    Link,
}

impl DatasetSource {
    pub fn id(self) -> &'static str {
        match self {
            DatasetSource::Seed => "seed",
            DatasetSource::File => "file",
            DatasetSource::Paste => "paste",
            DatasetSource::Link => "link",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dataset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: DatasetSource,
    pub columns: Vec<Column>,
    pub rows: Vec<DataRow>,
    /// When it last changed, as an ISO time.
    pub updated_at: String,
}

impl Dataset {
    /// A column by its id, or none where the dataset has none.
    pub fn column(&self, id: Option<&str>) -> Option<&Column> {
        let id = id?;
        self.columns.iter().find(|column| column.id == id)
    }
}

/// The views a chart is drawn as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewKind {
    Bar,
    Line,
    Area,
    Scatter,
    Pie,
    Heatmap,
    Map,
    Table,
    Stat,
}

impl ViewKind {
    pub const ALL: [ViewKind; 9] = [
        ViewKind::Bar,
        ViewKind::Line,
        ViewKind::Area,
        ViewKind::Scatter,
        ViewKind::Pie,
        ViewKind::Heatmap,
        ViewKind::Map,
        ViewKind::Table,
        ViewKind::Stat,
    ];

    pub fn id(self) -> &'static str {
        match self {
            ViewKind::Bar => "bar",
            ViewKind::Line => "line",
            ViewKind::Area => "area",
            ViewKind::Scatter => "scatter",
            ViewKind::Pie => "pie",
            ViewKind::Heatmap => "heatmap",
            ViewKind::Map => "map",
            ViewKind::Table => "table",
            ViewKind::Stat => "stat",
        }
    }

    pub fn from_id(id: &str) -> Option<ViewKind> {
        ViewKind::ALL.into_iter().find(|one| one.id() == id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Aggregate {
    Sum,
    Avg,
    Count,
    Min,
    Max,
}

impl Aggregate {
    /// The aggregates in the order the builder offers them.
    pub const ALL: [Aggregate; 5] = [
        Aggregate::Sum,
        Aggregate::Avg,
        Aggregate::Count,
        Aggregate::Min,
        Aggregate::Max,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Aggregate::Sum => "sum",
            Aggregate::Avg => "avg",
            Aggregate::Count => "count",
            Aggregate::Min => "min",
            Aggregate::Max => "max",
        }
    }

    pub fn from_id(id: &str) -> Option<Aggregate> {
        Aggregate::ALL.into_iter().find(|one| one.id() == id)
    }

    /// The charts' data module's way of combining figures, the same one.
    pub fn combine(self) -> sokkel_egui_charts::data::Aggregate {
        use sokkel_egui_charts::data::Aggregate as Combine;
        match self {
            Aggregate::Sum => Combine::Sum,
            Aggregate::Avg => Combine::Avg,
            Aggregate::Count => Combine::Count,
            Aggregate::Min => Combine::Min,
            Aggregate::Max => Combine::Max,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Aggregate::Sum => "Sum",
            Aggregate::Avg => "Mean",
            Aggregate::Count => "Count",
            Aggregate::Min => "Min",
            Aggregate::Max => "Max",
        }
    }
}

/// How dates along a chart are grouped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DateBin {
    Day,
    Week,
    Month,
}

impl DateBin {
    pub const ALL: [DateBin; 3] = [DateBin::Day, DateBin::Week, DateBin::Month];

    pub fn id(self) -> &'static str {
        match self {
            DateBin::Day => "day",
            DateBin::Week => "week",
            DateBin::Month => "month",
        }
    }

    pub fn from_id(id: &str) -> Option<DateBin> {
        DateBin::ALL.into_iter().find(|one| one.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            DateBin::Day => "Day",
            DateBin::Week => "Week",
            DateBin::Month => "Month",
        }
    }
}

/// The categories in their own order (dates in time, names A to Z), or by their figure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Natural,
    Desc,
    Asc,
}

impl SortOrder {
    pub const ALL: [SortOrder; 3] = [SortOrder::Natural, SortOrder::Desc, SortOrder::Asc];

    pub fn id(self) -> &'static str {
        match self {
            SortOrder::Natural => "natural",
            SortOrder::Desc => "desc",
            SortOrder::Asc => "asc",
        }
    }

    pub fn from_id(id: &str) -> Option<SortOrder> {
        SortOrder::ALL.into_iter().find(|one| one.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            SortOrder::Natural => "A to Z",
            SortOrder::Desc => "Largest",
            SortOrder::Asc => "Smallest",
        }
    }
}

/// How many categories a chart keeps, as the builder offers it; 0 keeps them all.
pub const LIMITS: [(&str, &str); 4] = [("0", "All"), ("5", "Top 5"), ("10", "Top 10"), ("20", "Top 20")];

/// What a chart reads from its dataset and how it draws it; fields name columns by id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViewSpec {
    pub view: ViewKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<String>,
    /// The measure; none counts the rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub aggregate: Aggregate,
    pub bin: DateBin,
    pub sort: SortOrder,
    /// How many categories it keeps; 0 or less keeps them all, a fraction its whole part.
    #[serde(serialize_with = "js_number")]
    pub limit: f64,
    pub stack: bool,
    pub horizontal: bool,
    /// A palette's id, or `default` for the one in the settings.
    pub palette: String,
}

/// A number written as JSON writes it in JavaScript: a whole one without its point.
fn js_number<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
    if value.fract() == 0.0 && value.abs() < 9_007_199_254_740_992.0 {
        serializer.serialize_i64(*value as i64)
    } else {
        serializer.serialize_f64(*value)
    }
}

/// A spec's column fields, as the builder's slots name them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    X,
    Y,
    Series,
    Size,
    Color,
}

impl Field {
    pub fn id(self) -> &'static str {
        match self {
            Field::X => "x",
            Field::Y => "y",
            Field::Series => "series",
            Field::Size => "size",
            Field::Color => "color",
        }
    }
}

impl ViewSpec {
    /// The spec as the seed starts every chart: bars, summed, by month, in their own order.
    pub fn base() -> Self {
        ViewSpec {
            view: ViewKind::Bar,
            x: None,
            y: None,
            series: None,
            size: None,
            color: None,
            aggregate: Aggregate::Sum,
            bin: DateBin::Month,
            sort: SortOrder::Natural,
            limit: 0.0,
            stack: false,
            horizontal: false,
            palette: "default".to_owned(),
        }
    }

    pub fn field(&self, field: Field) -> Option<&str> {
        match field {
            Field::X => self.x.as_deref(),
            Field::Y => self.y.as_deref(),
            Field::Series => self.series.as_deref(),
            Field::Size => self.size.as_deref(),
            Field::Color => self.color.as_deref(),
        }
    }

    pub fn set_field(&mut self, field: Field, value: Option<String>) {
        match field {
            Field::X => self.x = value,
            Field::Y => self.y = value,
            Field::Series => self.series = value,
            Field::Size => self.size = value,
            Field::Color => self.color = value,
        }
    }

    /// The column ids it names, as `Object.values(spec)` holds them among its other values.
    pub fn names_column(&self, id: &str) -> bool {
        [&self.x, &self.y, &self.series, &self.size, &self.color]
            .into_iter()
            .any(|field| field.as_deref() == Some(id))
            || self.palette == id
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedChart {
    pub id: String,
    pub name: String,
    pub dataset_id: String,
    pub spec: ViewSpec,
    pub updated_at: String,
}

/// How much of a board's width a chart takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PanelSpan {
    Third,
    Half,
    TwoThirds,
    Full,
}

impl PanelSpan {
    pub const ALL: [PanelSpan; 4] = [PanelSpan::Third, PanelSpan::Half, PanelSpan::TwoThirds, PanelSpan::Full];

    pub fn id(self) -> &'static str {
        match self {
            PanelSpan::Third => "third",
            PanelSpan::Half => "half",
            PanelSpan::TwoThirds => "two-thirds",
            PanelSpan::Full => "full",
        }
    }

    pub fn from_id(id: &str) -> Option<PanelSpan> {
        PanelSpan::ALL.into_iter().find(|one| one.id() == id)
    }

    /// Its label in the width pickers, as the web's `SPANS` says it.
    pub fn label(self) -> &'static str {
        match self {
            PanelSpan::Third => "Third",
            PanelSpan::Half => "Half",
            PanelSpan::TwoThirds => "Two thirds",
            PanelSpan::Full => "Full",
        }
    }
}

/// A chart placed on a board, as wide as its span.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardItem {
    pub id: String,
    pub chart_id: String,
    pub span: PanelSpan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub id: String,
    pub name: String,
    pub description: String,
    pub items: Vec<BoardItem>,
    /// Filters over every chart on the board whose dataset has the column, by the column's name.
    #[serde(with = "filters_json")]
    pub filters: Vec<Filter>,
    pub updated_at: String,
}

/// A filter's operator as the web names it.
pub fn operator_id(operator: FilterOperator) -> &'static str {
    match operator {
        FilterOperator::Is => "is",
        FilterOperator::IsNot => "isNot",
        FilterOperator::Contains => "contains",
        FilterOperator::OneOf => "oneOf",
        FilterOperator::NoneOf => "noneOf",
        FilterOperator::Between => "between",
        FilterOperator::AtLeast => "atLeast",
        FilterOperator::AtMost => "atMost",
        FilterOperator::After => "after",
        FilterOperator::Before => "before",
    }
}

pub fn operator_from_id(id: &str) -> Option<FilterOperator> {
    use FilterOperator::*;
    [
        Is, IsNot, Contains, OneOf, NoneOf, Between, AtLeast, AtMost, After, Before,
    ]
    .into_iter()
    .find(|one| operator_id(*one) == id)
}

/// Filters written as the web keeps them: `{ field, operator, values }`, the operator by its name.
pub mod filters_json {
    use super::*;

    #[derive(Serialize, Deserialize)]
    struct Kept {
        field: String,
        operator: String,
        values: Vec<String>,
    }

    pub fn serialize<S: Serializer>(filters: &[Filter], serializer: S) -> Result<S::Ok, S::Error> {
        let kept: Vec<Kept> = filters
            .iter()
            .map(|filter| Kept {
                field: filter.field.clone(),
                operator: operator_id(filter.operator).to_owned(),
                values: filter.values.clone(),
            })
            .collect();
        kept.serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Filter>, D::Error> {
        let kept = Vec::<Kept>::deserialize(deserializer)?;
        kept.into_iter()
            .map(|one| {
                let operator = operator_from_id(&one.operator)
                    .ok_or_else(|| D::Error::custom(format!("no operator {}", one.operator)))?;
                Ok(Filter::new(one.field, operator, one.values))
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NumberStyle {
    Grouped,
    Compact,
    Plain,
}

impl NumberStyle {
    pub const ALL: [NumberStyle; 3] = [NumberStyle::Grouped, NumberStyle::Compact, NumberStyle::Plain];

    pub fn id(self) -> &'static str {
        match self {
            NumberStyle::Grouped => "grouped",
            NumberStyle::Compact => "compact",
            NumberStyle::Plain => "plain",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            NumberStyle::Grouped => "Grouped",
            NumberStyle::Compact => "Compact",
            NumberStyle::Plain => "Plain",
        }
    }
}

/// Decimals in figures: `Auto` writes up to two.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Decimals {
    Auto,
    Fixed(u8),
}

impl Decimals {
    pub const ALL: [Decimals; 4] = [
        Decimals::Auto,
        Decimals::Fixed(0),
        Decimals::Fixed(1),
        Decimals::Fixed(2),
    ];

    pub fn id(self) -> String {
        match self {
            Decimals::Auto => "auto".to_owned(),
            Decimals::Fixed(places) => places.to_string(),
        }
    }

    pub fn from_id(id: &str) -> Option<Decimals> {
        match id {
            "auto" => Some(Decimals::Auto),
            "0" | "1" | "2" => id.parse().ok().map(Decimals::Fixed),
            _ => None,
        }
    }

    pub fn label(self) -> String {
        match self {
            Decimals::Auto => "Auto".to_owned(),
            Decimals::Fixed(places) => places.to_string(),
        }
    }
}

impl Serialize for Decimals {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.id())
    }
}

impl<'de> Deserialize<'de> for Decimals {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let id = String::deserialize(deserializer)?;
        Decimals::from_id(&id).ok_or_else(|| D::Error::custom(format!("no decimals {id}")))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub id: String,
    /// The palette a chart takes when it names none of its own.
    pub palette: String,
    pub numbers: NumberStyle,
    /// Decimals in figures: `auto` writes up to two.
    pub decimals: Decimals,
    /// How a new chart combines its figures at first.
    pub aggregate: Aggregate,
    /// Whether the builder suggests views for the columns picked.
    pub suggestions: bool,
    /// The boards in the order the person dragged them into, by id; none for the order they were
    /// made in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board_order: Option<Vec<String>>,
}

impl Default for Preferences {
    fn default() -> Self {
        Preferences {
            id: "prefs".to_owned(),
            palette: "theme".to_owned(),
            numbers: NumberStyle::Grouped,
            decimals: Decimals::Auto,
            aggregate: Aggregate::Sum,
            suggestions: true,
            board_order: None,
        }
    }
}

/// A palette of color tokens; the theme's own order has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Palette {
    pub id: &'static str,
    pub label: &'static str,
    pub tokens: Option<&'static [&'static str]>,
}

pub const PALETTES: [Palette; 4] = [
    Palette {
        id: "theme",
        label: "Theme",
        tokens: None,
    },
    Palette {
        id: "cool",
        label: "Cool",
        tokens: Some(&["kind-teal", "kind-blue", "kind-violet", "kind-jade", "chart-2"]),
    },
    Palette {
        id: "warm",
        label: "Warm",
        tokens: Some(&["kind-pink", "chart-5", "chart-4", "chart-3", "kind-violet"]),
    },
    Palette {
        id: "earth",
        label: "Earth",
        tokens: Some(&["kind-green", "kind-jade", "chart-4", "kind-teal", "chart-3"]),
    },
];

/// The tokens the theme gives series in turn, for a swatch of its palette.
pub const THEME_TOKENS: [&str; 5] = ["chart-2", "chart-3", "chart-4", "kind-teal", "kind-pink"];

/// A palette's tokens: the chart's own, or the settings' where it says `default`; none for the
/// theme's order.
pub fn palette_of(id: &str, prefs: &Preferences) -> Option<&'static [&'static str]> {
    let chosen = if id == "default" { prefs.palette.as_str() } else { id };
    PALETTES
        .iter()
        .find(|palette| palette.id == chosen)
        .and_then(|palette| palette.tokens)
}

/// The boards in the person's order: any made since they last ordered them first, the newest
/// first, then the rest as they placed them.
pub fn order_boards(boards: &[Board], order: Option<&[String]>) -> Vec<Board> {
    let mut placed: Vec<usize> = Vec::new();
    for id in order.unwrap_or_default() {
        for (at, board) in boards.iter().enumerate() {
            if &board.id == id {
                placed.push(at);
            }
        }
    }
    let mut out: Vec<Board> = boards
        .iter()
        .enumerate()
        .filter(|(at, _)| !placed.contains(at))
        .map(|(_, board)| board.clone())
        .collect();
    out.extend(placed.iter().map(|at| boards[*at].clone()));
    out
}

/// A fresh id with a prefix, unique in this session: `prefix-<now in base 36>-<count>`, as the
/// web's `freshId` makes them with its own count.
pub fn fresh_id(prefix: &str, now: i64, made: &mut u64) -> String {
    *made += 1;
    format!("{prefix}-{}-{made}", js::base36(now))
}

/// A moment as `toISOString` writes it.
pub fn iso_time(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    let of_day = ms.rem_euclid(86_400_000);
    let (year, month, day) = js::civil_from_days(days);
    let (hour, minute, second, milli) = (
        of_day / 3_600_000,
        of_day / 60_000 % 60,
        of_day / 1000 % 60,
        of_day % 1000,
    );
    let year = if (0..=9999).contains(&year) {
        format!("{year:04}")
    } else {
        format!("{}{:06}", if year < 0 { '-' } else { '+' }, year.abs())
    };
    format!("{year}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milli:03}Z")
}

/// A count with its noun: `1 row`, `12 rows`, the count grouped as `en-US` writes it.
pub fn counted(count: usize, noun: &str) -> String {
    counted_as(count, noun, &format!("{noun}s"))
}

/// A count with its noun and the noun's own plural: `2 categories`.
pub fn counted_as(count: usize, noun: &str, plural: &str) -> String {
    format!("{} {}", js::locale_count(count), if count == 1 { noun } else { plural })
}
