//! Drawing a schema form: an object's properties laid out in rows, each property's label, help
//! and control, or the fields, items or rows it opens into.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use egui::{Align, Id, Layout, Rect, Sense, Ui, UiBuilder, pos2, vec2};
use serde_json::Value;

use super::map::{MapRow, blank_of, object_of, repeated_keys, rows_of};
use super::mark::{MarkWords, Shown, fieldset_edge, issue_mark, mark_spot, paint_edge, worst};
use super::read::{
    Kind, field_schema, flag, get, given, initial, kind_of, label_of, lock, number_value, pointer, read_scalar, same,
    scalar_union, short, shown, shown_default, shows_default, text, types, unset_choice, value_at,
};
use super::{IssueTone, MapEntry, MapState, Move, SchemaFormWords, SchemaPath, State};
use crate::widgets::{
    Banner, BannerTone, Button, ButtonSize, ButtonVariant, Checkbox, Field, IconButton, Input, Select, Switch,
    SwitchField, read_number, write_number,
};
use crate::{Icon, TextRole, Tokens, tokens};

/// What drawing the form reads and writes beside the value.
pub(super) struct Cx<'a> {
    pub id: Id,
    pub root: &'a Value,
    pub words: &'a SchemaFormWords,
    pub state: &'a mut State,
    /// The issues each place shows this frame.
    pub shown: BTreeMap<String, Vec<Shown>>,
    pub fallbacks: bool,
    /// The places drawn, in order, for the order the issues are listed in.
    pub drawn: Vec<String>,
    pub unread_now: BTreeSet<String>,
    pub held_now: BTreeMap<String, String>,
    /// The places a draft was typed at this frame, which a change there keeps.
    pub drafted_now: BTreeSet<String>,
    /// Fixes chosen this frame, put in once the form is drawn.
    pub fixes: Vec<(SchemaPath, Value)>,
    pub changed: bool,
}

/// How a control is drawn where it stands.
struct Opts {
    /// The control's name, for a control drawn without a label of its own (a list's item).
    label: String,
    required: bool,
    /// What a text field holds once emptied: unset, or `""` for a list's item.
    empty: Option<Value>,
}

impl Cx<'_> {
    fn shown(&self, at: &str) -> &[Shown] {
        self.shown.get(at).map_or(&[], Vec::as_slice)
    }

    /// A control's id: its path with each list index read as its item's key, so a control's
    /// typing stays with its item as items are removed and moved.
    fn widget_id(&self, path: &[String]) -> Id {
        let mut id = self.id.with("field");
        for (depth, part) in path.iter().enumerate() {
            let list = pointer(&path[..depth]);
            let key = part
                .parse::<usize>()
                .ok()
                .and_then(|index| self.state.row_keys.get(&list).and_then(|keys| keys.get(index)));
            id = match key {
                Some(key) => id.with(("row", *key)),
                None => id.with(part),
            };
        }
        id
    }

    /// The keys of the list at a place, made for new items and cut for removed ones.
    fn keys_at(&mut self, at: &str, length: usize) -> Vec<u64> {
        let next = &mut self.state.next_key;
        let keys = self.state.row_keys.entry(at.to_owned()).or_default();
        while keys.len() < length {
            *next += 1;
            keys.push(*next);
        }
        keys.truncate(length);
        keys.clone()
    }

    fn mark_words(&self) -> MarkWords<'_> {
        MarkWords {
            critical: &self.words.issue_critical,
            warning: &self.words.issue_warning,
            use_fix: self.words.use_fix,
        }
    }

    /// Changes the value at a path, the draft there let go unless typed this frame.
    pub(super) fn set(&mut self, value: &mut Value, path: &[String], next: Option<Value>) {
        super::read::set_at(value, path, next);
        let at = pointer(path);
        if !self.drafted_now.contains(&at) {
            self.state.drafts.remove(&at);
        }
        self.state.changed.insert(at);
        self.changed = true;
    }

    /// The text typed at a place, while the value is the one it was typed over.
    fn draft(&self, at: &str, current: Option<&Value>) -> Option<String> {
        let (text, over) = self.state.drafts.get(at)?;
        (serde_json::to_string(&current).unwrap_or_default() == *over).then(|| text.clone())
    }

    /// Keeps the text typed at a place over the value it sets there.
    fn set_draft(&mut self, at: &str, text: String, over: Option<&Value>) {
        let over = serde_json::to_string(&over).unwrap_or_default();
        self.state.drafts.insert(at.to_owned(), (text, over));
        self.drafted_now.insert(at.to_owned());
    }

    /// Marks a field as left, so its issues show from now on, once it holds something.
    fn touch(&mut self, path: &[String], current: Option<&Value>) {
        let at = pointer(path);
        if given(current) || self.draft(&at, current).is_some() || self.unread_now.contains(&at) {
            self.state.touched.insert(at);
        }
    }

    /// The issue mark at `spot` for the place, opening and shutting its popover; the fix chosen.
    fn mark(&mut self, ui: &mut Ui, path: &[String], spot: Rect) -> Option<Value> {
        let at = pointer(path);
        let issues = self.shown(&at).to_vec();
        if issues.is_empty() {
            return None;
        }
        let open = self.state.open.as_deref() == Some(at.as_str());
        let id = self.widget_id(path);
        let out = issue_mark(ui, id, spot, &issues, open, &self.mark_words());
        if out.toggled {
            self.state.open = if open { None } else { Some(at) };
        } else if out.closed {
            self.state.open = None;
        }
        out.fix
    }

    /// Removes or moves a list's item, the marks kept by place moving with the items after it.
    fn reorder(&mut self, value: &mut Value, path: &[String], one: Move) {
        let at = pointer(path);
        let items = value_at(value, path)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let keys = self.keys_at(&at, items.len());
        let mut next_items: Vec<Option<Value>> = vec![None; items.len()];
        let mut next_keys: Vec<Option<u64>> = vec![None; items.len()];
        for (index, item) in items.into_iter().enumerate() {
            if let Some(to) = one.map(index) {
                next_items[to] = Some(item);
                next_keys[to] = Some(keys[index]);
            }
        }
        let next_items: Vec<Value> = next_items.into_iter().flatten().collect();
        let next_keys: Vec<u64> = next_keys.into_iter().flatten().collect();
        // the keys of lists inside the items move with their items, as the items' own keys do
        fn shift_map<T>(one: &Move, map: &mut BTreeMap<String, T>) {
            for (key, held) in std::mem::take(map) {
                if let Some(moved) = one.shift(&key) {
                    map.insert(moved, held);
                }
            }
        }
        self.state.row_keys.remove(&at);
        shift_map(&one, &mut self.state.row_keys);
        self.state.row_keys.insert(at.clone(), next_keys);
        shift_map(&one, &mut self.state.drafts);
        shift_map(&one, &mut self.state.numbers);
        shift_map(&one, &mut self.state.maps);
        shift_map(&one, &mut self.state.held);
        let shift_set = |set: &mut BTreeSet<String>| {
            *set = std::mem::take(set)
                .into_iter()
                .filter_map(|key| one.shift(&key))
                .collect();
        };
        shift_set(&mut self.state.touched);
        shift_set(&mut self.state.unread);
        shift_set(&mut self.state.changed);
        self.state.open = self.state.open.take().and_then(|open| one.shift(&open));
        self.state.moves.push(one);
        self.set(value, path, Some(Value::Array(next_items)));
    }
}

