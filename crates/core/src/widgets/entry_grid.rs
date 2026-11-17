//! Numbers entered by row and column, as sokkel-ui's `DataTable` draws a grid of counts edited in
//! place: rows in folding groups, columns under bands, and the rows stacked into fields where the
//! columns do not fit.

use std::collections::HashSet;

use egui::{Id, Rect, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{Icon, Input, SearchField, TextRole, icons, read_number, tokens, write_number};

/// One line of a grid: what it is called and the group it is listed under.
#[derive(Clone, Debug)]
pub struct EntryRow {
    pub id: String,
    pub label: String,
    pub group: Option<String>,
}

/// One column of a grid: what it is called and the band over it.
#[derive(Clone, Debug)]
pub struct EntryColumn {
    pub id: String,
    pub label: String,
    pub band: Option<String>,
}

/// The words a grid says.
#[derive(Clone, Debug)]
pub struct EntryGridWords {
    pub search: String,
    /// `{}` the count of rows shown.
    pub rows: String,
    pub total: String,
    pub empty: String,
    /// Said by a cell holding what is not a whole number of 0 or more.
    pub whole: String,
}

impl Default for EntryGridWords {
    fn default() -> Self {
        Self {
            search: "Search".into(),
            rows: "{} rows".into(),
            total: "Total".into(),
            empty: "Nothing matches.".into(),
            whole: "Counts are whole numbers, 0 or more.".into(),
        }
    }
}

/// A cell given a new value, `None` for emptied.
#[derive(Clone, Debug, PartialEq)]
pub struct EntryEdit {
    pub row: String,
    pub column: String,
    pub value: Option<f64>,
}

#[derive(Clone, Default)]
struct Held {
    shut: HashSet<String>,
    typing: Vec<((String, String), String)>,
}

/// A grid of numbers: `rows` down, `columns` across, each cell's value from `value`, a total per
/// row, the rows in folding groups. Where the columns, each at least `column_width`, fit beside the
/// row's label, it is a table with a band row over the columns; otherwise each row stacks into a
/// title over a field per column, named by its band and its own name. `toolbar` draws the caller's
/// controls under the search. Answers with the cells changed.
pub struct EntryGrid<'a> {
    id: Id,
    rows: &'a [EntryRow],
    columns: &'a [EntryColumn],
    value: &'a dyn Fn(&str, &str) -> Option<f64>,
    label_width: f32,
    column_width: f32,
    total: bool,
    read_only: bool,
    query: Option<&'a mut String>,
    words: EntryGridWords,
}

impl<'a> EntryGrid<'a> {
    pub fn new(
        id: impl egui::AsIdSalt,
        rows: &'a [EntryRow],
        columns: &'a [EntryColumn],
        value: &'a dyn Fn(&str, &str) -> Option<f64>,
    ) -> Self {
        Self {
            id: Id::unique(id),
            rows,
            columns,
            value,
            label_width: 224.0,
            column_width: 104.0,
            total: true,
            read_only: false,
            query: None,
            words: EntryGridWords::default(),
        }
    }

    /// The row label's least width and each column's.
    pub fn widths(mut self, label: f32, column: f32) -> Self {
        self.label_width = label;
        self.column_width = column;
        self
    }

    /// A total column at the end; on by default.
    pub fn total(mut self, total: bool) -> Self {
        self.total = total;
        self
    }

    /// The values shown, never typed in, as a record's answers are read.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// A search over the rows' labels and groups, its text held by the caller.
    pub fn search(mut self, query: &'a mut String) -> Self {
        self.query = Some(query);
        self
    }

