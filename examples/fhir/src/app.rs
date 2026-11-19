//! The workbench's frame and its state, as the web's `App.tsx` keeps them: the rail's places with
//! their counts, the trail, the page a route opens, the palette's places and actions, the palette's
//! search over the server's resources, and the Settings the editors and the tools read. `App` holds
//! what the web keeps across pages (the server, the edits, the values kept in the browser, the
//! notes shown and the routes gone to) and the open page's own state; each page's words and
//! actions are in `pages`.

use serde::Serialize;
use serde_json::Value;

use crate::editing::Editing;
use crate::js::trim;
use crate::pages::{PageState, PanelState};
use crate::server::{Backend, Settled, Stored};
use crate::types::TYPE_NAMES;
use crate::ui::{Prefs, title_of};

/// The places the rail names, by their route.
pub fn title_of_place(place: &str) -> Option<&'static str> {
    Some(match place {
        "overview" => "Overview",
        "resources" => "Resources",
        "fhirpath" => "FHIRPath",
        "cql" => "CQL",
        "search" => "Search",
        _ => return None,
    })
}

/// A route as its parts, as the web's router reads the hash: `#/a/b` is `a` and `b`.
pub fn route_of(hash: &str) -> Vec<String> {
    let rest = hash.strip_prefix('#').unwrap_or(hash);
    let rest = rest.strip_prefix('/').unwrap_or(rest);
    rest.split('/')
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Where a route is: its place, overview for one the app has no page for, and whether it has none.
pub fn place_of(route: &[String]) -> (&'static str, bool) {
    let first = route.first().map_or("", String::as_str);
    match first {
        "resources" => ("resources", false),
        "fhirpath" => ("fhirpath", false),
        "cql" => ("cql", false),
        "search" => ("search", false),
        "overview" => ("overview", false),
        "" => ("overview", false),
        _ => ("overview", true),
    }
}

/// The resource type a route shows, on the resources' pages: the one it names, else Patient.
pub fn resource_type_of(route: &[String]) -> Option<String> {
    let (place, _) = place_of(route);
    if place != "resources" {
        return None;
    }
    Some(
        route
            .get(1)
            .filter(|second| TYPE_NAMES.contains(&second.as_str()))
            .cloned()
            .unwrap_or_else(|| "Patient".to_owned()),
    )
}

/// The page a route opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Overview,
    Resources(String),
    Resource(String, String),
    FhirPath(Option<String>),
    CqlLibraries,
    CqlEditor(String),
    Search,
    NotFound,
}

impl Page {
    /// The web component's name.
    pub fn name(&self) -> &'static str {
        match self {
            Page::Overview => "Overview",
            Page::Resources(_) => "Resources",
            Page::Resource(..) => "ResourcePage",
            Page::FhirPath(_) => "FhirPathPage",
            Page::CqlLibraries => "CqlLibraries",
            Page::CqlEditor(_) => "CqlEditor",
            Page::Search => "SearchPage",
            Page::NotFound => "NotFound",
        }
    }

    /// The key the web draws it under, which mounts it afresh when it changes.
    pub fn key(&self) -> String {
        match self {
            Page::Resource(kind, id) => format!("{kind}/{id}"),
            Page::CqlEditor(id) => id.clone(),
            _ => "null".to_owned(),
        }
    }
}

/// The page a route opens.
pub fn page_of(route: &[String]) -> Page {
    let (place, missing) = place_of(route);
    let second = route.get(1).cloned();
    let third = route.get(2).cloned();
    match place {
        "resources" => {
            let kind = resource_type_of(route).unwrap_or_default();
            match third {
                Some(id) => Page::Resource(kind, id),
                None => Page::Resources(kind),
            }
        }
        "fhirpath" => Page::FhirPath(match (second, third) {
            (Some(kind), Some(id)) => Some(format!("{kind}/{id}")),
            _ => None,
        }),
        "cql" => second.map_or(Page::CqlLibraries, Page::CqlEditor),
        "search" => Page::Search,
        _ if missing => Page::NotFound,
        _ => Page::Overview,
    }
}

