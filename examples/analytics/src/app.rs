//! The analytics app, as the web's `Analytics.tsx`, `state.tsx` and `App.tsx`: the rail of its
//! pages, the page shown with its range and segment, the live stream and the notes it leaves, and
//! the status line of MRR and live changes.

use std::collections::HashMap;

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, DashboardEdit, DesktopWindow, Icon, IconButton, Page, Rail, RailEntry,
    RailGroup, Shell, ShellPart, StatusLine, StatusTone, TableState, Toast, ToastKind, Toaster, TopBar, install,
};

use crate::days::counted;
use crate::metrics::{EVERYONE, Figures, Kpis, Segment};
use crate::model::{BoardId, ChangeKind, Pace, Placed, Plan, Prefs, Region, default_layout};
use crate::stream::{LATENCY, SETTLE, Stream, StreamState};
use crate::view::{Range, Span, View, previous_of, span_of};
use crate::world::{Base, Kept, World, build};

/// 2026-11-18 09:00 UTC: the moment the app's screenshots start at, as the web's are shot.
pub const DEMO_START: i64 = 1_794_992_400_000;

/// The Tidevann mark the rail heads with.
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// The pages of the rail, in its order.
pub const PAGES: [&str; 6] = ["overview", "revenue", "customers", "funnels", "reports", "goals"];

/// A page's name in the trail.
pub fn label_of(page: &str) -> &'static str {
    match page {
        "overview" => "Overview",
        "revenue" => "Revenue",
        "customers" => "Customers",
        "funnels" => "Funnels",
        "reports" => "Reports",
        "goals" => "Goals",
        "settings" => "Settings",
        _ => "Not found",
    }
}

/// Dollars written in the reader's currency, as the web's `money`: whole, or short.
#[derive(Clone, Copy, Debug)]
pub struct Money(pub crate::model::Currency);

impl Money {
    fn sign(&self, value: f64) -> (&'static str, &'static str) {
        let symbol = match self.0 {
            crate::model::Currency::Usd => "$",
            crate::model::Currency::Eur => "€",
            crate::model::Currency::Nok => "NOK\u{a0}",
        };
        (if value < 0.0 { "-" } else { "" }, symbol)
    }

    /// `$554,767`.
    pub fn full(&self, dollars: f64) -> String {
        let value = dollars * self.0.rate();
        let (minus, symbol) = self.sign(value);
        format!(
            "{minus}{symbol}{}",
            sokkel_egui::format_value(value.abs().round(), Some(0))
        )
    }

    /// `$554.8K`.
    pub fn short(&self, dollars: f64) -> String {
        let value = dollars * self.0.rate();
        let (minus, symbol) = self.sign(value);
        format!("{minus}{symbol}{}", sokkel_egui::format_compact(value.abs()))
    }
}

/// What a count of the key figures is kept under: the segment, the span's ends and its steps.
type KpiKey = (Option<Plan>, Option<Region>, i64, i64, usize);

pub struct AnalyticsApp {
    pub appearance: Appearance,
    /// The app stands in a desktop window with its title bar hidden.
    pub desktop: bool,
    /// The clock: what it read as the app started, and the frame time then, so it runs on.
    pub started: i64,
    started_at: Option<f64>,
    /// The time the app has run, in milliseconds, past `started`.
    pub elapsed: i64,
    /// A clock held still for the screenshots, moved only by `run_for`.
    pub held: bool,
    pub base: Base,
    pub kept: Kept,
    pub world: World,
    /// A rebuild of the world asked for, at the moment it is due.
    rebuild_at: Option<i64>,
    pub stream: Stream,
    pub prefs: Prefs,
    pub view: View,
    /// The settings over the page.
    pub settings: bool,
    pub layouts: HashMap<BoardId, Vec<Placed>>,
    pub edit: DashboardEdit<Placed>,
    /// The feed's items read by their Open, beside those before `feed_seen`.
    pub read: Vec<String>,
    pub tables: HashMap<String, TableState>,
    /// The key figures of a segment and span, counted once per world.
    kpis: HashMap<KpiKey, Kpis>,
    /// The saved funnels and the one being built.
    pub funnels: crate::funnels::FunnelsPage,
    /// A customer's page: its dialog and questions.
    pub customer: crate::customer::CustomerPage,
    /// The customers page's filters and search.
    pub customers: crate::customers::CustomersPage,
    /// Revenue's invoices: their views and the listing's setup.
    pub invoices: crate::invoices::InvoicesPage,
    /// The map of revenue by country, and whether it has been framed on its points.
    pub map: sokkel_egui_maps::MapView,
    pub map_framed: bool,
    /// Where a toast's Open goes, by the toast's id.
    pub opens: HashMap<u64, String>,
    said: Vec<(Toast, Option<String>)>,
}

