//! The studio, as the web's `Studio.tsx`: the rail of places over the workspace's tree, the page the
//! route names, the palettes and the dialogs.

use std::collections::{HashMap, HashSet};

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, Button, ButtonSize, ButtonVariant, Empty, Icon, IconButton, Page, Rail,
    RailBody, RailEntry, RailGroup, SearchField, Shell, ShellPart, StatusLine, StatusTone, TableState, Toaster, TopBar,
    Tree, TreeNode, install, tokens,
};

use crate::backend::{Asking, Backend, SaveState, tab_href};
use crate::model::{FileKind, KINDS, PROJECTS, Project, WorkFile, clock, counted, file_order, kind_of};
use crate::sql::Table;
use crate::tables::tables_with;

/// 2026-11-18 09:00 UTC: the moment the app's screenshots are taken at, as the web's are shot.
pub const DEMO_START: i64 = 1_794_992_400_000;

/// The Tidevann mark the rail heads with.
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// A kind's icon, as the web's `KINDS` gives it.
pub fn kind_icon(kind: FileKind) -> Icon {
    match kind {
        FileKind::Pipeline => Icon::Workflow,
        FileKind::Schema => Icon::Braces,
        FileKind::Query => Icon::Database,
    }
}

/// The project and kind a tree row stands for, when it is a project or one of its groups.
pub fn place_of(id: &str) -> Option<(Project, Option<FileKind>)> {
    if let Some(project) = id.strip_prefix("project:") {
        return Project::named(project).map(|p| (p, None));
    }
    let rest = id.strip_prefix("group:")?;
    let (project, kind) = rest.split_once(':')?;
    let kind = KINDS.iter().find(|k| k.kind.name() == kind)?.kind;
    Project::named(project).map(|p| (p, Some(kind)))
}

/// The rail's tree: each project's files by kind, then the tables a query reads.
fn workspace_tree(
    files: &[WorkFile],
    dirty: &HashSet<String>,
    tables: &[Table],
    unsaved: egui::Color32,
) -> Vec<TreeNode> {
    let mut nodes: Vec<TreeNode> = PROJECTS
        .iter()
        .map(|project| {
            TreeNode::new(format!("project:{}", project.name()), project.name())
                .icon(Icon::FolderOpen)
                .children(
                    KINDS
                        .iter()
                        .map(|kind| {
                            let mut inside: Vec<&WorkFile> = files
                                .iter()
                                .filter(|f| f.project == *project && f.kind == kind.kind)
                                .collect();
                            inside.sort_by(|a, b| file_order(a, b));
                            TreeNode::new(format!("group:{}:{}", project.name(), kind.kind.name()), kind.plural)
                                .children(
                                    inside
                                        .into_iter()
                                        .map(|file| {
                                            let node = TreeNode::new(&file.id, &file.name).icon(kind_icon(kind.kind));
                                            if dirty.contains(&file.id) {
                                                node.dot(unsaved, "Not saved")
                                            } else {
                                                node
                                            }
                                        })
                                        .collect(),
                                )
                        })
                        .collect(),
                )
        })
        .collect();
    nodes.push(
        TreeNode::new("tables", "Tables").icon(Icon::SquareTerminal).children(
            tables
                .iter()
                .map(|table| {
                    TreeNode::new(format!("table:{}", table.name), &table.name)
                        .icon(Icon::Sheet)
                        .meta(crate::js::locale_number(table.rows.len() as f64))
                })
                .collect(),
        ),
    );
    nodes
}

pub struct StudioApp {
    pub appearance: Appearance,
    pub backend: Backend,
    /// The address, split at its slashes.
    pub path: Vec<String>,
    /// The settings over the page, and the section they show.
    pub settings: Option<String>,
    /// The search palette and the file switcher while open: what is typed and the row lit.
    pub palette: Option<(String, usize)>,
    pub switching: Option<(String, usize)>,
    /// The rail's filter over the tree, and the tree's open rows.
    pub filter: String,
    pub open: HashSet<String>,
    /// Each table's sort and widths, by the table's name.
    pub tables: HashMap<String, TableState>,
    /// What each editor page and the pipelines' graphs keep, by file, and each search typed.
    pub views: HashMap<String, crate::work::FileView>,
    pub canvases: HashMap<String, crate::pipeline_editor::CanvasState>,
    pub queries: crate::work::Queries,
    /// The dialog's fields, and the question before the demo's data goes back.
    pub dialog: crate::dialogs::DialogState,
    pub resetting: bool,
    started: i64,
    started_at: Option<f64>,
    elapsed: i64,
    held: bool,
    pub desktop: bool,
}