/// The key the shell mounts the page's content afresh under: the route's first three parts.
pub fn content_key(route: &[String]) -> String {
    route.iter().take(3).cloned().collect::<Vec<_>>().join("/")
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RailChild {
    pub id: String,
    pub label: String,
    pub href: String,
    pub active: bool,
    pub count: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RailEntry {
    pub id: &'static str,
    pub label: &'static str,
    pub description: Option<&'static str>,
    /// The lucide icon the web draws.
    pub icon: &'static str,
    pub href: String,
    pub active: bool,
    pub count: Option<usize>,
    pub keywords: Vec<&'static str>,
    pub entries: Vec<RailChild>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RailGroup {
    pub id: &'static str,
    pub title: Option<&'static str>,
    pub entries: Vec<RailEntry>,
}

/// How many resources of each type the server holds, as the rail counts them.
pub fn counts(rows: &[Stored]) -> Vec<(&'static str, usize)> {
    TYPE_NAMES
        .iter()
        .map(|name| (*name, rows.iter().filter(|row| row.kind == *name).count()))
        .collect()
}

/// The rail's groups, the place shown marked and each type counted.
pub fn rail_groups(place: &str, kind: Option<&str>, counted: &[(&'static str, usize)]) -> Vec<RailGroup> {
    let count = |name: &str| counted.iter().find(|(one, _)| *one == name).map(|(_, n)| *n);
    let entry = |id: &'static str, label, description, icon, href: &str| RailEntry {
        id,
        label,
        description: Some(description),
        icon,
        href: href.to_owned(),
        active: place == id,
        count: None,
        keywords: Vec::new(),
        entries: Vec::new(),
    };
    let mut resources = entry(
        "resources",
        "Resources",
        "Read and edit as JSON",
        "Database",
        "#/resources/Patient",
    );
    resources.active = place == "resources" && kind.is_none();
    resources.entries = TYPE_NAMES
        .iter()
        .map(|name| RailChild {
            id: format!("resources-{name}"),
            label: (*name).to_owned(),
            href: format!("#/resources/{name}"),
            active: place == "resources" && kind == Some(*name),
            count: count(name),
        })
        .collect();
    let mut cql = entry("cql", "CQL", "Libraries in versions", "FileCodeCorner", "#/cql");
    cql.count = count("Library");
    let mut search = entry("search", "Search", "By search parameters", "Search", "#/search");
    search.keywords = vec!["query", "find"];
    vec![
        RailGroup {
            id: "server",
            title: None,
            entries: vec![
                entry("overview", "Overview", "What the server holds", "LayoutDashboard", "#/"),
                resources,
            ],
        },
        RailGroup {
            id: "tools",
            title: Some("Tools"),
            entries: vec![
                entry("fhirpath", "FHIRPath", "Evaluate expressions", "Braces", "#/fhirpath"),
                cql,
                search,
            ],
        },
    ]
}

/// A command the palette offers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Command {
    pub id: String,
    pub label: String,
    pub hint: Option<String>,
    pub verb: Option<String>,
    pub keywords: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommandGroup {
    pub title: String,
    pub verb: Option<String>,
    pub commands: Vec<Command>,
}

/// The palette's own groups: the places (as sokkel-ui's `placeCommands` makes them, a place's
/// own entries after it) and the actions.
pub fn palette(groups: &[RailGroup]) -> Vec<CommandGroup> {
    let mut places = Vec::new();
    for group in groups {
        for entry in &group.entries {
            places.push(Command {
                id: format!("place-{}", entry.id),
                label: entry.label.to_owned(),
                hint: entry.description.map(str::to_owned),
                verb: None,
                keywords: entry.keywords.iter().map(|one| (*one).to_owned()).collect(),
            });
            for child in &entry.entries {
                places.push(Command {
                    id: format!("place-{}", child.id),
                    label: child.label.clone(),
                    hint: Some(entry.label.to_owned()),
                    verb: None,
                    keywords: Vec::new(),
                });
            }
        }
    }
    let mut actions: Vec<Command> = TYPE_NAMES
        .iter()
        .map(|name| Command {
            id: format!("new-{name}"),
            label: format!("New {name}"),
            hint: None,
            verb: Some("Make".to_owned()),
            keywords: Vec::new(),
        })
        .collect();
    actions.push(Command {
        id: "settings".to_owned(),
        label: "Settings".to_owned(),
        hint: None,
        verb: None,
        keywords: vec!["preferences".to_owned(), "theme".to_owned()],
    });
    vec![
        CommandGroup {
            title: "Pages".to_owned(),
            verb: None,
            commands: places,
        },
        CommandGroup {
            title: "Actions".to_owned(),
            verb: Some("Open".to_owned()),
            commands: actions,
        },
    ]
}

/// Where a palette action goes: a new resource's page; none for Settings.
pub fn command_route(id: &str) -> Option<String> {
    if let Some(name) = id.strip_prefix("new-") {
        return Some(format!("/resources/{name}/new"));
    }
    None
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub meta: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SearchGroup {
    pub kind: &'static str,
    pub title: &'static str,
    pub results: Vec<SearchResult>,
}

/// What the palette finds for a query: up to six of each type whose title or id holds it.
pub fn search(query: &str, rows: &[Stored]) -> Vec<SearchGroup> {
    let asked = trim(query).to_lowercase();
    TYPE_NAMES
        .iter()
        .map(|name| SearchGroup {
            kind: name,
            title: name,
            results: rows
                .iter()
                .filter(|row| {
                    row.kind == *name
                        && (title_of(&row.resource).to_lowercase().contains(&asked)
                            || crate::model::id_of(&row.resource).contains(&asked))
                })
                .take(6)
                .map(|row| SearchResult {
                    id: row.id.clone(),
                    title: title_of(&row.resource),
                    meta: row.id.clone(),
                })
                .collect(),
        })
        .collect()
}

/// The palette searches once a query has this many characters.
pub const SEARCH_MIN_QUERY: usize = 2;

/// A step of the top bar's trail.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Crumb {
    pub label: String,
    pub href: Option<String>,
    pub code: bool,
}

/// The top bar's trail: the server, the place, and the type and resource or library opened.
pub fn trail(route: &[String]) -> Vec<Crumb> {
    let (place, missing) = place_of(route);
    let crumb = |label: &str, href: Option<String>, code| Crumb {
        label: label.to_owned(),
        href,
        code,
    };
    let mut out = vec![
        crumb(crate::server::SERVER_NAME, Some("#/".to_owned()), true),
        crumb(
            if missing {
                "Not found"
            } else {
                title_of_place(place).unwrap_or("")
            },
            None,
            false,
        ),
    ];
    if let Some(kind) = resource_type_of(route) {
        let third = route.get(2);
        out[1] = crumb("Resources", Some("#/resources/Patient".to_owned()), false);
        out.push(crumb(&kind, third.map(|_| format!("#/resources/{kind}")), false));
        if let Some(third) = third {
            out.push(crumb(if third == "new" { "New" } else { third }, None, third != "new"));
        }
    }
    if place == "cql"
        && let Some(second) = route.get(1)
    {
        out[1] = crumb("CQL", Some("#/cql".to_owned()), false);
        out.push(crumb(second, None, true));
    }
    out
}

/// The app's name, as the rail and the palette's foot write it.
pub const NAME: &str = "FHIR Workbench";

/// What the palette's field asks for.
pub const PALETTE_PLACEHOLDER: &str = "Go to a page, find a resource";

/// The shortcuts the app lists: what each does and its keys.
pub const SHORTCUTS: [(&str, &str, &[&str]); 3] = [
    ("save", "Save the resource or library", &["Mod", "S"]),
    ("evaluate", "Evaluate the expression", &["Mod", "Enter"]),
    ("search", "Run the search", &["Enter"]),
];

/// A note the app shows for a moment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Toast {
    /// `success`, or `error` for a failure; none for a plain note.
    #[serde(rename = "type")]
    pub kind: Option<&'static str>,
    pub title: String,
    pub description: Option<String>,
}

impl Toast {
    pub fn note(title: impl Into<String>, description: Option<String>) -> Self {
        Toast {
            kind: None,
            title: title.into(),
            description,
        }
    }

    pub fn success(title: impl Into<String>, description: Option<String>) -> Self {
        Toast {
            kind: Some("success"),
            title: title.into(),
            description,
        }
    }

    pub fn error(title: impl Into<String>, description: impl Into<String>) -> Self {
        Toast {
            kind: Some("error"),
            title: title.into(),
            description: Some(description.into()),
        }
    }
}

/// The values the app keeps in the browser, each under its key once it is set.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Kept {
    pub values: Vec<(String, Value)>,
}

impl Kept {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.iter().find(|(one, _)| one == key).map(|(_, value)| value)
    }

    pub fn text(&self, key: &str, initial: &str) -> String {
        self.get(key).and_then(Value::as_str).unwrap_or(initial).to_owned()
    }

    pub fn set(&mut self, key: &str, value: Value) {
        match self.values.iter().position(|(one, _)| one == key) {
            Some(at) => self.values[at].1 = value,
            None => self.values.push((key.to_owned(), value)),
        }
    }
}

/// What the app waits on from the server, under a call's ticket.
#[derive(Clone, Debug, PartialEq)]
pub struct Waiting {
    pub ticket: u64,
    /// The page that asked, by its serial; its own state is told only while it is open.
    pub page: u64,
    pub then: crate::pages::Then,
}

/// The app's state: the server, the route, the edits and the kept values, what it said and where
/// it went, and the open page's and side panel's own state.
#[derive(Clone, Debug)]
pub struct App {
    pub backend: Backend,
    pub hash: String,
    pub editing: Editing,
    pub kept: Kept,
    pub toasts: Vec<Toast>,
    pub went: Vec<String>,
    pub page: PageState,
    pub page_key: String,
    pub page_serial: u64,
    pub panel: Option<PanelState>,
    pub panel_key: String,
    pub waiting: Vec<Waiting>,
    /// The answers to the calls made on the server directly, as they came.
    pub calls: Vec<Value>,
}

impl App {
    /// The app as it opens at `now`, on the overview, every collection listed.
    pub fn new(now: i64) -> Self {
        let mut backend = Backend::new(now);
        backend.list_all();
        let mut app = App {
            backend,
            hash: "#/".to_owned(),
            editing: Editing::default(),
            kept: Kept::default(),
            toasts: Vec::new(),
            went: Vec::new(),
            page: PageState::NotFound,
            page_key: String::new(),
            page_serial: 0,
            panel: None,
            panel_key: String::new(),
            waiting: Vec::new(),
            calls: Vec::new(),
        };
        app.mount();
        app
    }

    pub fn route(&self) -> Vec<String> {
        route_of(&self.hash)
    }

    pub fn page_of(&self) -> Page {
        page_of(&self.route())
    }

    /// The editors' settings, the kept ones over the defaults.
    pub fn prefs(&self) -> Prefs {
        let mut prefs = Prefs::default();
        if let Some(Value::Object(kept)) = self.kept.get("prefs") {
            if let Some(Value::Bool(on)) = kept.get("lineNumbers") {
                prefs.line_numbers = *on;
            }
            if let Some(Value::Bool(on)) = kept.get("wrap") {
                prefs.wrap = *on;
            }
            if let Some(Value::String(size)) = kept.get("size") {
                prefs.size = size.clone();
            }
            if let Some(Value::Bool(on)) = kept.get("live") {
                prefs.live = *on;
            }
            if let Some(size) = kept.get("pageSize").and_then(Value::as_f64) {
                prefs.page_size = size;
            }
        }
        prefs
    }

    /// Changes one setting, as the Settings dialog's control does.
    pub fn set_pref(&mut self, id: &str, value: &Value) {
        let mut prefs = self.prefs();
        match id {
            "line-numbers" => prefs.line_numbers = value.as_bool().unwrap_or(prefs.line_numbers),
            "wrap" => prefs.wrap = value.as_bool().unwrap_or(prefs.wrap),
            "size" => prefs.size = value.as_str().unwrap_or("sm").to_owned(),
            "live" => prefs.live = value.as_bool().unwrap_or(prefs.live),
            "page-size" => {
                prefs.page_size = crate::js::number_of_text(value.as_str().unwrap_or("20"));
            }
            _ => {}
        }
        self.kept.set("prefs", serde_json::to_value(&prefs).unwrap_or_default());
    }

    /// Goes to a route, as the web's `go` sets the hash, and mounts the page it opens.
    pub fn go(&mut self, path: &str) {
        self.went.push(path.to_owned());
        self.hash = if path.starts_with('#') {
            path.to_owned()
        } else {
            format!("#{path}")
        };
        self.mount();
    }

    /// Mounts the page and the side panel the route opens, when they are not the ones open.
    pub fn mount(&mut self) {
        let page = self.page_of();
        let route = self.route();
        let key = format!("{}:{}|{}", page.name(), page.key(), content_key(&route));
        if key != self.page_key {
            self.page_key = key;
            self.page_serial += 1;
            self.page = PageState::fresh(&page);
        }
        let panel_key = match &page {
            Page::Resource(kind, id) => format!("{kind}/{id}"),
            _ => String::new(),
        };
        if panel_key != self.panel_key {
            self.panel_key = panel_key.clone();
            self.panel = (!panel_key.is_empty()).then(PanelState::default);
        }
    }

    pub fn toast(&mut self, toast: Toast) {
        self.toasts.push(toast);
    }

    /// Waits on a call's answer for the page open now.
    pub fn wait(&mut self, ticket: u64, then: crate::pages::Then) {
        self.waiting.push(Waiting {
            ticket,
            page: self.page_serial,
            then,
        });
    }

    /// Moves the clock to `to`, every timer due fired at its time and every answer handed to the
    /// page that waits on it.
    pub fn advance(&mut self, to: i64) {
        loop {
            self.settle();
            if !self.backend.fire_next(to) {
                break;
            }
        }
        self.backend.clock.now = self.backend.clock.now.max(to);
        self.settle();
    }

    /// Hands every answer the server gave to the page that waits on it.
    pub fn settle(&mut self) {
        for Settled { ticket, outcome } in self.backend.take_settled() {
            let Some(at) = self.waiting.iter().position(|one| one.ticket == ticket) else {
                continue;
            };
            let waiting = self.waiting.remove(at);
            crate::pages::answered(self, waiting, outcome);
        }
    }

    /// The resources a page shows.
    pub fn rows(&self) -> &[Stored] {
        self.backend.resources.seen()
    }
}
