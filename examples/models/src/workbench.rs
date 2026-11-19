//! The app inside the shell, as the web's `ModelsApp.tsx` draws it: the rail with the pulls under
//! way and the models in memory counted, the top bar's trail, search and the runner's menu, the page
//! the address names (the chat with its settings panel beside it), the status line, and the
//! palette, Settings and the questions over them. The clock is the app's: the backend's answers,
//! the pulls and the answers being written move on as it does.

use std::collections::BTreeMap;

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, AppearanceCommandWords, Button, ButtonSize, ButtonVariant, CommandGroup,
    CommandPalette, ConfirmAnswer, ConfirmDialog, Consequence, ConsequenceList, ConsequenceListWords, Filter, Icon,
    Input, MenuItem, PaletteWords, Rail, RailEntry, RailGroup, Segmented, ServerMenu, ServerMenuWords, ServerState,
    SettingRow, SettingsDialog, SettingsSection, Shell, ShellPart, SliderField, StatusLine, StatusTone, Switch,
    TableState, ThemeWords, Toast, Toaster, TopBar, appearance_commands, install, place_commands, place_of,
    run_appearance_command, theme_settings_listed, toast_acted,
};

use crate::actions::{delete_dialog, deleted_note};
use crate::app::{actions, default_model, label, palette_conversations, probe, rail, server_menu, status_line, trail};
use crate::data::backend::Backend;
use crate::data::format::bytes;
use crate::data::model::{Model, Prefs, PrefsPatch, ReplyState, default_prefs};
use crate::pages::compare::Draft;
use crate::pages::settings::{demo, moved, reset, root_applies, sections};
use crate::runtime::{Host, Outcome, Promise};
use crate::screens::Act;
use crate::themes::CARRIED;

/// 2026-11-18 09:00 UTC: the moment the app's screenshots are taken at, as the web's are shot.
pub const DEMO_START: i64 = 1_794_992_400_000;

/// The app's name, over the rail and in the palette's foot.
pub const NAME: &str = "Models";

/// Who uses the app, and where.
pub const ME: &str = "ada";
pub const WHERE: &str = "tidevann.example.org";

/// The palette's placeholder.
pub const PALETTE_PLACEHOLDER: &str = "Go to a page, open a model or a conversation";

/// The runner's name and address, as its menu says them.
pub const RUNNER_NAME: &str = "Local runner";

/// The Tidevann mark the rail heads with.
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// The question asked before the demo data is put back.
pub const RESET_QUESTION: (&str, &str) = (
    "Reset the demo data?",
    "Models pulled, conversations and comparisons made since are gone, and the page loads again.",
);

/// A lucide icon by its name as the web writes it, `MessageSquare`.
pub fn icon_of(page: &str) -> Icon {
    match page {
        "library" => Icon::Boxes,
        "disk" => Icon::HardDrive,
        "discover" => Icon::Search,
        "downloads" => Icon::ArrowDownToLine,
        "chat" => Icon::MessageSquare,
        "compare" => Icon::Columns3,
        "runner" => Icon::Cpu,
        _ => Icon::Settings,
    }
}

/// What a page holds while it is shown, beyond what the backend keeps: its tables, what is typed
/// and chosen on it; each starts afresh on another page, as the web's pages mount anew.
#[derive(Default)]
pub struct Views {
    /// The page's main table, and a second one (a sheet's variants, the loaded models).
    pub table: TableState,
    pub second: TableState,
    /// The library's saved view chosen, its filters and what is searched.
    pub view: Option<String>,
    pub filters: Vec<Filter>,
    pub query: String,
    /// The system prompt typed on a model's page, kept once the field is left.
    pub system: Option<String>,
    /// The max tokens typed in a field, as a number.
    pub max_tokens: Option<f64>,
    /// Discover's results by the entries' ids, none while a search is out, the search that gave
    /// them, and when the one out answers.
    pub results: Option<Vec<String>>,
    pub searched: Option<(String, String, String)>,
    pub search_due: Option<i64>,
    /// The entry whose sheet is open.
    pub open: Option<String>,
    /// The runner's model to load, and the log's source and search.
    pub pick: Option<String>,
    pub log_source: usize,
    pub log_query: String,
    /// The library root typed in Settings.
    pub root: Option<String>,
    /// The chat's files waiting to go, and the history's sheet.
    pub files: Vec<crate::data::model::Attachment>,
}

/// A question asked in a dialog before it is done.
#[derive(Clone, Debug, PartialEq)]
pub enum Asking {
    /// Models to delete, and where the app goes once they are gone.
    Delete(Vec<Model>, AfterDelete),
    Reset,
}

