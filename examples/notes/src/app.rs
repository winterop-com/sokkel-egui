//! The notes app, as the web's `Notes.tsx`: the rail of places over the vault's tree, the page the
//! route names, and the status line.

use std::collections::HashSet;

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, Empty, Icon, IconButton, Markdown, MarkdownScale, MenuItem, Page, Rail,
    RailBody, RailEntry, RailGroup, Search, SearchField, SearchResponse, SearchResult, SearchWords, Shell, ShellPart,
    StatusLine, StatusTone, Toaster, TopBar, Tree, TreeOrder, install,
};

use crate::backend::{Backend, Placing, SaveState};
use crate::model::{clock, word_count};
use crate::vault::{folder_and_below, vault_tree};

/// 2026-11-18 09:00 UTC: the moment the app's screenshots are taken at, as the web's are shot.
pub const DEMO_START: i64 = 1_794_992_400_000;

/// The Tidevann mark the rail heads with.
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// The places the route names.
const PAGES: [&str; 7] = ["notes", "folder", "note", "daily", "tags", "graph", "search"];

pub struct NotesApp {
    pub appearance: Appearance,
    pub backend: Backend,
    /// The address, split at its slashes.
    pub path: Vec<String>,
    /// The settings over the page, and the section they show.
    pub settings: Option<String>,
    /// The search palette while it is open: what is typed and the row lit.
    pub palette: Option<(String, usize)>,
    /// The rail's filter over the tree, and the tree's open folders.
    pub filter: String,
    pub open: HashSet<String>,
    /// What is typed in the top bar's search.
    pub top_query: String,
    /// The new note typed on a listing.
    pub quick: String,
    /// Each table's sort, widths and the row opened, by the table's name.
    pub tables: std::collections::HashMap<String, sokkel_egui::TableState>,
    /// The month the daily page shows, once it is moved off today's.
    pub daily_month: Option<sokkel_egui::Day>,
    /// What is typed in the tags' filter.
    pub tag_query: String,
    /// A heading of the note shown to scroll to, once drawn.
    pub reveal: Option<String>,
    /// What the search page searches for, and the facets' values chosen there.
    pub search_query: String,
    pub search_chosen: sokkel_egui::SearchChosen,
    /// The daily template while it is typed in Settings, and when it was last typed.
    pub template_draft: Option<(String, Option<f64>)>,
    /// The question before the demo's data goes back is open.
    pub resetting: bool,
    /// The graph shows the notes around the open one, and the names linked to with no note yet.
    pub graph_around: bool,
    pub graph_missing: bool,
    /// The note switcher while it is open: what is typed and the row lit.
    pub switching: Option<(String, usize)>,
    /// The question asked in a dialog, and the dialog's fields.
    pub asking: Option<crate::backend::Asking>,
    pub dialog: crate::dialogs::DialogState,
    /// The tab shown in the panel beside a note.
    pub panel_tab: usize,
    started: i64,
    started_at: Option<f64>,
    elapsed: i64,
    held: bool,
    pub desktop: bool,
}

