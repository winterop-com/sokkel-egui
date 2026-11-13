//! The one table, after sokkel-ui's `DataTable`: every listing, its rows at the density's row
//! height and only those in view drawn; heads that sort and resize; selection with a bar; one row
//! opened for a detail beside it.

use std::collections::BTreeSet;

use egui::{CornerRadius, CursorIcon, Id, Rect, Response, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{Button, ButtonSize, ButtonVariant, Checkbox, ChipTone, Icon, TextRole, icons, tokens};

/// A column: its head, its starting width, and how its cells sit.
#[derive(Clone, Debug)]
pub struct Column {
    pub title: String,
    pub width: f32,
    pub numeric: bool,
    pub sortable: bool,
}

impl Column {
    pub fn new(title: impl Into<String>, width: f32) -> Self {
        Self {
            title: title.into(),
            width,
            numeric: false,
            sortable: true,
        }
    }

    /// Figures: set at the cell's end, and sorted as numbers.
    pub fn numeric(mut self) -> Self {
        self.numeric = true;
        self
    }

    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }
}

/// What a cell holds.
#[derive(Clone, Debug)]
pub enum Cell {
    Text(String),
    /// An id or a code, in the mono face.
    Mono(String),
    /// A state as a chip.
    Chip(String, ChipTone),
}

impl Cell {
    fn sort_key(&self) -> &str {
        match self {
            Cell::Text(text) | Cell::Mono(text) | Cell::Chip(text, _) => text,
        }
    }
}

impl From<&str> for Cell {
    fn from(text: &str) -> Self {
        Cell::Text(text.to_owned())
    }
}

/// What a table keeps between frames; the app holds it.
#[derive(Clone, Debug, Default)]
pub struct TableState {
    widths: Vec<f32>,
    /// The sorted column and whether it ascends.
    pub sort: Option<(usize, bool)>,
    /// The selected rows, by their index in the rows given.
    pub selected: BTreeSet<usize>,
    /// The row opened for its detail, by its index in the rows given.
    pub opened: Option<usize>,
}

impl TableState {
    /// A state that starts sorted by `column`, ascending or not.
    pub fn sorted(column: usize, ascending: bool) -> Self {
        Self {
            sort: Some((column, ascending)),
            ..Self::default()
        }
    }
}

/// A table's look: `Table` with a head as tall as a row, `List` with the compact panel head.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableVariant {
    #[default]
    Table,
    List,
}

/// The words a table says, English by default.
#[derive(Clone, Debug)]
pub struct TableWords {
    pub selected: String,
    pub clear: String,
    pub select_all: String,
    pub select_row: String,
    pub rows: String,
    pub row: String,
    pub empty: String,
}

impl Default for TableWords {
    fn default() -> Self {
        Self {
            selected: "{count} selected".into(),
            clear: "Clear".into(),
            select_all: "Select all rows".into(),
            select_row: "Select row".into(),
            rows: "{count} rows".into(),
            row: "{count} row".into(),
            empty: "Nothing to show".into(),
        }
    }
}

/// The table: `columns` over `rows`, its state held by the app.
pub struct DataTable<'a> {
    id: Id,
    columns: &'a [Column],
    rows: &'a [Vec<Cell>],
    state: &'a mut TableState,
    selectable: bool,
    variant: TableVariant,
    height: f32,
    words: TableWords,
}

impl<'a> DataTable<'a> {
    pub fn new(id: impl egui::AsId, columns: &'a [Column], rows: &'a [Vec<Cell>], state: &'a mut TableState) -> Self {
        Self {
            id: Id::new(id),
            columns,
            rows,
            state,
            selectable: false,
            variant: TableVariant::Table,
            height: 360.0,
            words: TableWords::default(),
        }
    }

