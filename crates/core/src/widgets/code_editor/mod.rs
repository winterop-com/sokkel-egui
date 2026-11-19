//! The code editor, after sokkel-design's `code-editor` part and sokkel-ui's CodeMirror-based
//! `CodeEditor`: a document in a language, colored by its tokens in the `code-*` colors, with line
//! numbers, the caret's line washed, bracket matching, undo and Tab indenting; marks washing a line
//! or underlining a range, a dot in the lint gutter and the message on hover; a JSON Schema
//! checking JSON or YAML as it is typed and completing its keys and values; the app's words offered
//! while typing; a find and replace bar, folding, brackets colored by depth and indent guides.
//!
//! egui keeps its own text editing (`TextEdit`: the caret, selection, undo and redo); the editor
//! lays the text out in its colors and draws everything else around and over it.

mod document;
mod edit;
mod highlight;
mod popup;
pub mod regex;
pub(super) mod schema;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use egui::text_edit::TextEditState;
use egui::{
    Color32, CursorIcon, Event, FontId, Frame, Id, Key, Margin, Pos2, Rect, Response, ScrollArea, Sense, Shape, Stroke,
    StrokeKind, TextEdit, Ui, UiBuilder, Vec2, WidgetInfo, pos2, vec2,
};

use crate::{Icon, Tokens, icons, tokens};

pub use highlight::{Kind as CodeTag, Token as CodeToken};

use document::ReadDocument;
use edit::{Fold, Offsets, WikiLink};
use highlight::{Kind, Token};
use popup::{Offer, SearchBar, Tip};

/// The language a document is read and colored in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CodeLanguage {
    Json,
    Yaml,
    Jq,
    Markdown,
    Sql,
    Shell,
    Jinja,
    #[default]
    Text,
    /// A language a plugin installed by `register_plugins` gives, by its name; colored as text
    /// until one of that name is installed.
    Plugin(&'static str),
}

impl CodeLanguage {
    /// The language's name as a problem says it: `Not valid JSON`.
    fn name(self) -> &'static str {
        match self {
            Self::Json => "JSON",
            Self::Yaml => "YAML",
            Self::Jq => "jq",
            Self::Markdown => "markdown",
            Self::Sql => "SQL",
            Self::Shell => "shell",
            Self::Jinja => "Jinja",
            Self::Text => "text",
            Self::Plugin(name) => name,
        }
    }
}

/// How the editor stands in its place: boxed like a field, filling a pane, or one line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CodeVariant {
    /// Boxed like an input: field ground, a 1px input edge, radius-control, at least 96 tall.
    #[default]
    Field,
    /// Fills the pane it is in edge to edge on the background, no edge.
    Pane,
    /// One line at a control's height, as a query bar: Enter runs, no line numbers.
    Line,
}

/// The text's size: md at the body's size on the code line height, sm for a dense pane, lg for
/// long writing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CodeSize {
    Sm,
    #[default]
    Md,
    Lg,
}

/// When the caret's line is washed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ActiveLine {
    Off,
    /// While the editor has focus.
    #[default]
    Focused,
    /// While the focus is elsewhere too.
    Always,
}

/// How bad a mark or a problem is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarkTone {
    Error,
    Warning,
    Info,
}

/// A place in the code to mark. A `line` (from 1) alone is washed in its tone; a `range` (byte
/// offsets) or, in JSON or YAML, the value at `path` with its key is underlined. A `message`
/// puts a dot in the lint gutter and says itself on hover.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CodeMark {
    pub line: Option<usize>,
    pub range: Option<(usize, usize)>,
    pub path: Option<Vec<String>>,
    pub tone: MarkTone,
    pub message: Option<String>,
}

/// A word offered while typing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CodeCompletion {
    pub label: String,
    /// Its icon: function, keyword, variable, property, constant, type, enum or text.
    pub kind: Option<String>,
    /// A few words beside the label, such as a signature.
    pub detail: Option<String>,
    /// What the word does, shown beside the list.
    pub info: Option<String>,
    /// The text put in, when it is not the label.
    pub apply: Option<String>,
}

/// One thing the editor shows as a problem: the schema's findings and the marks with a message,
/// in document order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeProblem {
    /// The line it starts on, from 1.
    pub line: usize,
    /// Byte offsets into the document.
    pub from: usize,
    pub to: usize,
    pub tone: MarkTone,
    pub message: String,
}

/// The words the editor says, in English unless given: the schema check's issues (with their
/// `{placeholders}`), and CodeMirror's own words for the find bar and the fold marks.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CodeEditorWords {
    pub issue_required: String,
    pub issue_additional: String,
    pub issue_type: String,
    pub issue_enum: String,
    pub issue_const: String,
    pub issue_minimum: String,
    pub issue_exclusive_minimum: String,
    pub issue_maximum: String,
    pub issue_exclusive_maximum: String,
    pub issue_min_length_one: String,
    pub issue_min_length_other: String,
    pub issue_max_length_one: String,
    pub issue_max_length_other: String,
    pub issue_pattern: String,
    pub issue_format: String,
    pub issue_min_items_one: String,
    pub issue_min_items_other: String,
    pub issue_max_items_one: String,
    pub issue_max_items_other: String,
    pub issue_unique_items: String,
    pub issue_deprecated: String,
    pub issue_unchecked: String,
    pub issue_other: String,
    /// The text is not valid in its language: `Not valid {language}`.
    pub syntax: String,
    /// Beside a deprecated key offered in completion.
    pub deprecated: String,
    /// The editor's name for a screen reader when no `label` is given.
    pub editor: String,
    pub find: String,
    pub replace: String,
    pub next: String,
    pub previous: String,
    pub match_case: String,
    pub regexp: String,
    pub by_word: String,
    pub replace_one: String,
    pub replace_all: String,
    pub close: String,
    pub fold: String,
    pub unfold: String,
    pub folded: String,
}

impl Default for CodeEditorWords {
    fn default() -> Self {
        let s = |w: &str| w.to_owned();
        Self {
            issue_required: s("Missing the property {property}"),
            issue_additional: s("{property} is not a known property"),
            issue_type: s("Expected {a} {expected}"),
            issue_enum: s("Expected one of {values}"),
            issue_const: s("Expected {value}"),
            issue_minimum: s("Must be at least {limit}"),
            issue_exclusive_minimum: s("Must be more than {limit}"),
            issue_maximum: s("Must be at most {limit}"),
            issue_exclusive_maximum: s("Must be less than {limit}"),
            issue_min_length_one: s("Use at least {count} character"),
            issue_min_length_other: s("Use at least {count} characters"),
            issue_max_length_one: s("Use at most {count} character"),
            issue_max_length_other: s("Use at most {count} characters"),
            issue_pattern: s("Must match {pattern}"),
            issue_format: s("Expected the {format} format"),
            issue_min_items_one: s("Needs at least {count} item"),
            issue_min_items_other: s("Needs at least {count} items"),
            issue_max_items_one: s("Holds at most {count} item"),
            issue_max_items_other: s("Holds at most {count} items"),
            issue_unique_items: s("Each item only once"),
            issue_deprecated: s("Deprecated: still accepted, and may be removed in a later version"),
            issue_unchecked: s("The schema cannot be checked: {reason}"),
            issue_other: s("{message}"),
            syntax: s("Not valid {language}"),
            deprecated: s("deprecated"),
            editor: s("Code editor"),
            find: s("Find"),
            replace: s("Replace"),
            next: s("next"),
            previous: s("previous"),
            match_case: s("match case"),
            regexp: s("regexp"),
            by_word: s("by word"),
            replace_one: s("replace"),
            replace_all: s("replace all"),
            close: s("close"),
            fold: s("Fold line"),
            unfold: s("Unfold line"),
            folded: s("folded code"),
        }
    }
}

/// What the editor did this frame.
pub struct CodeEditorResponse {
    /// The editable text's response.
    pub response: Response,
    /// The document changed, by typing or by the editor (indenting, a completion, a replace).
    pub changed: bool,
    /// Run asked: Enter in the line variant, Mod-Enter otherwise.
    pub run: bool,
    /// Mod-s pressed.
    pub saved: bool,
    /// A `[[link]]`'s target, clicked with Mod held.
    pub followed: Option<String>,
    /// The heading after `#` in the `[[link]]` followed, if it names one.
    pub followed_heading: Option<String>,
    /// Every problem shown: the schema's findings and the marks with a message, in document order.
    pub problems: Vec<CodeProblem>,
}

/// Past this many bytes folding, bracket colors, indent guides and link following are dropped.
pub const LARGE_DOCUMENT: usize = 1_000_000;

/// How long the pointer rests on a word before its tooltip shows, in seconds.
const HOVER_WAIT: f32 = 0.3;

/// What a word is, as the app says it.
type Describe<'a> = Box<dyn Fn(&str) -> Option<String> + 'a>;

/// The words offered for the word being typed and what is before it on its line.
type Complete<'a> = Box<dyn Fn(&str, &str) -> Vec<CodeCompletion> + 'a>;

/// How a document is read: its language, and the plugin's reading when the language is one.
#[derive(Clone)]
struct Reading {
    language: CodeLanguage,
    plugin: Option<crate::plugins::Highlight>,
    /// How many times plugins were installed, so a language installed again reads anew.
    generation: u64,
}

