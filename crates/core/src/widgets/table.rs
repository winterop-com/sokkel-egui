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
    /// The band it sits under: neighbors with the same band share one head above theirs, a rule
    /// where each band starts and ends.
    pub band: Option<String>,
    /// What a group's head says in this column for the group's rows, as the web's `aggregate`.
    pub aggregate: Option<Aggregate>,
}

/// What a group head says in a column for its rows: given the rows given and the indexes of the
/// group's rows, and the column's index.
pub type Aggregate = fn(rows: &[Vec<Cell>], group: &[usize], column: usize) -> String;

/// The sum of a column's figures over a group's rows, its thousands grouped, as a tally's total.
pub fn sum_aggregate(rows: &[Vec<Cell>], group: &[usize], column: usize) -> String {
    let total: f64 = group
        .iter()
        .filter_map(|i| rows.get(*i).and_then(|row| row.get(column)))
        .filter_map(|cell| cell.draft().replace(',', "").trim().parse::<f64>().ok())
        .sum();
    crate::numbers::locale_number(total)
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
            band: None,
            aggregate: None,
        }
    }

    /// The band the column sits under, such as one option of a cut whose other options are the
    /// columns beside it.
    pub fn band(mut self, band: impl Into<String>) -> Self {
        self.band = Some(band.into());
        self
    }

    /// What a group's head says in this column for its rows, such as [`sum_aggregate`].
    pub fn aggregate(mut self, aggregate: Aggregate) -> Self {
        self.aggregate = Some(aggregate);
        self
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
    /// A name at medium weight over a line of small muted words, as a `Stack` of a `font-medium`
    /// name and a `text-help` line; the rows want `row_height` to hold both.
    Lines {
        title: String,
        detail: String,
        /// The small line over the name rather than under it.
        over: bool,
    },
    /// An id or a code, in the mono face.
    Mono(String),
    /// A code in the mono face in the muted ink, as a quieter code beside the row's own (a run's
    /// step).
    MutedCode(String),
    /// A name over its id small and muted in the mono face, as an object named over its code; the
    /// name at medium weight when `strong`.
    NamedCode {
        name: String,
        code: String,
        strong: bool,
    },
    /// An icon in the muted ink, then 6 on a name at medium weight, as a document named by its kind.
    IconName {
        icon: Icon,
        name: String,
    },
    /// A tag: the hash in the muted ink, then 4 on its name at medium weight.
    TagName(String),
    /// An 8 dot in a color token's color, the second the words a screen reader says for it, as a
    /// problem's kind.
    Dot(String, String),
    /// A name, then 8 on its code small and muted in the mono face, on one line.
    NameThenCode {
        name: String,
        code: String,
    },
    /// A chip, then 8 on a note in the muted ink, as a last run's state and when it was.
    ChipNote {
        label: String,
        tone: ChipTone,
        note: String,
    },
    /// A code in the mono face at the body's size, as a `font-mono` cell in a `text-sm` row.
    Code(String),
    /// A code in the mono face at the body's size over a line of small muted words, as a kind's
    /// name over what it holds.
    CodeLines {
        code: String,
        detail: String,
    },
    /// A state as a chip.
    Chip(String, ChipTone),
    /// A check box whose press the table reports as `toggled`.
    Check(bool),
    /// A small ghost icon button named `label`, at the cell's end, whose press the table reports as
    /// `pressed`, as a row's Take out.
    Button {
        icon: Icon,
        label: String,
    },
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
    /// Tags as outline chips in the code face, as `TagChips` with `variant="outline"`.
    OutlineTags(Vec<String>),
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
    /// Words in runs, each in its own ink token and face, as a cell of spans: a muted formula with
    /// its joining word in the faint ink, or a rule with its operator in the code face.
    Runs(Vec<CellRun>),
    /// A quiet badge in the code face, as the web's `Badge variant="secondary"` with `font-mono`:
    /// a model's format.
    Badge(String),
    /// A cell the app draws itself, with `DataTable::slot`, such as a download's button or a
    /// progress bar; `words` is what it says for the sort, the search and a stacked line.
    Slot {
        words: String,
    },
    /// A figure written as `text` and sorted by `value`, as a column's `sortValue` sorts by the
    /// number rather than the words: `144 ms`.
    Number {
        value: f64,
        text: String,
    },
    Empty,
}

/// One run of a `Cell::Runs`: its words, the color token they are drawn in, and whether in the code
/// face.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellRun {
    pub text: String,
    pub ink: &'static str,
    pub mono: bool,
}

impl CellRun {
    pub fn new(text: impl Into<String>, ink: &'static str) -> Self {
        Self {
            text: text.into(),
            ink,
            mono: false,
        }
    }

    /// In the code face.
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }
}

/// The runs as one text, each in its ink and face, at `role`'s size.
fn runs_job(ui: &Ui, t: &crate::Tokens, runs: &[CellRun], role: TextRole) -> egui::text::LayoutJob {
    let style = ui.style();
    let mut job = egui::text::LayoutJob::default();
    for run in runs {
        let rich = if run.mono {
            egui::RichText::new(&run.text)
                .font(crate::code_font(ui.ctx(), t, role))
                .line_height(Some(role.line_height(t)))
        } else {
            role.rich(t, &run.text)
        };
        rich.color(t.color(run.ink))
            .append_to(&mut job, style, egui::FontSelection::Default, egui::Align::Center);
    }
    job
}

fn runs_text(runs: &[CellRun]) -> String {
    runs.iter().map(|run| run.text.as_str()).collect()
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
            | Cell::MutedCode(text)
            | Cell::Code(text)
            | Cell::Badge(text)
            | Cell::Chip(text, _)
            | Cell::Person(text) => text.clone(),
            Cell::Lines { title, .. } => title.clone(),
            Cell::CodeLines { code, .. } => code.clone(),
            Cell::NamedCode { name, .. } | Cell::NameThenCode { name, .. } | Cell::IconName { name, .. } => {
                name.clone()
            }
            Cell::TagName(name) | Cell::Dot(_, name) => name.clone(),
            Cell::ChipNote { label, note, .. } => format!("{label} {note}"),
            Cell::Number { text, .. } | Cell::Marked { text, .. } | Cell::Swatch { text, .. } => text.clone(),
            Cell::Due { date, .. } => date.map(|d| d.to_string()).unwrap_or_default(),
            Cell::Tags(tags) | Cell::OutlineTags(tags) => tags.join(" "),
            Cell::Relative { at, now } => super::relative_time(at - now),
            Cell::Clock { clock, .. } => clock.clone(),
            Cell::Runs(runs) => runs_text(runs),
            Cell::Slot { words } => words.clone(),
            Cell::Check(_) | Cell::Switch { .. } | Cell::Button { .. } | Cell::Trend(..) | Cell::Empty => String::new(),
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
            | Cell::MutedCode(text)
            | Cell::Code(text)
            | Cell::Badge(text)
            | Cell::Chip(text, _)
            | Cell::Person(text) => text.clone(),
            Cell::Lines { title, .. } => title.clone(),
            Cell::CodeLines { code, .. } => code.clone(),
            Cell::NamedCode { name, .. } | Cell::NameThenCode { name, .. } | Cell::IconName { name, .. } => {
                name.clone()
            }
            Cell::TagName(name) | Cell::Dot(_, name) => name.clone(),
            Cell::ChipNote { label, note, .. } => format!("{label} {note}"),
            Cell::Check(on) | Cell::Switch { on, .. } => u8::from(*on).to_string(),
            Cell::Button { .. } => String::new(),
            Cell::Clock { at, .. } => format!("{:024.3}", at + 1e18),
            Cell::Due { date, .. } => date.map_or("9999".into(), |d| d.to_string()),
            Cell::Tags(tags) | Cell::OutlineTags(tags) => tags.join(" "),
            Cell::Relative { at, .. } => format!("{:024.3}", at + 1e18),
            Cell::Trend(values, _) => format!("{:024.3}", values.last().copied().unwrap_or(0.0) + 1e18),
            // lifted by 1e12 so a negative figure still sorts as a word, small enough that an f64
            // read back keeps every digit
            Cell::Number { value, .. } => format!("{:024.6}", value + 1e12),
            Cell::Marked { text, .. } | Cell::Swatch { text, .. } => text.clone(),
            Cell::Runs(runs) => runs_text(runs),
            Cell::Slot { words } => words.clone(),
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
    /// The row a selection was last turned on or off at, where a Shift press's range starts.
    anchor: Option<usize>,
    /// The row a press or Enter picked last, by its index in the rows given, as the web's
    /// `onOpen`: a report for the app, which takes it. The row drawn as opened is the app's,
    /// given by [`DataTable::opened`].
    pub opened: Option<usize>,
    /// The rows opened into their own detail under them (a table with `detail`), by their index.
    pub expanded: BTreeSet<usize>,
    /// The groups folded shut under their heads (a table with `groups`), by their key.
    pub folded: BTreeSet<String>,
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
    /// The page shown of a paged table, from 0, and its rows to a page; 0 until it is first drawn.
    pub page: usize,
    pub page_size: usize,
    /// The rows shown as cards in a grid, in a table that draws them (`DataTable::cards`), as the
    /// web's `view="grid"`.
    pub grid: bool,
    /// The branches whose children the table asked the app for (`TableOutput::load_children`),
    /// so each is asked once while it stays open and waits.
    asked: BTreeSet<usize>,
    /// The branches whose children did not load, said under each with Try again until it is
    /// pressed or the branch closes.
    refusals: BTreeSet<usize>,
    /// The active row was moved by a key this frame: the rows scroll to keep it in view.
    reveal: bool,
    /// The active row was moved by a key last, not by a press: it is marked, as the web marks it
    /// only under keyboard focus.
    keyed: bool,
    /// How far the columns are scrolled sideways, where they run past a table's room.
    pub scroll_x: f32,
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

    /// Make the cell at `row` and `column` the active one, the table holding the focus, as a press
    /// on it would: it is marked as the table's `cell_focus` draws it.
    pub fn focus_cell(&mut self, row: usize, column: usize) {
        self.active = Some((row, column));
        // the row is named by its index until the table next draws and keys it
        self.active_key = None;
        self.focused = true;
        self.head = None;
    }

    /// Make `row` the active row of a tree, the table holding the focus, as the keys would move
    /// it there: marked with its bar at its start, the keys going on from it.
    pub fn focus_row(&mut self, row: usize) {
        self.focus_cell(row, 0);
        self.keyed = true;
    }

    /// Say the children of the branch at `row` did not load (the app's answer to
    /// `TableOutput::load_children`), as the web's failed `loadChildren`: a line under the branch
    /// says so with Try again, which asks for them again. Closing the branch forgets it.
    pub fn refuse_children(&mut self, row: usize) {
        self.refusals.insert(row);
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
    /// A row's control that opens its branch or its detail (`dataTable.expand`), and closes it
    /// (`dataTable.collapse`); a table's `expand_label` names it for its row instead.
    pub expand: String,
    pub collapse: String,
    /// What an open branch's control says while its children load (`dataTable.loadingChildren`).
    pub loading_children: String,
    /// A failed read's heading by what the server answered: refused (`pageState.refused`), a 404
    /// (`pageState.missing`) or no answer (`pageState.unreachable`); and its Try again
    /// (`pageState.retry`).
    pub refused: String,
    pub missing: String,
    pub unreachable: String,
    pub retry: String,
    /// A group head's control while the group is folded, and while it is open.
    pub show_group: String,
    pub hide_group: String,
    pub edit: EditWords,
    /// The bar's choice of the table and of the cards' grid, in a table that draws cards.
    pub table_view: String,
    pub grid_view: String,
    /// A card's button that opens its row's actions.
    pub actions: String,
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
            expand: "Expand".into(),
            collapse: "Collapse".into(),
            loading_children: "Loading".into(),
            refused: "The server refused this read".into(),
            missing: "Nothing is there".into(),
            unreachable: "The server is not answering".into(),
            retry: "Try again".into(),
            show_group: "Show group".into(),
            hide_group: "Hide group".into(),
            edit: EditWords::default(),
            table_view: "Table".into(),
            grid_view: "Grid".into(),
            actions: "More actions".into(),
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
    /// A cell's button was pressed: its row and column.
    pub pressed: Option<(usize, usize)>,
    /// A row's action was chosen: the row and the action's index in its menu.
    pub action: Option<(usize, usize)>,
    /// A row was dragged by its grip from one place in the order shown to another.
    pub reordered: Option<(usize, usize)>,
    /// A cell's edit was saved: the app stores it.
    pub edited: Option<CellEdit>,
    /// A branch opened whose children are not given yet (a row `lazy` says has some): the app
    /// brings them, as rows whose parent is this one. Asked once while the branch stays open.
    pub load_children: Option<usize>,
    /// Try again was pressed under a failed read: the app reads the rows again.
    pub retry: bool,
}

/// What the server answered a read that failed, as the web's `ReadOutcome`: an HTTP status (404
/// says nothing is there, any other that it refused), or no answer at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadOutcome {
    Status(u16),
    Unreachable,
}

/// A read of the rows that failed: the server's diagnostic and what it answered.
struct Failure {
    error: String,
    status: ReadOutcome,
}

/// Where each row stands in a tree, by its index in the rows given: its parent, how deep it is,
/// whether rows are given under it, and whether it says it has children not given yet.
struct TreeShape {
    parent: Vec<Option<usize>>,
    depth: Vec<usize>,
    has_kids: Vec<bool>,
    lazy: Vec<bool>,
}

impl TreeShape {
    /// The shape `parents` give `count` rows: a parent out of range, the row itself or one that
    /// would close a loop makes a row top level.
    fn of(parents: &[Option<usize>], count: usize, lazy: impl Fn(usize) -> bool) -> Self {
        let mut parent: Vec<Option<usize>> = (0..count)
            .map(|i| parents.get(i).copied().flatten().filter(|p| *p < count && *p != i))
            .collect();
        // a row whose line of parents comes back to it is top level
        for i in 0..count {
            let mut at = parent[i];
            let mut steps = 0;
            while let Some(p) = at {
                if p == i || steps > count {
                    parent[i] = None;
                    break;
                }
                at = parent[p];
                steps += 1;
            }
        }
        let depth = (0..count)
            .map(|i| {
                let (mut depth, mut at) = (0, parent[i]);
                while let Some(p) = at {
                    depth += 1;
                    at = parent[p];
                }
                depth
            })
            .collect();
        let mut has_kids = vec![false; count];
        for p in parent.iter().flatten() {
            has_kids[*p] = true;
        }
        let lazy = (0..count).map(|i| !has_kids[i] && lazy(i)).collect();
        Self {
            parent,
            depth,
            has_kids,
            lazy,
        }
    }

    /// The top-level row `row` stands under, or itself.
    fn root(&self, row: usize) -> usize {
        let mut at = row;
        while let Some(p) = self.parent[at] {
            at = p;
        }
        at
    }

    /// `row` and every row under it, at any depth, open or not: what selecting a branch selects.
    fn branch_rows(&self, row: usize) -> Vec<usize> {
        let mut kids: Vec<Vec<usize>> = vec![Vec::new(); self.parent.len()];
        for (child, parent) in self.parent.iter().enumerate() {
            if let Some(parent) = parent {
                kids[*parent].push(child);
            }
        }
        let mut rows = vec![row];
        let mut at = 0;
        while at < rows.len() {
            let parent = rows[at];
            rows.extend(&kids[parent]);
            at += 1;
        }
        rows
    }

    /// Whether `row` opens: it has children given or still to come.
    fn branch(&self, row: usize) -> bool {
        self.has_kids[row] || self.lazy[row]
    }

    /// The rows as drawn, from `sorted` (every row in the order its level sorts in): each top-level
    /// row in that order, an open row's children right under it in that order too.
    fn flatten(&self, sorted: &[usize], expanded: &BTreeSet<usize>) -> Vec<usize> {
        let mut kids: Vec<Vec<usize>> = vec![Vec::new(); self.parent.len()];
        let mut roots = Vec::new();
        for &i in sorted {
            match self.parent[i] {
                Some(p) => kids[p].push(i),
                None => roots.push(i),
            }
        }
        let mut flat = Vec::with_capacity(sorted.len());
        let mut stack: Vec<usize> = roots.into_iter().rev().collect();
        while let Some(row) = stack.pop() {
            flat.push(row);
            if expanded.contains(&row) {
                stack.extend(kids[row].iter().rev());
            }
        }
        flat
    }
}

/// A placeholder bar's pulse: opacity 1 to 0.5 and back over two seconds, as `animate-pulse`;
/// held at 1 while motion is off.
/// A stacked row's place in a tree: its indent, whether it opens, and whether its children are on
/// their way.
#[derive(Clone, Copy)]
struct StackedTree {
    indent: f32,
    branch: bool,
    waiting: bool,
}

