//! Filters on a listing and the views it is saved as, after sokkel-ui's `lib/filters`,
//! `lib/list-views`, `FilterBar` and `SavedViews`: what a filter holds and how rows are matched
//! against it, the bar of chips that sets them, and the menu of the listing's saved setups.

use egui::{
    Id, Popup, PopupCloseBehavior, RectAlign, Sense, Stroke, StrokeKind, TextWrapMode, Ui, WidgetText, pos2, vec2,
};

use super::{Button, ButtonSize, ButtonVariant, Checkbox, Input, Segmented, popup_frame};
use crate::{Icon, TextRole, icons, tokens};

/// What a field holds, which decides the operators it offers and the editor it opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterType {
    Text,
    Set,
    Number,
    Date,
}

/// How a filter compares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum FilterOperator {
    Is,
    IsNot,
    Contains,
    OneOf,
    NoneOf,
    Between,
    AtLeast,
    AtMost,
    After,
    Before,
}

impl FilterType {
    /// The operators the type offers, the first the one a new filter starts with.
    pub fn operators(self) -> &'static [FilterOperator] {
        use FilterOperator::*;
        match self {
            FilterType::Text => &[Contains, Is, IsNot],
            FilterType::Set => &[OneOf, NoneOf],
            FilterType::Number => &[Between, AtLeast, AtMost],
            FilterType::Date => &[Between, After, Before],
        }
    }
}

/// One filter: the field it is on, how it compares, and what with, as strings: a number in plain
/// decimals, a day as `YYYY-MM-DD`; `Between` holds its two ends, either empty for an open end.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Filter {
    pub field: String,
    pub operator: FilterOperator,
    pub values: Vec<String>,
}

impl Filter {
    pub fn new(
        field: impl Into<String>,
        operator: FilterOperator,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            field: field.into(),
            operator,
            values: values.into_iter().map(Into::into).collect(),
        }
    }

    /// Whether it says enough to filter by: a value, one end of a range, or a chosen value.
    pub fn complete(&self) -> bool {
        let first = self.values.first().map_or("", String::as_str);
        let second = self.values.get(1).map_or("", String::as_str);
        match self.operator {
            FilterOperator::OneOf | FilterOperator::NoneOf => self.values.iter().any(|v| !v.is_empty()),
            FilterOperator::Between => !first.trim().is_empty() || !second.trim().is_empty(),
            _ => !first.trim().is_empty(),
        }
    }

    /// The filter moved to another operator, keeping what still fits.
    pub fn with_operator(&self, operator: FilterOperator) -> Self {
        let range = |one: FilterOperator| one == FilterOperator::Between;
        let mut next = self.clone();
        if operator == self.operator {
            return next;
        }
        next.operator = operator;
        if range(operator) && !range(self.operator) {
            next.values = vec![self.values.first().cloned().unwrap_or_default(), String::new()];
        } else if !range(operator) && range(self.operator) {
            next.values = self
                .values
                .iter()
                .find(|v| !v.trim().is_empty())
                .cloned()
                .into_iter()
                .collect();
        }
        next
    }
}

/// A field's value in a row, as the matching reads it.
#[derive(Clone, Debug, PartialEq)]
pub enum FilterValue {
    Text(String),
    Number(f64),
    /// A day as `YYYY-MM-DD`.
    Day(String),
    Set(Vec<String>),
    None,
}

/// Whether a row's value passes one filter, as the web's `matchesFilter`: text without case, a set
/// by any chosen value, a number or a day within a range with its ends, `After` and `Before`
/// leaving the day itself out. A filter that does not say enough passes everything.
pub fn matches_filter(value: &FilterValue, filter: &Filter, kind: FilterType) -> bool {
    if !filter.complete() {
        return true;
    }
    let first = filter.values.first().map_or("", String::as_str).trim();
    let second = filter.values.get(1).map_or("", String::as_str).trim();
    use FilterOperator::*;
    if matches!(kind, FilterType::Text | FilterType::Set) {
        let fold = |s: &str| s.trim().to_lowercase();
        let texts: Vec<String> = match value {
            FilterValue::Text(t) | FilterValue::Day(t) => vec![fold(t)],
            FilterValue::Number(n) => vec![n.to_string()],
            FilterValue::Set(values) => values.iter().map(|v| fold(v)).collect(),
            FilterValue::None => Vec::new(),
        };
        let wanted: Vec<String> = filter
            .values
            .iter()
            .filter(|v| !v.is_empty())
            .map(|v| fold(v))
            .collect();
        let first = wanted.first().cloned().unwrap_or_default();
        return match filter.operator {
            Is => texts.contains(&first),
            IsNot => !texts.contains(&first),
            Contains => texts.iter().any(|t| t.contains(&first)),
            OneOf => texts.iter().any(|t| wanted.contains(t)),
            NoneOf => !texts.iter().any(|t| wanted.contains(t)),
            _ => true,
        };
    }
    // a number compared as one, a day as `YYYY-MM-DD`, which sorts as text
    let order = |a: &str, b: &str| -> Option<std::cmp::Ordering> {
        if kind == FilterType::Date {
            Some(
                a.chars()
                    .take(10)
                    .collect::<String>()
                    .cmp(&b.chars().take(10).collect::<String>()),
            )
        } else {
            a.parse::<f64>().ok()?.partial_cmp(&b.parse::<f64>().ok()?)
        }
    };
    let own = match value {
        FilterValue::Number(n) => n.to_string(),
        FilterValue::Day(d) | FilterValue::Text(d) => d.clone(),
        _ => return false,
    };
    // a value a range cannot read fails it
    if order(&own, &own).is_none() {
        return false;
    }
    let against =
        |end: &str, keep: fn(std::cmp::Ordering) -> bool| end.is_empty() || order(&own, end).is_some_and(keep);
    use std::cmp::Ordering::{Equal, Greater, Less};
    match filter.operator {
        Between => against(first, |o| o != Less) && against(second, |o| o != Greater),
        AtLeast => against(first, |o| o != Less),
        AtMost => against(first, |o| o != Greater),
        After => against(first, |o| o == Greater),
        Before => against(first, |o| o == Less),
        Is => against(first, |o| o == Equal),
        IsNot => against(first, |o| o != Equal),
        _ => true,
    }
}