impl NotesApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64);
        let appearance = cc
            .storage
            .and_then(|s| eframe::get_value(s, "appearance"))
            .unwrap_or_default();
        let mut app = Self::at(now, appearance);
        app.held = false;
        app.desktop = true;
        app
    }

    /// The app at a moment, its clock held there, as the tests and the screenshots take it.
    pub fn at(now: i64, appearance: Appearance) -> Self {
        Self {
            appearance,
            backend: Backend::seeded(now),
            path: vec!["notes".into()],
            settings: None,
            palette: None,
            filter: String::new(),
            open: ["folder:runbooks", "folder:jobs"].map(str::to_owned).into(),
            top_query: String::new(),
            quick: String::new(),
            tables: Default::default(),
            panel_tab: 0,
            switching: None,
            asking: None,
            dialog: Default::default(),
            graph_around: false,
            graph_missing: true,
            template_draft: None,
            resetting: false,
            search_query: String::new(),
            search_chosen: Default::default(),
            reveal: None,
            tag_query: String::new(),
            daily_month: None,
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

    pub fn go(&mut self, path: &str) {
        // settings open over the page, at the section the address names
        if let Some(rest) = path.trim_matches('/').strip_prefix("settings") {
            let section = rest.trim_start_matches('/');
            self.settings = Some(if section.is_empty() { "editor" } else { section }.to_owned());
            return;
        }
        self.path = path
            .trim_matches('/')
            .split('/')
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect();
        if self.path.is_empty() {
            self.path.push("notes".into());
        }
        // the search page opens on the query its address names
        if self.path[0] == "search" {
            self.search_query = self.path.get(1).cloned().unwrap_or_default();
            self.search_chosen = Default::default();
        }
    }

    /// Opens a note on its page, as a tab.
    pub fn open_note(&mut self, id: &str) {
        self.backend.show(id);
        self.go(&format!("note/{id}"));
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        let ctx = ui.ctx().clone();
        if !self.held {
            let time = ctx.input(|i| i.time);
            let since = *self.started_at.get_or_insert(time);
            self.elapsed = ((time - since) * 1000.0) as i64;
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
        let now = self.now();
        self.backend.tick(now);
        let first = self.path.first().cloned().unwrap_or_else(|| "notes".into());
        let second = self.path.get(1).cloned();
        let page = if PAGES.contains(&first.as_str()) {
            first.clone()
        } else {
            "missing".into()
        };
        let note_id = (page == "note").then(|| second.clone()).flatten();
        let folder_id = (page == "folder").then(|| second.clone()).flatten();

        let vault = self.backend.vault();
        let note = note_id.as_deref().and_then(|id| vault.note(id)).cloned();
        let entry = |id: &str, label: &str, icon: Icon, count: Option<usize>| {
            let entry = RailEntry::new(id, label, icon);
            match count {
                Some(n) => entry.count(n as u32),
                None => entry,
            }
        };
        let groups = vec![
            RailGroup::new(
                None,
                [
                    entry("notes", "All notes", Icon::Library, Some(vault.notes.len())),
                    entry("daily", "Daily", Icon::CalendarDays, None),
                    entry("tags", "Tags", Icon::Hash, Some(vault.tags().len())),
                    entry("graph", "Graph", Icon::Network, None),
                    entry("search", "Search", Icon::Search, None),
                ],
            ),
            RailGroup::new(Some("System"), [entry("settings", "Settings", Icon::Settings, None)]),
        ];
        let prefs = self.backend.prefs.clone();
        let tree = vault_tree(vault, prefs.daily_folder.as_deref(), self.backend.order());
        let label = match page.as_str() {
            "note" => note.as_ref().map_or("Not found".to_owned(), |n| n.title.clone()),
            "folder" => folder_id
                .as_deref()
                .and_then(|id| vault.folder(id))
                .map_or("Not found".to_owned(), |f| f.name.clone()),
            "notes" => "All notes".into(),
            "daily" => "Daily".into(),
            "tags" => "Tags".into(),
            "graph" => "Graph".into(),
            "search" => "Search".into(),
            _ => "Not found".into(),
        };
        let mut trail: Vec<String> = vec!["Notes".into()];
        if let Some(n) = note.as_ref().filter(|n| n.folder.is_some()) {
            trail.push(vault.path(n.folder.as_deref()));
        }
        if page == "tags"
            && let Some(tag) = &second
        {
            trail.extend(["Tags".into(), format!("#{tag}")]);
        } else {
            trail.push(label);
        }
        let trail_words: Vec<&str> = trail.iter().map(String::as_str).collect();
        let words = note.as_ref().map(|n| word_count(&n.body));
        let links_in = note.as_ref().map(|n| vault.backlinks(&n.id).len());
        let notes_count = vault.notes.len();
        let unsaved = !self.backend.drafted().is_empty();
        let (saying, failed) = match &self.backend.save {
            _ if unsaved => ("Saving".to_owned(), false),
            SaveState::Saved { at } => (format!("Saved at {}", clock(*at)), false),
            SaveState::Failed { reason } => (format!("Not saved: {reason}"), true),
            SaveState::Idle => ("All changes saved".to_owned(), false),
        };
        let chosen_node = note_id
            .clone()
            .or_else(|| folder_id.as_ref().map(|f| format!("folder:{f}")));

        let mut rail_place = match page.as_str() {
            "note" | "folder" => "notes".to_owned(),
            other => other.to_owned(),
        };
        let was = rail_place.clone();
        let mut picked = chosen_node.clone();
        let mut new_note = false;
        let mut new_folder = false;
        let mut account = None;
        let mut searched = sokkel_egui::palette_shortcut(&ctx);
        let mut new_from_top = false;
        let mut filter = std::mem::take(&mut self.filter);
        let mut open = std::mem::take(&mut self.open);
        // the top bar's search: the vault searched, six of each kind, each note's text to preview
        let top_found: Vec<sokkel_egui::SearchGroup> =
            crate::search::search_vault(vault, &self.top_query, &Default::default())
                .groups
                .into_iter()
                .map(|mut group| {
                    group.results.truncate(6);
                    group
                })
                .collect();
        let top_bodies: std::collections::HashMap<String, String> = top_found
            .iter()
            .flat_map(|g| &g.results)
            .filter_map(|r| vault.note(&r.id).map(|n| (n.id.clone(), n.body.clone())))
            .collect();
        let mut top_asked = SearchResponse::default();
        let mut top_query = std::mem::take(&mut self.top_query);
        let (filter, open, top_query) = (&mut filter, &mut open, &mut top_query);
        let mut panel_open = prefs.panel;
        let mut panel_asked = crate::note_panel::PanelAsked::default();
        let mut panel_tab = self.panel_tab;
        let mut shell = Shell::new();
        if self.desktop {
            shell = shell.desktop(sokkel_egui::DesktopWindow::native());
        }
        // the panel beside a note: what links to it, what it links to, its outline
        let panel_vault = note.as_ref().map(|_| vault.clone());
        if let (Some(shown), Some(panel_vault)) = (note.clone(), panel_vault) {
            let body = self
                .backend
                .draft(&shown.id)
                .map_or_else(|| shown.body.clone(), str::to_owned);
            let (asked, tab) = (&mut panel_asked, &mut panel_tab);
            shell = shell.panel(&mut panel_open, move |ui| {
                *asked = crate::note_panel::note_panel(ui, &panel_vault, &shown, &body, tab);
            });
        }
        // each folder with the folders under it, which it cannot be moved into
        let below: std::collections::HashMap<String, Vec<String>> = vault
            .folders
            .iter()
            .map(|f| (f.id.clone(), folder_and_below(vault, &f.id)))
            .collect();
        let manual = self.backend.order() == crate::model::Order::Manual;
        let mut tree_chosen = None;
        let mut tree_action: Option<(String, usize)> = None;
        let mut tree_moved: Option<(String, Option<String>, usize)> = None;
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                let query = filter.clone();
                let body = RailBody {
                    title: Some("Vault".into()),
                    search: Some(Box::new(|ui: &mut Ui| {
                        SearchField::new("vault-filter", filter)
                            .placeholder("Filter")
                            .keys("/")
                            .width(ui.available_width() - 2.0 * 30.0)
                            .show(ui);
                        new_note = ui.add(IconButton::new(Icon::FilePlus, "New note")).clicked();
                        new_folder = ui.add(IconButton::new(Icon::FolderPlus, "New folder")).clicked();
                    })),
                    content: Box::new(|ui: &mut Ui| {
                        let menu = |node: &sokkel_egui::TreeNode| Some(row_actions(node));
                        // a row goes into a folder or the top, never into a note, nor a folder
                        // into itself
                        let can_move = |id: &str, into: Option<&str>| match into {
                            None => true,
                            Some(into) => {
                                into.starts_with("folder:")
                                    && !id
                                        .strip_prefix("folder:")
                                        .and_then(|f| below.get(f))
                                        .is_some_and(|inside| inside.iter().any(|b| into == format!("folder:{b}")))
                            }
                        };
                        // folders stand before notes in every level, and by name the vault sorts
                        // each level itself
                        let band = |node: &sokkel_egui::TreeNode| {
                            if node.id.starts_with("folder:") {
                                "folder"
                            } else {
                                "note"
                            }
                        };
                        let out = Tree::new("vault-tree", &tree, &mut picked, open)
                            .query(&query)
                            .actions(&menu)
                            .movable(&can_move)
                            .order(TreeOrder {
                                band: Some(&band),
                                sorted: !manual,
                            })
                            .show(ui);
                        tree_chosen = out.chosen;
                        tree_action = out.action;
                        tree_moved = out.moved;
                    }),
                };
                ui.add(
                    Rail::new("Notes", &groups, &mut rail_place)
                        .mark(TIDEVANN_MARK)
                        .body(body),
                );
            }
            ShellPart::RailFoot => {
                account = AccountMenu::new("ada")
                    .place("tidevann.example.org")
                    .appearance(true)
                    .show(ui);
            }
            ShellPart::Top => {
                searched |= TopBar::new(&trail_words)
                    .start(|ui| {
                        let preview = |result: &SearchResult, ui: &mut Ui| {
                            if let Some(body) = top_bodies.get(&result.id) {
                                Markdown::new(body).scale(MarkdownScale::Document).show(ui);
                            }
                        };
                        top_asked = Search::new("top-search", top_query, &top_found)
                            .see_all(true)
                            .preview(&preview)
                            .width(256.0)
                            .words(SearchWords {
                                placeholder: "Search notes".into(),
                                ..Default::default()
                            })
                            .show(ui);
                    })
                    .search("Search or run a command", "⌘K")
                    .show(ui, |ui| {
                        new_from_top = ui.add(IconButton::new(Icon::FilePlus, "New note")).clicked();
                    })
                    .search;
            }
            ShellPart::Page => match page.as_str() {
                "notes" => Page::scroll(ui, |ui| self.all_notes(ui, None)),
                "note" if note.is_some() => {
                    let id = note_id.clone().unwrap_or_default();
                    self.note_page(ui, &id);
                }
                "daily" => Page::scroll(ui, |ui| self.daily(ui)),
                "search" => Page::scroll(ui, |ui| self.search_page(ui)),
                "graph" => Page::show(ui, |ui| self.graph_page(ui)),
                "tags" => {
                    let tag = second.clone();
                    Page::scroll(ui, |ui| self.tags(ui, tag.as_deref()));
                }
                "folder" if folder_id.is_some() => {
                    let folder = folder_id.clone();
                    Page::scroll(ui, |ui| self.all_notes(ui, folder.as_deref()));
                }
                _ => Page::scroll(ui, |ui| {
                    Empty::new(format!("No {} here", if page == "missing" { "page" } else { &page }))
                        .description("It was deleted, or the address is wrong.")
                        .show(ui, |_| {});
                }),
            },
            ShellPart::Foot => {
                let mut line = StatusLine::new(&saying);
                if failed {
                    line = line.tone(StatusTone::Warn);
                }
                if let Some(w) = words {
                    line = line.item("words", w);
                }
                if let Some(l) = links_in {
                    line = line.item("links in", l);
                }
                ui.add(line.item("notes", notes_count));
            }
        });
        self.panel_tab = panel_tab;
        if panel_open != prefs.panel {
            self.backend.set_prefs(|p| p.panel = panel_open);
        }
        if let Some(id) = panel_asked.open {
            self.open_note(&id);
        }
        if let Some(target) = panel_asked.make {
            self.follow(&target, note_id.as_deref());
        }
        if let Some(heading) = panel_asked.heading {
            self.reveal = Some(heading);
        }
        if let Some(tag) = panel_asked.tag {
            self.go(&format!("tags/{tag}"));
        }
        if let (Some((from, at)), Some(shown)) = (panel_asked.link, note.as_ref()) {
            let title = shown.title.clone();
            let _ = self.backend.rewrite(
                &from,
                |current| {
                    let written = &current[at.clone()];
                    let text = if written == title {
                        format!("[[{title}]]")
                    } else {
                        format!("[[{title}|{written}]]")
                    };
                    format!("{}{}{}", &current[..at.start], text, &current[at.end..])
                },
                now,
            );
        }
        self.filter = std::mem::take(filter);
        self.open = std::mem::take(open);
        self.top_query = std::mem::take(top_query);
        if let Some((_, id)) = top_asked.opened {
            self.open_note(&id);
        }
        if let Some(query) = top_asked.see_all {
            self.go(&format!("search/{query}"));
        }
        if searched {
            self.palette = Some(Default::default());
        }
        // Mod-O opens a note by name, Mod-Shift-D today's note, Mod-Shift-F the search page
        let (switch, today, search) = ctx.input_mut(|i| {
            let shift = egui::Modifiers::COMMAND | egui::Modifiers::SHIFT;
            (
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::O),
                i.consume_key(shift, egui::Key::D),
                i.consume_key(shift, egui::Key::F),
            )
        });
        if switch {
            self.switching = Some(Default::default());
        }
        if today {
            let date = crate::model::day(0, now);
            self.open_day(&ctx, &date);
        }
        if search {
            self.go("search");
        }
        if rail_place != was {
            self.go(&rail_place);
        }
        if let Some(id) = tree_chosen {
            match id.strip_prefix("folder:") {
                Some(folder) => self.go(&format!("folder/{folder}")),
                None => self.open_note(&id),
            }
        }
        if let Some((id, into, index)) = tree_moved {
            self.place_from_tree(&ctx, &id, into.as_deref(), index);
        }
        if let Some((id, index)) = tree_action {
            match id.strip_prefix("folder:") {
                Some(folder) => self.folder_action(folder, index),
                None => self.note_action(&id, index),
            }
        }
        if new_note || new_from_top {
            let folder = note.as_ref().and_then(|n| n.folder.clone());
            self.new_note_in(folder.as_deref());
        }
        if new_folder {
            self.ask(crate::backend::Asking::NewFolder { parent: None });
        }
        match account {
            Some(AccountAction::Settings) => self.go("settings"),
            Some(AccountAction::Appearance) => self.settings = Some("appearance".into()),
            None => {}
        }
        self.search_palette(&ctx, &groups);
        self.note_switcher(&ctx);
        self.dialogs(&ctx);
        self.settings_dialog(&ctx);
        Toaster::new().show(&ctx);
    }
}