fn pulse(ui: &Ui) -> f32 {
    if super::sign_in_art::still(ui) {
        return 1.0;
    }
    ui.ctx().request_repaint();
    let phase = (ui.input(|i| i.time) as f32 * std::f32::consts::PI).cos() * 0.5 + 0.5;
    0.5 + 0.5 * phase
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
    row_height: Option<f32>,
    chevron: bool,
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
    search_placeholder: Option<String>,
    bar: bool,
    heads: bool,
    toolbar: Option<Toolbar<'a>>,
    foot: Option<String>,
    paging: Option<Vec<usize>>,
    auto: bool,
    detail: Option<RowDetail<'a>>,
    groups: Option<Vec<String>>,
    group_order: Vec<String>,
    opened: Option<usize>,
    fits: bool,
    cards: Option<Cards<'a>>,
    actions_hover: bool,
    bulk: Option<Bulk<'a>>,
    slot: Option<Slot<'a>>,
    cell_focus: CellFocus,
    parents: Option<Vec<Option<usize>>>,
    lazy: Option<LazyRows<'a>>,
    expand_label: Option<ExpandLabel<'a>>,
    loading: bool,
    failure: Option<Failure>,
    problems: Vec<String>,
    retry: bool,
}

/// Whether a row has children the app has not given yet, by its index in the rows given.
type LazyRows<'a> = Box<dyn Fn(usize) -> bool + 'a>;

/// A row's disclosure name for its index in the rows given and whether it is open.
type ExpandLabel<'a> = Box<dyn Fn(usize, bool) -> String + 'a>;

/// How an editable table marks its active cell while it has the focus, as sokkel-ui's
/// `cellFocus`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CellFocus {
    /// A line 2 tall in `ring` along the cell's foot, inside it.
    #[default]
    Underline,
    /// A ring 2 wide in `ring` inside the cell's edge.
    Ring,
    /// The cell on `hover` with a ring 1 wide in `border-strong` inside its edge.
    Soft,
    /// A line 1 tall in `border-strong` along the cell's foot, inside it, and its row on `hover`.
    Row,
}

/// Which columns start a band edge: a shown column whose band differs from the shown one before
/// it, either having one. Such a column stands a further gap after the one before, its rule in
/// the middle, so its words sit 12 from the rule on each side, as the web's cell padding.
fn band_edges(columns: &[Column], hidden: &BTreeSet<usize>) -> Vec<bool> {
    let mut edges = vec![false; columns.len()];
    let shown: Vec<usize> = (0..columns.len()).filter(|i| !hidden.contains(i)).collect();
    for pair in shown.windows(2) {
        let (before, here) = (&columns[pair[0]].band, &columns[pair[1]].band);
        edges[pair[1]] = before != here && (before.is_some() || here.is_some());
    }
    edges
}

/// The lines beside a table's pinned columns across `row`, as the web's sticky cells' 1px
/// `border` shadows: after the first column's end and before the end column's start.
fn pinned_rules(painter: &egui::Painter, t: &crate::Tokens, row: Rect, (start, end): (Option<f32>, Option<f32>)) {
    let stroke = Stroke::new(1.0, t.color("border"));
    if let Some(x) = start {
        painter.vline(x + 0.5, row.y_range(), stroke);
    }
    if let Some(x) = end {
        painter.vline(x - 0.5, row.y_range(), stroke);
    }
}

/// The rules where each band starts and ends, across `row`: a 1px `border` line a gap before a
/// column at a band edge, in the middle of the two gaps that column stands after.
fn band_rules(
    painter: &egui::Painter,
    t: &crate::Tokens,
    columns: &[Column],
    hidden: &BTreeSet<usize>,
    places: &[(f32, f32)],
    row: Rect,
) {
    for (c, edge) in band_edges(columns, hidden).into_iter().enumerate() {
        if edge {
            let x = (places[c].0 - GAP).round() + 0.5;
            painter.vline(x, row.y_range(), Stroke::new(1.0, t.color("border")));
        }
    }
}

/// Mark the active cell `cell` as `focus` draws it.
fn mark_cell(painter: &egui::Painter, t: &crate::Tokens, cell: Rect, focus: CellFocus) {
    let foot = |height: f32, color| {
        painter.rect_filled(
            Rect::from_min_max(pos2(cell.left(), cell.bottom() - height), cell.max),
            CornerRadius::ZERO,
            color,
        );
    };
    match focus {
        CellFocus::Underline => foot(2.0, t.color("ring")),
        CellFocus::Ring => {
            painter.rect_stroke(
                cell,
                CornerRadius::ZERO,
                Stroke::new(2.0, t.color("ring")),
                egui::StrokeKind::Inside,
            );
        }
        CellFocus::Soft => {
            // its `hover` ground is laid under the cell's content before it is drawn
            painter.rect_stroke(
                cell,
                CornerRadius::ZERO,
                Stroke::new(1.0, t.color("border-strong")),
                egui::StrokeKind::Inside,
            );
        }
        CellFocus::Row => foot(1.0, t.color("border-strong")),
    }
}

/// How the app draws a `Cell::Slot`: in a child of the cell's room, given the row's index in the
/// rows given and the column's.
type Slot<'a> = Box<dyn Fn(&mut Ui, usize, usize) + 'a>;

/// What the selection bar offers for the rows chosen, by their indexes in the rows given.
type Bulk<'a> = Box<dyn FnOnce(&mut Ui, &BTreeSet<usize>) + 'a>;

/// What a row opens into under it, drawn for its index in the rows given.
pub type RowDetail<'a> = Box<dyn Fn(usize, &mut Ui) + 'a>;

/// What a table's toolbar line draws.
type Toolbar<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A row drawn as a card, for its index in the rows given, `width` wide: its press.
pub type RowCard<'a> = Box<dyn Fn(&mut Ui, usize, f32) -> Response + 'a>;

