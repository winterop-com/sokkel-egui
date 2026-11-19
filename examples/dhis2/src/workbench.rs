//! The workbench inside the shell, as the web's `App.tsx`: the rail with its places, the top bar's
//! trail, search and the server menu with its version, the page the address names, the status
//! line, and the palette, Settings and the questions over them. The dhis2 add-on's languages are
//! installed with the names the running version knows, again whenever it changes. The clock is the
//! app's: the backend's answers come as it moves on.

use std::collections::{HashMap, HashSet};

use eframe::egui::{self, Ui};
use sokkel_egui::plugins::register_plugins;
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, AppearanceCommandWords, CommandGroup, CommandPalette, ConfirmAnswer,
    ConfirmDialog, Icon, MenuItem, PaletteWords, Rail, RailEntry, RailGroup, Segmented, ServerMenu, ServerState,
    SettingRow, SettingsDialog, SettingsSection, Shell, ShellPart, StatusLine, StatusTone, TableState, ThemeWords,
    Toast, Toaster, TopBar, appearance_commands, install, place_commands, place_of, run_appearance_command,
    theme_settings_listed,
};
use sokkel_egui_dhis2::dhis2_plugin;

use crate::app::{
    NAME, PALETTE_PLACEHOLDER, RESET_SETTING, SEARCH_MIN_QUERY, VERSION_SETTING, palette, rail_groups, search,
    search_route, server_facts, trail, version_of,
};
use crate::backend::{Backend, Event, ME, SERVER_ADDRESS, SERVER_NAME, Saving, saving_note};
use crate::pages::{indicator, org_units::OrgUnits, program_rules::ProgramRules, validation_rule};
use crate::themes::CARRIED;
use crate::versions::{LATEST, VERSIONS, names_of};

/// 2026-11-18 09:00 UTC: the moment the app's screenshots are taken at, as the web's are shot.
pub const DEMO_START: i64 = 1_794_992_400_000;

/// The Tidevann mark the rail heads with.
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// A list page's search, its filters and its table.
#[derive(Default)]
pub struct ListView {
    pub query: String,
    /// The first filter's values: the indicators' types, the functions' kinds.
    pub first: Vec<String>,
    /// The second filter's values: the indicators' checks, the functions' known or not.
    pub second: Vec<String>,
    pub table: TableState,
}

/// What a page holds while it is shown, as the web's pages keep in their own state; each starts
/// afresh on another page, as the web's `contentKey` mounts it anew.
pub struct Views {
    pub indicators: ListView,
    pub rules: ListView,
    pub elements: ListView,
    pub functions: ListView,
    pub indicator: Option<indicator::Editor>,
    pub below: TableState,
    pub rule: Option<validation_rule::Editor>,
    pub violations: TableState,
    pub program: ProgramRules,
    pub program_open: HashSet<String>,
    pub effects: TableState,
    pub broken: TableState,
    pub units: OrgUnits,
    pub units_open: HashSet<String>,
    pub units_for: Option<String>,
    pub values: TableState,
    /// The widths of the columns beside a page and of the data element's sheet.
    pub column: f32,
    pub sheet: f32,
}

impl Default for Views {
    fn default() -> Self {
        Self {
            indicators: ListView::default(),
            rules: ListView::default(),
            elements: ListView::default(),
            functions: ListView::default(),
            indicator: None,
            below: TableState::default(),
            rule: None,
            violations: TableState::default(),
            program: ProgramRules::new(),
            program_open: HashSet::new(),
            effects: TableState::default(),
            broken: TableState::default(),
            units: OrgUnits::default(),
            units_open: HashSet::new(),
            units_for: None,
            values: TableState::default(),
            column: 264.0,
            sheet: 448.0,
        }
    }
}

