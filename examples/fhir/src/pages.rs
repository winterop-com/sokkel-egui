//! Each page's words and logic, as the web's `pages/*.tsx` and `pane.tsx` keep them: the overview's
//! counts, chart and latest changes; a type's resources; a resource's JSON, checked, saved as a new
//! version and deleted, with its side panel (problems, versions with a diff, references, vital
//! signs); FHIRPath against a resource or the whole server; CQL libraries in versions; and search
//! by parameters with the searches kept. A page's state is what its React state holds; its actions
//! change it, call the server and hear its answer (`answered`).

use serde::Serialize;
use serde_json::{Value, json};

use crate::app::{App, Page, Toast, Waiting};
use crate::diff::{DiffRow, diff_counts, diff_lines, fold_diff};
use crate::evaluate::{Completion, Evaluation, element_names, evaluate, functions, variables_offered};
use crate::js::{locale_compare, month_short, pretty};
use crate::model::{
    Issue, Part, Severity, concept_text, field, field_str, from_base64, id_of, js_string, last_updated, quantity_text,
    reference_text, to_base64, version_id, version_of,
};
use crate::search::{RESULT_PARAMS, SearchParam, SearchResult, params_of, read_query, search};
use crate::seed::{LIBRARY_BASE, LIBRARY_TYPE, VITALS, template};
use crate::server::{
    Failure, ME, Outcome, SERVER_ADDRESS, SERVER_BASE, SERVER_NAME, SERVER_VERSION, SavedSearch, Stored, Version,
    parse, pointing_at,
};
use crate::types::{TYPE_NAMES, type_def};
use crate::ui::{count, number, on_day, state_of, title_of, when};
use crate::validate::cql_header;

// --- What the pages draw ---------------------------------------------------------------------------

/// A page's header: its title, the code beside it, what it says under it, and its facts as text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Header {
    pub title: String,
    pub code: Option<String>,
    pub description: Option<String>,
    pub meta: Option<String>,
}

/// A button: its words, why it cannot be pressed now, and whether it is busy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ButtonLook {
    pub label: String,
    pub reason: Option<String>,
    pub busy: bool,
}

fn button(label: &str, reason: Option<String>, busy: bool) -> ButtonLook {
    ButtonLook {
        label: label.to_owned(),
        reason,
        busy,
    }
}

/// A mark on an editor's text: the line (from 1) and columns it covers, or the value at a path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Mark {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<Part>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<usize>,
    pub tone: &'static str,
    pub message: String,
}

fn line_mark(line: usize, tone: &'static str, message: impl Into<String>) -> Mark {
    Mark {
        line: Some(line),
        path: None,
        from: None,
        to: None,
        tone,
        message: message.into(),
    }
}

/// An editor: what it is called, its text, its marks and whether it only shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EditorLook {
    pub label: String,
    pub value: String,
    pub marks: Vec<Mark>,
    #[serde(rename = "readOnly")]
    pub read_only: bool,
}

/// A table: its rows' ids, its column heads, each cell as text, and what it says when empty.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Table {
    pub ids: Vec<String>,
    pub headers: Vec<String>,
    pub cells: Vec<Vec<String>>,
    pub empty: Option<String>,
}

/// What the shell's foot says for the page: a note at its start, facts at its end.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Foot {
    pub note: Option<String>,
    pub items: Option<String>,
}

/// A pane's head: its title and its short note.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PaneLook {
    pub title: String,
    pub meta: Option<String>,
}

fn pane(title: impl Into<String>, meta: Option<String>) -> PaneLook {
    PaneLook {
        title: title.into(),
        meta,
    }
}

/// A diff as the diff view draws it: its rows with the far unchanged lines folded, its counts, and
/// what it says when nothing changed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DiffLook {
    pub rows: Vec<DiffRow>,
    pub added: usize,
    pub removed: usize,
    pub unchanged: Option<String>,
}

fn diff_look(before: &str, after: &str) -> DiffLook {
    let lines = diff_lines(before, after);
    let (added, removed) = diff_counts(&lines);
    DiffLook {
        rows: fold_diff(&lines, 3, &[]),
        added,
        removed,
        unchanged: Some("The editor holds this version.".to_owned()),
    }
}

/// A state as its chip is read as text.
fn chip(state: &str) -> String {
    format!("[{state}]")
}

/// The server's findings as marks on the values they are about.
pub fn issue_marks(issues: &[Issue]) -> Vec<Mark> {
    issues
        .iter()
        .map(|issue| {
            let at_path = issue.path.as_ref().filter(|path| !path.is_empty());
            Mark {
                line: at_path.is_none().then_some(1),
                path: at_path.cloned(),
                from: None,
                to: None,
                tone: match issue.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Information => "info",
                },
                message: issue.message.clone(),
            }
        })
        .collect()
}

// --- The pages' state ------------------------------------------------------------------------------

/// The open page's own state, as its React state holds it.
#[derive(Clone, Debug, PartialEq)]
pub enum PageState {
    Overview,
    Resources {
        query: String,
    },
    Resource {
        saving: bool,
        deleting: bool,
    },
    FhirPath {
        ran: Option<(String, String)>,
    },
    CqlLibraries {
        busy: bool,
    },
    CqlEditor {
        saving: bool,
        marks: Vec<Mark>,
        compared: Option<i64>,
    },
    Search(Box<SearchState>),
    NotFound,
}

/// The search page's own state, and its save dialog's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SearchState {
    /// `table` or `bundle`.
    pub view: String,
    pub saving: bool,
    /// The order just changed, held while it is kept.
    pub held: Option<Vec<SavedSearch>>,
    pub label: String,
    pub busy: bool,
}

impl PageState {
    /// A page's state as it mounts.
    pub fn fresh(page: &Page) -> Self {
        match page {
            Page::Overview => PageState::Overview,
            Page::Resources(_) => PageState::Resources { query: String::new() },
            Page::Resource(..) => PageState::Resource {
                saving: false,
                deleting: false,
            },
            Page::FhirPath(_) => PageState::FhirPath { ran: None },
            Page::CqlLibraries => PageState::CqlLibraries { busy: false },
            Page::CqlEditor(_) => PageState::CqlEditor {
                saving: false,
                marks: Vec::new(),
                compared: None,
            },
            Page::Search => PageState::Search(Box::new(SearchState {
                view: "table".to_owned(),
                ..SearchState::default()
            })),
            Page::NotFound => PageState::NotFound,
        }
    }
}

/// The side panel's own state: the version its history compares.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PanelState {
    pub chosen: Option<i64>,
}

/// What a page does once the server answers a call it made.
#[derive(Clone, Debug, PartialEq)]
pub enum Then {
    /// A resource saved (`making` for one created): its key, type and the version it was read at.
    SaveResource {
        key: String,
        kind: String,
        making: bool,
        version: f64,
    },
    DeleteResource {
        key: String,
        kind: String,
    },
    MakeLibrary,
    /// A library saved, published when `status` is `active`.
    SaveLibrary {
        key: String,
        draft_key: String,
        status: Option<String>,
        name: String,
        version: Option<String>,
        line: usize,
    },
    /// The saved searches kept, and what follows: a toast for one removed or saved.
    KeepSearches {
        removed: Option<String>,
        saved: Option<String>,
    },
    /// A call made on the server directly, its answer written down as it comes.
    Record,
}

fn refused_toast(app: &mut App, key: &str, failure: &Failure, verb: &str) {
    match failure {
        Failure::Refused(refused) => {
            app.editing.set_answer(key, refused.issues.clone());
            app.toast(Toast::error(
                format!("Not {verb}"),
                format!("{}: {}", refused.status, refused.message()),
            ));
        }
        Failure::Error(message) => app.toast(Toast::error(format!("Not {verb}"), message.clone())),
    }
}