/// The cards' grid: each card at least `min` wide and `height` tall, and how one is drawn.
struct Cards<'a> {
    min: f32,
    height: f32,
    card: RowCard<'a>,
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
    pub fn new(
        id: impl egui::AsIdSalt,
        columns: &'a [Column],
        rows: &'a [Vec<Cell>],
        state: &'a mut TableState,
    ) -> Self {
        Self {
            id: Id::unique(id),
            columns,
            rows,
            state,
            selectable: false,
            variant: TableVariant::Table,
            height: 360.0,
            search_placeholder: None,
            row_height: None,
            chevron: false,
            words: TableWords::default(),
            reorder: false,
            cell_focus: CellFocus::default(),
            actions: None,
            empty: None,
            footer: true,
            stretch: false,
            framed: true,
            today: Day::today(),
            keys: Vec::new(),
            stack_layout: StackLayout::Never,
            stack_below: None,
            toolbar: None,
            foot: None,
            paging: None,
            search: None,
            bar: false,
            heads: true,
            auto: false,
            detail: None,
            groups: None,
            group_order: Vec::new(),
            opened: None,
            fits: false,
            cards: None,
            actions_hover: false,
            bulk: None,
            slot: None,
            parents: None,
            lazy: None,
            expand_label: None,
            loading: false,
            failure: None,
            problems: Vec::new(),
            retry: false,
        }
    }

    /// The rows as a tree, as the web's `childRows`: each row's parent by its index in the rows
    /// given (in the rows' order), `None` for a top-level row. A row with children draws a chevron
    /// before its first cell, indented 20 a level, that opens and closes it; its children sort
    /// among themselves under it. The open rows are the state's `expanded`. With `groups`, a
    /// child stands in its top-level row's group. Right opens the active branch, or moves to its
    /// first child once open; Left closes it, or moves to its parent.
    pub fn tree(mut self, parents: impl IntoIterator<Item = Option<usize>>) -> Self {
        self.parents = Some(parents.into_iter().collect());
        self
    }

    /// Which rows have children not given yet, as the web's `hasChildren` with `loadChildren`: such
    /// a row opens, and while it is open with none given its control turns to a spinner said as
    /// `words.loading_children` and the table asks for them once (`TableOutput::load_children`).
    pub fn lazy(mut self, lazy: impl Fn(usize) -> bool + 'a) -> Self {
        self.lazy = Some(Box::new(lazy));
        self
    }

    /// The name of a row's control that opens or closes it, its accessible name and its tooltip,
    /// given its index in the rows given and whether it is open, as the web's `expandLabel`: "Show
    /// the tokens of ada". `words.expand` and `words.collapse` by default.
    pub fn expand_label(mut self, label: impl Fn(usize, bool) -> String + 'a) -> Self {
        self.expand_label = Some(Box::new(label));
        self
    }

    /// Rows on their way, as the web's `loading`: placeholder rows under the heads, filling the
    /// view while there are no rows yet, or three after the last row while more come. The bar, its
    /// search, the heads and the frame keep their places, and nothing is counted until rows come.
    /// It is drawn over a failure too, while the rows are read again.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// The rows could not be read, as the web's `error` and `status`: while not `loading`, the
    /// failure stands in place of the rows under the heads, unframed, headed by what the server
    /// answered with `error` (its diagnostic) under it. The bar and the frame stay, counting
    /// nothing.
    pub fn failed(mut self, error: impl Into<String>, status: ReadOutcome) -> Self {
        self.failure = Some(Failure {
            error: error.into(),
            status,
        });
        self
    }

    /// Each problem a failed read's refusal named, a line each under its diagnostic.
    pub fn problems(mut self, problems: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.problems = problems.into_iter().map(Into::into).collect();
        self
    }

    /// Try again under a failed read, as the web's `onRetry`: pressed, `TableOutput::retry`.
    pub fn retry(mut self, retry: bool) -> Self {
        self.retry = retry;
        self
    }

    /// The failure drawn now: given, and not loading again.
    fn failing(&self) -> bool {
        self.failure.is_some() && !self.loading
    }

    /// The tree's shape, when the rows are a tree.
    fn tree_shape(&self) -> Option<TreeShape> {
        let parents = self.parents.as_ref()?;
        let lazy = |i: usize| self.lazy.as_ref().is_some_and(|lazy| lazy(i));
        Some(TreeShape::of(parents, self.rows.len(), lazy))
    }

    /// Draws each `Cell::Slot` with `slot`, in a child of the cell's room laid from its start and
    /// centered down it, given the row's index in the rows given and the column's: a download's
    /// button, a progress bar.
    pub fn slot(mut self, slot: impl Fn(&mut Ui, usize, usize) + 'a) -> Self {
        self.slot = Some(Box::new(slot));
        self
    }

    /// What the selection bar offers for the rows chosen, after its Clear, as the web's
    /// `bulkActions`: drawn with the chosen rows' indexes in the rows given.
    pub fn bulk(mut self, bulk: impl FnOnce(&mut Ui, &BTreeSet<usize>) + 'a) -> Self {
        self.bulk = Some(Box::new(bulk));
        self
    }

    /// The rows as cards in a grid too, as the web's `card` with `view` and `onViewChange`: each
    /// card at least `min` wide (`cardWidth`) and `height` tall (`cardHeight`), drawn by `card` for
    /// a row's index in the rows given and its width, its press opening the row. The bar offers
    /// the table and the grid, `TableState::grid` holding which is shown; in the grid the bar sorts
    /// by a menu, as a stacked table's does, and a row's actions stand at its card's corner.
    pub fn cards(mut self, min: f32, height: f32, card: impl Fn(&mut Ui, usize, f32) -> Response + 'a) -> Self {
        self.cards = Some(Cards {
            min,
            height,
            card: Box::new(card),
        });
        self
    }

    /// A row's actions button shown only while the row is pointed at, has the focus or its menu is
    /// open, as the web's `actionButton="hover"`.
    pub fn actions_on_hover(mut self, hover: bool) -> Self {
        self.actions_hover = hover;
        self
    }

    /// Whether the rows stand as cards now.
    fn gridded(&self) -> bool {
        self.cards.is_some() && self.state.grid
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

    /// A chevron at the end of every row, saying a press on it leaves the list for the row's
    /// page: an end column 32 wide, as the web's `chevron`.
    pub fn chevron(mut self, chevron: bool) -> Self {
        self.chevron = chevron;
        self
    }

    /// Every row this tall, for cells of two lines (`Cell::Lines`); the density's row by default.
    /// A table that fits its room, as the web's `minWidth="auto"`: its row menu keeps the row's
    /// padding after it rather than standing pinned at the row's very end.
    pub fn fits(mut self, fits: bool) -> Self {
        self.fits = fits;
        self
    }

    /// Rows as tall as their cells' words wrapped onto lines, 8 over and under them and never
    /// under a row's height, as the web's `rowHeight="auto"`.
    pub fn auto_rows(mut self) -> Self {
        self.auto = true;
        self
    }

    /// Each row opens into `detail`, drawn under it across the table's width from where its first
    /// cell starts, as the web's `detail`: a chevron before the first cell opens and closes it, and
    /// so does a press on the row. The open rows are the state's `expanded`.
    pub fn detail(mut self, detail: impl Fn(usize, &mut Ui) + 'a) -> Self {
        self.detail = Some(Box::new(detail));
        self
    }

    /// The rows under a head for each group, by each row's key (one for each row, in the rows'
    /// order), the groups in the order their keys first come among the rows as given, as the web's
    /// `groupBy`: the head says the key and how many rows it holds, and a press on it folds the
    /// group shut or opens it. The folded groups are the state's `folded`. A paged table draws no
    /// groups, as the web's.
    pub fn groups<S: Into<String>>(mut self, keys: impl IntoIterator<Item = S>) -> Self {
        self.groups = Some(keys.into_iter().map(Into::into).collect());
        self
    }

    /// The row shown in a detail beside the list, by its index in the rows given, as the web's
    /// `opened`: drawn as opened (a row or a listed row on its opened ground, a card ringed). A
    /// press picks a row into `TableState::opened`, the app's to take; only this draws it.
    pub fn opened(mut self, row: Option<usize>) -> Self {
        self.opened = row;
        self
    }

    /// The group keys to put first, in this order, as the web's `groupOrder`; the other groups
    /// follow in the order their keys first come.
    pub fn group_order<S: Into<String>>(mut self, keys: impl IntoIterator<Item = S>) -> Self {
        self.group_order = keys.into_iter().map(Into::into).collect();
        self
    }

    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = Some(height);
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

    /// How the active cell of an editable table is marked: a line along its foot by default.
    pub fn cell_focus(mut self, focus: CellFocus) -> Self {
        self.cell_focus = focus;
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
    pub fn keys<K: egui::AsIdSalt>(mut self, keys: impl IntoIterator<Item = K>) -> Self {
        self.keys = keys.into_iter().map(Id::unique).collect();
        self
    }

    /// A bar over the rows with a search field (reached by "/") that edits `text`, the rows' count
    /// and a sort menu of the columns that sort; the app narrows the rows by what is typed.
    pub fn search(mut self, text: &'a mut String) -> Self {
        self.search = Some(text);
        self.bar = true;
        self
    }

    /// The search field's word in place of "Search", such as what it searches by: "Identifier".
    pub fn search_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.search_placeholder = Some(placeholder.into());
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
            + GAP
                * (self.columns.len().saturating_sub(1)
                    + band_edges(self.columns, &self.state.hidden)
                        .iter()
                        .filter(|e| **e)
                        .count()) as f32
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

    /// A line of the bar under its search, as the web's `toolbar`: a filter bar.
    pub fn toolbar(mut self, toolbar: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.toolbar = Some(Box::new(toolbar));
        self.bar = true;
        self
    }

    /// The rows a page at a time, as the web's `paging="client"`: `sizes` the rows to a page the
    /// reader picks from, the first to start with, and a pager under the rows.
    /// The head row left out, for a list whose columns say what they hold without names, as the
    /// web's `columnHeads={false}`.
    pub fn heads(mut self, heads: bool) -> Self {
        self.heads = heads;
        self
    }

    pub fn paging(mut self, sizes: &[usize]) -> Self {
        self.paging = Some(sizes.to_vec());
        self
    }

    /// A line inside the frame under the rows, in the code face, as the web's `foot`: what the
    /// rows add up to.
    pub fn foot(mut self, foot: impl Into<String>) -> Self {
        self.foot = Some(foot.into());
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
        let name = self
            .columns
            .get(open.column)
            .map(|c| c.title.clone())
            .unwrap_or_default();
        let moved = match editor {
            Editor::Text | Editor::Number => {
                let moved = table_edit::text(ui, place, &mut open, id);
                ui.ctx().accesskit_node_builder(id, |b| b.set_label(name));
                moved
            }
            Editor::Date(week_start) => {
                let moved = table_edit::date(ui, place, &mut open, id, *week_start, self.today, &self.words.edit);
                ui.ctx().accesskit_node_builder(id.with("field"), |b| b.set_label(name));
                moved
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
        (pad, lead, tree): (f32, f32, Option<StackedTree>),
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
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Move"));
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
        // a tree's indent and the control that opens the row, 4 before what follows, as the web's
        // compact row
        if let Some(tree) = tree {
            x += tree.indent;
            let control = Rect::from_min_size(pos2(x, line.center().y - 10.0), vec2(20.0, 20.0));
            self.tree_control(ui, t, control, index, tree.branch, tree.waiting);
            x += 24.0;
        }
        let rows = self.rows;
        let row = &rows[index];
        for (c, column) in self.columns.iter().enumerate() {
            if column.stack == StackPlace::Lead {
                let width = lead_width(ui, t, &row[c], column.width);
                let place = Rect::from_min_size(pos2(x, line.top()), vec2(width, line.height()));
                if paint_cell(ui, t, &row[c], place, false, self.id.with(("cell", index, c)), false).is_some() {
                    toggled = true;
                }
                x += width + 8.0;
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
                    paint_cell(ui, t, &row[c], place, false, self.id.with(("cell", index, c)), false);
                    if edit.is_some() {
                        // named by what it says, as the web's title button is
                        let hit = ui.interact(place, self.id.with(("edit", index, c)), Sense::click());
                        // an empty value is named by its column
                        let words = Some(row[c].draft())
                            .filter(|w| !w.is_empty())
                            .unwrap_or_else(|| self.columns[c].title.clone());
                        hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &words));
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
                // an empty value is named by its column
                let words = Some(row[c].draft())
                    .filter(|w| !w.is_empty())
                    .unwrap_or_else(|| column.title.clone());
                hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &words));
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
        // a control tall with its search or its toolbar, a small control without
        let toolbar = self.toolbar.take();
        let searched = self.search.is_some();
        let control = super::finger_tall(
            ui.ctx(),
            t,
            t.density(if searched || toolbar.is_some() {
                "control"
            } else {
                "control-sm"
            }),
        );
        // the toolbar's line under the search, as tall as a control, 8 under it; with no search
        // the toolbar stands in the search's place
        let line = if toolbar.is_some() && searched {
            t.density("control") + 8.0
        } else {
            0.0
        };
        let (bar, _) = ui.allocate_exact_size(
            vec2(ui.available_width(), 8.0 + control + 8.0 + line + 1.0),
            Sense::hover(),
        );
        if let Some(toolbar) = toolbar {
            let below = if searched { control + 8.0 } else { 0.0 };
            let place = Rect::from_min_size(
                pos2(bar.left() + 12.0, bar.top() + 8.0 + below),
                vec2(
                    bar.width() - 24.0 - if searched { 0.0 } else { 160.0 },
                    t.density("control"),
                ),
            );
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(place)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            child.spacing_mut().item_spacing.x = 8.0;
            toolbar(&mut child);
        }
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
            let mut search = SearchField::new(id, text).keys("/");
            if let Some(placeholder) = &self.search_placeholder {
                search = search.placeholder(placeholder);
            }
            search.show(&mut child);
        }
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        child.spacing_mut().item_spacing.x = 16.0;
        // from the end: the columns' menu, then the view's toggle, then the sort, as the web's bar
        let gridded = self.gridded();
        if !stacked && !gridded && self.columns.len() > 1 {
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
                                .variation("wght", t.weight(crate::Weight::Medium))
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
        if self.cards.is_some() {
            self.view_toggle(&mut child, t);
        }
        let sortable: Vec<usize> = (0..self.columns.len())
            .filter(|c| self.columns[*c].sortable && (stacked || gridded))
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
        // a failed read, or the first rows on their way, have nothing to count
        if self.failing() || (self.loading && self.rows.is_empty()) {
            return;
        }
        let count = self.rows.len();
        let words = if count == 1 { &self.words.row } else { &self.words.rows };
        let mono = crate::mono_font(ui.ctx(), crate::mono_small_size(t, TextRole::Help.font(t).size), 400.0);
        child.label(
            egui::RichText::new(words.replace("{count}", &super::format_value(count as f64, Some(0))))
                .font(mono)
                .color(t.color("muted-foreground")),
        );
    }

    /// The bar's choice of the table or the grid: two icon buttons in one muted group, the one
    /// shown raised on the card's ground, as the web's view toggle.
    fn view_toggle(&mut self, ui: &mut Ui, t: &crate::Tokens) {
        const CELL: f32 = 24.0;
        let (group, _) = ui.allocate_exact_size(vec2(2.0 * CELL + 6.0, CELL + 4.0), Sense::hover());
        ui.painter().rect_filled(group, t.radius("control"), t.color("muted"));
        for (at, (icon, label, grid)) in [
            (Icon::Table2, &self.words.table_view, false),
            (Icon::LayoutGrid, &self.words.grid_view, true),
        ]
        .into_iter()
        .enumerate()
        {
            let cell = Rect::from_min_size(
                pos2(group.left() + 2.0 + at as f32 * (CELL + 2.0), group.top() + 2.0),
                vec2(CELL, CELL),
            );
            let hit = ui.interact(cell, self.id.with(("view", at)), Sense::click());
            hit.widget_info(|| {
                egui::WidgetInfo::selected(egui::Role::Button, true, self.state.grid == grid, label.as_str())
            });
            let chosen = self.state.grid == grid;
            if chosen {
                let corners = CornerRadius::same(t.radius("control").nw.saturating_sub(2));
                ui.painter().add(t.shadow("elevation-raised").as_shape(cell, corners));
                ui.painter().rect_filled(cell, corners, t.color("card"));
            } else if hit.hovered() {
                ui.painter().rect_filled(cell, t.radius("control"), t.color("hover"));
            }
            let ink = if chosen || hit.hovered() {
                t.color("foreground")
            } else {
                t.color("muted-foreground")
            };
            crate::icons::paint(ui.painter(), cell.center(), icon, 14.0, ink);
            super::focus_ring(ui.painter(), t, cell, t.radius("control"), &hit);
            if hit.clicked() {
                self.state.grid = grid;
            }
        }
    }

    /// The rows as cards in a grid, 16 in and 16 apart, as many across as fit at their least width,
    /// stretched to fill the row: each card drawn by the table's `card`, a press on it opening its
    /// row, and the row's actions behind a small button at its top corner.
    fn card_grid(&mut self, ui: &mut Ui, t: &crate::Tokens, order: &[usize], out: &mut TableOutput) {
        let shape = self.tree_shape();
        let Some(cards) = self.cards.take() else { return };
        const GAP: f32 = 16.0;
        let room = ui.available_width() - 2.0 * GAP;
        let across = (((room + GAP) / (cards.min + GAP)).floor() as usize).max(1);
        let width = (room - GAP * (across - 1) as f32) / across as f32;
        // a stretched table's grid scrolls in the room left, another's in its height
        let most = if self.stretch {
            ui.available_height()
        } else {
            self.height
        };
        egui::ScrollArea::vertical()
            .id_salt(self.id.with("grid"))
            .auto_shrink([false, true])
            .max_height(most)
            .show(ui, |ui| {
                ui.add_space(GAP);
                for line in order.chunks(across) {
                    // a line as tall as its tallest card, `height` at least
                    let row = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), cards.height));
                    let mut tallest = cards.height;
                    for (at, &index) in line.iter().enumerate() {
                        let place = Rect::from_min_size(
                            pos2(row.left() + GAP + at as f32 * (width + GAP), row.top()),
                            vec2(width, cards.height),
                        );
                        let mut child = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(place)
                                .id_salt(("card", index))
                                .layout(egui::Layout::top_down(egui::Align::Min)),
                        );
                        let pressed = (cards.card)(&mut child, index, width);
                        tallest = tallest.max(pressed.rect.height());
                        let card = Rect::from_min_size(place.min, vec2(width, pressed.rect.height().max(cards.height)));
                        let selected = self.state.selected.contains(&index);
                        // a chosen card ringed in its paint, an opened one in its own, outside its
                        // edge, as the web's `ring-2 ring-primary` and `ring-2 ring-ring/60`
                        let ring = if selected {
                            t.paint("table.base.selected.card")
                        } else if self.opened == Some(index) {
                            t.paint("table.base.opened.card")
                        } else {
                            None
                        };
                        if let Some((color, width)) = ring.and_then(|p| p.ring) {
                            ui.painter().rect_stroke(
                                card,
                                t.radius("surface"),
                                Stroke::new(width, color),
                                egui::StrokeKind::Outside,
                            );
                        }
                        // the selection box at the card's top corner, 8 in, on the page's ground at
                        // 85% padded 4: shown while the card is pointed at, chosen, or any row is,
                        // as the web's grid box; a press on it selects the row alone
                        if self.selectable
                            && (selected || !self.state.selected.is_empty() || ui.rect_contains_pointer(card))
                        {
                            let pad = Rect::from_min_size(card.min + vec2(8.0, 8.0), vec2(24.0, 24.0));
                            ui.painter()
                                .rect_filled(pad, t.radius("md"), t.color_at("background", 0.85));
                            let mut checked = selected;
                            if put_at(
                                ui,
                                pad.shrink(4.0),
                                Checkbox::new(&mut checked, self.words.select_row.clone()).hide_label(),
                            )
                            .clicked()
                            {
                                let groups = self.groups.as_deref().filter(|g| g.len() == self.rows.len());
                                toggle_selection(self.state, shape.as_ref(), groups, order, index, false);
                            }
                        }
                        if pressed.clicked()
                            && !(self.selectable
                                && select_press(
                                    ui,
                                    self.state,
                                    shape.as_ref(),
                                    self.groups.as_deref().filter(|g| g.len() == self.rows.len()),
                                    order,
                                    index,
                                ))
                        {
                            self.state.opened = Some(index);
                        }
                        // the row's actions at the card's top corner, 10 in
                        if let Some(actions) = &self.actions {
                            let items = actions(index);
                            if !items.is_empty() {
                                let button = Rect::from_min_size(
                                    pos2(place.right() - 10.0 - 20.0, place.top() + 10.0),
                                    vec2(20.0, 20.0),
                                );
                                let trigger = ui.interact(button, self.id.with(("card-menu", index)), Sense::click());
                                trigger.widget_info(|| {
                                    egui::WidgetInfo::labeled(egui::Role::Button, true, self.words.actions.as_str())
                                });
                                let ground = if trigger.hovered() {
                                    t.color("hover")
                                } else {
                                    t.color("card")
                                };
                                ui.painter().rect_filled(button, t.radius("control"), ground);
                                ui.painter().rect_stroke(
                                    button,
                                    t.radius("control"),
                                    Stroke::new(1.0, t.color("border")),
                                    egui::StrokeKind::Inside,
                                );
                                crate::icons::paint(
                                    ui.painter(),
                                    button.center(),
                                    Icon::Ellipsis,
                                    12.0,
                                    t.color("foreground"),
                                );
                                if let Some(chosen) = DropdownMenu::new(&items).show(&trigger) {
                                    out.action = Some((index, chosen));
                                }
                            }
                        }
                    }
                    ui.allocate_exact_size(vec2(row.width(), tallest), Sense::hover());
                    ui.add_space(GAP);
                }
            });
        self.cards = Some(cards);
    }

    /// An editable line's field in a stacked row, at `field`: its open editor in a box of its own,
    /// or the value (a refused draft marked) in a field's box that opens the editor on a press, as
    /// the web's stacked editing line. The saved edit, if any.
    fn stacked_field(
        &mut self,
        ui: &mut Ui,
        t: &crate::Tokens,
        field: Rect,
        (index, c): (usize, usize),
        editor: &Editor,
        editing: &Editing<'_>,
    ) -> Option<CellEdit> {
        let column = &self.columns[c];
        let open_here = self
            .state
            .open
            .as_ref()
            .is_some_and(|o| (o.row, o.column) == (index, c));
        if open_here {
            if let Some(open) = &mut self.state.open {
                open.boxed = true;
            }
            let id = self.id.with(("editor", editing.keys[index], c));
            return self.run_editor(ui, editor, field, id, editing);
        }
        let refused = self.state.refused.get(&(editing.keys[index], c));
        let radius = t.radius("control");
        let (ground, edge) = if refused.is_some() {
            (t.color_at("critical", 0.08), t.color_at("critical", 0.8))
        } else {
            (t.color("field"), t.color("input"))
        };
        ui.painter().rect_filled(field, radius, ground);
        ui.painter()
            .rect_stroke(field, radius, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
        let words = refused.map_or_else(|| self.rows[index][c].draft(), |(draft, _)| draft.clone());
        let inner = field.shrink2(vec2(10.0, 0.0));
        let galley = WidgetText::from(TextRole::Body.rich(t, &words)).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            inner.width().max(1.0),
            TextRole::Body.font(t),
        );
        let x = if column.numeric {
            inner.right() - galley.size().x
        } else {
            inner.left()
        };
        ui.painter().galley(
            pos2(x, inner.center().y - galley.size().y / 2.0),
            galley,
            t.color("foreground"),
        );
        let hit = ui.interact(field, self.id.with(("edit", index, c)), Sense::click());
        let name = match &column.band {
            Some(band) => format!("{band} · {}", column.title),
            None => column.title.clone(),
        };
        hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &name));
        if hit.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Text);
        }
        if let Some((_, error)) = refused
            && hit.hovered()
        {
            hit.clone().on_hover_text(error.clone());
        }
        if hit.clicked() {
            open_cell(self.state, self.rows, editing.keys, (index, c));
        }
        None
    }

    /// The rows as labeled cards (`StackLayout::Labeled`), 16 in and 16 apart: each card 12 across
    /// and 8 down, its title on a 24 line with the menu at its end, a 20 line for each `Line`
    /// column with a value, its name before it as wide as the widest name, and the `Secondary`
    /// columns on one 24 line of facts. A card's height follows its lines, so every card is drawn.
    /// A line whose column edits is always drawn, a field that opens its editor on a press in its
    /// place, a control tall (36 in a card); a line's name is its band before its own. Grouped rows
    /// stand under their group heads, as a table's.
    fn labeled_cards(
        &mut self,
        ui: &mut Ui,
        t: &crate::Tokens,
        order: &[usize],
        (keys, side): (&[Id], f32),
        out: &mut TableOutput,
    ) {
        let shape = self.tree_shape();
        let columns = self.columns;
        let rows = self.rows;
        let groups: Option<Vec<String>> = self.groups.clone().filter(|g| g.len() == rows.len());
        // a column's name where it stands alone: its band before its own, as the web's `columnName`
        let name_of = |c: usize| match &columns[c].band {
            Some(band) => format!("{band} · {}", columns[c].title),
            None => columns[c].title.clone(),
        };
        let field_height = if self.variant == TableVariant::List {
            t.density("control")
        } else {
            36.0
        };
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
                WidgetText::from(TextRole::Help.rich(t, name_of(*c)))
                    .into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, TextRole::Help.font(t))
                    .size()
                    .x
            })
            .fold(0.0, f32::max)
            .min(192.0);
        let said = |cell: &Cell| {
            !matches!(cell, Cell::Empty)
                && !matches!(cell, Cell::Tags(tags) | Cell::OutlineTags(tags) if tags.is_empty())
        };
        let cards_id = self.id.with("cards");
        // a table shown whole (`height` without bound) lays its rows in the page, which scrolls
        let whole = !self.height.is_finite();
        let mut body = |ui: &mut Ui| {
            ui.add_space(apart);
            let mut last_group: Option<&String> = None;
            for (position, &index) in order.iter().enumerate() {
                let row = &rows[index];
                if let Some(groups) = &groups {
                    let key = &groups[index];
                    if last_group != Some(key) {
                        last_group = Some(key);
                        let shut = self.state.folded.contains(key);
                        let head = GroupHead {
                            key,
                            held: order.iter().filter(|i| groups[**i] == *key).count(),
                            shut,
                            start: side,
                            label_end: None,
                            aggregates: &[],
                            // a listed group's head is its words 6 over and under, as the web's `py-1.5`
                            height: if listed {
                                t.text(TextRole::ColumnHead.key()).line_height + 12.0
                            } else {
                                t.density("row")
                            },
                        };
                        if group_head(ui, t, &head, &self.words) {
                            if shut {
                                self.state.folded.remove(key);
                            } else {
                                self.state.folded.insert(key.clone());
                            }
                        }
                    }
                    if self.state.folded.contains(key) {
                        continue;
                    }
                }
                // an editable line is always drawn, as the field it is edited in
                let shown: Vec<usize> = lines
                    .iter()
                    .copied()
                    .filter(|c| columns[*c].edit.is_some() || said(&row[*c]))
                    .collect();
                let line_height = |c: usize| {
                    if columns[c].edit.is_some() { field_height } else { 20.0 }
                };
                let primaries: Vec<usize> = primary.iter().copied().filter(|c| said(&row[*c])).collect();
                let has_facts = facts.iter().any(|c| said(&row[*c]));
                // a tree's row: its control in the title's line after its indent; listed, every line
                // starts where its title does, as the web's listed rows
                let opens = shape.is_some() || self.detail.is_some();
                let depth = shape.as_ref().map_or(0, |s| s.depth[index]) as f32;
                let refused = self.state.refusals.contains(&index);
                let tree = opens.then(|| StackedTree {
                    indent: depth * 20.0,
                    branch: self.detail.is_some() || shape.as_ref().is_some_and(|s| s.branch(index)),
                    waiting: self.state.expanded.contains(&index)
                        && shape.as_ref().is_some_and(|s| s.lazy[index])
                        && !refused,
                });
                let indent = if listed && opens { depth * 20.0 + 24.0 } else { 0.0 };
                let text_width = (ui.available_width() - inset * 2.0 - 2.0 * (1.0 + 12.0) - indent).max(1.0);
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
                    + shown.iter().map(|c| line_height(*c)).sum::<f32>()
                    + if has_facts { 24.0 } else { 0.0 }
                    + gap * parts as f32
                    + pad
                    + 1.0;
                let width = ui.available_width() - inset * 2.0;
                let (outer, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
                let card = Rect::from_min_size(pos2(outer.left() + inset, outer.top()), vec2(width, height));
                card_rects.push(card);
                let hit = ui.interact(card, self.id.with(("card", index)), Sense::click());
                if hit.clicked()
                    && !(self.selectable
                        && select_press(
                            ui,
                            self.state,
                            shape.as_ref(),
                            self.groups.as_deref().filter(|g| g.len() == self.rows.len()),
                            order,
                            index,
                        ))
                {
                    self.state.opened = Some(index);
                }
                let selected = self.state.selected.contains(&index);
                let opened = self.opened == Some(index);
                if listed {
                    // a chosen or opened listed row on its paint's ground, hovered or not, as the
                    // web's `bg-secondary hover:bg-secondary`
                    let chosen = if selected {
                        t.paint("table.base.selected.row")
                    } else if opened {
                        t.paint("table.base.opened.row")
                    } else {
                        None
                    };
                    if let Some(ground) = chosen.and_then(|p| p.ground) {
                        ui.painter().rect_filled(card, 0.0, ground);
                    } else if hit.hovered() {
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
                    // a chosen card ringed in its paint, an opened one in its own, outside its
                    // edge, as the web's `ring-2 ring-primary` and `ring-2 ring-ring/60`
                    let ring = if selected {
                        t.paint("table.base.selected.card")
                    } else if opened {
                        t.paint("table.base.opened.card")
                    } else {
                        None
                    };
                    if let Some((color, width)) = ring.and_then(|p| p.ring) {
                        ui.painter()
                            .rect_stroke(card, radius, Stroke::new(width, color), egui::StrokeKind::Outside);
                    }
                }
                let left = card.left() + 1.0 + 12.0;
                let right = card.right() - 1.0 - 12.0;
                let mut y = card.top() + 1.0 + pad;
                // the grip before the title, dragged to move the card
                let mut title_left = left;
                if self.reorder {
                    let grip = Rect::from_min_size(pos2(left, y), vec2(GRIP, 24.0));
                    let hold = ui.interact(grip, self.id.with(("card-grip", index)), Sense::drag());
                    hold.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Move"));
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
                // the selection box before the tree's control and the title, 8 before them; a
                // press on it selects the row alone, as the web's stacked `boxFor`
                if self.selectable {
                    let mut checked = selected;
                    let place = Rect::from_min_size(pos2(title_left, y + 12.0 - 8.0), vec2(16.0, 16.0));
                    if put_at(
                        ui,
                        place,
                        Checkbox::new(&mut checked, self.words.select_row.clone()).hide_label(),
                    )
                    .clicked()
                    {
                        let groups = self.groups.as_deref().filter(|g| g.len() == self.rows.len());
                        toggle_selection(self.state, shape.as_ref(), groups, order, index, false);
                    }
                    title_left += 16.0 + 8.0;
                }
                if let Some(tree) = tree {
                    let control =
                        Rect::from_min_size(pos2(title_left + tree.indent, y + 12.0 - 10.0), vec2(20.0, 20.0));
                    self.tree_control(ui, t, control, index, tree.branch, tree.waiting);
                    title_left += tree.indent + 24.0;
                }
                let left = left + indent;
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
                    trigger.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Actions"));
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
                    let height = line_height(c);
                    let name = WidgetText::from(TextRole::Help.rich(t, name_of(c))).into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        name_width,
                        TextRole::Help.font(t),
                    );
                    ui.painter().galley(
                        pos2(left, y + height / 2.0 - name.size().y / 2.0),
                        name,
                        t.color("muted-foreground"),
                    );
                    let x = left + name_width + 8.0;
                    let place = Rect::from_min_max(pos2(x, y), pos2(right, y + height));
                    match &columns[c].edit {
                        Some(editor) => {
                            let field =
                                Rect::from_center_size(place.center(), vec2(place.width(), t.density("control")));
                            let editing = Editing { keys, order, position };
                            if let Some(edit) = self.stacked_field(ui, t, field, (index, c), editor, &editing) {
                                out.edited = Some(edit);
                            }
                        }
                        None => line_value(ui, t, &row[c], place),
                    }
                    y += height + gap;
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
                            ui.painter()
                                .galley(pos2(x, middle - dot.size().y / 2.0), dot.clone(), t.color("faint"));
                            x += dot.size().x + 6.0;
                        }
                        first = false;
                        x += paint_fact(ui, t, &row[c], pos2(x, middle), (right - x).max(1.0));
                    }
                }
                // listed, the line under a branch whose children did not load, from where its
                // children would start, padded 8 over and under, as the web's listed refusal
                if listed && refused && shape.is_some() {
                    let height = TextRole::Body.line_height(t) + 16.0 + 1.0;
                    let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
                    ui.painter()
                        .hline(line.x_range(), line.bottom() - 0.5, Stroke::new(1.0, t.color("border")));
                    let start = line.left() + (depth + 1.0) * 20.0 + 24.0;
                    self.refused_line(ui, t, line, (start, 12.0), index);
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
        };
        if whole {
            body(ui);
        } else {
            egui::ScrollArea::vertical()
                .id_salt(cards_id)
                .auto_shrink([false, true])
                .show(ui, |ui| body(ui));
        }
    }

    /// A row's control that opens its branch or its detail, 20 square in `control`: a spinner
    /// while its children load (`waiting`), a chevron on a branch, room alone on a row with nothing
    /// to open. A press opens or closes it; true when pressed.
    fn tree_control(
        &mut self,
        ui: &mut Ui,
        t: &crate::Tokens,
        control: Rect,
        index: usize,
        branch: bool,
        waiting: bool,
    ) -> bool {
        let open = self.state.expanded.contains(&index);
        if waiting {
            let hit = ui.interact(control, self.id.with(("expand", index)), Sense::hover());
            let words = &self.words.loading_children;
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ProgressIndicator, true, words));
            super::spinner(ui, ui.painter(), control.center(), 5.5, t.color("muted-foreground"));
            return false;
        }
        if !branch {
            return false;
        }
        let hit = ui.interact(control, self.id.with(("expand", index)), Sense::click());
        let words = match &self.expand_label {
            Some(label) => label(index, open),
            None if open => self.words.collapse.clone(),
            None => self.words.expand.clone(),
        };
        hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &words));
        if hit.hovered() {
            ui.painter().rect_filled(control, t.radius("sm"), t.color("accent"));
        }
        icons::paint(
            ui.painter(),
            control.center(),
            if open { Icon::ChevronDown } else { Icon::ChevronRight },
            14.0,
            t.color(if hit.hovered() {
                "foreground"
            } else {
                "muted-foreground"
            }),
        );
        let clicked = hit.clicked();
        hit.on_hover_text(words);
        if clicked && !self.state.expanded.remove(&index) {
            self.state.expanded.insert(index);
        }
        clicked
    }

    /// What the line under a branch whose children did not load says, in `line` from `left` (`pad`
    /// kept at its end): the refusal in `critical`, then Try again 8 after it, which asks again.
    fn refused_line(&mut self, ui: &mut Ui, t: &crate::Tokens, line: Rect, (left, pad): (f32, f32), index: usize) {
        let retry = WidgetText::from(TextRole::Help.rich(t, &self.words.retry).color(t.color("foreground")))
            .into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, TextRole::Help.font(t));
        // the refusal cut short where Try again would not fit after it, which keeps its width
        let said = WidgetText::from(TextRole::Body.rich(t, &self.words.refused).color(t.color("critical")))
            .into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                (line.right() - pad - left - 8.0 - 18.0 - retry.size().x).max(1.0),
                TextRole::Body.font(t),
            );
        let said_w = said.size().x;
        // read as the words it says
        let said_at = Rect::from_min_size(pos2(left, line.center().y - said.size().y / 2.0), said.size());
        let refusal = &self.words.refused;
        ui.interact(said_at, self.id.with(("refused-children", index)), Sense::hover())
            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, refusal));
        ui.painter().galley(said_at.min, said, t.color("critical"));
        // the icon 14, 4 before the words, 8 after the refusal
        let button = Rect::from_min_size(
            pos2(left + said_w + 8.0, line.center().y - 10.0),
            vec2(14.0 + 4.0 + retry.size().x, 20.0),
        );
        let hit = ui.interact(button, self.id.with(("retry-children", index)), Sense::click());
        let words = &self.words.retry;
        hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, words));
        if hit.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        icons::paint(
            ui.painter(),
            pos2(button.left() + 7.0, button.center().y),
            Icon::RotateCw,
            14.0,
            t.color("foreground"),
        );
        let at = pos2(button.left() + 18.0, button.center().y - retry.size().y / 2.0);
        let words_rect = Rect::from_min_size(at, retry.size());
        ui.painter().galley(at, retry, t.color("foreground"));
        if hit.hovered() {
            ui.painter().hline(
                words_rect.x_range(),
                words_rect.bottom() - 1.0,
                Stroke::new(1.0, t.color("foreground")),
            );
        }
        if hit.clicked() {
            self.state.refusals.remove(&index);
            self.state.asked.remove(&index);
        }
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
        let tree = self.tree_shape();
        if let Some(groups) = self.groups.as_ref().filter(|g| g.len() == self.rows.len()) {
            // the groups in the order their keys first come among the rows as given (not as
            // sorted), as the web's `groupRows`, each keeping its rows' order; in a tree a row
            // stands in its top-level row's group
            let key = |i: usize| &groups[tree.as_ref().map_or(i, |t| t.root(i))];
            let mut first: Vec<&String> = Vec::new();
            for i in (0..self.rows.len()).filter(|i| tree.as_ref().is_none_or(|t| t.parent[*i].is_none())) {
                if !first.contains(&key(i)) {
                    first.push(key(i));
                }
            }
            // the keys `group_order` names first, in its order, then the rest as they came
            let rank = |k: &String| {
                self.group_order
                    .iter()
                    .position(|o| o == k)
                    .unwrap_or(self.group_order.len())
            };
            first.sort_by_key(|k| rank(k));
            order.sort_by_key(|i| first.iter().position(|k| *k == key(*i)).unwrap_or(0));
        }
        match tree {
            Some(tree) => tree.flatten(&order, &self.state.expanded),
            None => order,
        }
    }

    /// The room after a row's last column: none where the actions' column stands at the row's very
    /// end, pinned there as the web's end column of a table with a least width, else the row's own
    /// padding.
    /// A column's head name laid out in its column's width `w`: wrapped between its words onto two
    /// lines at most, past them cut with an ellipsis; room kept for the sort arrow while sorted and
    /// for a row's grip over the first column; lined up at the end in a column read from its end.
    fn head_galley(&self, ui: &Ui, t: &crate::Tokens, index: usize, w: f32) -> std::sync::Arc<egui::Galley> {
        let column = &self.columns[index];
        let upper = t.word("table-head-transform") == Some("uppercase");
        let title = if upper {
            column.title.to_uppercase()
        } else {
            column.title.clone()
        };
        let lead = if index == 0 && self.reorder { GRIP + 6.0 } else { 0.0 };
        let sorted = self.state.sort.is_some_and(|(c, _)| c == index);
        let room = (w - lead - if sorted { 18.0 } else { 0.0 }).max(1.0);
        let mut job = egui::text::LayoutJob::default();
        TextRole::ColumnHead.rich(t, title).append_to(
            &mut job,
            ui.style(),
            egui::FontSelection::Default,
            egui::Align::Center,
        );
        job.halign = if column.numeric {
            egui::Align::Max
        } else {
            egui::Align::Min
        };
        job.wrap = egui::text::TextWrapping {
            max_width: room,
            max_rows: 2,
            break_anywhere: false,
            overflow_character: Some('…'),
        };
        ui.painter().layout_job(job)
    }

    /// The room between the last column and the chevron's: a list's column gap, as its grid has;
    /// a table's end column stands against the last cell's own padding.
    fn chevron_gap(&self) -> f32 {
        if self.variant == TableVariant::List { GAP } else { 0.0 }
    }

    fn end_pad(&self, pad: f32) -> f32 {
        if self.actions.is_some() && !self.fits { 0.0 } else { pad }
    }

    /// Each column's start and width within a row from `left` to `right`, as unscrolled.
    fn unscrolled(&self, left: f32, right: f32) -> Vec<(f32, f32)> {
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
        let edges = band_edges(self.columns, &self.state.hidden);
        let gaps = GAP * (shown.saturating_sub(1) + edges.iter().filter(|e| **e).count()) as f32;
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
                if edges[i] {
                    x += GAP;
                }
                let place = (x, w);
                if !hidden(i) {
                    x += w + GAP;
                }
                place
            })
            .collect()
    }

    /// Each column's start and width within a row from `left` to `right`, scrolled sideways: the
    /// columns running past the room move under the pinned ones by the reader's offset, as the
    /// web's table wider than its box; a table's first column stays, a list's scrolls with the rest.
    fn layout(&self, left: f32, right: f32) -> Vec<(f32, f32)> {
        let mut places = self.unscrolled(left, right);
        let shift = self.state.scroll_x.clamp(0.0, self.overflow_of(&places, right));
        if shift > 0.0 {
            let first = self.pinned_first();
            for (i, place) in places.iter_mut().enumerate() {
                if Some(i) != first {
                    place.0 -= shift;
                }
            }
        }
        places
    }

    /// Where a table's pinned first column ends and its pinned end column starts in `row`, as
    /// the web's sticky first and last cells; a list pins none, and a table with neither a menu
    /// nor a chevron has no end column.
    fn pinned_edges(&self, row: Rect, pad: f32, places: &[(f32, f32)]) -> (Option<f32>, Option<f32>) {
        if self.variant != TableVariant::Table {
            return (None, None);
        }
        let start = self.pinned_first().map(|f| places[f].0 + places[f].1 + GAP / 2.0);
        let chevron = if self.chevron { 32.0 + self.chevron_gap() } else { 0.0 };
        // the end column starts where the menu's does, or with the chevron alone after the last
        // cell's own padding
        let end = if self.actions.is_some() {
            Some(row.right() - self.end_pad(pad) - chevron - ACTIONS)
        } else if self.chevron {
            Some(row.right() - chevron)
        } else {
            None
        };
        (start, end)
    }

    /// How far the columns laid out from `left` to `right` run past their room, unscrolled.
    fn overflow(&self, left: f32, right: f32) -> f32 {
        self.overflow_of(&self.unscrolled(left, right), right)
    }

    /// How far the shown columns in `places` run past their room ending at `right`, before the
    /// actions' column.
    fn overflow_of(&self, places: &[(f32, f32)], right: f32) -> f32 {
        let right = right - if self.actions.is_some() { ACTIONS + GAP } else { 0.0 };
        let end = (0..self.columns.len())
            .filter(|i| !self.state.hidden.contains(i))
            .map(|i| places[i].0 + places[i].1)
            .fold(f32::NEG_INFINITY, f32::max);
        (end - right).max(0.0)
    }

    /// The column pinned at a table's start as the rest scroll under it: its first shown one; a
    /// list pins none.
    fn pinned_first(&self) -> Option<usize> {
        (self.variant == TableVariant::Table)
            .then(|| (0..self.columns.len()).find(|i| !self.state.hidden.contains(i)))
            .flatten()
    }

    /// A placeholder row in `rect` while rows load, the `line`th: a `muted` bar 12 tall at 60
    /// percent of each shown cell (a stacked row's title and facts lines), pulsing unless motion
    /// is off, and the row's rule.
    fn placeholder_row(
        &self,
        ui: &Ui,
        t: &crate::Tokens,
        rect: Rect,
        (pad, select_width): (f32, f32),
        stacked: bool,
        line: usize,
    ) {
        let painter = ui.painter();
        let ground = t.color_at("muted", pulse(ui));
        let radius = t.radius("md");
        let bar = |left: f32, width: f32, middle: f32| {
            let place = Rect::from_min_size(pos2(left, middle - 6.0), vec2(width.max(0.0), 12.0));
            painter.rect_filled(place, radius, ground);
        };
        if stacked {
            // the title's line and the facts' line, as a stacked row stands
            let left = rect.left() + pad + select_width;
            let room = rect.right() - pad - left;
            // the web's placeholder widths, two fifths to three fifths, turning with the line
            let share = [0.4, 0.5, 1.0 / 3.0, 0.6][line % 4];
            bar(left, room * 0.6, rect.top() + 8.0 + 12.0);
            bar(left, room * share, rect.top() + 8.0 + 24.0 + 10.0);
        } else {
            let end = if self.chevron { 32.0 + self.chevron_gap() } else { 0.0 };
            let places = self.layout(rect.left() + pad + select_width, rect.right() - self.end_pad(pad) - end);
            for (c, column) in self.columns.iter().enumerate() {
                if self.state.hidden.contains(&c) {
                    continue;
                }
                let (x, w) = places[c];
                let width = w * 0.6;
                let left = if column.numeric { x + w - width } else { x };
                bar(left, width, rect.center().y);
            }
            let row = Rect::from_x_y_ranges(rect.x_range(), rect.top()..=rect.bottom() - 1.0);
            pinned_rules(painter, t, row, self.pinned_edges(rect, pad, &places));
        }
        painter.hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, t.color("border")));
    }

    /// The failed read in place of the rows, as the web's unframed `PageState`: the `destructive`
    /// mark, the heading by what the server answered at 500, the diagnostic and each problem in
    /// `text.help`, and Try again as an outline small button, padded `density.panel-pad` across
    /// and 12 down. True when Try again was pressed.
    fn failure_line(&self, ui: &mut Ui) -> bool {
        let Some(failure) = &self.failure else {
            return false;
        };
        let words = super::PageStateWords {
            refused: self.words.refused.clone(),
            missing: self.words.missing.clone(),
            unreachable: self.words.unreachable.clone(),
            retry: self.words.retry.clone(),
            ..Default::default()
        };
        let error = failure.error.clone();
        let state = match failure.status {
            ReadOutcome::Status(404) => super::PageState::missing(error),
            ReadOutcome::Unreachable => super::PageState::unreachable(error),
            ReadOutcome::Status(_) => super::PageState::refused(error),
        };
        state
            .problems(self.problems.iter().cloned())
            .framed(false)
            .retry(self.retry)
            .words(words)
            .show(ui)
    }

    /// Draw the table, stacked top to bottom whatever layout the row around it has.
    pub fn show(self, ui: &mut Ui) -> TableOutput {
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| self.show_stacked(ui))
            .inner
    }

    fn show_stacked(mut self, ui: &mut Ui) -> TableOutput {
        let t = tokens(ui.ctx());
        // a paged table is never grouped, as the web's `grouped` (groupBy without paging): its
        // rows are drawn, paged and ranged as one list
        if self.paging.is_some() {
            self.groups = None;
        }
        if self.state.widths.len() != self.columns.len() {
            self.state.widths = self.columns.iter().map(|c| c.width).collect();
        }
        let row_height = self.row_height.unwrap_or(t.density("row"));
        let head_height = match self.variant {
            TableVariant::Table => row_height,
            TableVariant::List => t.density("panel-head"),
        };
        let pad = match self.variant {
            TableVariant::Table => 12.0,
            TableVariant::List => t.density("panel-pad"),
        };
        // the web's `CHECK`, 2.5rem, the box 12 in from its start
        let select_width = if self.selectable { 28.0 + 12.0 } else { 0.0 };
        let stacked = self.stacks(ui.available_width(), pad, select_width);
        // a stacked row: 8 over and under a 24 title line and a 20 line of facts, and its rule
        let row_height = if stacked {
            8.0 + 24.0 + 20.0 + 8.0 + 1.0
        } else {
            row_height
        };
        let head_height = if stacked || !self.heads { 0.0 } else { head_height };
        let keys: Vec<Id> = (0..self.rows.len())
            .map(|index| match self.keys.get(index) {
                Some(key) if self.keys.len() == self.rows.len() => *key,
                _ => Id::unique(("row", index)),
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
        // a tree's open branches whose children are not given yet ask for them, each once while it
        // waits; one closed, or whose children came, is forgotten
        let shape = self.tree_shape();
        let mut load_children = None;
        if let Some(shape) = &shape {
            let expanded = self.state.expanded.clone();
            let waiting = |i: usize| shape.lazy.get(i).copied().unwrap_or(false) && expanded.contains(&i);
            self.state.asked.retain(|i| waiting(*i));
            // a refusal is kept while its branch stays open and waits
            self.state.refusals.retain(|i| waiting(*i));
            let mut asking = order
                .iter()
                .copied()
                .filter(|i| waiting(*i) && !self.state.asked.contains(i) && !self.state.refusals.contains(i));
            load_children = asking.next();
            if asking.next().is_some() {
                // the next branch is asked on the next frame
                ui.ctx().request_repaint();
            }
            if let Some(row) = load_children {
                self.state.asked.insert(row);
            }
        }
        let failing = self.failing();
        // the first rows on their way, or a failed read: nothing to count yet
        let uncounted = failing || (self.loading && self.rows.is_empty());
        let has_detail = self.detail.is_some();
        // the room before a row's first cell: its tree indent, 20 a level, and the 20 control
        // that opens it with 4 after it
        let lead_of = |index: usize| -> f32 {
            match &shape {
                Some(shape) => shape.depth.get(index).copied().unwrap_or(0) as f32 * 20.0 + 24.0,
                None if has_detail => 24.0,
                None => 0.0,
            }
        };
        let tree_keys = shape.is_some() && !self.columns.iter().any(|c| c.edit.is_some());
        // a paged table draws its page of the rows in their order; a tree's pages count its
        // top-level rows, each with the open rows under it, so a page never parts a row from its
        // children
        let starts: Vec<usize> = match &shape {
            Some(shape) => (0..order.len()).filter(|p| shape.parent[order[*p]].is_none()).collect(),
            None => (0..order.len()).collect(),
        };
        let total = starts.len();
        let order = match &self.paging {
            Some(sizes) => {
                if self.state.page_size == 0 {
                    self.state.page_size = sizes.first().copied().unwrap_or(10).max(1);
                }
                let size = self.state.page_size;
                let pages = total.div_ceil(size).max(1);
                self.state.page = self.state.page.min(pages - 1);
                let start_of = |unit: usize| starts.get(unit).copied().unwrap_or(order.len());
                order[start_of(self.state.page * size)..start_of((self.state.page + 1) * size)].to_vec()
            }
            None => order,
        };
        let radius = t.radius("surface");
        let width = ui.available_width();
        let room = ui.available_height();
        // where a stretched table's frame must end: the room's bottom, less the row count under it
        let limit = ui.cursor().top() + room - if self.footer { 26.0 } else { 0.0 };
        let foot_room = if self.foot.is_some() { 32.0 } else { 0.0 };
        let mut out = TableOutput {
            response: ui.response(),
            toggled: None,
            switched: None,
            pressed: None,
            action: None,
            reordered: None,
            edited: None,
            load_children,
            retry: false,
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
                if let Some(bulk) = self.bulk.take()
                    && !self.state.selected.is_empty()
                {
                    let chosen = self.state.selected.clone();
                    bulk(ui, &chosen);
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
                    ui.set_min_height((room - 2.0 - if self.footer { 26.0 } else { 0.0 }).max(0.0));
                }
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                if self.bar {
                    self.draw_bar(ui, &t, stacked);
                }
                // a table with no rows given its own words says them alone, without its heads
                if self.rows.is_empty()
                    && !self.loading
                    && !failing
                    && let Some(empty) = self.empty.take()
                {
                    empty(ui);
                    return;
                }
                // the cards' grid in place of the rows, while it is chosen
                if self.gridded() && !self.rows.is_empty() && !failing {
                    self.card_grid(ui, &t, &order, &mut out);
                    return;
                }
                // labeled cards in place of the rows
                if stacked && self.stack_layout == StackLayout::Labeled && !self.rows.is_empty() && !failing {
                    self.labeled_cards(ui, &t, &order, (&keys, pad + select_width), &mut out);
                    return;
                }
                // columns running past the room scroll sideways under the pointer, as the web's
                // table wider than its box, the offset kept within what runs past
                if !stacked {
                    let room = ui.available_rect_before_wrap();
                    let end = if self.chevron { 32.0 + self.chevron_gap() } else { 0.0 };
                    let over = self.overflow(room.left() + pad + select_width, room.right() - self.end_pad(pad) - end);
                    let mut x = self.state.scroll_x.clamp(0.0, over);
                    if over > 0.0 && ui.rect_contains_pointer(room) {
                        let dx = ui.input(|i| i.smooth_scroll_delta.x);
                        if dx != 0.0 {
                            x = (x - dx).clamp(0.0, over);
                            ui.input_mut(|i| i.smooth_scroll_delta.x = 0.0);
                        }
                    }
                    self.state.scroll_x = x;
                }
                // the head, which a stacked table has none of
                // a head's name too wide for its column wraps onto a second line, and the head row
                // grows to hold it: one line padded by half of what the row leaves over it
                let head_height = if stacked || !self.heads {
                    head_height
                } else {
                    let room = ui.available_rect_before_wrap();
                    let end = if self.chevron { 32.0 + self.chevron_gap() } else { 0.0 };
                    let places = self.layout(room.left() + pad + select_width, room.right() - self.end_pad(pad) - end);
                    let line = t.text(TextRole::ColumnHead.key()).line_height;
                    let lines = (0..self.columns.len())
                        .filter(|i| !self.state.hidden.contains(i))
                        .map(|i| self.head_galley(ui, &t, i, places[i].1).rows.len())
                        .max()
                        .unwrap_or(1)
                        .clamp(1, 2);
                    head_height + (lines - 1) as f32 * line
                };
                // columns under bands: a line of band heads above theirs
                let banded = !stacked && self.heads && self.columns.iter().any(|c| c.band.is_some());
                if banded {
                    let band_height = match self.variant {
                        TableVariant::Table => row_height,
                        TableVariant::List => t.density("panel-head"),
                    };
                    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), band_height), Sense::hover());
                    let top = CornerRadius { sw: 0, se: 0, ..radius };
                    ui.painter()
                        .rect_filled(row, top, t.color_or("table-head-ground", t.color("card")));
                    ui.painter().hline(
                        row.x_range(),
                        row.bottom() - 0.5,
                        Stroke::new(1.0, t.color_or("table-head-line", t.color("border"))),
                    );
                    let end = if self.chevron { 32.0 + self.chevron_gap() } else { 0.0 };
                    let places = self.layout(row.left() + pad + select_width, row.right() - self.end_pad(pad) - end);
                    let shown: Vec<usize> = (0..self.columns.len())
                        .filter(|i| !self.state.hidden.contains(i))
                        .collect();
                    let mut at = 0;
                    while at < shown.len() {
                        let band = &self.columns[shown[at]].band;
                        let mut until = at + 1;
                        while until < shown.len() && self.columns[shown[until]].band == *band {
                            until += 1;
                        }
                        if let Some(band) = band {
                            let left = places[shown[at]].0;
                            let (x, w) = places[shown[until - 1]];
                            let galley = WidgetText::from(TextRole::ColumnHead.rich(&t, band)).into_galley(
                                ui,
                                Some(TextWrapMode::Truncate),
                                (x + w - left).max(1.0),
                                TextRole::ColumnHead.font(&t),
                            );
                            let cut = galley.elided;
                            let place =
                                Rect::from_min_size(pos2(left, row.center().y - galley.size().y / 2.0), galley.size());
                            ui.painter().galley(
                                place.min,
                                galley,
                                t.color_or("table-head-ink", t.color("muted-foreground")),
                            );
                            let hit = ui.interact(place, self.id.with(("band", at)), Sense::hover());
                            hit.widget_info(|| {
                                egui::WidgetInfo::labeled(egui::Role::ColumnHeader, true, band.as_str())
                            });
                            if cut {
                                hit.on_hover_text(band.as_str());
                            }
                        }
                        at = until;
                    }
                    band_rules(ui.painter(), &t, self.columns, &self.state.hidden, &places, row);
                }
                let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), head_height), Sense::hover());
                if !stacked && self.heads {
                    let top = if banded {
                        CornerRadius::ZERO
                    } else {
                        CornerRadius { sw: 0, se: 0, ..radius }
                    };
                    ui.painter()
                        .rect_filled(head, top, t.color_or("table-head-ground", t.color("card")));
                    ui.painter().hline(
                        head.x_range(),
                        head.bottom() - 0.5,
                        Stroke::new(1.0, t.color_or("table-head-line", t.color("border"))),
                    );
                    let upper = t.word("table-head-transform") == Some("uppercase");
                    let end = if self.chevron { 32.0 + self.chevron_gap() } else { 0.0 };
                    let places = self.layout(head.left() + pad + select_width, head.right() - self.end_pad(pad) - end);
                    band_rules(ui.painter(), &t, self.columns, &self.state.hidden, &places, head);
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
                    // the heads scrolled sideways show only between the pinned columns
                    let (pin_start, pin_end) = self.pinned_edges(head, pad, &places);
                    let head_clip = ui.clip_rect();
                    let scrolled_clip = Rect::from_x_y_ranges(
                        pin_start.unwrap_or(head.left())..=pin_end.unwrap_or(head.right()),
                        head.y_range(),
                    )
                    .intersect(head_clip);
                    let first = self.pinned_first();
                    for (index, column) in self.columns.iter().enumerate() {
                        if self.state.hidden.contains(&index) {
                            continue;
                        }
                        ui.set_clip_rect(if Some(index) != first { scrolled_clip } else { head_clip });

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
                        let _ = upper;
                        let galley = self.head_galley(ui, &t, index, w);
                        if galley.elided {
                            hit.clone().on_hover_text(&column.title);
                        }
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
                        // a galley laid out at the end stands to the left of where it is put
                        let at_x = if column.numeric { cell.right() } else { text_x };
                        ui.painter()
                            .galley(pos2(at_x, cell.center().y - galley.size().y / 2.0), galley, ink);
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
                    ui.set_clip_rect(head_clip);
                    pinned_rules(ui.painter(), &t, head, (pin_start, pin_end));
                }

                // a failed read in place of the rows under the heads, unframed inside the frame
                if failing {
                    if self.failure_line(ui) {
                        out.retry = true;
                    }
                    return;
                }
                // the first rows on their way: placeholder rows filling the view
                if self.rows.is_empty() && self.loading {
                    let view = if self.stretch {
                        limit - ui.cursor().top() - foot_room - 1.0
                    } else {
                        self.height
                    };
                    let rows = ((view / row_height).floor() as usize).max(3);
                    for line in 0..rows {
                        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), row_height), Sense::hover());
                        self.placeholder_row(ui, &t, rect, (pad, select_width), stacked, line);
                    }
                    return;
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
                // rows that grow to their words or open into a detail are each as tall as they
                // measure, every one drawn; the details' heights are last frame's
                let rows_count = self.rows.len();
                let groups: Option<Vec<String>> = self.groups.clone().filter(|g| g.len() == rows_count);
                let groups = groups.as_ref();
                // an open branch whose children did not load: a line under it says so
                let refusals = self.state.refusals.clone();
                let refused_under = |index: usize| refusals.contains(&index);
                let refused_lines = order.iter().filter(|i| refused_under(**i)).count();
                let free = (!stacked && (self.auto || self.detail.is_some() || groups.is_some())) || refused_lines > 0;
                // a listed row's refusal: its words padded 8 over and under, as the web's `py-2`
                let refused_item = TextRole::Body.line_height(&t) + 16.0 + 1.0;
                let refused_height = if stacked { refused_item } else { row_height };
                let heights: Vec<f32> = if free && self.auto && !stacked {
                    let end = if self.chevron { 32.0 + self.chevron_gap() } else { 0.0 };
                    let left = ui.cursor().left();
                    let places = self.layout(
                        left + pad + select_width,
                        left + ui.available_width() - self.end_pad(pad) - end,
                    );
                    (0..self.rows.len())
                        .map(|index| {
                            let content = self
                                .columns
                                .iter()
                                .enumerate()
                                .filter(|(c, _)| !self.state.hidden.contains(c))
                                .map(|(c, _)| {
                                    let room = places[c].1 - if c == 0 { lead_of(index) } else { 0.0 };
                                    cell_height(ui, &t, &self.rows[index][c], room.max(1.0))
                                })
                                .fold(0.0, f32::max);
                            (content + 16.0 + 1.0).max(row_height)
                        })
                        .collect()
                } else {
                    vec![row_height; self.rows.len()]
                };
                let own = self.id;
                let detail_id = move |index: usize| own.with(("detail", index));
                let detail_height = |ui: &Ui, index: usize| -> f32 {
                    if self.detail.is_some() && self.state.expanded.contains(&index) {
                        ui.ctx().data(|d| d.get_temp::<f32>(detail_id(index))).unwrap_or(0.0)
                    } else {
                        0.0
                    }
                };
                let folded = |index: usize| groups.is_some_and(|g| self.state.folded.contains(&g[index]));
                let heads = groups.map_or(0, |g| {
                    let mut keys: Vec<&String> = g.iter().collect();
                    keys.sort();
                    keys.dedup();
                    keys.len()
                });
                // more rows on their way: three placeholder rows after the last
                let more = if self.loading { 3 } else { 0 };
                let whole: f32 = order
                    .iter()
                    .filter(|i| !folded(**i))
                    .map(|i| heights[*i] + detail_height(ui, *i))
                    .sum::<f32>()
                    + (heads + more) as f32 * row_height
                    + refused_lines as f32 * refused_height;
                let tall = if self.stretch {
                    // what is left under the bar and the head, less the foot and the frame's edge
                    (limit - ui.cursor().top() - foot_room - 1.0).max(row_height)
                } else if free {
                    self.height.min(whole + 1.0)
                } else {
                    // never taller than its rows: a height without bound shows them all
                    self.height.min((order.len() + more) as f32 * row_height + 1.0)
                };
                let mut area = egui::ScrollArea::vertical()
                    .id_salt(self.id.with("rows"))
                    .max_height(tall)
                    .auto_shrink([false, true]);
                if !self.stretch {
                    // as tall as asked, even inside a page that scrolls itself
                    area = area.min_scrolled_height(tall);
                }
                // a row the keys moved to scrolls into view, the least way, as the web's `moveTo`
                if std::mem::take(&mut self.state.reveal)
                    && !free
                    && taller.is_none()
                    && let Some(at) = self
                        .state
                        .active
                        .and_then(|(row, _)| order.iter().position(|r| *r == row))
                {
                    let scrolled =
                        egui::scroll_area::State::load(ui.ctx(), ui.make_persistent_id(self.id.with("rows")))
                            .map_or(0.0, |state| state.clamped_offset().y);
                    let top = at as f32 * row_height;
                    let bottom = top + row_height;
                    let view = tall - 1.0;
                    if top < scrolled {
                        area = area.vertical_scroll_offset(top);
                    } else if bottom > scrolled + view {
                        area = area.vertical_scroll_offset(bottom - view);
                    }
                }
                let count = order.len();
                let mut last_group: Option<String> = None;
                let mut body = |ui: &mut Ui, range: std::ops::Range<usize>| {
                    for position in range {
                        if position >= count {
                            let (rect, _) =
                                ui.allocate_exact_size(vec2(ui.available_width(), row_height), Sense::hover());
                            self.placeholder_row(ui, &t, rect, (pad, select_width), stacked, position);
                            continue;
                        }
                        let index = order[position];
                        if let Some(groups) = groups {
                            let key = &groups[index];
                            if last_group.as_ref() != Some(key) {
                                last_group = Some(key.clone());
                                let shut = self.state.folded.contains(key);
                                let members: Vec<usize> =
                                    order.iter().copied().filter(|i| groups[*i] == *key).collect();
                                let held = members.len();
                                // each aggregating column's words for the group, in its place
                                let room = ui.available_rect_before_wrap();
                                let end = if self.chevron { 32.0 + self.chevron_gap() } else { 0.0 };
                                let places = self
                                    .layout(room.left() + pad + select_width, room.right() - self.end_pad(pad) - end);
                                let shown: Vec<usize> = (0..self.columns.len())
                                    .filter(|i| !self.state.hidden.contains(i))
                                    .collect();
                                // the label spans the columns up to the first after the first that aggregates
                                let span_end = shown
                                    .iter()
                                    .skip(1)
                                    .find(|c| self.columns[**c].aggregate.is_some())
                                    .map(|c| places[*c].0 - GAP);
                                let aggregates: Vec<(f32, f32, bool, String)> = if stacked {
                                    Vec::new()
                                } else {
                                    shown
                                        .iter()
                                        .skip(1)
                                        .filter_map(|c| {
                                            let column = &self.columns[*c];
                                            let said = (column.aggregate?)(self.rows, &members, *c);
                                            Some((places[*c].0, places[*c].1, column.numeric, said))
                                        })
                                        .collect()
                                };
                                let head = GroupHead {
                                    key,
                                    held,
                                    shut,
                                    start: pad + select_width,
                                    label_end: span_end,
                                    aggregates: &aggregates,
                                    height: t.density("row"),
                                };
                                let rect = ui.available_rect_before_wrap();
                                let pressed = group_head(ui, &t, &head, &self.words);
                                let row = Rect::from_min_size(rect.min, vec2(rect.width(), t.density("row")));
                                if !stacked && self.columns.iter().any(|c| c.band.is_some()) {
                                    band_rules(ui.painter(), &t, self.columns, &self.state.hidden, &places, row);
                                }
                                // the label spans the columns; the end column stays pinned beside it
                                if !stacked {
                                    let (_, end) = self.pinned_edges(row, pad, &places);
                                    pinned_rules(ui.painter(), &t, row, (None, end));
                                }
                                if pressed {
                                    if shut {
                                        self.state.folded.remove(key);
                                    } else {
                                        self.state.folded.insert(key.clone());
                                    }
                                }
                            }
                            if self.state.folded.contains(key) {
                                continue;
                            }
                        }
                        let height = if free && !stacked {
                            heights[index]
                        } else {
                            row_height + grown(self.state, index)
                        };
                        let (rect, row) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
                        let expanded = self.detail.is_some() && self.state.expanded.contains(&index);
                        row_rects.push(rect);
                        // a screen reader reads and presses the row by its cells' words, as a row reads
                        row.widget_info(|| {
                            let name = self.rows[index]
                                .iter()
                                .map(Cell::draft)
                                .filter(|d| !d.is_empty())
                                .collect::<Vec<_>>()
                                .join(", ");
                            egui::WidgetInfo::labeled(egui::Role::Button, true, name)
                        });
                        // a press with Shift, Cmd or Ctrl selects, and opens nothing
                        let picked = row.clicked()
                            && !(self.selectable
                                && select_press(
                                    ui,
                                    self.state,
                                    shape.as_ref(),
                                    self.groups.as_deref().filter(|g| g.len() == self.rows.len()),
                                    &order,
                                    index,
                                ));
                        if picked && self.detail.is_some() {
                            // a press opens the row's detail or closes it
                            if !self.state.expanded.remove(&index) {
                                self.state.expanded.insert(index);
                            }
                        } else if picked {
                            // the press reported, as the web's `onOpen`
                            self.state.opened = Some(index);
                        }
                        if row.clicked() && tree_keys {
                            // a press makes the row the active one, the table taking the keys
                            self.state.active = Some((index, 0));
                            self.state.focused = true;
                            self.state.keyed = false;
                            self.state.head = None;
                        }
                        let selected = self.state.selected.contains(&index);
                        let opened = self.opened == Some(index);
                        let ground = if selected || opened {
                            t.color("secondary")
                        } else if row.hovered() || ui.rect_contains_pointer(rect) || dragging == Some(position) {
                            // the pointer anywhere over the row, its cells' controls too, as `hover:`
                            t.color("hover")
                        } else if position % 2 == 1 {
                            zebra
                        } else {
                            t.color("card")
                        };
                        ui.painter().rect_filled(rect, CornerRadius::ZERO, ground);
                        // the active row moved by keys: a 2 bar in `ring` inside its start edge
                        if tree_keys
                            && self.state.focused
                            && self.state.keyed
                            && self.state.active.map(|a| a.0) == Some(index)
                        {
                            ui.painter().rect_filled(
                                Rect::from_min_size(rect.min, vec2(2.0, rect.height() - 1.0)),
                                CornerRadius::ZERO,
                                t.color("ring"),
                            );
                        }
                        // the active cell's row, while the table has the focus or a typed editor is open
                        let typed_open = self.state.open.as_ref().is_some_and(|o| {
                            o.row == index
                                && !matches!(
                                    self.columns.get(o.column).and_then(|c| c.edit.as_ref()),
                                    Some(Editor::Select(_))
                                )
                        });
                        if self.cell_focus == CellFocus::Row
                            && (typed_open
                                || (self.state.focused
                                    && self.state.open.is_none()
                                    && self.state.active.map(|a| a.0) == Some(index)))
                        {
                            ui.painter().rect_filled(rect, CornerRadius::ZERO, t.color("hover"));
                        }
                        // a list's last row leaves its rule to the frame, as the web's `border-b-0`; a
                        // row and its open detail read as one, with no line between them
                        if !expanded && !(self.variant == TableVariant::List && position + 1 == count) {
                            ui.painter().hline(
                                rect.x_range(),
                                rect.bottom() - 0.5,
                                Stroke::new(1.0, t.color("border")),
                            );
                        }
                        if self.selectable {
                            // a branch's box takes every row under it, and is mixed while some are chosen
                            let branch = shape.as_ref().map(|s| s.branch_rows(index));
                            let some = !selected
                                && branch
                                    .as_ref()
                                    .is_some_and(|rows| rows.iter().any(|r| self.state.selected.contains(r)));
                            let mut checked = selected;
                            let place =
                                Rect::from_min_size(pos2(rect.left() + pad, rect.center().y - 8.0), vec2(16.0, 16.0));
                            if put_at(
                                ui,
                                place,
                                Checkbox::new(&mut checked, self.words.select_row.clone())
                                    .hide_label()
                                    .indeterminate(some),
                            )
                            .clicked()
                            {
                                // Shift on the box selects the range from the row pressed last
                                let range = ui.input(|i| i.modifiers.shift);
                                toggle_selection(
                                    self.state,
                                    shape.as_ref(),
                                    self.groups.as_deref().filter(|g| g.len() == self.rows.len()),
                                    &order,
                                    index,
                                    range,
                                );
                            }
                        }
                        if stacked {
                            let editing = Editing {
                                keys: &keys,
                                order: &order,
                                position,
                            };
                            let tree = (has_detail || shape.is_some()).then(|| StackedTree {
                                indent: shape.as_ref().map_or(0.0, |s| s.depth[index] as f32 * 20.0),
                                branch: has_detail || shape.as_ref().is_some_and(|s| s.branch(index)),
                                waiting: self.state.expanded.contains(&index)
                                    && shape.as_ref().is_some_and(|s| s.lazy[index])
                                    && !refused_under(index),
                            });
                            let (toggled, edited) =
                                self.stacked_row(ui, &t, rect, index, (pad, select_width, tree), &editing);
                            if toggled {
                                out.toggled = Some(index);
                            }
                            if edited.is_some() {
                                out.edited = edited;
                            }
                        }
                        let end = if self.chevron { 32.0 + self.chevron_gap() } else { 0.0 };
                        if self.chevron && !stacked {
                            // a table's chevron alone stands in the middle of its pinned end column,
                            // as the web's; beside a menu, or in a list, after the row's padding
                            let inset = if self.variant == TableVariant::Table && self.actions.is_none() {
                                0.0
                            } else {
                                self.end_pad(pad)
                            };
                            icons::paint(
                                ui.painter(),
                                pos2(rect.right() - inset - 16.0, rect.center().y),
                                Icon::ChevronRight,
                                16.0,
                                t.color("faint"),
                            );
                        }
                        let places =
                            self.layout(rect.left() + pad + select_width, rect.right() - self.end_pad(pad) - end);
                        if !stacked {
                            band_rules(ui.painter(), &t, self.columns, &self.state.hidden, &places, rect);
                        }
                        let edges = band_edges(self.columns, &self.state.hidden);
                        // the cells scrolled sideways show only between the pinned columns
                        let (pin_start, pin_end) = self.pinned_edges(rect, pad, &places);
                        let row_clip = ui.clip_rect();
                        let scrolled_clip = Rect::from_x_y_ranges(
                            pin_start.unwrap_or(rect.left())..=pin_end.unwrap_or(rect.right()),
                            rect.y_range(),
                        )
                        .intersect(row_clip);
                        let first = self.pinned_first();
                        for (c, column) in self.columns.iter().enumerate().filter(|_| !stacked) {
                            if self.state.hidden.contains(&c) {
                                continue;
                            }
                            ui.set_clip_rect(if Some(c) != first { scrolled_clip } else { row_clip });
                            let (x, w) = places[c];
                            let mut cell = Rect::from_min_size(pos2(x, rect.top()), vec2(w, rect.height()));
                            // the cell's whole box, as the web's cell with its padding: to the row's
                            // padding at its ends, half the gap between columns
                            let first_shown = (0..c).all(|i| self.state.hidden.contains(&i));
                            let last_shown = (c + 1..self.columns.len()).all(|i| self.state.hidden.contains(&i));
                            let cell_box = Rect::from_min_max(
                                pos2(
                                    if first_shown {
                                        x - pad
                                    } else if edges[c] {
                                        x - GAP
                                    } else {
                                        x - GAP / 2.0
                                    },
                                    rect.top(),
                                ),
                                pos2(
                                    if last_shown {
                                        x + w + self.end_pad(pad)
                                    } else if (c + 1..self.columns.len())
                                        .find(|i| !self.state.hidden.contains(i))
                                        .is_some_and(|next| edges[next])
                                    {
                                        x + w + GAP
                                    } else {
                                        x + w + GAP / 2.0
                                    },
                                    rect.bottom() - 1.0,
                                ),
                            );
                            if c == 0 && (has_detail || shape.is_some()) {
                                // the control that opens the row's branch or detail, 20 square and
                                // 4 before the cell, after its tree indent; a spinner while its
                                // children load, and room alone on a row with nothing to open
                                let lead = lead_of(index);
                                let control = Rect::from_min_size(
                                    pos2(x + lead - 24.0, rect.center().y - 10.0),
                                    vec2(20.0, 20.0),
                                );
                                let branch = has_detail || shape.as_ref().is_some_and(|s| s.branch(index));
                                // a branch whose children did not load shows its chevron again
                                let waiting = self.state.expanded.contains(&index)
                                    && shape.as_ref().is_some_and(|s| s.lazy[index])
                                    && !refused_under(index);
                                if self.tree_control(ui, &t, control, index, branch, waiting) && tree_keys {
                                    self.state.active = Some((index, 0));
                                    self.state.focused = true;
                                    self.state.keyed = false;
                                    self.state.head = None;
                                }
                                cell.min.x += lead;
                            }
                            if c == 0 && self.reorder {
                                // the grip: drag it to move the row
                                let grip = Rect::from_min_size(pos2(x - 4.0, rect.top()), vec2(GRIP, rect.height()));
                                let hit = ui.interact(grip, self.id.with(("grip", index)), Sense::drag());
                                hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Move"));
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
                                    if self.cell_focus == CellFocus::Soft
                                        && self.state.focused
                                        && self.state.active == Some(here)
                                    {
                                        ui.painter().rect_filled(cell_box, CornerRadius::ZERO, t.color("hover"));
                                    }
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
                                                self.auto,
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
                                    if self.state.focused
                                        && self.state.active == Some(here)
                                        && !self.state.refused.contains_key(&(keys[index], c))
                                    {
                                        mark_cell(ui.painter(), &t, cell_box, self.cell_focus);
                                    }
                                }
                            } else if let (Cell::Slot { .. }, Some(slot)) = (&self.rows[index][c], &self.slot) {
                                let mut child = ui.new_child(
                                    egui::UiBuilder::new()
                                        .max_rect(cell)
                                        .id_salt(("slot", index, c))
                                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                                );
                                child.set_clip_rect(cell.intersect(ui.clip_rect()));
                                slot(&mut child, index, c);
                            } else if let Some(toggled) = paint_cell(
                                ui,
                                &t,
                                &self.rows[index][c],
                                cell,
                                column.numeric,
                                self.id.with(("cell", index, c)),
                                self.auto,
                            ) {
                                let _ = toggled;
                                if matches!(self.rows[index][c], Cell::Button { .. }) {
                                    out.pressed = Some((index, c));
                                } else if matches!(self.rows[index][c], Cell::Switch { .. }) {
                                    out.switched = Some(index);
                                } else {
                                    out.toggled = Some(index);
                                }
                            }
                        }
                        ui.set_clip_rect(row_clip);
                        if !stacked {
                            pinned_rules(ui.painter(), &t, rect, (pin_start, pin_end));
                        }
                        if let Some(actions) = &self.actions {
                            let place = Rect::from_min_size(
                                pos2(rect.right() - self.end_pad(pad) - ACTIONS, rect.top()),
                                vec2(ACTIONS, rect.height()),
                            );
                            // stacked, the menu stands at the row's end on the title's line, a 24 box
                            let middle = if stacked {
                                pos2(rect.right() - pad - 12.0, rect.top() + 8.0 + 12.0)
                            } else if self.chevron {
                                // before the chevron, 22 from its middle, as the web's end column
                                pos2(rect.right() - self.end_pad(pad) - 16.0 - 22.0, rect.center().y)
                            } else {
                                place.center()
                            };
                            let button = Rect::from_center_size(middle, vec2(28.0, 28.0));
                            let trigger = ui.interact(button, self.id.with(("actions", index)), Sense::click());
                            trigger.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Actions"));
                            if trigger.hovered() {
                                ui.painter().rect_filled(button, t.radius("control"), t.color("hover"));
                            }
                            // a menu shown on hover stands out of sight until its row is pointed
                            // at, has the focus, or is open
                            let menu_open =
                                egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&trigger));
                            let seen = !self.actions_hover
                                || ui.rect_contains_pointer(rect)
                                || trigger.has_focus()
                                || menu_open;
                            if seen {
                                icons::paint(
                                    ui.painter(),
                                    button.center(),
                                    Icon::Ellipsis,
                                    16.0,
                                    t.color("muted-foreground"),
                                );
                            }
                            let items = actions(index);
                            if let Some(chosen) = DropdownMenu::new(&items).show(&trigger) {
                                out.action = Some((index, chosen));
                            }
                        }
                        if let Some(detail) = self.detail.as_ref().filter(|_| expanded) {
                            // the open detail, from where the row's first cell starts, 12 over and
                            // under it and 12 from the end, on the row's own ground
                            let ground = if position % 2 == 1 { zebra } else { t.color("card") };
                            let behind = ui.painter().add(egui::Shape::Noop);
                            let margin = egui::Margin {
                                left: (select_width + lead_of(index) + pad) as i8,
                                right: 12,
                                top: 12,
                                bottom: 12,
                            };
                            let drawn = egui::Frame::new()
                                .inner_margin(margin)
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    detail(index, ui);
                                })
                                .response
                                .rect;
                            ui.painter()
                                .set(behind, egui::Shape::rect_filled(drawn, CornerRadius::ZERO, ground));
                            if !(self.variant == TableVariant::List && position + 1 == count) {
                                ui.painter().hline(
                                    drawn.x_range(),
                                    drawn.bottom() - 0.5,
                                    Stroke::new(1.0, t.color("border")),
                                );
                            }
                            ui.ctx().data_mut(|d| d.insert_temp(detail_id(index), drawn.height()));
                        }
                        if refused_under(index) {
                            // the line under a branch whose children did not load, from where its
                            // children would start: a row's height, or a listed row's, padded 8
                            let height = if stacked { refused_item } else { row_height };
                            let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
                            ui.painter().rect_filled(line, CornerRadius::ZERO, t.color("card"));
                            ui.painter().hline(
                                line.x_range(),
                                line.bottom() - 0.5,
                                Stroke::new(1.0, t.color("border")),
                            );
                            let left = if stacked {
                                line.left() + lead_of(index) + 20.0
                            } else {
                                line.left() + pad + select_width + lead_of(index) + 20.0
                            };
                            self.refused_line(ui, &t, line, (left, pad), index);
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
                    None if free => {
                        area.show(ui, |ui| body(ui, 0..count + more));
                    }
                    None => {
                        area.show_rows(ui, row_height, count + more, body);
                    }
                }
                // columns running past the room: a thin bar along the rows' foot, dragged or
                // scrolled, showing how far they are scrolled, while the pointer is over them
                if !stacked {
                    let body = Rect::from_x_y_ranges(ui.max_rect().x_range(), ui.min_rect().y_range());
                    let end = if self.chevron { 32.0 + self.chevron_gap() } else { 0.0 };
                    let over = self.overflow(body.left() + pad + select_width, body.right() - self.end_pad(pad) - end);
                    let bar_id = self.id.with("sideways");
                    let held = ui.ctx().is_being_dragged(bar_id);
                    if over > 0.0 && (ui.rect_contains_pointer(body) || held) {
                        let places =
                            self.layout(body.left() + pad + select_width, body.right() - self.end_pad(pad) - end);
                        let (start, finish) = self.pinned_edges(body, pad, &places);
                        let track = Rect::from_x_y_ranges(
                            start.unwrap_or(body.left()) + 2.0..=finish.unwrap_or(body.right()) - 2.0,
                            body.bottom() - t.length("scrollbar-width") + 2.0..=body.bottom() - 2.0,
                        );
                        let seen = track.width();
                        let handle_w = (seen * seen / (seen + over)).max(24.0).min(seen);
                        let at = track.left() + (seen - handle_w) * (self.state.scroll_x / over);
                        let handle = Rect::from_min_size(pos2(at, track.top()), vec2(handle_w, track.height()));
                        let grip = ui.interact(handle.expand(4.0), bar_id, Sense::drag());
                        if grip.dragged() && seen > handle_w {
                            let dx = grip.drag_delta().x * over / (seen - handle_w);
                            self.state.scroll_x = (self.state.scroll_x + dx).clamp(0.0, over);
                        }
                        let ink = if grip.hovered() || grip.dragged() {
                            t.color("muted-foreground")
                        } else {
                            t.color("border")
                        };
                        ui.painter().rect_filled(handle, t.radius("thumb"), ink);
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
                // the pager: which rows of how many, the rows to a page, and the page's buttons
                if let Some(sizes) = self.paging.clone() {
                    let size = self.state.page_size.max(1);
                    let pages = total.div_ceil(size).max(1);
                    let page = self.state.page;
                    let mono = crate::mono_font(
                        ui.ctx(),
                        crate::mono_small_size(&t, TextRole::Help.font(&t).size),
                        400.0,
                    );
                    let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::hover());
                    ui.painter()
                        .hline(line.x_range(), line.top() + 0.5, Stroke::new(1.0, t.color("border")));
                    let first = if total == 0 { 0 } else { page * size + 1 };
                    let last = ((page + 1) * size).min(total);
                    let mut left = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(line.shrink2(vec2(12.0, 6.0)))
                            .layout(egui::Layout::left_to_right(egui::Align::Center)),
                    );
                    left.spacing_mut().item_spacing.x = 16.0;
                    left.label(
                        egui::RichText::new(format!("{first}–{last} of {total}"))
                            .font(mono)
                            .color(t.color("muted-foreground")),
                    );
                    left.spacing_mut().item_spacing.x = 8.0;
                    left.label(
                        TextRole::Help
                            .rich(&t, "Rows per page")
                            .color(t.color("muted-foreground")),
                    );
                    let mut at = sizes.iter().position(|s| *s == size).unwrap_or(0);
                    let names: Vec<String> = sizes.iter().map(ToString::to_string).collect();
                    if left
                        .add(
                            super::Select::new(&mut at, names, "Rows per page")
                                .small(true)
                                .width(64.0),
                        )
                        .changed()
                    {
                        self.state.page_size = sizes[at];
                        self.state.page = 0;
                    }
                    let mut right = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(line.shrink2(vec2(12.0, 6.0)))
                            .layout(egui::Layout::right_to_left(egui::Align::Center)),
                    );
                    right.spacing_mut().item_spacing.x = 4.0;
                    let mut go = None;
                    for (icon, label, to, open) in [
                        (Icon::ChevronsRight, "Last page", pages - 1, page + 1 < pages),
                        (Icon::ChevronRight, "Next page", page + 1, page + 1 < pages),
                        (Icon::ChevronLeft, "Previous page", page.saturating_sub(1), page > 0),
                        (Icon::ChevronsLeft, "First page", 0, page > 0),
                    ] {
                        let pressed = right
                            .add_enabled_ui(open, |ui| ui.add(super::IconButton::new(icon, label)).clicked())
                            .inner;
                        if pressed {
                            go = Some(to);
                        }
                    }
                    right.add_space(8.0);
                    right.label(TextRole::Help.rich(&t, format!("Page {} of {pages}", page + 1)));
                    if let Some(to) = go {
                        self.state.page = to;
                    }
                }
                // the foot: a rule over it, its words in the code face, 12 in and 8 down
                if let Some(foot) = &self.foot {
                    let mono = crate::mono_font(
                        ui.ctx(),
                        crate::mono_small_size(&t, TextRole::Help.font(&t).size),
                        400.0,
                    );
                    let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
                    ui.painter()
                        .hline(line.x_range(), line.top() + 0.5, Stroke::new(1.0, t.color("border")));
                    ui.painter().text(
                        pos2(line.left() + 12.0, line.center().y),
                        egui::Align2::LEFT_CENTER,
                        foot,
                        mono,
                        t.color("muted-foreground"),
                    );
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
            // Space with Shift too: consuming a key leaves Shift aside
            let (space, space_shift) = ui.input_mut(|i| {
                let space = tree_keys && i.consume_key(egui::Modifiers::NONE, egui::Key::Space);
                (space, i.modifiers.shift)
            });
            let (home, end, page_up, page_down) = ui.input_mut(|i| {
                let none = egui::Modifiers::NONE;
                (
                    i.consume_key(none, egui::Key::Home),
                    i.consume_key(none, egui::Key::End),
                    i.consume_key(none, egui::Key::PageUp),
                    i.consume_key(none, egui::Key::PageDown),
                )
            });
            if tree_keys && (enter || left || right || up || down || space || home || end || page_up || page_down) {
                self.state.keyed = true;
            }
            // a view's worth of rows, less one, as the web's PageUp and PageDown
            let view = (((self.height / row_height).floor() as usize).saturating_sub(1)).max(1);
            let at = |p: usize| order.get(p.min(order.len().saturating_sub(1))).copied();
            // in a tree the arrows open and close the branches: everywhere in a table that does not
            // edit, and in the first column of one that does, as the web's editing gives a
            // branch's arrows back there
            let first_shown = (0..self.columns.len()).find(|c| !self.state.hidden.contains(c));
            let tree_here = shape.is_some() && (tree_keys || Some(column) == first_shown);
            let branch_here = has_detail || shape.as_ref().is_some_and(|s| s.branch(row));
            let open_here = self.state.expanded.contains(&row);
            let last = self.columns.len().saturating_sub(1);
            // a character typed opens a text cell on it; Backspace or Delete opens a text or a date
            // cell empty
            let editor = self.columns.get(column).and_then(|c| c.edit.clone());
            let typed = match editor {
                Some(Editor::Text | Editor::Number) => ui.input_mut(|i| {
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
            } else if right && tree_here && branch_here && !open_here {
                // Right opens the active branch, as the web's tree
                self.state.expanded.insert(row);
            } else if right && tree_keys {
                // an open branch or a leaf: Right does nothing more
            } else if let (true, true, Some(shape)) = (left, tree_here, &shape) {
                // Left closes the active branch, or moves to its parent
                if branch_here && open_here {
                    self.state.expanded.remove(&row);
                } else if let Some(parent) = shape.parent[row] {
                    self.state.active = Some((parent, column));
                    self.state.reveal = true;
                }
            } else if home || end || page_up || page_down {
                // Home and End to the first and the last row, PageUp and PageDown a view's worth
                let to = if home {
                    at(0)
                } else if end {
                    at(usize::MAX)
                } else if page_up {
                    at(position.saturating_sub(view))
                } else {
                    at(position + view)
                };
                if let Some(to) = to {
                    self.state.active = Some((to, column));
                    self.state.reveal = true;
                }
            } else if tree_keys && (enter || space) {
                // Space selects the active row, or picks it in a table that does not select; Enter
                // picks it: its detail opens or closes, or it is opened beside the list
                if space && self.selectable {
                    // a branch takes every row under it, as its box does; Shift takes the range
                    // from the row pressed last
                    toggle_selection(
                        self.state,
                        shape.as_ref(),
                        self.groups.as_deref().filter(|g| g.len() == self.rows.len()),
                        &order,
                        row,
                        space_shift,
                    );
                } else if has_detail {
                    if !self.state.expanded.remove(&row) {
                        self.state.expanded.insert(row);
                    }
                } else {
                    self.state.opened = Some(row);
                }
            } else if left {
                self.state.active = Some((row, column.saturating_sub(1)));
            } else if right {
                self.state.active = Some((row, (column + 1).min(last)));
            } else if up && position == 0 && sorts {
                // up from the first row, onto the head, where the keys sort
                self.state.head = Some(column);
            } else if up {
                self.state.active = Some((order[position.saturating_sub(1)], column));
                self.state.reveal = true;
            } else if down {
                self.state.active = Some((order[(position + 1).min(order.len().saturating_sub(1))], column));
                self.state.reveal = true;
            }
        }
        // a press outside the table takes its focus away
        if ui.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !response.rect.contains(p)))
            && self.state.open.is_none()
        {
            self.state.focused = false;
        }
        if self.footer && !uncounted {
            let count = self.rows.len();
            let words = if count == 1 { &self.words.row } else { &self.words.rows };
            ui.add_space(6.0);
            ui.label(
                TextRole::Help
                    .rich(
                        &t,
                        words.replace("{count}", &super::format_value(count as f64, Some(0))),
                    )
                    .color(t.color("muted-foreground")),
            );
        }
        out.response = response;
        self.state.active_key = self.state.active.and_then(|(row, _)| keys.get(row).copied());
        out
    }
}

