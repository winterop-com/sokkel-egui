//! The one table, after sokkel-ui's `DataTable`: every listing, its rows at the density's row
//! height and only those in view drawn; heads that sort and resize; selection with a bar; one row
//! opened for a detail beside it.

use std::collections::{BTreeSet, HashMap};

use egui::{CornerRadius, CursorIcon, Id, Rect, Response, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use super::table_edit::{self, EditMove, EditWords, Editor, OpenCell};
use crate::{
    AvatarSize, Button, ButtonSize, ButtonVariant, Checkbox, ChipTone, Day, DropdownMenu, DueText, Icon, MenuItem,
    SearchField, TextRole, icons, tokens,
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
    /// Its share of that room beside the other filling columns', as a track's `fr`; 1 when not
    /// given.
    pub share: f32,
    /// How its cells are edited in place; `None` for a column that is not edited.
    pub edit: Option<Editor>,
    pub validate: Option<Validate>,
    /// Where its cells go when the rows stack.
    pub stack: StackPlace,
}

/// Where a column's cells go in a stacked row, as sokkel-ui's column `stack`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackPlace {
    /// Before the title, such as a box that marks a row done.
    Lead,
    /// A line of its own; in the compact layout, one of the quiet facts under the title.
    #[default]
    Line,
    /// One of the quiet facts on a stacked row's last line, in the labeled layout too.
    Secondary,
    /// A value of its own under the title, in the body's type, with no name before it: what a
    /// row is ("The app people sign in to").
    Primary,
    /// Left out of a stacked row.
    None,
}

/// How a narrow table stacks its rows, as sokkel-ui's `stackLayout`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StackLayout {
    /// The rows stay rows however narrow the table is.
    #[default]
    Never,
    /// The title over one quiet line of the other columns, for short things on a phone.
    Compact,
    /// A card per row: the title, a labeled line for each `Line` column, and the `Secondary`
    /// columns as one quiet line of facts.
    Labeled,
}

impl Column {
    pub fn new(title: impl Into<String>, width: f32) -> Self {
        Self {
            title: title.into(),
            width,
            numeric: false,
            sortable: true,
            fill: false,
            share: 1.0,
            edit: None,
            validate: None,
            stack: StackPlace::Line,
        }
    }