/// Hands a call's answer to the page that waits on it: what it says and where it goes, and its own
/// state when it is still the page open.
pub fn answered(app: &mut App, waiting: Waiting, outcome: Outcome) {
    let open = waiting.page == app.page_serial;
    match waiting.then {
        Then::Record => app.calls.push(match outcome {
            Outcome::Kept { resource, warnings } => json!({ "ok": { "resource": resource, "warnings": warnings } }),
            Outcome::Done => json!({ "ok": null }),
            Outcome::Failed(Failure::Refused(refused)) => json!({ "failed": {
                "status": refused.status, "message": refused.message(), "issues": refused.issues,
            } }),
            Outcome::Failed(Failure::Error(message)) => {
                json!({ "failed": { "status": null, "message": message, "issues": null } })
            }
        }),
        Then::SaveResource {
            key,
            kind,
            making,
            version,
        } => match outcome {
            Outcome::Kept { resource, warnings } => {
                if making {
                    let id = id_of(&resource);
                    app.editing.set_draft(&key, None);
                    app.editing.set_answer(&key, Vec::new());
                    app.editing.set_answer(&format!("{kind}/{id}"), warnings);
                    app.toast(Toast::success(
                        format!("{kind}/{id} created"),
                        Some(format!(
                            "Saved as version {}.",
                            version_id(&resource).unwrap_or_else(|| "1".to_owned())
                        )),
                    ));
                    app.go(&format!("/resources/{kind}/{id}"));
                } else {
                    app.editing.set_draft(&key, None);
                    app.editing.set_answer(&key, warnings);
                    let kept = version_id(&resource);
                    if kept.as_deref() == Some(crate::js::number_to_string(version).as_str()) {
                        app.toast(Toast::note(
                            "Nothing changed",
                            Some("The server kept the version it had.".to_owned()),
                        ));
                    } else {
                        app.toast(Toast::success(
                            format!("Saved as version {}", kept.unwrap_or_default()),
                            Some(format!("{key} on {SERVER_NAME}")),
                        ));
                    }
                }
                if open && let PageState::Resource { saving, .. } = &mut app.page {
                    *saving = false;
                }
            }
            Outcome::Failed(failure) => {
                if open && let PageState::Resource { saving, .. } = &mut app.page {
                    *saving = false;
                }
                refused_toast(app, &key, &failure, if making { "created" } else { "saved" });
            }
            Outcome::Done => {}
        },
        Then::DeleteResource { key, kind } => {
            if open && let PageState::Resource { saving, deleting } = &mut app.page {
                *saving = false;
                *deleting = false;
            }
            match outcome {
                Outcome::Failed(failure) => refused_toast(app, &key, &failure, "deleted"),
                _ => {
                    app.editing.set_draft(&key, None);
                    app.toast(Toast::success(
                        format!("{key} deleted"),
                        Some("Its versions stay in the history.".to_owned()),
                    ));
                    app.go(&format!("/resources/{kind}"));
                }
            }
        }
        Then::MakeLibrary => {
            if open && let PageState::CqlLibraries { busy } = &mut app.page {
                *busy = false;
            }
            match outcome {
                Outcome::Kept { resource, .. } => app.go(&format!("/cql/{}", id_of(&resource))),
                Outcome::Failed(failure) => app.toast(Toast::error("Not created", failure.message())),
                Outcome::Done => {}
            }
        }
        Then::SaveLibrary {
            key,
            draft_key,
            status,
            name,
            version,
            line,
        } => {
            if open && let PageState::CqlEditor { saving, .. } = &mut app.page {
                *saving = false;
            }
            match outcome {
                Outcome::Kept { resource, .. } => {
                    app.editing.set_draft(&draft_key, None);
                    let title = if status.as_deref() == Some("active") {
                        format!("{name} {} published", version.unwrap_or_default())
                    } else {
                        format!("Saved as version {}", version_id(&resource).unwrap_or_default())
                    };
                    app.toast(Toast::success(title, Some(format!("{key} on {SERVER_NAME}"))));
                }
                Outcome::Failed(Failure::Refused(refused)) => {
                    let marks = issue_marks(&refused.issues)
                        .into_iter()
                        .map(|mark| {
                            if mark.path.is_none() {
                                mark
                            } else {
                                line_mark(line, mark.tone, mark.message)
                            }
                        })
                        .collect();
                    if open && let PageState::CqlEditor { marks: held, .. } = &mut app.page {
                        *held = marks;
                    }
                    app.toast(Toast::error("Not saved", refused.message()));
                }
                Outcome::Failed(Failure::Error(message)) => {
                    app.toast(Toast::error("Not saved", format!("Error: {message}")));
                }
                Outcome::Done => {}
            }
        }
        Then::KeepSearches { removed, saved } => {
            let kept = !matches!(outcome, Outcome::Failed(_));
            if let Outcome::Failed(failure) = &outcome {
                app.toast(Toast::error("Saved searches not kept", failure.message()));
            }
            if open && let PageState::Search(state) = &mut app.page {
                state.held = None;
            }
            if kept && let Some(label) = removed {
                app.toast(Toast::note(format!("{label} removed from Saved"), None));
            }
            if let Some(label) = saved {
                if kept {
                    app.kept.set("search-side", json!("saved"));
                    app.toast(Toast::success(format!("Saved as {label}"), None));
                }
                if open && let PageState::Search(state) = &mut app.page {
                    state.busy = false;
                    if kept {
                        state.label = String::new();
                        state.saving = false;
                    }
                }
            }
        }
    }
}

// --- Overview --------------------------------------------------------------------------------------

/// One bar of the overview's chart: how many of a vital sign were measured in a month.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Measured {
    pub month: String,
    pub at: String,
    pub vital: String,
    pub count: usize,
}

/// The vital signs measured month by month, by measurement, in the order the months come.
pub fn measured(rows: &[Stored]) -> Vec<Measured> {
    let mut out: Vec<Measured> = Vec::new();
    for row in rows.iter().filter(|row| row.kind == "Observation") {
        let at = crate::js::slice_units(
            &field(&row.resource, "effectiveDateTime")
                .filter(|one| !one.is_null())
                .map(js_string)
                .unwrap_or_default(),
            7,
        );
        if at.is_empty() {
            continue;
        }
        let vital = concept_text(field(&row.resource, "code").unwrap_or(&Value::Null));
        match out.iter_mut().find(|one| one.at == at && one.vital == vital) {
            Some(found) => found.count += 1,
            None => {
                let month = at.get(5..7).and_then(|m| m.parse::<i64>().ok()).unwrap_or(1);
                out.push(Measured {
                    month: month_short(month).to_owned(),
                    at,
                    vital,
                    count: 1,
                });
            }
        }
    }
    out.sort_by(|a, b| locale_compare(&a.at, &b.at, false));
    out
}

/// The overview's stats: how many of the four types the server holds.
pub fn overview_stats(rows: &[Stored]) -> Vec<(String, String)> {
    ["Patient", "Observation", "Condition", "Library"]
        .iter()
        .map(|kind| {
            (
                (*kind).to_owned(),
                number(rows.iter().filter(|row| row.kind == *kind).count() as f64),
            )
        })
        .collect()
}

/// The latest ten changes, the newest first.
pub fn recent(versions: &[Version]) -> Vec<Version> {
    let mut sorted = versions.to_vec();
    sorted.sort_by(|left, right| locale_compare(&right.at, &left.at, false));
    sorted.truncate(10);
    sorted
}

pub fn changes_table(versions: &[Version]) -> Table {
    Table {
        ids: versions.iter().map(|one| one.id.clone()).collect(),
        headers: ["Resource", "Version", "Change", "By", "When"]
            .map(str::to_owned)
            .to_vec(),
        cells: versions
            .iter()
            .map(|one| {
                vec![
                    one.key.clone(),
                    one.version_id.to_string(),
                    chip(one.change.key()),
                    one.by.clone(),
                    when(Some(&one.at)),
                ]
            })
            .collect(),
        empty: None,
    }
}

pub fn overview_header() -> Header {
    Header {
        title: "Overview".to_owned(),
        code: None,
        description: Some(format!(
            "The FHIR server at {SERVER_ADDRESS}, {SERVER_VERSION}: what it holds, what was measured, and what changed last."
        )),
        meta: None,
    }
}

pub fn overview_foot(rows: &[Stored]) -> Foot {
    Foot {
        note: Some(format!("{} resources on {SERVER_ADDRESS}", number(rows.len() as f64))),
        items: None,
    }
}

/// Where a stat goes: a type's resources, or the CQL libraries.
pub fn stat_route(kind: &str) -> String {
    if kind == "Library" {
        "/cql".to_owned()
    } else {
        format!("/resources/{kind}")
    }
}

/// Where a change opens: a library's CQL, else the resource.
pub fn change_route(version: &Version) -> String {
    if version.kind == "Library" && version.change != crate::server::Change::Delete {
        format!("/cql/{}", version.key.split('/').nth(1).unwrap_or(""))
    } else {
        format!("/resources/{}", version.key)
    }
}

// --- Resources ---------------------------------------------------------------------------------------

/// A sort value as a column hands it: a number or a text.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SortValue {
    Text(String),
    Number(f64),
}

fn first_city(resource: &Value) -> String {
    field(resource, "address")
        .and_then(|list| list.get(0))
        .and_then(|one| one.get("city"))
        .filter(|one| !one.is_null())
        .map(js_string)
        .unwrap_or_default()
}

fn opt_text(resource: &Value, name: &str) -> String {
    field(resource, name)
        .filter(|one| !one.is_null())
        .map(js_string)
        .unwrap_or_default()
}

fn day_of(resource: &Value, name: &str) -> String {
    on_day(field_str(resource, name))
}

/// The columns a type's listing shows between its title and its last change: each one's id, head
/// and text for a row.
/// A listing's column: its id, its head and its text for a resource.
pub type Column = (&'static str, &'static str, fn(&Value) -> String);