/// Draw one cell's content in `rect`; true when its check box was pressed.
/// Draw `cell` in `rect`, its words on one line cut short, or when `wrap` broken onto as many lines
/// as they take (a table whose rows grow to hold them).
fn paint_cell(
    ui: &mut Ui,
    t: &crate::Tokens,
    cell: &Cell,
    rect: Rect,
    numeric: bool,
    id: Id,
    wrap: bool,
) -> Option<bool> {
    let room = rect.width().max(1.0);
    let mode = if wrap {
        TextWrapMode::Wrap
    } else {
        TextWrapMode::Truncate
    };
    let text = |ui: &mut Ui, rich: egui::RichText, ink: egui::Color32| {
        let galley = WidgetText::from(rich).into_galley(ui, Some(mode), room, TextRole::Body.font(t));
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
            TextRole::Body
                .rich(t, label)
                .variation("wght", t.weight(crate::Weight::Medium)),
            t.color("foreground"),
        ),
        Cell::Struck(label) => text(
            ui,
            TextRole::Body.rich(t, label).strikethrough(),
            t.color("muted-foreground"),
        ),
        Cell::Muted(label) => text(ui, TextRole::Body.rich(t, label), t.color("muted-foreground")),
        Cell::Runs(runs) => {
            let job = runs_job(ui, t, runs, TextRole::Body);
            let galley = WidgetText::from(job).into_galley(ui, Some(mode), room, TextRole::Body.font(t));
            let x = if numeric {
                rect.right() - galley.size().x
            } else {
                rect.left()
            };
            ui.painter().galley(
                pos2(x, rect.center().y - galley.size().y / 2.0),
                galley,
                t.color("foreground"),
            );
        }
        Cell::Dot(color, _) => {
            ui.painter()
                .circle_filled(pos2(rect.left() + 4.0, rect.center().y), 4.0, t.color(color));
        }
        Cell::Toned(label, ink) => text(ui, TextRole::Body.rich(t, label), t.color(ink)),
        Cell::CodeLines { code, detail } => {
            let lay = |ui: &mut Ui, rich: egui::RichText, role: TextRole| {
                WidgetText::from(rich).into_galley(ui, Some(mode), room, role.font(t))
            };
            let name = lay(ui, code_rich(ui, t, code, TextRole::Body), TextRole::Body);
            let small = lay(ui, TextRole::Help.rich(t, detail), TextRole::Help);
            let y = rect.center().y - (name.size().y + small.size().y) / 2.0;
            let height = name.size().y;
            ui.painter().galley(pos2(rect.left(), y), name, t.color("foreground"));
            ui.painter()
                .galley(pos2(rect.left(), y + height), small, t.color("muted-foreground"));
        }
        Cell::Code(label) => text(ui, code_rich(ui, t, label, TextRole::Body), t.color("foreground")),
        Cell::TagName(name) => {
            icons::paint(
                ui.painter(),
                pos2(rect.left() + 8.0, rect.center().y),
                Icon::Hash,
                16.0,
                t.color("muted-foreground"),
            );
            let galley = WidgetText::from(
                TextRole::Body
                    .rich(t, name)
                    .variation("wght", t.weight(crate::Weight::Medium)),
            )
            .into_galley(ui, Some(mode), (room - 20.0).max(1.0), TextRole::Body.font(t));
            ui.painter().galley(
                pos2(rect.left() + 20.0, rect.center().y - galley.size().y / 2.0),
                galley,
                t.color("foreground"),
            );
        }
        Cell::IconName { icon, name } => {
            icons::paint(
                ui.painter(),
                pos2(rect.left() + 8.0, rect.center().y),
                *icon,
                16.0,
                t.color("muted-foreground"),
            );
            let galley = WidgetText::from(
                TextRole::Body
                    .rich(t, name)
                    .variation("wght", t.weight(crate::Weight::Medium)),
            )
            .into_galley(ui, Some(mode), (room - 22.0).max(1.0), TextRole::Body.font(t));
            ui.painter().galley(
                pos2(rect.left() + 22.0, rect.center().y - galley.size().y / 2.0),
                galley,
                t.color("foreground"),
            );
        }
        Cell::NameThenCode { name, code } => {
            let mut job = egui::text::LayoutJob::default();
            TextRole::Body.rich(t, name).color(t.color("foreground")).append_to(
                &mut job,
                ui.style(),
                egui::FontSelection::Default,
                egui::Align::BOTTOM,
            );
            egui::RichText::new(format!("  {code}"))
                .font(crate::mono_font(
                    ui.ctx(),
                    crate::mono_small_size(t, TextRole::Help.font(t).size),
                    400.0,
                ))
                .color(t.color("muted-foreground"))
                .append_to(&mut job, ui.style(), egui::FontSelection::Default, egui::Align::BOTTOM);
            job.wrap = egui::text::TextWrapping::truncate_at_width(room);
            let galley = ui.fonts_mut(|f| f.layout_job(job));
            ui.painter().galley(
                pos2(rect.left(), rect.center().y - galley.size().y / 2.0),
                galley,
                t.color("foreground"),
            );
        }
        Cell::ChipNote { label, tone, note } => {
            let width = super::chip::paint_at(ui, rect.left(), rect.center().y, label, *tone);
            let galley = WidgetText::from(TextRole::Body.rich(t, note)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                (room - width - 8.0).max(1.0),
                TextRole::Body.font(t),
            );
            ui.painter().galley(
                pos2(rect.left() + width + 8.0, rect.center().y - galley.size().y / 2.0),
                galley,
                t.color("muted-foreground"),
            );
        }
        Cell::NamedCode { name, code, strong } => {
            let lay = |ui: &mut Ui, rich: egui::RichText, role: TextRole| {
                WidgetText::from(rich).into_galley(ui, Some(mode), room, role.font(t))
            };
            let weight = if *strong {
                t.weight(crate::Weight::Medium)
            } else {
                400.0
            };
            let first = lay(
                ui,
                TextRole::Body.rich(t, name).variation("wght", weight),
                TextRole::Body,
            );
            let second = lay(ui, code_rich(ui, t, code, TextRole::Help), TextRole::Help);
            let y = rect.center().y - (first.size().y + second.size().y) / 2.0;
            let height = first.size().y;
            ui.painter().galley(pos2(rect.left(), y), first, t.color("foreground"));
            ui.painter()
                .galley(pos2(rect.left(), y + height), second, t.color("muted-foreground"));
        }
        Cell::Lines { title, detail, over } => {
            let lay = |ui: &mut Ui, rich: egui::RichText, role: TextRole| {
                WidgetText::from(rich).into_galley(ui, Some(mode), room.max(1.0), role.font(t))
            };
            let name = lay(
                ui,
                TextRole::Body
                    .rich(t, title)
                    .variation("wght", t.weight(crate::Weight::Medium)),
                TextRole::Body,
            );
            let small = lay(ui, TextRole::Help.rich(t, detail), TextRole::Help);
            let y = rect.center().y - (name.size().y + small.size().y) / 2.0;
            let (first, second, first_ink, second_ink) = if *over {
                (small, name, t.color("muted-foreground"), t.color("foreground"))
            } else {
                (name, small, t.color("foreground"), t.color("muted-foreground"))
            };
            let height = first.size().y;
            ui.painter().galley(pos2(rect.left(), y), first, first_ink);
            ui.painter().galley(pos2(rect.left(), y + height), second, second_ink);
        }
        Cell::Mono(label) => text(ui, code_rich(ui, t, label, TextRole::Body), t.color("foreground")),
        Cell::Badge(label) => {
            paint_badge(ui, t, label, pos2(rect.left(), rect.center().y));
        }
        Cell::MutedCode(label) => text(ui, code_rich(ui, t, label, TextRole::Body), t.color("muted-foreground")),
        Cell::Marked { text: label, dot } => {
            // a 10 dot, 8 before the code, as the web's `Dot` and `gap-2`
            let lead = if let Some(color) = dot {
                ui.painter()
                    .circle_filled(pos2(rect.left() + 5.0, rect.center().y), 5.0, *color);
                18.0
            } else {
                0.0
            };
            // in the cell's text, as the web's `font-mono` in a `text-sm` cell
            let rich = egui::RichText::new(label).font(crate::code_font(ui.ctx(), t, TextRole::Body));
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
        Cell::Button { icon, label } => {
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect)
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            if child.add(super::IconButton::new(*icon, label.clone()).xs()).clicked() {
                return Some(true);
            }
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
            let mono = egui::RichText::new(clock).font(crate::mono_font(
                ui.ctx(),
                crate::mono_small_size(t, TextRole::Help.font(t).size),
                400.0,
            ));
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
            paint_tags(ui, tags, pos2(rect.left(), rect.center().y), rect.width(), false);
        }
        Cell::OutlineTags(tags) => {
            paint_tags(ui, tags, pos2(rect.left(), rect.center().y), rect.width(), true);
        }
        Cell::Relative { .. } => text(ui, TextRole::Body.rich(t, cell.draft()), t.color("muted-foreground")),
        Cell::Trend(values, ink) => {
            let place = Rect::from_min_size(
                pos2(rect.left(), rect.center().y - 14.0),
                vec2(112.0_f32.min(room), 28.0),
            );
            super::sparkline(ui.painter(), place, values, *ink);
        }
        Cell::Slot { .. } | Cell::Empty => {}
    }
    None
}