/// Says one of a field's values on its chip.
pub type SaidValue<'a> = Box<dyn Fn(&str) -> String + 'a>;

/// One field a listing filters on: what it is called, what it holds, and how a row gives it.
pub struct FilterField<'a, T> {
    pub id: String,
    pub label: String,
    pub kind: FilterType,
    /// A set's values and their labels, which its editor lists to tick.
    pub options: Vec<(String, String)>,
    /// The operators offered, the first the one a new filter starts with; the type's own when empty.
    pub operators: Vec<FilterOperator>,
    pub icon: Option<Icon>,
    /// Says one of the field's values on the chip, such as a number in its own format.
    pub format: Option<SaidValue<'a>>,
    /// The field's value in a row.
    pub value: Box<dyn Fn(&T) -> FilterValue + 'a>,
}

impl<'a, T> FilterField<'a, T> {
    pub fn new(id: &str, label: &str, kind: FilterType, value: impl Fn(&T) -> FilterValue + 'a) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind,
            options: Vec::new(),
            operators: Vec::new(),
            icon: None,
            format: None,
            value: Box::new(value),
        }
    }

    pub fn options(mut self, options: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        self.options = options.into_iter().map(|(v, l)| (v.into(), l.into())).collect();
        self
    }

    pub fn operators(mut self, operators: impl IntoIterator<Item = FilterOperator>) -> Self {
        self.operators = operators.into_iter().collect();
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn format(mut self, format: impl Fn(&str) -> String + 'a) -> Self {
        self.format = Some(Box::new(format));
        self
    }

    fn offered(&self) -> Vec<FilterOperator> {
        if self.operators.is_empty() {
            self.kind.operators().to_vec()
        } else {
            self.operators.clone()
        }
    }
}

/// The rows that pass every filter, in their order, as the web's `filterRows`.
pub fn filter_rows<'r, T>(
    rows: impl IntoIterator<Item = &'r T>,
    filters: &[Filter],
    fields: &[FilterField<'_, T>],
) -> Vec<&'r T>
where
    T: 'r,
{
    let rules: Vec<(&Filter, &FilterField<'_, T>)> = filters
        .iter()
        .filter(|f| f.complete())
        .filter_map(|f| fields.iter().find(|field| field.id == f.field).map(|field| (f, field)))
        .collect();
    rows.into_iter()
        .filter(|row| {
            rules
                .iter()
                .all(|(f, field)| matches_filter(&(field.value)(row), f, field.kind))
        })
        .collect()
}

/// Whether two lists of filters filter alike: the same complete filters, in any order.
pub fn same_filters(a: &[Filter], b: &[Filter]) -> bool {
    let keys = |filters: &[Filter]| {
        let mut keys: Vec<String> = filters
            .iter()
            .filter(|f| f.complete())
            .map(|f| {
                let mut values: Vec<String> = f.values.iter().map(|v| v.trim().to_owned()).collect();
                if matches!(f.operator, FilterOperator::OneOf | FilterOperator::NoneOf) {
                    values.retain(|v| !v.is_empty());
                    values.sort();
                }
                format!("{}|{:?}|{}", f.field, f.operator, values.join("\u{1}"))
            })
            .collect();
        keys.sort();
        keys
    };
    keys(a) == keys(b)
}

/// How a listing is set up, as sokkel-ui's `ListViewState`: its filters, its sort (a column's id
/// and whether it rises) and its hidden columns' ids.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ListViewState {
    pub filters: Vec<Filter>,
    pub sort: Option<(String, bool)>,
    pub hidden: Vec<String>,
}

impl ListViewState {
    /// Whether the listing now differs from this saved setup, as the web's `viewChanges` finds any.
    pub fn differs(&self, now: &ListViewState) -> bool {
        let hidden = |s: &ListViewState| {
            let mut h = s.hidden.clone();
            h.sort();
            h.dedup();
            h
        };
        !same_filters(&self.filters, &now.filters) || self.sort != now.sort || hidden(self) != hidden(now)
    }
}

/// The words a filter bar says.
#[derive(Clone, Debug)]
pub struct FilterBarWords {
    pub add: String,
    pub add_more: String,
    pub fields: String,
    pub no_field: String,
    pub clear: String,
    pub search: String,
    pub empty: String,
    pub from: String,
    pub to: String,
    pub value: String,
    pub since: String,
    pub until: String,
}