/// What follows a delete: nothing, the library's selection cleared, or the library opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterDelete {
    Stay,
    ClearSelection,
    Library,
}

/// What the app keeps in the browser beside the backend's rows, as the web's `useStored` keys.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Kept {
    /// `chat.panel`: the chat's settings panel open.
    pub chat_panel: bool,
    /// `chat.history`: the conversations beside the chat.
    pub chat_history: bool,
    /// `chat.drafts`: what is typed in each conversation's composer.
    pub drafts: BTreeMap<String, String>,
    /// `compare.draft`.
    pub compare: Draft,
    /// `discover.query`, `discover.source` and `discover.format`.
    pub discover_query: String,
    pub discover_source: String,
    pub discover_format: String,
}

impl Default for Kept {
    fn default() -> Self {
        Kept {
            chat_panel: false,
            chat_history: true,
            drafts: BTreeMap::new(),
            compare: Draft::default(),
            discover_query: String::new(),
            discover_source: "all".to_owned(),
            discover_format: "all".to_owned(),
        }
    }
}

/// The backend's rows eframe's storage keeps between visits.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Stored {
    pub models: Vec<Model>,
    pub conversations: Vec<crate::data::model::Conversation>,
    pub comparisons: Vec<crate::data::model::Comparison>,
    pub prefs: Vec<Prefs>,
    pub views: Vec<crate::data::backend::LibraryView>,
    pub kept: Kept,
}

/// Something waited on: a promise, and what the app does with its answer.
type Then = Box<dyn FnOnce(&mut ModelsApp, Outcome)>;

pub struct ModelsApp {
    pub appearance: Appearance,
    /// The page's backend: its collections, its clock and the engines.
    pub backend: Backend,
    /// The address, as the web's hash route without `#/`.
    pub route: String,
    /// Settings over the page while they are open, the section shown and what is searched.
    pub settings: Option<String>,
    pub settings_search: String,
    /// The palette while it is open: what is typed and the row lit.
    pub palette: Option<(String, usize)>,
    pub views: Views,
    /// The page the views were made for.
    views_for: String,
    pub kept: Kept,
    pub asking: Option<Asking>,
    /// A delete or a reset under way, as the dialog's busy button says.
    pub busy: bool,
    /// The runner's menu asks it again until then.
    checking: Option<i64>,
    /// The promises waited on, and the toasts said since the last frame.
    waiting: Vec<(Promise, Then)>,
    pub notes: Vec<Toast>,
    /// The toasts whose action opens a model, by the toast's id.
    opens: Vec<(u64, String)>,
    started: i64,
    started_at: Option<f64>,
    elapsed: i64,
    held: bool,
    pub desktop: bool,
}

