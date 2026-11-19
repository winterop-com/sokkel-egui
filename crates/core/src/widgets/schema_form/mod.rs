//! A form drawn from a JSON Schema, after sokkel-ui's `SchemaForm`, `useSchemaForm` and
//! `MapField`: a field for each property in the schema's order, labeled by its title with the key
//! beside it (or by the key alone), its description as help. Strings, numbers, enums, booleans,
//! nested objects, lists of values, lists of objects, sets of choices, maps of scalars and choices
//! of scalar types each get their own control; anything else is edited as JSON. An unset field
//! with a schema `default` shows it, muted, as what applies.
//!
//! What is wrong is checked against the schema by the core's checker and said the way every form
//! here says it: a colored edge and a mark at the end of the control, which opens the message and
//! a fix when one is plain. A field's issues show once it has been left with a value, or once the
//! form has been submitted, and each goes the moment the value is put right.

mod draw;
mod issues;
mod map;
pub(crate) mod mark;
mod read;

use std::collections::{BTreeMap, BTreeSet};

use egui::{Id, Ui};
use serde_json::Value;

pub use issues::{IssueSaid, IssueTone, SchemaIssue, SchemaIssueKind, say_issue, schema_issues, without_deferred};
pub use map::{MapRow, object_of, repeated_keys, rows_of};
pub use read::{SchemaPath, blank_item, initial_value, pointer, read_scalar, scalar_union, unset_choice};

use self::mark::Shown;
use self::read::{Kind, field_schema, kind_of, path_of, schema_at, title_at, types, value_at};
use super::IssueEntry;

/// How a map row's Remove names the row: by its key, or by its place from 1 while it has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapEntry<'a> {
    Key(&'a str),
    Row(usize),
}

/// The words a schema form says, in English unless an app passes its own.
#[derive(Clone, Debug)]
pub struct SchemaFormWords {
    /// An issue in words, given the field's schema: the message, and what to do when the message
    /// alone does not say it.
    pub say: fn(&SchemaIssue, &Value) -> IssueSaid,
    /// The action that puts in the value a fix offers.
    pub use_fix: fn(&Value) -> String,
    pub add: String,
    pub remove: String,
    /// A select's placeholder while nothing is chosen.
    pub choose: String,
    /// A select's choice that unsets a field without a default.
    pub none: String,
    /// Said at a JSON field whose text does not parse.
    pub not_json: String,
    /// What a list's item is called, given its index: `Item 1`.
    pub item: fn(usize) -> String,
    pub move_up: String,
    pub move_down: String,
    /// What applies when a field with a schema `default` is left empty, for a form drawn with
    /// `fallbacks`.
    pub fallback: fn(&Value) -> String,
    /// Marks the value an unset field shows as its schema `default`, which applies until one is
    /// chosen.
    pub default: String,
    /// A map's column heads.
    pub key: String,
    pub value: String,
    /// A map row's key box and value box, named by the map's label and the row's place from 1.
    pub map_key: fn(&str, usize) -> String,
    pub map_value: fn(&str, usize) -> String,
    /// A map row's Remove, named by the map's label and the row.
    pub remove_entry: fn(&str, MapEntry<'_>) -> String,
    /// The choice that clears a select back to its schema default, given the default as shown.
    pub use_default: fn(&str) -> String,
    /// Said at a map that has a key twice; the second is not taken.
    pub repeated_key: String,
    /// The disclosure of a folded form's other fields, and its words once open.
    pub more: fn(usize) -> String,
    pub fewer: String,
    /// Before a critical issue in a mark's name, as "Error: ".
    pub issue_critical: String,
    /// Before a warning in a mark's name, as "Warning: ".
    pub issue_warning: String,
}

impl Default for SchemaFormWords {
    fn default() -> Self {
        Self {
            say: say_issue,
            use_fix: |fix| match fix {
                Value::Array(_) => "Use the list without the extras".into(),
                Value::String(s) => format!("Use {s}"),
                other => format!("Use {}", serde_json::to_string(other).unwrap_or_default()),
            },
            add: "Add".into(),
            remove: "Remove".into(),
            choose: "Choose".into(),
            none: "None".into(),
            not_json: "Not valid JSON".into(),
            item: |index| format!("Item {}", index + 1),
            move_up: "Move up".into(),
            move_down: "Move down".into(),
            fallback: |value| format!("Default is {}", read::shown(value)),
            default: "default".into(),
            key: "Key".into(),
            value: "Value".into(),
            map_key: |field, row| format!("{field} key {row}"),
            map_value: |field, row| format!("{field} value {row}"),
            remove_entry: |field, row| match row {
                MapEntry::Key(key) => format!("Remove {field} {key}"),
                MapEntry::Row(place) => format!("Remove {field} row {place}"),
            },
            use_default: |value| format!("Default ({value})"),
            repeated_key: "Each key only once".into(),
            more: |count| format!("{count} more {}", if count == 1 { "field" } else { "fields" }),
            fewer: "Fewer fields".into(),
            issue_critical: "Error: ".into(),
            issue_warning: "Warning: ".into(),
        }
    }
}

/// What a server said about a value it refused, at the path it concerns: shown at its field until
/// that field is changed.
#[derive(Clone, Debug, PartialEq)]
pub struct ServerIssue {
    pub path: SchemaPath,
    pub text: String,
    pub detail: Option<String>,
    pub tone: IssueTone,
}

/// An issue the form shows, with its field's title, in the order the fields are drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct ShownIssue {
    pub path: SchemaPath,
    /// The field's title, or its key when the schema gives none.
    pub label: String,
    pub tone: IssueTone,
    pub text: String,
    pub detail: Option<String>,
}