impl AnalyticsApp {
    /// The app in its window, its clock the computer's and its days read at its zone.
    pub fn new(cc: &eframe::CreationContext<'_>, local_offset: i64) -> Self {
        crate::days::set_local_offset(local_offset);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(DEMO_START, |d| d.as_millis() as i64);
        let mut app = Self::at(now, Appearance::default());
        // the street map only in the running app: tests draw the map without the network
        sokkel_egui_maps::install(cc.wgpu_render_state.as_ref());
        app.map = std::mem::replace(&mut app.map, sokkel_egui_maps::MapView::new(0.0, 0.0, 1.0)).with_tiles(
            &cc.egui_ctx,
            "tidevann-analytics/0.1 (an example app of a component library)",
            std::env::temp_dir().join("sokkel-egui-tiles").into(),
        );
        app.desktop = true;
        app.held = false;
        app
    }

    /// The app as it opens at `now`, held still: the world as the backend first answers, 170 ms
    /// in, and the stream's first wait drawn.
    pub fn at(now: i64, appearance: Appearance) -> Self {
        let prefs = Prefs {
            feed_seen: now,
            ..Prefs::default()
        };
        let base = Base::new(now + LATENCY + SETTLE);
        let kept = Kept::default();
        let world = build(&base, &kept, now + LATENCY + SETTLE);
        Self {
            appearance,
            desktop: false,
            started: now,
            started_at: None,
            elapsed: 0,
            held: true,
            stream: Stream::new(now, prefs.pace),
            view: View {
                path: Vec::new(),
                range: Range::preset(&prefs.range),
                compare: prefs.compare,
                plan: None,
                region: None,
            },
            prefs,
            base,
            kept,
            world,
            rebuild_at: None,
            settings: false,
            layouts: HashMap::new(),
            edit: DashboardEdit::default(),
            read: Vec::new(),
            tables: HashMap::new(),
            kpis: HashMap::new(),
            invoices: Default::default(),
            customers: Default::default(),
            customer: Default::default(),
            funnels: crate::funnels::FunnelsPage::new(now),
            map: sokkel_egui_maps::MapView::new(30.0, 10.0, 1.0).framed(false),
            map_framed: false,
            opens: HashMap::new(),
            said: Vec::new(),
        }
    }

    /// The moment it is now.
    pub fn now(&self) -> i64 {
        self.started + self.elapsed
    }

    /// Runs the app's own time on by `ms`, as the web's clock runs in its screenshots: each event
    /// the stream sends and each rebuild of the world on its moment.
    pub fn run_for(&mut self, ms: i64) {
        let until = self.now() + ms;
        loop {
            let due = [self.stream.next, self.rebuild_at]
                .into_iter()
                .flatten()
                .filter(|at| *at <= until)
                .min();
            let Some(at) = due else { break };
            self.elapsed = at - self.started;
            self.step();
        }
        self.elapsed = until - self.started;
    }

    /// What is due now: the world built again, and the stream's next event.
    fn step(&mut self) {
        let now = self.now();
        if self.rebuild_at.is_some_and(|at| at <= now) {
            self.rebuild_at = None;
            self.world = build(&self.base, &self.kept, now);
            self.kpis.clear();
        }
        if let Some(at) = self.stream.next
            && at <= now
        {
            let before = self.kept.teams.len() + self.kept.changes.len();
            let event = self.stream.tick(&self.world, &mut self.kept, self.prefs.pace, now);
            let kept = self.kept.teams.len() + self.kept.changes.len() - before;
            // each thing kept takes the backend its while; the world is built once the last lands
            let landed = now + LATENCY * kept as i64;
            if kept > 0 {
                self.rebuild_at = Some(landed + SETTLE);
            }
            if let Some(event) = event {
                self.note(&event);
            }
            self.stream.wait(landed, self.prefs.pace);
        }
    }

    /// The world built again now, from what the backend keeps.
    pub fn rebuild(&mut self) {
        self.world = build(&self.base, &self.kept, self.now());
        self.kpis.clear();
    }