/// The form: what is wrong with it as a whole in a banner, then the root object's fields.
pub(super) fn form(ui: &mut Ui, cx: &mut Cx<'_>, value: &mut Value, fold: bool) {
    let root = field_schema(cx.root, cx.root);
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 16.0;
        for issue in cx.shown("").to_vec() {
            let words = match &issue.detail {
                Some(detail) => format!("{} {detail}", issue.text),
                None => issue.text.clone(),
            };
            let tone = match issue.tone {
                IssueTone::Critical => BannerTone::Critical,
                IssueTone::Warning => BannerTone::Warning,
            };
            Banner::new(tone, words).show(ui);
        }
        fields(ui, cx, value, &[], &root, fold);
    });
}

/// An object's properties laid out in rows: half-width fields pair up when the form is wide enough.
fn fields(ui: &mut Ui, cx: &mut Cx<'_>, value: &mut Value, path: &[String], raw: &Value, fold: bool) {
    // a read-only object, the form's root or a list's item among them, locks every field it holds
    let schema = if flag(raw, "readOnly") { lock(raw) } else { raw.clone() };
    let required: Vec<String> = get(&schema, "required")
        .and_then(Value::as_array)
        .map(|r| r.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default();
    let entries: Vec<(String, Value)> = get(&schema, "properties")
        .and_then(Value::as_object)
        .map(|p| p.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    let at = pointer(path);
    let with = |name: &str| {
        let mut p = path.to_vec();
        p.push(name.to_owned());
        p
    };
    // folded, the fields that are neither required nor set when the form is first drawn wait behind one disclosure
    let folded: Vec<String> = if fold {
        cx.state
            .folded
            .entry(at.clone())
            .or_insert_with(|| {
                entries
                    .iter()
                    .map(|(name, _)| name.clone())
                    .filter(|name| !required.contains(name) && !given(value_at(value, &with(name))))
                    .collect()
            })
            .clone()
    } else {
        Vec::new()
    };
    // a folded field with an issue to show opens the fold, so the count and a submit can reach it
    let opened = cx.state.unfolded.contains(&at)
        || folded.iter().any(|name| {
            let place = pointer(&with(name));
            cx.shown
                .iter()
                .any(|(k, v)| !v.is_empty() && (*k == place || k.starts_with(&format!("{place}/"))))
        });
    let visible: Vec<(String, Value)> = entries
        .into_iter()
        .filter(|(name, _)| opened || !folded.contains(name))
        .collect();
    let wide = ui.available_width() >= 512.0;
    let half_of = |cx: &Cx<'_>, child: &Value| {
        let at = field_schema(child, cx.root);
        wide && short(&at, kind_of(&at, cx.root))
    };
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 16.0;
        let mut index = 0;
        while index < visible.len() {
            let (name, child) = &visible[index];
            let half = half_of(cx, child);
            let pair = half && visible.get(index + 1).is_some_and(|(_, next)| half_of(cx, next));
            if half {
                let width = (ui.available_width() - 20.0) / 2.0;
                let take = if pair { 2 } else { 1 };
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 20.0;
                    for (name, child) in &visible[index..index + take] {
                        ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
                            ui.set_width(width);
                            property(ui, cx, value, &with(name), child, name, required.contains(name));
                        });
                    }
                });
                index += take;
            } else {
                property(ui, cx, value, &with(name), child, name, required.contains(name));
                index += 1;
            }
        }
        if !folded.is_empty() {
            let words = if opened {
                cx.words.fewer.clone()
            } else {
                (cx.words.more)(folded.len())
            };
            ui.horizontal(|ui| {
                ui.add_space(-10.0);
                let button = Button::new(words).variant(ButtonVariant::Ghost).size(ButtonSize::Sm);
                if ui.add(button).clicked() {
                    if opened {
                        cx.state.unfolded.remove(&at);
                    } else {
                        cx.state.unfolded.insert(at.clone());
                    }
                }
            });
        }
    });
}