/// What a schema form did this frame and what it has to say.
#[derive(Clone, Debug, Default)]
pub struct SchemaFormResponse {
    /// The value was changed this frame.
    pub changed: bool,
    /// Every issue shown now, in the order the fields are drawn.
    pub issues: Vec<ShownIssue>,
    /// How many shown issues stop the submit.
    pub critical: usize,
    /// How many shown issues are warnings.
    pub warning: usize,
    /// The value has an issue that stops the submit, shown yet or not: a verb can be shut quietly
    /// while it is true.
    pub blocking: bool,
}

impl SchemaFormResponse {
    /// The shown issues as `IssueCount` lists them.
    pub fn entries(&self) -> Vec<IssueEntry> {
        self.issues
            .iter()
            .map(|issue| IssueEntry {
                label: issue.label.clone(),
                text: issue.text.clone(),
            })
            .collect()
    }
}

/// A list's removal or move: each old index's new one, or `None` for taken out.
#[derive(Clone, Debug, PartialEq)]
enum Move {
    Remove { list: String, index: usize },
    Move { list: String, from: usize, to: usize },
}

impl Move {
    fn list(&self) -> &str {
        match self {
            Move::Remove { list, .. } | Move::Move { list, .. } => list,
        }
    }

    fn map(&self, at: usize) -> Option<usize> {
        match *self {
            Move::Remove { index, .. } => match at.cmp(&index) {
                std::cmp::Ordering::Equal => None,
                std::cmp::Ordering::Greater => Some(at - 1),
                std::cmp::Ordering::Less => Some(at),
            },
            Move::Move { from, to, .. } => Some(if at == from {
                to
            } else if from < to && at > from && at <= to {
                at - 1
            } else if from > to && at >= to && at < from {
                at + 1
            } else {
                at
            }),
        }
    }

    /// A place carried to where its item is now, or `None` once its item was taken out.
    fn shift(&self, key: &str) -> Option<String> {
        let list = self.list();
        let Some(rest) = key.strip_prefix(list).and_then(|r| r.strip_prefix('/')) else {
            return Some(key.to_owned());
        };
        let (head, tail) = rest.split_once('/').map_or((rest, None), |(h, t)| (h, Some(t)));
        let Ok(index) = head.parse::<usize>() else {
            return Some(key.to_owned());
        };
        let next = self.map(index)?;
        Some(match tail {
            Some(tail) => format!("{list}/{next}/{tail}"),
            None => format!("{list}/{next}"),
        })
    }
}