/// A lucide icon's name as the web writes it, `ShieldCheck`, in the catalog's `shield-check`.
fn kebab(name: &str) -> String {
    let mut out = String::new();
    for (at, c) in name.char_indices() {
        if c.is_ascii_uppercase() && at > 0 {
            out.push('-');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// A question asked in a dialog before it is done.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asking {
    DeleteIndicator,
    DeleteRule,
    DeleteProgramRule(String),
    Reset,
}

pub struct WorkbenchApp {
    pub appearance: Appearance,
    pub backend: Backend,
    /// The address, split at its slashes.
    pub route: Vec<String>,
    /// The settings over the page, and the section they show.
    pub settings: Option<String>,
    /// The palette while it is open: what is typed and the row lit.
    pub palette: Option<(String, usize)>,
    /// The version the simulated server runs, as kept.
    pub version: String,
    pub views: Views,
    pub asking: Option<Asking>,
    /// The version the add-on's languages were installed with, and the context they were installed in.
    plugged: Option<(String, egui::Context)>,
    toast_ids: HashMap<u64, ()>,
    started: i64,
    started_at: Option<f64>,
    elapsed: i64,
    held: bool,
    pub desktop: bool,
}

/// What eframe's storage keeps between visits: the rows of each collection and the version.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Kept {
    pub indicators: Vec<crate::backend::Indicator>,
    pub validation_rules: Vec<crate::backend::ValidationRule>,
    pub program_rules: Vec<crate::program::ProgramRule>,
    pub version: String,
}

impl WorkbenchApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_millis() as i64);
        let appearance = cc
            .storage
            .and_then(|s| eframe::get_value(s, "appearance"))
            .unwrap_or_default();
        let mut app = Self::at(now, appearance);
        if let Some(kept) = cc.storage.and_then(|s| eframe::get_value::<Kept>(s, "dhis2")) {
            app.backend
                .restore(kept.indicators, kept.validation_rules, kept.program_rules);
            app.version = version_of(&kept.version).to_owned();
        }
        app.held = false;
        app.desktop = true;
        app
    }

    /// The workbench at a moment with the data as the demo starts, its clock held there, as the
    /// tests and the screenshots take it.
    pub fn at(now: i64, appearance: Appearance) -> Self {
        Self {
            appearance,
            backend: Backend::new(now),
            route: Vec::new(),
            settings: None,
            palette: None,
            version: LATEST.to_owned(),
            views: Views::default(),
            asking: None,
            plugged: None,
            toast_ids: HashMap::new(),
            started: now,
            started_at: None,
            elapsed: 0,
            held: true,
            desktop: false,
        }
    }

    pub fn now(&self) -> i64 {
        self.started + self.elapsed
    }

    /// Moves a held clock on, as the tests wait for the backend's answers.
    pub fn wait(&mut self, ms: i64) {
        self.elapsed += ms;
        let now = self.now();
        self.backend.advance(now);
        self.heard();
    }

    /// The version the server runs as: the one kept, or the latest.
    pub fn version(&self) -> &'static str {
        version_of(&self.version)
    }

    /// Runs as another version.
    pub fn set_version(&mut self, version: &str) {
        self.version = version_of(version).to_owned();
    }

    /// Goes to an address, as the web's hash routes: `indicators[/<id>]`,
    /// `validation-rules[/<id>]`, `program-rules[/<id>]`, `data-elements[/<id>]`,
    /// `org-units[/<id>]` and `functions`; `settings` opens over the page, which stays.
    pub fn go(&mut self, path: &str) {
        let path = path.trim_start_matches('#').trim_matches('/');
        if path == "settings" || path.starts_with("settings/") {
            self.open_settings("appearance");
            return;
        }
        let next = crate::app::route_of(path);
        if next.first() != self.route.first() || next.get(1) != self.route.get(1) {
            let column = self.views.column;
            let sheet = self.views.sheet;
            self.views = Views {
                column,
                sheet,
                ..Views::default()
            };
        }
        self.route = next;
    }

    pub fn open_settings(&mut self, section: &str) {
        self.settings = Some(section.to_owned());
    }

    /// The pages hear what the backend answered, and the app follows where it went.
    fn heard(&mut self) {
        for event in self.backend.take_events() {
            if let Some(editor) = &mut self.views.indicator {
                editor.hear(&event);
            }
            if let Some(editor) = &mut self.views.rule {
                editor.hear(&event);
            }
            if event == Event::Settled || event == Event::Discard {
                self.views.program.hear(&event);
            }
        }
        for path in std::mem::take(&mut self.backend.went) {
            self.go(&path);
        }
    }

    /// The notes the backend said since the last frame, as toasts.
    fn say(&mut self, ctx: &egui::Context) {
        for toast in std::mem::take(&mut self.backend.toasts) {
            let mut note = match toast.kind {
                Some("error") => Toast::error(toast.title),
                Some("success") => Toast::success(toast.title),
                _ => Toast::new(toast.title),
            }
            .timeout(Some(4.0));
            if let Some(description) = toast.description {
                note = note.description(description);
            }
            let id = note.show(ctx);
            self.toast_ids.insert(id, ());
        }
    }

    /// The add-on's languages, knowing the names the running version has.
    fn plug(&mut self, ctx: &egui::Context) {
        let version = self.version();
        if self
            .plugged
            .as_ref()
            .is_some_and(|(was, at)| was == version && at == ctx)
        {
            return;
        }
        register_plugins(ctx, [dhis2_plugin(&names_of(version).plugin_names())]);
        self.plugged = Some((version.to_owned(), ctx.clone()));
    }

    /// Each page's own state made for the route it shows, as the web's keys make a page anew; an
    /// editor's saved row follows the server's.
    fn follow_pages(&mut self) {
        let first = self.route.first().map(String::as_str);
        let second = self.route.get(1).cloned();
        match (first, second) {
            (Some("indicators"), Some(id)) => {
                if let Some(saved) = self.backend.indicators.seen().iter().find(|one| one.id == id) {
                    match &mut self.views.indicator {
                        Some(editor) if editor.saved.id == id => editor.saved = saved.clone(),
                        _ => self.views.indicator = Some(indicator::Editor::new(saved)),
                    }
                }
            }
            (Some("validation-rules"), Some(id)) => {
                if let Some(saved) = self.backend.validation_rules.seen().iter().find(|one| one.id == id) {
                    match &mut self.views.rule {
                        Some(editor) if editor.saved.id == id => editor.saved = saved.clone(),
                        _ => self.views.rule = Some(validation_rule::Editor::new(saved)),
                    }
                }
            }
            _ => {}
        }
    }

    /// The rail's groups, the place shown marked.
    pub fn rail(&self) -> (Vec<RailGroup>, String) {
        let (place, _) = crate::app::place_of(&self.route);
        let mut active = String::new();
        let groups = rail_groups(place)
            .into_iter()
            .map(|group| {
                let entries: Vec<RailEntry> = group
                    .entries
                    .iter()
                    .map(|entry| {
                        if entry.active {
                            active = entry.id.to_owned();
                        }
                        let icon = sokkel_egui::icons::by_name(&kebab(entry.icon)).unwrap_or(Icon::Circle);
                        RailEntry::new(entry.id, entry.label, icon).description(entry.description)
                    })
                    .collect();
                RailGroup::new(group.title, entries)
            })
            .collect();
        (groups, active)
    }

    /// What the status line says at its start for the page shown.
    fn foot_note(&self) -> String {
        crate::screens::foot(self)
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        let ctx = ui.ctx().clone();
        if !self.held {
            let time = ctx.input(|i| i.time);
            let since = *self.started_at.get_or_insert(time);
            self.elapsed = ((time - since) * 1000.0) as i64;
            let busy = self.backend.next_due().is_some();
            ctx.request_repaint_after(std::time::Duration::from_millis(if busy { 33 } else { 1000 }));
        }
        if self.route.is_empty() && self.settings.is_none() {
            self.go("indicators");
        }
        self.plug(&ctx);
        let now = self.now();
        self.backend.advance(now);
        self.heard();
        self.say(&ctx);
        self.follow_pages();

        let version = self.version();
        let (groups, mut active) = self.rail();
        let was = active.clone();
        let crumbs = trail(
            &self.route,
            self.backend.indicators.seen(),
            self.backend.validation_rules.seen(),
        );
        let words: Vec<&str> = crumbs.iter().map(|one| one.label.as_str()).collect();
        let facts = server_facts(version);
        let fact_words: Vec<&str> = facts.iter().map(String::as_str).collect();
        let mut server_items = vec![MenuItem::Label(VERSION_SETTING.0.to_owned())];
        server_items.extend(VERSIONS.iter().map(|one| {
            let item = MenuItem::action(*one).code(true);
            if *one == version {
                item.icon(Icon::Check)
            } else {
                item.icon(Icon::Dot)
            }
        }));
        server_items.push(MenuItem::Separator);
        server_items.push(MenuItem::action("What this version knows").icon(Icon::SquareFunction));
        let note = self.foot_note();
        let (saving, tone, live) = saving_note(&self.backend.saving);
        let mut account = None;
        let mut server = sokkel_egui::ServerMenuResponse::default();
        let mut searched = sokkel_egui::palette_shortcut(&ctx);
        let mut crumb = None;
        let mut shell = Shell::new();
        if self.desktop {
            shell = shell.desktop(sokkel_egui::DesktopWindow::native());
        }
        let mut views = std::mem::take(&mut self.views);
        let mut acts: Vec<crate::screens::Act> = Vec::new();
        {
            let app = &*self;
            shell.show_window(ui, |part, ui| match part {
                ShellPart::Rail => {
                    ui.add(Rail::new(NAME, &groups, &mut active).mark(TIDEVANN_MARK));
                }
                ShellPart::RailFoot => {
                    account = AccountMenu::new(ME)
                        .place(SERVER_ADDRESS)
                        .local(true)
                        .appearance(true)
                        .show(ui);
                }
                ShellPart::Top => {
                    let out = TopBar::new(&words)
                        .linked(true)
                        .code(0)
                        .search("Search", "\u{2318}K")
                        .show(ui, |ui| {
                            server = ServerMenu::new(SERVER_NAME, ServerState::Up)
                                .version(Some(version))
                                .facts(&fact_words)
                                .items(&server_items)
                                .show(ui);
                        });
                    searched |= out.search;
                    crumb = out.crumb;
                }
                ShellPart::Page => crate::screens::page(ui, app, &mut views, &mut acts),
                ShellPart::Foot => {
                    // the page's note, or where the changes stand while they are on their way
                    let mut line = if self.backend.saving == Saving::Saved {
                        StatusLine::new(note.clone())
                    } else {
                        StatusLine::new(saving.clone()).live(live)
                    };
                    if tone == "error" {
                        line = line.tone(StatusTone::Error);
                    }
                    ui.add(line.item("server", version));
                }
            });
        }
        self.views = views;
        for act in acts {
            self.apply(act);
        }
        if active != was {
            self.go(&active);
        }
        if let Some(index) = crumb
            && let Some(href) = crumbs.get(index).and_then(|one| one.href.clone())
        {
            self.go(&href);
        }
        match account {
            Some(AccountAction::Settings) => self.open_settings("appearance"),
            Some(AccountAction::Appearance) => self.open_settings("appearance"),
            None => {}
        }
        if let Some(item) = server.item {
            // the label stands first, then the versions, the rule and the functions
            match item.checked_sub(1) {
                Some(at) if at < VERSIONS.len() => self.set_version(VERSIONS[at]),
                Some(_) => self.go("functions"),
                None => {}
            }
        }
        if searched {
            self.palette = Some(Default::default());
        }
        self.heard();
        self.search_palette(&ctx, &groups);
        self.settings_dialog(&ctx);
        self.questions(&ctx);
        self.heard();
        Toaster::new().show(&ctx);
    }

    /// The palette while it is open: the places, the server versions, Settings and the look, and
    /// once two letters are typed, the indicators, rules, data elements and units they find.
    fn search_palette(&mut self, ctx: &egui::Context, rail: &[RailGroup]) {
        let Some((mut query, mut highlight)) = self.palette.take() else {
            return;
        };
        let (place, _) = crate::app::place_of(&self.route);
        let mut groups: Vec<CommandGroup> = Vec::new();
        if crate::js::trim(&query).chars().count() >= SEARCH_MIN_QUERY {
            for group in search(
                &query,
                self.backend.indicators.seen(),
                self.backend.validation_rules.seen(),
                self.backend.program_rules.seen(),
            ) {
                if group.results.is_empty() {
                    continue;
                }
                let icon = match group.kind {
                    "indicators" => Icon::Calculator,
                    "validation-rules" => Icon::ShieldCheck,
                    "program-rules" => Icon::GitBranch,
                    "data-elements" => Icon::Database,
                    _ => Icon::Network,
                };
                groups.push(CommandGroup::new(
                    group.title,
                    group.results.iter().map(|one| {
                        MenuItem::action(&one.title)
                            .icon(icon)
                            .hint(&one.meta)
                            .id(format!("open:{}", search_route(group.kind, &one.id)))
                    }),
                ));
            }
        }
        let mut own = palette(place);
        let places = own.remove(0);
        groups.extend(place_commands(rail, places.title));
        for group in own {
            let title = group.title.clone();
            groups.push(CommandGroup::new(
                title.clone(),
                group.commands.iter().map(|command| {
                    let mut item = MenuItem::action(&command.label)
                        .keywords(command.keywords.iter().cloned())
                        .id(format!("do:{}", command.id));
                    item = if title == "Actions" {
                        item.icon(Icon::Settings)
                    } else {
                        item.icon(Icon::SquareFunction)
                    };
                    if let Some(code) = &command.code {
                        item = item.hint(code);
                    }
                    item
                }),
            ));
        }
        let themes: Vec<_> = sokkel_egui::themes()
            .filter(|theme| CARRIED.contains(&theme.name.as_str()))
            .collect();
        groups.extend(appearance_commands(
            &self.appearance,
            &themes,
            &AppearanceCommandWords::default(),
        ));
        let outcome = CommandPalette::new("palette", &groups, &mut query, &mut highlight)
            .words(PaletteWords {
                placeholder: PALETTE_PLACEHOLDER.to_owned(),
                ..Default::default()
            })
            .show(ctx);
        let command = outcome
            .run
            .and_then(|(g, i)| groups[g].items[i].command_id().map(str::to_owned));
        if let Some(command) = command.as_deref() {
            if let Some(path) = command.strip_prefix("open:") {
                self.go(path);
            } else if let Some(does) = command.strip_prefix("do:") {
                match does.strip_prefix("version-") {
                    Some(version) => self.set_version(version),
                    None => self.open_settings("appearance"),
                }
            } else if let Some(place) = place_of(command) {
                self.go(place);
            } else {
                run_appearance_command(&mut self.appearance, command);
            }
        }
        if !outcome.close {
            self.palette = Some((query, highlight));
        }
    }

    /// Settings over the page while they are open, each change kept as it is made.
    fn settings_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut active) = self.settings.take() else { return };
        let sections = [
            SettingsSection::new("appearance", "Appearance", None),
            SettingsSection::new("server", "Server", None),
            SettingsSection::new("demo", "Demo data", None).reset(false),
        ];
        let mut version = VERSIONS.iter().position(|one| *one == self.version()).unwrap_or(0);
        let mut ask_reset = false;
        let appearance = &mut self.appearance;
        let outcome = SettingsDialog::new("settings", &sections, &mut active).show(ctx, |section, ui| match section {
            "server" => {
                let mut keywords: Vec<&str> = vec!["version", "functions", "variables"];
                keywords.extend(VERSIONS);
                SettingRow::new(VERSION_SETTING.0, VERSION_SETTING.1)
                    .keywords(&keywords)
                    .below()
                    .show(ui, |ui| {
                        ui.add(Segmented::new(&mut version, VERSIONS));
                    });
            }
            "demo" => {
                SettingRow::new(RESET_SETTING.0, RESET_SETTING.1).show(ui, |ui| {
                    let reset = sokkel_egui::Button::new("Reset")
                        .variant(sokkel_egui::ButtonVariant::Outline)
                        .size(sokkel_egui::ButtonSize::Sm);
                    ask_reset = ui.add(reset).clicked();
                });
            }
            _ => theme_settings_listed(ui, appearance, &CARRIED, true, &ThemeWords::default()),
        });
        if VERSIONS[version] != self.version() {
            self.set_version(VERSIONS[version]);
        }
        if ask_reset {
            self.asking = Some(Asking::Reset);
        }
        if outcome.reset {
            match active.as_str() {
                "appearance" => self.appearance = Appearance::default(),
                "server" => self.set_version(LATEST),
                _ => {}
            }
        }
        if !outcome.close {
            self.settings = Some(active);
        }
    }

    /// The question asked before something that cannot be undone, while it is asked.
    fn questions(&mut self, ctx: &egui::Context) {
        let Some(asking) = self.asking.clone() else { return };
        let (question, detail, label) = match &asking {
            Asking::DeleteIndicator => match &self.views.indicator {
                Some(editor) => {
                    let (question, detail) = editor.delete_question();
                    (question, detail.to_owned(), "Delete")
                }
                None => {
                    self.asking = None;
                    return;
                }
            },
            Asking::DeleteRule => match &self.views.rule {
                Some(editor) => {
                    let (question, detail) = editor.delete_question();
                    (question, detail.to_owned(), "Delete")
                }
                None => {
                    self.asking = None;
                    return;
                }
            },
            Asking::DeleteProgramRule(id) => match self.backend.program_rules.seen().iter().find(|one| one.id == *id) {
                Some(saved) => {
                    let (question, detail) = ProgramRules::delete_question(saved);
                    (question, detail.to_owned(), "Delete")
                }
                None => {
                    self.asking = None;
                    return;
                }
            },
            Asking::Reset => ("Reset the demo data?".to_owned(), RESET_SETTING.1.to_owned(), "Reset"),
        };
        let answer = ConfirmDialog::new(("question", format!("{asking:?}")), &question, label)
            .detail(detail)
            .destructive(true)
            .show(ctx);
        match answer {
            Some(ConfirmAnswer::Confirm) => {
                self.asking = None;
                match asking {
                    Asking::DeleteIndicator => {
                        if let Some(editor) = &self.views.indicator {
                            editor.remove(&mut self.backend);
                        }
                    }
                    Asking::DeleteRule => {
                        if let Some(editor) = &self.views.rule {
                            editor.remove(&mut self.backend);
                        }
                    }
                    Asking::DeleteProgramRule(id) => {
                        if let Some(saved) = self
                            .backend
                            .program_rules
                            .seen()
                            .iter()
                            .find(|one| one.id == id)
                            .cloned()
                        {
                            self.views.program.remove(&saved, &mut self.backend);
                        }
                    }
                    Asking::Reset => self.reset_demo(),
                }
            }
            Some(_) => self.asking = None,
            None => {}
        }
    }

    /// Everything as the demo started: the indicators, rules and program rules seeded again, the
    /// page loaded again.
    pub fn reset_demo(&mut self) {
        self.backend = Backend::new(self.now());
        self.views = Views::default();
        self.settings = None;
        self.route = vec!["indicators".to_owned()];
    }
}

impl eframe::App for WorkbenchApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "appearance", &self.appearance);
        let kept = Kept {
            indicators: self.backend.indicators.rows.clone(),
            validation_rules: self.backend.validation_rules.rows.clone(),
            program_rules: self.backend.program_rules.rows.clone(),
            version: self.version.clone(),
        };
        eframe::set_value(storage, "dhis2", &kept);
    }
}