impl Reading {
    fn of(ctx: &egui::Context, language: CodeLanguage) -> Self {
        let plugin = match language {
            CodeLanguage::Plugin(name) => crate::plugins::plugin_language(ctx, name).map(|one| one.highlight),
            _ => None,
        };
        Self {
            language,
            plugin,
            generation: crate::plugins::generation(ctx),
        }
    }

    fn tokens(&self, text: &str) -> Vec<Token> {
        match &self.plugin {
            Some(read) => {
                // a plugin's tokens kept in order, inside the text and apart
                let mut out: Vec<Token> = Vec::new();
                for one in read(text) {
                    let from = one.from.max(out.last().map_or(0, |last| last.to));
                    let to = one.to.min(text.len());
                    if to > from && text.is_char_boundary(from) && text.is_char_boundary(to) {
                        out.push(Token {
                            from,
                            to,
                            kind: one.kind,
                        });
                    }
                }
                out
            }
            None => highlight::tokens(self.language, text),
        }
    }
}

/// The code editor; see the module's documentation.
#[must_use = "show it with `.show(ui)`"]
pub struct CodeEditor<'a> {
    id: Id,
    text: &'a mut String,
    label: Option<String>,
    language: CodeLanguage,
    variant: CodeVariant,
    size: CodeSize,
    read_only: bool,
    line_numbers: bool,
    active_line: ActiveLine,
    wrap: bool,
    placeholder: Option<String>,
    marks: &'a [CodeMark],
    completions: &'a [CodeCompletion],
    complete: Option<Complete<'a>>,
    hover: Option<Describe<'a>>,
    links: Option<&'a [String]>,
    schema: Option<&'a serde_json::Value>,
    deferred: Option<&'a dyn Fn(&serde_json::Value) -> bool>,
    search: bool,
    folding: bool,
    bracket_colors: bool,
    indent_guides: bool,
    reveal: Option<usize>,
    height: Option<f32>,
    invalid: bool,
    words: CodeEditorWords,
}

impl<'a> CodeEditor<'a> {
    pub fn new(id: impl egui::AsIdSalt, text: &'a mut String) -> Self {
        Self {
            id: Id::unique(id),
            text,
            label: None,
            language: CodeLanguage::Text,
            variant: CodeVariant::Field,
            size: CodeSize::Md,
            read_only: false,
            line_numbers: true,
            active_line: ActiveLine::Focused,
            wrap: false,
            placeholder: None,
            marks: &[],
            completions: &[],
            complete: None,
            hover: None,
            links: None,
            schema: None,
            deferred: None,
            search: false,
            folding: false,
            bracket_colors: false,
            indent_guides: false,
            reveal: None,
            height: None,
            invalid: false,
            words: CodeEditorWords::default(),
        }
    }

    /// What the editor holds, for a screen reader.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn language(mut self, language: CodeLanguage) -> Self {
        self.language = language;
        self
    }

    pub fn variant(mut self, variant: CodeVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: CodeSize) -> Self {
        self.size = size;
        self
    }

    /// Not editable, still focusable and selectable; outside a pane on muted at 40 percent.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// The line numbers' gutter; never in the line variant.
    pub fn line_numbers(mut self, on: bool) -> Self {
        self.line_numbers = on;
        self
    }

    pub fn active_line(mut self, active: ActiveLine) -> Self {
        self.active_line = active;
        self
    }

    /// Folds long lines at the editor's edge instead of scrolling sideways.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Shown in muted-foreground while the document is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn marks(mut self, marks: &'a [CodeMark]) -> Self {
        self.marks = marks;
        self
    }

    /// Words offered while typing, beside the language's own; an app's word replaces the
    /// language's or the schema's of the same label.
    pub fn completions(mut self, words: &'a [CodeCompletion]) -> Self {
        self.completions = words;
        self
    }

    /// The words offered as they are typed, worked out for the word being typed (letters, digits,
    /// `_`, a leading `$` or `@`) and what stands before it on its line, as the web's completion
    /// source: inside `#{` one list, after a dot another. Given, it stands in for `completions`.
    pub fn complete(mut self, source: impl Fn(&str, &str) -> Vec<CodeCompletion> + 'a) -> Self {
        self.complete = Some(Box::new(source));
        self
    }

    /// What a word is (letters, digits, `_`, a leading `$` or `@`), shown after the pointer rests
    /// on it; `None` shows nothing.
    pub fn hover(mut self, describe: impl Fn(&str) -> Option<String> + 'a) -> Self {
        self.hover = Some(Box::new(describe));
        self
    }

    /// In markdown: the names `[[` completes from; a `[[link]]` clicked with Mod held is followed.
    pub fn links(mut self, notes: &'a [String]) -> Self {
        self.links = Some(notes);
        self
    }

    /// A JSON Schema (2020-12) a JSON or YAML document is checked against and completed from.
    pub fn schema(mut self, schema: &'a serde_json::Value) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Values the app checks itself, such as `${...}` references filled in later: the schema says
    /// nothing of them, nor of what they hold, as the web's `deferred`.
    pub fn deferred(mut self, deferred: &'a dyn Fn(&serde_json::Value) -> bool) -> Self {
        self.deferred = Some(deferred);
        self
    }

    /// The find and replace bar, opened with Mod-f.
    pub fn search(mut self, on: bool) -> Self {
        self.search = on;
        self
    }

    /// A fold gutter and the fold keys, for JSON and YAML.
    pub fn folding(mut self, on: bool) -> Self {
        self.folding = on;
        self
    }

    pub fn bracket_colors(mut self, on: bool) -> Self {
        self.bracket_colors = on;
        self
    }

    pub fn indent_guides(mut self, on: bool) -> Self {
        self.indent_guides = on;
        self
    }

    /// Scrolls to a line (from 1), selects it and focuses the editor, once for each line given.
    pub fn reveal(mut self, line: Option<usize>) -> Self {
        self.reveal = line;
        self
    }

    /// A field's height; without one a field grows with its text, at least 96 tall. A pane fills
    /// its room.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// The critical edge outside a pane, and the invalid ring while focused.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn words(mut self, words: CodeEditorWords) -> Self {
        self.words = words;
        self
    }
}

// --- State ------------------------------------------------------------------------------------

/// What one document is read into, kept while its text stays the same.
#[derive(Clone, Default)]
struct Analysis {
    key: u64,
    tokens: Arc<Vec<Token>>,
    brackets: Arc<Vec<(usize, usize)>>,
    folds: Arc<Vec<Fold>>,
    links: Arc<Vec<WikiLink>>,
}

/// A problem placed on the text.
#[derive(Clone, Debug, PartialEq)]
struct Placed {
    from: usize,
    to: usize,
    tone: MarkTone,
    message: Option<String>,
    /// A range is underlined; a whole line's mark is washed and its message stands on its text.
    underline: bool,
}

#[derive(Clone, Default)]
struct Checked {
    key: u64,
    read: Option<Arc<ReadDocument>>,
    /// Underlined ranges: problems and marks with a range or a path.
    ranges: Arc<Vec<Placed>>,
    /// Lines washed by a mark with a line alone, from 0.
    lines: Arc<Vec<(usize, MarkTone)>>,
    problems: Arc<Vec<CodeProblem>>,
}

/// The words offered, open.
#[derive(Clone, Debug)]
struct Open {
    /// Where the word being completed starts.
    from: usize,
    items: Vec<Item>,
    selected: usize,
    explicit: bool,
}

/// One word offered, and what choosing it does.
#[derive(Clone, Debug)]
struct Item {
    offer: Offer,
    insert: String,
    /// Offsets into `insert` selected once it is in.
    select: (usize, usize),
    fields: Vec<(usize, usize)>,
    /// Inside a JSON string an option replaces the whole string, its quotes included.
    quoted: bool,
    boost: i32,
}

/// A snippet's fields, Tab going from one to the next.
#[derive(Clone, Debug)]
struct Snippet {
    fields: Vec<(usize, usize)>,
    at: usize,
    length: usize,
}

#[derive(Clone, Default)]
struct State {
    analysis: Analysis,
    checked: Checked,
    folded: Vec<usize>,
    open: Option<Open>,
    snippet: Option<Snippet>,
    search: Option<SearchBar>,
    revealed: Option<usize>,
    /// The text's laid-out size last frame, for a field that grows with it.
    content: Vec2,
}

fn hash_of(parts: impl Hash) -> u64 {
    let mut h = DefaultHasher::new();
    parts.hash(&mut h);
    h.finish()
}

fn analyse(text: &str, reading: &Reading, cache: &mut Analysis) {
    let language = reading.language;
    let key = hash_of((text, language, reading.generation));
    if cache.key == key && cache.key != 0 {
        return;
    }
    let tokens = reading.tokens(text);
    let large = text.len() > LARGE_DOCUMENT;
    *cache = Analysis {
        key,
        brackets: Arc::new(if large {
            Vec::new()
        } else {
            edit::bracket_depths(text, &tokens)
        }),
        folds: Arc::new(if large {
            Vec::new()
        } else {
            edit::folds(text, &tokens, language)
        }),
        links: Arc::new(if language == CodeLanguage::Markdown && !large {
            edit::wiki_links(text)
        } else {
            Vec::new()
        }),
        tokens: Arc::new(tokens),
    };
}

// --- Keys -------------------------------------------------------------------------------------

