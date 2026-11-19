//! The workbench inside the shell, as the web's `App.tsx`: the rail with the server's places and
//! the resource types with their counts, the top bar's trail and search, the page the address
//! names with a resource's side panel beside it, the status line, and the palette, Settings and the
//! questions over them. The fhir add-on's languages are installed once. The clock is the app's: the
//! server's answers come as it moves on.

use eframe::egui::{self, Ui};
use serde_json::{Value, json};
use sokkel_egui::plugins::register_plugins;
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, AppearanceCommandWords, Button, ButtonSize, ButtonVariant, CodeProblem,
    CommandGroup, CommandPalette, ConfirmAnswer, ConfirmDialog, Icon, MenuItem, PaletteWords, Rail, RailEntry,
    RailGroup, Segmented, SettingRow, SettingsDialog, SettingsSection, Shell, ShellPart, StatusLine, Switch,
    TableState, ThemeWords, Toast, Toaster, TopBar, appearance_commands, install, place_commands, place_of,
    run_appearance_command, theme_settings_listed,
};

use crate::app::{App, Crumb, NAME, PALETTE_PLACEHOLDER, SEARCH_MIN_QUERY, counts, rail_groups, search, trail};
use crate::server::{ME, SERVER_ADDRESS};
use crate::themes::CARRIED;
use crate::types::TYPE_NAMES;

/// 2026-11-18 09:00 UTC: the moment the app's screenshots are taken at, as the web's are shot.
pub const DEMO_START: i64 = 1_794_992_400_000;

/// The Tidevann mark the rail heads with.
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// The Settings' words: each setting's title and what it does.
pub const LINE_NUMBERS: (&str, &str) = ("Line numbers", "Number the lines of every editor.");
pub const WRAP: (&str, &str) = (
    "Wrap long lines",
    "Fold long lines at the editor\u{2019}s edge instead of scrolling sideways.",
);
pub const TEXT_SIZE: (&str, &str) = ("Text size", "Small fits more of a resource; regular reads easier.");
pub const LIVE: (&str, &str) = (
    "Evaluate as typed",
    "Off, an expression is evaluated only on Evaluate or Mod-Enter.",
);
pub const PAGE_SIZE: (&str, &str) = (
    "Matches per page",
    "How many matches a search returns when the query gives no _count.",
);
pub const RESET: (&str, &str) = (
    "Reset the demo data",
    "Puts back the resources and versions the server starts with, and drops unsaved edits.",
);

/// What a page holds while it is shown beyond what the web's page state keeps: its tables, the
/// side panel's tab and the editor's problems; each starts afresh on another page.
#[derive(Default)]
pub struct Views {
    pub changes: TableState,
    pub resources: TableState,
    pub libraries: TableState,
    pub versions: TableState,
    pub matches: TableState,
    pub side: TableState,
    pub panel: [TableState; 4],
    /// The side panel's tab, by its id, once one is chosen.
    pub tab: Option<&'static str>,
    /// What the resource's editor shows as problems.
    pub problems: Vec<CodeProblem>,
    /// A line the resource's editor is asked to go to.
    pub reveal: Option<usize>,
}

/// A question asked in a dialog before it is done.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asking {
    DeleteResource(String, String),
    Reset,
}

pub struct WorkbenchApp {
    pub appearance: Appearance,
    /// The web app's state: the server, the route, the edits, the kept values and the page's own.
    pub app: App,
    /// The settings over the page, and the section they show.
    pub settings: Option<String>,
    /// The palette while it is open: what is typed and the row lit.
    pub palette: Option<(String, usize)>,
    pub views: Views,
    /// The page the views were made for, by its serial.
    views_for: u64,
    pub asking: Option<Asking>,
    pub panel_open: bool,
    plugged: Option<egui::Context>,
    started: i64,
    started_at: Option<f64>,
    elapsed: i64,
    held: bool,
    pub desktop: bool,
}

