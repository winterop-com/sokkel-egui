//! The workbench's frame, as the web's `App.tsx` draws it: the rail's places, the trail, the page
//! a route opens, the palette's places and server versions, the palette's search over the
//! server's metadata, the server menu's facts and the Settings words.

use serde::Serialize;

use crate::backend::{Indicator, SERVER_ADDRESS, ValidationRule};
use crate::js::trim;
use crate::metadata::{data_elements, org_units};
use crate::program::ProgramRule;
use crate::versions::{LATEST, VERSIONS, names_of};

/// The place a route's first part names, none for a route the app has no page for.
fn place_by_route(first: &str) -> Option<&'static str> {
    Some(match first {
        "" | "indicators" => "indicators",
        "validation-rules" => "validation-rules",
        "program-rules" => "program-rules",
        "data-elements" => "data-elements",
        "org-units" => "org-units",
        "functions" => "functions",
        _ => return None,
    })
}

/// A place's title.
pub fn title_of(place: &str) -> &'static str {
    match place {
        "indicators" => "Indicators",
        "validation-rules" => "Validation rules",
        "program-rules" => "Program rules",
        "data-elements" => "Data elements",
        "org-units" => "Organization units",
        "functions" => "Functions",
        _ => "",
    }
}

/// The place a route opens, indicators for one the app has no page for, and whether it has none.
pub fn place_of(route: &[String]) -> (&'static str, bool) {
    let first = route.first().map_or("", String::as_str);
    match place_by_route(first) {
        Some(place) => (place, false),
        None => ("indicators", true),
    }
}

/// The page a route opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Indicators,
    Indicator(String),
    ValidationRules,
    ValidationRule(String),
    ProgramRules(Option<String>),
    DataElements(Option<String>),
    OrgUnits(Option<String>),
    Functions,
    NotFound,
}

impl Page {
    /// The web component's name.
    pub fn name(&self) -> &'static str {
        match self {
            Page::Indicators => "Indicators",
            Page::Indicator(_) => "IndicatorPage",
            Page::ValidationRules => "ValidationRules",
            Page::ValidationRule(_) => "ValidationRulePage",
            Page::ProgramRules(_) => "ProgramRules",
            Page::DataElements(_) => "DataElements",
            Page::OrgUnits(_) => "OrgUnits",
            Page::Functions => "Functions",
            Page::NotFound => "NotFound",
        }
    }
}