/// A title as a heading draws it: the label, a star when required, and the key beside a title that
/// does not say it, as a field's label line.
fn title_job(ui: &Ui, t: &Tokens, label: &str, hint: Option<&str>, required: bool) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let style = ui.style().clone();
    let add = |rich: egui::RichText, job: &mut egui::text::LayoutJob| {
        rich.append_to(job, &style, egui::FontSelection::Default, Align::BOTTOM);
    };
    add(TextRole::Label.rich(t, label).color(t.color("foreground")), &mut job);
    if required {
        add(TextRole::Label.rich(t, " *").color(t.color("critical-ink")), &mut job);
    }
    if let Some(hint) = hint {
        add(
            TextRole::Help
                .rich(t, format!("\u{2002}\u{2009}{hint}"))
                .color(t.color("faint")),
            &mut job,
        );
    }
    job
}

/// Help under a field: its description, and what applies when it is left empty.
fn help_lines(t: Arc<Tokens>, description: Option<String>, fallback: Option<String>) -> impl FnOnce(&mut Ui) + 'static {
    move |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for line in [description, fallback].into_iter().flatten() {
            ui.add(egui::Label::new(TextRole::Help.rich(&t, line).color(t.color("muted-foreground"))).wrap());
        }
    }
}

/// One property of an object: its label, its help, and the control or the fields it opens into.
fn property(
    ui: &mut Ui,
    cx: &mut Cx<'_>,
    value: &mut Value,
    path: &[String],
    child: &Value,
    name: &str,
    required: bool,
) {
    let t = tokens(ui.ctx());
    let at = field_schema(child, cx.root);
    let kind = kind_of(&at, cx.root);
    let (label, hint) = label_of(&at, name);
    let locked = flag(&at, "readOnly");
    let current = value_at(value, path).cloned();
    let place = pointer(path);
    cx.drawn.push(place.clone());
    let description = text(&at, "description").map(str::to_owned);
    let fallback = (cx.fallbacks && get(&at, "default").is_some_and(|d| !d.is_null()))
        .then(|| (cx.words.fallback)(get(&at, "default").unwrap_or(&Value::Null)));
    match kind {
        Kind::Object => {
            let job = title_job(ui, &t, &label, hint.as_deref(), required);
            let legend = ui.painter().layout_job(job);
            let left = ui.cursor().left();
            let top = ui.cursor().top();
            let width = ui.available_width();
            let lh = legend.size().y;
            let inner = Rect::from_min_size(pos2(left + 12.0, top + lh + 8.0), vec2(width - 24.0, 100_000.0));
            let mut body = ui.new_child(UiBuilder::new().max_rect(inner).layout(Layout::top_down(Align::Min)));
            body.spacing_mut().item_spacing.y = 8.0;
            if let Some(description) = &description {
                body.add(
                    egui::Label::new(TextRole::Help.rich(&t, description).color(t.color("muted-foreground"))).wrap(),
                );
            }
            body.add_space(4.0);
            let sub = if locked { lock(&at) } else { at.clone() };
            fields(&mut body, cx, value, path, &sub, false);
            let outer = Rect::from_min_max(pos2(left, top), pos2(left + width, body.min_rect().bottom() + 12.0));
            ui.allocate_rect(outer, Sense::hover());
            let edge = Rect::from_min_max(pos2(left, top + lh / 2.0), outer.max);
            fieldset_edge(ui, &t, edge, left + 12.0, left + 12.0 + legend.size().x + 8.0);
            ui.painter()
                .galley(pos2(left + 16.0, top), legend, t.color("foreground"));
        }
        Kind::Boolean => {
            let mut on = match &current {
                Some(Value::Bool(b)) => *b,
                _ => get(&at, "default") == Some(&Value::Bool(true)),
            };
            let note = (current.is_none() && get(&at, "default").is_some_and(Value::is_boolean))
                .then(|| cx.words.default.clone());
            let issues = !cx.shown(&place).is_empty();
            let side = t.density("control-sm");
            let width = ui.available_width() - if issues { side + 8.0 } else { 0.0 };
            let row = ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let inner =
                    ui.allocate_ui_with_layout(vec2(width, t.density("control")), Layout::top_down(Align::Min), |ui| {
                        ui.set_width(width);
                        let mut field = SwitchField::new(&mut on, &label).enabled(!locked);
                        if let Some(description) = &description {
                            field = field.explain(description);
                        }
                        ui.add(field)
                    });
                let spot = issues.then(|| ui.allocate_exact_size(vec2(side, side), Sense::hover()).0);
                (inner.inner, inner.response.rect, spot)
            });
            let (switch, rect, spot) = row.inner;
            if let Some(note) = note {
                let label_width = ui
                    .painter()
                    .layout_no_wrap(label.clone(), TextRole::Body.font(&t), t.color("foreground"))
                    .size()
                    .x;
                let x = rect.left() + label_width + if description.is_some() { 22.0 } else { 0.0 } + 8.0;
                ui.painter().text(
                    pos2(x, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    note,
                    TextRole::Help.font(&t),
                    t.color("muted-foreground"),
                );
            }
            if switch.changed() {
                cx.set(value, path, Some(Value::Bool(on)));
            }
            if let Some(spot) = spot
                && let Some(fix) = cx.mark(ui, path, spot)
            {
                cx.fixes.push((path.to_vec(), fix));
            }
        }
        Kind::Choices => choices(ui, cx, value, path, &at, &label, hint.as_deref(), required, locked),
        Kind::Records => records(ui, cx, value, path, &at, &label, hint.as_deref(), required, locked),
        Kind::Map => map_field(ui, cx, value, path, &at, &label, hint.as_deref(), required, locked),
        Kind::List => list(ui, cx, value, path, &at, &label, hint.as_deref(), required, locked),
        _ => {
            let fallback = if shows_default(&at, kind, current.as_ref()) {
                None
            } else {
                fallback
            };
            let mut field = Field::new(&label).required(required).hint(hint);
            if description.is_some() || fallback.is_some() {
                field = field.help_ui(help_lines(t.clone(), description, fallback));
            }
            let change = field.show(ui, |ui| {
                let opts = Opts {
                    label: label.clone(),
                    required,
                    empty: None,
                };
                control(ui, cx, path, &at, current.as_ref(), &opts)
            });
            if let Some(next) = change {
                cx.set(value, path, next);
            }
        }
    }
}