impl ModelsApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_millis() as i64);
        let appearance = cc
            .storage
            .and_then(|s| eframe::get_value(s, "appearance"))
            .unwrap_or_default();
        let mut app = Self::at(now, appearance);
        if let Some(stored) = cc.storage.and_then(|s| eframe::get_value::<Stored>(s, "models")) {
            app.restore(stored);
        }
        app.held = false;
        app.desktop = true;
        app
    }

    /// The app at a moment with the data as the demo starts, its clock held there, as the tests and
    /// the screenshots take it.
    pub fn at(now: i64, appearance: Appearance) -> Self {
        let mut backend = Backend::new(now);
        // the pages' own collections, listed as the app's provider mounts
        backend.list(|b| &mut b.events);
        backend.list(|b| &mut b.views);
        Self {
            appearance,
            backend,
            route: "library".to_owned(),
            settings: None,
            settings_search: String::new(),
            palette: None,
            views: Views {
                view: Some("all".to_owned()),
                ..Views::default()
            },
            views_for: "library".to_owned(),
            kept: Kept::default(),
            asking: None,
            busy: false,
            checking: None,
            waiting: Vec::new(),
            notes: Vec::new(),
            opens: Vec::new(),
            started: now,
            started_at: None,
            elapsed: 0,
            held: true,
            desktop: false,
        }
    }

    /// The rows kept from a visit before, put back as the backend kept them; an answer cut off as
    /// the app closed is kept as stopped.
    fn restore(&mut self, stored: Stored) {
        let backend = &mut self.backend;
        let mut conversations = stored.conversations;
        for talk in &mut conversations {
            for message in &mut talk.messages {
                if let crate::data::model::Message::Assistant(answer) = message {
                    for version in &mut answer.versions {
                        if version.state == ReplyState::Streaming {
                            version.state = ReplyState::Stopped;
                        }
                    }
                }
            }
        }
        backend.models.rows = stored.models.clone();
        backend.models.seen = stored.models;
        backend.conversations.rows = conversations.clone();
        backend.conversations.seen = conversations;
        backend.comparisons.rows = stored.comparisons.clone();
        backend.comparisons.seen = stored.comparisons;
        if !stored.prefs.is_empty() {
            backend.prefs.rows = stored.prefs.clone();
            backend.prefs.seen = stored.prefs;
        }
        backend.views.rows = stored.views.clone();
        backend.views.seen = stored.views;
        self.kept = stored.kept;
    }

    pub fn now(&self) -> i64 {
        self.started + self.elapsed
    }

    /// Moves a held clock on, as the tests wait for the backend's answers.
    pub fn wait(&mut self, ms: i64) {
        self.elapsed += ms;
        self.tick();
    }

    /// The backend moved on to the clock, and what it said since heard.
    fn tick(&mut self) {
        let now = self.now();
        self.backend.advance(now);
        self.hear();
    }

    /// The preferences as the pages read them.
    pub fn prefs(&self) -> Prefs {
        self.backend.prefs.seen.first().cloned().unwrap_or_else(default_prefs)
    }

    /// The routes the engines went to, the pulls that landed, and the promises that settled.
    fn hear(&mut self) {
        for route in std::mem::take(&mut self.backend.went) {
            self.go(&route);
        }
        for pulled in std::mem::take(&mut self.backend.pulled) {
            let id = crate::data::catalog::model_id(&pulled.name, &pulled.quant);
            let toast = Toast::success(format!("{} is in the library", pulled.name))
                .description(format!("{}, {}", pulled.quant, bytes(pulled.size)))
                .action("Open")
                .timeout(Some(4.0));
            self.notes.push(toast);
            self.opens.push((u64::MAX, format!("library/{id}")));
        }
        loop {
            let at = self
                .waiting
                .iter()
                .position(|(promise, _)| self.backend.runtime.settled(*promise).is_some());
            let Some(at) = at else { break };
            let (promise, then) = self.waiting.remove(at);
            let outcome = self
                .backend
                .runtime
                .settled(promise)
                .cloned()
                .unwrap_or(Ok(crate::runtime::unit()));
            then(self, outcome);
        }
    }

    /// Waits on a promise of the backend's, and does `then` with its answer.
    pub fn then(&mut self, promise: Promise, then: impl FnOnce(&mut ModelsApp, Outcome) + 'static) {
        self.waiting.push((promise, Box::new(then)));
    }

    /// A refusal said as a toast, as the pages' `catch` says it.
    pub fn failed(&mut self, promise: Promise, title: String) {
        self.then(promise, move |app, outcome| {
            if let Err(why) = outcome {
                app.notes.push(Toast::error(title).description(why).timeout(Some(4.0)));
            }
        });
    }

    /// The route split at its slashes: the page and what it names.
    pub fn parts(&self) -> (String, Option<String>) {
        let mut parts = self.route.splitn(2, '/');
        let first = parts.next().unwrap_or("").to_owned();
        let first = if first.is_empty() { "library".to_owned() } else { first };
        (first, parts.next().filter(|rest| !rest.is_empty()).map(str::to_owned))
    }

    /// Goes to an address, as the web's hash routes: `library[/<id>]`, `disk`, `discover`,
    /// `downloads`, `chat[/<id>]`, `compare[/<id>|new]`, `runner`; `settings` opens over the page,
    /// which stays.
    pub fn go(&mut self, path: &str) {
        let path = path.trim_start_matches('#').trim_matches('/');
        if path == "settings" || path.starts_with("settings/") {
            self.settings = Some(path.strip_prefix("settings/").unwrap_or("library").to_owned());
            return;
        }
        self.route = path.to_owned();
        self.settings = None;
        self.follow_page();
    }

    /// The views made anew for a page mounted anew, as the web's pages mount on a new address.
    fn follow_page(&mut self) {
        if self.views_for != self.route {
            self.views_for = self.route.clone();
            self.views = Views {
                view: Some("all".to_owned()),
                ..Views::default()
            };
        }
    }

    /// Asks before models are deleted, unless the preferences say not to.
    pub fn ask_delete(&mut self, gone: Vec<Model>, after: AfterDelete) {
        if gone.is_empty() {
            return;
        }
        if self.prefs().confirm_delete {
            self.asking = Some(Asking::Delete(gone, after));
        } else {
            self.delete(gone, after);
        }
    }

    /// Deletes models, says so once they are gone, and does what follows.
    fn delete(&mut self, gone: Vec<Model>, after: AfterDelete) {
        self.busy = true;
        let all = self.backend.models.seen.clone();
        let prefs = self.prefs();
        let promise = self.backend.delete_models(gone.clone(), all, &prefs.default_model);
        self.then(promise, move |app, outcome| {
            app.busy = false;
            match outcome {
                Ok(_) => {
                    app.asking = None;
                    let (title, description) = deleted_note(&gone);
                    app.notes
                        .push(Toast::success(title).description(description).timeout(Some(4.0)));
                    match after {
                        AfterDelete::Stay => {}
                        AfterDelete::ClearSelection => app.views.table.selected.clear(),
                        AfterDelete::Library => app.go("library"),
                    }
                }
                Err(why) => app
                    .notes
                    .push(Toast::error("Could not delete").description(why).timeout(Some(4.0))),
            }
        });
    }

    /// Opens a new conversation with a model and goes to it.
    pub fn chat_with(&mut self, model: &Model) {
        self.backend.chat_with(model);
    }

    /// Loads a model in the runner, saying why it could not.
    pub fn load(&mut self, model: &Model) {
        let promise = self.backend.ensure_loaded(model, false);
        self.failed(promise, format!("Could not load {}", model.name));
    }

    pub fn unload(&mut self, id: &str) {
        self.backend.unload_model(id, "by hand");
    }

    /// Starts the runner while it is down, and stops it while it is up.
    pub fn toggle_runner(&mut self) {
        if self.backend.runner_up() {
            self.backend.stop_runner();
        } else {
            self.backend.start_runner();
        }
    }

    /// Changes the preferences, saying why it could not.
    pub fn save_prefs(&mut self, patch: PrefsPatch) {
        let promise = self
            .backend
            .update(|b| &mut b.prefs, "prefs", move |row| patch.apply(row));
        self.failed(promise, "Could not save".to_owned());
    }

    /// The rail's groups, and the place shown marked.
    pub fn rail(&self) -> (Vec<RailGroup>, String) {
        let (page, _) = self.parts();
        let page = if crate::app::PAGES.contains(&page.as_str()) {
            page
        } else {
            String::new()
        };
        let mut active = String::new();
        let groups = rail(&page, &self.backend.downloads.seen, &self.backend.loaded.seen)
            .into_iter()
            .map(|(_, title, entries)| {
                let entries: Vec<RailEntry> = entries
                    .iter()
                    .map(|entry| {
                        if entry.active {
                            active = entry.id.to_owned();
                        }
                        let mut one = RailEntry::new(entry.id, entry.label, icon_of(entry.id));
                        if let Some(count) = entry.count {
                            one = one.count(count as u32);
                        }
                        one
                    })
                    .collect();
                RailGroup::new(Some(title), entries)
            })
            .collect();
        (groups, active)
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        let ctx = ui.ctx().clone();
        if !self.held {
            let time = ctx.input(|i| i.time);
            let since = *self.started_at.get_or_insert(time);
            self.elapsed = ((time - since) * 1000.0) as i64;
            let busy = !self.backend.live.is_empty() || self.backend.downloads.seen.iter().any(|one| one.state.going());
            ctx.request_repaint_after(std::time::Duration::from_millis(if busy { 33 } else { 1000 }));
        }
        self.tick();
        if self.checking.is_some_and(|until| self.now() >= until) {
            self.checking = None;
            let (level, line) = probe(self.backend.runner.seen.first());
            self.backend.log(level, crate::data::model::LogSource::Runner, line);
        }
        self.say(&ctx);

        let now = self.now();
        let (page, second) = self.parts();
        let (groups, mut active) = self.rail();
        let was = active.clone();
        let route: Vec<&str> = self.route.split('/').collect();
        let crumbs = trail(&route, &self.backend.models.seen, &self.backend.conversations.seen);
        let words: Vec<&str> = crumbs.iter().map(|one| one.0.as_str()).collect();
        let runner = self.backend.runner.seen.first().cloned();
        let menu = server_menu(runner.as_ref(), &self.backend.loaded.seen, now);
        let foot = status_line(
            runner.as_ref(),
            &self.backend.downloads.seen,
            &self.backend.loaded.seen,
            &self.backend.models.seen,
            now,
        );
        let talk = if page == "chat" {
            second
                .as_ref()
                .and_then(|id| self.backend.conversations.seen.iter().find(|one| one.id == *id))
                .cloned()
        } else {
            None
        };
        let state = if self.checking.is_some() {
            ServerState::Checking
        } else {
            match menu.state {
                "up" => ServerState::Up,
                "down" => ServerState::Down,
                _ => ServerState::Checking,
            }
        };
        let facts: Vec<&str> = menu.facts.iter().map(String::as_str).collect();
        let items = [
            MenuItem::action(menu.toggle)
                .icon(if menu.toggle == "Stop the runner" {
                    Icon::PowerOff
                } else {
                    Icon::Power
                })
                .disabled(!menu.toggle_on),
            MenuItem::action("Open the runner").icon(Icon::SquareTerminal),
        ];
        let server_words = ServerMenuWords {
            up: "Running".into(),
            down: "Stopped".into(),
            checking: menu.checking.into(),
            label: "Runner".into(),
            ..ServerMenuWords::default()
        };
        let version = runner.as_ref().map(|one| one.version.clone());

        let mut account = None;
        let mut searched = sokkel_egui::palette_shortcut(&ctx);
        let mut crumb = None;
        let mut server = None;
        let mut acts: Vec<Act> = Vec::new();
        let mut panel_acts: Vec<Act> = Vec::new();
        let mut panel_open = talk.is_some() && self.kept.chat_panel;
        let mut views = std::mem::take(&mut self.views);
        {
            let app = &*self;
            let mut shell = Shell::new();
            if self.desktop {
                shell = shell.desktop(sokkel_egui::DesktopWindow::native());
            }
            if let Some(talk) = &talk {
                let panel_acts = &mut panel_acts;
                shell = shell.panel(&mut panel_open, move |ui| {
                    crate::talk::settings_panel(ui, app, talk, panel_acts);
                });
            }
            shell.show_window(ui, |part, ui| match part {
                ShellPart::Rail => {
                    ui.add(Rail::new(NAME, &groups, &mut active).mark(TIDEVANN_MARK));
                }
                ShellPart::RailFoot => {
                    account = AccountMenu::new(ME).place(WHERE).local(true).appearance(true).show(ui);
                }
                ShellPart::Top => {
                    let mut bar = TopBar::new(&words).linked(true);
                    for (at, one) in crumbs.iter().enumerate() {
                        if one.2 {
                            bar = bar.code(at);
                        }
                    }
                    let out = bar.search("Search", "\u{2318}K").show(ui, |ui| {
                        server = Some(
                            ServerMenu::new(RUNNER_NAME, state)
                                .version(version.as_deref())
                                .facts(&facts)
                                .check(true)
                                .items(&items)
                                .words(server_words.clone())
                                .show(ui),
                        );
                    });
                    searched |= out.search;
                    crumb = out.crumb;
                }
                ShellPart::Page => crate::screens::page(ui, app, &page, second.as_deref(), &mut views, &mut acts),
                ShellPart::Foot => {
                    let mut line = StatusLine::new(foot.note.clone()).live(foot.live);
                    if foot.warn {
                        line = line.tone(StatusTone::Warn);
                    }
                    ui.add(line.item("loaded", &foot.loaded).item("on disk", &foot.on_disk));
                }
            });
        }
        self.views = views;
        if talk.is_some() {
            self.kept.chat_panel = panel_open;
        }
        for act in acts.into_iter().chain(panel_acts) {
            self.apply(act);
        }
        if active != was {
            let target = active.clone();
            self.go(&target);
        }
        if let Some(index) = crumb
            && let Some(href) = crumbs.get(index).and_then(|one| one.1.clone())
        {
            self.go(&href);
        }
        if let Some(server) = server {
            if server.check {
                self.checking = Some(self.now() + 600);
            }
            match server.item {
                Some(0) => self.toggle_runner(),
                Some(1) => self.go("runner"),
                _ => {}
            }
        }
        match account {
            Some(AccountAction::Settings) => self.go("settings"),
            Some(AccountAction::Appearance) => self.go("settings/appearance"),
            None => {}
        }
        if searched {
            self.palette = Some(Default::default());
        }
        self.search_palette(&ctx, &groups);
        self.settings_dialog(&ctx);
        self.questions(&ctx);
        self.hear();
        self.say(&ctx);
        for acted in toast_acted(&ctx) {
            if let Some(at) = self.opens.iter().position(|(id, _)| *id == acted) {
                let (_, route) = self.opens.remove(at);
                self.go(&route);
            }
        }
        Toaster::new().show(&ctx);
    }

    /// The toasts said since the last frame.
    fn say(&mut self, ctx: &egui::Context) {
        for toast in std::mem::take(&mut self.notes) {
            let opens = toast.action.is_some();
            let id = toast.show(ctx);
            if opens && let Some(one) = self.opens.iter_mut().find(|(at, _)| *at == u64::MAX) {
                one.0 = id;
            }
        }
    }

    /// The palette while it is open: the pages, the app's actions, the models, a chat with each,
    /// the conversations, and the look.
    fn search_palette(&mut self, ctx: &egui::Context, rail_groups: &[RailGroup]) {
        let Some((mut query, mut highlight)) = self.palette.take() else {
            return;
        };
        let models = self.backend.models.seen.clone();
        let prefs = self.prefs();
        let mut groups: Vec<CommandGroup> = place_commands(rail_groups, "Pages").into_iter().collect();
        let icon = |id: &str| match id {
            "new-chat" => Icon::MessageSquarePlus,
            "pull" => Icon::ArrowDownToLine,
            "compare" => Icon::Columns3,
            "runner-stop" => Icon::PowerOff,
            "runner-start" => Icon::Power,
            _ => Icon::Trash2,
        };
        groups.push(CommandGroup::new(
            "Actions",
            actions(
                self.backend.runner.seen.first(),
                &self.backend.downloads.seen,
                &models,
                &prefs,
            )
            .into_iter()
            .map(|(id, label, hint)| {
                let mut item = MenuItem::action(label).icon(icon(id)).id(format!("do:{id}"));
                if let Some(hint) = hint {
                    item = item.hint(hint);
                }
                match id {
                    "pull" => item.keywords(["download", "discover"]),
                    "compare" => item.keywords(["side by side"]),
                    _ => item,
                }
            }),
        ));
        groups.push(CommandGroup::new(
            "Models",
            models.iter().map(|model| {
                let mut words = vec![
                    model.family.clone(),
                    model.format.key().to_owned(),
                    model.publisher.clone(),
                ];
                words.extend(model.tags.iter().cloned());
                MenuItem::action(&model.name)
                    .icon(Icon::Boxes)
                    .detail(&model.quant)
                    .hint(bytes(model.size))
                    .keywords(words)
                    .id(format!("open:library/{}", model.id))
            }),
        ));
        groups.push(CommandGroup::new(
            "Chat with",
            models.iter().map(|model| {
                MenuItem::action(format!("Chat with {}", model.name))
                    .icon(Icon::MessageSquarePlus)
                    .hint(&model.quant)
                    .keywords(["new chat".to_owned(), model.family.clone()])
                    .id(format!("chat:{}", model.id))
            }),
        ));
        groups.push(CommandGroup::new(
            "Conversations",
            palette_conversations(&self.backend.conversations.seen, &models)
                .into_iter()
                .map(|(id, title, model)| {
                    let id = id.trim_start_matches("conversation-").to_owned();
                    MenuItem::action(title)
                        .icon(Icon::MessageSquare)
                        .hint(model)
                        .id(format!("open:chat/{id}"))
                }),
        ));
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
            } else if let Some(id) = command.strip_prefix("chat:") {
                if let Some(model) = models.iter().find(|one| one.id == id) {
                    self.chat_with(model);
                }
            } else if let Some(action) = command.strip_prefix("do:") {
                match action {
                    "new-chat" => {
                        if let Some(model) = default_model(&models, &prefs).cloned() {
                            self.chat_with(&model);
                        }
                    }
                    "pull" => self.go("discover"),
                    "compare" => self.go("compare/new"),
                    "runner-stop" | "runner-start" => self.toggle_runner(),
                    _ => {
                        self.backend.clear_ended();
                    }
                }
            } else if let Some(place) = place_of(command) {
                let place = place.to_owned();
                self.go(&place);
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
        let prefs = self.prefs();
        let said = sections(&prefs);
        let icon = |id: &str| match id {
            "library" => Icon::FolderOpen,
            "models" => Icon::Boxes,
            "downloads" => Icon::ArrowDownToLine,
            _ => Icon::Cpu,
        };
        let mut list: Vec<SettingsSection> = said
            .iter()
            .map(|one| SettingsSection::new(one.id, one.label, icon(one.id)).group("Models"))
            .collect();
        list.push(SettingsSection::new("appearance", "Appearance", Icon::Palette).group("This app"));
        let demo = demo();
        list.push(
            SettingsSection::new(demo.id, demo.label, Icon::Database)
                .group("This app")
                .reset(false),
        );
        let models = self.backend.models.seen.clone();
        let mut root = self.views.root.clone().unwrap_or_else(|| prefs.root.clone());
        let mut patch = PrefsPatch::default();
        let mut moved_to = None;
        let mut ask_reset = false;
        let appearance = &mut self.appearance;
        let setting = |section: &str, id: &str| {
            said.iter()
                .find(|one| one.id == section)
                .and_then(|one| one.settings.iter().find(|setting| setting.id == id))
                .map(|one| (one.title, one.description.clone()))
                .unwrap_or_default()
        };
        let outcome = SettingsDialog::new("settings", &list, &mut active).show(ctx, |section, ui| match section {
            "library" => {
                let (title, description) = setting("library", "root");
                SettingRow::new(title, &description)
                    .keywords(&["folder", "path", "disk", "storage"])
                    .below()
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            let valid = crate::data::disk::valid_root(&root);
                            let field = ui.add(
                                Input::new(&mut root)
                                    .width(256.0)
                                    .mono(true)
                                    .invalid(!valid)
                                    .name(title),
                            );
                            let applies = root_applies(&root, &prefs.root);
                            let press = ui.add_enabled(
                                applies,
                                Button::new("Move").variant(ButtonVariant::Outline).size(ButtonSize::Sm),
                            );
                            let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            if applies && (press.clicked() || entered) {
                                moved_to = Some(crate::js::trim(&root).to_owned());
                            }
                        });
                    });
                let (title, description) = setting("library", "confirm-delete");
                SettingRow::new(title, &description).show(ui, |ui| {
                    let mut on = prefs.confirm_delete;
                    if ui.add(Switch::new(&mut on, title)).changed() {
                        patch.confirm_delete = Some(on);
                    }
                });
            }
            "models" => {
                let (title, description) = setting("models", "default-model");
                SettingRow::new(title, &description)
                    .keywords(&["model", "new chat"])
                    .show(ui, |ui| {
                        if let Some(id) =
                            crate::talk::model_select(ui, "Default model", &models, &prefs.default_model, false)
                        {
                            patch.default_model = Some(id);
                        }
                    });
            }
            "downloads" => {
                let (title, description) = setting("downloads", "parallel");
                SettingRow::new(title, &description).show(ui, |ui| {
                    let options = ["1", "2", "3", "4"];
                    let was = options
                        .iter()
                        .position(|one| *one == crate::js::number_to_string(prefs.parallel))
                        .unwrap_or(1);
                    let mut at = was;
                    ui.add(Segmented::new(&mut at, options));
                    if at != was {
                        patch.parallel = Some((at + 1) as f64);
                    }
                });
                let (title, description) = setting("downloads", "bandwidth");
                SettingRow::new(title, &description)
                    .keywords(&["speed", "network"])
                    .show(ui, |ui| {
                        let values = [0.0, 25.0, 50.0, 100.0];
                        let was = values.iter().position(|one| *one == prefs.bandwidth).unwrap_or(0);
                        let mut at = was;
                        ui.add(Segmented::new(&mut at, ["None", "25 MB/s", "50 MB/s", "100 MB/s"]));
                        if at != was {
                            patch.bandwidth = Some(values[at]);
                        }
                    });
            }
            "runner" => {
                let (title, description) = setting("runner", "memory");
                SettingRow::new(title, &description)
                    .keywords(&["ram", "limit"])
                    .below()
                    .show(ui, |ui| {
                        let mut memory = prefs.memory;
                        ui.add(SliderField::new(title, &mut memory, 32.0).min(4.0).step(1.0).unit("GB"));
                        if memory != prefs.memory {
                            patch.memory = Some(memory);
                        }
                    });
                let (title, description) = setting("runner", "keep-alive");
                SettingRow::new(title, &description)
                    .keywords(&["keep-alive", "unload", "idle"])
                    .show(ui, |ui| {
                        let values = [5.0, 10.0, 30.0, 0.0];
                        let was = values.iter().position(|one| *one == prefs.keep_alive).unwrap_or(1);
                        let mut at = was;
                        ui.add(Segmented::new(&mut at, ["5 min", "10 min", "30 min", "Always"]));
                        if at != was {
                            patch.keep_alive = Some(values[at]);
                        }
                    });
                let (title, description) = setting("runner", "start-runner");
                SettingRow::new(title, &description).show(ui, |ui| {
                    let mut on = prefs.start_runner;
                    if ui.add(Switch::new(&mut on, title)).changed() {
                        patch.start_runner = Some(on);
                    }
                });
                let (title, description) = setting("runner", "auto-load");
                SettingRow::new(title, &description)
                    .keywords(&["on demand"])
                    .show(ui, |ui| {
                        let mut on = prefs.auto_load;
                        if ui.add(Switch::new(&mut on, title)).changed() {
                            patch.auto_load = Some(on);
                        }
                    });
            }
            "demo" => {
                let setting = &demo.settings[0];
                SettingRow::new(setting.title, &setting.description).show(ui, |ui| {
                    let reset = Button::new("Reset")
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm);
                    ask_reset = ui.add(reset).clicked();
                });
            }
            _ => theme_settings_listed(ui, appearance, &CARRIED, true, &ThemeWords::default()),
        });
        self.views.root = (root != prefs.root).then_some(root);
        if let Some(root) = moved_to {
            patch.root = Some(root.clone());
            let (line, description) = moved(&root, models.len());
            self.backend.log(
                crate::data::model::Level::Info,
                crate::data::model::LogSource::Library,
                line,
            );
            self.notes.push(
                Toast::success("Library moved")
                    .description(description)
                    .timeout(Some(4.0)),
            );
            self.views.root = None;
        }
        if patch != PrefsPatch::default() {
            self.save_prefs(patch);
        }
        if ask_reset {
            self.asking = Some(Asking::Reset);
        }
        if outcome.reset {
            if active == "appearance" {
                self.appearance = Appearance::default();
            } else if let Some(patch) = reset(&active) {
                self.save_prefs(patch);
                self.views.root = None;
            }
        }
        if !outcome.close {
            self.settings = Some(active);
        }
    }

    /// The question asked before something that cannot be undone, while it is asked.
    fn questions(&mut self, ctx: &egui::Context) {
        let Some(asking) = self.asking.clone() else { return };
        match &asking {
            Asking::Delete(doomed, after) => {
                let prefs = self.prefs();
                let said = delete_dialog(doomed, &self.backend.conversations.seen, &prefs.root);
                let mut items = vec![
                    Consequence::new("Space freed")
                        .detail(&said.freed)
                        .icon(Icon::HardDrive),
                    Consequence::new("Files removed").count(said.files).icon(Icon::FileText),
                ];
                if let Some((label, count)) = &said.talks {
                    items.push(
                        Consequence::new(label)
                            .count(*count)
                            .detail("They keep their messages and ask for another model.")
                            .icon(Icon::MessageSquare),
                    );
                }
                let words = ConsequenceListWords {
                    heading: "What this does".to_owned(),
                    ..ConsequenceListWords::default()
                };
                let answer = ConfirmDialog::new("delete-models", &said.question, "Delete")
                    .detail(&said.detail)
                    .destructive(true)
                    .busy(self.busy.then(|| "Deleting".to_owned()))
                    .show_with(ctx, |ui| {
                        ConsequenceList::new(&items).words(words.clone()).show(ui);
                    });
                match answer {
                    Some(ConfirmAnswer::Confirm) => {
                        let (doomed, after) = (doomed.clone(), *after);
                        self.delete(doomed, after);
                    }
                    Some(_) if !self.busy => self.asking = None,
                    _ => {}
                }
            }
            Asking::Reset => {
                let answer = ConfirmDialog::new("reset-demo", RESET_QUESTION.0, "Reset")
                    .detail(RESET_QUESTION.1)
                    .destructive(true)
                    .show(ctx);
                match answer {
                    Some(ConfirmAnswer::Confirm) => {
                        self.asking = None;
                        self.reset_demo();
                    }
                    Some(_) => self.asking = None,
                    None => {}
                }
            }
        }
    }

    /// Everything as the demo started, the page loaded again where it was.
    pub fn reset_demo(&mut self) {
        let route = self.route.clone();
        let appearance = self.appearance.clone();
        let desktop = self.desktop;
        let held = self.held;
        let started_at = self.started_at;
        let (started, elapsed) = (self.started, self.elapsed);
        *self = Self::at(started + elapsed, appearance);
        self.started = started;
        self.elapsed = elapsed;
        self.started_at = started_at;
        self.held = held;
        self.desktop = desktop;
        // the backend starts again at this moment
        self.backend.runtime.now = started + elapsed;
        self.go(&route);
    }

    /// The label of a page, as the rail and the trail say it.
    pub fn label(page: &str) -> &'static str {
        label(page)
    }
}

impl eframe::App for ModelsApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "appearance", &self.appearance);
        let backend = &self.backend;
        let stored = Stored {
            models: backend.models.rows.clone(),
            conversations: backend.conversations.rows.clone(),
            comparisons: backend.comparisons.rows.clone(),
            prefs: backend.prefs.rows.clone(),
            views: backend.views.rows.clone(),
            kept: self.kept.clone(),
        };
        eframe::set_value(storage, "models", &stored);
    }
}