    /// A check box on every row and one over them in the head.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    pub fn variant(mut self, variant: TableVariant) -> Self {
        self.variant = variant;
        self
    }

    /// The most the rows take before they scroll.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn words(mut self, words: TableWords) -> Self {
        self.words = words;
        self
    }

    /// The rows' order under the current sort, as indices into the rows given.
    fn order(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.rows.len()).collect();
        if let Some((column, ascending)) = self.state.sort {
            let numeric = self.columns.get(column).is_some_and(|c| c.numeric);
            order.sort_by(|a, b| {
                let (x, y) = (self.rows[*a][column].sort_key(), self.rows[*b][column].sort_key());
                let ordering = if numeric {
                    let parse = |s: &str| {
                        s.replace(',', "")
                            .trim_end_matches(|c: char| !c.is_ascii_digit())
                            .parse::<f64>()
                            .unwrap_or(f64::MIN)
                    };
                    parse(x).total_cmp(&parse(y))
                } else {
                    x.cmp(y)
                };
                if ascending { ordering } else { ordering.reverse() }
            });
        }
        order
    }

    /// Draw the table, stacked top to bottom whatever layout the row around it has.
    pub fn show(self, ui: &mut Ui) -> Response {
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| self.show_stacked(ui))
            .inner
    }

    fn show_stacked(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        if self.state.widths.len() != self.columns.len() {
            self.state.widths = self.columns.iter().map(|c| c.width).collect();
        }
        let row_height = t.density("row");
        let head_height = match self.variant {
            TableVariant::Table => row_height,
            TableVariant::List => t.density("panel-head"),
        };
        let pad = match self.variant {
            TableVariant::Table => 12.0,
            TableVariant::List => t.density("panel-pad"),
        };
        let select_width = if self.selectable { 40.0 } else { 0.0 };
        let order = self.order();
        let radius = t.radius("surface");
        let width = ui.available_width();

        // the selection bar, over the table while any row is chosen
        if self.selectable && !self.state.selected.is_empty() {
            ui.horizontal(|ui| {
                let words = self
                    .words
                    .selected
                    .replace("{count}", &self.state.selected.len().to_string());
                ui.label(TextRole::Label.rich(&t, words).color(t.color("foreground")));
                if ui
                    .add(
                        Button::new(self.words.clear.clone())
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Sm),
                    )
                    .clicked()
                {
                    self.state.selected.clear();
                }
            });
            ui.add_space(6.0);
        }

        let frame = egui::Frame::new()
            .fill(t.color("card"))
            .stroke(Stroke::new(1.0, t.color("border-strong")))
            .corner_radius(radius);
        let response = frame
            .show(ui, |ui| {
                ui.set_width(width - 2.0);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                // the head
                let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), head_height), Sense::hover());
                let top = CornerRadius { sw: 0, se: 0, ..radius };
                ui.painter()
                    .rect_filled(head, top, t.color_or("table-head-ground", t.color("card")));
                ui.painter().hline(
                    head.x_range(),
                    head.bottom() - 0.5,
                    Stroke::new(1.0, t.color_or("table-head-line", t.color("border"))),
                );
                let upper = t.word("table-head-transform") == Some("uppercase");
                let mut x = head.left() + select_width;
                if self.selectable {
                    let all = !self.rows.is_empty() && self.state.selected.len() == self.rows.len();
                    let some = !self.state.selected.is_empty() && !all;
                    let mut checked = all;
                    let place = Rect::from_center_size(pos2(head.left() + 20.0, head.center().y), vec2(16.0, 16.0));
                    let box_response = put_at(
                        ui,
                        place,
                        Checkbox::new(&mut checked, self.words.select_all.clone())
                            .hide_label()
                            .indeterminate(some),
                    );
                    if box_response.clicked() {
                        self.state.selected = if all {
                            BTreeSet::new()
                        } else {
                            (0..self.rows.len()).collect()
                        };
                    }
                }
                let last = self.columns.len().saturating_sub(1);
                for (index, column) in self.columns.iter().enumerate() {
                    let w = if index == last {
                        (head.right() - x).max(64.0)
                    } else {
                        self.state.widths[index]
                    };
                    let cell = Rect::from_min_size(pos2(x, head.top()), vec2(w, head.height()));
                    let hit = ui.interact(
                        cell,
                        self.id.with(("head", index)),
                        if column.sortable {
                            Sense::click()
                        } else {
                            Sense::hover()
                        },
                    );
                    let sorted = self.state.sort.filter(|(c, _)| *c == index);
                    if column.sortable && hit.clicked() {
                        // ascending, then descending, then as given
                        self.state.sort = match sorted {
                            None => Some((index, true)),
                            Some((_, true)) => Some((index, false)),
                            Some((_, false)) => None,
                        };
                    }
                    let ink = if sorted.is_some() || (column.sortable && hit.hovered()) {
                        t.color("foreground")
                    } else {
                        t.color_or("table-head-ink", t.color("muted-foreground"))
                    };
                    let title = if upper {
                        column.title.to_uppercase()
                    } else {
                        column.title.clone()
                    };
                    let galley = WidgetText::from(TextRole::Head.rich(&t, title)).into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        w - 2.0 * pad - 16.0,
                        TextRole::Head.font(&t),
                    );
                    let text_x = if column.numeric {
                        cell.right() - pad - galley.size().x - if sorted.is_some() { 16.0 } else { 0.0 }
                    } else {
                        cell.left() + pad
                    };
                    let text_w = galley.size().x;
                    ui.painter()
                        .galley(pos2(text_x, cell.center().y - galley.size().y / 2.0), galley, ink);
                    if let Some((_, ascending)) = sorted {
                        let arrow = if ascending { Icon::ArrowUp } else { Icon::ArrowDown };
                        icons::paint(
                            ui.painter(),
                            pos2(text_x + text_w + 10.0, cell.center().y),
                            arrow,
                            14.0,
                            ink,
                        );
                    }
                    // the resize handle on the column's end edge
                    if index != last {
                        let edge =
                            Rect::from_center_size(pos2(cell.right(), cell.center().y), vec2(8.0, cell.height()));
                        let drag = ui.interact(edge, self.id.with(("resize", index)), Sense::drag());
                        if drag.hovered() || drag.dragged() {
                            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                            ui.painter()
                                .vline(cell.right(), cell.y_range(), Stroke::new(2.0, t.color("ring")));
                        }
                        if drag.dragged() {
                            self.state.widths[index] = (self.state.widths[index] + drag.drag_delta().x).max(64.0);
                        }
                        if drag.double_clicked() {
                            self.state.widths[index] = column.width;
                        }
                    }
                    x += w;
                }

                // the rows, only those in view
                if self.rows.is_empty() {
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.add_space(pad);
                        ui.label(
                            TextRole::Body
                                .rich(&t, &self.words.empty)
                                .color(t.color("muted-foreground")),
                        );
                    });
                    ui.add_space(12.0);
                    return;
                }
                let zebra = t.color_or("table-zebra", t.color("card"));
                egui::ScrollArea::vertical()
                    .id_salt(self.id.with("rows"))
                    .max_height(self.height)
                    .auto_shrink([false, true])
                    .show_rows(ui, row_height, order.len(), |ui, range| {
                        for position in range {
                            let index = order[position];
                            let (rect, row) =
                                ui.allocate_exact_size(vec2(ui.available_width(), row_height), Sense::click());
                            if row.clicked() {
                                self.state.opened = if self.state.opened == Some(index) {
                                    None
                                } else {
                                    Some(index)
                                };
                            }
                            let selected = self.state.selected.contains(&index);
                            let opened = self.state.opened == Some(index);
                            let ground = if selected || opened {
                                t.color("secondary")
                            } else if row.hovered() {
                                t.color("hover")
                            } else if position % 2 == 1 {
                                zebra
                            } else {
                                t.color("card")
                            };
                            ui.painter().rect_filled(rect, CornerRadius::ZERO, ground);
                            ui.painter().hline(
                                rect.x_range(),
                                rect.bottom() - 0.5,
                                Stroke::new(1.0, t.color("border")),
                            );
                            if self.selectable {
                                let mut checked = selected;
                                let place =
                                    Rect::from_center_size(pos2(rect.left() + 20.0, rect.center().y), vec2(16.0, 16.0));
                                if put_at(
                                    ui,
                                    place,
                                    Checkbox::new(&mut checked, self.words.select_row.clone()).hide_label(),
                                )
                                .clicked()
                                {
                                    if checked {
                                        self.state.selected.insert(index);
                                    } else {
                                        self.state.selected.remove(&index);
                                    }
                                }
                            }
                            let mut x = rect.left() + select_width;
                            for (c, column) in self.columns.iter().enumerate() {
                                let w = if c == last {
                                    (rect.right() - x).max(64.0)
                                } else {
                                    self.state.widths[c]
                                };
                                let cell = Rect::from_min_size(pos2(x, rect.top()), vec2(w, rect.height()));
                                paint_cell(ui, &t, &self.rows[index][c], cell, pad, column.numeric);
                                x += w;
                            }
                        }
                    });
            })
            .response;
        let count = self.rows.len();
        let words = if count == 1 { &self.words.row } else { &self.words.rows };
        ui.add_space(6.0);
        ui.label(
            TextRole::Help
                .rich(&t, words.replace("{count}", &count.to_string()))
                .color(t.color("muted-foreground")),
        );
        response
    }
}