impl Default for FilterBarWords {
    fn default() -> Self {
        Self {
            add: "Filter".into(),
            add_more: "Add filter".into(),
            fields: "Filter by".into(),
            no_field: "No field matches.".into(),
            clear: "Clear all".into(),
            search: "Search".into(),
            empty: "Nothing matches.".into(),
            from: "From".into(),
            to: "To".into(),
            value: "Value".into(),
            since: "from".into(),
            until: "until".into(),
        }
    }
}

/// How an operator is said, in the editor and on the chip.
pub fn operator_word(operator: FilterOperator) -> &'static str {
    use FilterOperator::*;
    match operator {
        Is => "is",
        IsNot => "is not",
        Contains => "contains",
        OneOf => "is any of",
        NoneOf => "is none of",
        Between => "between",
        AtLeast => "at least",
        AtMost => "at most",
        After => "after",
        Before => "before",
    }
}

/// A day (`YYYY-MM-DD`) said short: `Nov 2`.
fn short_day(value: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let parts: Vec<i32> = value.split('-').filter_map(|p| p.parse().ok()).collect();
    match parts.as_slice() {
        [_, m, d] if (1..=12).contains(m) => format!("{} {d}", MONTHS[(*m - 1) as usize]),
        _ => value.to_owned(),
    }
}

/// How a filter is said on its chip, as the web's `saidFilter`: the operator, then its value.
fn said<T>(filter: &Filter, field: &FilterField<'_, T>, words: &FilterBarWords) -> (String, String) {
    let one = |value: &str| match &field.format {
        Some(format) => format(value),
        None => match field.kind {
            FilterType::Date => short_day(value),
            FilterType::Number => value
                .parse::<f64>()
                .map_or_else(|_| value.to_owned(), |n| super::format_value(n, None)),
            _ => value.to_owned(),
        },
    };
    let chosen: Vec<&String> = filter.values.iter().filter(|v| !v.is_empty()).collect();
    use FilterOperator::*;
    match filter.operator {
        OneOf | NoneOf => {
            let labels: Vec<String> = field
                .options
                .iter()
                .filter(|(v, _)| chosen.contains(&v))
                .map(|(_, l)| l.clone())
                .collect();
            let value = match labels.len() {
                0 => String::new(),
                1 | 2 => labels.join(", "),
                n => format!("{} +{}", labels[0], n - 1),
            };
            let single = chosen.len() == 1;
            let operator = match (filter.operator, single) {
                (OneOf, true) => Is,
                (OneOf, false) => OneOf,
                (_, true) => IsNot,
                _ => NoneOf,
            };
            (operator_word(operator).into(), value)
        }
        Between => {
            let from = filter.values.first().map_or("", |v| v.trim());
            let to = filter.values.get(1).map_or("", |v| v.trim());
            let date = field.kind == FilterType::Date;
            if !from.is_empty() && !to.is_empty() {
                (String::new(), format!("{} – {}", one(from), one(to)))
            } else if !from.is_empty() {
                (
                    if date {
                        words.since.clone()
                    } else {
                        operator_word(AtLeast).into()
                    },
                    one(from),
                )
            } else {
                (
                    if date {
                        words.until.clone()
                    } else {
                        operator_word(AtMost).into()
                    },
                    one(to),
                )
            }
        }
        _ => (
            operator_word(filter.operator).into(),
            chosen.first().map_or(String::new(), |v| one(v)),
        ),
    }
}

/// Filters on a listing, as sokkel-ui's `FilterBar`: each a chip that says what it keeps (the
/// field, how it compares and the value) with a button that takes it away. Filter adds one from a
/// menu of the fields; choosing a field adds its filter and opens its editor at once. The editor
/// offers the field's operators over its value, which applies as it is changed; closing it with
/// nothing filled in takes its filter away. Clear all removes every filter.
pub struct FilterBar<'a, 'f, T> {
    id: Id,
    fields: &'a [FilterField<'f, T>],
    filters: &'a mut Vec<Filter>,
    words: FilterBarWords,
}