    /// The notes the stream leaves in the corner, as the settings ask: a big order, a team leaving.
    fn note(&mut self, event: &crate::stream::StreamEvent) {
        let money = self.money();
        let open = Some(format!("customers/{}", event.change.team));
        if self.prefs.big_order > 0 && event.billed >= self.prefs.big_order {
            let description = if event.change.kind == ChangeKind::Upgrade {
                format!(
                    "Moved to {} with {} seats",
                    event.change.plan.name(),
                    event.change.seats
                )
            } else {
                "Bought more seats".to_owned()
            };
            let toast = Toast::success(format!("{}: {} billed", event.team, money.full(event.billed as f64)))
                .description(description)
                .action("Open");
            self.said.push((toast, open.clone()));
        }
        if self.prefs.notify_churn && event.change.kind == ChangeKind::Churn {
            let mut toast = Toast::new(format!("{} left {}", event.team, event.change.plan.name()))
                .kind(ToastKind::Warning)
                .action("Open");
            if let Some((_, paid)) = event.paid_before {
                toast = toast.description(format!("It paid {} a month.", money.full(paid as f64)));
            }
            self.said.push((toast, open));
        }
    }

    pub fn money(&self) -> Money {
        Money(self.prefs.currency)
    }

    pub fn segment(&self) -> Segment {
        Segment {
            plan: self.view.plan,
            region: self.view.region,
        }
    }

    /// The span the view names and the one before it, as the world stands.
    pub fn spans(&self) -> (Span, Span) {
        let span = span_of(&self.view.range, self.world.now);
        let previous = previous_of(&span);
        (span, previous)
    }

    /// The key figures of a segment and span, counted once per world, as the web's `kpisOnce`.
    pub fn kpis_of(&mut self, segment: Segment, span: &Span) -> Kpis {
        let key = (segment.plan, segment.region, span.from, span.to, span.buckets.len());
        if let Some(found) = self.kpis.get(&key) {
            return found.clone();
        }
        let found = Figures::new(&self.world).kpis(segment, span);
        self.kpis.insert(key, found.clone());
        found
    }

    /// The page shown, the first part of the path.
    pub fn page(&self) -> &str {
        self.view.path.first().map_or("overview", String::as_str)
    }

    pub fn go(&mut self, path: &str) {
        self.view.path = path.split('/').filter(|p| !p.is_empty()).map(str::to_owned).collect();
        if self.edit.editing() {
            self.edit.cancel();
        }
    }

    pub fn toggle_stream(&mut self) {
        self.prefs.pace = if self.prefs.pace == Pace::Off {
            self.prefs.resume.unwrap_or(Pace::Fast)
        } else {
            self.prefs.resume = Some(self.prefs.pace);
            Pace::Off
        };
    }

