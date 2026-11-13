//! The one table, after sokkel-ui's `DataTable`: every listing, its rows at the density's row
//! height and only those in view drawn; heads that sort and resize; selection with a bar; one row
//! opened for a detail beside it.

use std::collections::{BTreeSet, HashMap};

use egui::{CornerRadius, CursorIcon, Id, Rect, Response, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use super::table_edit::{self, EditMove, EditWords, Editor, OpenCell};
use crate::{
    AvatarSize, Button, ButtonSize, ButtonVariant, Checkbox, ChipTone, Day, DropdownMenu, DueText, Icon, MenuItem,
    TextRole, icons, tokens,
};

/// Why a draft cannot be saved, or `None` when it can: a column's check on what was typed.
pub type Validate = fn(&str) -> Option<String>;

/// A column: its head, its starting width, and how its cells sit.
#[derive(Clone, Debug)]
pub struct Column {
    pub title: String,
    pub width: f32,
    pub numeric: bool,
    pub sortable: bool,
    /// Takes the room the other columns leave, at least `width`.
    pub fill: bool,
    /// How its cells are edited in place; `None` for a column that is not edited.
    pub edit: Option<Editor>,
    pub validate: Option<Validate>,
}

impl Column {
    pub fn new(title: impl Into<String>, width: f32) -> Self {
        Self {
            title: title.into(),
            width,
            numeric: false,
            sortable: true,
            fill: false,
            edit: None,
            validate: None,
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

    /// The column that takes the room the others leave, as a `minmax(width, 1fr)` track.
    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    /// Edit the column's cells in place with `editor`: a double click, or a press on the active
    /// cell, opens it; Enter saves, Escape drops, Tab saves and opens the next editable cell, and a
    /// press elsewhere saves. A saved value comes back as the table's `edited`.
    pub fn editable(mut self, editor: Editor) -> Self {
        self.edit = Some(editor);
        self
    }

    /// Refuse a draft `validate` finds wrong, saying why at the cell, before it is saved.
    pub fn validate(mut self, validate: Validate) -> Self {
        self.validate = Some(validate);
        self
    }
}

/// What a cell holds.
#[derive(Clone, Debug)]
pub enum Cell {
    Text(String),
    /// Text struck through and muted: a thing done.
    Struck(String),
    /// An id or a code, in the mono face.
    Mono(String),
    /// A state as a chip.
    Chip(String, ChipTone),
    /// A check box whose press the table reports as `toggled`.
    Check(bool),
    /// A person: their avatar and their name.
    Person(String),
    /// A due date, toned by where it falls against `now`; a done task's is quiet.
    Due {
        date: Option<Day>,
        now: Day,
        done: bool,
    },
    Empty,
}

impl Cell {
    /// The text an editor starts from: the text, the person's name, or the day as `YYYY-MM-DD`.
    fn draft(&self) -> String {
        match self {
            Cell::Text(text) | Cell::Struck(text) | Cell::Mono(text) | Cell::Chip(text, _) | Cell::Person(text) => {
                text.clone()
            }
            Cell::Due { date, .. } => date.map(|d| d.to_string()).unwrap_or_default(),
            Cell::Check(_) | Cell::Empty => String::new(),
        }
    }

    fn sort_key(&self) -> String {
        match self {
            Cell::Text(text) | Cell::Struck(text) | Cell::Mono(text) | Cell::Chip(text, _) | Cell::Person(text) => {
                text.clone()
            }
            Cell::Check(on) => u8::from(*on).to_string(),
            Cell::Due { date, .. } => date.map_or("9999".into(), |d| d.to_string()),
            Cell::Empty => String::new(),
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
    /// The active cell of an editing table, by its row's index and its column's.
    pub active: Option<(usize, usize)>,
    /// The table was pressed last: the active cell is marked and takes keys.
    focused: bool,
    open: Option<OpenCell>,
    /// Drafts refused as the focus left them, kept at their cells with why.
    refused: HashMap<(usize, usize), (String, String)>,
    /// A cell to open as the table next draws.
    pending: Option<(usize, usize)>,
}

impl TableState {
    /// A state that starts sorted by `column`, ascending or not.
    pub fn sorted(column: usize, ascending: bool) -> Self {
        Self {
            sort: Some((column, ascending)),
            ..Self::default()
        }
    }

    /// Open the editor of the cell at `row` (its index in the rows given) and `column` as the table
    /// next draws, as a double click on it would.
    pub fn edit(&mut self, row: usize, column: usize) {
        self.pending = Some((row, column));
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
    pub edit: EditWords,
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
            edit: EditWords::default(),
        }
    }
}

/// What the table was asked this frame, by the index of a row in the rows given.
pub struct TableOutput {
    pub response: Response,
    /// A row's check box was pressed.
    pub toggled: Option<usize>,
    /// A row's action was chosen: the row and the action's index in its menu.
    pub action: Option<(usize, usize)>,
    /// A row was dragged by its grip from one place in the order shown to another.
    pub reordered: Option<(usize, usize)>,
    /// A cell's edit was saved: the app stores it.
    pub edited: Option<CellEdit>,
}

/// A saved edit: the row's index in the rows given, the column's, and the value as text (a day as
/// `YYYY-MM-DD`, or empty for none).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellEdit {
    pub row: usize,
    pub column: usize,
    pub value: String,
}

/// What a table draws in place of its rows when it has none.
pub type EmptyContent<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// The menu of actions a row offers, built for each row.
pub type RowActions<'a> = Box<dyn Fn(usize) -> Vec<MenuItem> + 'a>;

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
    reorder: bool,
    actions: Option<RowActions<'a>>,
    empty: Option<EmptyContent<'a>>,
    footer: bool,
    stretch: bool,
    today: Day,
}

/// How wide a row's grip is, in the first column before its content.
const GRIP: f32 = 22.0;
/// The room between two columns.
const GAP: f32 = 12.0;
/// The width of the actions' column at a row's end.
const ACTIONS: f32 = 40.0;

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
            reorder: false,
            actions: None,
            empty: None,
            footer: true,
            stretch: false,
            today: Day::today(),
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

    /// A grip at each row's start that drags the row to another place: the order is the person's.
    pub fn reorder(mut self, reorder: bool) -> Self {
        self.reorder = reorder;
        self
    }

    /// A menu of actions at each row's end, built for the row.
    pub fn actions(mut self, actions: impl Fn(usize) -> Vec<MenuItem> + 'a) -> Self {
        self.actions = Some(Box::new(actions));
        self
    }

    /// What the table shows with no rows, in place of the words' `empty`.
    pub fn empty(mut self, empty: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.empty = Some(Box::new(empty));
        self
    }

    /// Stretch the table's frame to the bottom of the room it is given, its rows scrolling inside,
    /// as a table that fills its page.
    pub fn stretch(mut self, stretch: bool) -> Self {
        self.stretch = stretch;
        self
    }

    /// The day a date editor's calendar marks as today.
    pub fn today(mut self, today: Day) -> Self {
        self.today = today;
        self
    }

    /// The row count under the table; on by default.
    pub fn footer(mut self, footer: bool) -> Self {
        self.footer = footer;
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
                    parse(&x).total_cmp(&parse(&y))
                } else {
                    x.cmp(&y)
                };
                if ascending { ordering } else { ordering.reverse() }
            });
        }
        order
    }

    /// Each column's start and width within a row from `left` to `right`.
    fn layout(&self, left: f32, right: f32) -> Vec<(f32, f32)> {
        let lead = if self.reorder { GRIP } else { 0.0 };
        let widths: Vec<f32> = self
            .columns
            .iter()
            .enumerate()
            .map(|(i, _)| self.state.widths[i] + if i == 0 { lead } else { 0.0 })
            .collect();
        let gaps = GAP * self.columns.len().saturating_sub(1) as f32;
        let tail = if self.actions.is_some() { ACTIONS + GAP } else { 0.0 };
        let fixed: f32 = widths.iter().sum::<f32>() + gaps + tail;
        let fill = self
            .columns
            .iter()
            .position(|c| c.fill)
            .or(self.columns.len().checked_sub(1));
        let mut x = left;
        widths
            .iter()
            .enumerate()
            .map(|(i, w)| {
                let w = if Some(i) == fill {
                    (w + (right - left - fixed)).max(*w)
                } else {
                    *w
                };
                let place = (x, w);
                x += w + GAP;
                place
            })
            .collect()
    }

    /// Draw the table, stacked top to bottom whatever layout the row around it has.
    pub fn show(self, ui: &mut Ui) -> TableOutput {
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| self.show_stacked(ui))
            .inner
    }

    fn show_stacked(mut self, ui: &mut Ui) -> TableOutput {
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
        let select_width = if self.selectable { 28.0 } else { 0.0 };
        if let Some(at) = self.state.pending.take()
            && at.0 < self.rows.len()
            && self.columns.get(at.1).is_some_and(|c| c.edit.is_some())
        {
            open_cell(self.state, self.rows, at);
        }
        let order = self.order();
        let radius = t.radius("surface");
        let width = ui.available_width();
        let room = ui.available_height();
        let mut out = TableOutput {
            response: ui.response(),
            toggled: None,
            action: None,
            reordered: None,
            edited: None,
        };

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
        let drag_id = self.id.with("dragging");
        let response = frame
            .show(ui, |ui| {
                ui.set_width(width - 2.0);
                if self.stretch {
                    // the frame to the room's bottom, less its edge and the footer's line
                    ui.set_min_height(room - 2.0 - if self.footer { 26.0 } else { 0.0 });
                }
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
                let places = self.layout(head.left() + pad + select_width, head.right() - pad);
                if self.selectable {
                    let all = !self.rows.is_empty() && self.state.selected.len() == self.rows.len();
                    let some = !self.state.selected.is_empty() && !all;
                    let mut checked = all;
                    let place = Rect::from_min_size(pos2(head.left() + pad, head.center().y - 8.0), vec2(16.0, 16.0));
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
                    let (x, w) = places[index];
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
                        w - 16.0,
                        TextRole::Head.font(&t),
                    );
                    // a head over a row's grip stands after it
                    let lead = if index == 0 && self.reorder { GRIP + 6.0 } else { 0.0 };
                    let text_x = if column.numeric {
                        cell.right() - galley.size().x - if sorted.is_some() { 16.0 } else { 0.0 }
                    } else {
                        cell.left() + lead
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
                    // the resize handle in the gap after the column
                    if index != last {
                        let edge = Rect::from_center_size(
                            pos2(cell.right() + GAP / 2.0, cell.center().y),
                            vec2(8.0, cell.height()),
                        );
                        let drag = ui.interact(edge, self.id.with(("resize", index)), Sense::drag());
                        if drag.hovered() || drag.dragged() {
                            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                            ui.painter()
                                .vline(edge.center().x, cell.y_range(), Stroke::new(2.0, t.color("ring")));
                        }
                        if drag.dragged() {
                            self.state.widths[index] = (self.state.widths[index] + drag.drag_delta().x).max(64.0);
                        }
                        if drag.double_clicked() {
                            self.state.widths[index] = column.width;
                        }
                    }
                }

                // with no rows, what the table says in their place
                if self.rows.is_empty() {
                    match self.empty.take() {
                        Some(empty) => empty(ui),
                        None => {
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
                        }
                    }
                    return;
                }
                let zebra = t.color_or("table-zebra", t.color("card"));
                let dragging: Option<usize> = ui.ctx().data(|d| d.get_temp(drag_id));
                let mut row_rects: Vec<Rect> = Vec::new();
                egui::ScrollArea::vertical()
                    .id_salt(self.id.with("rows"))
                    .max_height(if self.stretch {
                        (room - head_height - 2.0).max(row_height)
                    } else {
                        self.height
                    })
                    .auto_shrink([false, true])
                    .show_rows(ui, row_height, order.len(), |ui, range| {
                        for position in range {
                            let index = order[position];
                            let (rect, row) =
                                ui.allocate_exact_size(vec2(ui.available_width(), row_height), Sense::click());
                            row_rects.push(rect);
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
                            } else if row.hovered() || dragging == Some(position) {
                                t.color("hover")
                            } else if position % 2 == 1 {
                                zebra
                            } else {
                                t.color("card")
                            };
                            ui.painter().rect_filled(rect, CornerRadius::ZERO, ground);
                            // the active cell's row, while the table has the focus or a typed editor is open
                            let typed_open = self.state.open.as_ref().is_some_and(|o| {
                                o.row == index
                                    && !matches!(
                                        self.columns.get(o.column).and_then(|c| c.edit.as_ref()),
                                        Some(Editor::Select(_))
                                    )
                            });
                            if typed_open
                                || (self.state.focused
                                    && self.state.open.is_none()
                                    && self.state.active.map(|a| a.0) == Some(index))
                            {
                                ui.painter().rect_filled(rect, CornerRadius::ZERO, t.color("hover"));
                            }
                            ui.painter().hline(
                                rect.x_range(),
                                rect.bottom() - 0.5,
                                Stroke::new(1.0, t.color("border")),
                            );
                            if self.selectable {
                                let mut checked = selected;
                                let place = Rect::from_min_size(
                                    pos2(rect.left() + pad, rect.center().y - 8.0),
                                    vec2(16.0, 16.0),
                                );
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
                            let places = self.layout(rect.left() + pad + select_width, rect.right() - pad);
                            for (c, column) in self.columns.iter().enumerate() {
                                let (x, w) = places[c];
                                let mut cell = Rect::from_min_size(pos2(x, rect.top()), vec2(w, rect.height()));
                                if c == 0 && self.reorder {
                                    // the grip: drag it to move the row
                                    let grip =
                                        Rect::from_min_size(pos2(x - 4.0, rect.top()), vec2(GRIP, rect.height()));
                                    let hit = ui.interact(grip, self.id.with(("grip", index)), Sense::drag());
                                    hit.widget_info(|| {
                                        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Move")
                                    });
                                    if hit.hovered() || hit.dragged() {
                                        ui.ctx().set_cursor_icon(if hit.dragged() {
                                            CursorIcon::Grabbing
                                        } else {
                                            CursorIcon::Grab
                                        });
                                    }
                                    if hit.drag_started() {
                                        ui.ctx().data_mut(|d| d.insert_temp(drag_id, position));
                                    }
                                    icons::paint(
                                        ui.painter(),
                                        pos2(x + 6.0, rect.center().y),
                                        Icon::GripVertical,
                                        14.0,
                                        t.color("muted-foreground"),
                                    );
                                    cell.min.x += GRIP;
                                }
                                if let Some(editor) = &column.edit {
                                    // an editable cell: its editor while open, else its content, opening on a press
                                    let place = Rect::from_min_max(cell.min, pos2(cell.right(), rect.bottom() - 1.0));
                                    let here = (index, c);
                                    let open_here = self.state.open.as_ref().is_some_and(|o| (o.row, o.column) == here);
                                    if open_here && let Some(mut open) = self.state.open.take() {
                                        let id = self.id.with(("editor", index, c));
                                        let moved = match editor {
                                            Editor::Text => table_edit::text(ui, place, &mut open, id),
                                            Editor::Date(week_start) => table_edit::date(
                                                ui,
                                                place,
                                                &mut open,
                                                id,
                                                *week_start,
                                                self.today,
                                                &self.words.edit,
                                            ),
                                            Editor::Select(options) => {
                                                table_edit::select(ui, place, &mut open, id, options)
                                            }
                                        };
                                        if let Some(edit) = settle(
                                            self.state,
                                            self.columns,
                                            self.rows,
                                            &order,
                                            position,
                                            open,
                                            moved,
                                            &self.words.edit,
                                        ) {
                                            out.edited = Some(edit);
                                        }
                                    } else {
                                        let hit = ui.interact(place, self.id.with(("edit", index, c)), Sense::click());
                                        match self.state.refused.get(&here) {
                                            Some((draft, error)) => table_edit::refused(ui, place, draft, error),
                                            None => {
                                                paint_cell(
                                                    ui,
                                                    &t,
                                                    &self.rows[index][c],
                                                    cell,
                                                    column.numeric,
                                                    self.id.with(("cell", index, c)),
                                                );
                                            }
                                        }
                                        let was_active = self.state.focused && self.state.active == Some(here);
                                        if hit.double_clicked() || (hit.clicked() && was_active) {
                                            open_cell(self.state, self.rows, here);
                                        } else if hit.clicked() {
                                            self.state.active = Some(here);
                                            self.state.focused = true;
                                        }
                                        if self.state.focused && self.state.active == Some(here) {
                                            ui.painter().rect_stroke(
                                                place,
                                                CornerRadius::ZERO,
                                                Stroke::new(2.0, t.color("ring")),
                                                egui::StrokeKind::Inside,
                                            );
                                        }
                                    }
                                } else if let Some(toggled) = paint_cell(
                                    ui,
                                    &t,
                                    &self.rows[index][c],
                                    cell,
                                    column.numeric,
                                    self.id.with(("cell", index, c)),
                                ) {
                                    let _ = toggled;
                                    out.toggled = Some(index);
                                }
                            }
                            if let Some(actions) = &self.actions {
                                let place = Rect::from_min_size(
                                    pos2(rect.right() - pad - ACTIONS, rect.top()),
                                    vec2(ACTIONS, rect.height()),
                                );
                                let button = Rect::from_center_size(place.center(), vec2(28.0, 28.0));
                                let trigger = ui.interact(button, self.id.with(("actions", index)), Sense::click());
                                trigger.widget_info(|| {
                                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Actions")
                                });
                                if trigger.hovered() {
                                    ui.painter().rect_filled(button, t.radius("control"), t.color("hover"));
                                }
                                icons::paint(
                                    ui.painter(),
                                    button.center(),
                                    Icon::Ellipsis,
                                    16.0,
                                    t.color("muted-foreground"),
                                );
                                let items = actions(index);
                                if let Some(chosen) = DropdownMenu::new(&items).show(&trigger) {
                                    out.action = Some((index, chosen));
                                }
                            }
                        }
                    });
                // a dragged row: the line where it lands, and its move on release
                if let Some(from) = dragging {
                    let pointer = ui.ctx().pointer_latest_pos();
                    let released = ui.input(|i| i.pointer.any_released());
                    if let (Some(pointer), Some(first)) = (pointer, row_rects.first()) {
                        let to = (((pointer.y - first.top()) / row_height).round().max(0.0) as usize).min(order.len());
                        let y = first.top() + to as f32 * row_height;
                        ui.painter()
                            .hline(first.x_range(), y, Stroke::new(2.0, t.color("primary")));
                        if released {
                            let to = if to > from { to - 1 } else { to };
                            if to != from {
                                out.reordered = Some((from, to));
                            }
                        }
                    }
                    if released {
                        ui.ctx().data_mut(|d| d.remove::<usize>(drag_id));
                    }
                }
            })
            .response;
        // the active cell's keys while the table has the focus: the arrows move it, Enter or F2 opens it
        if self.state.focused
            && self.state.open.is_none()
            && let Some((row, column)) = self.state.active
        {
            let position = order.iter().position(|r| *r == row).unwrap_or(0);
            let (enter, left, right, up, down) = ui.input_mut(|i| {
                let none = egui::Modifiers::NONE;
                (
                    i.consume_key(none, egui::Key::Enter) || i.consume_key(none, egui::Key::F2),
                    i.consume_key(none, egui::Key::ArrowLeft),
                    i.consume_key(none, egui::Key::ArrowRight),
                    i.consume_key(none, egui::Key::ArrowUp),
                    i.consume_key(none, egui::Key::ArrowDown),
                )
            });
            let last = self.columns.len().saturating_sub(1);
            if enter && self.columns.get(column).is_some_and(|c| c.edit.is_some()) {
                open_cell(self.state, self.rows, (row, column));
            } else if left {
                self.state.active = Some((row, column.saturating_sub(1)));
            } else if right {
                self.state.active = Some((row, (column + 1).min(last)));
            } else if up {
                self.state.active = Some((order[position.saturating_sub(1)], column));
            } else if down {
                self.state.active = Some((order[(position + 1).min(order.len().saturating_sub(1))], column));
            }
        }
        // a press outside the table takes its focus away
        if ui.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !response.rect.contains(p)))
            && self.state.open.is_none()
        {
            self.state.focused = false;
        }
        if self.footer {
            let count = self.rows.len();
            let words = if count == 1 { &self.words.row } else { &self.words.rows };
            ui.add_space(6.0);
            ui.label(
                TextRole::Help
                    .rich(&t, words.replace("{count}", &count.to_string()))
                    .color(t.color("muted-foreground")),
            );
        }
        out.response = response;
        out
    }
}