impl<'a, 'f, T> FilterBar<'a, 'f, T> {
    pub fn new(id: impl egui::AsIdSalt, fields: &'a [FilterField<'f, T>], filters: &'a mut Vec<Filter>) -> Self {
        Self {
            id: Id::unique(id),
            fields,
            filters,
            words: FilterBarWords::default(),
        }
    }

    pub fn words(mut self, words: FilterBarWords) -> Self {
        self.words = words;
        self
    }

    /// Draws the bar; true when the filters changed.
    pub fn show(self, ui: &mut Ui) -> bool {
        let t = tokens(ui.ctx());
        let before = self.filters.clone();
        let editing_id = self.id.with("editing");
        let mut editing: Option<String> = ui.data(|d| d.get_temp(editing_id)).flatten();
        let control = t.density("control");
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            let mut remove = None;
            for index in 0..self.filters.len() {
                let Some(field) = self.fields.iter().find(|f| f.id == self.filters[index].field) else {
                    continue;
                };
                let filter = self.filters[index].clone();
                let (operator, value) = if filter.complete() {
                    said(&filter, field, &self.words)
                } else {
                    (String::new(), String::new())
                };
                let set = !value.is_empty();
                // the chip's face: its icon, the field, the operator and the value, 10 in, 6 apart
                let font = TextRole::Body.font(&t);
                let galley = |ui: &Ui, text: &str, weight: f32| {
                    WidgetText::from(TextRole::Body.rich(&t, text).variation("wght", weight)).into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        192.0,
                        font.clone(),
                    )
                };
                let facet = galley(ui, &field.label, 400.0);
                let said_operator = (set && !operator.is_empty()).then(|| galley(ui, &operator, 400.0));
                let said_value = set.then(|| galley(ui, &value, 500.0));
                let icon_room = if field.icon.is_some() { 16.0 + 6.0 } else { 0.0 };
                let mut face_width = 10.0 + icon_room + facet.size().x + 10.0;
                if let Some(g) = &said_operator {
                    face_width += 6.0 + g.size().x;
                }
                if let Some(g) = &said_value {
                    face_width += 6.0 + g.size().x;
                }
                if !set {
                    face_width += 6.0 + 14.0;
                }
                let clear_width = if set { 1.0 + 6.0 + 14.0 + 6.0 } else { 0.0 };
                let (chip, _) = ui.allocate_exact_size(vec2(face_width + clear_width, control), Sense::hover());
                let face_rect = egui::Rect::from_min_size(chip.min, vec2(face_width, control));
                let face = ui.interact(face_rect, self.id.with(("chip", &field.id)), Sense::click());
                let radius = t.radius("control");
                let painter = ui.painter().clone();
                let (fill, edge) = if set {
                    (t.color("accent"), t.color_at("primary", 0.4))
                } else {
                    (t.color("field"), t.color("input"))
                };
                painter.rect_filled(chip, radius, fill);
                if face.hovered() {
                    painter.rect_filled(face_rect, radius, t.color("hover"));
                }
                let mut x = chip.left() + 10.0;
                let muted = t.color("muted-foreground");
                if let Some(icon) = field.icon {
                    icons::paint(&painter, pos2(x + 8.0, chip.center().y), icon, 16.0, muted);
                    x += icon_room;
                }
                let place = |g: std::sync::Arc<egui::Galley>, x: f32, ink| {
                    let w = g.size().x;
                    painter.galley(pos2(x, chip.center().y - g.size().y / 2.0), g, ink);
                    x + w
                };
                x = place(facet, x, if set { muted } else { t.color("foreground") });
                if let Some(g) = said_operator {
                    x = place(g, x + 6.0, muted);
                }
                if let Some(g) = said_value {
                    place(g, x + 6.0, t.color("foreground"));
                } else {
                    icons::paint(
                        &painter,
                        pos2(x + 6.0 + 7.0, chip.center().y),
                        Icon::ChevronDown,
                        14.0,
                        muted,
                    );
                }
                if set {
                    let clear_rect = egui::Rect::from_min_max(pos2(face_rect.right(), chip.top()), chip.max);
                    let clear = ui.interact(clear_rect, self.id.with(("clear", &field.id)), Sense::click());
                    if clear.hovered() {
                        painter.rect_filled(clear_rect.shrink(1.0), 0.0, t.color("hover"));
                    }
                    painter.vline(
                        clear_rect.left() + 0.5,
                        chip.y_range(),
                        Stroke::new(1.0, t.color_at("primary", 0.3)),
                    );
                    icons::paint(
                        &painter,
                        clear_rect.center(),
                        Icon::X,
                        14.0,
                        if clear.hovered() { t.color("foreground") } else { muted },
                    );
                    if clear.clicked() {
                        remove = Some(index);
                    }
                }
                painter.rect_stroke(chip, radius, Stroke::new(1.0, edge), StrokeKind::Inside);
                // the editor: the field's operators over its value
                let popup_id = self.id.with(("editor", &field.id));
                if editing.as_deref() == Some(field.id.as_str()) && !Popup::is_id_open(ui.ctx(), popup_id) {
                    if ui.data(|d| d.get_temp::<bool>(popup_id.with("shown"))).unwrap_or(false) {
                        // it closed: an editor left empty takes its filter away
                        editing = None;
                        if !self.filters[index].complete() {
                            remove = Some(index);
                        }
                        ui.data_mut(|d| d.insert_temp(popup_id.with("shown"), false));
                    } else {
                        Popup::open_id(ui.ctx(), popup_id);
                    }
                }
                if face.clicked() {
                    editing = Some(field.id.clone());
                }
                let shown = Popup::menu(&face)
                    .id(popup_id)
                    .align(RectAlign::BOTTOM_START)
                    .frame(popup_frame(&t).inner_margin(egui::Margin::ZERO))
                    .gap(4.0)
                    .width(320.0)
                    .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                    .show(|ui| {
                        ui.set_width(320.0);
                        editor(ui, field, &mut self.filters[index], &self.words, popup_id);
                    });
                if shown.is_some() {
                    ui.data_mut(|d| d.insert_temp(popup_id.with("shown"), true));
                }
            }
            if let Some(index) = remove {
                self.filters.remove(index);
            }
            // Filter, or the icon that adds another once some are set
            let empty = self.filters.is_empty();
            let add = if empty {
                ui.add(
                    Button::new(&self.words.add)
                        .icon(Icon::ListFilter)
                        .variant(ButtonVariant::Ghost)
                        .muted(true),
                )
            } else {
                ui.add(super::IconButton::new(Icon::Plus, &self.words.add_more))
            };
            let mut picked = None;
            Popup::menu(&add)
                .id(self.id.with("fields"))
                .align(RectAlign::BOTTOM_START)
                .frame(popup_frame(&t).inner_margin(egui::Margin::same(4)))
                .gap(4.0)
                .width(256.0)
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    ui.set_width(248.0);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for field in self.fields {
                        let (row, response) = ui.allocate_exact_size(vec2(248.0, 32.0), Sense::click());
                        if response.hovered() {
                            ui.painter().rect_filled(row, t.radius("control"), t.color("hover"));
                        }
                        let mut x = row.left() + 8.0;
                        if let Some(icon) = field.icon {
                            icons::paint(
                                ui.painter(),
                                pos2(x + 8.0, row.center().y),
                                icon,
                                16.0,
                                t.color("muted-foreground"),
                            );
                            x += 24.0;
                        }
                        ui.painter().text(
                            pos2(x, row.center().y),
                            egui::Align2::LEFT_CENTER,
                            &field.label,
                            TextRole::Body.font(&t),
                            t.color("popover-foreground"),
                        );
                        if response.clicked() {
                            picked = Some(field);
                        }
                    }
                });
            if let Some(field) = picked {
                Popup::close_id(ui.ctx(), self.id.with("fields"));
                if !self.filters.iter().any(|f| f.field == field.id) {
                    let operator = field.offered()[0];
                    self.filters
                        .push(Filter::new(field.id.clone(), operator, Vec::<String>::new()));
                }
                editing = Some(field.id.clone());
            }
            if !empty
                && ui
                    .add(Button::new(&self.words.clear).variant(ButtonVariant::Ghost).muted(true))
                    .clicked()
            {
                self.filters.clear();
                editing = None;
            }
        });
        ui.data_mut(|d| d.insert_temp(editing_id, editing));
        *self.filters != before
    }
}