    pub fn words(mut self, words: EntryGridWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(mut self, ui: &mut Ui, toolbar: impl FnOnce(&mut Ui)) -> Vec<EntryEdit> {
        let t = tokens(ui.ctx());
        let query = self.query.take();
        let mut held: Held = ui.data(|d| d.get_temp(self.id)).unwrap_or_default();
        let mut edits = Vec::new();
        let width = ui.available_width();
        let pad = t.density("panel-pad");
        let shown = self.rows.len();
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            if !self.read_only {
                // the toolbar: the search and the count of rows, the caller's controls under them
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(pad as i8, 8))
                    .show(ui, |ui| {
                        ui.set_width(width - 2.0 * pad);
                        ui.spacing_mut().item_spacing.y = 8.0;
                        if let Some(query) = query {
                            ui.horizontal(|ui| {
                                SearchField::new(self.id.with("search"), query)
                                    .width(256.0)
                                    .keys("/")
                                    .show(ui);
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let mono = egui::FontId::monospace(TextRole::Help.font(&t).size);
                                    ui.label(
                                        egui::RichText::new(self.words.rows.replace("{}", &shown.to_string()))
                                            .font(mono)
                                            .color(t.color("muted-foreground")),
                                    );
                                });
                            });
                        }
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            toolbar(ui);
                        });
                    });
                let rule = |ui: &mut Ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(width, 1.0), Sense::hover());
                    ui.painter()
                        .hline(r.x_range(), r.center().y, Stroke::new(1.0, t.color("border")));
                };
                rule(ui);
            }
            if self.rows.is_empty() {
                ui.add_space(24.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        TextRole::Body
                            .rich(&t, &self.words.empty)
                            .color(t.color("muted-foreground")),
                    );
                });
                ui.add_space(24.0);
                return;
            }
            let across = self.columns.len() + usize::from(self.total);
            let fits = self.label_width + across as f32 * self.column_width + 2.0 * pad <= width;
            if fits {
                self.band_head(ui, width, pad);
            }
            let mut group: Option<&String> = None;
            let mut first = true;
            for row in self.rows {
                if row.group.as_ref() != group || first {
                    first = false;
                    group = row.group.as_ref();
                    if let Some(name) = group {
                        let count = self.rows.iter().filter(|r| r.group.as_ref() == Some(name)).count();
                        let (head, hit) = ui.allocate_exact_size(vec2(width, t.density("panel-head")), Sense::click());
                        let shut = held.shut.contains(name);
                        hit.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, !shut, name));
                        ui.painter().rect_filled(head, 0.0, t.color("section-head-ground"));
                        ui.painter()
                            .hline(head.x_range(), head.bottom() - 0.5, Stroke::new(1.0, t.color("border")));
                        icons::paint(
                            ui.painter(),
                            pos2(head.left() + pad + 8.0, head.center().y),
                            if shut { Icon::ChevronRight } else { Icon::ChevronDown },
                            14.0,
                            t.color("muted-foreground"),
                        );
                        // a group's head in the column head's type, as the web's group rows
                        let title = WidgetText::from(TextRole::ColumnHead.rich(&t, name)).into_galley(
                            ui,
                            Some(TextWrapMode::Extend),
                            f32::INFINITY,
                            TextRole::ColumnHead.font(&t),
                        );
                        let x = head.left() + pad + 24.0;
                        let title_width = title.size().x;
                        ui.painter().galley(
                            pos2(x, head.center().y - title.size().y / 2.0),
                            title,
                            t.color("foreground"),
                        );
                        ui.painter().text(
                            pos2(x + title_width + 8.0, head.center().y),
                            egui::Align2::LEFT_CENTER,
                            count.to_string(),
                            TextRole::Help.font(&t),
                            t.color("muted-foreground"),
                        );
                        if hit.clicked() {
                            if shut {
                                held.shut.remove(name);
                            } else {
                                held.shut.insert(name.clone());
                            }
                        }
                    }
                }
                if group.is_some_and(|g| held.shut.contains(g)) {
                    continue;
                }
                if fits {
                    self.table_row(ui, &mut held, row, width, pad, &mut edits);
                } else {
                    self.stacked_row(ui, &mut held, row, width, pad, &mut edits);
                }
            }
        });
        ui.data_mut(|d| d.insert_temp(self.id, held));
        edits
    }

    /// The heads over the columns: the bands in a row of their own over the names.
    fn band_head(&self, ui: &mut Ui, width: f32, pad: f32) {
        let t = tokens(ui.ctx());
        let banded = self.columns.iter().any(|c| c.band.is_some());
        let line = t.density("row");
        let (head, _) = ui.allocate_exact_size(vec2(width, line * if banded { 2.0 } else { 1.0 }), Sense::hover());
        let quiet = t.color("muted-foreground");
        let font = TextRole::Help.font(&t);
        let left = head.left() + pad + self.label_width;
        let mut x = left;
        let mut start = 0;
        while start < self.columns.len() {
            let band = &self.columns[start].band;
            let end = (start..self.columns.len())
                .find(|i| self.columns[*i].band != *band)
                .unwrap_or(self.columns.len());
            if let Some(band) = band {
                let span = (end - start) as f32 * self.column_width;
                ui.painter().text(
                    pos2(x + span / 2.0, head.top() + line / 2.0),
                    egui::Align2::CENTER_CENTER,
                    band,
                    font.clone(),
                    quiet,
                );
            }
            x += (end - start) as f32 * self.column_width;
            start = end;
        }
        let top = if banded { head.top() + line } else { head.top() };
        for (index, column) in self.columns.iter().enumerate() {
            ui.painter().text(
                pos2(left + (index + 1) as f32 * self.column_width - 12.0, top + line / 2.0),
                egui::Align2::RIGHT_CENTER,
                &column.label,
                font.clone(),
                quiet,
            );
        }
        if self.total {
            ui.painter().text(
                pos2(head.right() - pad, top + line / 2.0),
                egui::Align2::RIGHT_CENTER,
                &self.words.total,
                font,
                quiet,
            );
        }
        ui.painter()
            .hline(head.x_range(), head.bottom() - 0.5, Stroke::new(1.0, t.color("border")));
    }

    fn total_of(&self, row: &EntryRow) -> f64 {
        self.columns.iter().filter_map(|c| (self.value)(&row.id, &c.id)).sum()
    }

    /// One cell's number input, the typed text kept while it is not yet a number.
    fn cell(
        &self,
        ui: &mut Ui,
        held: &mut Held,
        row: &EntryRow,
        column: &EntryColumn,
        name: String,
        edits: &mut Vec<EntryEdit>,
    ) {
        let key = (row.id.clone(), column.id.clone());
        let value = (self.value)(&row.id, &column.id);
        if self.read_only {
            let t = tokens(ui.ctx());
            let (slot, _) = ui.allocate_exact_size(vec2(ui.available_width(), t.density("control")), Sense::hover());
            ui.painter().text(
                pos2(slot.right() - 8.0, slot.center().y),
                egui::Align2::RIGHT_CENTER,
                value.map(|v| crate::format_value(v, Some(0))).unwrap_or_default(),
                TextRole::Body.font(&t),
                t.color("foreground"),
            );
            return;
        }
        let at = held.typing.iter().position(|(k, _)| *k == key);
        let mut text = at.map_or_else(
            || value.map(write_number).unwrap_or_default(),
            |i| held.typing[i].1.clone(),
        );
        let read = read_number(&text);
        let wrong = !text.trim().is_empty() && read.is_none_or(|v| v.fract() != 0.0 || v < 0.0);
        let response = ui.add(
            Input::new(&mut text)
                .name(name)
                .invalid(wrong)
                .width(ui.available_width()),
        );
        if wrong {
            response.on_hover_text(&self.words.whole);
        }
        let read = read_number(&text);
        let fine = text.trim().is_empty() || read.is_some_and(|v| v.fract() == 0.0 && v >= 0.0);
        let next = if text.trim().is_empty() { None } else { read };
        if fine && next != value {
            edits.push(EntryEdit {
                row: row.id.clone(),
                column: column.id.clone(),
                value: next,
            });
        }
        match (at, fine && next == value || fine) {
            (Some(i), true) => {
                held.typing.remove(i);
            }
            (Some(i), false) => held.typing[i].1 = text,
            (None, false) => held.typing.push((key, text)),
            (None, true) => {}
        }
    }

    fn stacked_row(
        &self,
        ui: &mut Ui,
        held: &mut Held,
        row: &EntryRow,
        width: f32,
        pad: f32,
        edits: &mut Vec<EntryEdit>,
    ) {
        let t = tokens(ui.ctx());
        egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(pad as i8, 12))
            .show(ui, |ui| {
                ui.set_width(width - 2.0 * pad);
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.label(TextRole::SectionTitle.rich(&t, &row.label).color(t.color("foreground")));
                for column in self.columns {
                    let name = match &column.band {
                        Some(band) => format!("{band} · {}", column.label),
                        None => column.label.clone(),
                    };
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let (slot, _) = ui.allocate_exact_size(vec2(92.0, t.density("control")), Sense::hover());
                        ui.painter().text(
                            pos2(slot.left(), slot.center().y),
                            egui::Align2::LEFT_CENTER,
                            &name,
                            TextRole::Help.font(&t),
                            t.color("muted-foreground"),
                        );
                        self.cell(ui, held, row, column, format!("{}, {name}", row.label), edits);
                    });
                }
                if self.total {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let (slot, _) = ui.allocate_exact_size(vec2(92.0, t.density("control")), Sense::hover());
                        ui.painter().text(
                            pos2(slot.left(), slot.center().y),
                            egui::Align2::LEFT_CENTER,
                            &self.words.total,
                            TextRole::Help.font(&t),
                            t.color("muted-foreground"),
                        );
                        ui.label(
                            TextRole::Body
                                .rich(&t, crate::format_value(self.total_of(row), Some(0)))
                                .variation("wght", 500.0),
                        );
                    });
                }
            });
        let (rule, _) = ui.allocate_exact_size(vec2(width, 1.0), Sense::hover());
        ui.painter()
            .hline(rule.x_range(), rule.center().y, Stroke::new(1.0, t.color("border")));
    }

    fn table_row(
        &self,
        ui: &mut Ui,
        held: &mut Held,
        row: &EntryRow,
        width: f32,
        pad: f32,
        edits: &mut Vec<EntryEdit>,
    ) {
        let t = tokens(ui.ctx());
        let height = t.density("row");
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        ui.painter().text(
            pos2(rect.left() + pad, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &row.label,
            TextRole::Body.font(&t),
            t.color("foreground"),
        );
        let left = rect.left() + pad + self.label_width;
        for (index, column) in self.columns.iter().enumerate() {
            let cell = Rect::from_min_size(
                pos2(left + index as f32 * self.column_width + 4.0, rect.top() + 2.0),
                vec2(self.column_width - 8.0, height - 4.0),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(cell));
            let name = match &column.band {
                Some(band) => format!("{}, {band} · {}", row.label, column.label),
                None => format!("{}, {}", row.label, column.label),
            };
            self.cell(&mut child, held, row, column, name, edits);
        }
        if self.total {
            ui.painter().text(
                pos2(rect.right() - pad, rect.center().y),
                egui::Align2::RIGHT_CENTER,
                crate::format_value(self.total_of(row), Some(0)),
                TextRole::Body.font(&t),
                t.color("foreground"),
            );
        }
        ui.painter()
            .hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, t.color("border")));
    }
}