/// Takes a key press with exactly these modifiers (Mod being Cmd on a Mac and Ctrl elsewhere).
fn take(ui: &Ui, key: Key, command: bool, shift: bool, alt: bool) -> bool {
    ui.input_mut(|i| {
        let at = i.events.iter().position(|e| {
            matches!(e, Event::Key { key: k, pressed: true, modifiers, .. }
                if *k == key && modifiers.command == command && modifiers.shift == shift && modifiers.alt == alt)
        });
        at.map(|at| i.events.remove(at)).is_some()
    })
}

/// Takes Ctrl and a key, the physical Control key on every system.
fn take_ctrl(ui: &Ui, key: Key) -> bool {
    ui.input_mut(|i| {
        let at = i.events.iter().position(|e| {
            matches!(e, Event::Key { key: k, pressed: true, modifiers, .. }
                if *k == key && modifiers.ctrl && !modifiers.shift && !modifiers.alt)
        });
        at.map(|at| i.events.remove(at)).is_some()
    })
}

fn selection_of(ctx: &egui::Context, id: Id, text: &str) -> Option<(usize, usize)> {
    let range = TextEditState::load(ctx, id)?.cursor.char_range()?;
    let o = Offsets::new(text);
    Some((
        o.to_byte(text, range.secondary.index.0),
        o.to_byte(text, range.primary.index.0),
    ))
}

fn set_selection(ctx: &egui::Context, id: Id, text: &str, (anchor, head): (usize, usize)) {
    let mut state = TextEditState::load(ctx, id).unwrap_or_default();
    let o = Offsets::new(text);
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(o.to_char(text, anchor)),
        CCursor::new(o.to_char(text, head)),
    )));
    state.store(ctx, id);
}

fn sorted((a, b): (usize, usize)) -> (usize, usize) {
    (a.min(b), a.max(b))
}

// --- Drawing helpers --------------------------------------------------------------------------

/// Where the laid-out text's rows stand: each row's character range and each line's rows.
struct Geometry {
    origin: Pos2,
    rows: Vec<(usize, usize)>,
    lines: Vec<(usize, usize)>,
}

impl Geometry {
    fn new(galley: &egui::Galley, origin: Pos2) -> Self {
        let mut rows = Vec::with_capacity(galley.rows.len());
        let mut lines = Vec::new();
        let mut at = 0;
        let mut start_row = 0;
        for (i, row) in galley.rows.iter().enumerate() {
            let count = row.row.char_count_excluding_newline().0;
            rows.push((at, at + count));
            at += row.char_count_including_newline().0;
            if row.ends_with_newline {
                lines.push((start_row, i));
                start_row = i + 1;
            }
        }
        lines.push((start_row, galley.rows.len().saturating_sub(1)));
        Self { origin, rows, lines }
    }

    /// A line's top and bottom on screen.
    fn line_span(&self, galley: &egui::Galley, line: usize) -> Option<(f32, f32)> {
        let (a, b) = *self.lines.get(line)?;
        let top = galley.rows.get(a)?.min_y();
        let bottom = galley.rows.get(b)?.max_y();
        Some((self.origin.y + top, self.origin.y + bottom))
    }

    /// The rectangles a character range covers, one per row.
    fn rects(&self, galley: &egui::Galley, from: usize, to: usize) -> Vec<Rect> {
        let mut out = Vec::new();
        let first = self.rows.partition_point(|(_, end)| *end < from);
        for (i, &(start, end)) in self.rows.iter().enumerate().skip(first) {
            if start > to || (start == to && to > from) {
                break;
            }
            let a = from.max(start);
            let b = to.min(end);
            if b < a {
                continue;
            }
            let row = &galley.rows[i];
            let x0 = row.pos.x + row.row.x_offset((a - start).into());
            let x1 = row.pos.x + row.row.x_offset((b - start).into());
            out.push(Rect::from_min_max(
                pos2(self.origin.x + x0, self.origin.y + row.min_y()),
                pos2(self.origin.x + x1, self.origin.y + row.max_y()),
            ));
        }
        out
    }
}

/// A wavy underline under a span, in a tone, 0.2em under the text.
fn wavy(shapes: &mut Vec<Shape>, rect: Rect, size: f32, color: Color32) {
    let y = rect.center().y + size * 0.5 + 1.0;
    let step = 2.0;
    let mut points = Vec::new();
    let mut x = rect.left();
    let mut up = true;
    while x <= rect.right().max(rect.left() + 4.0) {
        points.push(pos2(x, if up { y - 1.25 } else { y + 1.25 }));
        up = !up;
        x += step;
    }
    if points.len() > 1 {
        shapes.push(Shape::line(points, Stroke::new(1.0, color)));
    }
}

fn tone_color(t: &Tokens, tone: MarkTone) -> Color32 {
    t.color(match tone {
        MarkTone::Error => "critical",
        MarkTone::Warning => "warning",
        MarkTone::Info => "info",
    })
}

/// The look a document is laid out in.
struct Look {
    regular: FontId,
    bold: FontId,
    line_height: f32,
    ink: Color32,
    kinds: Vec<(Kind, Color32)>,
    brackets: Option<[Color32; 3]>,
    /// The `[[links]]` underlined while Mod is held.
    held: Option<Color32>,
    wrap: bool,
}

fn layout(text: &str, analysis: &Analysis, hidden: &[(usize, usize)], look: &Look, wrap_width: f32) -> LayoutJob {
    let mut job = LayoutJob {
        halign: egui::Align::LEFT,
        keep_trailing_whitespace: true,
        ..Default::default()
    };
    job.wrap.max_width = if look.wrap { wrap_width } else { f32::INFINITY };
    let tokens = &analysis.tokens;
    let brackets: &[(usize, usize)] = if look.brackets.is_some() {
        &analysis.brackets
    } else {
        &[]
    };
    let links: Vec<(usize, usize)> = if look.held.is_some() {
        analysis.links.iter().map(|l| (l.from, l.to)).collect()
    } else {
        Vec::new()
    };
    let mut cuts: Vec<usize> = Vec::with_capacity(tokens.len() * 2 + brackets.len() * 2 + 2);
    cuts.push(0);
    cuts.push(text.len());
    for tok in tokens.iter() {
        cuts.push(tok.from);
        cuts.push(tok.to);
    }
    for (at, _) in brackets {
        cuts.push(*at);
        cuts.push(at + 1);
    }
    for (a, b) in hidden.iter().chain(links.iter()) {
        cuts.push(*a);
        cuts.push(*b);
    }
    cuts.retain(|c| *c <= text.len());
    cuts.sort_unstable();
    cuts.dedup();
    let (mut ti, mut bi) = (0, 0);
    for pair in cuts.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if a >= b {
            continue;
        }
        while ti < tokens.len() && tokens[ti].to <= a {
            ti += 1;
        }
        while bi < brackets.len() && brackets[bi].0 < a {
            bi += 1;
        }
        let kind = tokens.get(ti).filter(|tok| tok.from <= a).map(|tok| tok.kind);
        let is_hidden = hidden.iter().any(|(x, y)| *x <= a && a < *y);
        let format = if is_hidden {
            TextFormat {
                font_id: FontId::monospace(1.0),
                color: Color32::TRANSPARENT,
                line_height: Some(0.0),
                ..Default::default()
            }
        } else {
            let mut color = kind
                .and_then(|k| look.kinds.iter().find(|(x, _)| *x == k).map(|(_, c)| *c))
                .unwrap_or(look.ink);
            if let (Some(colors), Some(&(at, depth))) = (look.brackets, brackets.get(bi))
                && at == a
                && b == a + 1
            {
                color = colors[depth % 3];
            }
            let heavy = matches!(kind, Some(Kind::Heading | Kind::Strong));
            let mut format = TextFormat {
                font_id: if heavy { look.bold.clone() } else { look.regular.clone() },
                color,
                line_height: Some(look.line_height),
                italics: matches!(kind, Some(Kind::Comment | Kind::Emphasis)),
                valign: egui::Align::Center,
                ..Default::default()
            };
            if kind == Some(Kind::Strikethrough) {
                format.strikethrough = Stroke::new(1.0, color);
            }
            if let Some(ink) = look.held
                && links.iter().any(|(x, y)| *x <= a && a < *y)
            {
                format.underline = Stroke::new(1.0, ink);
            }
            format
        };
        job.append(&text[a..b], 0.0, format);
    }
    job
}

// --- Completion -------------------------------------------------------------------------------

struct Sources<'s> {
    language: CodeLanguage,
    completions: &'s [CodeCompletion],
    complete: Option<&'s Complete<'s>>,
    links: Option<&'s [String]>,
    schema: Option<&'s serde_json::Value>,
    words: &'s CodeEditorWords,
}

fn app_item(c: &CodeCompletion, insert: String) -> Item {
    let len = insert.len();
    Item {
        offer: Offer {
            label: c.label.clone(),
            detail: c.detail.clone(),
            info: c.info.clone(),
            kind: c.kind.clone(),
            deprecated: false,
        },
        insert,
        select: (len, len),
        fields: Vec::new(),
        quoted: false,
        boost: 0,
    }
}