/// A filter's editor: its operators over its value, which applies as it is changed.
fn editor<T>(ui: &mut Ui, field: &FilterField<'_, T>, filter: &mut Filter, words: &FilterBarWords, id: Id) {
    let t = tokens(ui.ctx());
    let operators = field.offered();
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    if operators.len() > 1 {
        let mut at = operators.iter().position(|o| *o == filter.operator).unwrap_or(0);
        egui::Frame::new().inner_margin(8).show(ui, |ui| {
            let labels: Vec<&str> = operators.iter().map(|o| operator_word(*o)).collect();
            if ui.add(Segmented::new(&mut at, labels).fill(true)).changed() {
                *filter = filter.with_operator(operators[at]);
            }
        });
        let y = ui.cursor().top();
        ui.painter()
            .hline(ui.max_rect().x_range(), y, Stroke::new(1.0, t.color("border")));
    }
    egui::Frame::new().inner_margin(8).show(ui, |ui| {
        ui.set_width(304.0);
        match field.kind {
            FilterType::Set => {
                ui.spacing_mut().item_spacing.y = 0.0;
                for (value, label) in &field.options {
                    let mut on = filter.values.contains(value);
                    let (row, _) = ui.allocate_exact_size(vec2(304.0, 32.0), Sense::hover());
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(row.shrink2(vec2(6.0, 0.0)))
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    );
                    if child.add(Checkbox::new(&mut on, label)).changed() {
                        if on {
                            filter.values.push(value.clone());
                        } else {
                            filter.values.retain(|v| v != value);
                        }
                    }
                }
                if field.options.is_empty() {
                    ui.label(TextRole::Body.rich(&t, &words.empty).color(t.color("muted-foreground")));
                }
            }
            _ if filter.operator == FilterOperator::Between => {
                while filter.values.len() < 2 {
                    filter.values.push(String::new());
                }
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let (from, to) = filter.values.split_at_mut(1);
                    ui.push_id(id.with("from"), |ui| {
                        ui.add(Input::new(&mut from[0]).placeholder(&words.from).width(136.0))
                    });
                    ui.label(TextRole::Body.rich(&t, "–").color(t.color("muted-foreground")));
                    ui.push_id(id.with("to"), |ui| {
                        ui.add(Input::new(&mut to[0]).placeholder(&words.to).width(136.0))
                    });
                });
            }
            _ => {
                if filter.values.is_empty() {
                    filter.values.push(String::new());
                }
                ui.add(Input::new(&mut filter.values[0]).placeholder(&words.value).width(304.0));
            }
        }
    });
}

/// A listing's setup kept under a name, as sokkel-ui's `SavedView`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SavedView {
    pub id: String,
    pub name: String,
    pub state: ListViewState,
    /// The app's own view: picked and saved as a new one, never changed, renamed or deleted.
    pub locked: bool,
}

/// What the saved views were asked this frame.
#[derive(Clone, Debug, PartialEq)]
pub enum SavedViewsAsked {
    /// A view picked, or the chosen one again on Reset: the app sets the listing to its state.
    Picked(String),
    /// The listing now kept in the chosen view.
    Save,
    /// The listing now kept as a new view under this name.
    SaveNew(String),
    Rename(String),
    Delete,
}