/// A map's rows while it is drawn, and the value they were last read from.
#[derive(Clone, Debug, Default)]
struct MapState {
    rows: Vec<MapRow>,
    seen: Option<Value>,
}

/// Everything a form holds between frames beside the value: what has been touched, what is being
/// typed, what the server said, and which issue is open.
#[derive(Clone, Debug, Default)]
struct State {
    started: bool,
    submitted: bool,
    touched: BTreeSet<String>,
    /// The places changed since the server's issues came, whose refusals no longer show.
    changed: BTreeSet<String>,
    /// Text typed into a JSON field or a choice of scalar types, with the value it was typed over.
    drafts: BTreeMap<String, (String, String)>,
    /// Number fields whose typed text is not a number, as of the last frame.
    unread: BTreeSet<String>,
    /// A field's own refusal of what was typed, such as a map's key given twice, as of the last frame.
    held: BTreeMap<String, String>,
    open: Option<String>,
    server_key: String,
    /// The removals and moves made since the server's issues came.
    moves: Vec<Move>,
    /// A key per item of each list, which stays with the item as items are removed and moved.
    row_keys: BTreeMap<String, Vec<u64>>,
    next_key: u64,
    maps: BTreeMap<String, MapState>,
    /// What is being typed into a number field.
    numbers: BTreeMap<String, String>,
    /// A folded form's fields behind the disclosure, by the object's place.
    folded: BTreeMap<String, Vec<String>>,
    unfolded: BTreeSet<String>,
}

/// A form drawn from a JSON Schema over a value the caller holds: each change is written into
/// `value` as it is made, and said by the response's `changed`.
///
/// ```ignore
/// let response = SchemaForm::new("settings", &schema, &mut value)
///     .deferred(&|v| v.as_str().is_some_and(|s| s.contains("${")))
///     .submitted(true)
///     .show(ui);
/// ```
#[must_use = "draw it with `.show(ui)`"]
pub struct SchemaForm<'a> {
    id: Id,
    schema: &'a Value,
    value: &'a mut Value,
    deferred: Option<&'a dyn Fn(&Value) -> bool>,
    submitted: bool,
    words: SchemaFormWords,
    server: &'a [ServerIssue],
    fallbacks: bool,
    fold: bool,
    open: Option<SchemaPath>,
}

impl<'a> SchemaForm<'a> {
    pub fn new(id: impl egui::AsIdSalt, schema: &'a Value, value: &'a mut Value) -> Self {
        Self {
            id: Id::unique(id),
            schema,
            value,
            deferred: None,
            submitted: false,
            words: SchemaFormWords::default(),
            server: &[],
            fallbacks: false,
            fold: false,
            open: None,
        }
    }

    /// Leaves a value unchecked, and anything inside it, such as one holding a reference filled in
    /// later (`${steps.fetch.rows}` where a number goes).
    pub fn deferred(mut self, deferred: &'a dyn Fn(&Value) -> bool) -> Self {
        self.deferred = Some(deferred);
        self
    }

    /// Shows every issue as if submitted once, as the web's `defaultSubmitted`.
    pub fn submitted(mut self, submitted: bool) -> Self {
        self.submitted = submitted;
        self
    }

    pub fn words(mut self, words: SchemaFormWords) -> Self {
        self.words = words;
        self
    }

    /// What a server refused, each shown at its field until that field is changed.
    pub fn issues(mut self, issues: &'a [ServerIssue]) -> Self {
        self.server = issues;
        self
    }

    /// Says a field's schema `default` under it, as what applies when it is left empty.
    pub fn fallbacks(mut self, fallbacks: bool) -> Self {
        self.fallbacks = fallbacks;
        self
    }