/// The page a route opens.
pub fn page_of(route: &[String]) -> Page {
    let first = route.first().map(String::as_str);
    let second = route.get(1).cloned();
    let (_, missing) = place_of(route);
    match (first, second) {
        (Some("indicators"), Some(id)) => Page::Indicator(id),
        (Some("validation-rules"), Some(id)) => Page::ValidationRule(id),
        (Some("validation-rules"), None) => Page::ValidationRules,
        (Some("program-rules"), selected) => Page::ProgramRules(selected),
        (Some("data-elements"), selected) => Page::DataElements(selected),
        (Some("org-units"), selected) => Page::OrgUnits(selected),
        (Some("functions"), _) => Page::Functions,
        _ if missing => Page::NotFound,
        _ => Page::Indicators,
    }
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RailEntry {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    /// The lucide icon the web draws.
    pub icon: &'static str,
    pub href: String,
    pub active: bool,
    pub keywords: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RailGroup {
    pub id: &'static str,
    pub title: Option<&'static str>,
    pub entries: Vec<RailEntry>,
}

/// The rail's groups, the place shown marked.
pub fn rail_groups(place: &str) -> Vec<RailGroup> {
    let entry = |id: &'static str, label, description, icon, keywords: &[&'static str]| RailEntry {
        id,
        label,
        description,
        icon,
        href: format!("#/{id}"),
        active: place == id,
        keywords: keywords.to_vec(),
    };
    vec![
        RailGroup {
            id: "expressions",
            title: None,
            entries: vec![
                entry(
                    "indicators",
                    "Indicators",
                    "Numerators over denominators",
                    "Calculator",
                    &["expression"],
                ),
                entry(
                    "validation-rules",
                    "Validation rules",
                    "What the data must hold to",
                    "ShieldCheck",
                    &["violations"],
                ),
                entry(
                    "program-rules",
                    "Program rules",
                    "How a form reacts as it is filled",
                    "GitBranch",
                    &["condition", "action"],
                ),
            ],
        },
        RailGroup {
            id: "metadata",
            title: Some("Metadata"),
            entries: vec![
                entry("data-elements", "Data elements", "What is reported", "Database", &[]),
                entry(
                    "org-units",
                    "Organization units",
                    "Where it is reported from",
                    "Network",
                    &["hierarchy", "places"],
                ),
                entry(
                    "functions",
                    "Functions",
                    "What the server version knows",
                    "SquareFunction",
                    &["variables", "version"],
                ),
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
    pub code: Option<String>,
    pub verb: Option<String>,
    pub keywords: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommandGroup {
    pub title: String,
    pub verb: Option<String>,
    pub commands: Vec<Command>,
}

/// The palette's own groups, before the frame's: the places, the server versions and Settings.
pub fn palette(place: &str) -> Vec<CommandGroup> {
    let places = rail_groups(place)
        .into_iter()
        .flat_map(|group| group.entries)
        .map(|entry| Command {
            id: format!("place-{}", entry.id),
            label: entry.label.to_owned(),
            hint: Some(entry.description.to_owned()),
            code: None,
            verb: None,
            keywords: entry.keywords.iter().map(|one| (*one).to_owned()).collect(),
        })
        .collect();
    vec![
        CommandGroup {
            title: "Pages".to_owned(),
            verb: None,
            commands: places,
        },
        CommandGroup {
            title: "Server version".to_owned(),
            verb: Some("Switch".to_owned()),
            commands: VERSIONS
                .iter()
                .map(|one| Command {
                    id: format!("version-{one}"),
                    label: format!("Run as {one}"),
                    hint: None,
                    code: Some((*one).to_owned()),
                    verb: None,
                    keywords: vec!["version".to_owned(), "server".to_owned()],
                })
                .collect(),
        },
        CommandGroup {
            title: "Actions".to_owned(),
            verb: None,
            commands: vec![Command {
                id: "settings".to_owned(),
                label: "Settings".to_owned(),
                hint: None,
                code: None,
                verb: Some("Open".to_owned()),
                keywords: vec!["preferences".to_owned(), "theme".to_owned(), "version".to_owned()],
            }],
        },
    ]
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

/// What the palette finds for a query: up to six of each kind whose name, uid or code holds it.
pub fn search(
    query: &str,
    indicators: &[Indicator],
    rules: &[ValidationRule],
    program_rules: &[ProgramRule],
) -> Vec<SearchGroup> {
    let asked = trim(query).to_lowercase();
    let matches = |texts: &[&str]| texts.iter().any(|one| one.to_lowercase().contains(&asked));
    let result = |id: &str, title: &str| SearchResult {
        id: id.to_owned(),
        title: title.to_owned(),
        meta: id.to_owned(),
    };
    vec![
        SearchGroup {
            kind: "indicators",
            title: "Indicators",
            results: indicators
                .iter()
                .filter(|one| matches(&[&one.name, &one.id, &one.code]))
                .take(6)
                .map(|one| result(&one.id, &one.name))
                .collect(),
        },
        SearchGroup {
            kind: "validation-rules",
            title: "Validation rules",
            results: rules
                .iter()
                .filter(|one| matches(&[&one.name, &one.id, &one.code]))
                .take(6)
                .map(|one| result(&one.id, &one.name))
                .collect(),
        },
        SearchGroup {
            kind: "program-rules",
            title: "Program rules",
            results: program_rules
                .iter()
                .filter(|one| matches(&[&one.name, &one.id]))
                .take(6)
                .map(|one| result(&one.id, &one.name))
                .collect(),
        },
        SearchGroup {
            kind: "data-elements",
            title: "Data elements",
            results: data_elements()
                .iter()
                .filter(|one| matches(&[&one.name, &one.id, &one.code]))
                .take(6)
                .map(|one| result(&one.id, &one.name))
                .collect(),
        },
        SearchGroup {
            kind: "org-units",
            title: "Organization units",
            results: org_units()
                .iter()
                .filter(|one| matches(&[&one.name, &one.id, &one.code]))
                .take(6)
                .map(|one| result(&one.id, &one.name))
                .collect(),
        },
    ]
}

/// The palette searches once a query has this many characters.
pub const SEARCH_MIN_QUERY: usize = 2;

/// Where a search result opens.
pub fn search_route(kind: &str, id: &str) -> String {
    format!("/{kind}/{id}")
}

/// A step of the top bar's trail.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Crumb {
    pub label: String,
    pub href: Option<String>,
    pub code: bool,
}

/// The top bar's trail: the project, the place, and the indicator or rule opened.
pub fn trail(route: &[String], indicators: &[Indicator], rules: &[ValidationRule]) -> Vec<Crumb> {
    let (place, missing) = place_of(route);
    let first = route.first().map(String::as_str);
    let opened = route.get(1).and_then(|second| match first {
        Some("indicators") => indicators
            .iter()
            .find(|one| &one.id == second)
            .map(|one| one.name.clone()),
        Some("validation-rules") => rules.iter().find(|one| &one.id == second).map(|one| one.name.clone()),
        _ => None,
    });
    let mut out = vec![
        Crumb {
            label: "larkspur".to_owned(),
            href: Some("#/".to_owned()),
            code: true,
        },
        Crumb {
            label: if missing { "Not found" } else { title_of(place) }.to_owned(),
            href: opened.as_ref().map(|_| format!("#/{place}")),
            code: false,
        },
    ];
    if let Some(opened) = opened {
        out.push(Crumb {
            label: opened,
            href: None,
            code: false,
        });
    }
    out
}

/// The version the app runs as: the one kept, or the latest when what is kept is no version.
pub fn version_of(stored: &str) -> &'static str {
    VERSIONS.iter().find(|one| **one == stored).copied().unwrap_or(LATEST)
}

/// The facts the server menu lists for a version.
pub fn server_facts(version: &str) -> Vec<String> {
    let names = names_of(version);
    vec![
        SERVER_ADDRESS.to_owned(),
        format!(
            "Knows {} functions and {} program variables",
            names.functions.len(),
            names.variables.len()
        ),
        "Simulated: every value is made in the page".to_owned(),
    ]
}

/// The app's name, as the rail and the palette's foot write it.
pub const NAME: &str = "DHIS2 Workbench";

/// What the palette's field asks for.
pub const PALETTE_PLACEHOLDER: &str = "Go to a page, find an indicator, a rule or a data element";

/// The Settings words: the server version's and the demo data's.
pub const VERSION_SETTING: (&str, &str) = (
    "Server version",
    "The version the simulated server runs. Each knows its own functions and variables: the editors color, offer and check only those.",
);
pub const RESET_SETTING: (&str, &str) = (
    "Reset the demo data",
    "Puts back the indicators, validation rules and program rules the demo starts with.",
);