impl StudioApp {
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
        let mut open: HashSet<String> = [
            "project:harbor",
            "group:harbor:pipeline",
            "group:harbor:schema",
            "group:harbor:query",
        ]
        .map(str::to_owned)
        .into();
        open.shrink_to_fit();
        Self {
            appearance,
            backend: Backend::seeded(),
            path: Vec::new(),
            settings: None,
            palette: None,
            switching: None,
            filter: String::new(),
            open,
            tables: HashMap::new(),
            views: HashMap::new(),
            canvases: HashMap::new(),
            queries: HashMap::new(),
            dialog: Default::default(),
            resetting: false,
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

    /// Moves a held clock on, as the tests watch a run go.
    pub fn wait(&mut self, ms: i64) {
        self.elapsed += ms;
    }

    /// Goes to an address; the workspace's own opens the tab last shown.
    pub fn go(&mut self, path: &str) {
        let path = path.trim_matches('/');
        if let Some(rest) = path.strip_prefix("settings") {
            let section = rest.trim_start_matches('/');
            self.settings = Some(if section.is_empty() { "editor" } else { section }.to_owned());
            return;
        }
        self.path = path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect();
        if self.path.is_empty()
            && let Some(active) = self.backend.prefs.active.clone()
        {
            self.path = tab_href(&active)
                .trim_matches('/')
                .split('/')
                .map(str::to_owned)
                .collect();
        }
        // a file or a table opened is its tab, and its project in the tree
        if let (Some(first), Some(second)) = (self.path.first().cloned(), self.path.get(1).cloned()) {
            let tab = if first == "table" {
                format!("table:{second}")
            } else {
                second.clone()
            };
            if first == "file" || first == "table" {
                self.backend.show(&tab);
            }
            if first == "file"
                && let Some(file) = self.backend.file(&second)
            {
                self.open.insert(format!("project:{}", file.project.name()));
                self.open
                    .insert(format!("group:{}:{}", file.project.name(), file.kind.name()));
            }
            if first == "table" {
                self.open.insert("tables".into());
            }
        }
    }

    /// Opens a file or a table in its tab.
    pub fn open_tab(&mut self, id: &str) {
        self.go(&tab_href(id));
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        let ctx = ui.ctx().clone();
        let t = tokens(&ctx);
        if !self.held {
            let time = ctx.input(|i| i.time);
            let since = *self.started_at.get_or_insert(time);
            self.elapsed = ((time - since) * 1000.0) as i64;
        }
        let now = self.now();
        self.backend.tick(now);
        if self.backend.moving() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        // the workspace's own address opens the tab last shown, unless settings stand over it
        if self.path.is_empty() && self.settings.is_none() && self.backend.prefs.active.is_some() {
            self.go("");
        }
        let first = self.path.first().cloned();
        let second = self.path.get(1).cloned();
        let page = match first.as_deref() {
            Some("file") | Some("table") => "work",
            Some("runs") if second.is_none() => "runs",
            Some("runs") => "run",
            None => "home",
            _ => "missing",
        };
        let tab = match first.as_deref() {
            Some("table") => format!("table:{}", second.clone().unwrap_or_default()),
            _ => second.clone().unwrap_or_default(),
        };
        let file = (first.as_deref() == Some("file"))
            .then(|| second.as_deref().and_then(|id| self.backend.file(id)).cloned())
            .flatten();
        let dirty = self.backend.dirty();
        let going = self.backend.runs.iter().filter(|r| r.state.name() == "running").count();
        let tables = tables_with(&self.backend.runs);

        let groups = vec![
            RailGroup::new(
                None,
                [
                    RailEntry::new("workspace", "Workspace", Icon::FileCode),
                    RailEntry::new("runs", "Runs", Icon::History).count(self.backend.runs.len() as u32),
                ],
            ),
            RailGroup::new(Some("System"), [RailEntry::new("settings", "Settings", Icon::Settings)]),
        ];
        let tree = workspace_tree(&self.backend.files, &dirty, &tables, t.color("warning"));
        let project = file.as_ref().map_or(Project::Harbor, |f| f.project);
        let kind = file.as_ref().map_or(FileKind::Pipeline, |f| f.kind);

        let mut trail: Vec<(String, bool)> = Vec::new();
        match (page, &file) {
            ("work", Some(file)) => {
                trail.push((file.project.name().into(), false));
                trail.push((kind_of(file.kind).plural.into(), false));
                trail.push((file.name.clone(), true));
            }
            ("work", None) => {
                trail.push(("Tables".into(), false));
                trail.push((second.clone().unwrap_or_default(), true));
            }
            ("runs", _) | ("run", _) => {
                trail.push(("Runs".into(), false));
                if let Some(run) = self.backend.runs.iter().find(|r| Some(&r.id) == second.as_ref()) {
                    trail.push((format!("Run {}", run.number), false));
                }
            }
            _ => trail.push(("Workspace".into(), false)),
        }
        let trail_words: Vec<&str> = trail.iter().map(|c| c.0.as_str()).collect();
        let saying = match &self.backend.saving {
            SaveState::Saving => "Saving".to_owned(),
            SaveState::Failed { reason } => format!("Not saved: {reason}"),
            _ if !dirty.is_empty() => format!("{} not saved", counted(dirty.len() as f64, "file", None)),
            SaveState::Saved { at } => format!("Saved at {}", clock(*at)),
            SaveState::Idle => "All changes saved".to_owned(),
        };
        let failed = matches!(self.backend.saving, SaveState::Failed { .. });
        let open_tabs = self
            .backend
            .prefs
            .tabs
            .iter()
            .filter(|id| self.backend.file(id).is_some())
            .count();
        let files_count = self.backend.files.len();

        let mut rail_place = match page {
            "runs" | "run" => "runs".to_owned(),
            _ => "workspace".to_owned(),
        };
        let was = rail_place.clone();
        let mut picked = (page == "work").then(|| tab.clone());
        let mut tree_chosen = None;
        let mut tree_action: Option<(String, usize)> = None;
        let mut new_file = false;
        let mut save_all = false;
        let mut account = None;
        let mut searched = sokkel_egui::palette_shortcut(&ctx);
        let mut filter = std::mem::take(&mut self.filter);
        let mut open = std::mem::take(&mut self.open);
        let (filter, open) = (&mut filter, &mut open);
        let mut shell = Shell::new();
        if self.desktop {
            shell = shell.desktop(sokkel_egui::DesktopWindow::native());
        }
        let backend_files = self.backend.files.clone();
        let dirty_now = dirty.clone();
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                let query = filter.clone();
                let body = RailBody {
                    title: Some("Workspace".into()),
                    places_above: true,
                    search: Some(Box::new(|ui: &mut Ui| {
                        SearchField::new("workspace-filter", filter)
                            .placeholder("Filter")
                            .keys("/")
                            .width(ui.available_width() - 30.0)
                            .show(ui);
                        new_file = ui.add(IconButton::new(Icon::FilePlus, "New file")).clicked();
                    })),
                    content: Box::new(|ui: &mut Ui| {
                        let menu = |node: &TreeNode| Some(row_actions(node, &backend_files, &dirty_now));
                        let out = Tree::new("workspace-tree", &tree, &mut picked, open)
                            .query(&query)
                            .actions(&menu)
                            .show(ui);
                        tree_chosen = out.chosen;
                        tree_action = out.action;
                    }),
                };
                ui.add(
                    Rail::new("Studio", &groups, &mut rail_place)
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
                let mut bar = TopBar::new(&trail_words).search("Search", "⌘K");
                if let Some(last) = trail.iter().rposition(|c| c.1) {
                    bar = bar.code(last);
                }
                searched |= bar
                    .show(ui, |ui| {
                        new_file = ui.add(IconButton::new(Icon::FilePlus, "New file")).clicked() || new_file;
                        save_all = ui
                            .add_enabled(!dirty.is_empty(), IconButton::new(Icon::Save, "Save all"))
                            .clicked();
                    })
                    .search;
            }
            ShellPart::Page => match page {
                "work" => self.work_area(ui, &tab),
                "runs" => Page::show(ui, |ui| self.runs_page(ui)),
                "run" => {
                    let id = second.clone().unwrap_or_default();
                    Page::show(ui, |ui| self.run_page(ui, &id));
                }
                "home" => Page::show(ui, |ui| self.home(ui)),
                _ => Page::show(ui, |ui| {
                    let mut back = false;
                    Empty::new("Nothing here")
                        .description("The address is wrong, or what it named was deleted.")
                        .show(ui, |ui| {
                            back = ui
                                .add(
                                    Button::new("Back to the workspace")
                                        .variant(ButtonVariant::Outline)
                                        .size(ButtonSize::Sm),
                                )
                                .clicked();
                        });
                    if back {
                        self.go("");
                    }
                }),
            },
            ShellPart::Foot => {
                let mut line = StatusLine::new(&saying);
                if failed {
                    line = line.tone(StatusTone::Warn);
                }
                if going > 0 {
                    line = line.item("running", going);
                }
                if open_tabs > 0 {
                    line = line.item("open", open_tabs);
                }
                ui.add(line.item("files", files_count));
            }
        });
        self.filter = std::mem::take(filter);
        self.open = std::mem::take(open);
        if searched {
            self.palette = Some(Default::default());
        }
        // Mod-P opens a file by name, Mod-Shift-S saves all
        let (switch, save_keys) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::P),
                i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::S),
            )
        });
        if switch {
            self.switching = Some(Default::default());
        }
        if save_keys {
            save_all = true;
        }
        if rail_place != was {
            match rail_place.as_str() {
                "workspace" => self.go(""),
                other => self.go(other),
            }
        }
        if let Some(id) = tree_chosen
            && place_of(&id).is_none()
            && id != "tables"
        {
            self.open_tab(&id);
        }
        if let Some((id, index)) = tree_action {
            self.row_action(&id, index, now);
        }
        if new_file {
            self.backend.asking = Some(Asking::New { project, kind });
        }
        if save_all {
            self.backend.save_all(now);
        }
        match account {
            Some(AccountAction::Settings) => self.go("settings"),
            Some(AccountAction::Appearance) => self.settings = Some("appearance".into()),
            None => {}
        }
        self.search_palette(&ctx, &groups, project, kind);
        self.file_switcher(&ctx);
        self.dialogs(&ctx);
        self.settings_dialog(&ctx);
        Toaster::new().show(&ctx);
    }

    /// The workspace with no tab open: a way to open a file or make one.
    fn home(&mut self, ui: &mut Ui) {
        let (mut open, mut new) = (false, false);
        Empty::new("No file open")
            .description("Open a pipeline, a schema or a query from the workspace, or make a new one.")
            .show(ui, |ui| {
                open = ui
                    .add(Button::new("Open a file").icon(Icon::FolderOpen).size(ButtonSize::Sm))
                    .clicked();
                new = ui
                    .add(
                        Button::new("New pipeline")
                            .icon(Icon::FilePlus)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm),
                    )
                    .clicked();
            });
        if open {
            self.switching = Some(Default::default());
        }
        if new {
            self.backend.asking = Some(Asking::New {
                project: Project::Harbor,
                kind: FileKind::Pipeline,
            });
        }
    }

    /// An entry pressed in a tree row's menu, as `row_actions` lists them.
    fn row_action(&mut self, id: &str, index: usize, now: i64) {
        if let Some((project, kind)) = place_of(id) {
            let kinds: Vec<FileKind> = match kind {
                Some(kind) => vec![kind],
                None => KINDS.iter().map(|k| k.kind).collect(),
            };
            if let Some(kind) = kinds.get(index) {
                self.backend.asking = Some(Asking::New { project, kind: *kind });
            }
            return;
        }
        if id.starts_with("table:") {
            self.open_tab(id);
            return;
        }
        let Some(file) = self.backend.file(id).cloned() else {
            return;
        };
        let dirty = self.backend.dirty().contains(id);
        let others: Vec<Project> = PROJECTS.iter().copied().filter(|p| *p != file.project).collect();
        // Open, Rename, Save (while unsaved), a rule, Move to each other project, a rule, Delete
        let mut at = 0;
        let mut next = |n: usize| {
            let hit = index >= at && index < at + n;
            let offset = index.wrapping_sub(at);
            at += n;
            hit.then_some(offset)
        };
        if next(1).is_some() {
            self.open_tab(id);
        } else if next(1).is_some() {
            self.backend.asking = Some(Asking::Rename { id: id.to_owned() });
        } else if dirty && next(1).is_some() {
            self.backend.save(id, now);
        } else if next(1).is_some() {
        } else if let Some(i) = next(others.len()) {
            self.backend.move_file(id, others[i], None);
        } else if next(1).is_some() {
        } else if next(1).is_some() {
            self.backend.asking = Some(Asking::Delete { id: id.to_owned() });
        }
    }
}