/// Draw one cell's content in `rect`; true when its check box was pressed.
fn paint_cell(ui: &mut Ui, t: &crate::Tokens, cell: &Cell, rect: Rect, numeric: bool, id: Id) -> Option<bool> {
    let room = rect.width().max(1.0);
    let text = |ui: &mut Ui, rich: egui::RichText, ink: egui::Color32| {
        let galley = WidgetText::from(rich).into_galley(ui, Some(TextWrapMode::Truncate), room, TextRole::Body.font(t));
        let x = if numeric {
            rect.right() - galley.size().x
        } else {
            rect.left()
        };
        ui.painter()
            .galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley, ink);
    };
    match cell {
        Cell::Chip(label, tone) => {
            super::chip::paint_at(ui, rect.left(), rect.center().y, label, *tone);
        }
        Cell::Text(label) => text(ui, TextRole::Body.rich(t, label), t.color("foreground")),
        Cell::Struck(label) => text(
            ui,
            TextRole::Body.rich(t, label).strikethrough(),
            t.color("muted-foreground"),
        ),
        Cell::Mono(label) => {
            let rich = egui::RichText::new(label).font(egui::FontId::monospace(TextRole::Help.font(t).size));
            text(ui, rich, t.color("foreground"));
        }
        Cell::Due { date, now, done } => {
            let (rich, ink) = DueText::new(*date, *now).done(*done).rich(t);
            text(ui, rich, ink);
        }
        Cell::Person(name) => {
            let avatar = Rect::from_min_size(pos2(rect.left(), rect.center().y - 8.0), vec2(16.0, 16.0));
            super::person::paint_avatar(ui, t, avatar, name, AvatarSize::Xxs, false);
            let galley = WidgetText::from(TextRole::Body.rich(t, name)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                room - 20.0,
                TextRole::Body.font(t),
            );
            ui.painter().galley(
                pos2(avatar.right() + 4.0, rect.center().y - galley.size().y / 2.0),
                galley,
                t.color("foreground"),
            );
        }
        Cell::Check(on) => {
            let mut checked = *on;
            let place = Rect::from_min_size(pos2(rect.left(), rect.center().y - 8.0), vec2(16.0, 16.0));
            if put_at(ui, place, Checkbox::new(&mut checked, "Done").hide_label()).clicked() {
                return Some(checked);
            }
            let _ = id;
        }
        Cell::Empty => {}
    }
    None
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

