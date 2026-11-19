//! The editor's frame, as the web's `Home.tsx`: the rail with the projects and the recent ones, the
//! trail, the page the address names (the projects, or one opened in the editor), the status line,
//! and the settings, the palette and the dialogs over them.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, Icon, IconButton, NotFound, NotFoundWords, Page, Rail, RailEntry,
    RailGroup, Shell, ShellPart, StatusLine, Toast, ToastKind, Toaster, TopBar, install, relative_time, toast_acted,
};

use crate::backend::{Backend, Notice, NoticeKind, RECENT};
use crate::dialogs::{Asking, DialogState};
use crate::editor_page::EditorView;
use crate::model::{ClipIds, length_of, length_text};

/// 2026-11-18 09:00 UTC: the moment the app's screenshots are taken at, as the web's are shot.
pub const DEMO_START: i64 = 1_794_992_400_000;

/// The Tidevann mark the rail heads with.
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// A notice from the backend as a toast: its kind, its title, the line under it and its button.
pub fn toast_of(notice: &Notice) -> Toast {
    let kind = match notice.kind {
        NoticeKind::Plain => ToastKind::Plain,
        NoticeKind::Success => ToastKind::Success,
        NoticeKind::Warning => ToastKind::Warning,
        NoticeKind::Error => ToastKind::Error,
    };
    let mut toast = Toast::new(&notice.title).kind(kind);
    if let Some(description) = &notice.description {
        toast = toast.description(description);
    }
    if let Some(action) = &notice.action {
        toast = toast.action(action);
    }
    toast
}

pub struct CutApp {
    pub appearance: Appearance,
    pub backend: Backend,
    /// The address, split at its slashes.
    pub path: Vec<String>,
    /// The settings over the page, and the section they show.
    pub settings: Option<String>,
    /// The palette while it is open: what is typed and the row lit.
    pub palette: Option<(String, usize)>,
    /// The question asked in a dialog, and the dialog's fields.
    pub asking: Option<Asking>,
    pub dialog: DialogState,
    /// The project open in the editor, with its history, playhead and what is chosen.
    pub editor: Option<EditorView>,
    /// What makes new clips' ids, over the visit.
    pub ids: ClipIds,
    /// The question before the demo's data goes back is open.
    pub resetting: bool,
    /// Toasts whose Open goes to a project made, by the toast's id.
    pub opens: Vec<(u64, String)>,
    started: i64,
    started_at: Option<f64>,
    elapsed: i64,
    held: bool,
    pub desktop: bool,
}

