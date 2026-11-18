//! The runs console, as the web's `Console.tsx`: the rail of its pages, the top bar with the trail,
//! the page the address names, a run beside it, and the status line.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, ConfirmAnswer, ConfirmDialog, Icon, IconButton, Page, Rail, RailEntry,
    RailGroup, Shell, ShellPart, StatusLine, StatusTone, Toast, ToastKind, Toaster, TopBar, install,
};

use crate::backend::Backend;
use crate::model::{OpenRuns, Person, Prefs, Run, Status, clock, day_of};
use crate::schedule::next_fire;
use crate::timeline::{RunAt, is_live, run_at};

/// 2026-11-18 09:00 UTC: the moment the app's screenshots are taken at, as the web's are shot.
pub const DEMO_START: i64 = 1_794_992_400_000;

/// The Tidevann mark the rail heads with.
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// A run with where it stands now.
#[derive(Clone, Debug)]
pub struct LiveRun {
    pub run: Run,
    pub at: RunAt,
}

/// Past every moment a run plans, so a settled run is read as it ended.
const LATER: i64 = 9_007_199_254_740_991;

pub struct RunsApp {
    pub appearance: Appearance,
    pub backend: Backend,
    /// The address, split at its slashes.
    pub path: Vec<String>,
    /// The settings over the page, and the section they show.
    pub settings: Option<String>,
    /// The search palette while it is open: what is typed and the row lit.
    pub palette: Option<(String, usize)>,
    /// The run shown beside the page, and its panel's tab.
    pub opened: Option<String>,
    pub panel_tab: usize,
    /// The runs asked to be cancelled, while the question stands.
    pub cancelling: Vec<String>,
    /// The runs page's search, filters, sort and chosen view.
    pub runs_view: crate::runs_page::RunsView,
    /// The jobs page's search and project, and the new job's form.
    pub jobs_view: crate::jobs_page::JobsView,
    /// The schedules page's span, jobs and chosen lane.
    pub schedules_view: crate::schedules_page::SchedulesView,
    /// What the sign-in page says went wrong.
    pub sign_in_error: Option<String>,
    /// The step chosen in a job page's graph.
    pub job_step: Option<String>,
    /// The step chosen in a run page's graph.
    pub run_step: Option<String>,
    /// The question before the demo's data goes back, while it is asked.
    pub resetting: bool,
    /// The job asked to be deleted, while the question stands.
    pub deleting: Option<String>,
    /// Each table's sort, widths and the row opened, by the table's name.
    pub tables: std::collections::HashMap<String, sokkel_egui::TableState>,
    started: i64,
    started_at: Option<f64>,
    elapsed: i64,
    ticked: i64,
    /// Whether the clock stands still, as the tests hold it.
    held: bool,
    /// The frame stands in a desktop window with its title bar hidden: set by the running app.
    pub desktop: bool,
    pub rail_collapsed: bool,
}