pub fn type_columns(kind: &str) -> Vec<Column> {
    match kind {
        "Patient" => vec![
            ("gender", "Gender", |r| opt_text(r, "gender")),
            ("birthDate", "Born", |r| day_of(r, "birthDate")),
            ("city", "City", first_city),
        ],
        "Practitioner" => vec![("qualification", "Qualification", |r| {
            concept_text(
                field(r, "qualification")
                    .and_then(|list| list.get(0))
                    .and_then(|one| one.get("code"))
                    .unwrap_or(&Value::Null),
            )
        })],
        "Organization" => vec![("city", "City", first_city)],
        "Observation" => vec![
            ("value", "Value", |r| {
                quantity_text(field(r, "valueQuantity").unwrap_or(&Value::Null))
            }),
            ("subject", "Subject", |r| {
                reference_text(field(r, "subject").unwrap_or(&Value::Null))
            }),
            ("effective", "Observed", |r| day_of(r, "effectiveDateTime")),
        ],
        "Condition" => vec![
            ("subject", "Subject", |r| {
                reference_text(field(r, "subject").unwrap_or(&Value::Null))
            }),
            ("onset", "Onset", |r| day_of(r, "onsetDateTime")),
        ],
        "Encounter" => vec![
            ("subject", "Subject", |r| {
                reference_text(field(r, "subject").unwrap_or(&Value::Null))
            }),
            ("class", "Class", |r| {
                concept_text(field(r, "class").unwrap_or(&Value::Null))
            }),
        ],
        "Library" => vec![
            ("name", "Name", |r| opt_text(r, "name")),
            ("version", "Version", |r| opt_text(r, "version")),
        ],
        _ => Vec::new(),
    }
}

/// A type's listing: its rows as the search leaves them, each cell and sort value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResourcesLook {
    pub table: Table,
    #[serde(rename = "sortValues")]
    pub sort_values: Vec<Vec<SortValue>>,
    pub types: Vec<(String, String, usize)>,
    pub foot: Foot,
}

pub fn resources_look(kind: &str, rows: &[Stored], query: &str) -> ResourcesLook {
    let of_type: Vec<&Stored> = rows.iter().filter(|row| row.kind == kind).collect();
    let asked = crate::js::trim(query).to_lowercase();
    let shown: Vec<&Stored> = of_type
        .iter()
        .filter(|row| {
            asked.is_empty()
                || title_of(&row.resource).to_lowercase().contains(&asked)
                || id_of(&row.resource).contains(&asked)
        })
        .copied()
        .collect();
    let has_state = of_type.iter().any(|row| state_of(&row.resource).is_some());
    let columns = type_columns(kind);
    let mut headers = vec![
        if kind == "Patient" || kind == "Practitioner" {
            "Name"
        } else {
            "Title"
        }
        .to_owned(),
        "Id".to_owned(),
    ];
    headers.extend(columns.iter().map(|(_, head, _)| (*head).to_owned()));
    if has_state {
        headers.push("State".to_owned());
    }
    headers.push("Version".to_owned());
    headers.push("Updated".to_owned());
    let mut cells = Vec::new();
    let mut sort_values = Vec::new();
    for row in &shown {
        let resource = &row.resource;
        let mut line = vec![title_of(resource), id_of(resource)];
        let mut sorts = vec![SortValue::Text(title_of(resource)), SortValue::Text(id_of(resource))];
        for (_, _, read) in &columns {
            line.push(read(resource));
            sorts.push(SortValue::Text(read(resource)));
        }
        if has_state {
            let state = state_of(resource);
            line.push(state.as_deref().map(chip).unwrap_or_default());
            sorts.push(SortValue::Text(state.unwrap_or_default()));
        }
        line.push(version_id(resource).unwrap_or_default());
        sorts.push(SortValue::Number(
            version_id(resource).map_or(0.0, |one| crate::js::number_of_text(&one)),
        ));
        line.push(when(last_updated(resource).as_deref()));
        sorts.push(SortValue::Text(last_updated(resource).unwrap_or_default()));
        cells.push(line);
        sort_values.push(sorts);
    }
    let about = type_def(kind).map_or("", |def| def.about);
    ResourcesLook {
        table: Table {
            ids: shown.iter().map(|row| row.id.clone()).collect(),
            headers,
            cells,
            empty: Some(if asked.is_empty() {
                format!("The server holds no {kind} resources. {about}")
            } else {
                format!("No {kind} matches the search.")
            }),
        },
        sort_values,
        types: TYPE_NAMES
            .iter()
            .map(|name| {
                (
                    (*name).to_owned(),
                    (*name).to_owned(),
                    rows.iter().filter(|row| row.kind == *name).count(),
                )
            })
            .collect(),
        foot: Foot {
            note: Some(format!(
                "Showing {} of {}",
                shown.len(),
                count(of_type.len(), kind, None)
            )),
            items: None,
        },
    }
}

pub fn resources_header() -> Header {
    Header {
        title: "Resources".to_owned(),
        code: None,
        description: Some(
            "What the server holds, one type at a time. A resource opens as its JSON, to read, change and save as a new version."
                .to_owned(),
        ),
        meta: None,
    }
}

// --- A resource ----------------------------------------------------------------------------------------

/// A resource's page as it stands: what the editor holds and what the page says of it.
#[derive(Clone, Debug, PartialEq)]
pub struct ResourceView {
    pub key: String,
    pub making: bool,
    pub stored: Option<Stored>,
    pub base: String,
    pub text: String,
    pub version: f64,
    pub dirty: bool,
    pub behind: bool,
    pub missing: bool,
    pub read_at: f64,
}

pub fn resource_view(app: &App, kind: &str, id: &str) -> ResourceView {
    let making = id == "new";
    let key = format!("{kind}/{id}");
    let stored = if making {
        None
    } else {
        app.rows().iter().find(|row| row.id == key).cloned()
    };
    let base = pretty(&if making {
        template(kind)
    } else {
        stored.as_ref().map_or(json!({}), |row| row.resource.clone())
    });
    let draft = app.editing.draft(&key).cloned();
    let text = draft.as_ref().map_or(base.clone(), |one| one.text.clone());
    let version = stored.as_ref().map_or(0.0, |row| version_of(&row.resource));
    let dirty = if making {
        draft.is_some()
    } else {
        draft.as_ref().is_some_and(|one| one.text != base)
    };
    let behind = draft.as_ref().is_some_and(|one| !making && one.read_at != version);
    ResourceView {
        missing: !making && app.backend.resources.loaded() && stored.is_none(),
        read_at: draft.as_ref().map_or(version, |one| one.read_at),
        key,
        making,
        stored,
        base,
        text,
        version,
        dirty,
        behind,
    }
}

/// The resource page's words.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResourceLook {
    pub header: Header,
    pub buttons: Vec<ButtonLook>,
    pub api: Option<String>,
    pub state: Value,
    pub editor: EditorLook,
    pub confirm: Value,
    pub foot: Foot,
}

pub fn resource_look(app: &App, kind: &str, id: &str) -> ResourceLook {
    let view = resource_view(app, kind, id);
    let (saving, deleting) = match &app.page {
        PageState::Resource { saving, deleting } => (*saving, *deleting),
        _ => (false, false),
    };
    let num = crate::js::number_to_string;
    let meta = match &view.stored {
        None => type_def(kind).map(|def| def.about.to_owned()),
        Some(row) => Some(format!(
            "{}Version {}{}",
            state_of(&row.resource).map(|one| chip(&one)).unwrap_or_default(),
            num(view.version),
            when(last_updated(&row.resource).as_deref())
        )),
    };
    let header = Header {
        title: if view.making {
            format!("New {kind}")
        } else {
            view.stored
                .as_ref()
                .map_or(view.key.clone(), |row| title_of(&row.resource))
        },
        code: (!view.making).then(|| view.key.clone()),
        description: None,
        meta,
    };
    let mut buttons = Vec::new();
    if !view.missing {
        if !view.making {
            buttons.push(button("FHIRPath", None, false));
            buttons.push(button("Delete", None, false));
        }
        buttons.push(button(
            "Revert",
            (!view.dirty).then(|| "Nothing to revert.".to_owned()),
            false,
        ));
        buttons.push(button(
            if view.making { "Create" } else { "Save" },
            (!(view.dirty || view.making)).then(|| "Nothing changed.".to_owned()),
            saving,
        ));
    }
    let mut answer = app.editing.answer(&view.key).to_vec();
    if view.behind {
        answer.push(Issue::new(
            Severity::Warning,
            "conflict",
            None,
            format!(
                "This edit started from version {}; the server has version {} now.",
                num(view.read_at),
                num(view.version)
            ),
        ));
    }
    let foot = Foot {
        note: if view.making {
            Some(format!("A new {kind}, not yet on the server"))
        } else {
            view.stored.as_ref().map(|row| {
                format!(
                    "Version {} \u{b7} saved {}",
                    num(view.version),
                    when(last_updated(&row.resource).as_deref())
                )
            })
        },
        items: view.dirty.then(|| "unsaved".to_owned()),
    };
    ResourceLook {
        header,
        buttons,
        api: (!view.missing && !view.making).then(|| format!("/{}", view.key)),
        state: json!({
            "loading": !app.backend.resources.loaded(),
            "error": view.missing.then(|| format!("The server holds no {}. It may have been deleted; its versions stay in the history.", view.key)),
            "status": view.missing.then_some(404),
        }),
        editor: EditorLook {
            label: if view.making {
                format!("New {kind}")
            } else {
                view.key.clone()
            },
            value: view.text.clone(),
            marks: issue_marks(&answer),
            read_only: false,
        },
        confirm: json!({
            "open": deleting,
            "question": format!("Delete {}?", view.key),
            "detail": "The server refuses while other resources point at it. Its versions stay in the history.",
            "busy": saving,
        }),
        foot,
    }
}