impl NotesApp {
    /// Puts a note or a folder where the tree's drag says, as the web's `placeAt`: into a folder, at
    /// its place among the notes or the folders there.
    fn place_from_tree(&mut self, ctx: &egui::Context, id: &str, into: Option<&str>, band: usize) {
        let folder = id.starts_with("folder:");
        let (what, own) = if folder {
            (Placing::Folder, id.trim_start_matches("folder:"))
        } else {
            (Placing::Note, id)
        };
        let into = into.map(|f| f.trim_start_matches("folder:"));
        match self.backend.place(what, own, into, band) {
            Ok(Some(notice)) => crate::daily::notify(ctx, notice),
            Ok(None) => {}
            Err(reason) => {
                sokkel_egui::Toast::error("Not moved").description(reason).show(ctx);
            }
        }
    }
}

/// What a row of the vault's tree offers in its menu, as the web's `useRowActions`: a folder's
/// New note, New folder, Rename, Move to and Delete; a note's Open, Rename, Move to and Delete.
pub fn row_actions(node: &sokkel_egui::TreeNode) -> Vec<MenuItem> {
    if node.id.starts_with("folder:") {
        return vec![
            MenuItem::action("New note").icon(Icon::FilePlus),
            MenuItem::action("New folder").icon(Icon::FolderPlus),
            MenuItem::Separator,
            MenuItem::action("Rename").icon(Icon::Pencil),
            MenuItem::action("Move to").icon(Icon::FolderInput),
            MenuItem::Separator,
            MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
        ];
    }
    crate::all_notes::note_menu()
}

impl eframe::App for NotesApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "appearance", &self.appearance);
    }
}