impl RunsApp {
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
        let mut backend = Backend::seeded(now);
        backend.start_engine(now);
        Self {
            appearance,
            backend,
            path: Vec::new(),
            settings: None,
            palette: None,
            opened: None,
            panel_tab: 0,
            cancelling: Vec::new(),
            runs_view: Default::default(),
            jobs_view: Default::default(),
            schedules_view: Default::default(),
            deleting: None,
            resetting: false,
            run_step: None,
            job_step: None,
            sign_in_error: None,
            tables: Default::default(),
            started: now,
            started_at: None,
            elapsed: 0,
            ticked: now,
            held: true,
            desktop: false,
            rail_collapsed: false,
        }
    }

    /// The moment it is now.
    pub fn now(&self) -> i64 {
        self.started + self.elapsed
    }

    pub fn prefs(&self) -> Prefs {
        self.backend.prefs.first().cloned().unwrap_or_default()
    }

    /// Who is signed in.
    pub fn me(&self) -> Person {
        self.prefs().user.unwrap_or(Person::Ada)
    }

    /// Every run as it stands now, newest first.
    pub fn live_runs(&self) -> Vec<LiveRun> {
        let now = self.now();
        self.backend
            .runs
            .iter()
            .map(|run| {
                let live = matches!(run.state, Status::Running | Status::Queued);
                let at = run_at(run, if live { now } else { LATER });
                LiveRun { run: run.clone(), at }
            })
            .collect()
    }

    pub fn go(&mut self, path: &str) {
        if path.trim_matches('/') == "settings" {
            self.settings = Some("general".into());
            return;
        }
        self.path = path
            .trim_matches('/')
            .split('/')
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect();
        // a run shown on its own page is not shown beside it as well
        if self.path.first().is_some_and(|p| p == "runs") && self.path.get(1) == self.opened.as_ref() {
            self.opened = None;
        }
    }

    pub fn first(&self) -> &str {
        self.path.first().map_or("overview", String::as_str)
    }

    /// Opens a run beside the page or on its own, as the person prefers.
    pub fn open(&mut self, id: &str) {
        if self.prefs().open_runs == OpenRuns::Page {
            self.opened = None;
            self.go(&format!("runs/{id}"));
        } else {
            self.opened = Some(id.to_owned());
        }
    }

    /// Starts a job now, saying so, and opens its run.
    pub fn run_now(&mut self, ctx: &egui::Context, job_id: &str) {
        let Some(job) = self.backend.job(job_id).cloned() else {
            return;
        };
        let run = self.backend.start_run(&job, self.me(), None, self.now());
        Toast::new(format!("{} queued", job.name))
            .kind(ToastKind::Info)
            .description(format!("{}, started by you", run.id))
            .show(ctx);
        self.open(&run.id);
    }

    /// Retries a finished run, saying so, and opens the new one.
    pub fn retry(&mut self, ctx: &egui::Context, run_id: &str) {
        let Some(run) = self.backend.run(run_id).cloned() else {
            return;
        };
        let Some(job) = self.backend.job(&run.job).cloned() else {
            Toast::new("The job is gone")
                .kind(ToastKind::Error)
                .description(format!("{} was deleted.", run.job))
                .show(ctx);
            return;
        };
        let next = self.backend.retry_run(&run, &job, self.me(), self.now());
        Toast::new(format!("{} queued again", job.name))
            .kind(ToastKind::Info)
            .description(format!("{} retries {}", next.id, run.id))
            .show(ctx);
        self.open(&next.id);
    }

    /// Cancels the runs going among `ids`, asking first when the person wants that.
    pub fn cancel(&mut self, ctx: &egui::Context, ids: &[String]) {
        let now = self.now();
        let some: Vec<String> = ids
            .iter()
            .filter(|id| self.backend.run(id).is_some_and(|run| is_live(run_at(run, now).state)))
            .cloned()
            .collect();
        if some.is_empty() {
            return;
        }
        if self.prefs().confirm_cancel {
            self.cancelling = some;
        } else {
            self.cancel_now(ctx, &some);
        }
    }

    fn cancel_now(&mut self, ctx: &egui::Context, ids: &[String]) {
        let (me, now) = (self.me(), self.now());
        for id in ids {
            self.backend.cancel_run(id, me, now);
        }
        Toast::new(if ids.len() == 1 {
            "Run cancelled".to_owned()
        } else {
            format!("{} runs cancelled", ids.len())
        })
        .description(ids.join(", "))
        .show(ctx);
    }

    /// Everything as the demo started: the jobs, two days of runs, the views and the settings; the
    /// look, the address and the clock stay.
    pub fn reset_demo(&mut self) {
        let now = self.now();
        let mut backend = Backend::seeded(now);
        backend.start_engine(now);
        self.backend = backend;
        self.resetting = false;
        self.opened = None;
        self.cancelling.clear();
        self.deleting = None;
        self.runs_view = Default::default();
        self.jobs_view = Default::default();
        self.schedules_view = Default::default();
        self.tables.clear();
    }

    /// The console's door after signing out, as the web's `SignInPage.tsx`: ada, grace, linus or
    /// margaret sign in with the password "tide".
    fn sign_in_page(&mut self, ui: &mut Ui) {
        let mut sign_in = sokkel_egui::SignIn::new("Tidevann")
            .mark(TIDEVANN_MARK)
            .eyebrow("Welcome to Tidevann")
            .description("Sign in as ada, grace, linus or margaret, with the password tide.")
            .facts([
                sokkel_egui::SignInFact::new("instance", "tidevann.example.org"),
                sokkel_egui::SignInFact::new("console", "jobs and runs"),
            ])
            .art(sokkel_egui::FlowLines::new())
            .remember(true)
            .resizable(true)
            .error(self.sign_in_error.as_deref());
        if self.desktop {
            sign_in = sign_in.desktop(sokkel_egui::DesktopWindow::native());
        }
        if let Some(values) = sign_in.show_window(ui) {
            let user = values.user.trim().to_lowercase();
            match crate::model::PEOPLE.iter().find(|p| p.word() == user) {
                Some(person) if values.password == "tide" => {
                    self.sign_in_error = None;
                    if let Some(mine) = self.backend.prefs.first_mut() {
                        mine.user = Some(*person);
                    }
                }
                _ => self.sign_in_error = Some("That username and password do not match.".into()),
            }
        }
    }

    /// Signs out, back to the sign-in page.
    pub fn sign_out(&mut self) {
        if let Some(mine) = self.backend.prefs.first_mut() {
            mine.user = None;
        }
    }

    /// Pauses a job's schedule or resumes it, saying so.
    pub fn toggle_pause(&mut self, ctx: &egui::Context, id: &str) {
        let Some(job) = self.backend.save_job(id, |job| job.paused = !job.paused) else {
            return;
        };
        let (title, detail) = if job.paused {
            (
                format!("{} paused", job.name),
                "Its schedule starts no runs until it is resumed.",
            )
        } else {
            (format!("{} resumed", job.name), "Its schedule runs again.")
        };
        Toast::new(title).description(detail).show(ctx);
    }

    fn delete_question(&mut self, ctx: &egui::Context) {
        let Some(id) = self.deleting.clone() else {
            return;
        };
        let asked = ConfirmDialog::new("delete-job", format!("Delete {id}?"), "Delete job")
            .detail("Its runs going now are cancelled and its schedule stops. Its past runs stay in the history.")
            .destructive(true)
            .show(ctx);
        match asked {
            Some(ConfirmAnswer::Confirm) => {
                self.deleting = None;
                let name = self.backend.job(&id).map_or(id.clone(), |job| job.name.clone());
                self.backend.delete_job(&id, self.me(), self.now());
                Toast::new(format!("{name} deleted"))
                    .description(format!("{id} and its schedule are gone."))
                    .show(ctx);
                self.go("jobs");
            }
            Some(_) => self.deleting = None,
            None => {}
        }
    }

    /// Moves the clock and the engine on, saying what the person wants to hear of.
    fn tick(&mut self, ctx: &egui::Context) {
        if !self.held {
            let time = ctx.input(|i| i.time);
            let since = *self.started_at.get_or_insert(time);
            self.elapsed = ((time - since) * 1000.0) as i64;
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
        let now = self.now();
        if now - self.ticked < crate::backend::TICK {
            return;
        }
        self.ticked = now;
        let (prefs, me) = (self.prefs(), self.me());
        for settled in self.backend.tick(now) {
            let run = settled.run;
            let name = self
                .backend
                .job(&run.job)
                .map_or(run.job.clone(), |job| job.name.clone());
            let mine = run.by == crate::model::By::Person(me);
            if run.state == Status::Failed && (prefs.notify_failures || (mine && prefs.notify_mine)) {
                Toast::new(format!("{name} failed"))
                    .kind(ToastKind::Error)
                    .description(format!(
                        "{}: {}",
                        run.id,
                        run.error.clone().unwrap_or_else(|| "a step failed".into())
                    ))
                    .show(ctx);
            } else if run.state == Status::Succeeded && mine && prefs.notify_mine {
                Toast::success(format!("{name} succeeded"))
                    .description(format!("{}, started by you", run.id))
                    .show(ctx);
            }
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        let ctx = ui.ctx().clone();
        self.tick(&ctx);
        if self.prefs().user.is_none() {
            self.sign_in_page(ui);
            Toaster::new().show(&ctx);
            return;
        }
        let runs = self.live_runs();
        let now = self.now();
        let prefs = self.prefs();
        let first = self.first().to_owned();
        let second = self.path.get(1).cloned();
        let page = match first.as_str() {
            "overview" | "runs" | "schedules" | "jobs" => first.clone(),
            _ => "missing".into(),
        };
        let live = runs.iter().filter(|r| is_live(r.at.state)).count();
        let today = day_of(now);
        let failed_today = runs
            .iter()
            .filter(|r| r.at.state == Status::Failed && day_of(r.run.created) == today)
            .count();
        let entry = |id: &str, label: &str, icon: Icon, count: usize| {
            let entry = RailEntry::new(id, label, icon);
            if count > 0 { entry.count(count as u32) } else { entry }
        };
        let groups = vec![
            RailGroup::new(None, [entry("overview", "Overview", Icon::LayoutDashboard, 0)]),
            RailGroup::new(
                Some("Operate"),
                [
                    entry("runs", "Runs", Icon::Play, live),
                    entry("schedules", "Schedules", Icon::CalendarClock, 0),
                ],
            ),
            RailGroup::new(Some("Build"), [entry("jobs", "Jobs", Icon::Workflow, 0)]),
            RailGroup::new(Some("System"), [entry("settings", "Settings", Icon::Settings, 0)]),
        ];
        let label = match page.as_str() {
            "overview" => "Overview",
            "runs" => "Runs",
            "schedules" => "Schedules",
            "jobs" => "Jobs",
            _ => "Not found",
        };
        let mut trail: Vec<String> = vec!["Tidevann".into()];
        match (&second, first.as_str()) {
            (Some(id), "runs") => trail.extend([label.into(), id.clone()]),
            (Some(id), "jobs") => {
                let name = self.backend.job(id).map_or(id.clone(), |job| job.name.clone());
                trail.extend([label.into(), name]);
            }
            _ => trail.push(label.into()),
        }
        let trail_words: Vec<&str> = trail.iter().map(String::as_str).collect();
        let next_up = self
            .backend
            .jobs
            .iter()
            .filter(|job| !job.paused)
            .filter_map(|job| {
                job.schedule
                    .as_ref()
                    .and_then(|s| next_fire(s, now))
                    .map(|at| (job, at))
            })
            .min_by_key(|(_, at)| *at);
        let note = if !prefs.scheduler {
            "The scheduler is off".to_owned()
        } else {
            match next_up {
                None => "No schedule is on".to_owned(),
                Some((job, at)) => format!("Next: {} at {} UTC", job.id, clock(at, false)),
            }
        };
        let running = runs.iter().any(|r| r.at.state == Status::Running);
        let mut chosen = page.clone();
        let was = chosen.clone();
        let mut account = None;
        let mut gear = false;
        let mut searched = sokkel_egui::palette_shortcut(&ctx);
        let mut panel_open = self.opened.is_some();
        let shown = self
            .opened
            .as_ref()
            .and_then(|id| runs.iter().find(|r| &r.run.id == id))
            .cloned();
        let mut panel_asked = crate::run_panel::Asked::default();
        let mut shell = Shell::new().collapsed(self.rail_collapsed);
        if self.desktop {
            shell = shell.desktop(sokkel_egui::DesktopWindow::native());
        }
        let mut tab = self.panel_tab;
        if let Some(run) = &shown {
            let name = self.backend.job(&run.run.job).map(|job| job.name.clone());
            let (asked, tab, prefs) = (&mut panel_asked, &mut tab, &prefs);
            shell = shell.panel(&mut panel_open, move |ui| {
                *asked = crate::run_panel::run_panel(ui, run, name.as_deref(), prefs, now, tab);
            });
        }
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                ui.add(
                    Rail::new("Tidevann", &groups, &mut chosen)
                        .mark(TIDEVANN_MARK)
                        .collapsible(&mut self.rail_collapsed),
                );
            }
            ShellPart::RailFoot => {
                account = AccountMenu::new(self.me().word())
                    .place("tidevann.example.org")
                    .appearance(true)
                    .show(ui);
            }
            ShellPart::Top => {
                searched |= TopBar::new(&trail_words)
                    .search("Search", "⌘K")
                    .show(ui, |ui| {
                        gear = ui.add(IconButton::new(Icon::Settings, "Settings")).clicked();
                    })
                    .search;
            }
            ShellPart::Page => match (page.as_str(), &second) {
                ("runs", None) => Page::show(ui, |ui| self.runs_page(ui, &runs)),
                ("runs", Some(id)) => Page::scroll(ui, |ui| self.run_page(ui, &runs, id)),
                ("jobs", None) => Page::show(ui, |ui| self.jobs_page(ui, &runs)),
                ("jobs", Some(id)) => Page::scroll(ui, |ui| self.job_page(ui, &runs, id)),
                ("schedules", None) => Page::show(ui, |ui| self.schedules_page(ui)),
                _ => Page::scroll(ui, |ui| {
                    sokkel_egui::Empty::new("Nothing at this address")
                        .description(format!("The console has no page at #/{}.", self.path.join("/")))
                        .show(ui, |_| {});
                }),
            },
            ShellPart::Foot => {
                let mut line = StatusLine::new(&note).live(running);
                if !prefs.scheduler {
                    line = line.tone(StatusTone::Warn);
                }
                ui.add(line.item("going", live).item("failed today", failed_today));
            }
        });
        self.panel_tab = tab;
        if !panel_open {
            self.opened = None;
        }
        self.after_panel(&ctx, panel_asked);
        if chosen != was {
            self.go(if chosen == "overview" { "" } else { &chosen });
        }
        if gear {
            self.go("settings");
        }
        match account {
            Some(AccountAction::Settings) => self.go("settings"),
            Some(AccountAction::Appearance) => self.settings = Some("appearance".into()),
            None => {}
        }
        if searched {
            self.palette = Some((String::new(), 0));
        }
        self.search_palette(&ctx, &groups);
        self.settings_dialog(&ctx);
        self.cancel_question(&ctx);
        self.delete_question(&ctx);
        self.new_job_dialog(&ctx);
        Toaster::new().show(&ctx);
    }

    fn after_panel(&mut self, ctx: &egui::Context, asked: crate::run_panel::Asked) {
        let Some(id) = self.opened.clone() else {
            return;
        };
        if asked.page {
            self.opened = None;
            self.go(&format!("runs/{id}"));
        }
        if asked.retry {
            self.retry(ctx, &id);
        }
        if asked.cancel {
            self.cancel(ctx, &[id]);
        }
    }

    fn cancel_question(&mut self, ctx: &egui::Context) {
        if self.cancelling.is_empty() {
            return;
        }
        let question = if self.cancelling.len() == 1 {
            format!("Cancel {}?", self.cancelling[0])
        } else {
            format!("Cancel {} runs?", self.cancelling.len())
        };
        let confirm = if self.cancelling.len() == 1 {
            "Cancel run"
        } else {
            "Cancel runs"
        };
        let asked = ConfirmDialog::new("cancel-runs", question, confirm)
            .detail("The step running now stops, and the steps after it are skipped.")
            .destructive(true)
            .cancel("Keep running")
            .show(ctx);
        match asked {
            Some(ConfirmAnswer::Confirm) => {
                let ids = std::mem::take(&mut self.cancelling);
                self.cancel_now(ctx, &ids);
            }
            Some(_) => self.cancelling.clear(),
            None => {}
        }
    }
}

impl eframe::App for RunsApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "appearance", &self.appearance);
    }
}