/// Types into a resource's editor: a draft from the version shown, none when it is that again.
pub fn resource_edit(app: &mut App, kind: &str, id: &str, next: &str) {
    let view = resource_view(app, kind, id);
    if !view.making && next == view.base {
        app.editing.set_draft(&view.key, None);
    } else {
        let read_at = app.editing.draft(&view.key).map_or(view.version, |one| one.read_at);
        app.editing.set_draft(
            &view.key,
            Some(crate::editing::Draft {
                text: next.to_owned(),
                read_at,
            }),
        );
    }
}

/// Saves a resource's editor: creates it or updates it, the answer told once it comes.
pub fn resource_save(app: &mut App, kind: &str, id: &str) {
    let view = resource_view(app, kind, id);
    let PageState::Resource { saving, .. } = &mut app.page else {
        return;
    };
    if *saving {
        return;
    }
    *saving = true;
    let then = Then::SaveResource {
        key: view.key.clone(),
        kind: kind.to_owned(),
        making: view.making,
        version: view.version,
    };
    let value = match parse(&view.text) {
        Ok(value) => value,
        Err(refused) => {
            let serial = app.page_serial;
            answered(
                app,
                Waiting {
                    ticket: 0,
                    page: serial,
                    then,
                },
                Outcome::Failed(Failure::Refused(refused)),
            );
            return;
        }
    };
    let ticket = if view.making {
        app.backend.create(kind, value)
    } else {
        app.backend.update(kind, id, value, view.read_at)
    };
    app.wait(ticket, then);
}

/// Deletes the resource shown, once asked.
pub fn resource_remove(app: &mut App, kind: &str, id: &str) {
    let key = format!("{kind}/{id}");
    if let PageState::Resource { saving, .. } = &mut app.page {
        *saving = true;
    }
    let ticket = app.backend.remove(&key);
    app.wait(
        ticket,
        Then::DeleteResource {
            key,
            kind: kind.to_owned(),
        },
    );
}

// --- The side panel ----------------------------------------------------------------------------------

/// A resource a panel list names: its key and what it is called.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Link {
    pub key: String,
    pub title: String,
}

/// The references a resource's text makes, each once, in the order they come.
pub fn references_in(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut from = 0;
    while let Some(found) = text[from..].find("\"reference\"").map(|at| at + from) {
        from = found + 1;
        let rest = text[found + 11..].trim_start_matches(crate::js::is_space);
        let Some(rest) = rest.strip_prefix(':') else { continue };
        let rest = rest.trim_start_matches(crate::js::is_space);
        let Some(rest) = rest.strip_prefix('"') else { continue };
        let Some((kind, after)) = rest.split_once('/') else {
            continue;
        };
        let mut letters = kind.chars();
        let kind_ok = letters.next().is_some_and(|c| c.is_ascii_uppercase())
            && kind.len() >= 2
            && letters.all(|c| c.is_ascii_alphabetic());
        let id: String = after
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '.')
            .collect();
        if !kind_ok || id.is_empty() || !after[id.len()..].starts_with('"') {
            continue;
        }
        let key = format!("{kind}/{id}");
        if !out.contains(&key) {
            out.push(key);
        }
    }
    out
}

/// One tab of the side panel: its sections, each with its list, its charts and its diff.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TabLook {
    pub id: &'static str,
    pub label: String,
    pub sections: Vec<SectionLook>,
    pub charts: Vec<ChartLook>,
    pub diff: Option<DiffLook>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SectionLook {
    pub title: String,
    pub table: Option<Table>,
}

/// One vital sign's chart: its rows over the year.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChartLook {
    pub title: String,
    pub subtitle: String,
    pub empty: bool,
    pub rows: Vec<Value>,
    pub label: String,
}

/// The panel beside a resource: the tab it opens on and every tab.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PanelLook {
    pub default: &'static str,
    pub tabs: Vec<TabLook>,
}

/// A resource's versions, the newest first.
pub fn versions_of(versions: &[Version], key: &str) -> Vec<Version> {
    let mut own: Vec<Version> = versions.iter().filter(|one| one.key == key).cloned().collect();
    own.sort_by_key(|one| std::cmp::Reverse(one.version_id));
    own
}

fn link_of(rows: &[Stored], key: &str) -> Link {
    Link {
        key: key.to_owned(),
        title: rows
            .iter()
            .find(|row| row.id == key)
            .map_or("Not on the server".to_owned(), |row| title_of(&row.resource)),
    }
}

fn link_table(links: &[Link], empty: &str) -> Table {
    Table {
        ids: links.iter().map(|one| one.key.clone()).collect(),
        headers: vec!["Resource".to_owned(), "Key".to_owned()],
        cells: links
            .iter()
            .map(|one| vec![one.title.clone(), one.key.clone()])
            .collect(),
        empty: Some(empty.to_owned()),
    }
}

/// The text the panel compares versions with: the draft, else the stored resource.
pub fn panel_current(app: &App, key: &str) -> String {
    match app.editing.draft(key) {
        Some(draft) => draft.text.clone(),
        None => app
            .rows()
            .iter()
            .find(|row| row.id == key)
            .map_or(String::new(), |row| pretty(&row.resource)),
    }
}

pub fn panel_look(app: &App, kind: &str, id: &str) -> PanelLook {
    let key = format!("{kind}/{id}");
    let rows = app.rows();
    let current = panel_current(app, &key);
    let chosen = app.panel.as_ref().and_then(|panel| panel.chosen);
    let mut tabs = vec![TabLook {
        id: "problems",
        label: "Problems".to_owned(),
        sections: vec![SectionLook {
            title: "Problems".to_owned(),
            table: Some(Table {
                ids: Vec::new(),
                headers: vec!["Kind".to_owned(), "Problem".to_owned(), "Where".to_owned()],
                cells: Vec::new(),
                empty: Some(
                    "The resource checks. The server checks it again, references included, as it is saved.".to_owned(),
                ),
            }),
        }],
        charts: Vec::new(),
        diff: None,
    }];
    if id != "new" {
        let versions = versions_of(app.backend.history.seen(), &key);
        let picked = versions
            .iter()
            .find(|one| Some(one.version_id) == chosen)
            .or(versions.get(1))
            .or(versions.first())
            .cloned();
        let mut sections = vec![SectionLook {
            title: "Versions".to_owned(),
            table: Some(Table {
                ids: versions.iter().map(|one| one.id.clone()).collect(),
                headers: vec!["Version".to_owned(), "Change".to_owned(), "By".to_owned()],
                cells: versions
                    .iter()
                    .map(|one| {
                        vec![
                            format!("v{}", one.version_id),
                            chip(one.change.key()),
                            format!("{}, {}", one.by, when(Some(&one.at))),
                        ]
                    })
                    .collect(),
                empty: Some("No versions yet: it is saved as version 1.".to_owned()),
            }),
        }];
        let diff = picked.as_ref().map(|one| {
            sections.push(SectionLook {
                title: format!("Version {} against the editor", one.version_id),
                table: None,
            });
            diff_look(&one.resource.as_ref().map(pretty).unwrap_or_default(), &current)
        });
        tabs.push(TabLook {
            id: "history",
            label: "History".to_owned(),
            sections,
            charts: Vec::new(),
            diff,
        });
        let out: Vec<Link> = references_in(&current).iter().map(|one| link_of(rows, one)).collect();
        let into: Vec<Link> = pointing_at(rows, &key).iter().map(|one| link_of(rows, one)).collect();
        tabs.push(TabLook {
            id: "references",
            label: "References".to_owned(),
            sections: vec![
                SectionLook {
                    title: "Points at".to_owned(),
                    table: Some(link_table(&out, "It points at no other resource.")),
                },
                SectionLook {
                    title: "Pointed at by".to_owned(),
                    table: Some(link_table(&into, "No resource points at it, so it can be deleted.")),
                },
            ],
            charts: Vec::new(),
            diff: None,
        });
        if kind == "Patient" {
            tabs.push(TabLook {
                id: "vitals",
                label: "Vitals".to_owned(),
                sections: Vec::new(),
                charts: vitals(rows, &key),
                diff: None,
            });
        }
    }
    PanelLook {
        default: if kind == "Patient" { "vitals" } else { "problems" },
        tabs,
    }
}