    /// Shows the required fields and those already set when the form is first drawn, and puts the
    /// rest behind one disclosure ("3 more fields"), opened by itself when one of them has an issue
    /// to show.
    pub fn fold(mut self, fold: bool) -> Self {
        self.fold = fold;
        self
    }

    /// The field whose issues start open, as the web's `defaultOpen`.
    pub fn open(mut self, path: &[&str]) -> Self {
        self.open = Some(path.iter().map(|p| (*p).to_owned()).collect());
        self
    }

    /// Shows every issue of the form with this id from now on, as the web's `submit`; the caller
    /// reads `blocking` to know whether the value may be sent.
    pub fn submit(ctx: &egui::Context, id: impl egui::AsIdSalt) {
        let key = Id::unique(id).with("schema-form");
        ctx.data_mut(|d| {
            let mut state: State = d.get_temp(key).unwrap_or_default();
            state.submitted = true;
            d.insert_temp(key, state);
        });
    }

    pub fn show(self, ui: &mut Ui) -> SchemaFormResponse {
        let key = self.id.with("schema-form");
        let mut state: State = ui.data(|d| d.get_temp(key)).unwrap_or_default();
        if !state.started {
            state.started = true;
            state.open = self.open.as_deref().map(pointer);
        }
        state.submitted |= self.submitted;
        let server_key = format!("{:?}", self.server);
        if state.server_key != server_key {
            state.server_key = server_key;
            state.changed.clear();
            state.moves.clear();
        }
        let gather =
            |state: &State, value: &Value| gather(self.schema, value, state, self.deferred, &self.words, self.server);
        let places = gather(&state, self.value);
        let shown: BTreeMap<String, Vec<Shown>> = places
            .iter()
            .map(|place| (place.at.clone(), place.shown(&state).to_vec()))
            .collect();
        let before = (state.unread.clone(), state.held.clone());
        let mut cx = draw::Cx {
            id: self.id,
            root: self.schema,
            words: &self.words,
            state: &mut state,
            shown,
            fallbacks: self.fallbacks,
            drawn: Vec::new(),
            unread_now: BTreeSet::new(),
            held_now: BTreeMap::new(),
            drafted_now: BTreeSet::new(),
            fixes: Vec::new(),
            changed: false,
        };
        draw::form(ui, &mut cx, self.value, self.fold);
        for (path, fix) in std::mem::take(&mut cx.fixes) {
            cx.set(self.value, &path, Some(fix));
        }
        let changed = cx.changed;
        let drawn = std::mem::take(&mut cx.drawn);
        let unread = std::mem::take(&mut cx.unread_now);
        let held = std::mem::take(&mut cx.held_now);
        state.unread = unread;
        state.held = held;
        if changed || before != (state.unread.clone(), state.held.clone()) {
            ui.ctx().request_repaint();
        }
        let mut places = gather(&state, self.value);
        let rank = |at: &str| drawn.iter().position(|d| d == at).unwrap_or(usize::MAX);
        places.sort_by_key(|place| rank(&place.at));
        let mut response = SchemaFormResponse {
            changed,
            ..Default::default()
        };
        for place in &places {
            response.blocking |= place.pending.iter().any(|i| i.tone == IssueTone::Critical);
            let path = path_of(&place.at);
            for issue in place.shown(&state) {
                match issue.tone {
                    IssueTone::Critical => response.critical += 1,
                    IssueTone::Warning => response.warning += 1,
                }
                response.issues.push(ShownIssue {
                    path: path.clone(),
                    label: title_at(self.schema, &path),
                    tone: issue.tone,
                    text: issue.text.clone(),
                    detail: issue.detail.clone(),
                });
            }
        }
        ui.data_mut(|d| d.insert_temp(key, state));
        response
    }
}

/// The issues at one place: all it has, and the server's alone, which show before it is touched.
struct Place {
    at: String,
    pending: Vec<Shown>,
    refused: Vec<Shown>,
}

impl Place {
    /// What the place shows now: everything once touched or submitted (and at the root at once),
    /// else only what the server refused.
    fn shown(&self, state: &State) -> &[Shown] {
        if self.at.is_empty() || state.submitted || state.touched.contains(&self.at) {
            &self.pending
        } else {
            &self.refused
        }
    }
}