    /// The overview's layout or the revenue page's, as the person arranged it.
    pub fn layout_of(&mut self, board: BoardId) -> &mut Vec<Placed> {
        self.layouts.entry(board).or_insert_with(|| default_layout(board))
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        if !self.held {
            let time = ui.input(|i| i.time);
            let since = *self.started_at.get_or_insert(time);
            let until = ((time - since) * 1000.0) as i64;
            if until > self.elapsed {
                self.run_for(until - self.elapsed);
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
        }
        let figures = Figures::new(&self.world);
        let now = figures.levels(EVERYONE, &[self.world.now]);
        let (paying, mrr) = (now.paying[0], now.mrr[0]);
        let page = self.page().to_owned();
        let entry =
            |id: &str, icon: Icon, description: &str| RailEntry::new(id, label_of(id), icon).description(description);
        let groups = vec![
            RailGroup::new(
                None,
                [entry(
                    "overview",
                    Icon::LayoutDashboard,
                    "Key figures and the live feed",
                )],
            ),
            RailGroup::new(
                Some("Sales"),
                [
                    entry("revenue", Icon::Banknote, "MRR, breakdowns and invoices"),
                    entry("customers", Icon::Users, "Every team and what it pays").count(paying as u32),
                ],
            ),
            RailGroup::new(
                Some("Explore"),
                [
                    entry("funnels", Icon::ListFilter, "How far teams get, step by step"),
                    entry("reports", Icon::FileText, "Panels put together, as CSV"),
                    entry("goals", Icon::Target, "Targets read live"),
                ],
            ),
            RailGroup::new(
                Some("System"),
                [entry("settings", Icon::Settings, "Preferences and demo data")],
            ),
        ];
        let mut place = if self.settings {
            "settings".to_owned()
        } else {
            page.clone()
        };
        let was = place.clone();
        let shell = if self.desktop {
            Shell::new().desktop(DesktopWindow::native())
        } else {
            Shell::new()
        };
        let workspace = self.prefs.workspace.clone();
        let trail: Vec<String> = match page.as_str() {
            "customers" if self.view.path.len() > 1 => {
                let id = &self.view.path[1];
                let name = self.world.team(id).map_or(id.clone(), |t| t.name.clone());
                vec![workspace.clone(), label_of("customers").into(), name]
            }
            "revenue" if self.view.path.get(1).is_some_and(|p| p == "invoices") => {
                vec![workspace.clone(), label_of("revenue").into(), "Invoices".into()]
            }
            _ => vec![workspace.clone(), label_of(&page).into()],
        };
        let trail: Vec<&str> = trail.iter().map(String::as_str).collect();
        let paused = self.prefs.pace == Pace::Off;
        let state = if paused { StreamState::Off } else { self.stream.state };
        let note = match state {
            StreamState::Live => match self.prefs.pace {
                Pace::Fast => "Live: an event every second or two",
                _ => "Live: an event every few seconds",
            },
            StreamState::Off => "Live stream paused",
            StreamState::Hidden => "Live stream waits while the page is hidden",
            StreamState::Full => "Live stream stopped at its limit: reset the demo data to start it again",
        };
        let tone = if state == StreamState::Full {
            StatusTone::Warn
        } else {
            StatusTone::Quiet
        };
        let live = state == StreamState::Live;
        let money = self.money();
        let live_count = self.world.live_count;
        let mut account = None;
        let mut toggled = false;
        let mut searched = sokkel_egui::palette_shortcut(ui.ctx());
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                ui.add(Rail::new(&workspace, &groups, &mut place).mark(TIDEVANN_MARK));
            }
            ShellPart::RailFoot => {
                account = AccountMenu::new("ada")
                    .local(true)
                    .scheme(self.appearance.scheme)
                    .show(ui);
            }
            ShellPart::Top => {
                searched |= TopBar::new(&trail)
                    .search("Search", "⌘K")
                    .show(ui, |ui| {
                        let (icon, label) = if paused {
                            (Icon::Play, "Resume the live stream")
                        } else {
                            (Icon::Pause, "Pause the live stream")
                        };
                        toggled = ui.add(IconButton::new(icon, label)).clicked();
                    })
                    .search;
            }
            ShellPart::Page if page == "overview" => Page::scroll(ui, |ui| self.overview(ui)),
            ShellPart::Page if page == "revenue" && self.view.path.get(1).is_some_and(|p| p == "invoices") => {
                Page::show(ui, |ui| self.revenue(ui))
            }
            ShellPart::Page if page == "revenue" => Page::scroll(ui, |ui| self.revenue(ui)),
            ShellPart::Page if page == "customers" && self.view.path.len() == 1 => {
                Page::show(ui, |ui| self.customers_page(ui))
            }
            ShellPart::Page if page == "customers" => {
                let id = self.view.path[1].clone();
                Page::scroll(ui, |ui| self.customer_page(ui, &id))
            }
            ShellPart::Page if page == "funnels" => Page::scroll(ui, |ui| self.funnels_page(ui)),
            ShellPart::Page => {}
            ShellPart::Foot => {
                ui.add(
                    StatusLine::new(note)
                        .tone(tone)
                        .live(live)
                        .item("MRR", money.short(mrr as f64))
                        .item("live", counted(live_count, "change")),
                );
            }
        });
        if toggled {
            self.toggle_stream();
        }
        if place != was {
            if place == "settings" {
                self.settings = true;
            } else {
                self.settings = false;
                self.go(if place == "overview" { "" } else { &place });
            }
        }
        let _ = searched;
        for (toast, open) in self.said.drain(..) {
            let id = toast.show(ui.ctx());
            if let Some(open) = open {
                self.opens.insert(id, open);
            }
        }
        for id in sokkel_egui::toast_acted(ui.ctx()) {
            if let Some(open) = self.opens.remove(&id) {
                self.view.path = open.split('/').map(str::to_owned).collect();
            }
        }
        Toaster::new().show(ui.ctx());
        match account {
            Some(AccountAction::Scheme(scheme)) => self.appearance.scheme = scheme,
            Some(AccountAction::Settings) => self.settings = true,
            None => {}
        }
    }
}

impl eframe::App for AnalyticsApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }
}