/// Open the editor of the cell `at`: on its refused draft if it has one, else on its value.
fn open_cell(state: &mut TableState, rows: &[Vec<Cell>], at: (usize, usize)) {
    let (draft, error) = match state.refused.get(&at) {
        Some((draft, error)) => (draft.clone(), Some(error.clone())),
        None => (rows[at.0][at.1].draft(), None),
    };
    state.active = Some(at);
    state.focused = true;
    state.open = Some(OpenCell {
        row: at.0,
        column: at.1,
        draft,
        error,
        fresh: true,
        calendar: false,
        highlight: 0,
    });
}

/// What an open editor's move does: keep it open, drop it, or save its draft (unchanged, refused or
/// saved) and close it or open the next editable cell. A saved draft is returned for the app.
#[allow(clippy::too_many_arguments)]
fn settle(
    state: &mut TableState,
    columns: &[Column],
    rows: &[Vec<Cell>],
    order: &[usize],
    position: usize,
    mut open: OpenCell,
    moved: EditMove,
    words: &EditWords,
) -> Option<CellEdit> {
    let at = (open.row, open.column);
    match moved {
        EditMove::Stay => {
            state.open = Some(open);
            return None;
        }
        EditMove::Cancel => {
            state.refused.remove(&at);
            return None;
        }
        _ => {}
    }
    let column = &columns[open.column];
    let unchanged = open.draft == rows[open.row][open.column].draft();
    let refusal = if unchanged {
        None
    } else if matches!(column.edit, Some(Editor::Date(_)))
        && !open.draft.trim().is_empty()
        && Day::parse(open.draft.trim()).is_none()
    {
        Some(words.unreadable_date.clone())
    } else {
        column.validate.and_then(|validate| validate(&open.draft))
    };
    if let Some(error) = refusal {
        if moved == EditMove::Leave {
            // left refused: the draft stays at its cell, marked, until it is opened again
            state.refused.insert(at, (open.draft, error));
        } else {
            open.error = Some(error);
            state.open = Some(open);
        }
        return None;
    }
    state.refused.remove(&at);
    let edit = (!unchanged).then(|| CellEdit {
        row: open.row,
        column: open.column,
        value: if matches!(column.edit, Some(Editor::Date(_))) {
            open.draft.trim().to_owned()
        } else {
            open.draft.clone()
        },
    });
    if moved == EditMove::Leave {
        state.focused = false;
    }
    if let EditMove::Next(forward) = moved
        && let Some(next) = next_editable(columns, order, position, open.column, forward)
    {
        open_cell(state, rows, next);
    }
    edit
}

/// The editable cell after (or before) the column `column` of the row at `position` in the order
/// shown: along the row, then on to the next row's first.
fn next_editable(
    columns: &[Column],
    order: &[usize],
    position: usize,
    column: usize,
    forward: bool,
) -> Option<(usize, usize)> {
    let editable: Vec<usize> = (0..columns.len()).filter(|c| columns[*c].edit.is_some()).collect();
    let cells: Vec<(usize, usize)> = order
        .iter()
        .enumerate()
        .flat_map(|(p, _)| editable.iter().map(move |c| (p, *c)))
        .collect();
    let here = cells.iter().position(|cell| *cell == (position, column))?;
    let next = if forward {
        here.checked_add(1)?
    } else {
        here.checked_sub(1)?
    };
    cells.get(next).map(|(p, c)| (order[*p], *c))
}