/// The words the saved views say.
#[derive(Clone, Debug)]
pub struct SavedViewsWords {
    pub views: String,
    pub none: String,
    pub changed: String,
    pub save: String,
    pub save_new: String,
    pub rename: String,
    pub delete: String,
    pub reset: String,
    pub name: String,
    pub cancel: String,
    pub name_taken: String,
    pub delete_detail: String,
}

impl Default for SavedViewsWords {
    fn default() -> Self {
        Self {
            views: "Views".into(),
            none: "Views".into(),
            changed: "Changed".into(),
            save: "Save".into(),
            save_new: "Save as new view".into(),
            rename: "Rename".into(),
            delete: "Delete".into(),
            reset: "Reset".into(),
            name: "Name".into(),
            cancel: "Cancel".into(),
            name_taken: "A view has this name already.".into(),
            delete_detail: "The listing stays as it is now.".into(),
        }
    }
}

/// The saved views of a listing, as sokkel-ui's `SavedViews`: a menu that names the chosen view,
/// lists the others to pick, and saves the listing now as a new view, renames the chosen one or
/// deletes it. While the listing differs from the chosen view the trigger says so, and Reset and
/// Save stand beside it. The app keeps the views.
pub struct SavedViews<'a> {
    id: Id,
    views: &'a [SavedView],
    chosen: Option<&'a str>,
    current: &'a ListViewState,
    words: SavedViewsWords,
}

impl<'a> SavedViews<'a> {
    pub fn new(
        id: impl egui::AsIdSalt,
        views: &'a [SavedView],
        chosen: Option<&'a str>,
        current: &'a ListViewState,
    ) -> Self {
        Self {
            id: Id::unique(id),
            views,
            chosen,
            current,
            words: SavedViewsWords::default(),
        }
    }

    pub fn words(mut self, words: SavedViewsWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<SavedViewsAsked> {
        let t = tokens(ui.ctx());
        let chosen = self.views.iter().find(|v| Some(v.id.as_str()) == self.chosen);
        let changed = chosen.is_some_and(|v| v.state.differs(self.current));
        let own = chosen.is_some_and(|v| !v.locked);
        let mut asked = None;
        let naming_id = self.id.with("naming");
        let mut naming: Option<(bool, String)> = ui.data(|d| d.get_temp(naming_id)).flatten();
        let deleting_id = self.id.with("deleting");
        let mut deleting = ui.data(|d| d.get_temp::<bool>(deleting_id)).unwrap_or(false);
        let trigger = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let name = chosen.map_or(self.words.none.as_str(), |v| v.name.as_str());
                let mut button = Button::new(name)
                    .icon(Icon::Bookmark)
                    .variant(ButtonVariant::Outline)
                    .trailing(Icon::ChevronDown);
                if changed {
                    button = button.note(&self.words.changed);
                }
                let trigger = ui.add(button);
                if changed {
                    if ui
                        .add(Button::new(&self.words.reset).variant(ButtonVariant::Ghost).muted(true))
                        .clicked()
                        && let Some(view) = chosen
                    {
                        asked = Some(SavedViewsAsked::Picked(view.id.clone()));
                    }
                    let locked = chosen.is_some_and(|v| v.locked);
                    if own || locked {
                        let label = if own { &self.words.save } else { &self.words.save_new };
                        if ui.add(Button::new(label).variant(ButtonVariant::Secondary)).clicked() {
                            if own {
                                asked = Some(SavedViewsAsked::Save);
                            } else {
                                naming = Some((true, String::new()));
                            }
                        }
                    }
                }
                trigger
            })
            .inner;
        let mut items = vec![super::MenuItem::Label(self.words.views.clone())];
        for view in self.views {
            let mut item = super::MenuItem::action(view.name.clone());
            if Some(view.id.as_str()) == self.chosen {
                item = item.icon(Icon::Check);
            }
            items.push(item);
        }
        items.push(super::MenuItem::Separator);
        let mut actions = Vec::new();
        if own {
            items.push(
                super::MenuItem::action(self.words.save.clone())
                    .icon(Icon::Save)
                    .disabled(!changed),
            );
            actions.push(0);
        }
        items.push(super::MenuItem::action(self.words.save_new.clone()).icon(Icon::Plus));
        actions.push(1);
        if own {
            items.push(super::MenuItem::action(self.words.rename.clone()).icon(Icon::Pencil));
            actions.push(2);
            items.push(
                super::MenuItem::action(self.words.delete.clone())
                    .icon(Icon::Trash2)
                    .destructive(),
            );
            actions.push(3);
        }
        if let Some(index) = super::DropdownMenu::new(&items).width(256.0).show(&trigger) {
            let first_action = 1 + self.views.len() + 1;
            if (1..=self.views.len()).contains(&index) {
                asked = Some(SavedViewsAsked::Picked(self.views[index - 1].id.clone()));
            } else if index >= first_action {
                match actions.get(index - first_action) {
                    Some(0) => asked = Some(SavedViewsAsked::Save),
                    Some(1) => naming = Some((true, String::new())),
                    Some(2) => naming = Some((false, chosen.map_or(String::new(), |v| v.name.clone()))),
                    Some(3) => deleting = true,
                    _ => {}
                }
            }
        }
        // a name asked under the trigger, for a new view or a rename
        if let Some((new, mut name)) = naming.take() {
            let trimmed = name.trim().to_lowercase();
            let taken = self
                .views
                .iter()
                .any(|v| v.name.trim().to_lowercase() == trimmed && (new || Some(v.id.as_str()) != self.chosen));
            let mut keep = true;
            egui::Area::new(naming_id.with("area"))
                .order(egui::Order::Foreground)
                .fixed_pos(trigger.rect.left_bottom() + vec2(0.0, 4.0))
                .show(ui.ctx(), |ui| {
                    popup_frame(&t).inner_margin(12).show(ui, |ui| {
                        ui.set_width(256.0);
                        ui.label(TextRole::Label.rich(&t, &self.words.name));
                        ui.add_space(6.0);
                        let field = ui.add(Input::new(&mut name).name(&self.words.name).width(256.0).invalid(taken));
                        if taken {
                            ui.label(
                                TextRole::Help
                                    .rich(&t, &self.words.name_taken)
                                    .color(t.color("critical-ink")),
                            );
                        }
                        ui.add_space(8.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let enter = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let label = if new { &self.words.save } else { &self.words.rename };
                            let ok = !name.trim().is_empty() && !taken;
                            if (ui.add(Button::new(label).size(ButtonSize::Sm).enabled(ok)).clicked() || enter) && ok {
                                asked = Some(if new {
                                    SavedViewsAsked::SaveNew(name.trim().to_owned())
                                } else {
                                    SavedViewsAsked::Rename(name.trim().to_owned())
                                });
                                keep = false;
                            }
                            if ui
                                .add(
                                    Button::new(&self.words.cancel)
                                        .variant(ButtonVariant::Ghost)
                                        .size(ButtonSize::Sm),
                                )
                                .clicked()
                            {
                                keep = false;
                            }
                        });
                    });
                });
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                keep = false;
            }
            if keep {
                naming = Some((new, name));
            }
        }
        if deleting && let Some(view) = chosen {
            match super::ConfirmDialog::new(
                deleting_id.with("dialog"),
                format!("Delete {}?", view.name),
                &self.words.delete,
            )
            .detail(&self.words.delete_detail)
            .destructive(true)
            .show(ui.ctx())
            {
                Some(super::ConfirmAnswer::Confirm) => {
                    asked = Some(SavedViewsAsked::Delete);
                    deleting = false;
                }
                Some(_) => deleting = false,
                None => {}
            }
        }
        ui.data_mut(|d| {
            d.insert_temp(naming_id, naming);
            d.insert_temp(deleting_id, deleting);
        });
        asked
    }
}