/// What a tree row offers in its menu, as the web's `rowActions`: a project or a group its New
/// entries, a table Open, a file Open, Rename, Save while unsaved, Move to each other project and
/// Delete.
fn row_actions(node: &TreeNode, files: &[WorkFile], dirty: &HashSet<String>) -> Vec<sokkel_egui::MenuItem> {
    use sokkel_egui::MenuItem;
    if let Some((_, kind)) = place_of(&node.id) {
        return KINDS
            .iter()
            .filter(|k| kind.is_none_or(|kind| k.kind == kind))
            .map(|k| MenuItem::action(format!("New {}", k.label.to_lowercase())).icon(kind_icon(k.kind)))
            .collect();
    }
    if node.id.starts_with("table:") {
        return vec![MenuItem::action("Open").icon(Icon::Sheet)];
    }
    let Some(file) = files.iter().find(|f| f.id == node.id) else {
        return Vec::new();
    };
    let mut items = vec![
        MenuItem::action("Open").icon(Icon::FileCode),
        MenuItem::action("Rename").icon(Icon::Pencil),
    ];
    if dirty.contains(&file.id) {
        items.push(MenuItem::action("Save").icon(Icon::Save));
    }
    items.push(MenuItem::Separator);
    for project in PROJECTS.iter().filter(|p| **p != file.project) {
        items.push(MenuItem::action(format!("Move to {}", project.name())).icon(Icon::FolderInput));
    }
    items.push(MenuItem::Separator);
    items.push(MenuItem::action("Delete").icon(Icon::Trash2).destructive());
    items
}

impl eframe::App for StudioApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "appearance", &self.appearance);
    }
}