/// A patient's vital signs over the year, one chart for each, from the observations on the server.
pub fn vitals(rows: &[Stored], key: &str) -> Vec<ChartLook> {
    let observations: Vec<&Stored> = rows
        .iter()
        .filter(|row| {
            row.kind == "Observation"
                && field(&row.resource, "subject")
                    .and_then(|one| one.get("reference"))
                    .and_then(Value::as_str)
                    == Some(key)
        })
        .collect();
    VITALS
        .iter()
        .map(|vital| {
            let mut points: Vec<(String, Value)> = observations
                .iter()
                .filter(|row| {
                    field(&row.resource, "code")
                        .and_then(|code| code.get("coding"))
                        .and_then(Value::as_array)
                        .is_some_and(|coding| {
                            coding
                                .iter()
                                .any(|one| one.get("code").and_then(Value::as_str) == Some(vital.code))
                        })
                })
                .map(|row| {
                    let at_value = field(&row.resource, "effectiveDateTime");
                    let at = at_value.filter(|one| !one.is_null()).map(js_string).unwrap_or_default();
                    let value = field(&row.resource, "valueQuantity")
                        .and_then(|one| one.get("value"))
                        .filter(|one| !one.is_null())
                        .cloned()
                        .unwrap_or(json!(0));
                    (
                        at.clone(),
                        json!({ "day": on_day(at_value.and_then(Value::as_str)), "at": at, "value": value }),
                    )
                })
                .collect();
            points.sort_by(|a, b| locale_compare(&a.0, &b.0, false));
            ChartLook {
                title: vital.display.to_owned(),
                subtitle: vital.unit.to_owned(),
                empty: points.is_empty(),
                rows: points.into_iter().map(|(_, row)| row).collect(),
                label: format!("{} over the year", vital.display),
            }
        })
        .collect()
}

// --- FHIRPath ----------------------------------------------------------------------------------------

/// The subject that stands for the whole server: every resource in one collection Bundle.
pub const SERVER_BUNDLE: &str = "bundle";

/// Expressions to start from: what each is called, the subject it is written for and its text.
pub const FHIRPATH_EXAMPLES: [(&str, &str, &str); 6] = [
    (
        "Official name",
        "Patient/pat-001",
        "// The official name, given names first\nname.where(use = 'official')\n  .select(given.join(' ') + ' ' + family)",
    ),
    (
        "Older than 65",
        "Patient/pat-006",
        "// Born more than 65 years ago\nbirthDate < today() - 65 years",
    ),
    (
        "High blood pressure",
        SERVER_BUNDLE,
        "// Who had a systolic pressure of 140 or more\nentry.resource.ofType(Observation)\n  .where(code.coding.exists(system = %loinc and code = '8480-6'))\n  .where(value.ofType(Quantity).value >= 140)\n  .subject.display\n  .distinct()",
    ),
    (
        "Active conditions",
        SERVER_BUNDLE,
        "entry.resource.ofType(Condition)\n  .where(clinicalStatus.coding.code = 'active')\n  .code.text\n  .distinct()",
    ),
    (
        "Mean heart rate",
        SERVER_BUNDLE,
        "/* The mean of every heart rate measured,\n   rounded to one decimal */\nentry.resource.ofType(Observation)\n  .where(code.coding.code = '8867-4')\n  .value.ofType(Quantity).value\n  .avg()\n  .round(1)",
    ),
    (
        "Patients by city",
        SERVER_BUNDLE,
        "entry.resource.ofType(Patient)\n  .where(address.city = 'Troms\u{f8}')\n  .name.family",
    ),
];

const SUBJECT_TYPES: [&str; 5] = ["Patient", "Practitioner", "Organization", "Condition", "Library"];

/// The whole server as one collection Bundle.
pub fn server_bundle(rows: &[Stored]) -> Value {
    json!({
        "resourceType": "Bundle",
        "id": "server",
        "type": "collection",
        "total": rows.len(),
        "entry": rows.iter().map(|row| json!({ "resource": row.resource })).collect::<Vec<_>>(),
    })
}

/// The FHIRPath page's words.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FhirPathLook {
    pub header: Header,
    pub buttons: Vec<ButtonLook>,
    pub examples: Vec<String>,
    pub subject: String,
    pub options: Vec<Value>,
    pub panes: Vec<PaneLook>,
    pub expression: EditorLook,
    pub words: Vec<String>,
    pub result: Value,
    #[serde(rename = "subjectEditor")]
    pub subject_editor: String,
    pub foot: Foot,
}

/// The expression the page evaluates, the subject's key and the subject.
pub fn fhirpath_subject(app: &App, subject_key: Option<&str>) -> (String, Option<Value>) {
    let chosen = app.kept.text("fhirpath-subject", "Patient/pat-001");
    let subject_id = subject_key.map_or(chosen, str::to_owned);
    let rows = app.rows();
    let subject = if !app.backend.resources.loaded() {
        None
    } else if subject_id == SERVER_BUNDLE {
        Some(server_bundle(rows))
    } else {
        rows.iter()
            .find(|row| row.id == subject_id)
            .map(|row| row.resource.clone())
    };
    (subject_id, subject)
}