/// What eframe's storage keeps between visits: the server's rows, the values the web keeps in the
/// browser and the unsaved edits.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Kept {
    pub resources: Vec<crate::server::Stored>,
    pub history: Vec<crate::server::Version>,
    pub saved: Vec<crate::server::SavedSearches>,
    pub kept: crate::app::Kept,
    pub drafts: Vec<(String, crate::editing::Draft)>,
}

/// A lucide icon's name as the web writes it, `LayoutDashboard`, in the catalog's `layout-dashboard`.
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

fn icon_of(name: &str) -> Icon {
    sokkel_egui::icons::by_name(&kebab(name)).unwrap_or(Icon::Circle)
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
        if let Some(kept) = cc.storage.and_then(|s| eframe::get_value::<Kept>(s, "fhir")) {
            app.app.backend.restore(kept.resources, kept.history, kept.saved);
            app.app.kept = kept.kept;
            app.app.editing.drafts = kept.drafts;
        }
        app.held = false;
        app.desktop = true;
        app
    }

    /// The workbench at a moment with the data as the demo starts, its clock held there, as the
    /// tests and the screenshots take it.
    pub fn at(now: i64, appearance: Appearance) -> Self {
        let app = App::new(now);
        Self {
            appearance,
            views_for: app.page_serial,
            app,
            settings: None,
            palette: None,
            views: Views::default(),
            asking: None,
            panel_open: true,
            plugged: None,
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

    /// Moves a held clock on, as the tests wait for the server's answers.
    pub fn wait(&mut self, ms: i64) {
        self.elapsed += ms;
        let now = self.now();
        self.app.advance(now);
    }

    /// Goes to an address, as the web's hash routes: `overview`, `resources[/<Type>[/<id>|new]]`,
    /// `fhirpath[/<Type>/<id>]`, `cql[/<id>]` and `search`; `settings` opens over the page, which
    /// stays.
    pub fn go(&mut self, path: &str) {
        let path = path.trim_start_matches('#').trim_matches('/');
        if path == "settings" || path.starts_with("settings/") {
            self.open_settings("appearance");
            return;
        }
        self.app.go(&format!("/{path}"));
    }

    pub fn open_settings(&mut self, section: &str) {
        self.settings = Some(section.to_owned());
    }

    /// The views made anew for a page mounted anew, as the web's keys mount it.
    fn follow_page(&mut self) {
        if self.views_for != self.app.page_serial {
            self.views_for = self.app.page_serial;
            self.views = Views::default();
        }
    }

    /// The notes the app said since the last frame, as toasts.
    fn say(&mut self, ctx: &egui::Context) {
        for toast in std::mem::take(&mut self.app.toasts) {
            let mut note = match toast.kind {
                Some("error") => Toast::error(toast.title),
                Some("success") => Toast::success(toast.title),
                _ => Toast::new(toast.title),
            }
            .timeout(Some(4.0));
            if let Some(description) = toast.description {
                note = note.description(description);
            }
            note.show(ctx);
        }
        self.app.went.clear();
    }

    /// The add-on's languages, once for the context.
    fn plug(&mut self, ctx: &egui::Context) {
        if self.plugged.as_ref() != Some(ctx) {
            register_plugins(ctx, [sokkel_egui_fhir::fhir()]);
            self.plugged = Some(ctx.clone());
        }
    }

    /// The rail's groups and every place's address by its id, the place shown marked.
    pub fn rail(&self) -> (Vec<RailGroup>, String, Vec<(String, String)>) {
        let route = self.app.route();
        let (place, _) = crate::app::place_of(&route);
        let kind = crate::app::resource_type_of(&route);
        let counted = counts(self.app.rows());
        let mut active = String::new();
        let mut hrefs = Vec::new();
        let groups = rail_groups(place, kind.as_deref(), &counted)
            .into_iter()
            .map(|group| {
                let entries: Vec<RailEntry> = group
                    .entries
                    .iter()
                    .map(|entry| {
                        if entry.active {
                            active = entry.id.to_owned();
                        }
                        hrefs.push((entry.id.to_owned(), entry.href.clone()));
                        let mut one = RailEntry::new(entry.id, entry.label, icon_of(entry.icon));
                        if let Some(description) = entry.description {
                            one = one.description(description);
                        }
                        if let Some(count) = entry.count {
                            one = one.count(count as u32);
                        }
                        let children: Vec<RailEntry> = entry
                            .entries
                            .iter()
                            .map(|child| {
                                if child.active {
                                    active = child.id.clone();
                                }
                                hrefs.push((child.id.clone(), child.href.clone()));
                                let mut made = RailEntry::new(&child.id, &child.label, icon_of(entry.icon));
                                if let Some(count) = child.count {
                                    made = made.count(count as u32);
                                }
                                made
                            })
                            .collect();
                        one.entries(children)
                    })
                    .collect();
                RailGroup::new(group.title, entries)
            })
            .collect();
        (groups, active, hrefs)
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        let ctx = ui.ctx().clone();
        if !self.held {
            let time = ctx.input(|i| i.time);
            let since = *self.started_at.get_or_insert(time);
            self.elapsed = ((time - since) * 1000.0) as i64;
            let busy = self.app.backend.next_due().is_some();
            ctx.request_repaint_after(std::time::Duration::from_millis(if busy { 33 } else { 1000 }));
        }
        self.plug(&ctx);
        let now = self.now();
        self.app.advance(now);
        self.follow_page();
        self.say(&ctx);

        let route = self.app.route();
        let (groups, mut active, hrefs) = self.rail();
        let was = active.clone();
        let crumbs: Vec<Crumb> = trail(&route);
        let words: Vec<&str> = crumbs.iter().map(|one| one.label.as_str()).collect();
        let (note, items) = crate::screens::foot(&self.app);
        let page = self.app.page_of();
        let panel = match &page {
            crate::app::Page::Resource(kind, id) => Some((kind.clone(), id.clone())),
            _ => None,
        };
        let mut account = None;
        let mut searched = sokkel_egui::palette_shortcut(&ctx);
        let mut crumb = None;
        let mut acts: Vec<crate::screens::Act> = Vec::new();
        let mut panel_acts: Vec<crate::screens::Act> = Vec::new();
        let mut views = std::mem::take(&mut self.views);
        let mut panel_views = Views {
            panel: std::mem::take(&mut views.panel),
            tab: views.tab,
            problems: views.problems.clone(),
            ..Views::default()
        };
        let mut panel_open = self.panel_open;
        {
            let app = &self.app;
            let mut shell = Shell::new();
            if self.desktop {
                shell = shell.desktop(sokkel_egui::DesktopWindow::native());
            }
            if let Some((kind, id)) = &panel {
                let (panel_views, panel_acts) = (&mut panel_views, &mut panel_acts);
                shell = shell.panel_width(400.0).panel(&mut panel_open, move |ui| {
                    crate::detail::resource_panel(ui, app, kind, id, panel_views, panel_acts);
                });
            }
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
                    let mut bar = TopBar::new(&words).linked(true).code(0);
                    for (at, one) in crumbs.iter().enumerate().skip(1) {
                        if one.code {
                            bar = bar.code(at);
                        }
                    }
                    let out = bar.search("Search", "\u{2318}K").show(ui, |_| {});
                    searched |= out.search;
                    crumb = out.crumb;
                }
                ShellPart::Page => crate::screens::page(ui, app, &mut views, &mut acts),
                ShellPart::Foot => {
                    let mut line = StatusLine::new(note.clone());
                    if let Some(items) = &items {
                        line = line.item("edit", items);
                    }
                    ui.add(line);
                }
            });
        }
        views.panel = panel_views.panel;
        views.tab = panel_views.tab;
        if panel_views.reveal.is_some() {
            views.reveal = panel_views.reveal;
        }
        self.views = views;
        self.panel_open = panel_open;
        for act in acts.into_iter().chain(panel_acts) {
            self.apply(act);
        }
        if active != was
            && let Some((_, href)) = hrefs.iter().find(|(id, _)| *id == active)
        {
            let href = href.clone();
            self.go(&href);
        }
        if let Some(index) = crumb
            && let Some(href) = crumbs.get(index).and_then(|one| one.href.clone())
        {
            self.go(&href);
        }
        match account {
            Some(AccountAction::Settings | AccountAction::Appearance) => self.open_settings("appearance"),
            None => {}
        }
        if searched {
            self.palette = Some(Default::default());
        }
        self.search_palette(&ctx, &groups, &hrefs);
        self.settings_dialog(&ctx);
        self.questions(&ctx);
        crate::screens::save_dialog(&ctx, self);
        self.follow_page();
        self.say(&ctx);
        Toaster::new().show(&ctx);
    }

    /// The palette while it is open: the places, a new resource of each type, Settings and the
    /// look, and once two letters are typed, the resources they find.
    fn search_palette(&mut self, ctx: &egui::Context, rail: &[RailGroup], hrefs: &[(String, String)]) {
        let Some((mut query, mut highlight)) = self.palette.take() else {
            return;
        };
        let mut groups: Vec<CommandGroup> = Vec::new();
        if crate::js::trim(&query).chars().count() >= SEARCH_MIN_QUERY {
            for group in search(&query, self.app.rows()) {
                if group.results.is_empty() {
                    continue;
                }
                groups.push(CommandGroup::new(
                    group.title,
                    group.results.iter().map(|one| {
                        MenuItem::action(&one.title)
                            .icon(Icon::Database)
                            .hint(&one.meta)
                            .id(format!("open:resources/{}", one.id))
                    }),
                ));
            }
        }
        groups.extend(place_commands(rail, "Pages"));
        let mut actions: Vec<MenuItem> = TYPE_NAMES
            .iter()
            .map(|name| {
                MenuItem::action(format!("New {name}"))
                    .icon(Icon::Database)
                    .id(format!("open:resources/{name}/new"))
            })
            .collect();
        actions.push(
            MenuItem::action("Settings")
                .icon(Icon::Settings)
                .keywords(["preferences", "theme"].map(str::to_owned))
                .id("do:settings"),
        );
        groups.push(CommandGroup::new("Actions", actions));
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
            } else if command == "do:settings" {
                self.open_settings("appearance");
            } else if let Some(place) = place_of(command) {
                if let Some((_, href)) = hrefs.iter().find(|(id, _)| id == place) {
                    let href = href.clone();
                    self.go(&href);
                }
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
            SettingsSection::new("editors", "Editors", None),
            SettingsSection::new("tools", "FHIRPath and search", None),
            SettingsSection::new("demo", "Demo server", None).reset(false),
        ];
        let prefs = self.app.prefs();
        let mut line_numbers = prefs.line_numbers;
        let mut wrap = prefs.wrap;
        let mut size = usize::from(prefs.size == "md");
        let mut live = prefs.live;
        let sizes = ["10", "20", "50"];
        let mut page_size = sizes
            .iter()
            .position(|one| *one == crate::js::number_to_string(prefs.page_size))
            .unwrap_or(1);
        let mut ask_reset = false;
        let appearance = &mut self.appearance;
        let outcome = SettingsDialog::new("settings", &sections, &mut active).show(ctx, |section, ui| match section {
            "editors" => {
                SettingRow::new(LINE_NUMBERS.0, LINE_NUMBERS.1).show(ui, |ui| {
                    ui.add(Switch::new(&mut line_numbers, LINE_NUMBERS.0));
                });
                SettingRow::new(WRAP.0, WRAP.1).show(ui, |ui| {
                    ui.add(Switch::new(&mut wrap, WRAP.0));
                });
                SettingRow::new(TEXT_SIZE.0, TEXT_SIZE.1).show(ui, |ui| {
                    ui.add(Segmented::new(&mut size, ["Small", "Regular"]));
                });
            }
            "tools" => {
                SettingRow::new(LIVE.0, LIVE.1).show(ui, |ui| {
                    ui.add(Switch::new(&mut live, LIVE.0));
                });
                SettingRow::new(PAGE_SIZE.0, PAGE_SIZE.1).show(ui, |ui| {
                    ui.add(Segmented::new(&mut page_size, sizes));
                });
            }
            "demo" => {
                SettingRow::new(RESET.0, RESET.1).show(ui, |ui| {
                    let reset = Button::new("Reset")
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm);
                    ask_reset = ui.add(reset).clicked();
                });
            }
            _ => theme_settings_listed(ui, appearance, &CARRIED, true, &ThemeWords::default()),
        });
        let app = &mut self.app;
        if line_numbers != prefs.line_numbers {
            app.set_pref("line-numbers", &json!(line_numbers));
        }
        if wrap != prefs.wrap {
            app.set_pref("wrap", &json!(wrap));
        }
        let size_key = if size == 1 { "md" } else { "sm" };
        if size_key != prefs.size {
            app.set_pref("size", &json!(size_key));
        }
        if live != prefs.live {
            app.set_pref("live", &json!(live));
        }
        if sizes[page_size] != crate::js::number_to_string(prefs.page_size) {
            app.set_pref("page-size", &json!(sizes[page_size]));
        }
        if ask_reset {
            self.asking = Some(Asking::Reset);
        }
        if outcome.reset {
            let defaults = crate::ui::Prefs::default();
            match active.as_str() {
                "appearance" => self.appearance = Appearance::default(),
                "editors" => {
                    self.app.set_pref("line-numbers", &json!(defaults.line_numbers));
                    self.app.set_pref("wrap", &json!(defaults.wrap));
                    self.app.set_pref("size", &json!(defaults.size));
                }
                "tools" => {
                    self.app.set_pref("live", &json!(defaults.live));
                    self.app.set_pref(
                        "page-size",
                        &Value::String(crate::js::number_to_string(defaults.page_size)),
                    );
                }
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
            Asking::DeleteResource(kind, id) => (
                format!("Delete {kind}/{id}?"),
                "The server refuses while other resources point at it. Its versions stay in the history.",
                "Delete",
            ),
            Asking::Reset => ("Reset the demo data?".to_owned(), RESET.1, "Reset"),
        };
        let answer = ConfirmDialog::new(("question", format!("{asking:?}")), &question, label)
            .detail(detail)
            .destructive(true)
            .show(ctx);
        match answer {
            Some(ConfirmAnswer::Confirm) => {
                self.asking = None;
                match asking {
                    Asking::DeleteResource(kind, id) => crate::pages::resource_remove(&mut self.app, &kind, &id),
                    Asking::Reset => self.reset_demo(),
                }
            }
            Some(_) => self.asking = None,
            None => {}
        }
    }

    /// Everything as the demo started: the server seeded again, the edits dropped, the page loaded
    /// again where it was.
    pub fn reset_demo(&mut self) {
        let hash = self.app.hash.clone();
        let kept = std::mem::take(&mut self.app.kept);
        self.app = App::new(self.now());
        self.app.kept = kept;
        self.app.go(&hash);
        self.views = Views::default();
        self.settings = None;
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
        let backend = &self.app.backend;
        let kept = Kept {
            resources: backend.resources.rows.clone(),
            history: backend.history.rows.clone(),
            saved: backend.saved.rows.clone(),
            kept: self.app.kept.clone(),
            drafts: self.app.editing.drafts.clone(),
        };
        eframe::set_value(storage, "fhir", &kept);
    }
}