/// A group's head: its title, the mark for what is wrong with it as a whole, and Add at the end.
fn head(
    ui: &mut Ui,
    cx: &mut Cx<'_>,
    path: &[String],
    label: &str,
    hint: Option<&str>,
    required: bool,
    add: bool,
) -> bool {
    let t = tokens(ui.ctx());
    let side = t.density("control-sm");
    let mut added = false;
    ui.horizontal(|ui| {
        ui.set_min_height(side);
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.label(title_job(ui, &t, label, hint, required));
        if !cx.shown(&pointer(path)).is_empty() {
            let (spot, _) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
            if let Some(fix) = cx.mark(ui, path, spot) {
                cx.fixes.push((path.to_vec(), fix));
            }
        }
        if add {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let button = Button::new(&cx.words.add)
                    .icon(Icon::Plus)
                    .variant(ButtonVariant::Ghost)
                    .size(ButtonSize::Sm);
                added = ui.add(button).clicked();
            });
        }
    });
    added
}

/// A group's description under its head, 4 under it.
fn group_description(ui: &mut Ui, t: &Tokens, description: Option<&str>) {
    if let Some(description) = description {
        ui.add_space(-4.0);
        ui.add(egui::Label::new(TextRole::Help.rich(t, description).color(t.color("muted-foreground"))).wrap());
    }
}

/// A set of choices: a checkbox for each value the items take.
#[allow(clippy::too_many_arguments)]
fn choices(
    ui: &mut Ui,
    cx: &mut Cx<'_>,
    value: &mut Value,
    path: &[String],
    at: &Value,
    label: &str,
    hint: Option<&str>,
    required: bool,
    locked: bool,
) {
    let t = tokens(ui.ctx());
    let item = get(at, "items").map(|i| field_schema(i, cx.root)).unwrap_or_default();
    let items: Vec<Value> = value_at(value, path)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        head(ui, cx, path, label, hint, required, false);
        let mut next = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(16.0, 8.0);
            for one in get(&item, "enum")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
            {
                let mut on = items.iter().any(|i| same(i, &one));
                let check = ui.add(Checkbox::new(&mut on, shown(&one)).enabled(!locked));
                if check.changed() {
                    next = Some(if on {
                        let mut list = items.clone();
                        list.push(one.clone());
                        list
                    } else {
                        items.iter().filter(|i| !same(i, &one)).cloned().collect()
                    });
                }
            }
        });
        if let Some(next) = next {
            let next = Value::Array(next);
            cx.set(value, path, Some(next.clone()));
            cx.touch(path, Some(&next));
        }
        if let Some(description) = text(at, "description") {
            ui.add(egui::Label::new(TextRole::Help.rich(&t, description).color(t.color("muted-foreground"))).wrap());
        }
    });
}

/// What a group's row asked this frame.
enum RowAsk {
    Remove(usize),
    Move(usize, usize),
}