impl CutApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let now = crate::js::now_ms();
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
            path: Vec::new(),
            settings: None,
            palette: None,
            asking: None,
            dialog: DialogState::default(),
            editor: None,
            ids: ClipIds::new(now),
            resetting: false,
            opens: Vec::new(),
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

    /// Moves a held clock on, as the tests watch an edit saved or played.
    pub fn wait(&mut self, ms: i64) {
        self.elapsed += ms;
    }

    /// Goes to an address: `projects`, `p/<id>`, or `settings` over the page.
    pub fn go(&mut self, path: &str) {
        let path = path.trim_matches('/');
        if let Some(rest) = path.strip_prefix("settings") {
            let section = rest.trim_start_matches('/');
            self.settings = Some(if section.is_empty() { "playback" } else { section }.to_owned());
            return;
        }
        self.path = path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect();
    }

    /// The project the address opens in the editor, if it names one that is there.
    pub fn routed(&self) -> Option<String> {
        match (self.path.first().map(String::as_str), self.path.get(1)) {
            (Some("p"), Some(id)) => self.backend.project(id).map(|p| p.id.clone()),
            _ => None,
        }
    }

    /// Keeps what the editor has not saved yet and closes it, as the web's editor saves as it goes.
    pub fn close_editor(&mut self, ctx: &egui::Context) {
        if let Some(view) = self.editor.take() {
            let now = self.now();
            if let Err(notice) = view.save_now(&mut self.backend, now) {
                toast_of(&notice).show(ctx);
            }
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        let ctx = ui.ctx().clone();
        if !self.held {
            let time = ctx.input(|i| i.time);
            let since = *self.started_at.get_or_insert(time);
            self.elapsed = ((time - since) * 1000.0) as i64;
            ctx.request_repaint_after(std::time::Duration::from_millis(1000));
        }
        let now = self.now();
        self.ids.now = now;
        // a bare address opens the projects, once no settings stand over it
        if self.path.is_empty() && self.settings.is_none() {
            self.path.push("projects".into());
        }

        // the editor follows the address: another project opens it anew, none closes it
        let routed = self.routed();
        if self.editor.as_ref().map(|e| e.project.clone()) != routed {
            self.close_editor(&ctx);
            if let Some(id) = &routed
                && let Some(project) = self.backend.project(id)
            {
                self.editor = Some(EditorView::new(project, &self.backend.prefs, now));
            }
        }
        if let Some(view) = &mut self.editor {
            if let Err(notice) = view.tick(&mut self.backend, now) {
                toast_of(&notice).show(&ctx);
            }
            if view.waiting() {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }

        let first = self.path.first().cloned().unwrap_or_default();
        let all: Vec<crate::model::Project> = self.backend.by_updated().into_iter().cloned().collect();
        let project = routed
            .as_deref()
            .and_then(|id| all.iter().find(|p| p.id == id))
            .cloned();
        let editing = project.is_some();

        let groups = vec![
            RailGroup::new(None, [RailEntry::new("projects", "Projects", Icon::Clapperboard)]),
            RailGroup::new(
                Some("Recent"),
                all.iter()
                    .take(RECENT)
                    .map(|one| RailEntry::new(&one.id, &one.name, Icon::Film)),
            ),
            RailGroup::new(
                Some("System"),
                [RailEntry::new("settings", "Settings", Icon::Settings2)],
            ),
        ];
        let mut trail: Vec<String> = vec!["Tidevann".into(), "Projects".into()];
        if let Some(project) = &project {
            trail.push(project.name.clone());
        }
        let trail_words: Vec<&str> = trail.iter().map(String::as_str).collect();

        let mut rail_place = match &project {
            Some(project) => project.id.clone(),
            None if first == "projects" => "projects".to_owned(),
            None => String::new(),
        };
        let was = rail_place.clone();
        let mut account = None;
        let mut searched = sokkel_egui::palette_shortcut(&ctx);
        let mut new_project = false;
        let mut home = false;
        let mut asked = crate::editor_page::EditorAsked::default();
        let mut projects_asked = crate::projects_page::ProjectsAsked::default();
        let status = match &project {
            Some(project) => ("length", length_text(length_of(&project.edit))),
            None => (
                if all.len() == 1 { "project" } else { "projects" },
                all.len().to_string(),
            ),
        };
        let address = format!("#/{}", self.path.join("/"));

        let mut shell = Shell::new().collapsed(editing);
        if self.desktop {
            shell = shell.desktop(sokkel_egui::DesktopWindow::native());
        }
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                ui.add(
                    Rail::new("Tidevann", &groups, &mut rail_place)
                        .mark(TIDEVANN_MARK)
                        .collapsed(editing),
                );
            }
            ShellPart::RailFoot => {
                account = AccountMenu::new("ada")
                    .place("tidevann.example.org")
                    .local(true)
                    .appearance(true)
                    .show(ui);
            }
            ShellPart::Top => {
                searched |= TopBar::new(&trail_words)
                    .search("Search", "⌘K")
                    .show(ui, |ui| {
                        new_project = ui.add(IconButton::new(Icon::Plus, "New project")).clicked();
                    })
                    .search;
            }
            ShellPart::Page => match (&project, first.as_str()) {
                (Some(project), _) => {
                    let name = project.name.clone();
                    if let Some(view) = &mut self.editor {
                        let prefs = self.backend.prefs.clone();
                        asked = Page::show(ui, |ui| {
                            crate::editor_page::editor_page(ui, view, &name, &prefs, &mut self.ids, now)
                        });
                    }
                }
                (None, "projects" | "") => {
                    let shown: Vec<&crate::model::Project> = all.iter().collect();
                    projects_asked = Page::scroll(ui, |ui| crate::projects_page::projects_page(ui, &shown, now));
                }
                _ => Page::scroll(ui, |ui| {
                    home = NotFound::new(&address)
                        .words(NotFoundWords {
                            home: "Go to the projects".into(),
                            ..NotFoundWords::default()
                        })
                        .show(ui);
                }),
            },
            ShellPart::Foot => {
                ui.add(StatusLine::new("All changes saved").item(status.0, &status.1));
            }
        });

        if searched {
            self.palette = Some(Default::default());
        }
        if rail_place != was {
            match rail_place.as_str() {
                "settings" => self.go("settings"),
                "projects" => self.go("projects"),
                id => self.go(&format!("p/{id}")),
            }
        }
        match account {
            Some(AccountAction::Settings) => self.go("settings"),
            Some(AccountAction::Appearance) => self.go("settings/appearance"),
            None => {}
        }
        if new_project || projects_asked.create {
            self.ask(Asking::New);
        }
        if home {
            self.go("projects");
        }
        if let Some(id) = projects_asked.open {
            self.go(&format!("p/{id}"));
        }
        if let Some(id) = projects_asked.rename {
            self.ask(Asking::Rename(id));
        }
        if let Some(id) = projects_asked.duplicate {
            self.duplicate(&ctx, &id);
        }
        if let Some(id) = projects_asked.delete {
            self.ask(Asking::Delete(id));
        }
        if let Some(project) = &project {
            if asked.rename {
                self.ask(Asking::Rename(project.id.clone()));
            }
            if asked.duplicate {
                self.duplicate(&ctx, &project.id);
            }
            if asked.delete {
                self.ask(Asking::Delete(project.id.clone()));
            }
        }
        for id in toast_acted(&ctx) {
            if let Some((_, project)) = self.opens.iter().find(|(toast, _)| *toast == id).cloned() {
                self.go(&format!("p/{project}"));
            }
        }

        let overlay = self.palette.is_some()
            || self.settings.is_some()
            || self.asking.is_some()
            || self.resetting
            || self.editor.as_ref().is_some_and(|e| e.exporting.is_some());
        if let Some(view) = &mut self.editor {
            if !overlay {
                let prefs = self.backend.prefs.clone();
                view.shortcuts(&ctx, &prefs, &mut self.ids, now);
            }
            view.export_dialog(&ctx);
        }
        self.search_palette(&ctx, &groups, project.as_ref().map(|p| p.id.clone()));
        self.dialogs(&ctx);
        self.settings_dialog(&ctx);
        Toaster::new().show(&ctx);
    }

    /// Asks a question in a dialog, its fields started anew.
    pub fn ask(&mut self, asking: Asking) {
        self.dialog = DialogState::default();
        self.asking = Some(asking);
    }

    /// A copy of a project, listed first, with a toast whose Open goes to it.
    pub fn duplicate(&mut self, ctx: &egui::Context, id: &str) {
        let now = self.now();
        self.ids.now = now;
        // a bare address opens the projects, once no settings stand over it
        if self.path.is_empty() && self.settings.is_none() {
            self.path.push("projects".into());
        }
        match self.backend.duplicate_project(id, now, &mut self.ids) {
            Ok((made, notice)) => {
                let toast = toast_of(&notice).show(ctx);
                self.opens.push((toast, made));
            }
            Err(notice) => {
                toast_of(&notice).show(ctx);
            }
        }
    }

    /// When a project was last edited, in words: `Edited 2 hours ago`.
    pub fn edited(updated: i64, now: i64) -> String {
        format!("Edited {}", relative_time((updated - now) as f64))
    }
}

impl eframe::App for CutApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "appearance", &self.appearance);
    }
}