/// One facet chosen from its options on its own, after sokkel-ui's `FilterChip` with `options`
/// and `multiple`: a control-tall chip naming the facet, opening a menu of the options to tick; once
/// one is ticked it names them (two, or the first and how many more) on the accent ground with a
/// button that clears them. Answers whether the choice changed.
pub struct FilterChip<'a> {
    id: egui::Id,
    facet: String,
    options: &'a [(String, String)],
    selected: &'a mut Vec<String>,
}

impl<'a> FilterChip<'a> {
    /// `options` are each a value and its label.
    pub fn new(
        id: impl egui::AsIdSalt,
        facet: impl Into<String>,
        options: &'a [(String, String)],
        selected: &'a mut Vec<String>,
    ) -> Self {
        Self {
            id: egui::Id::unique(id),
            facet: facet.into(),
            options,
            selected,
        }
    }

    pub fn show(self, ui: &mut Ui) -> bool {
        let t = tokens(ui.ctx());
        let labels: Vec<&str> = self
            .options
            .iter()
            .filter(|(v, _)| self.selected.contains(v))
            .map(|(_, l)| l.as_str())
            .collect();
        let said = match labels.len() {
            0 => None,
            1 | 2 => Some(labels.join(", ")),
            n => Some(format!("{} +{}", labels[0], n - 1)),
        };
        let lay = |ui: &Ui, rich: egui::RichText| {
            egui::WidgetText::from(rich).into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                TextRole::Body.font(&t),
            )
        };
        let facet = lay(
            ui,
            TextRole::Body.rich(&t, &self.facet).color(if said.is_some() {
                t.color("muted-foreground")
            } else {
                t.color("foreground")
            }),
        );
        let value = said.as_ref().map(|s| {
            lay(
                ui,
                TextRole::Body
                    .rich(&t, s)
                    .variation("wght", 500.0)
                    .color(t.color("foreground")),
            )
        });
        let face_width =
            10.0 + facet.size().x + value.as_ref().map_or(6.0 + 14.0, |v| 6.0 + v.size().x.min(192.0)) + 10.0;
        let clear_width = if said.is_some() { 26.0 } else { 0.0 };
        let height = t.density("control");
        let (rect, _) = ui.allocate_exact_size(egui::vec2(face_width + clear_width, height), egui::Sense::hover());
        let face_rect = egui::Rect::from_min_size(rect.min, egui::vec2(face_width, height));
        let face = ui.interact(face_rect, self.id.with("face"), egui::Sense::click());
        face.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.facet));
        let popup = self.id.with("menu");
        let open = egui::Popup::is_id_open(ui.ctx(), popup);
        let (ground, edge) = if said.is_some() {
            (t.color("accent"), t.color_at("primary", 0.4))
        } else {
            (t.color("field"), t.color("input"))
        };
        let radius = t.radius("control");
        ui.painter().rect(
            rect,
            radius,
            ground,
            egui::Stroke::new(1.0, edge),
            egui::StrokeKind::Inside,
        );
        if face.hovered() || open {
            ui.painter()
                .rect_filled(face_rect.shrink(1.0), radius, t.color("hover"));
        }
        let mut x = rect.left() + 10.0;
        let facet_width = facet.size().x;
        ui.painter().galley(
            pos2(x, rect.center().y - facet.size().y / 2.0),
            facet,
            t.color("foreground"),
        );
        x += facet_width + 6.0;
        match value {
            Some(value) => {
                ui.painter().galley(
                    pos2(x, rect.center().y - value.size().y / 2.0),
                    value,
                    t.color("foreground"),
                );
            }
            None => icons::paint(
                ui.painter(),
                pos2(x + 7.0, rect.center().y),
                Icon::ChevronDown,
                14.0,
                t.color("muted-foreground"),
            ),
        }
        let mut changed = false;
        if said.is_some() {
            let clear_rect = egui::Rect::from_min_max(pos2(face_rect.right(), rect.top()), rect.max);
            ui.painter().vline(
                clear_rect.left(),
                rect.y_range(),
                egui::Stroke::new(1.0, t.color_at("primary", 0.3)),
            );
            let clear = ui.interact(clear_rect, self.id.with("clear"), egui::Sense::click());
            let words = format!("Clear {}", self.facet);
            clear.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &words));
            if clear.hovered() {
                ui.painter()
                    .rect_filled(clear_rect.shrink(1.0), radius, t.color("hover"));
            }
            icons::paint(
                ui.painter(),
                clear_rect.center(),
                Icon::X,
                14.0,
                t.color("muted-foreground"),
            );
            if clear.clicked() {
                self.selected.clear();
                changed = true;
            }
        }
        egui::Popup::menu(&face)
            .id(popup)
            .frame(super::popup_frame(&t).inner_margin(4.0))
            .gap(4.0)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_min_width(rect.width().max(160.0));
                ui.spacing_mut().item_spacing.y = 0.0;
                for (value, label) in self.options {
                    let mut on = self.selected.contains(value);
                    let (row, hit) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width().max(200.0), t.density("control")),
                        egui::Sense::click(),
                    );
                    hit.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, on, label));
                    if hit.hovered() {
                        ui.painter().rect_filled(row, t.radius("md"), t.color("accent"));
                    }
                    if on {
                        icons::paint(
                            ui.painter(),
                            pos2(row.left() + 8.0 + 7.0, row.center().y),
                            Icon::Check,
                            14.0,
                            t.color("popover-foreground"),
                        );
                    }
                    ui.painter().text(
                        pos2(row.left() + 8.0 + 14.0 + 8.0, row.center().y),
                        egui::Align2::LEFT_CENTER,
                        label,
                        TextRole::Body.font(&t),
                        t.color("popover-foreground"),
                    );
                    if hit.clicked() {
                        on = !on;
                        if on {
                            self.selected.push(value.clone());
                        } else {
                            self.selected.retain(|v| v != value);
                        }
                        changed = true;
                    }
                }
            });
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_matched_as_the_web_matches_them() {
        let set = Filter::new("status", FilterOperator::OneOf, ["failed"]);
        assert!(matches_filter(
            &FilterValue::Text("Failed".into()),
            &set,
            FilterType::Set
        ));
        assert!(!matches_filter(
            &FilterValue::Text("paid".into()),
            &set,
            FilterType::Set
        ));
        let least = Filter::new("amount", FilterOperator::AtLeast, ["1500"]);
        assert!(matches_filter(&FilterValue::Number(1500.0), &least, FilterType::Number));
        assert!(!matches_filter(
            &FilterValue::Number(1499.0),
            &least,
            FilterType::Number
        ));
        let days = Filter::new("date", FilterOperator::Between, ["2026-11-01", ""]);
        assert!(matches_filter(
            &FilterValue::Day("2026-11-01".into()),
            &days,
            FilterType::Date
        ));
        assert!(!matches_filter(
            &FilterValue::Day("2026-10-31".into()),
            &days,
            FilterType::Date
        ));
        let after = Filter::new("date", FilterOperator::After, ["2026-11-01"]);
        assert!(!matches_filter(
            &FilterValue::Day("2026-11-01".into()),
            &after,
            FilterType::Date
        ));
        // a filter still being filled in passes everything
        let open = Filter::new("amount", FilterOperator::AtLeast, Vec::<String>::new());
        assert!(matches_filter(&FilterValue::Number(1.0), &open, FilterType::Number));
        assert!(same_filters(
            &[Filter::new("a", FilterOperator::OneOf, ["x", "y"])],
            &[Filter::new("a", FilterOperator::OneOf, ["y", "x"]), open]
        ));
    }
}