/// A list of objects, as a repeated group: a box per item, its name with Move up, Move down and
/// Remove over the item's fields.
#[allow(clippy::too_many_arguments)]
fn records(
    ui: &mut Ui,
    cx: &mut Cx<'_>,
    value: &mut Value,
    path: &[String],
    at: &Value,
    label: &str,
    hint: Option<&str>,
    required: bool,
    locked: bool,
) {
    let t = tokens(ui.ctx());
    let item = get(at, "items").map(|i| field_schema(i, cx.root)).unwrap_or_default();
    let item = if locked { lock(&item) } else { item };
    let count = value_at(value, path).and_then(Value::as_array).map_or(0, Vec::len);
    let place = pointer(path);
    cx.keys_at(&place, count);
    let mut ask = None;
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        let added = head(ui, cx, path, label, hint, required, !locked);
        group_description(ui, &t, text(at, "description"));
        for index in 0..count {
            egui::Frame::new()
                .stroke(egui::Stroke::new(1.0, t.color("border")))
                .corner_radius(t.radius("surface"))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 12.0;
                    let name = (cx.words.item)(index);
                    ui.horizontal(|ui| {
                        ui.set_min_height(t.density("control-sm"));
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.label(TextRole::Badge.rich(&t, &name).color(t.color("muted-foreground")));
                        if locked {
                            return;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let remove = Button::new(&cx.words.remove)
                                .icon(Icon::X)
                                .variant(ButtonVariant::Ghost)
                                .size(ButtonSize::Sm)
                                .muted(true);
                            let remove_name = format!("{}: {name}", cx.words.remove);
                            if ui.add(remove).on_hover_text(&remove_name).clicked() {
                                ask = Some(RowAsk::Remove(index));
                            }
                            if count > 1
                                && let Some(to) = move_buttons(ui, cx, index, count, &name)
                            {
                                ask = Some(RowAsk::Move(index, to));
                            }
                        });
                    });
                    let mut row = path.to_vec();
                    row.push(index.to_string());
                    fields(ui, cx, value, &row, &item, false);
                });
        }
        if added {
            let mut items = value_at(value, path)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            items.push(initial(get(at, "items").unwrap_or(&Value::Null), cx.root, true).unwrap_or(Value::Null));
            cx.set(value, path, Some(Value::Array(items)));
        }
    });
    apply(cx, value, path, ask);
}

/// Move down and Move up, drawn from the end; shut at the last and the first item.
fn move_buttons(ui: &mut Ui, cx: &Cx<'_>, index: usize, count: usize, name: &str) -> Option<usize> {
    let mut to = None;
    let down = IconButton::new(Icon::ArrowDown, format!("{}: {name}", cx.words.move_down)).md();
    if ui.add_enabled(index + 1 < count, down).clicked() {
        to = Some(index + 1);
    }
    let up = IconButton::new(Icon::ArrowUp, format!("{}: {name}", cx.words.move_up)).md();
    if ui.add_enabled(index > 0, up).clicked() {
        to = Some(index - 1);
    }
    to
}

fn apply(cx: &mut Cx<'_>, value: &mut Value, path: &[String], ask: Option<RowAsk>) {
    let list = pointer(path);
    match ask {
        Some(RowAsk::Remove(index)) => cx.reorder(value, path, Move::Remove { list, index }),
        Some(RowAsk::Move(from, to)) => cx.reorder(value, path, Move::Move { list, from, to }),
        None => {}
    }
}

/// A list of scalars: a control per item, with Move up, Move down and Remove after it.
#[allow(clippy::too_many_arguments)]
fn list(
    ui: &mut Ui,
    cx: &mut Cx<'_>,
    value: &mut Value,
    path: &[String],
    at: &Value,
    label: &str,
    hint: Option<&str>,
    required: bool,
    locked: bool,
) {
    let t = tokens(ui.ctx());
    let items_schema = get(at, "items").cloned().unwrap_or(Value::Null);
    let item = field_schema(&items_schema, cx.root);
    let item = if locked { lock(&item) } else { item };
    let items: Vec<Value> = value_at(value, path)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let place = pointer(path);
    cx.keys_at(&place, items.len());
    let mut ask = None;
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        let added = head(ui, cx, path, label, hint, required, !locked);
        group_description(ui, &t, text(at, "description"));
        // an icon button is a control square, and a finger square on a phone
        let side = crate::widgets::finger_tall(ui.ctx(), &t, t.density("control"));
        let buttons = if locked {
            0.0
        } else {
            (side + 6.0) * if items.len() > 1 { 3.0 } else { 1.0 }
        };
        for (index, current) in items.iter().enumerate() {
            let mut row = path.to_vec();
            row.push(index.to_string());
            let name = (cx.words.item)(index);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let width = ui.available_width() - buttons;
                let change = ui
                    .allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
                        ui.set_width(width);
                        let opts = Opts {
                            label: format!("{label} {}", index + 1),
                            required: false,
                            empty: Some(Value::String(String::new())),
                        };
                        let current = (!current.is_null()).then_some(current);
                        control(ui, cx, &row, &item, current, &opts)
                    })
                    .inner;
                if let Some(next) = change {
                    cx.set(value, &row, next);
                }
                if locked {
                    return;
                }
                if items.len() > 1 {
                    let up = IconButton::new(Icon::ArrowUp, format!("{}: {name}", cx.words.move_up)).md();
                    if ui.add_enabled(index > 0, up).clicked() {
                        ask = Some(RowAsk::Move(index, index - 1));
                    }
                    let down = IconButton::new(Icon::ArrowDown, format!("{}: {name}", cx.words.move_down)).md();
                    if ui.add_enabled(index + 1 < items.len(), down).clicked() {
                        ask = Some(RowAsk::Move(index, index + 1));
                    }
                }
                let remove = IconButton::new(Icon::X, format!("{} {name}", cx.words.remove)).md();
                if ui.add(remove).clicked() {
                    ask = Some(RowAsk::Remove(index));
                }
            });
        }
        if added {
            let mut next = items.clone();
            next.push(super::read::blank_item(&items_schema, cx.root).unwrap_or(Value::Null));
            cx.set(value, path, Some(Value::Array(next)));
        }
    });
    apply(cx, value, path, ask);
}