pub fn fhirpath_look(app: &App, subject_key: Option<&str>) -> FhirPathLook {
    let prefs = app.prefs();
    let expression = app.kept.text("fhirpath-expression", FHIRPATH_EXAMPLES[0].2);
    let (subject_id, subject) = fhirpath_subject(app, subject_key);
    let ran = match &app.page {
        PageState::FhirPath { ran } => ran.clone(),
        _ => None,
    };
    let shown = if prefs.live {
        expression.clone()
    } else {
        match ran {
            Some((text, subject)) if subject == subject_id => text,
            _ => String::new(),
        }
    };
    let result = evaluate(&shown, subject.as_ref(), app.backend.now());
    let rows = app.rows();
    let mut words: Vec<Completion> = functions();
    words.extend(variables_offered());
    let named = match &subject {
        Some(value) if value.get("resourceType").and_then(Value::as_str) == Some("Bundle") => element_names(
            &Value::Array(rows.iter().take(80).map(|row| row.resource.clone()).collect()),
        ),
        Some(value) => element_names(value),
        None => Vec::new(),
    };
    words.extend(named);
    let counted = |n: usize| format!("{} in 0.0 ms", count(n, "item", None));
    let foot = Foot {
        note: Some(match &result {
            Evaluation::Done { value, .. } => counted(value.len()),
            Evaluation::Failed { .. } => "The expression does not evaluate".to_owned(),
            Evaluation::Empty => "Nothing evaluated yet".to_owned(),
        }),
        items: None,
    };
    let mut options = vec![json!({
        "value": SERVER_BUNDLE,
        "label": format!("The whole server, as a Bundle of {}", count(rows.len(), "resource", None)),
    })];
    options.extend(
        rows.iter()
            .filter(|row| SUBJECT_TYPES.contains(&row.kind.as_str()) || row.id == subject_id)
            .map(|row| json!({ "value": row.id, "label": format!("{}: {}", row.kind, title_of(&row.resource)) })),
    );
    let marks = match &result {
        Evaluation::Failed {
            message,
            line: Some(line),
            column: Some(column),
        } => vec![Mark {
            line: Some(*line),
            path: None,
            from: Some(column + 1),
            to: Some(column + 2),
            tone: "error",
            message: message.clone(),
        }],
        Evaluation::Failed { message, .. } => vec![line_mark(1, "error", message.clone())],
        _ => Vec::new(),
    };
    let result_look = match &result {
        Evaluation::Failed { message, line, column } => json!({
            "banner": match (line, column) {
                (Some(line), Some(column)) => format!("Line {line}, column {}: {message}", column + 1),
                _ => message.clone(),
            },
            "tone": "critical",
        }),
        Evaluation::Done { value, .. } => json!({ "editor": {
            "label": "Result", "value": pretty(&Value::Array(value.clone())), "marks": [], "readOnly": true,
        } }),
        Evaluation::Empty => json!({ "editor": { "label": "Result", "value": "", "marks": [], "readOnly": true } }),
    };
    let mut buttons = vec![button("", None, false)];
    if !prefs.live {
        buttons.push(button("Evaluate", None, false));
    }
    FhirPathLook {
        header: Header {
            title: "FHIRPath".to_owned(),
            code: None,
            description: Some(
                "Write an expression and read what it selects, evaluated against a resource on the server or the whole server as one Bundle."
                    .to_owned(),
            ),
            meta: None,
        },
        buttons,
        examples: FHIRPATH_EXAMPLES.iter().map(|one| one.0.to_owned()).collect(),
        subject: subject_id.clone(),
        options,
        panes: vec![
            pane(
                "Expression",
                Some(if prefs.live { "Evaluated as typed" } else { "Mod-Enter evaluates" }.to_owned()),
            ),
            pane(
                "Result",
                match &result {
                    Evaluation::Done { value, .. } => Some(counted(value.len())),
                    Evaluation::Failed { .. } => Some("Refused".to_owned()),
                    Evaluation::Empty => None,
                },
            ),
            pane(
                if subject_id == SERVER_BUNDLE { "Bundle".to_owned() } else { subject_id.clone() },
                subject
                    .as_ref()
                    .and_then(|one| one.get("resourceType"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            ),
        ],
        expression: EditorLook {
            label: "FHIRPath expression".to_owned(),
            value: expression,
            marks,
            read_only: false,
        },
        words: words.into_iter().map(|one| one.label).collect(),
        result: result_look,
        subject_editor: "Subject".to_owned(),
        foot,
    }
}

/// Chooses the subject: on a resource's own address, back to the page's own first.
pub fn fhirpath_choose(app: &mut App, subject_key: Option<&str>, key: &str) {
    if subject_key.is_some() {
        app.go("/fhirpath");
    }
    app.kept.set("fhirpath-subject", json!(key));
}

/// Evaluates the expression as it stands against the subject shown.
pub fn fhirpath_run(app: &mut App, subject_key: Option<&str>) {
    let expression = app.kept.text("fhirpath-expression", FHIRPATH_EXAMPLES[0].2);
    let (subject_id, _) = fhirpath_subject(app, subject_key);
    if let PageState::FhirPath { ran } = &mut app.page {
        *ran = Some((expression, subject_id));
    }
}

/// Takes an example: its expression, its subject, evaluated at once.
pub fn fhirpath_example(app: &mut App, subject_key: Option<&str>, label: &str) {
    let Some((_, subject, expression)) = FHIRPATH_EXAMPLES.iter().find(|one| one.0 == label) else {
        return;
    };
    app.kept.set("fhirpath-expression", json!(expression));
    fhirpath_choose(app, subject_key, subject);
    if let PageState::FhirPath { ran } = &mut app.page {
        *ran = Some(((*expression).to_owned(), (*subject).to_owned()));
    }
}

// --- CQL -----------------------------------------------------------------------------------------------

/// A Library's CQL, or none when it carries none.
pub fn cql_of(resource: Option<&Value>) -> Option<String> {
    let content = resource?.get("content")?.as_array()?;
    let one = content
        .iter()
        .find(|one| one.get("contentType").and_then(Value::as_str) == Some("text/cql"))?;
    let data = one.get("data").filter(|data| !data.is_null())?;
    from_base64(&js_string(data))
}

pub fn libraries_table(rows: &[Stored]) -> Table {
    let libraries: Vec<&Stored> = rows.iter().filter(|row| row.kind == "Library").collect();
    let text = |row: &Stored, names: &[&str]| {
        names
            .iter()
            .find_map(|name| field(&row.resource, name).filter(|one| !one.is_null()))
            .map(js_string)
    };
    Table {
        ids: libraries.iter().map(|row| row.id.clone()).collect(),
        headers: ["Title", "Name", "Version", "State", "Lines", "Updated"]
            .map(str::to_owned)
            .to_vec(),
        cells: libraries
            .iter()
            .map(|row| {
                vec![
                    text(row, &["title", "name", "id"]).unwrap_or_default(),
                    text(row, &["name"]).unwrap_or_default(),
                    text(row, &["version"]).unwrap_or_default(),
                    chip(&text(row, &["status"]).unwrap_or_else(|| "unknown".to_owned())),
                    cql_of(Some(&row.resource))
                        .unwrap_or_default()
                        .split('\n')
                        .count()
                        .to_string(),
                    when(last_updated(&row.resource).as_deref()),
                ]
            })
            .collect(),
        empty: Some("The server holds no libraries. New makes one.".to_owned()),
    }
}

pub fn libraries_header() -> Header {
    Header {
        title: "CQL".to_owned(),
        code: None,
        description: Some(
            "Clinical Quality Language libraries, kept on the server as Library resources. Each save is a new version, and any two can be compared."
                .to_owned(),
        ),
        meta: None,
    }
}

/// Makes a new draft library, under the first `NewLibraryN` no library has.
pub fn make_library(app: &mut App) {
    if let PageState::CqlLibraries { busy } = &mut app.page {
        *busy = true;
    }
    let taken: Vec<String> = app
        .rows()
        .iter()
        .filter(|row| row.kind == "Library")
        .map(|row| {
            field(&row.resource, "name")
                .filter(|one| !one.is_null())
                .map(js_string)
                .unwrap_or_default()
        })
        .collect();
    let mut n = 1;
    while taken.contains(&format!("NewLibrary{n}")) {
        n += 1;
    }
    let name = format!("NewLibrary{n}");
    let cql = format!(
        "library {name} version '0.1.0'\n\nusing FHIR version '4.0.1'\n\ncontext Patient\n\ndefine \"In Population\":\n  true\n"
    );
    let resource = json!({
        "resourceType": "Library",
        "url": format!("{LIBRARY_BASE}/{name}"),
        "version": "0.1.0",
        "name": name,
        "title": format!("New library {n}"),
        "status": "draft",
        "type": { "coding": [{ "system": LIBRARY_TYPE, "code": "logic-library" }] },
        "content": [{ "contentType": "text/cql", "data": to_base64(&cql) }],
    });
    let ticket = app.backend.create("Library", resource);
    app.wait(ticket, Then::MakeLibrary);
}

/// The CQL editor as it stands.
#[derive(Clone, Debug, PartialEq)]
pub struct CqlView {
    pub key: String,
    pub draft_key: String,
    pub stored: Option<Stored>,
    pub base: String,
    pub text: String,
    pub version: f64,
    pub dirty: bool,
    pub own: Vec<Version>,
    pub against: Option<Version>,
    pub header: Option<crate::validate::CqlHeader>,
    pub status: String,
    pub declared: Option<Value>,
    pub frozen: bool,
    pub missing: bool,
    pub read_at: f64,
}

pub fn cql_view(app: &App, id: &str) -> CqlView {
    let key = format!("Library/{id}");
    let draft_key = format!("{key}#cql");
    let stored = app.rows().iter().find(|row| row.id == key).cloned();
    let base = cql_of(stored.as_ref().map(|row| &row.resource)).unwrap_or_default();
    let draft = app.editing.draft(&draft_key).cloned();
    let text = draft.as_ref().map_or(base.clone(), |one| one.text.clone());
    let version = stored.as_ref().map_or(0.0, |row| version_of(&row.resource));
    let dirty = draft.as_ref().is_some_and(|one| one.text != base);
    let own = versions_of(app.backend.history.seen(), &key);
    let compared = match &app.page {
        PageState::CqlEditor { compared, .. } => *compared,
        _ => None,
    };
    let against = own
        .iter()
        .find(|one| Some(one.version_id) == compared)
        .or(own.get(1))
        .or(own.first())
        .cloned();
    let header = cql_header(&text);
    let stored_field = |name: &str| stored.as_ref().and_then(|row| field(&row.resource, name)).cloned();
    let status = stored_field("status")
        .filter(|one| !one.is_null())
        .map(|one| js_string(&one))
        .unwrap_or_default();
    let declared = header.as_ref().and_then(|one| {
        one.version
            .clone()
            .map(Value::String)
            .or_else(|| stored_field("version"))
    });
    let frozen = status == "active" && declared == stored_field("version") && text != base;
    CqlView {
        missing: app.backend.resources.loaded() && stored.is_none(),
        read_at: draft.as_ref().map_or(version, |one| one.read_at),
        key,
        draft_key,
        stored,
        base,
        text,
        version,
        dirty,
        own,
        against,
        header,
        status,
        declared,
        frozen,
    }
}

/// The CQL editor's words.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CqlLook {
    pub header: Header,
    pub buttons: Vec<ButtonLook>,
    pub state: Value,
    pub panes: Vec<PaneLook>,
    pub editor: EditorLook,
    pub versions: Value,
    pub diff: Option<DiffLook>,
    pub foot: Foot,
}

pub fn cql_look(app: &App, id: &str) -> CqlLook {
    let view = cql_view(app, id);
    let (saving, marks) = match &app.page {
        PageState::CqlEditor { saving, marks, .. } => (*saving, marks.clone()),
        _ => (false, Vec::new()),
    };
    let num = crate::js::number_to_string;
    let stored_text = |name: &str| {
        view.stored
            .as_ref()
            .and_then(|row| field(&row.resource, name))
            .filter(|one| !one.is_null())
            .map(js_string)
    };
    let mut live = Vec::new();
    match &view.header {
        None => live.push(line_mark(
            1,
            "error",
            "A library starts by declaring itself: library Name version '1.0.0'.",
        )),
        Some(header) if view.frozen => live.push(line_mark(
            header.line,
            "warning",
            format!(
                "{} is active, so its content is fixed. Raise the version to save a change.",
                stored_text("version").unwrap_or_default()
            ),
        )),
        _ => {}
    }
    live.extend(marks);
    let mut buttons = Vec::new();
    if !view.missing {
        buttons.push(button("Library resource", None, false));
        buttons.push(button(
            "Revert",
            (!view.dirty).then(|| "Nothing to revert.".to_owned()),
            false,
        ));
        if view.status == "draft" {
            buttons.push(button(
                "Publish",
                view.header
                    .is_none()
                    .then(|| "The library does not declare itself.".to_owned()),
                false,
            ));
        }
        buttons.push(button(
            "Save",
            if view.dirty {
                view.frozen
                    .then(|| "An active version is fixed: raise the version first.".to_owned())
            } else {
                Some("Nothing changed.".to_owned())
            },
            saving,
        ));
    }
    let versions_table = Table {
        ids: view.own.iter().map(|one| one.id.clone()).collect(),
        headers: vec!["Version".to_owned(), "State".to_owned(), "Saved".to_owned()],
        cells: view
            .own
            .iter()
            .map(|one| {
                vec![
                    format!("v{}", one.version_id),
                    match &one.resource {
                        None => chip("delete"),
                        Some(resource) => format!(
                            "{} {}",
                            field(resource, "version")
                                .filter(|v| !v.is_null())
                                .map(js_string)
                                .unwrap_or_default(),
                            chip(
                                &field(resource, "status")
                                    .filter(|v| !v.is_null())
                                    .map(js_string)
                                    .unwrap_or_default()
                            )
                        ),
                    },
                    format!("{}, {}", one.by, when(Some(&one.at))),
                ]
            })
            .collect(),
        empty: None,
    };
    let opened: Vec<String> = view
        .own
        .iter()
        .filter(|one| {
            view.against
                .as_ref()
                .is_some_and(|against| against.version_id == one.version_id)
        })
        .map(|one| one.id.clone())
        .collect();
    let mut versions = serde_json::to_value(&versions_table).unwrap_or_default();
    versions["opened"] = json!(opened);
    CqlLook {
        header: Header {
            title: stored_text("title")
                .or_else(|| stored_text("name"))
                .unwrap_or_else(|| view.key.clone()),
            code: view.stored.as_ref().map(|_| {
                format!(
                    "{}|{}",
                    stored_text("name").unwrap_or_default(),
                    stored_text("version").unwrap_or_default()
                )
            }),
            description: None,
            meta: view.stored.as_ref().map(|row| {
                format!(
                    "{}Version {}{}",
                    chip(&view.status),
                    num(view.version),
                    when(last_updated(&row.resource).as_deref())
                )
            }),
        },
        buttons,
        state: json!({
            "loading": !app.backend.resources.loaded(),
            "error": view.missing.then(|| format!("The server holds no {}.", view.key)),
            "status": view.missing.then_some(404),
        }),
        panes: {
            vec![
                pane(
                    format!(
                        "{}.cql",
                        view.header
                            .as_ref()
                            .map_or("Library".to_owned(), |one| one.name.clone())
                    ),
                    Some(if view.dirty { "Unsaved changes" } else { "Saved" }.to_owned()),
                ),
                pane("Versions", Some(count(view.own.len(), "version", None))),
                pane(
                    view.against.as_ref().map_or("Compare".to_owned(), |one| {
                        format!("Version {} against the editor", one.version_id)
                    }),
                    None,
                ),
            ]
        },
        editor: EditorLook {
            label: format!("{} CQL", view.key),
            value: view.text.clone(),
            marks: live,
            read_only: false,
        },
        versions,
        diff: view
            .against
            .as_ref()
            .map(|one| diff_look(&cql_of(one.resource.as_ref()).unwrap_or_default(), &view.text)),
        foot: Foot {
            note: view.stored.as_ref().map(|_| {
                format!(
                    "{} {} \u{b7} saved as version {}",
                    stored_text("name").unwrap_or_default(),
                    stored_text("version").unwrap_or_default(),
                    num(view.version)
                )
            }),
            items: view.dirty.then(|| "unsaved".to_owned()),
        },
    }
}

/// Types into the CQL editor: a draft from the version shown, none when it is that again.
pub fn cql_edit(app: &mut App, id: &str, next: &str) {
    let view = cql_view(app, id);
    if let PageState::CqlEditor { marks, .. } = &mut app.page {
        marks.clear();
    }
    let draft = (next != view.base).then(|| crate::editing::Draft {
        text: next.to_owned(),
        read_at: view.read_at,
    });
    app.editing.set_draft(&view.draft_key, draft);
}

/// Saves the library from its CQL, publishing it when `next_status` is `active`.
pub fn cql_save(app: &mut App, id: &str, next_status: Option<&str>) {
    let view = cql_view(app, id);
    let Some(stored) = view.stored.clone() else { return };
    if matches!(app.page, PageState::CqlEditor { saving: true, .. }) {
        return;
    }
    let Some(header) = view.header.clone() else {
        app.toast(Toast::error(
            "Not saved",
            "The library does not declare itself on its first line.",
        ));
        return;
    };
    let stored_version = field(&stored.resource, "version").cloned();
    if view.frozen && next_status.is_none() {
        app.toast(Toast::error(
            "Not saved",
            format!(
                "Version {} is active, so its content is fixed. Raise the version in the declaration.",
                stored_version
                    .as_ref()
                    .filter(|v| !v.is_null())
                    .map(js_string)
                    .unwrap_or_default()
            ),
        ));
        return;
    }
    let raised = view.declared != stored_version;
    let mut resource = stored.resource.clone();
    if let Some(fields) = resource.as_object_mut() {
        fields.insert("name".to_owned(), json!(header.name));
        if let Some(version) = &header.version {
            fields.insert("version".to_owned(), json!(version));
        }
        fields.insert("url".to_owned(), json!(format!("{LIBRARY_BASE}/{}", header.name)));
        let status = next_status.map_or_else(
            || {
                if raised && view.status == "active" {
                    "draft".to_owned()
                } else {
                    view.status.clone()
                }
            },
            str::to_owned,
        );
        fields.insert("status".to_owned(), json!(status));
        fields.insert("date".to_owned(), json!(crate::js::iso_instant(app.backend.now())));
        fields.insert(
            "content".to_owned(),
            json!([{ "contentType": "text/cql", "data": to_base64(&view.text) }]),
        );
    }
    if let PageState::CqlEditor { saving, .. } = &mut app.page {
        *saving = true;
    }
    let ticket = app.backend.update("Library", id, resource, view.read_at);
    app.wait(
        ticket,
        Then::SaveLibrary {
            key: view.key,
            draft_key: view.draft_key,
            status: next_status.map(str::to_owned),
            name: header.name,
            version: header.version,
            line: header.line,
        },
    );
}

// --- Search ----------------------------------------------------------------------------------------

/// Queries to start from: what each is called and its text.
pub const SEARCH_EXAMPLES: [(&str, &str); 8] = [
    (
        "Women born before 1970",
        "Patient?gender=female&birthdate=lt1970-01-01&_sort=birthdate",
    ),
    ("Patients in Troms\u{f8}", "Patient?address-city=Troms&_sort=family"),
    (
        "High systolic pressure",
        "Observation?code=http://loinc.org|8480-6&value-quantity=ge150&_sort=-date",
    ),
    (
        "One patient\u{2019}s weights",
        "Observation?patient=Patient/pat-002&code=29463-7&_sort=date",
    ),
    ("Active hypertension", "Condition?code=38341003&clinical-status=active"),
    ("Visits this fall", "Encounter?date=ge2026-09-01&_count=10"),
    ("Draft libraries", "Library?status=draft"),
    ("How many observations", "Observation?_summary=count"),
];

/// A query as it is run: on one line, without the spaces at its ends.
fn one_line(text: &str) -> String {
    crate::js::trim(&text.replace('\n', "")).to_owned()
}

pub fn search_query(app: &App) -> String {
    app.kept.text("search-query", SEARCH_EXAMPLES[0].1)
}

pub fn search_ran(app: &App) -> String {
    app.kept.text("search-ran", SEARCH_EXAMPLES[0].1)
}

pub fn search_side(app: &App) -> String {
    app.kept.text("search-side", "saved")
}

/// The searches the person kept: the order held while it is kept, else the saved one.
pub fn searches(app: &App) -> Vec<SavedSearch> {
    if let PageState::Search(state) = &app.page
        && let Some(held) = &state.held
    {
        return held.clone();
    }
    app.backend
        .saved
        .seen()
        .iter()
        .find(|row| row.id == ME)
        .map(|row| row.searches.clone())
        .unwrap_or_default()
}

pub fn search_result(app: &App) -> Option<SearchResult> {
    app.backend
        .resources
        .loaded()
        .then(|| search(app.rows(), &search_ran(app), app.prefs().page_size))
}

/// Runs a query: kept as the one run.
pub fn search_run(app: &mut App, text: Option<&str>) {
    let text = text.map_or_else(|| search_query(app), str::to_owned);
    app.kept.set("search-ran", json!(one_line(&text)));
}

/// Writes and runs a query.
pub fn search_set(app: &mut App, text: &str) {
    app.kept.set("search-query", json!(text));
    search_run(app, Some(text));
}

/// Goes to the page of matches starting at `offset`.
pub fn search_page(app: &mut App, offset: f64) {
    let Some(result) = search_result(app) else { return };
    let (_, params) = read_query(&result.query);
    let mut next: Vec<(String, String)> = params.into_iter().filter(|(name, _)| name != "_offset").collect();
    if offset != 0.0 {
        next.push(("_offset".to_owned(), crate::js::number_to_string(offset)));
    }
    let text = if next.is_empty() {
        result.kind.clone()
    } else {
        format!(
            "{}?{}",
            result.kind,
            next.iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&")
        )
    };
    search_set(app, &text);
}

/// Adds a parameter to the query, after a `?` or an `&`.
pub fn search_add_param(app: &mut App, name: &str) {
    let query = search_query(app);
    let base = query
        .strip_suffix('?')
        .or_else(|| query.strip_suffix('&'))
        .unwrap_or(&query)
        .to_owned();
    let joint = if base.contains('?') { '&' } else { '?' };
    app.kept.set("search-query", json!(format!("{base}{joint}{name}=")));
}

/// Keeps the saved searches in a new order or set, held while they are kept.
pub fn search_keep(app: &mut App, next: Vec<SavedSearch>, then: Then) {
    if let PageState::Search(state) = &mut app.page {
        state.held = Some(next.clone());
    }
    let ticket = app.backend.keep_searches(next);
    app.wait(ticket, then);
}

pub fn search_remove(app: &mut App, id: &str) {
    let all = searches(app);
    let label = all.iter().find(|one| one.id == id).map(|one| one.label.clone());
    let next = all.into_iter().filter(|one| one.id != id).collect();
    search_keep(
        app,
        next,
        Then::KeepSearches {
            removed: label,
            saved: None,
        },
    );
}

pub fn search_reorder(app: &mut App, from: usize, to: usize) {
    let mut next = searches(app);
    sokkel_egui::move_item(&mut next, from, to);
    search_keep(
        app,
        next,
        Then::KeepSearches {
            removed: None,
            saved: None,
        },
    );
}

/// Saves the query under the dialog's label, at the end of Saved.
pub fn search_save(app: &mut App) {
    let label = match &app.page {
        PageState::Search(state) => crate::js::trim(&state.label).to_owned(),
        _ => return,
    };
    if label.is_empty() {
        return;
    }
    if let PageState::Search(state) = &mut app.page {
        state.busy = true;
    }
    let mut next = searches(app);
    next.push(SavedSearch {
        id: format!("saved-{}", crate::js::base36(app.backend.now())),
        label: label.clone(),
        query: one_line(&search_query(app)),
    });
    search_keep(
        app,
        next,
        Then::KeepSearches {
            removed: None,
            saved: Some(label),
        },
    );
}

fn stored_table(rows: &[Stored], result: Option<&SearchResult>) -> Table {
    Table {
        ids: rows.iter().map(|row| row.id.clone()).collect(),
        headers: ["Resource", "Key", "State", "Updated"].map(str::to_owned).to_vec(),
        cells: rows
            .iter()
            .map(|row| {
                vec![
                    title_of(&row.resource),
                    row.id.clone(),
                    state_of(&row.resource).map(|one| chip(&one)).unwrap_or_default(),
                    when(last_updated(&row.resource).as_deref()),
                ]
            })
            .collect(),
        empty: Some(match result {
            Some(result) if result.total > 0 => {
                format!(
                    "{} match; _summary=count lists none.",
                    count(result.total, "resource", None)
                )
            }
            _ => "Nothing matches the query.".to_owned(),
        }),
    }
}

fn params_table(params: &[SearchParam]) -> Table {
    Table {
        ids: params.iter().map(|one| one.name.to_owned()).collect(),
        headers: ["Parameter", "Type", "What it matches", "FHIRPath"]
            .map(str::to_owned)
            .to_vec(),
        cells: params
            .iter()
            .map(|one| {
                vec![
                    one.name.to_owned(),
                    one.kind.key().to_owned(),
                    one.doc.to_owned(),
                    one.expression.to_owned(),
                ]
            })
            .collect(),
        empty: Some("Name a resource type to see its parameters.".to_owned()),
    }
}

/// The search page's words.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SearchLook {
    pub header: Header,
    pub buttons: Vec<ButtonLook>,
    pub api: Option<String>,
    pub examples: Vec<String>,
    pub query: EditorLook,
    pub banner: Option<Value>,
    pub panes: Vec<PaneLook>,
    pub segments: Vec<String>,
    pub tables: Vec<Table>,
    pub bundle: Option<String>,
    pub dialog: Value,
    pub foot: Foot,
}