/// A labeled line's value in `place`: text and a person at the help's size in the foreground,
/// tags and chips as they are.
fn line_value(ui: &Ui, t: &crate::Tokens, cell: &Cell, place: Rect) {
    let middle = place.center().y;
    match cell {
        Cell::Tags(tags) | Cell::OutlineTags(tags) => {
            let outline = matches!(cell, Cell::OutlineTags(_));
            paint_tags(ui, tags, pos2(place.left(), middle), place.width(), outline);
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

/// Tags as neutral chips in the code face (outline chips when `outline`), 4 apart, from `at` on
/// the line's middle, as many as fit in `room`: the width they took.
fn paint_tags(ui: &Ui, tags: &[String], at: egui::Pos2, room: f32, outline: bool) -> f32 {
    let tone = if outline { ChipTone::Outline } else { ChipTone::Neutral };
    let mut x = at.x;
    for tag in tags {
        let width = super::chip::paint_tag_at(ui, x, at.y, tag, tone);
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
        Cell::Tags(tags) => paint_tags(ui, tags, at, room, false),
        Cell::OutlineTags(tags) => paint_tags(ui, tags, at, room, true),
        Cell::Badge(label) => paint_badge(ui, t, label, at),
        Cell::NamedCode { name: label, .. }
        | Cell::NameThenCode { name: label, .. }
        | Cell::IconName { name: label, .. }
        | Cell::TagName(label) => text(ui, TextRole::Help.rich(t, label), quiet, at.x, room),
        Cell::ChipNote { label, tone, .. } => super::chip::paint_at(ui, at.x, at.y, label, *tone),
        Cell::CodeLines { code: label, .. } | Cell::Code(label) => {
            let rich = egui::RichText::new(label).font(crate::mono_font(
                ui.ctx(),
                crate::mono_small_size(t, TextRole::Help.font(t).size),
                400.0,
            ));
            text(ui, rich, quiet, at.x, room)
        }
        Cell::Relative { .. } => text(ui, TextRole::Help.rich(t, cell.draft()), quiet, at.x, room),
        Cell::Struck(label) => text(ui, TextRole::Help.rich(t, label).strikethrough(), quiet, at.x, room),
        Cell::Muted(label) => text(ui, TextRole::Help.rich(t, label), quiet, at.x, room),
        Cell::Runs(runs) => text(ui, TextRole::Help.rich(t, runs_text(runs)), quiet, at.x, room),
        Cell::Dot(color, _) => {
            ui.painter().circle_filled(pos2(at.x + 4.0, at.y), 4.0, t.color(color));
            8.0
        }
        Cell::Lines { title, .. } => text(ui, TextRole::Help.rich(t, title), quiet, at.x, room),
        Cell::Toned(label, ink) => text(ui, TextRole::Help.rich(t, label), t.color(ink), at.x, room),
        Cell::Mono(label) | Cell::MutedCode(label) | Cell::Marked { text: label, .. } => {
            let rich = egui::RichText::new(label).font(crate::mono_font(
                ui.ctx(),
                crate::mono_small_size(t, TextRole::Help.font(t).size),
                400.0,
            ));
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
        Cell::Check(_)
        | Cell::Switch { .. }
        | Cell::Button { .. }
        | Cell::Trend(..)
        | Cell::Slot { .. }
        | Cell::Empty => 0.0,
    }
}

/// A group's head as a line of the table, on the panel's ground: the control that folds it, the
/// key in the column heads' words and how many rows it holds, small in the code face; whether it
/// was pressed.
/// A group's head as a table line draws it: its key, how many rows it holds, whether it is shut,
/// where its control starts, where its label must end (before the first column that aggregates),
/// and each aggregating column's start, width, alignment and words.
struct GroupHead<'h> {
    key: &'h str,
    held: usize,
    shut: bool,
    start: f32,
    label_end: Option<f32>,
    aggregates: &'h [(f32, f32, bool, String)],
    /// The head's height: a row's in a table, the head's words 6 over and under in a stacked list.
    height: f32,
}

fn group_head(ui: &mut Ui, t: &crate::Tokens, head: &GroupHead<'_>, words: &TableWords) -> bool {
    let GroupHead {
        key,
        held,
        shut,
        start,
        label_end,
        aggregates,
        height,
    } = *head;
    let (rect, row) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    let said = if shut { &words.show_group } else { &words.hide_group };
    row.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, format!("{said}: {key}")));
    if row.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, t.color("panel-ground"));
    ui.painter()
        .hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, t.color("border")));
    let control = Rect::from_min_size(pos2(rect.left() + start, rect.center().y - 10.0), vec2(20.0, 20.0));
    if row.hovered() && ui.rect_contains_pointer(control) {
        ui.painter().rect_filled(control, t.radius("sm"), t.color("accent"));
    }
    icons::paint(
        ui.painter(),
        control.center(),
        if shut { Icon::ChevronRight } else { Icon::ChevronDown },
        14.0,
        t.color("muted-foreground"),
    );
    let x = control.right() + 4.0;
    let label_room = label_end.unwrap_or(rect.right()) - x - 48.0;
    let label = WidgetText::from(TextRole::ColumnHead.rich(t, key)).into_galley(
        ui,
        Some(TextWrapMode::Truncate),
        label_room.max(1.0),
        TextRole::ColumnHead.font(t),
    );
    // each aggregating column's words for the group, at its place, cut to one line
    for (left, width, end, said) in aggregates {
        let galley = WidgetText::from(TextRole::ColumnHead.rich(t, said)).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            width.max(1.0),
            TextRole::ColumnHead.font(t),
        );
        let at = if *end { left + width - galley.size().x } else { *left };
        ui.painter().galley(
            pos2(at, rect.center().y - galley.size().y / 2.0),
            galley,
            t.color_or("table-head-ink", t.color("foreground")),
        );
    }
    let width = label.size().x;
    ui.painter().galley(
        pos2(x, rect.center().y - label.size().y / 2.0),
        label,
        t.color_or("table-head-ink", t.color("foreground")),
    );
    let count = WidgetText::from(egui::RichText::new(thousands(held)).font(crate::mono_font(
        ui.ctx(),
        crate::mono_small_size(t, TextRole::Help.font(t).size),
        400.0,
    )))
    .into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, TextRole::Help.font(t));
    ui.painter().galley(
        pos2(x + width + 8.0, rect.center().y - count.size().y / 2.0),
        count,
        t.color("muted-foreground"),
    );
    row.clicked()
}