/// Every place with something to say: the schema's issues not left for later, a number field's
/// unreadable text, a field's own refusal, JSON that does not parse, and the server's refusals.
fn gather(
    schema: &Value,
    value: &Value,
    state: &State,
    deferred: Option<&dyn Fn(&Value) -> bool>,
    words: &SchemaFormWords,
    server: &[ServerIssue],
) -> Vec<Place> {
    let mut all: BTreeMap<String, Vec<Shown>> = BTreeMap::new();
    let draft_at = |at: &str| -> Option<&String> {
        let (text, over) = state.drafts.get(at)?;
        let now = serde_json::to_string(&value_at(value, &path_of(at))).unwrap_or_default();
        (now == *over).then_some(text)
    };
    let found = without_deferred(schema_issues(schema, value), value, deferred);
    for issue in found {
        let at = pointer(&issue.path);
        if issue.kind == SchemaIssueKind::Required
            && (draft_at(&at).is_some_and(|d| !d.trim().is_empty()) || state.unread.contains(&at))
        {
            continue;
        }
        let said = (words.say)(&issue, &issues::issue_schema(schema, &issue));
        all.entry(at).or_default().push(Shown {
            tone: issue.tone,
            text: said.text,
            detail: said.detail,
            fix: issue.fix,
        });
    }
    // text typed into a number field that does not read as one leaves the value as it was, and says so
    for at in &state.unread {
        let field = schema_at(schema, &path_of(at), schema);
        let expected = types(&field).0.first().cloned().unwrap_or_else(|| "number".into());
        let issue = SchemaIssue {
            kind: SchemaIssueKind::Type { expected },
            path: path_of(at),
            tone: IssueTone::Critical,
            fix: None,
        };
        let said = (words.say)(&issue, &field);
        all.entry(at.clone()).or_default().push(Shown {
            tone: IssueTone::Critical,
            text: said.text,
            detail: said.detail,
            fix: None,
        });
    }
    for (at, text) in &state.held {
        all.entry(at.clone()).or_default().push(Shown {
            tone: IssueTone::Critical,
            text: text.clone(),
            detail: None,
            fix: None,
        });
    }
    for at in state.drafts.keys() {
        let Some(text) = draft_at(at) else { continue };
        let field = field_schema(&schema_at(schema, &path_of(at), schema), schema);
        if text.trim().is_empty()
            || kind_of(&field, schema) != Kind::Json
            || serde_json::from_str::<Value>(text).is_ok()
        {
            continue;
        }
        all.entry(at.clone()).or_default().push(Shown {
            tone: IssueTone::Critical,
            text: words.not_json.clone(),
            detail: None,
            fix: None,
        });
    }
    // each refusal at its item's place now, gone with an item that was taken out
    let mut refused: BTreeMap<String, Vec<Shown>> = BTreeMap::new();
    for issue in server {
        let mut at = Some(pointer(&issue.path));
        for one in &state.moves {
            at = at.and_then(|a| one.shift(&a));
        }
        let Some(at) = at else { continue };
        if state.changed.contains(&at) {
            continue;
        }
        refused.entry(at).or_default().push(Shown {
            tone: issue.tone,
            text: issue.text.clone(),
            detail: issue.detail.clone(),
            fix: None,
        });
    }
    let places: BTreeSet<String> = all.keys().chain(refused.keys()).cloned().collect();
    places
        .into_iter()
        .map(|at| {
            let refused = refused.remove(&at).unwrap_or_default();
            let mut pending = refused.clone();
            pending.extend(all.remove(&at).unwrap_or_default());
            // the critical ones first, each tone in the order found
            pending.sort_by_key(|issue| issue.tone != IssueTone::Critical);
            let mut refused = refused;
            refused.sort_by_key(|issue| issue.tone != IssueTone::Critical);
            Place { at, pending, refused }
        })
        .collect()
}