pub fn search_look(app: &App) -> SearchLook {
    let state = match &app.page {
        PageState::Search(state) => (**state).clone(),
        _ => SearchState::default(),
    };
    let query = search_query(app);
    let side = search_side(app);
    let saved = searches(app);
    let result = search_result(app);
    let kind = read_query(&query).0;
    let known = TYPE_NAMES.contains(&kind.as_str());
    let already = saved.iter().find(|one| one.query == one_line(&query));
    let mut buttons = vec![
        button(
            "Save",
            match already {
                Some(one) => Some(format!("Saved as {}.", one.label)),
                None if crate::js::trim(&query).is_empty() => Some("Write a query first.".to_owned()),
                None => None,
            },
            false,
        ),
        button("", None, false),
        button("Search", None, false),
    ];
    let (offset, page_count, total) = result
        .as_ref()
        .map_or((0.0, 0.0, 0), |one| (one.offset, one.count, one.total));
    buttons.push(button(
        "Previous page",
        (result.is_none() || offset == 0.0).then(|| "This is the first page.".to_owned()),
        false,
    ));
    buttons.push(button(
        "Next page",
        (result.is_none() || page_count == 0.0 || offset + page_count >= total as f64)
            .then(|| "This is the last page.".to_owned()),
        false,
    ));
    let mut tables = Vec::new();
    let mut bundle = None;
    if state.view == "table" {
        tables.push(stored_table(
            result.as_ref().map_or(&[][..], |one| &one.matches),
            result.as_ref(),
        ));
    } else {
        bundle = Some(result.as_ref().map_or(String::new(), |one| pretty(&one.bundle)));
    }
    if side == "saved" {
        tables.push(Table {
            ids: saved.iter().map(|one| one.id.clone()).collect(),
            headers: vec!["Name".to_owned(), "Query".to_owned()],
            cells: saved.iter().map(|one| vec![one.label.clone(), one.query.clone()]).collect(),
            empty: Some("No search is saved. Save keeps the query above here, to run again; drag a saved search by its grip to order them.".to_owned()),
        });
    } else {
        let mut params = if known { params_of(&kind) } else { Vec::new() };
        if known {
            params.extend(RESULT_PARAMS);
        }
        tables.push(params_table(&params));
    }
    let foot = Foot {
        note: result.as_ref().map(|one| {
            let shown = one.matches.len();
            let mut note = count(one.total, "match", Some("matches"));
            if shown < one.total && shown > 0 {
                note.push_str(&format!(
                    ", {} to {} shown",
                    crate::js::number_to_string(one.offset + 1.0),
                    crate::js::number_to_string(one.offset + shown as f64)
                ));
            }
            note
        }),
        items: None,
    };
    SearchLook {
        header: Header {
            title: "Search".to_owned(),
            code: None,
            description: Some(
                "Search the server with FHIR search parameters: a resource type, then its parameters, as in Patient?gender=female&birthdate=lt1970. Enter runs the query."
                    .to_owned(),
            ),
            meta: None,
        },
        buttons,
        api: result.as_ref().map(|one| format!("/{}", one.query)),
        examples: SEARCH_EXAMPLES.iter().map(|one| one.0.to_owned()).collect(),
        query: EditorLook {
            label: "Query".to_owned(),
            value: query.clone(),
            marks: Vec::new(),
            read_only: false,
        },
        banner: result.as_ref().filter(|one| !one.issues.is_empty()).map(|one| {
            json!({
                "text": one.issues.iter().map(|issue| issue.message.clone()).collect::<Vec<_>>().join(" "),
                "tone": if one.issues.iter().any(|issue| issue.severity == Severity::Error) { "critical" } else { "warning" },
            })
        }),
        panes: vec![
            pane(
                result
                    .as_ref()
                    .map_or("Matches".to_owned(), |one| format!("{} matches", one.kind)),
                result.as_ref().map(|one| count(one.total, "match", Some("matches"))),
            ),
            pane(
                if side == "saved" {
                    "Saved".to_owned()
                } else if known {
                    format!("{kind} parameters")
                } else {
                    "Parameters".to_owned()
                },
                (side == "saved").then(|| saved.len().to_string()),
            ),
        ],
        segments: vec![state.view.clone(), side.clone()],
        tables,
        bundle,
        dialog: json!({
            "open": state.saving,
            "query": query,
            "label": state.label,
            "why": crate::js::trim(&state.label).is_empty().then_some("Name the search first."),
            "busy": state.busy,
        }),
        foot,
    }
}

/// The whole address an API link copies, for a path on the server.
pub fn api_address(path: &str) -> String {
    format!("{SERVER_BASE}{path}")
}