/// A count with its thousands grouped by commas, as `toLocaleString('en-US')` writes it.
pub(crate) fn thousands(count: usize) -> String {
    let digits = count.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// A code in the mono face at `role`'s size, on lines as tall as a cell's words.
/// A quiet badge in the code face from `at` (on its middle), 20 tall and 6 across: the width it took.
fn paint_badge(ui: &Ui, t: &crate::Tokens, label: &str, at: egui::Pos2) -> f32 {
    let role = TextRole::Badge;
    let font = crate::mono_font(
        ui.ctx(),
        crate::mono_size(t, role.font(t).size),
        t.text(role.key()).weight,
    );
    let ink = t.color("secondary-foreground");
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, ink);
    let rect = Rect::from_min_size(pos2(at.x, at.y - 10.0), vec2(galley.size().x + 12.0, 20.0));
    ui.painter().rect_filled(rect, t.radius("chip"), t.color("secondary"));
    let width = rect.width();
    ui.painter().galley(
        pos2(rect.left() + 6.0, rect.center().y - galley.size().y / 2.0),
        galley,
        ink,
    );
    width
}

fn code_rich(ui: &Ui, t: &crate::Tokens, text: &str, role: TextRole) -> egui::RichText {
    egui::RichText::new(text)
        .font(crate::code_font(ui.ctx(), t, role))
        .line_height(Some(TextRole::Body.line_height(t)))
}