/// A map of scalars: a key and a value per row, with Add in its head and Remove on each row.
#[allow(clippy::too_many_arguments)]
fn map_field(
    ui: &mut Ui,
    cx: &mut Cx<'_>,
    value: &mut Value,
    path: &[String],
    at: &Value,
    label: &str,
    hint: Option<&str>,
    required: bool,
    locked: bool,
) {
    let t = tokens(ui.ctx());
    let item = get(at, "additionalProperties")
        .filter(|a| a.is_object())
        .map(|a| field_schema(a, cx.root))
        .unwrap_or_default();
    let item = if locked { lock(&item) } else { item };
    let blank = blank_of(&item);
    let place = pointer(path);
    let current = value_at(value, path).cloned();
    let mut map = cx.state.maps.remove(&place).unwrap_or_else(|| MapState {
        rows: rows_of(current.as_ref(), &mut cx.state.next_key),
        seen: current.clone(),
    });
    // a value changed from outside the table, not by its own rows, draws its rows again
    if map.seen != current {
        map.seen = current.clone();
        let whole = current.clone().unwrap_or_else(|| Value::Object(Default::default()));
        if object_of(&map.rows, blank.as_ref()) != whole {
            map.rows = rows_of(current.as_ref(), &mut cx.state.next_key);
        }
    }
    let repeated = repeated_keys(&map.rows);
    if !repeated.is_empty() {
        cx.held_now.insert(place.clone(), cx.words.repeated_key.clone());
    }
    let flagged = !cx.shown(&place).is_empty();
    let mut dirty = false;
    let mut removed = None;
    let mut touched = false;
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 8.0;
        if head(ui, cx, path, label, hint, required, !locked) {
            cx.state.next_key += 1;
            map.rows.push(MapRow {
                id: cx.state.next_key,
                key: String::new(),
                value: None,
            });
            dirty = true;
        }
        group_description(ui, &t, text(at, "description"));
        if map.rows.is_empty() {
            return;
        }
        // an icon button is a control square, and a finger square on a phone
        let side = crate::widgets::finger_tall(ui.ctx(), &t, t.density("control"));
        let room = ui.available_width() - 2.0 * 6.0 - side;
        let (key_width, value_width) = (room * 2.0 / 5.0, room * 3.0 / 5.0);
        // a row as tall as its words, not a control's height
        let least = ui.spacing().interact_size.y;
        ui.spacing_mut().interact_size.y = 0.0;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for (words, width) in [(&cx.words.key, key_width), (&cx.words.value, value_width)] {
                ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
                    ui.set_width(width);
                    ui.label(TextRole::Help.rich(&t, words).color(t.color("muted-foreground")));
                });
            }
        });
        ui.spacing_mut().interact_size.y = least;
        for index in 0..map.rows.len() {
            let row = map.rows[index].clone();
            let twice = repeated.contains(&row.id);
            let settled = !row.key.is_empty() && !twice;
            let place_number = index + 1;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let mut key = row.key.clone();
                let key_response = ui
                    .allocate_ui_with_layout(vec2(key_width, 0.0), Layout::top_down(Align::Min), |ui| {
                        ui.set_width(key_width);
                        ui.push_id(cx.widget_id(path).with(("key", row.id)), |ui| {
                            ui.add(
                                Input::new(&mut key)
                                    .mono(true)
                                    .name((cx.words.map_key)(label, place_number))
                                    .invalid(twice && flagged)
                                    .read_only(locked),
                            )
                        })
                        .inner
                    })
                    .inner;
                if key_response.changed() {
                    map.rows[index].key = key;
                    dirty = true;
                }
                touched |= key_response.lost_focus();
                // a row whose key is not settled holds its value apart, at a place no issue reaches
                let mut row_path = path.to_vec();
                row_path.push(if settled {
                    row.key.clone()
                } else {
                    format!("#{}", row.id)
                });
                let change = ui
                    .allocate_ui_with_layout(vec2(value_width, 0.0), Layout::top_down(Align::Min), |ui| {
                        ui.set_width(value_width);
                        let opts = Opts {
                            label: (cx.words.map_value)(label, place_number),
                            required: false,
                            empty: None,
                        };
                        control(ui, cx, &row_path, &item, row.value.as_ref(), &opts)
                    })
                    .inner;
                if let Some(next) = change {
                    map.rows[index].value = next;
                    dirty = true;
                }
                if !locked {
                    let entry = if settled {
                        MapEntry::Key(&row.key)
                    } else {
                        MapEntry::Row(place_number)
                    };
                    let remove = IconButton::new(Icon::X, (cx.words.remove_entry)(label, entry)).md();
                    if ui.add(remove).clicked() {
                        removed = Some(row.id);
                    }
                }
            });
        }
    });
    if let Some(id) = removed {
        map.rows.retain(|row| row.id != id);
        dirty = true;
    }
    if dirty {
        let next = object_of(&map.rows, blank.as_ref());
        cx.set(value, path, Some(next));
    }
    if touched {
        cx.touch(path, value_at(value, path));
    }
    cx.state.maps.insert(place, map);
}