fn paint_cell(ui: &mut Ui, t: &crate::Tokens, cell: &Cell, rect: Rect, pad: f32, numeric: bool) {
    let room = (rect.width() - 2.0 * pad).max(1.0);
    match cell {
        Cell::Chip(text, tone) => {
            let place = Rect::from_min_size(pos2(rect.left() + pad, rect.center().y - 10.0), vec2(room, 20.0));
            super::chip::paint_at(ui, place.left(), place.center().y, text, *tone);
        }
        Cell::Text(text) | Cell::Mono(text) => {
            let rich = if matches!(cell, Cell::Mono(_)) {
                egui::RichText::new(text).font(egui::FontId::monospace(TextRole::Help.font(t).size))
            } else {
                TextRole::Body.rich(t, text)
            };
            let galley =
                WidgetText::from(rich).into_galley(ui, Some(TextWrapMode::Truncate), room, TextRole::Body.font(t));
            let x = if numeric {
                rect.right() - pad - galley.size().x
            } else {
                rect.left() + pad
            };
            ui.painter().galley(
                pos2(x, rect.center().y - galley.size().y / 2.0),
                galley,
                t.color("foreground"),
            );
        }
    }
}

/// Add `widget` at `rect` in a child of `ui` that leaves `ui`'s own layout where it was, so a cell's
/// content never moves the rows after it.
fn put_at(ui: &mut Ui, rect: Rect, widget: impl egui::Widget) -> Response {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.add(widget)
}