/// The words offered at the caret: a `[[link]]` being typed, else the app's words, the language's
/// and the schema's, the app's first and in place of any of the same label.
fn offers(text: &str, caret: usize, explicit: bool, sources: &Sources, tokens: &[Token]) -> Option<(usize, Vec<Item>)> {
    if let Some(links) = sources.links
        && sources.language == CodeLanguage::Markdown
        && let Some((from, _)) = edit::wiki_query(text, caret)
    {
        let closed = text[caret..].starts_with("]]");
        let items = links
            .iter()
            .map(|name| {
                let mut item = app_item(
                    &CodeCompletion {
                        label: name.clone(),
                        kind: Some("link".into()),
                        ..Default::default()
                    },
                    if closed { name.clone() } else { format!("{name}]]") },
                );
                item.offer.kind = Some("link".into());
                item
            })
            .collect();
        return Some((from, items));
    }
    let before = &text[..caret];
    // the word being typed: a leading `$` or `@` included, a `.` not
    let word_len = before.len() - before.trim_end_matches(|c: char| c.is_alphanumeric() || c == '_').len();
    let mut from = caret - word_len;
    if before[..from].ends_with(['$', '@']) {
        from -= 1;
    }
    let after_dot = before[..from].ends_with('.');
    let mut results: Vec<(usize, Vec<Item>)> = Vec::new();
    let worked: Vec<CodeCompletion>;
    let completions = match sources.complete {
        Some(complete) if from < caret || explicit || after_dot => {
            let line_start = before[..from].rfind('\n').map_or(0, |at| at + 1);
            worked = complete(&before[from..], &before[line_start..from]);
            worked.as_slice()
        }
        Some(_) => &[],
        None => sources.completions,
    };
    if !completions.is_empty() && (from < caret || explicit || after_dot) {
        let items = completions
            .iter()
            .map(|c| app_item(c, c.apply.clone().unwrap_or_else(|| c.label.clone())))
            .collect();
        results.push((from, items));
    }
    if sources.language == CodeLanguage::Jq && !edit::in_string_or_comment(tokens, caret) {
        let name_from = caret
            - (before.len()
                - before
                    .trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_')
                    .len());
        let lead = before[..name_from].chars().next_back();
        if (name_from < caret || explicit) && !matches!(lead, Some('.' | '$' | '@')) {
            let mut items: Vec<Item> = highlight::JQ_BUILTINS
                .iter()
                .map(|(signature, info)| {
                    let (label, detail) = match signature.find('(') {
                        Some(p) => (&signature[..p], Some(signature[p..].to_owned())),
                        None => (*signature, None),
                    };
                    app_item(
                        &CodeCompletion {
                            label: label.to_owned(),
                            kind: Some("function".into()),
                            detail,
                            info: Some((*info).to_owned()),
                            apply: None,
                        },
                        label.to_owned(),
                    )
                })
                .collect();
            items.extend(highlight::JQ_KEYWORDS.iter().map(|k| {
                app_item(
                    &CodeCompletion {
                        label: (*k).to_owned(),
                        kind: Some("keyword".into()),
                        ..Default::default()
                    },
                    (*k).to_owned(),
                )
            }));
            results.push((name_from, items));
        }
    }
    if let Some(schema) = sources.schema
        && matches!(sources.language, CodeLanguage::Json | CodeLanguage::Yaml)
    {
        let json = sources.language == CodeLanguage::Json;
        let slot = if json {
            schema::json_slot(text, caret)
        } else {
            Some(schema::yaml_slot(text, caret))
        };
        if let Some(slot) = slot
            && (slot.from != caret || explicit || (json && slot.quoted))
        {
            let offsets = Offsets::new(text);
            let line = offsets.line_of(slot.from);
            let line_start = offsets.line_start(line);
            let column = slot.from - line_start;
            let indent = text[line_start..].len() - text[line_start..].trim_start_matches(' ').len();
            let present = if !slot.at_key {
                Vec::new()
            } else if json {
                schema::json_keys(text, caret)
            } else {
                schema::yaml_keys(text, caret, column)
            };
            let read = if json {
                document::read_json(text)
            } else {
                document::read_yaml(text)
            };
            let options = schema::options(
                schema,
                json,
                &slot,
                &present,
                column,
                read.value.as_ref(),
                sources.words,
                indent.min(column),
            );
            if !options.is_empty() {
                let items = options
                    .into_iter()
                    .map(|o| Item {
                        offer: Offer {
                            label: o.label,
                            detail: o.detail,
                            info: o.info,
                            kind: Some(o.kind.to_owned()),
                            deprecated: o.deprecated,
                        },
                        insert: o.insert,
                        select: o.select,
                        fields: o.fields,
                        quoted: slot.quoted,
                        boost: o.boost,
                    })
                    .collect();
                results.push((slot.from, items));
            }
        }
    }
    let (first_from, _) = *results.first()?;
    let own: Vec<String> = completions.iter().map(|c| c.label.clone()).collect();
    let mut items: Vec<Item> = Vec::new();
    for (i, (at, list)) in results.into_iter().enumerate() {
        if at != first_from {
            continue;
        }
        for item in list {
            if i > 0 && own.contains(&item.offer.label) {
                continue;
            }
            items.push(item);
        }
    }
    (!items.is_empty()).then_some((first_from, items))
}

/// The words that match what is typed, best first: a prefix, then inside, then the letters in order.
fn filter(items: &[Item], typed: &str) -> Vec<Item> {
    let lower = typed.to_lowercase();
    let mut kept: Vec<(u8, i32, usize, Item)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            let label = item.offer.label.to_lowercase();
            let rank = if typed.is_empty() || label.starts_with(&lower) {
                0
            } else if label.contains(&lower) {
                1
            } else if popup::matched(&item.offer.label, typed).is_some() {
                2
            } else {
                return None;
            };
            Some((rank, -item.boost, i, item.clone()))
        })
        .collect();
    kept.sort_by_key(|(rank, boost, i, _)| (*rank, *boost, *i));
    kept.into_iter().map(|(.., item)| item).collect()
}

// --- Show -------------------------------------------------------------------------------------