/// A field's own control, with the edge and mark of its issues: answers the value it now holds
/// when it was changed, `None` inside for unset.
fn control(
    ui: &mut Ui,
    cx: &mut Cx<'_>,
    path: &[String],
    at: &Value,
    current: Option<&Value>,
    opts: &Opts,
) -> Option<Option<Value>> {
    let t = tokens(ui.ctx());
    let kind = kind_of(at, cx.root);
    let place = pointer(path);
    let locked = flag(at, "readOnly");
    let tone = worst(cx.shown(&place));
    let critical = tone == Some(IssueTone::Critical);
    // an unset field with a default shows it, muted, as what applies: "normal (default)"
    let placeholder = shown_default(at, current).map(|f| format!("{} ({})", shown(f), cx.words.default));
    let id = cx.widget_id(path);
    let mut out = None;
    let mut top = false;
    let mut leave = false;
    let rect = match kind {
        Kind::Enum => {
            let unset = unset_choice(at, opts.required, &cx.words.none, cx.words.use_default);
            let mut labels = Vec::new();
            let mut values: Vec<Option<Value>> = Vec::new();
            if let Some(unset) = unset {
                labels.push(unset);
                values.push(None);
            }
            for one in get(at, "enum").and_then(Value::as_array).cloned().unwrap_or_default() {
                labels.push(shown(&one));
                values.push(Some(one));
            }
            let mut chosen = if given(current) {
                values
                    .iter()
                    .position(|v| v.as_ref().zip(current).is_some_and(|(a, b)| same(a, b)))
                    .unwrap_or(usize::MAX)
            } else if placeholder.is_none() {
                values.iter().position(Option::is_none).unwrap_or(usize::MAX)
            } else {
                usize::MAX
            };
            let words = placeholder.clone().unwrap_or_else(|| cx.words.choose.clone());
            let width = ui.available_width();
            if locked {
                let mut shown_text = labels.get(chosen).cloned().unwrap_or_default();
                let mut input = Input::new(&mut shown_text).name(&opts.label).read_only(true);
                if labels.get(chosen).is_none() {
                    input = input.placeholder(words);
                }
                ui.push_id(id, |ui| ui.scope(|ui| ui.add(input)).response.rect).inner
            } else {
                let response = ui
                    .push_id(id, |ui| {
                        ui.add(
                            Select::new(&mut chosen, labels, &opts.label)
                                .placeholder(words)
                                .width(width),
                        )
                    })
                    .inner;
                if response.changed() {
                    out = Some(values.get(chosen).cloned().flatten());
                    leave = true;
                }
                if tone.is_some() {
                    // the chevron moves in before the mark at the end, as the trigger's `pr-9`
                    let side = t.density("control-sm");
                    let cover = Rect::from_min_max(
                        pos2(response.rect.right() - 4.0 - side - 26.0, response.rect.top() + 1.0),
                        pos2(response.rect.right() - 1.0, response.rect.bottom() - 1.0),
                    );
                    let ground = if response.hovered() {
                        t.color("hover")
                    } else {
                        t.color("field")
                    };
                    ui.painter().rect_filled(cover, t.radius("control"), ground);
                    crate::icons::paint(
                        ui.painter(),
                        pos2(cover.left() + 12.0, response.rect.center().y),
                        Icon::ChevronDown,
                        16.0,
                        t.color("muted-foreground"),
                    );
                }
                response.rect
            }
        }
        Kind::Number => {
            let integer = types(at).0.first().map(String::as_str) == Some("integer");
            let typed = cx.state.numbers.get(&place).cloned();
            let mut typing =
                typed.unwrap_or_else(|| current.and_then(Value::as_f64).map(write_number).unwrap_or_default());
            let scope = ui.push_id(id, |ui| {
                ui.scope(|ui| {
                    let mut input = Input::new(&mut typing)
                        .name(&opts.label)
                        .invalid(critical)
                        .read_only(locked);
                    if let Some(placeholder) = &placeholder {
                        input = input.placeholder(placeholder);
                    }
                    ui.add(input)
                })
            });
            let response = scope.inner.inner;
            let readable = |s: &str| read_number(s).filter(|n| !integer || n.fract() == 0.0);
            if response.changed() {
                if typing.trim().is_empty() {
                    if given(current) {
                        out = Some(opts.empty.clone());
                    }
                } else if let Some(n) = readable(&typing) {
                    let next = number_value(n);
                    if current.is_none_or(|c| !same(c, &next)) {
                        out = Some(Some(next));
                    }
                }
            }
            if response.has_focus() && !locked {
                // the arrows step the value within its bounds, ten steps with Shift
                let steps = ui.input_mut(|i| {
                    let mut steps = 0.0;
                    for (modifiers, by) in [(egui::Modifiers::NONE, 1.0), (egui::Modifiers::SHIFT, 10.0)] {
                        if i.consume_key(modifiers, egui::Key::ArrowUp) {
                            steps += by;
                        }
                        if i.consume_key(modifiers, egui::Key::ArrowDown) {
                            steps -= by;
                        }
                    }
                    steps
                });
                if steps != 0.0 {
                    let step = get(at, "multipleOf").and_then(Value::as_f64).unwrap_or(1.0);
                    let from = readable(&typing).or(current.and_then(Value::as_f64)).unwrap_or(0.0);
                    let mut next = ((from / step).round() + steps) * step;
                    if let Some(min) = get(at, "minimum").and_then(Value::as_f64) {
                        next = next.max(min);
                    }
                    if let Some(max) = get(at, "maximum").and_then(Value::as_f64) {
                        next = next.min(max);
                    }
                    typing = write_number(next);
                    out = Some(Some(number_value(next)));
                }
            }
            let unreadable = !typing.trim().is_empty() && readable(&typing).is_none();
            if unreadable {
                cx.unread_now.insert(place.clone());
            }
            if response.lost_focus() {
                leave = true;
            }
            if response.has_focus() || unreadable {
                cx.state.numbers.insert(place.clone(), typing);
            } else {
                cx.state.numbers.remove(&place);
            }
            scope.inner.response.rect
        }
        Kind::Boolean => {
            let mut on = match current {
                Some(Value::Bool(b)) => *b,
                _ => get(at, "default") == Some(&Value::Bool(true)),
            };
            let width = ui.available_width();
            let row = ui.allocate_ui_with_layout(
                vec2(width, t.density("control")),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.set_min_size(vec2(width, t.density("control")));
                    ui.push_id(id, |ui| ui.add(Switch::new(&mut on, &opts.label).enabled(!locked)))
                        .inner
                },
            );
            if row.inner.changed() {
                out = Some(Some(Value::Bool(on)));
                leave = true;
            }
            // a switch wears no edge; its mark stands at the row's end
            let rect = row.response.rect;
            if let Some(fix) = cx.mark(ui, path, mark_spot(&t, rect, false)) {
                out = Some(Some(fix));
            }
            if leave {
                cx.touch(path, out.as_ref().and_then(Option::as_ref).or(current));
            }
            return out;
        }
        Kind::Scalars | Kind::Json | Kind::LongText | Kind::Text => {
            let allowed = scalar_union(at, cx.root).unwrap_or_default();
            let mut typing = match kind {
                Kind::Scalars => cx.draft(&place, current).unwrap_or_else(|| {
                    if given(current) {
                        current.map(shown).unwrap_or_default()
                    } else {
                        String::new()
                    }
                }),
                Kind::Json => cx.draft(&place, current).unwrap_or_else(|| {
                    current
                        .map(|v| serde_json::to_string_pretty(v).unwrap_or_default())
                        .unwrap_or_default()
                }),
                _ => match current {
                    Some(Value::String(s)) => s.clone(),
                    Some(Value::Null) | None => String::new(),
                    Some(other) => shown(other),
                },
            };
            let multiline = matches!(kind, Kind::Json | Kind::LongText);
            top = multiline;
            let mono = kind == Kind::Json
                || (kind == Kind::Text
                    && (get(at, "pattern").is_some()
                        || matches!(text(at, "format"), Some("uri" | "uuid" | "hostname"))));
            let scope = ui.push_id(id, |ui| {
                ui.scope(|ui| {
                    let mut input = Input::new(&mut typing)
                        .name(&opts.label)
                        .invalid(critical)
                        .read_only(locked)
                        .multiline(multiline)
                        .mono(mono);
                    if let Some(placeholder) = placeholder.as_ref().filter(|_| kind != Kind::Json) {
                        input = input.placeholder(placeholder);
                    }
                    ui.add(input)
                })
            });
            let response = scope.inner.inner;
            if response.changed() {
                match kind {
                    Kind::Scalars => {
                        let next = read_scalar(&typing, &allowed);
                        cx.set_draft(&place, typing.clone(), next.as_ref());
                        out = Some(next);
                    }
                    Kind::Json => {
                        let parsed = if typing.trim().is_empty() {
                            Some(None)
                        } else {
                            serde_json::from_str::<Value>(&typing).ok().map(Some)
                        };
                        // the text stays as typed and says it does not parse
                        let over = parsed.clone().unwrap_or_else(|| current.cloned());
                        cx.set_draft(&place, typing.clone(), over.as_ref());
                        out = parsed;
                    }
                    _ => {
                        out = Some(if typing.is_empty() {
                            opts.empty.clone()
                        } else {
                            Some(Value::String(typing.clone()))
                        });
                    }
                }
            }
            leave = response.lost_focus();
            scope.inner.response.rect
        }
        Kind::Object | Kind::Records | Kind::List | Kind::Choices | Kind::Map => {
            // drawn by the property itself; a list's item of these kinds is edited as JSON
            return None;
        }
    };
    if let Some(tone) = tone {
        paint_edge(ui, &t, rect, tone);
    }
    if let Some(fix) = cx.mark(ui, path, mark_spot(&t, rect, top)) {
        out = Some(Some(fix));
    }
    if leave {
        let now = out.as_ref().map_or(current, Option::as_ref);
        cx.touch(path, now);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_place_moves_with_its_item() {
        let list = "/hooks".to_owned();
        let removed = Move::Remove {
            list: list.clone(),
            index: 0,
        };
        assert_eq!(removed.shift("/hooks/0/token"), None);
        assert_eq!(removed.shift("/hooks/2/token").as_deref(), Some("/hooks/1/token"));
        assert_eq!(removed.shift("/other/2").as_deref(), Some("/other/2"));
        let moved = Move::Move { list, from: 0, to: 1 };
        assert_eq!(moved.shift("/hooks/0").as_deref(), Some("/hooks/1"));
        assert_eq!(moved.shift("/hooks/1").as_deref(), Some("/hooks/0"));
        assert_eq!(moved.shift("/hooks/2").as_deref(), Some("/hooks/2"));
    }
}