/// How wide a lead stands before a stacked row's title: a mark (a dot, a check, an icon) 16, and
/// words, such as a version's `v3`, as wide as they are, at most their column's width.
fn lead_width(ui: &Ui, t: &crate::Tokens, cell: &Cell, most: f32) -> f32 {
    let rich = match cell {
        Cell::Text(label) | Cell::Strong(label) | Cell::Muted(label) => TextRole::Body.rich(t, label),
        Cell::Mono(label) | Cell::MutedCode(label) => code_rich(ui, t, label, TextRole::Body),
        Cell::Code(label) => code_rich(ui, t, label, TextRole::Body),
        _ => return 16.0,
    };
    let width = WidgetText::from(rich)
        .into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, TextRole::Body.font(t))
        .size()
        .x;
    width.ceil().min(most.max(16.0))
}

/// How tall `cell` stands with its words wrapped onto lines `room` wide, for a row that grows to
/// hold them; a cell of a fixed height says 20.
fn cell_height(ui: &Ui, t: &crate::Tokens, cell: &Cell, room: f32) -> f32 {
    let lay = |rich: egui::RichText, role: TextRole| {
        WidgetText::from(rich)
            .into_galley(ui, Some(TextWrapMode::Wrap), room, role.font(t))
            .size()
            .y
    };
    match cell {
        Cell::Text(label)
        | Cell::Strong(label)
        | Cell::Struck(label)
        | Cell::Muted(label)
        | Cell::Toned(label, _)
        | Cell::Number { text: label, .. } => lay(TextRole::Body.rich(t, label), TextRole::Body),
        Cell::Mono(label) | Cell::MutedCode(label) => lay(code_rich(ui, t, label, TextRole::Body), TextRole::Body),
        Cell::Code(label) => lay(code_rich(ui, t, label, TextRole::Body), TextRole::Body),
        Cell::Runs(runs) => {
            WidgetText::from(runs_job(ui, t, runs, TextRole::Body))
                .into_galley(ui, Some(TextWrapMode::Wrap), room, TextRole::Body.font(t))
                .size()
                .y
        }
        Cell::Lines { title, detail, .. } => {
            lay(TextRole::Body.rich(t, title), TextRole::Body) + lay(TextRole::Help.rich(t, detail), TextRole::Help)
        }
        Cell::NamedCode { name, code, .. } => {
            lay(TextRole::Body.rich(t, name), TextRole::Body)
                + lay(code_rich(ui, t, code, TextRole::Help), TextRole::Help)
        }
        Cell::CodeLines { code, detail } => {
            lay(code_rich(ui, t, code, TextRole::Body), TextRole::Body)
                + lay(TextRole::Help.rich(t, detail), TextRole::Help)
        }
        _ => 20.0,
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
/// Select or clear `row`, as the web's `toggle`: with `range` and a row pressed before, every row
/// shown from that one to this, all on unless this one is on and that one is not; else the row
/// alone, a branch with every row under it. The row becomes the next range's start.
///
/// The range runs over the web's `flat`: the rows in `order` (with `groups`, each group's rows
/// after the one before), less those of a folded group, which are not drawn.
fn toggle_selection(
    state: &mut TableState,
    shape: Option<&TreeShape>,
    groups: Option<&[String]>,
    order: &[usize],
    row: usize,
    range: bool,
) {
    if range && let Some(anchor) = state.anchor {
        let on = !state.selected.contains(&row) || state.selected.contains(&anchor);
        let flat: Vec<usize> = order
            .iter()
            .copied()
            .filter(|r| groups.is_none_or(|g| !state.folded.contains(&g[*r])))
            .collect();
        let rows = match (
            flat.iter().position(|r| *r == anchor),
            flat.iter().position(|r| *r == row),
        ) {
            (Some(a), Some(b)) => flat[a.min(b)..=a.max(b)].to_vec(),
            _ => vec![row],
        };
        for r in rows {
            if on {
                state.selected.insert(r);
            } else {
                state.selected.remove(&r);
            }
        }
    } else {
        let on = !state.selected.contains(&row);
        for r in shape.map_or_else(|| vec![row], |s| s.branch_rows(row)) {
            if on {
                state.selected.insert(r);
            } else {
                state.selected.remove(&r);
            }
        }
    }
    state.anchor = Some(row);
}

/// A press on a row of a table that selects: with Shift it selects the range from the row
/// pressed last, with Cmd or Ctrl it selects or clears the row, as the web's `press`. True when it
/// did, so the press opens nothing.
fn select_press(
    ui: &Ui,
    state: &mut TableState,
    shape: Option<&TreeShape>,
    groups: Option<&[String]>,
    order: &[usize],
    row: usize,
) -> bool {
    let modifiers = ui.input(|i| i.modifiers);
    if modifiers.shift {
        toggle_selection(state, shape, groups, order, row, true);
        true
    } else if modifiers.command || modifiers.ctrl || modifiers.mac_cmd {
        toggle_selection(state, shape, groups, order, row, false);
        true
    } else {
        false
    }
}

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
    } else if matches!(column.edit, Some(Editor::Number)) && table_edit::read_number(&open.draft).is_err() {
        Some(words.not_number.clone())
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
        } else if matches!(column.edit, Some(Editor::Number)) {
            table_edit::read_number(&open.draft).ok().flatten().unwrap_or_default()
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