    /// Where the column's cells go when the rows stack: `StackPlace::Lead` before the title.
    pub fn stack(mut self, stack: StackPlace) -> Self {
        self.stack = stack;
        self
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

    /// A filling column taking `share` parts of the room left, as a `minmax(width, <share>fr)`
    /// track: `fill_share(2.0)` beside a `fill()` takes twice its room.
    pub fn fill_share(mut self, share: f32) -> Self {
        self.fill = true;
        self.share = share.max(0.0);
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
    /// Text at medium weight: a row's name, as a `font-medium` cell.
    Strong(String),
    /// Text struck through and muted: a thing done.
    Struck(String),
    /// Text in the muted ink: a quieter fact beside the row's own, as a `text-muted-foreground` cell.
    Muted(String),
    /// Text in a token's ink, such as `critical-ink` for a fall: a change written in its tone.
    Toned(String, &'static str),
    /// An id or a code, in the mono face.
    Mono(String),
    /// A state as a chip.
    Chip(String, ChipTone),
    /// A check box whose press the table reports as `toggled`.
    Check(bool),
    /// A switch named `label` for a screen reader, whose press the table reports as `switched`, as a
    /// rule's On column.
    Switch {
        on: bool,
        label: String,
    },
    /// A moment as its clock in the code face, then 8 on how long ago it was in the short words
    /// (`3 hr. ago`), muted, sorted by the moment; `at` and `now` in milliseconds of one clock.
    Clock {
        clock: String,
        at: f64,
        now: f64,
    },
    /// A person: their avatar and their name.
    Person(String),
    /// A due date, toned by where it falls against `now`; a done task's is quiet.
    Due {
        date: Option<Day>,
        now: Day,
        done: bool,
    },
    /// Tags as chips in the code face, 4 apart, after sokkel-ui's `TagChips`.
    Tags(Vec<String>),
    /// An instant said against `now` in words ("48 minutes ago"), sorted by the instant; both in
    /// milliseconds of one clock.
    Relative {
        at: f64,
        now: f64,
    },
    /// Values as a sparkline 112 by 28, in `ink`.
    Trend(Vec<f64>, egui::Color32),
    /// A code in the mono face after a dot in `dot`'s color, as a row's name with sokkel-ui's
    /// `Dot` before it (its threshold's color); no dot when `None`.
    Marked {
        text: String,
        dot: Option<egui::Color32>,
    },
    /// A series' name after its swatch, a square of 10 in `color`, as a chart legend's entry.
    Swatch {
        text: String,
        color: egui::Color32,
    },
    /// A figure written as `text` and sorted by `value`, as a column's `sortValue` sorts by the
    /// number rather than the words: `144 ms`.
    Number {
        value: f64,
        text: String,
    },
    Empty,
}

impl Cell {
    /// The text an editor starts from: the text, the person's name, or the day as `YYYY-MM-DD`.
    fn draft(&self) -> String {
        match self {
            Cell::Text(text)
            | Cell::Strong(text)
            | Cell::Struck(text)
            | Cell::Muted(text)
            | Cell::Toned(text, _)
            | Cell::Mono(text)
            | Cell::Chip(text, _)
            | Cell::Person(text) => text.clone(),
            Cell::Number { text, .. } | Cell::Marked { text, .. } | Cell::Swatch { text, .. } => text.clone(),
            Cell::Due { date, .. } => date.map(|d| d.to_string()).unwrap_or_default(),
            Cell::Tags(tags) => tags.join(" "),
            Cell::Relative { at, now } => super::relative_time(at - now),
            Cell::Clock { clock, .. } => clock.clone(),
            Cell::Check(_) | Cell::Switch { .. } | Cell::Trend(..) | Cell::Empty => String::new(),
        }
    }

    fn sort_key(&self) -> String {
        match self {
            Cell::Text(text)
            | Cell::Strong(text)
            | Cell::Struck(text)
            | Cell::Muted(text)
            | Cell::Toned(text, _)
            | Cell::Mono(text)
            | Cell::Chip(text, _)
            | Cell::Person(text) => text.clone(),
            Cell::Check(on) | Cell::Switch { on, .. } => u8::from(*on).to_string(),
            Cell::Clock { at, .. } => format!("{:024.3}", at + 1e18),
            Cell::Due { date, .. } => date.map_or("9999".into(), |d| d.to_string()),
            Cell::Tags(tags) => tags.join(" "),
            Cell::Relative { at, .. } => format!("{:024.3}", at + 1e18),
            Cell::Trend(values, _) => format!("{:024.3}", values.last().copied().unwrap_or(0.0) + 1e18),
            // lifted by 1e12 so a negative figure still sorts as a word, small enough that an f64
            // read back keeps every digit
            Cell::Number { value, .. } => format!("{:024.6}", value + 1e12),
            Cell::Marked { text, .. } | Cell::Swatch { text, .. } => text.clone(),
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
    /// The active cell's row by its key, so it follows that row when the rows move.
    active_key: Option<Id>,
    /// The table was pressed last: the active cell is marked and takes keys.
    focused: bool,
    open: Option<OpenCell>,
    /// Drafts refused as the focus left them, kept at their cells with why.
    refused: HashMap<(Id, usize), (String, String)>,
    /// A cell to open as the table next draws.
    pending: Option<(usize, usize)>,
    /// The head's column the keys stand on, reached by Up from the first row, where they sort.
    head: Option<usize>,
    /// The columns the reader hid from the bar's Columns menu, by their index.
    pub hidden: BTreeSet<usize>,
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
    /// The bar's sort button while the rows stand in their own order.
    pub sort: String,
    /// The bar's menu of the columns to show, and its button's label.
    pub columns: String,
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
            sort: "Sort".into(),
            columns: "Columns".into(),
            edit: EditWords::default(),
        }
    }
}

/// What the table was asked this frame, by the index of a row in the rows given.
pub struct TableOutput {
    pub response: Response,
    /// A row's check box was pressed.
    pub toggled: Option<usize>,
    /// A row's switch was pressed.
    pub switched: Option<usize>,
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

/// Where an editor's row stands this frame: the rows' keys, the order shown, and its place in it.
struct Editing<'k> {
    keys: &'k [Id],
    order: &'k [usize],
    position: usize,
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
    framed: bool,
    today: Day,
    keys: Vec<Id>,
    stack_layout: StackLayout,
    /// The width below which the rows stack, in place of 480 and the columns' own.
    stack_below: Option<f32>,
    search: Option<&'a mut String>,
    bar: bool,
}

/// How wide a row's grip is, in the first column before its content.
const GRIP: f32 = 22.0;
/// How many lines a primary value takes in a stacked card, as the web's `stackLines` default.
const PRIMARY_LINES: f32 = 2.0;
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
            framed: true,
            today: Day::today(),
            keys: Vec::new(),
            stack_layout: StackLayout::Never,
            stack_below: None,
            search: None,
            bar: false,
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
    /// Draws the table's own border and corners; off for a table set into a frame of its own,
    /// such as a flush panel.
    pub fn framed(mut self, framed: bool) -> Self {
        self.framed = framed;
        self
    }

    pub fn stretch(mut self, stretch: bool) -> Self {
        self.stretch = stretch;
        self
    }

    /// The day a date editor's calendar marks as today.
    /// Name each row by what it holds (a task's id), in the order of the rows given, as
    /// sokkel-ui's `rowKey`: the active cell, an open editor and a refused draft stay with their
    /// row when the rows move, and go when it does. Without keys a row is known by its index.
    pub fn keys<K: egui::AsId>(mut self, keys: impl IntoIterator<Item = K>) -> Self {
        self.keys = keys.into_iter().map(Id::new).collect();
        self
    }

    /// A bar over the rows with a search field (reached by "/") that edits `text`, the rows' count
    /// and a sort menu of the columns that sort; the app narrows the rows by what is typed.
    pub fn search(mut self, text: &'a mut String) -> Self {
        self.search = Some(text);
        self.bar = true;
        self
    }

    /// The bar over the rows without a search: the rows' count and the sort menu.
    pub fn bar(mut self, bar: bool) -> Self {
        self.bar = bar;
        self
    }

    /// Stack the rows as cards where a list's columns no longer fit, or it is narrower than 480:
    /// `StackLayout::Compact` draws each as its leads and title over one quiet line of the rest.
    pub fn stack_layout(mut self, layout: StackLayout) -> Self {
        self.stack_layout = layout;
        self
    }

    /// Stack the rows below `width` alone, the columns shrinking to fit above it, as the web's
    /// `stackBelow` (a table in a half-width panel, `stackBelow={420}`).
    pub fn stack_below(mut self, width: f32) -> Self {
        self.stack_below = Some(width);
        self
    }

    /// Whether the rows stack at `width`: a table below its `stack_below`; a list, or a table naming
    /// none, below `stack_below` (480 when not given) or once the columns no longer fit, whichever is
    /// wider, so it never scrolls sideways.
    fn stacks(&self, width: f32, pad: f32, select_width: f32) -> bool {
        // a compact list stacks; labeled cards stack in either look, as a `minWidth="auto"` table
        let stacks = match self.stack_layout {
            StackLayout::Never => false,
            StackLayout::Compact => self.variant == TableVariant::List,
            StackLayout::Labeled => true,
        };
        if !stacks {
            return false;
        }
        // a table names the width it stacks below; a list (`minWidth="auto"` on the web) stacks there
        // or once its columns no longer fit, whichever is wider
        if let Some(below) = self.stack_below
            && self.variant != TableVariant::List
        {
            return width < below;
        }
        let columns: f32 = self.columns.iter().map(|c| c.width).sum::<f32>()
            + GAP * self.columns.len().saturating_sub(1) as f32
            + 2.0 * pad
            + select_width
            + if self.reorder { GRIP } else { 0.0 }
            + if self.actions.is_some() { ACTIONS } else { 0.0 };
        // a table whose columns shrink to fit (`minWidth="auto"`) stacks below `stack_below` (480,
        // as the web's `stackDefault`) or once its columns no longer fit, whichever is wider
        width < columns.max(self.stack_below.unwrap_or(480.0))
    }

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
    /// Draw the open editor at `place` and settle what it was asked: the saved edit, if any.
    fn run_editor(
        &mut self,
        ui: &mut Ui,
        editor: &Editor,
        place: Rect,
        id: Id,
        editing: &Editing<'_>,
    ) -> Option<CellEdit> {
        let mut open = self.state.open.take()?;
        let moved = match editor {
            Editor::Text => table_edit::text(ui, place, &mut open, id),
            Editor::Date(week_start) => {
                table_edit::date(ui, place, &mut open, id, *week_start, self.today, &self.words.edit)
            }
            Editor::Select(options) => table_edit::select(ui, place, &mut open, id, options),
        };
        settle(
            self.state,
            self.columns,
            self.rows,
            editing.keys,
            editing.order,
            editing.position,
            open,
            moved,
            &self.words.edit,
        )
    }

    /// A row stacked in the compact layout: its grip, its leads and its title on a 24 line, and
    /// under the title one 20 line of the other columns' values, small and quiet, a faint dot
    /// between them. True when a lead's box was pressed.
    fn stacked_row(
        &mut self,
        ui: &mut Ui,
        t: &crate::Tokens,
        rect: Rect,
        index: usize,
        (pad, lead): (f32, f32),
        editing: &Editing<'_>,
    ) -> (bool, Option<CellEdit>) {
        let mut toggled = false;
        let mut edited = None;
        // the column whose editor is open on this row
        let open = self.state.open.as_ref().filter(|o| o.row == index).map(|o| o.column);
        let table_id = self.id;
        let editor_id = |c: usize| table_id.with(("editor", editing.keys[index], c));
        // `lead`: the room a select box takes at the start
        let title = self.columns.iter().position(|c| c.stack != StackPlace::Lead);
        // the title's line, a control tall while its field is open
        let line_height = if open.is_some() && open == title {
            t.density("control")
        } else {
            24.0
        };
        let line = Rect::from_min_size(
            pos2(rect.left() + pad + lead, rect.top() + 8.0),
            vec2(rect.width(), line_height),
        );
        let mut x = line.left();
        if self.reorder {
            let grip = Rect::from_min_size(pos2(x, line.top()), vec2(GRIP, line.height()));
            let hit = ui.interact(grip, self.id.with(("grip", index)), Sense::drag());
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Move"));
            if hit.hovered() || hit.dragged() {
                ui.ctx().set_cursor_icon(if hit.dragged() {
                    CursorIcon::Grabbing
                } else {
                    CursorIcon::Grab
                });
            }
            if hit.drag_started()
                && let Some(position) = self.order().iter().position(|r| *r == index)
            {
                ui.ctx().data_mut(|d| d.insert_temp(self.id.with("dragging"), position));
            }
            icons::paint(
                ui.painter(),
                pos2(x + 5.0, line.center().y),
                Icon::GripVertical,
                14.0,
                t.color("muted-foreground"),
            );
            x += GRIP + 8.0;
        }
        let rows = self.rows;
        let row = &rows[index];
        for (c, column) in self.columns.iter().enumerate() {
            if column.stack == StackPlace::Lead {
                let place = Rect::from_min_size(pos2(x, line.top()), vec2(16.0, line.height()));
                if paint_cell(ui, t, &row[c], place, false, self.id.with(("cell", index, c))).is_some() {
                    toggled = true;
                }
                x += 16.0 + 8.0;
            }
        }
        // the title runs to 8 before the menu's 24 box at the row's end
        let end = rect.right() - pad - if self.actions.is_some() { 24.0 + 8.0 } else { 0.0 };
        let columns = self.columns;
        if let Some(c) = title {
            let place = Rect::from_min_max(pos2(x, line.top()), pos2(end, line.bottom()));
            match (&columns[c].edit, open == Some(c)) {
                // the open editor from the title's line down, a control tall, a finger tall on a
                // phone, over the facts it hides
                (Some(editor), true) => {
                    let tall = if super::is_phone(ui.ctx()) {
                        t.length("spacing-finger")
                    } else {
                        t.density("control")
                    };
                    let field = Rect::from_min_size(place.min, vec2(place.width(), tall));
                    if let Some(open) = &mut self.state.open {
                        open.boxed = true;
                    }
                    edited = self.run_editor(ui, editor, field, editor_id(c), editing);
                }
                (edit, _) => {
                    paint_cell(ui, t, &row[c], place, false, self.id.with(("cell", index, c)));
                    if edit.is_some() {
                        // named by what it says, as the web's title button is
                        let hit = ui.interact(place, self.id.with(("edit", index, c)), Sense::click());
                        let words = row[c].draft();
                        hit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &words));
                        if hit.clicked() {
                            open_cell(self.state, rows, editing.keys, (index, c));
                        }
                    }
                }
            }
        }
        // the facts: every other column but those left out, on one quiet line
        let mut fx = x;
        let facts_height = if open.is_some() && open != title {
            t.density("control")
        } else {
            20.0
        };
        let facts = Rect::from_min_size(pos2(x, line.bottom()), vec2(end - x, facts_height));
        // a fact's open editor takes the line's place, after its column's name
        if let Some(c) = open.filter(|c| Some(*c) != title)
            && let Some(editor) = &columns[c].edit
        {
            let name = WidgetText::from(TextRole::Help.rich(t, &columns[c].title)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                96.0,
                TextRole::Help.font(t),
            );
            let start = fx + name.size().x + 8.0;
            ui.painter().galley(
                pos2(fx, facts.center().y - name.size().y / 2.0),
                name,
                t.color("muted-foreground"),
            );
            let field = Rect::from_min_max(
                pos2(start, facts.center().y - t.density("control") / 2.0),
                pos2(facts.right(), facts.center().y + t.density("control") / 2.0),
            );
            if let Some(open) = &mut self.state.open {
                open.boxed = true;
            }
            edited = self.run_editor(ui, editor, field, editor_id(c), editing);
            return (toggled, edited);
        }
        // the title's open field covers the facts
        if open.is_some() && open == title {
            return (toggled, edited);
        }
        let mut first = true;
        for (c, column) in columns.iter().enumerate() {
            if Some(c) == title
                || !matches!(column.stack, StackPlace::Line | StackPlace::Secondary)
                || matches!(row[c], Cell::Empty | Cell::Check(_))
            {
                continue;
            }
            if !first {
                let dot = WidgetText::from(TextRole::Help.rich(t, "·")).into_galley(
                    ui,
                    None,
                    f32::INFINITY,
                    TextRole::Help.font(t),
                );
                fx += 6.0;
                ui.painter().galley(
                    pos2(fx, facts.center().y - dot.size().y / 2.0),
                    dot.clone(),
                    t.color("faint"),
                );
                fx += dot.size().x + 6.0;
            }
            first = false;
            let width = paint_fact(
                ui,
                t,
                &row[c],
                pos2(fx, facts.center().y),
                (facts.right() - fx).max(1.0),
            );
            // a press on an editable fact opens its editor
            if column.edit.is_some() {
                let place = Rect::from_min_size(pos2(fx, facts.top()), vec2(width, facts.height()));
                let hit = ui.interact(place, self.id.with(("edit", index, c)), Sense::click());
                // named by what it says, as the web's value button is
                let words = row[c].draft();
                hit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &words));
                if hit.clicked() {
                    open_cell(self.state, rows, editing.keys, (index, c));
                }
            }
            fx += width;
        }
        (toggled, edited)
    }

    /// The bar over the rows: the search, 256 wide, then at the end the rows' count in the code
    /// face and, over columns, the Columns menu that hides every column but the first; over
    /// stacked rows the sort, a ghost button naming the column it sorts by, its menu the columns
    /// that sort. 8 over and under a control, 12 across, a rule under it.
    fn draw_bar(&mut self, ui: &mut Ui, t: &crate::Tokens, stacked: bool) {
        // a control tall with its search, a small control without
        let control = super::finger_tall(
            ui.ctx(),
            t,
            t.density(if self.search.is_some() { "control" } else { "control-sm" }),
        );
        let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 8.0 + control + 8.0 + 1.0), Sense::hover());
        ui.painter()
            .hline(bar.x_range(), bar.bottom() - 0.5, Stroke::new(1.0, t.color("border")));
        let row = Rect::from_min_max(
            pos2(bar.left() + 12.0, bar.top() + 8.0),
            pos2(bar.right() - 12.0, bar.top() + 8.0 + control),
        );
        let id = self.id.with("search");
        if let Some(text) = self.search.as_deref_mut() {
            let field = Rect::from_min_size(row.min, vec2(256.0_f32.min(row.width()), control));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field));
            SearchField::new(id, text).keys("/").show(&mut child);
        }
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        child.spacing_mut().item_spacing.x = 16.0;
        if !stacked && self.columns.len() > 1 {
            let trigger = child.add(super::IconButton::new(Icon::Columns3, &self.words.columns));
            egui::Popup::menu(&trigger)
                .id(self.id.with("columns"))
                .align(egui::RectAlign::BOTTOM_END)
                .frame(super::popup_frame(t).inner_margin(egui::Margin::same(4)))
                .gap(4.0)
                .width(256.0)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    ui.set_width(248.0);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add_space(6.0);
                        ui.label(
                            TextRole::Help
                                .rich(t, &self.words.columns)
                                .variation("wght", 500.0)
                                .color(t.color("muted-foreground")),
                        );
                    });
                    ui.add_space(4.0);
                    for c in 1..self.columns.len() {
                        let place = ui.allocate_space(vec2(248.0, 32.0)).1;
                        if ui.rect_contains_pointer(place) {
                            ui.painter().rect_filled(place, t.radius("control"), t.color("hover"));
                        }
                        let mut shown = !self.state.hidden.contains(&c);
                        let mut row = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(place.shrink2(vec2(6.0, 0.0)))
                                .layout(egui::Layout::left_to_right(egui::Align::Center)),
                        );
                        if row
                            .add(super::Checkbox::new(&mut shown, &self.columns[c].title))
                            .changed()
                        {
                            if shown {
                                self.state.hidden.remove(&c);
                            } else {
                                self.state.hidden.insert(c);
                            }
                        }
                    }
                });
        }
        let sortable: Vec<usize> = (0..self.columns.len())
            .filter(|c| self.columns[*c].sortable && stacked)
            .collect();
        if let Some(&first) = sortable.first() {
            // the column it sorts by, or "Sort" while the rows stand in their own order
            let (column, ascending) = self.state.sort.unwrap_or((first, true));
            let (icon, label) = match self.state.sort {
                Some((c, true)) => (Icon::ArrowUp, self.columns[c].title.clone()),
                Some((c, false)) => (Icon::ArrowDown, self.columns[c].title.clone()),
                None => (Icon::ArrowUpDown, self.words.sort.clone()),
            };
            let trigger = child.add(
                Button::new(label)
                    .icon(icon)
                    .variant(ButtonVariant::Ghost)
                    .size(ButtonSize::Sm),
            );
            let items: Vec<MenuItem> = sortable
                .iter()
                .map(|c| MenuItem::action(self.columns[*c].title.clone()))
                .collect();
            if let Some(chosen) = DropdownMenu::new(&items).show(&trigger) {
                let picked = sortable[chosen];
                // the column it sorts by turns the other way; another starts ascending
                self.state.sort = Some(if picked == column {
                    (picked, !ascending)
                } else {
                    (picked, true)
                });
            }
        }
        let count = self.rows.len();
        let words = if count == 1 { &self.words.row } else { &self.words.rows };
        let mono = egui::FontId::monospace(TextRole::Help.font(t).size);
        child.label(
            egui::RichText::new(words.replace("{count}", &count.to_string()))
                .font(mono)
                .color(t.color("muted-foreground")),
        );
    }

    /// The rows as labeled cards (`StackLayout::Labeled`), 16 in and 16 apart: each card 12 across
    /// and 8 down, its title on a 24 line with the menu at its end, a 20 line for each `Line`
    /// column with a value, its name before it as wide as the widest name, and the `Secondary`
    /// columns on one 24 line of facts. A card's height follows its lines, so every card is drawn.
    fn labeled_cards(&mut self, ui: &mut Ui, t: &crate::Tokens, order: &[usize], out: &mut TableOutput) {
        let title = self.columns.iter().position(|c| c.stack != StackPlace::Lead);
        let lines: Vec<usize> = (0..self.columns.len())
            .filter(|c| Some(*c) != title && self.columns[*c].stack == StackPlace::Line)
            .collect();
        // the title is the title alone, whatever place its column names
        let facts: Vec<usize> = (0..self.columns.len())
            .filter(|c| Some(*c) != title && self.columns[*c].stack == StackPlace::Secondary)
            .collect();
        let primary: Vec<usize> = (0..self.columns.len())
            .filter(|c| Some(*c) != title && self.columns[*c].stack == StackPlace::Primary)
            .collect();
        // a list's rows stand as a plain divided list, as the web's narrow list, not as cards
        let listed = self.variant == TableVariant::List;
        let (inset, apart) = if listed { (0.0, 0.0) } else { (16.0, 16.0) };
        // a listed row is padded 10 and keeps 4 between its parts, as the web's `py-2.5 gap-1`
        let (pad, gap) = if listed { (10.0, 4.0) } else { (8.0, 0.0) };
        let drag_id = self.id.with("card-drag");
        let dragging: Option<usize> = ui.ctx().data(|d| d.get_temp(drag_id));
        let mut card_rects: Vec<Rect> = Vec::new();
        let name_width = lines
            .iter()
            .map(|c| {
                WidgetText::from(TextRole::Help.rich(t, &self.columns[*c].title))
                    .into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, TextRole::Help.font(t))
                    .size()
                    .x
            })
            .fold(0.0, f32::max)
            .min(192.0);
        let said = |cell: &Cell| !matches!(cell, Cell::Empty) && !matches!(cell, Cell::Tags(tags) if tags.is_empty());
        egui::ScrollArea::vertical()
            .id_salt(self.id.with("cards"))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.add_space(apart);
                for &index in order {
                    let row = &self.rows[index];
                    let shown: Vec<usize> = lines.iter().copied().filter(|c| said(&row[*c])).collect();
                    let primaries: Vec<usize> = primary.iter().copied().filter(|c| said(&row[*c])).collect();
                    let has_facts = facts.iter().any(|c| said(&row[*c]));
                    let text_width = (ui.available_width() - inset * 2.0 - 2.0 * (1.0 + 12.0)).max(1.0);
                    // the primary values in the body's type, each over at most two 22 lines, wrapped
                    let primary_galleys: Vec<std::sync::Arc<egui::Galley>> = primaries
                        .iter()
                        .map(|&c| {
                            let mut job = egui::text::LayoutJob::single_section(
                                row[c].draft(),
                                egui::TextFormat {
                                    font_id: TextRole::Body.font(t),
                                    color: t.color("card-foreground"),
                                    line_height: Some(22.0),
                                    ..Default::default()
                                },
                            );
                            job.wrap = egui::text::TextWrapping {
                                max_width: text_width,
                                max_rows: PRIMARY_LINES as usize,
                                break_anywhere: false,
                                overflow_character: Some('…'),
                            };
                            ui.fonts_mut(|f| f.layout_job(job))
                        })
                        .collect();
                    // a listed row is as tall as its words; a card keeps each primary value's every
                    // line, as the web places its cards by `stackedCardPixels`
                    let primary_lines: f32 = if listed {
                        primary_galleys.iter().map(|g| g.rows.len().max(1) as f32).sum()
                    } else {
                        PRIMARY_LINES * primary_galleys.len() as f32
                    };
                    let items = self.actions.as_ref().map(|actions| actions(index)).unwrap_or_default();
                    // 22 a primary line, 20 a labeled one
                    let parts = primaries.len() + shown.len() + usize::from(has_facts);
                    let height = 1.0
                        + pad
                        + 24.0
                        + 22.0 * primary_lines
                        + 20.0 * shown.len() as f32
                        + if has_facts { 24.0 } else { 0.0 }
                        + gap * parts as f32
                        + pad
                        + 1.0;
                    let width = ui.available_width() - inset * 2.0;
                    let (outer, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
                    let card = Rect::from_min_size(pos2(outer.left() + inset, outer.top()), vec2(width, height));
                    card_rects.push(card);
                    let hit = ui.interact(card, self.id.with(("card", index)), Sense::click());
                    if hit.clicked() {
                        self.state.opened = Some(index);
                    }
                    if listed {
                        if hit.hovered() {
                            ui.painter().rect_filled(card, 0.0, t.color("hover"));
                        }
                        ui.painter()
                            .hline(card.x_range(), card.bottom() - 0.5, Stroke::new(1.0, t.color("border")));
                    } else {
                        let ground = if hit.hovered() {
                            t.color("hover")
                        } else {
                            t.color("card")
                        };
                        let radius = t.radius("surface");
                        ui.painter().rect_filled(card, radius, ground);
                        ui.painter().rect_stroke(
                            card,
                            radius,
                            Stroke::new(1.0, t.color("border")),
                            egui::StrokeKind::Inside,
                        );
                    }
                    let left = card.left() + 1.0 + 12.0;
                    let right = card.right() - 1.0 - 12.0;
                    let mut y = card.top() + 1.0 + pad;
                    // the grip before the title, dragged to move the card
                    let mut title_left = left;
                    if self.reorder {
                        let grip = Rect::from_min_size(pos2(left, y), vec2(GRIP, 24.0));
                        let hold = ui.interact(grip, self.id.with(("card-grip", index)), Sense::drag());
                        hold.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Move"));
                        if hold.hovered() || hold.dragged() {
                            ui.ctx().set_cursor_icon(if hold.dragged() {
                                CursorIcon::Grabbing
                            } else {
                                CursorIcon::Grab
                            });
                        }
                        if hold.drag_started()
                            && let Some(position) = order.iter().position(|r| *r == index)
                        {
                            ui.ctx().data_mut(|d| d.insert_temp(drag_id, position));
                        }
                        icons::paint(
                            ui.painter(),
                            pos2(left + 6.0, y + 12.0),
                            Icon::GripVertical,
                            14.0,
                            t.color("muted-foreground"),
                        );
                        title_left += GRIP + 8.0;
                    }
                    // the title, the menu at the line's end
                    let menu = if items.is_empty() { 0.0 } else { 28.0 + 8.0 };
                    if let Some(c) = title {
                        let words = row[c].draft();
                        // a code stays in the code face, at the title's size
                        let rich = match &row[c] {
                            // in the title's weight, as the web's `font-mono` inside `font-medium`
                            Cell::Mono(_) | Cell::Marked { .. } => egui::RichText::new(words).font(crate::mono_font(
                                ui.ctx(),
                                crate::mono_size(t, TextRole::Action.font(t).size),
                                t.text(TextRole::Action.key()).weight,
                            )),
                            _ => TextRole::Action.rich(t, words),
                        };
                        let galley = WidgetText::from(rich).into_galley(
                            ui,
                            Some(TextWrapMode::Truncate),
                            (right - title_left - menu).max(1.0),
                            TextRole::Action.font(t),
                        );
                        ui.painter().galley(
                            pos2(title_left, y + 12.0 - galley.size().y / 2.0),
                            galley,
                            t.color("card-foreground"),
                        );
                    }
                    if !items.is_empty() {
                        let button = Rect::from_center_size(pos2(right - 14.0, y + 12.0), vec2(28.0, 24.0));
                        let trigger = ui.interact(button, self.id.with(("actions", index)), Sense::click());
                        trigger.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Actions"));
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
                        if let Some(chosen) = DropdownMenu::new(&items).show(&trigger) {
                            out.action = Some((index, chosen));
                        }
                    }
                    y += 24.0 + gap;
                    // the primary values, as laid out above
                    for galley in primary_galleys {
                        // the lines it takes, which the card's height counted
                        let lines = galley.rows.len().max(1) as f32;
                        ui.painter().galley(pos2(left, y), galley, t.color("card-foreground"));
                        y += 22.0 * lines + gap;
                    }
                    // a line per column with a value: its name, then the value
                    for &c in &shown {
                        let name = WidgetText::from(TextRole::Help.rich(t, &self.columns[c].title)).into_galley(
                            ui,
                            Some(TextWrapMode::Truncate),
                            name_width,
                            TextRole::Help.font(t),
                        );
                        ui.painter().galley(
                            pos2(left, y + 10.0 - name.size().y / 2.0),
                            name,
                            t.color("muted-foreground"),
                        );
                        let x = left + name_width + 8.0;
                        let place = Rect::from_min_max(pos2(x, y), pos2(right, y + 20.0));
                        line_value(ui, t, &row[c], place);
                        y += 20.0 + gap;
                    }
                    // the facts, one quiet line, a faint dot between them
                    if has_facts {
                        let mut x = left;
                        let middle = y + 12.0;
                        let mut first = true;
                        for &c in &facts {
                            if !said(&row[c]) {
                                continue;
                            }
                            if !first {
                                let dot = WidgetText::from(TextRole::Help.rich(t, "·")).into_galley(
                                    ui,
                                    None,
                                    f32::INFINITY,
                                    TextRole::Help.font(t),
                                );
                                x += 6.0;
                                ui.painter().galley(
                                    pos2(x, middle - dot.size().y / 2.0),
                                    dot.clone(),
                                    t.color("faint"),
                                );
                                x += dot.size().x + 6.0;
                            }
                            first = false;
                            x += paint_fact(ui, t, &row[c], pos2(x, middle), (right - x).max(1.0));
                        }
                    }
                    ui.add_space(apart);
                }
                // a dragged card: the line where it lands, and its move on release
                if let Some(from) = dragging {
                    let released = ui.input(|i| i.pointer.any_released());
                    if let Some(pointer) = ui.ctx().pointer_latest_pos()
                        && let (Some(first), Some(last)) = (card_rects.first(), card_rects.last())
                    {
                        let to = card_rects.iter().filter(|r| pointer.y > r.center().y).count();
                        let y = if to < card_rects.len() {
                            card_rects[to].top() - 8.0
                        } else {
                            last.bottom() + 8.0
                        };
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
            });
    }

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
        let hidden = |i: usize| self.state.hidden.contains(&i);
        let widths: Vec<f32> = self
            .columns
            .iter()
            .enumerate()
            .map(|(i, _)| {
                if hidden(i) {
                    0.0
                } else {
                    self.state.widths[i] + if i == 0 { lead } else { 0.0 }
                }
            })
            .collect();
        let shown = (0..self.columns.len()).filter(|i| !hidden(*i)).count();
        let gaps = GAP * shown.saturating_sub(1) as f32;
        let tail = if self.actions.is_some() { ACTIONS + GAP } else { 0.0 };
        // a table that fits its room (the web's `minWidth="auto"`: one that stacks rather than
        // scrolls) shrinks its columns in proportion where they run past it, none under 56
        let room = right - left - gaps - tail;
        let sum: f32 = widths.iter().sum();
        let fits_room = self.stack_below.is_some() || self.stack_layout == StackLayout::Labeled;
        let widths: Vec<f32> = if fits_room && sum > room && sum > 0.0 {
            widths.iter().map(|w| (w * room / sum).max(56.0)).collect()
        } else {
            widths
        };
        // the filling columns share the room as `minmax(width, <share>fr)` tracks: one whose share
        // falls under its width keeps its width, and the rest share again what is left
        let mut fills: Vec<usize> = (0..self.columns.len())
            .filter(|i| self.columns[*i].fill && !hidden(*i))
            .collect();
        if fills.is_empty() {
            fills.extend((0..self.columns.len()).rev().find(|i| !hidden(*i)));
        }
        let share = |i: usize| self.columns.get(i).map_or(1.0, |c| c.share);
        let mut widths = widths;
        let mut open = fills.clone();
        loop {
            let fixed: f32 = (0..widths.len())
                .filter(|i| !open.contains(i))
                .map(|i| widths[i])
                .sum::<f32>()
                + gaps
                + tail;
            let shares: f32 = open.iter().map(|i| share(*i)).sum();
            let unit = if shares > 0.0 {
                (right - left - fixed).max(0.0) / shares
            } else {
                0.0
            };
            let under: Vec<usize> = open.iter().copied().filter(|i| share(*i) * unit < widths[*i]).collect();
            if under.is_empty() {
                for i in &open {
                    widths[*i] = share(*i) * unit;
                }
                break;
            }
            open.retain(|i| !under.contains(i));
            if open.is_empty() {
                break;
            }
        }
        let mut x = left;
        widths
            .iter()
            .enumerate()
            .map(|(i, w)| {
                let w = *w;
                let place = (x, w);
                if !hidden(i) {
                    x += w + GAP;
                }
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
        let stacked = self.stacks(ui.available_width(), pad, select_width);
        // a stacked row: 8 over and under a 24 title line and a 20 line of facts, and its rule
        let row_height = if stacked {
            8.0 + 24.0 + 20.0 + 8.0 + 1.0
        } else {
            row_height
        };
        let head_height = if stacked { 0.0 } else { head_height };
        let keys: Vec<Id> = (0..self.rows.len())
            .map(|index| match self.keys.get(index) {
                Some(key) if self.keys.len() == self.rows.len() => *key,
                _ => Id::new(("row", index)),
            })
            .collect();
        follow_keys(self.state, &keys);
        if let Some(at) = self.state.pending.take()
            && at.0 < self.rows.len()
            && self.columns.get(at.1).is_some_and(|c| c.edit.is_some())
        {
            open_cell(self.state, self.rows, &keys, at);
        }
        let order = self.order();
        let radius = t.radius("surface");
        let width = ui.available_width();
        let room = ui.available_height();
        let mut out = TableOutput {
            response: ui.response(),
            toggled: None,
            switched: None,
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

        let frame = if self.framed {
            egui::Frame::new()
                .fill(t.color("card"))
                .stroke(Stroke::new(1.0, t.color("border-strong")))
                .corner_radius(radius)
        } else {
            egui::Frame::new()
                .fill(t.color("card"))
                .inner_margin(egui::Margin::same(1))
        };
        let drag_id = self.id.with("dragging");
        let response = frame
            .show(ui, |ui| {
                ui.set_width(width - 2.0);
                if self.stretch {
                    // the frame to the room's bottom, less its edge and the footer's line
                    ui.set_min_height(room - 2.0 - if self.footer { 26.0 } else { 0.0 });
                }
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                if self.bar {
                    self.draw_bar(ui, &t, stacked);
                }
                // a table with no rows given its own words says them alone, without its heads
                if self.rows.is_empty()
                    && let Some(empty) = self.empty.take()
                {
                    empty(ui);
                    return;
                }
                // labeled cards in place of the rows
                if stacked && self.stack_layout == StackLayout::Labeled && !self.rows.is_empty() {
                    self.labeled_cards(ui, &t, &order, &mut out);
                    return;
                }
                // the head, which a stacked table has none of
                let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), head_height), Sense::hover());
                if !stacked {
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
                        let place =
                            Rect::from_min_size(pos2(head.left() + pad, head.center().y - 8.0), vec2(16.0, 16.0));
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
                        if self.state.hidden.contains(&index) {
                            continue;
                        }
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
                            cycle_sort(self.state, index);
                        }
                        // the head's cell the keys stand on, ringed as the active cell is
                        if self.state.focused && self.state.head == Some(index) {
                            ui.painter().rect_stroke(
                                cell,
                                CornerRadius::ZERO,
                                Stroke::new(2.0, t.color("ring")),
                                egui::StrokeKind::Inside,
                            );
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
                        // the arrow 4 after the name, or before it in a column read from its end,
                        // as the web's head turns its row round there
                        let text_w = galley.size().x;
                        let text_x = if column.numeric {
                            cell.right() - text_w
                        } else {
                            cell.left() + lead
                        };
                        ui.painter()
                            .galley(pos2(text_x, cell.center().y - galley.size().y / 2.0), galley, ink);
                        if let Some((_, ascending)) = sorted {
                            let arrow = if ascending { Icon::ArrowUp } else { Icon::ArrowDown };
                            let center = if column.numeric {
                                text_x - 4.0 - 7.0
                            } else {
                                text_x + text_w + 4.0 + 7.0
                            };
                            icons::paint(ui.painter(), pos2(center, cell.center().y), arrow, 14.0, ink);
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
                // a stacked row grows to hold its open field: 8 for the title's, 12 for a fact's
                let title_column = self.columns.iter().position(|c| c.stack != StackPlace::Lead);
                let grown = |state: &TableState, index: usize| -> f32 {
                    match state.open.as_ref().filter(|o| stacked && o.row == index) {
                        Some(o) if Some(o.column) == title_column => 8.0,
                        Some(_) => 12.0,
                        None => 0.0,
                    }
                };
                // where the taller row stands in the order shown, and by how much, computed before the
                // rows borrow the state
                let taller: Option<(usize, f32)> = self.state.open.as_ref().and_then(|o| {
                    let extra = grown(self.state, o.row);
                    let position = order.iter().position(|r| *r == o.row)?;
                    (extra > 0.0).then_some((position, extra))
                });
                let area = egui::ScrollArea::vertical()
                    .id_salt(self.id.with("rows"))
                    .max_height(if self.stretch {
                        (room - head_height - 2.0).max(row_height)
                    } else {
                        // never taller than its rows: a height without bound shows them all
                        self.height.min(order.len() as f32 * row_height + 1.0)
                    })
                    .auto_shrink([false, true]);
                let count = order.len();
                let mut body = |ui: &mut Ui, range: std::ops::Range<usize>| {
                    for position in range {
                        let index = order[position];
                        let height = row_height + grown(self.state, index);
                        let (rect, row) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
                        row_rects.push(rect);
                        // a screen reader reads and presses the row by its cells' words, as a row reads
                        row.widget_info(|| {
                            let name = self.rows[index]
                                .iter()
                                .map(Cell::draft)
                                .filter(|d| !d.is_empty())
                                .collect::<Vec<_>>()
                                .join(", ");
                            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name)
                        });
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
                        // a list's last row leaves its rule to the frame, as the web's `border-b-0`
                        if !(self.variant == TableVariant::List && position + 1 == count) {
                            ui.painter().hline(
                                rect.x_range(),
                                rect.bottom() - 0.5,
                                Stroke::new(1.0, t.color("border")),
                            );
                        }
                        if self.selectable {
                            let mut checked = selected;
                            let place =
                                Rect::from_min_size(pos2(rect.left() + pad, rect.center().y - 8.0), vec2(16.0, 16.0));
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
                        if stacked {
                            let editing = Editing {
                                keys: &keys,
                                order: &order,
                                position,
                            };
                            let (toggled, edited) =
                                self.stacked_row(ui, &t, rect, index, (pad, select_width), &editing);
                            if toggled {
                                out.toggled = Some(index);
                            }
                            if edited.is_some() {
                                out.edited = edited;
                            }
                        }
                        let places = self.layout(rect.left() + pad + select_width, rect.right() - pad);
                        for (c, column) in self.columns.iter().enumerate().filter(|_| !stacked) {
                            if self.state.hidden.contains(&c) {
                                continue;
                            }
                            let (x, w) = places[c];
                            let mut cell = Rect::from_min_size(pos2(x, rect.top()), vec2(w, rect.height()));
                            if c == 0 && self.reorder {
                                // the grip: drag it to move the row
                                let grip = Rect::from_min_size(pos2(x - 4.0, rect.top()), vec2(GRIP, rect.height()));
                                let hit = ui.interact(grip, self.id.with(("grip", index)), Sense::drag());
                                hit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Move"));
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
                                if open_here {
                                    let id = self.id.with(("editor", keys[index], c));
                                    let editing = Editing {
                                        keys: &keys,
                                        order: &order,
                                        position,
                                    };
                                    if let Some(edit) = self.run_editor(ui, editor, place, id, &editing) {
                                        out.edited = Some(edit);
                                    }
                                } else {
                                    let hit = ui.interact(place, self.id.with(("edit", index, c)), Sense::click());
                                    match self.state.refused.get(&(keys[index], c)) {
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
                                        open_cell(self.state, self.rows, &keys, here);
                                    } else if hit.clicked() {
                                        self.state.active = Some(here);
                                        self.state.focused = true;
                                        self.state.head = None;
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
                                if matches!(self.rows[index][c], Cell::Switch { .. }) {
                                    out.switched = Some(index);
                                } else {
                                    out.toggled = Some(index);
                                }
                            }
                        }
                        if let Some(actions) = &self.actions {
                            let place = Rect::from_min_size(
                                pos2(rect.right() - pad - ACTIONS, rect.top()),
                                vec2(ACTIONS, rect.height()),
                            );
                            // stacked, the menu stands at the row's end on the title's line, a 24 box
                            let middle = if stacked {
                                pos2(rect.right() - pad - 12.0, rect.top() + 8.0 + 12.0)
                            } else {
                                place.center()
                            };
                            let button = Rect::from_center_size(middle, vec2(28.0, 28.0));
                            let trigger = ui.interact(button, self.id.with(("actions", index)), Sense::click());
                            trigger
                                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Actions"));
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
                };
                match taller {
                    // one stacked row is taller while it holds an open field: only the rows in
                    // view are drawn still, each placed past the taller one by its extra
                    Some((tall, extra)) => {
                        let top_of =
                            |position: usize| position as f32 * row_height + if position > tall { extra } else { 0.0 };
                        let at = |y: f32| {
                            let y = if y > top_of(tall) + row_height + extra {
                                y - extra
                            } else {
                                y.min(top_of(tall))
                            };
                            (y / row_height).floor().max(0.0) as usize
                        };
                        area.show_viewport(ui, |ui, viewport| {
                            ui.set_height(count as f32 * row_height + extra);
                            let first = at(viewport.min.y).min(count);
                            let last = (at(viewport.max.y) + 1).min(count);
                            let top = ui.max_rect().top();
                            let rect = Rect::from_x_y_ranges(
                                ui.max_rect().x_range(),
                                top + top_of(first)..=top + top_of(last),
                            );
                            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                                ui.skip_ahead_auto_ids(first);
                                body(ui, first..last);
                            });
                        });
                    }
                    None => {
                        area.show_rows(ui, row_height, count, body);
                    }
                }
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
        // on the head row, the keys sort: Left and Right choose a column, Enter or Space cycles its
        // sort, Down goes back to the rows
        let sorts = !stacked && self.columns.iter().any(|c| c.sortable);
        if self.state.focused
            && self.state.open.is_none()
            && let Some(column) = self.state.head.filter(|_| sorts)
        {
            let (left, right, down, enter) = ui.input_mut(|i| {
                let none = egui::Modifiers::NONE;
                (
                    i.consume_key(none, egui::Key::ArrowLeft),
                    i.consume_key(none, egui::Key::ArrowRight),
                    i.consume_key(none, egui::Key::ArrowDown),
                    i.consume_key(none, egui::Key::Enter) || i.consume_key(none, egui::Key::Space),
                )
            });
            let last = self.columns.len().saturating_sub(1);
            if left {
                self.state.head = Some(column.saturating_sub(1));
            } else if right {
                self.state.head = Some((column + 1).min(last));
            } else if enter && self.columns[column].sortable {
                cycle_sort(self.state, column);
            } else if down {
                self.state.head = None;
                if let Some(first) = self.order().first() {
                    self.state.active = Some((*first, column));
                }
            }
        }
        // the active cell's keys while the table has the focus: the arrows move it, Enter or F2 opens it
        else if self.state.focused
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
            // a character typed opens a text cell on it; Backspace or Delete opens a text or a date
            // cell empty
            let editor = self.columns.get(column).and_then(|c| c.edit.clone());
            let typed = match editor {
                Some(Editor::Text) => ui.input_mut(|i| {
                    let text = i.events.iter().find_map(|e| match e {
                        egui::Event::Text(text) if !text.is_empty() => Some(text.clone()),
                        _ => None,
                    });
                    i.events.retain(|e| !matches!(e, egui::Event::Text(_)));
                    text.or_else(|| erased(i))
                }),
                Some(Editor::Date(_)) => ui.input_mut(erased),
                _ => None,
            };
            if let Some(draft) = typed {
                open_cell(self.state, self.rows, &keys, (row, column));
                if let Some(open) = &mut self.state.open {
                    open.draft = draft;
                    open.error = None;
                    open.select_all = false;
                }
            } else if enter && editor.is_some() {
                open_cell(self.state, self.rows, &keys, (row, column));
            } else if left {
                self.state.active = Some((row, column.saturating_sub(1)));
            } else if right {
                self.state.active = Some((row, (column + 1).min(last)));
            } else if up && position == 0 && sorts {
                // up from the first row, onto the head, where the keys sort
                self.state.head = Some(column);
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
        self.state.active_key = self.state.active.and_then(|(row, _)| keys.get(row).copied());
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
        Cell::Text(label) | Cell::Number { text: label, .. } => {
            text(ui, TextRole::Body.rich(t, label), t.color("foreground"))
        }
        Cell::Strong(label) => text(
            ui,
            TextRole::Body.rich(t, label).variation("wght", 500.0),
            t.color("foreground"),
        ),
        Cell::Struck(label) => text(
            ui,
            TextRole::Body.rich(t, label).strikethrough(),
            t.color("muted-foreground"),
        ),
        Cell::Muted(label) => text(ui, TextRole::Body.rich(t, label), t.color("muted-foreground")),
        Cell::Toned(label, ink) => text(ui, TextRole::Body.rich(t, label), t.color(ink)),
        Cell::Mono(label) => {
            let rich = egui::RichText::new(label).font(egui::FontId::monospace(TextRole::Help.font(t).size));
            text(ui, rich, t.color("foreground"));
        }
        Cell::Marked { text: label, dot } => {
            // a 10 dot, 8 before the code, as the web's `Dot` and `gap-2`
            let lead = if let Some(color) = dot {
                ui.painter()
                    .circle_filled(pos2(rect.left() + 5.0, rect.center().y), 5.0, *color);
                18.0
            } else {
                0.0
            };
            let rich = egui::RichText::new(label).font(egui::FontId::monospace(TextRole::Help.font(t).size));
            let galley = WidgetText::from(rich).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                (room - lead).max(1.0),
                TextRole::Body.font(t),
            );
            ui.painter().galley(
                pos2(rect.left() + lead, rect.center().y - galley.size().y / 2.0),
                galley,
                t.color("foreground"),
            );
        }
        Cell::Swatch { text: label, color } => {
            // a 10 square, 6 before the name, as the web legend's `Swatch` and `gap-1.5`
            let swatch = Rect::from_center_size(pos2(rect.left() + 5.0, rect.center().y), vec2(10.0, 10.0));
            ui.painter().rect_filled(swatch, t.radius("mini"), *color);
            let galley = WidgetText::from(TextRole::Body.rich(t, label)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                (room - 16.0).max(1.0),
                TextRole::Body.font(t),
            );
            ui.painter().galley(
                pos2(rect.left() + 16.0, rect.center().y - galley.size().y / 2.0),
                galley,
                t.color("foreground"),
            );
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
        Cell::Switch { on, label } => {
            let mut switched = *on;
            let place = Rect::from_min_size(rect.min, vec2(rect.width(), rect.height()));
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(place)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            if child.add(super::Switch::new(&mut switched, label)).changed() {
                return Some(switched);
            }
        }
        Cell::Clock { clock, at, now } => {
            let mono = egui::RichText::new(clock).font(egui::FontId::monospace(TextRole::Help.font(t).size));
            let galley =
                WidgetText::from(mono).into_galley(ui, Some(TextWrapMode::Truncate), room, TextRole::Body.font(t));
            let width = galley.size().x;
            ui.painter().galley(
                pos2(rect.left(), rect.center().y - galley.size().y / 2.0),
                galley,
                t.color("foreground"),
            );
            let ago = WidgetText::from(TextRole::Body.rich(t, super::relative_time_short(at - now))).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                (room - width - 8.0).max(1.0),
                TextRole::Body.font(t),
            );
            ui.painter().galley(
                pos2(rect.left() + width + 8.0, rect.center().y - ago.size().y / 2.0),
                ago,
                t.color("muted-foreground"),
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
        Cell::Tags(tags) => {
            paint_tags(ui, tags, pos2(rect.left(), rect.center().y), rect.width());
        }
        Cell::Relative { .. } => text(ui, TextRole::Body.rich(t, cell.draft()), t.color("muted-foreground")),
        Cell::Trend(values, ink) => {
            let place = Rect::from_min_size(
                pos2(rect.left(), rect.center().y - 14.0),
                vec2(112.0_f32.min(room), 28.0),
            );
            super::sparkline(ui.painter(), place, values, *ink);
        }
        Cell::Empty => {}
    }
    None
}

/// A labeled line's value in `place`: text and a person at the help's size in the foreground,
/// tags and chips as they are.
fn line_value(ui: &Ui, t: &crate::Tokens, cell: &Cell, place: Rect) {
    let middle = place.center().y;
    match cell {
        Cell::Tags(tags) => {
            paint_tags(ui, tags, pos2(place.left(), middle), place.width());
        }
        Cell::Chip(label, tone) => {
            super::chip::paint_at(ui, place.left(), middle, label, *tone);
        }
        Cell::Person(name) => {
            let avatar = Rect::from_min_size(pos2(place.left(), middle - 8.0), vec2(16.0, 16.0));
            super::person::paint_avatar(ui, t, avatar, name, AvatarSize::Xxs, false);
            let galley = WidgetText::from(TextRole::Help.rich(t, name)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                (place.width() - 20.0).max(1.0),
                TextRole::Help.font(t),
            );
            ui.painter().galley(
                pos2(avatar.right() + 4.0, middle - galley.size().y / 2.0),
                galley,
                t.color("foreground"),
            );
        }
        other => {
            let galley = WidgetText::from(TextRole::Help.rich(t, other.draft())).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                place.width().max(1.0),
                TextRole::Help.font(t),
            );
            ui.painter().galley(
                pos2(place.left(), middle - galley.size().y / 2.0),
                galley,
                t.color("foreground"),
            );
        }
    }
}

/// Tags as neutral chips in the code face, 4 apart, from `at` on the line's middle, as many as fit
/// in `room`: the width they took.
fn paint_tags(ui: &Ui, tags: &[String], at: egui::Pos2, room: f32) -> f32 {
    let mut x = at.x;
    for tag in tags {
        let width = super::chip::paint_tag_at(ui, x, at.y, tag);
        if x + width > at.x + room {
            break;
        }
        x += width + 4.0;
    }
    (x - at.x - 4.0).max(0.0)
}

/// A value on a stacked row's facts line, small and quiet, its start at `at` (on the line's middle):
/// the width it took.
fn paint_fact(ui: &mut Ui, t: &crate::Tokens, cell: &Cell, at: egui::Pos2, room: f32) -> f32 {
    let quiet = t.color("muted-foreground");
    let text = |ui: &mut Ui, rich: egui::RichText, ink: egui::Color32, x: f32, room: f32| {
        let galley =
            WidgetText::from(rich).into_galley(ui, Some(TextWrapMode::Truncate), room.max(1.0), TextRole::Help.font(t));
        let width = galley.size().x;
        ui.painter().galley(pos2(x, at.y - galley.size().y / 2.0), galley, ink);
        width
    };
    match cell {
        Cell::Text(label) | Cell::Strong(label) | Cell::Number { text: label, .. } => {
            text(ui, TextRole::Help.rich(t, label), quiet, at.x, room)
        }
        Cell::Chip(label, tone) => super::chip::paint_at(ui, at.x, at.y, label, *tone),
        Cell::Tags(tags) => paint_tags(ui, tags, at, room),
        Cell::Relative { .. } => text(ui, TextRole::Help.rich(t, cell.draft()), quiet, at.x, room),
        Cell::Struck(label) => text(ui, TextRole::Help.rich(t, label).strikethrough(), quiet, at.x, room),
        Cell::Muted(label) => text(ui, TextRole::Help.rich(t, label), quiet, at.x, room),
        Cell::Toned(label, ink) => text(ui, TextRole::Help.rich(t, label), t.color(ink), at.x, room),
        Cell::Mono(label) | Cell::Marked { text: label, .. } => {
            let rich = egui::RichText::new(label).font(egui::FontId::monospace(TextRole::Help.font(t).size));
            text(ui, rich, quiet, at.x, room)
        }
        Cell::Swatch { text: label, .. } => text(ui, TextRole::Help.rich(t, label), quiet, at.x, room),
        Cell::Due { date, now, done } => {
            let (rich, ink) = DueText::new(*date, *now).done(*done).role(TextRole::Help).rich(t);
            text(ui, rich, ink, at.x, room)
        }
        Cell::Person(name) => {
            let avatar = Rect::from_min_size(pos2(at.x, at.y - 8.0), vec2(16.0, 16.0));
            super::person::paint_avatar(ui, t, avatar, name, AvatarSize::Xxs, false);
            20.0 + text(
                ui,
                TextRole::Help.rich(t, name),
                quiet,
                avatar.right() + 4.0,
                room - 20.0,
            )
        }
        Cell::Clock { clock, at: moment, now } => text(
            ui,
            TextRole::Help.rich(t, format!("{clock} {}", super::relative_time_short(moment - now))),
            quiet,
            at.x,
            room,
        ),
        Cell::Check(_) | Cell::Switch { .. } | Cell::Trend(..) | Cell::Empty => 0.0,
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

/// Sort by `column`: ascending, then descending, then as given.
fn cycle_sort(state: &mut TableState, column: usize) {
    state.sort = match state.sort.filter(|(c, _)| *c == column) {
        None => Some((column, true)),
        Some((_, true)) => Some((column, false)),
        Some((_, false)) => None,
    };
}

/// Find the rows the editing state names by key among `keys`, this frame's: an open editor or an
/// active cell whose row is gone closes, and one whose row moved follows it.
fn follow_keys(state: &mut TableState, keys: &[Id]) {
    let index_of = |key: Id| keys.iter().position(|k| *k == key);
    if let Some(mut open) = state.open.take()
        && let Some(row) = index_of(open.key)
    {
        open.row = row;
        state.open = Some(open);
    }
    match (state.active, state.active_key.and_then(index_of)) {
        (Some((_, column)), Some(row)) => state.active = Some((row, column)),
        (Some(_), None) if state.active_key.is_some() => {
            state.active = None;
            state.focused = false;
        }
        _ => {}
    }
}

/// Open the editor of the cell `at`: on its refused draft if it has one, else on its value.
fn open_cell(state: &mut TableState, rows: &[Vec<Cell>], keys: &[Id], at: (usize, usize)) {
    let key = keys[at.0];
    let (draft, error) = match state.refused.get(&(key, at.1)) {
        Some((draft, error)) => (draft.clone(), Some(error.clone())),
        None => (rows[at.0][at.1].draft(), None),
    };
    state.active = Some(at);
    state.focused = true;
    state.head = None;
    state.open = Some(OpenCell {
        key,
        row: at.0,
        column: at.1,
        draft,
        error,
        fresh: true,
        calendar: false,
        highlight: 0,
        select_all: true,
        focus_day: None,
        boxed: false,
    });
}

/// Backspace or Delete pressed, consumed: the cell opens empty.
fn erased(input: &mut egui::InputState) -> Option<String> {
    let none = egui::Modifiers::NONE;
    (input.consume_key(none, egui::Key::Backspace) || input.consume_key(none, egui::Key::Delete)).then(String::new)
}

/// What an open editor's move does: keep it open, drop it, or save its draft (unchanged, refused or
/// saved) and close it or open the next editable cell. A saved draft is returned for the app.
#[allow(clippy::too_many_arguments)]
fn settle(
    state: &mut TableState,
    columns: &[Column],
    rows: &[Vec<Cell>],
    keys: &[Id],
    order: &[usize],
    position: usize,
    mut open: OpenCell,
    moved: EditMove,
    words: &EditWords,
) -> Option<CellEdit> {
    let at = (open.key, open.column);
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
        open_cell(state, rows, keys, next);
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