impl CodeEditor<'_> {
    pub fn show(mut self, ui: &mut Ui) -> CodeEditorResponse {
        let ctx = ui.ctx().clone();
        let t = tokens(&ctx);
        let id = self.id;
        let text_id = id.with("text");
        let mut st: State = ctx.data_mut(|d| d.remove_temp::<State>(id)).unwrap_or_default();
        let one_line = self.variant == CodeVariant::Line;
        let large = self.text.len() > LARGE_DOCUMENT;
        let editable = !self.read_only;
        let numbered = self.line_numbers && !one_line;
        let folding =
            self.folding && !one_line && !large && matches!(self.language, CodeLanguage::Json | CodeLanguage::Yaml);
        let following = self.links.is_some() && self.language == CodeLanguage::Markdown && !large;
        let schema_language = self.schema.is_some() && matches!(self.language, CodeLanguage::Json | CodeLanguage::Yaml);
        let diagnosing = schema_language || self.marks.iter().any(|m| m.message.is_some());
        let before = self.text.clone();

        // the text's size and line height, as the spec's SIZES set them
        let (size, line_height) = match self.size {
            CodeSize::Sm => (t.text("xs").size, t.text("xs").line_height),
            CodeSize::Md => {
                // leading-code, the code line height, or text.sm's own until the tokens carry it
                let leading = t.lengths().find(|(k, _)| *k == "leading-code").map(|(_, v)| v);
                (t.text("sm").size, leading.unwrap_or(t.text("sm").line_height))
            }
            CodeSize::Lg => (t.text("base").size, t.text("base").line_height),
        };
        let regular = FontId::monospace(size);
        let bold = crate::mono_font(&ctx, size, 600.0);
        let char_width = ui.fonts_mut(|f| f.glyph_width(&regular, '0'));

        let focused = ctx.memory(|m| m.has_focus(text_id));
        let reading = Reading::of(&ctx, self.language);
        let mut run = false;
        let mut saved = false;
        let mut explicit = false;

        analyse(self.text, &reading, &mut st.analysis);

        // --- keys, before the text sees them
        if focused {
            if take(ui, Key::S, true, false, false) {
                saved = true;
            }
            if take(ui, Key::Enter, true, false, false) {
                run = true;
            }
            if self.search && take(ui, Key::F, true, false, false) {
                let mut bar = st.search.take().unwrap_or_default();
                if let Some(sel) = selection_of(&ctx, text_id, self.text).map(sorted)
                    && sel.0 < sel.1
                    && !self.text[sel.0..sel.1].contains('\n')
                {
                    bar.query.search = self.text[sel.0..sel.1].to_owned();
                }
                bar.focus = true;
                st.search = Some(bar);
            }
            if take_ctrl(ui, Key::Space) {
                explicit = true;
            }
            // the list of words, while open
            if st.open.is_some() {
                let len = st.open.as_ref().map_or(0, |o| o.items.len());
                let step = |delta: isize, open: &mut Open| {
                    open.selected = (open.selected as isize + delta).rem_euclid(len.max(1) as isize) as usize;
                };
                if take(ui, Key::ArrowDown, false, false, false)
                    && let Some(open) = st.open.as_mut()
                {
                    step(1, open);
                }
                if take(ui, Key::ArrowUp, false, false, false)
                    && let Some(open) = st.open.as_mut()
                {
                    step(-1, open);
                }
                if take(ui, Key::PageDown, false, false, false)
                    && let Some(open) = st.open.as_mut()
                {
                    open.selected = (open.selected + 8).min(len.saturating_sub(1));
                }
                if take(ui, Key::PageUp, false, false, false)
                    && let Some(open) = st.open.as_mut()
                {
                    open.selected = open.selected.saturating_sub(8);
                }
                if take(ui, Key::Escape, false, false, false) {
                    st.open = None;
                }
                if take(ui, Key::Enter, false, false, false)
                    && let Some(open) = st.open.take()
                {
                    self.accept(&ctx, text_id, &open, open.selected, &mut st);
                }
            }
            // a snippet's fields
            if let Some(snippet) = st.snippet.clone() {
                let forward = take(ui, Key::Tab, false, false, false);
                let back = !forward && take(ui, Key::Tab, false, true, false);
                if take(ui, Key::Escape, false, false, false) {
                    st.snippet = None;
                } else if forward || back {
                    let delta = self.text.len() as isize - snippet.length as isize;
                    let mut fields = snippet.fields.clone();
                    // the field typed in grows by what was typed; the ones after it move
                    if let Some(f) = fields.get_mut(snippet.at) {
                        f.1 = (f.1 as isize + delta).max(f.0 as isize) as usize;
                    }
                    for f in fields.iter_mut().skip(snippet.at + 1) {
                        f.0 = (f.0 as isize + delta).max(0) as usize;
                        f.1 = (f.1 as isize + delta).max(0) as usize;
                    }
                    let next = if forward {
                        snippet.at + 1
                    } else {
                        snippet.at.saturating_sub(1)
                    };
                    if let Some(&(a, b)) = fields.get(next) {
                        set_selection(
                            &ctx,
                            text_id,
                            self.text,
                            (a.min(self.text.len()), b.min(self.text.len())),
                        );
                        st.snippet = Some(Snippet {
                            fields,
                            at: next,
                            length: self.text.len(),
                        });
                    } else {
                        let end = fields.last().map_or(0, |f| f.1).min(self.text.len());
                        set_selection(&ctx, text_id, self.text, (end, end));
                        st.snippet = None;
                    }
                }
            }
            if one_line {
                if take(ui, Key::Enter, false, false, false) {
                    run = true;
                }
            } else if editable {
                if take(ui, Key::Tab, false, true, false)
                    && let Some(sel) = selection_of(&ctx, text_id, self.text)
                {
                    let (a, b) = sorted(sel);
                    let change = edit::indent_less(self.text, a, b);
                    *self.text = change.text;
                    set_selection(&ctx, text_id, self.text, change.selection);
                }
                if take(ui, Key::Tab, false, false, false)
                    && let Some(sel) = selection_of(&ctx, text_id, self.text)
                {
                    let (a, b) = sorted(sel);
                    let change = edit::indent_more(self.text, a, b);
                    *self.text = change.text;
                    set_selection(&ctx, text_id, self.text, change.selection);
                }
                if take(ui, Key::Enter, false, false, false)
                    && let Some(sel) = selection_of(&ctx, text_id, self.text)
                {
                    let (a, b) = sorted(sel);
                    let change = edit::newline(self.text, a, b, self.language);
                    *self.text = change.text;
                    set_selection(&ctx, text_id, self.text, change.selection);
                }
            }
            if folding {
                analyse(self.text, &reading, &mut st.analysis);
                let caret_line = selection_of(&ctx, text_id, self.text)
                    .map(|(_, h)| Offsets::new(self.text).line_of(h))
                    .unwrap_or(0);
                let fold_all = take(ui, Key::Period, true, false, true) || take_ctrl_alt(ui, Key::OpenBracket);
                let unfold_all = take(ui, Key::Period, true, true, true) || take_ctrl_alt(ui, Key::CloseBracket);
                let fold = take(ui, Key::Period, true, false, false) || take_ctrl_shift(ui, Key::OpenBracket);
                let unfold = take(ui, Key::Period, true, true, false) || take_ctrl_shift(ui, Key::CloseBracket);
                if fold_all {
                    st.folded = st.analysis.folds.iter().map(|f| f.line).collect();
                }
                if unfold_all {
                    st.folded.clear();
                }
                if fold && st.analysis.folds.iter().any(|f| f.line == caret_line) && !st.folded.contains(&caret_line) {
                    st.folded.push(caret_line);
                }
                if unfold {
                    st.folded.retain(|l| *l != caret_line);
                }
            }
        }
        if one_line && self.text.contains('\n') {
            *self.text = self.text.replace("\r\n", " ").replace('\n', " ");
        }
        if let Some(bar) = st.search.as_ref()
            && !self.search
        {
            let _ = bar;
            st.search = None;
        }

        // --- the box
        let paper = match self.variant {
            CodeVariant::Pane => t.color("background"),
            _ if self.read_only => t.color("background").lerp_to_gamma(t.color("muted"), 0.4),
            _ => t.color("field"),
        };
        let edged = self.variant != CodeVariant::Pane;
        let width = ui.available_width();
        let height = match self.variant {
            CodeVariant::Pane => {
                let room = ui.available_height();
                if room.is_finite() && room > 0.0 {
                    room
                } else {
                    st.content.y.max(96.0)
                }
            }
            CodeVariant::Line => super::finger_tall(&ctx, &t, t.density("control")),
            CodeVariant::Field => self.height.unwrap_or_else(|| (st.content.y + 2.0).max(96.0)),
        };
        let (outer, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let box_shape = ui.painter().add(Shape::Noop);
        let inner = if edged { outer.shrink(1.0) } else { outer };
        let mut area = ui.new_child(
            UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        area.set_clip_rect(inner.intersect(ui.clip_rect()));

        // --- the find bar
        let mut text_rect = inner;
        let mut asked = popup::SearchAsked::default();
        if let Some(bar) = st.search.as_mut() {
            let (taken, said) = popup::search_bar(
                &mut area,
                inner,
                id.with("search"),
                &t,
                bar,
                &self.words,
                self.read_only,
            );
            text_rect.min.y += taken;
            asked = said;
        }

        // --- the gutters
        analyse(self.text, &reading, &mut st.analysis);
        st.folded.retain(|l| st.analysis.folds.iter().any(|f| f.line == *l));
        if !folding {
            st.folded.clear();
        }
        let line_count = Offsets::new(self.text).lines();
        let digits = line_count.to_string().len().max(1) as f32;
        let numbers_width = if numbered {
            (12.0 + 8.0 + digits * char_width).max(40.0)
        } else {
            0.0
        };
        let fold_width = if folding { char_width + 8.0 } else { 0.0 };
        let lint_width = if diagnosing && !one_line { 16.0 } else { 0.0 };
        let gutter = numbers_width + fold_width + lint_width;

        // --- the text
        let modifiers = ctx.input(|i| i.modifiers);
        let pointer_over = ctx
            .input(|i| i.pointer.hover_pos())
            .is_some_and(|p| text_rect.contains(p));
        let held = following && modifiers.command && pointer_over;
        let colors = |k: Kind| k.ink().map(|key| (k, t.color(key)));
        let look = Look {
            regular: regular.clone(),
            bold: bold.clone(),
            line_height: if one_line {
                ui.fonts_mut(|f| f.row_height(&regular))
            } else {
                line_height
            },
            ink: t.color("foreground"),
            kinds: [
                Kind::Keyword,
                Kind::Operator,
                Kind::Punctuation,
                Kind::String,
                Kind::Regexp,
                Kind::Number,
                Kind::Constant,
                Kind::Comment,
                Kind::Function,
                Kind::Variable,
                Kind::Type,
                Kind::Property,
                Kind::Tag,
                Kind::Attribute,
                Kind::Heading,
                Kind::Link,
                Kind::Inserted,
                Kind::Deleted,
                Kind::Invalid,
                Kind::Quote,
            ]
            .into_iter()
            .filter_map(colors)
            .collect(),
            brackets: (self.bracket_colors && !large).then(|| {
                [
                    t.color("code-bracket-1"),
                    t.color("code-bracket-2"),
                    t.color("code-bracket-3"),
                ]
            }),
            held: held.then(|| t.color("code-link")),
            wrap: self.wrap && !one_line,
        };
        let folded_lines = st.folded.clone();
        let mut cache = st.analysis.clone();
        let mut layouter = |ui: &Ui, buffer: &dyn egui::TextBuffer, wrap_width: f32| {
            let text = buffer.as_str();
            analyse(text, &reading, &mut cache);
            let offsets = Offsets::new(text);
            let hidden: Vec<(usize, usize)> = cache
                .folds
                .iter()
                .filter(|f| folded_lines.contains(&f.line))
                .map(|f| {
                    let end = offsets.line_end(text, f.last);
                    (offsets.line_start(f.first), (end + 1).min(text.len()))
                })
                .collect();
            let job = layout(text, &cache, &hidden, &look, wrap_width);
            ui.fonts_mut(|f| f.layout_job(job))
        };

        let placeholder = self.placeholder.as_ref().map(|p| {
            egui::RichText::new(p)
                .font(regular.clone())
                .color(t.color("muted-foreground"))
        });
        let name = self.label.clone().unwrap_or_else(|| self.words.editor.clone());
        let visuals_scope = |ui: &mut Ui| {
            let v = ui.visuals_mut();
            v.selection.bg_fill = t.color_at("primary", 0.28);
            v.text_cursor.stroke = Stroke::new(1.5, t.color("foreground"));
        };

        // a read-only document is shown from a copy egui cannot change: still focusable and selectable
        let mut text_holder = std::mem::take(self.text);
        let readonly_buffer = if self.read_only {
            text_holder.clone()
        } else {
            String::new()
        };
        let mut readonly_view: &str = readonly_buffer.as_str();

        let mut text_ui = area.new_child(
            UiBuilder::new()
                .max_rect(text_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        text_ui.set_clip_rect(text_rect.intersect(area.clip_rect()));
        visuals_scope(&mut text_ui);
        // the editor's ground, its wash over the box's own
        let editor_ground = paper.blend(t.color("editor-ground"));
        let back = text_ui.painter().add(Shape::Noop);

        let shown = if one_line {
            let pad = ((text_rect.height() - look.line_height) / 2.0).floor().max(0.0) as i8;
            let mut edit = if self.read_only {
                TextEdit::singleline(&mut readonly_view)
            } else {
                TextEdit::singleline(&mut text_holder)
            };
            edit = edit
                .id(text_id)
                .font(regular.clone())
                .text_color(t.color("foreground"))
                .frame(Frame::NONE.inner_margin(Margin {
                    left: 12,
                    right: 12,
                    top: pad,
                    bottom: pad,
                }))
                .desired_width(text_rect.width())
                .min_size(text_rect.size())
                .clip_text(true)
                .return_key(None)
                .layouter(&mut layouter);
            if let Some(p) = placeholder.clone() {
                edit = edit.hint_text(p);
            }
            let output = edit.show(&mut text_ui);
            (output, text_rect)
        } else {
            let fixed = self.variant == CodeVariant::Pane || self.height.is_some();
            let scroll = ScrollArea::new([!look.wrap, fixed])
                .id_salt(id.with("scroll"))
                .auto_shrink([false, false])
                .max_height(text_rect.height());
            let content_width = st.content.x;
            let out = scroll.show_viewport(&mut text_ui, |ui, viewport| {
                visuals_scope(ui);
                let visible = ui.clip_rect();
                let min_width = if look.wrap {
                    visible.width()
                } else {
                    (content_width + gutter + 24.0).max(visible.width())
                };
                let mut edit = if self.read_only {
                    TextEdit::multiline(&mut readonly_view)
                } else {
                    TextEdit::multiline(&mut text_holder)
                };
                edit = edit
                    .id(text_id)
                    .font(regular.clone())
                    .text_color(t.color("foreground"))
                    .frame(Frame::NONE.inner_margin(Margin {
                        left: (gutter + 12.0).round().min(127.0) as i8,
                        right: 12,
                        top: 8,
                        bottom: 8,
                    }))
                    .desired_width(if look.wrap { visible.width() } else { f32::INFINITY })
                    .min_size(vec2(
                        min_width,
                        if fixed {
                            viewport.height().max(text_rect.height())
                        } else {
                            0.0
                        },
                    ))
                    .lock_focus(true)
                    .return_key(None)
                    .layouter(&mut layouter);
                if let Some(p) = placeholder.clone() {
                    edit = edit.hint_text(p);
                }
                let output = edit.show(ui);
                (output, visible)
            });
            out.inner
        };
        let (output, visible) = shown;
        *self.text = text_holder;
        drop(layouter);
        st.analysis = cache;
        let galley = output.galley.clone();
        let response = output.response.response.clone().accessible_name(&name);
        let has_focus = response.has_focus();
        let origin = output.galley_pos - vec2(galley.rect.left(), 0.0);
        let geometry = Geometry::new(&galley, origin);
        let text: &str = self.text.as_str();
        let offsets = Offsets::new(text);
        let caret = output
            .cursor_range
            .or_else(|| output.state.cursor.char_range())
            .map(|r| {
                (
                    offsets.to_byte(text, r.secondary.index.0),
                    offsets.to_byte(text, r.primary.index.0),
                )
            });
        let caret_line = caret.map(|(_, h)| offsets.line_of(h));
        let content = galley.size() + vec2(0.0, 16.0);
        if (content - st.content).length() > 0.5 {
            st.content = content;
            ctx.request_repaint();
        }

        // a caret inside a folded block opens it
        if let Some(line) = caret_line {
            let folds = st.analysis.folds.clone();
            st.folded
                .retain(|l| !folds.iter().any(|f| f.line == *l && f.first <= line && line <= f.last));
        }

        // --- the checks: the schema's findings and the marks
        let mark_key = hash_of((
            text,
            self.marks,
            self.schema.map(|s| s as *const _ as usize),
            self.language,
            &self.words,
        ));
        if st.checked.key != mark_key {
            st.checked = self.check(text, &offsets, mark_key);
        }
        let checked = st.checked.clone();

        // --- reveal
        if let Some(line) = self.reveal {
            if st.revealed != Some(line) && line >= 1 && line <= offsets.lines() {
                st.revealed = Some(line);
                let from = offsets.line_start(line - 1);
                let to = offsets.line_end(text, line - 1);
                set_selection(&ctx, text_id, text, (from, to));
                response.request_focus();
                if let Some((top, bottom)) = geometry.line_span(&galley, line - 1) {
                    text_ui.scroll_to_rect(
                        Rect::from_min_max(pos2(visible.left(), top), pos2(visible.right(), bottom)),
                        Some(egui::Align::Center),
                    );
                }
                ctx.request_repaint();
            }
        } else {
            st.revealed = None;
        }

        // --- the search
        let matches = st
            .search
            .as_ref()
            .map(|bar| edit::matches(text, &bar.query))
            .unwrap_or_default();
        let mut changed_by_bar = false;
        if let Some(bar) = st.search.as_mut()
            && bar.current.is_some_and(|c| !matches.contains(&c))
        {
            bar.current = None;
        }
        if st.search.is_some() {
            let sel = st
                .search
                .as_ref()
                .and_then(|b| b.current)
                .or(caret.map(sorted))
                .unwrap_or((0, 0));
            let go = |target: Option<&(usize, usize)>, text: &str| -> Option<(usize, usize)> {
                let &(a, b) = target?;
                set_selection(&ctx, text_id, text, (a, b));
                if let Some(r) = geometry
                    .rects(&galley, offsets.to_char(text, a), offsets.to_char(text, b))
                    .first()
                {
                    text_ui.scroll_to_rect(*r, Some(egui::Align::Center));
                }
                Some((a, b))
            };
            let mut gone = None;
            if asked.next {
                gone = go(
                    matches.iter().find(|m| m.0 >= sel.1 && *m != &sel).or(matches.first()),
                    text,
                );
            }
            if asked.previous {
                gone = go(
                    matches
                        .iter()
                        .rev()
                        .find(|m| m.1 <= sel.0 && *m != &sel)
                        .or(matches.last()),
                    text,
                );
            }
            if let (Some(gone), Some(bar)) = (gone, st.search.as_mut()) {
                bar.current = Some(gone);
            }
            if editable && (asked.replace || asked.replace_all) {
                let with = st.search.as_ref().map(|b| b.replace.clone()).unwrap_or_default();
                if asked.replace_all {
                    let mut out = String::with_capacity(text.len());
                    let mut last = 0;
                    for (a, b) in &matches {
                        out.push_str(&text[last..*a]);
                        out.push_str(&with);
                        last = *b;
                    }
                    out.push_str(&text[last..]);
                    *self.text = out;
                    changed_by_bar = true;
                } else if let Some(&(a, b)) = matches
                    .iter()
                    .find(|m| **m == sel)
                    .or(matches.iter().find(|m| m.0 >= sel.0))
                {
                    let mut out = self.text.clone();
                    out.replace_range(a..b, &with);
                    *self.text = out;
                    let next = a + with.len();
                    set_selection(&ctx, text_id, self.text, (next, next));
                    changed_by_bar = true;
                }
            }
            if asked.close {
                st.search = None;
                response.request_focus();
            }
        }
        let text: &str = self.text.as_str();

        // --- under the text: its ground, the caret's line, line marks, matches, the bracket
        let mut under: Vec<Shape> = vec![Shape::rect_filled(visible, 0.0, editor_ground)];
        let full = |top: f32, bottom: f32| {
            Rect::from_min_max(
                pos2(visible.left() + gutter, top),
                pos2(visible.right().max(origin.x + galley.size().x + 12.0), bottom),
            )
        };
        let active = match self.active_line {
            ActiveLine::Off => false,
            ActiveLine::Focused => has_focus,
            ActiveLine::Always => true,
        } && !one_line;
        if active
            && let Some(line) = caret_line
            && let Some((top, bottom)) = geometry.line_span(&galley, line)
        {
            under.push(Shape::rect_filled(
                full(top, bottom),
                0.0,
                t.color_at("foreground", 0.05),
            ));
        }
        if !self.text.is_empty() || !one_line {
            for (line, tone) in checked.lines.iter() {
                if let Some((top, bottom)) = geometry.line_span(&galley, *line) {
                    let share = if *tone == MarkTone::Info { 0.12 } else { 0.14 };
                    under.push(Shape::rect_filled(
                        full(top, bottom),
                        0.0,
                        tone_color(&t, *tone).gamma_multiply(share),
                    ));
                }
            }
        }
        let current = st.search.as_ref().and_then(|b| b.current).or(caret.map(sorted));
        let half = CornerRadius_half(&t);
        for m in &matches {
            let share = if Some(*m) == current { 0.48 } else { 0.24 };
            for r in geometry.rects(&galley, offsets.to_char(text, m.0), offsets.to_char(text, m.1)) {
                under.push(Shape::rect_filled(r, half, t.color_at("warning", share)));
            }
        }
        // the problem the caret stands in
        if let Some((_, h)) = caret
            && has_focus
        {
            for p in checked
                .ranges
                .iter()
                .filter(|p| p.message.is_some() && p.from <= h && h <= p.to)
            {
                for r in geometry.rects(&galley, offsets.to_char(text, p.from), offsets.to_char(text, p.to)) {
                    under.push(Shape::rect_filled(r, 0.0, t.color_at("warning", 0.18)));
                }
            }
        }
        if has_focus
            && let Some((_, h)) = caret
            && caret.is_some_and(|(a, b)| a == b)
            && let Some((a, b)) = edit::matching_bracket(text, &st.analysis.tokens, h)
        {
            for at in [a, b] {
                for r in geometry.rects(&galley, offsets.to_char(text, at), offsets.to_char(text, at) + 1) {
                    under.push(Shape::rect_filled(r, 0.0, t.color_at("primary", 0.22)));
                    under.push(Shape::rect_stroke(
                        r,
                        0.0,
                        Stroke::new(1.0, t.color_at("primary", 0.5)),
                        StrokeKind::Inside,
                    ));
                }
            }
        }
        // indent guides: a rule at each indent step of a line's leading spaces
        if self.indent_guides && !large && !one_line {
            let step = edit::UNIT.len() as f32 * char_width;
            for line in 0..offsets.lines() {
                let start = offsets.line_start(line);
                let end = offsets.line_end(text, line);
                let lead = text[start..end].len() - text[start..end].trim_start_matches(' ').len();
                if lead < edit::UNIT.len() || lead == end - start {
                    continue;
                }
                let Some((top, bottom)) = geometry.line_span(&galley, line) else {
                    continue;
                };
                if bottom < visible.top() || top > visible.bottom() || bottom - top < 1.0 {
                    continue;
                }
                let x0 = origin.x;
                let mut k = 0;
                while (k * edit::UNIT.len()) < lead {
                    let x = (x0 + k as f32 * step).round() + 0.5;
                    under.push(Shape::line_segment(
                        [pos2(x, top), pos2(x, bottom)],
                        Stroke::new(1.0, t.color("border")),
                    ));
                    k += 1;
                }
            }
        }
        text_ui.painter().set(back, Shape::Vec(under));

        // --- over the text: underlines, fold placeholders
        let painter = text_ui.painter().clone();
        let mut over: Vec<Shape> = Vec::new();
        for p in checked.ranges.iter().filter(|p| p.underline) {
            let (a, b) = (offsets.to_char(text, p.from), offsets.to_char(text, p.to));
            for r in geometry.rects(&galley, a, b.max(a + 1)) {
                wavy(&mut over, r, size, tone_color(&t, p.tone));
            }
        }
        painter.extend(over);
        let mut unfold_clicked = None;
        for f in st.analysis.folds.iter().filter(|f| st.folded.contains(&f.line)) {
            let end = offsets.line_end(text, f.line);
            let Some(r) = geometry
                .rects(&galley, offsets.to_char(text, end), offsets.to_char(text, end))
                .into_iter()
                .next()
            else {
                continue;
            };
            let words = "\u{2026}";
            let g =
                ui.fonts_mut(|fo| fo.layout_no_wrap(words.to_owned(), regular.clone(), t.color("muted-foreground")));
            let pill = Rect::from_min_size(
                pos2(r.right() + 2.0, r.center().y - g.size().y / 2.0),
                g.size() + vec2(12.0, 0.0),
            );
            let pressed = text_ui.interact(pill, id.with(("placeholder", f.line)), Sense::click());
            pressed.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, &self.words.folded));
            painter.rect_filled(pill, t.radius("control"), t.color("muted"));
            painter.galley(pill.min + vec2(6.0, 0.0), g, t.color("muted-foreground"));
            if pressed.clicked() {
                unfold_clicked = Some(f.line);
            }
        }

        // --- the gutters, opaque over the text scrolled under them
        if gutter > 0.0 {
            let ground = paper.blend(t.color("editor-ground")).blend(t.color("editor-gutter"));
            let column = Rect::from_min_max(visible.left_top(), pos2(visible.left() + gutter, visible.bottom()));
            painter.rect_filled(column, 0.0, ground);
            let problem_lines: Vec<(usize, MarkTone)> = checked
                .ranges
                .iter()
                .filter(|p| p.message.is_some())
                .map(|p| (offsets.line_of(p.from), p.tone))
                .collect();
            for line in 0..offsets.lines() {
                let Some((top, bottom)) = geometry.line_span(&galley, line) else {
                    continue;
                };
                if bottom < visible.top() || top > visible.bottom() || bottom - top < 1.0 {
                    continue;
                }
                let first_row = geometry.lines[line].0;
                let row_bottom = origin.y + galley.rows[first_row].max_y();
                let center = (top + row_bottom) / 2.0;
                if numbered {
                    let ink = if active && Some(line) == caret_line {
                        t.color("foreground")
                    } else {
                        t.color("editor-gutter-ink")
                    };
                    painter.text(
                        pos2(column.left() + numbers_width - 8.0, center),
                        egui::Align2::RIGHT_CENTER,
                        (line + 1).to_string(),
                        regular.clone(),
                        ink,
                    );
                }
                if folding && let Some(f) = st.analysis.folds.iter().find(|f| f.line == line) {
                    let is_folded = st.folded.contains(&f.line);
                    let mark = Rect::from_min_size(
                        pos2(column.left() + numbers_width, top),
                        vec2(fold_width, row_bottom - top),
                    );
                    let pressed = text_ui.interact(mark, id.with(("fold", line)), Sense::click());
                    let words = if is_folded {
                        &self.words.unfold
                    } else {
                        &self.words.fold
                    };
                    pressed.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, words));
                    let ink = if pressed.hovered() {
                        t.color("foreground")
                    } else {
                        t.color("editor-gutter-ink")
                    };
                    let icon = if is_folded {
                        Icon::ChevronRight
                    } else {
                        Icon::ChevronDown
                    };
                    icons::paint(&painter, mark.center(), icon, size, ink);
                    if pressed.clicked() {
                        if is_folded {
                            unfold_clicked = Some(line);
                        } else {
                            st.folded.push(line);
                        }
                    }
                }
                if lint_width > 0.0 {
                    let worst = problem_lines
                        .iter()
                        .filter(|(l, _)| *l == line)
                        .map(|(_, tone)| *tone)
                        .min_by_key(|tone| match tone {
                            MarkTone::Error => 0,
                            MarkTone::Warning => 1,
                            MarkTone::Info => 2,
                        });
                    if let Some(tone) = worst {
                        let x = column.left() + numbers_width + fold_width + lint_width / 2.0;
                        painter.circle_filled(pos2(x, center), 4.0, tone_color(&t, tone));
                    }
                }
            }
        }
        if let Some(line) = unfold_clicked {
            st.folded.retain(|l| *l != line);
        }

        // --- the pointer: hover tooltips, the links followed with Mod held
        let mut followed = None;
        let mut followed_heading = None;
        let pointer = ctx
            .input(|i| i.pointer.hover_pos())
            .filter(|p| visible.contains(*p) && ui.is_rect_visible(visible));
        let resting = ctx.input(|i| i.pointer.time_since_last_movement());
        let mut tips: Vec<Tip> = Vec::new();
        let mut tip_anchor = None;
        if let Some(p) = pointer
            && ctx.layer_id_at(p) == Some(ui.layer_id())
        {
            let in_lint =
                lint_width > 0.0 && p.x >= visible.left() + numbers_width + fold_width && p.x < visible.left() + gutter;
            let char_at = galley.cursor_from_pos(p - origin).index.0;
            let byte = offsets.to_byte(text, char_at);
            let over_glyph = geometry
                .rects(&galley, char_at, char_at + 1)
                .first()
                .is_some_and(|r| r.expand2(vec2(0.0, 0.5)).contains(p))
                && p.x >= visible.left() + gutter;
            if held
                && over_glyph
                && let Some(link) = st.analysis.links.iter().find(|l| l.from <= byte && byte < l.to)
            {
                ctx.set_cursor_icon(CursorIcon::PointingHand);
                if ctx.input(|i| i.pointer.primary_pressed()) {
                    followed = Some(link.target.clone());
                    followed_heading = link.heading.clone();
                }
            }
            if resting >= HOVER_WAIT && st.open.is_none() {
                if in_lint {
                    let line = (0..offsets.lines()).find(|l| {
                        geometry
                            .line_span(&galley, *l)
                            .is_some_and(|(a, b)| a <= p.y && p.y < b)
                    });
                    if let Some(line) = line {
                        for r in checked
                            .ranges
                            .iter()
                            .filter(|r| r.message.is_some() && offsets.line_of(r.from) == line)
                        {
                            tips.push(Tip::Problem(r.tone, r.message.clone().unwrap_or_default()));
                        }
                        if let Some((top, bottom)) = geometry.line_span(&galley, line) {
                            tip_anchor = Some(Rect::from_min_max(pos2(p.x, top), pos2(p.x, bottom)));
                        }
                    }
                } else if over_glyph {
                    for r in checked
                        .ranges
                        .iter()
                        .filter(|r| r.message.is_some() && r.from <= byte && byte < r.to.max(r.from + 1))
                    {
                        tips.push(Tip::Problem(r.tone, r.message.clone().unwrap_or_default()));
                    }
                    if let Some(schema) = self.schema
                        && schema_language
                        && let Some(read) = checked.read.as_ref()
                        && let Some((kind, description, _)) = schema::hover(schema, read, byte)
                    {
                        tips.push(Tip::Detail(kind));
                        if let Some(d) = description {
                            tips.push(Tip::Text(d));
                        }
                    }
                    if let Some(describe) = &self.hover
                        && let Some((a, b)) = edit::word_at(text, byte)
                        && let Some(words) = describe(&text[a..b])
                        && !words.is_empty()
                    {
                        tips.push(Tip::Text(words));
                    }
                    if let Some(r) = geometry.rects(&galley, char_at, char_at + 1).first() {
                        tip_anchor = Some(Rect::from_min_max(pos2(r.left(), r.top()), pos2(r.left(), r.bottom())));
                    }
                }
            } else if (self.hover.is_some() || diagnosing) && resting < HOVER_WAIT {
                ctx.request_repaint_after(std::time::Duration::from_secs_f32(HOVER_WAIT - resting + 0.01));
            }
        }
        if let Some(anchor) = tip_anchor
            && !tips.is_empty()
        {
            popup::tooltip(ui, id.with("tooltip"), anchor, &t, &regular, &tips);
        }

        // --- the words offered
        let edited = *self.text != before && !changed_by_bar;
        if let Some((anchor, head)) = caret {
            let typed_in = edited && head > 0 && self.text.len() > before.len() && anchor == head;
            let sources = Sources {
                language: self.language,
                completions: self.completions,
                complete: self.complete.as_ref(),
                links: if following { self.links } else { None },
                schema: if schema_language { self.schema } else { None },
                words: &self.words,
            };
            let keep_open = st
                .open
                .as_ref()
                .is_some_and(|o| o.from <= head && !text[o.from..head.max(o.from)].contains('\n'));
            if editable && has_focus && (explicit || typed_in || (keep_open && edited)) {
                match offers(text, head, explicit, &sources, &st.analysis.tokens) {
                    Some((from, items)) => {
                        let typed = &text[from..head];
                        let shown = filter(&items, typed);
                        st.open = (!shown.is_empty()).then_some(Open {
                            from,
                            items: shown,
                            selected: 0,
                            explicit,
                        });
                    }
                    None => st.open = None,
                }
            } else if !keep_open || !has_focus || anchor != head {
                st.open = None;
            }
            if let Some(open) = st.open.clone() {
                let typed = text[open.from..head.max(open.from)].to_owned();
                let at_rect = geometry
                    .rects(
                        &galley,
                        offsets.to_char(text, open.from),
                        offsets.to_char(text, open.from),
                    )
                    .into_iter()
                    .next();
                if let Some(r) = at_rect {
                    let offers: Vec<Offer> = open.items.iter().map(|i| i.offer.clone()).collect();
                    let pressed = popup::completion(
                        ui,
                        id.with("completion"),
                        pos2(r.left() - 8.0 - size - 6.0, r.bottom() + 2.0),
                        &t,
                        &regular,
                        &bold,
                        &offers,
                        &typed,
                        open.selected.min(offers.len().saturating_sub(1)),
                    );
                    if let Some(i) = pressed {
                        st.open = None;
                        self.accept(&ctx, text_id, &open, i, &mut st);
                        response.request_focus();
                    }
                    let _ = open.explicit;
                }
            }
        }
        // the snippet ends when the caret leaves its fields
        if let (Some(snippet), Some((_, h))) = (st.snippet.as_ref(), caret) {
            let delta = self.text.len() as isize - snippet.length as isize;
            let inside = snippet
                .fields
                .get(snippet.at)
                .is_some_and(|f| f.0 <= h && h as isize <= f.1 as isize + delta.max(0));
            if !inside {
                st.snippet = None;
            }
        }

        // --- the box's ground and edge
        let focus_within = has_focus
            || ctx.memory(|m| {
                m.focused()
                    .is_some_and(|f| f == id.with("search").with("find") || f == id.with("search").with("replace"))
            });
        let radius = if edged {
            t.radius("control")
        } else {
            egui::CornerRadius::ZERO
        };
        let mut box_shapes = vec![Shape::rect_filled(outer, radius, paper)];
        if edged {
            let edge = match (self.invalid, focus_within) {
                (true, true) => t.color("critical"),
                (true, false) => t.color_at("critical", 0.8),
                (false, true) => t.color_at("ring", 0.8),
                (false, false) => t.color("input"),
            };
            box_shapes.push(Shape::rect_stroke(
                outer,
                radius,
                Stroke::new(1.0, edge),
                StrokeKind::Inside,
            ));
            if self.invalid && focus_within {
                let width = t.length("invalid-ring-width");
                box_shapes.push(Shape::rect_stroke(
                    outer,
                    radius,
                    Stroke::new(width, t.color("invalid-ring")),
                    StrokeKind::Outside,
                ));
            }
        }
        ui.painter().set(box_shape, Shape::Vec(box_shapes));

        let changed = *self.text != before;
        if changed {
            ctx.request_repaint();
        }
        let problems = st.checked.problems.as_ref().clone();
        ctx.data_mut(|d| d.insert_temp(id, st));
        CodeEditorResponse {
            response,
            changed,
            run,
            saved,
            followed,
            followed_heading,
            problems,
        }
    }

    /// Puts in the word chosen from the list: its text in place of what was typed (a JSON
    /// string whole, its quotes included), what it selects selected, a snippet's fields kept.
    fn accept(&mut self, ctx: &egui::Context, text_id: Id, open: &Open, index: usize, st: &mut State) {
        let Some(item) = open.items.get(index) else { return };
        let Some((_, head)) = selection_of(ctx, text_id, self.text) else {
            return;
        };
        let mut start = open.from.min(head);
        let mut end = head;
        if item.quoted {
            start = start.saturating_sub(1);
            if self.text[end..].starts_with('"') {
                end += 1;
            }
        }
        let mut out = self.text.clone();
        out.replace_range(start..end, &item.insert);
        *self.text = out;
        let (a, b) = item.select;
        set_selection(ctx, text_id, self.text, (start + a, start + b));
        st.snippet = (item.fields.len() > 1).then(|| Snippet {
            fields: item.fields.iter().map(|(a, b)| (start + a, start + b)).collect(),
            at: 0,
            length: self.text.len(),
        });
    }

    /// The marks and the schema's findings placed on the text.
    fn check(&self, text: &str, offsets: &Offsets, key: u64) -> Checked {
        let json_or_yaml = matches!(self.language, CodeLanguage::Json | CodeLanguage::Yaml);
        let read = json_or_yaml.then(|| {
            Arc::new(if self.language == CodeLanguage::Json {
                document::read_json(text)
            } else {
                document::read_yaml(text)
            })
        });
        let mut ranges: Vec<Placed> = Vec::new();
        let mut lines = Vec::new();
        for mark in self.marks {
            let at = if let Some(path) = &mark.path {
                read.as_ref().and_then(|r| schema::path_range(r, path))
            } else {
                mark.range.map(|(a, b)| {
                    let a = edit::floor_char(text, a.min(text.len()));
                    (a, edit::floor_char(text, b.min(text.len())).max(a))
                })
            };
            match (at, mark.line) {
                (Some((a, b)), _) => ranges.push(Placed {
                    from: a,
                    to: b,
                    tone: mark.tone,
                    message: mark.message.clone(),
                    underline: true,
                }),
                (None, Some(line)) if mark.path.is_none() && line >= 1 && line <= offsets.lines() => {
                    lines.push((line - 1, mark.tone));
                    if mark.message.is_some() {
                        let from = offsets.line_start(line - 1);
                        ranges.push(Placed {
                            from,
                            to: offsets.line_end(text, line - 1).max(from),
                            tone: mark.tone,
                            message: mark.message.clone(),
                            underline: false,
                        });
                    }
                }
                _ => {}
            }
        }
        if let (Some(schema), Some(read)) = (self.schema, read.as_ref()) {
            for (finding, _) in schema::findings(text, read, schema, self.language.name(), &self.words, self.deferred) {
                ranges.push(Placed {
                    from: finding.from,
                    to: finding.to,
                    tone: finding.tone,
                    message: Some(finding.message),
                    underline: true,
                });
            }
        }
        ranges.sort_by_key(|p| (p.from, p.to));
        let problems: Vec<CodeProblem> = ranges
            .iter()
            .filter_map(|p| {
                p.message.as_ref().map(|m| CodeProblem {
                    line: offsets.line_of(p.from) + 1,
                    from: p.from,
                    to: p.to,
                    tone: p.tone,
                    message: m.clone(),
                })
            })
            .collect();
        let kept = ranges;
        Checked {
            key,
            read,
            ranges: Arc::new(kept),
            lines: Arc::new(lines),
            problems: Arc::new(problems),
        }
    }
}

#[allow(non_snake_case)]
fn CornerRadius_half(t: &Tokens) -> egui::CornerRadius {
    egui::CornerRadius::same((t.length("radius") / 2.0).round() as u8)
}

fn take_ctrl_shift(ui: &Ui, key: Key) -> bool {
    ui.input_mut(|i| {
        let at = i.events.iter().position(|e| {
            matches!(e, Event::Key { key: k, pressed: true, modifiers, .. }
                if *k == key && modifiers.ctrl && modifiers.shift && !modifiers.alt)
        });
        at.map(|at| i.events.remove(at)).is_some()
    })
}

fn take_ctrl_alt(ui: &Ui, key: Key) -> bool {
    ui.input_mut(|i| {
        let at = i.events.iter().position(|e| {
            matches!(e, Event::Key { key: k, pressed: true, modifiers, .. }
                if *k == key && modifiers.ctrl && modifiers.alt && !modifiers.shift)
        });
        at.map(|at| i.events.remove(at)).is_some()
    })
}
