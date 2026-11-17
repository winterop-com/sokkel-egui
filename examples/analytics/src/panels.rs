//! The dashboard's panels, as the web's `panels.tsx` and `parts.tsx`: each reads the range,
//! comparison and segment of the view, so the overview, the revenue page and a report draw the
//! same panel the same way.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    ActivityItem, ActivityList, ActivityTone, ActivityWords, Button, ButtonSize, ButtonVariant, Cell, ChipTone, Column,
    DashboardPanel, DataTable, Icon, IconButton, PanelSpan, StatPanel, TableVariant,
};
use sokkel_egui_charts::{BarPanel, DonutPanel, FunnelPanel, HeatmapPanel, Series, TimeSeriesPanel, WaterfallPanel};

use crate::app::AnalyticsApp;
use crate::metrics::{Figures, Kpis, SIGNUP_FUNNEL, TopCustomer};
use crate::model::{BEFORE_COLOR, Change, ChangeKind, Pace, Plan, REGIONS, Team, WidgetId};
use crate::stream::StreamState;
use crate::view::range_name;

/// The change from one figure to another, for a reader: `+4.2%`.
pub fn change(now: f64, before: f64) -> Option<String> {
    if before == 0.0 {
        return (now == 0.0).then(|| "0%".to_owned());
    }
    let percent = (now - before) / before.abs() * 100.0;
    Some(format!("{}{percent:.1}%", if percent >= 0.0 { "+" } else { "" }))
}

/// The change between two percentages, in points: `+1.2 pts`.
pub fn points(now: f64, before: f64) -> String {
    let diff = now - before;
    format!("{}{diff:.1} pts", if diff >= 0.0 { "+" } else { "" })
}

/// A plan as a chip, as the web's `PlanChip` says it.
pub fn plan_chip(plan: Plan) -> Cell {
    let tone = match plan {
        Plan::Free => ChipTone::Neutral,
        Plan::Team => ChipTone::Info,
        Plan::Business => ChipTone::Good,
    };
    Cell::Chip(plan.name().into(), tone)
}

/// What a change says in the live feed, the icon and tone it is drawn in, and what it did to MRR.
pub struct Said {
    pub text: String,
    pub icon: Icon,
    pub tone: ActivityTone,
    pub mrr: i64,
}

pub fn describe(change: &Change, team: Option<&Team>) -> Said {
    let name = team.map_or(change.team.as_str(), |t| t.name.as_str());
    let index = team.and_then(|t| t.changes.iter().position(|one| one.id == change.id));
    let before = index.filter(|i| *i > 0).and_then(|i| team.map(|t| &t.changes[i - 1]));
    let was = before.map_or(0, |b| {
        if b.kind == ChangeKind::Churn {
            0
        } else {
            b.plan.price() * b.seats
        }
    });
    let now = if change.kind == ChangeKind::Churn {
        0
    } else {
        change.plan.price() * change.seats
    };
    let by = change.by.map_or(String::new(), |p| format!(", by {}", p.name()));
    match change.kind {
        ChangeKind::Signup => Said {
            text: format!(
                "{name} signed up from {}",
                team.map_or("somewhere", |t| t.country.as_str())
            ),
            icon: Icon::UserPlus,
            tone: ActivityTone::Info,
            mrr: 0,
        },
        ChangeKind::Upgrade => Said {
            text: format!("{name} moved to {} with {} seats{by}", change.plan.name(), change.seats),
            icon: Icon::ArrowUpRight,
            tone: ActivityTone::Good,
            mrr: now - was,
        },
        ChangeKind::Downgrade => Said {
            text: format!("{name} moved down to {}{by}", change.plan.name()),
            icon: Icon::ArrowDownRight,
            tone: ActivityTone::Warning,
            mrr: now - was,
        },
        ChangeKind::Seats => {
            let added = change.seats - before.map_or(0, |b| b.seats);
            if added >= 0 {
                Said {
                    text: format!(
                        "{name} bought {added} more {}{by}",
                        if added == 1 { "seat" } else { "seats" }
                    ),
                    icon: Icon::Plus,
                    tone: ActivityTone::Good,
                    mrr: now - was,
                }
            } else {
                Said {
                    text: format!(
                        "{name} let {} {} go{by}",
                        -added,
                        if added == -1 { "seat" } else { "seats" }
                    ),
                    icon: Icon::ArrowDownRight,
                    tone: ActivityTone::Warning,
                    mrr: now - was,
                }
            }
        }
        ChangeKind::Churn => Said {
            text: format!("{name} left {}{by}", change.plan.name()),
            icon: Icon::LogOut,
            tone: ActivityTone::Critical,
            mrr: -was,
        },
    }
}

/// How long ago a moment was, short: `12s ago`, `4m ago`.
fn ago(at: i64, now: i64) -> String {
    let seconds = crate::seed::round(((now - at) as f64 / 1000.0).max(0.0)) as i64;
    if seconds < 60 {
        return if seconds < 5 {
            "just now".into()
        } else {
            format!("{seconds}s ago")
        };
    }
    if seconds < 3600 {
        return format!("{}m ago", seconds / 60);
    }
    if seconds < 86_400 {
        return format!("{}h ago", seconds / 3600);
    }
    format!("{}d ago", seconds / 86_400)
}

/// A key figure a dashboard can show as a tile.
pub fn kpi_name(id: WidgetId) -> &'static str {
    match id {
        WidgetId::Revenue => "Revenue",
        WidgetId::Mrr => "MRR",
        WidgetId::Arr => "ARR",
        WidgetId::Active => "Active teams",
        WidgetId::Paying => "Paying teams",
        WidgetId::Arpa => "ARPA",
        WidgetId::Signups => "Sign-ups",
        WidgetId::Conversion => "Conversion",
        WidgetId::Churn => "Churn",
        WidgetId::Refunded => "Refunded",
        _ => "",
    }
}

impl AnalyticsApp {
    /// A widget of a dashboard, drawn as the web's `WIDGET_PARTS` draws it.
    pub fn widget(&mut self, ui: &mut Ui, id: WidgetId) {
        match id {
            WidgetId::RevenueChart => self.revenue_panel(ui),
            WidgetId::Funnel => self.funnel_panel(ui),
            WidgetId::Feed => self.feed_panel(ui, 7),
            WidgetId::Customers => self.customers_panel(ui),
            WidgetId::Regions => self.regions_panel(ui),
            WidgetId::Countries => self.countries_panel(ui),
            WidgetId::Plans => self.plans_panel(ui),
            WidgetId::Movements => self.movements_panel(ui),
            WidgetId::Cohorts => self.cohorts_panel(ui),
            kpi => self.kpi_panel(ui, kpi),
        }
    }

    /// One key figure as a tile, with its trend over the range and its change from the period before.
    pub fn kpi_panel(&mut self, ui: &mut Ui, id: WidgetId) {
        let segment = self.segment();
        let (span, previous) = self.spans();
        let now = self.kpis_of(segment, &span);
        let before: Option<Kpis> = self.view.compare.then(|| self.kpis_of(segment, &previous));
        let money = self.money();
        let short = move |v: f64| money.short(v);
        let full = move |v: f64| money.full(v);
        let title = kpi_name(id);
        fn versus<'t>(tile: StatPanel<'t>, a: f64, b: Option<f64>, higher_is_good: bool) -> StatPanel<'t> {
            match b {
                None => tile,
                Some(b) => {
                    let tile = tile.good(Some(if higher_is_good { a >= b } else { a <= b }));
                    match change(a, b) {
                        Some(delta) => tile.delta(delta),
                        None => tile,
                    }
                }
            }
        }
        match id {
            WidgetId::Revenue => versus(
                StatPanel::new(title, Some(now.revenue as f64))
                    .format(&short)
                    .color("chart-2")
                    .sparkline(&now.trend.revenue)
                    .info("Invoices paid in the range, less refunds."),
                now.revenue as f64,
                before.as_ref().map(|b| b.revenue as f64),
                true,
            )
            .show(ui),
            WidgetId::Mrr => versus(
                StatPanel::new(title, Some(now.mrr as f64))
                    .format(&short)
                    .color("chart-1")
                    .sparkline(&now.trend.mrr)
                    .info("What the paying teams pay each month, at the end of the range."),
                now.mrr as f64,
                before.as_ref().map(|b| b.mrr as f64),
                true,
            )
            .show(ui),
            WidgetId::Arr => StatPanel::new(title, Some(now.mrr as f64 * 12.0))
                .format(&short)
                .color("chart-1")
                .note("MRR times twelve")
                .show(ui),
            WidgetId::Active => versus(
                StatPanel::new(title, Some(now.active as f64))
                    .color("chart-3")
                    .sparkline(&now.trend.active),
                now.active as f64,
                before.as_ref().map(|b| b.active as f64),
                true,
            )
            .show(ui),
            WidgetId::Paying => versus(
                StatPanel::new(title, Some(now.paying as f64)).color("chart-2"),
                now.paying as f64,
                before.as_ref().map(|b| b.paying as f64),
                true,
            )
            .show(ui),
            WidgetId::Arpa => {
                let arpa = |of: &Kpis| {
                    if of.paying == 0 {
                        0.0
                    } else {
                        of.mrr as f64 / of.paying as f64
                    }
                };
                versus(
                    StatPanel::new(title, Some(arpa(&now)))
                        .format(&full)
                        .color("chart-3")
                        .info("Average revenue per account: MRR over the paying teams."),
                    arpa(&now),
                    before.as_ref().map(arpa),
                    true,
                )
                .show(ui)
            }
            WidgetId::Signups => versus(
                StatPanel::new(title, Some(now.signups as f64))
                    .color("chart-4")
                    .sparkline(&now.trend.signups),
                now.signups as f64,
                before.as_ref().map(|b| b.signups as f64),
                true,
            )
            .show(ui),
            WidgetId::Conversion | WidgetId::Churn => {
                let conversion = id == WidgetId::Conversion;
                let (value, trend, was) = if conversion {
                    (
                        now.conversion,
                        &now.trend.conversion,
                        before.as_ref().map(|b| b.conversion),
                    )
                } else {
                    (now.churn, &now.trend.churn, before.as_ref().map(|b| b.churn))
                };
                let mut tile = StatPanel::new(title, Some(value))
                    .unit("%")
                    .decimals(1)
                    .color(if conversion { "good" } else { "critical" })
                    .sparkline(trend);
                if !conversion {
                    tile = tile.info("Paying teams that left in the range, as a share of those paying at its start.");
                }
                if let Some(was) = was {
                    tile =
                        tile.delta(points(value, was))
                            .good(Some(if conversion { value >= was } else { value <= was }));
                }
                tile.show(ui);
            }
            WidgetId::Refunded => {
                let figures = Figures::new(&self.world);
                let (count, amount) = figures.refunds(segment, span.from, span.to);
                let was = self
                    .view
                    .compare
                    .then(|| figures.refunds(segment, previous.from, previous.to).1);
                let mut tile = StatPanel::new(title, Some(amount as f64))
                    .format(&full)
                    .color("warning")
                    .note(format!("{count} {}", if count == 1 { "invoice" } else { "invoices" }));
                if let Some(was) = was {
                    if let Some(delta) = change(amount as f64, was as f64) {
                        tile = tile.delta(delta);
                    }
                    tile = tile.good(Some(amount <= was));
                }
                tile.show(ui);
            }
            _ => {}
        }
    }

    /// Revenue over the range, by plan, with the period before as a dashed line.
    pub fn revenue_panel(&mut self, ui: &mut Ui) {
        let segment = self.segment();
        let (span, previous) = self.spans();
        let figures = Figures::new(&self.world);
        let (team, business) = figures.revenue_by_plan(segment, &span);
        let as_f64 = |values: Vec<Option<i64>>| values.into_iter().map(|v| v.map(|v| v as f64)).collect::<Vec<_>>();
        let mut panel = TimeSeriesPanel::new("Revenue", span.buckets.iter().map(|b| b.label.clone()))
            .subtitle(format!("by plan, {}", range_name(&self.view.range).to_lowercase()))
            .info("Invoices paid in each step of the range, stacked by plan. The dashed line is the same step of the period before.")
            .stacked(true)
            .fill(sokkel_egui_charts::Fill::Solid)
            .height(256.0)
            .empty(segment.plan == Some(Plan::Free));
        if segment.plan != Some(Plan::Business) {
            panel = panel.series(
                Series::gappy(Plan::Team.name(), as_f64(team))
                    .color(Plan::Team.color())
                    .stack("paid"),
            );
        }
        if segment.plan != Some(Plan::Team) {
            panel = panel.series(
                Series::gappy(Plan::Business.name(), as_f64(business))
                    .color(Plan::Business.color())
                    .stack("paid"),
            );
        }
        if self.view.compare {
            let before = figures.revenue_series(segment, &previous);
            panel = panel.series(
                Series::gappy("Period before", as_f64(before))
                    .color(BEFORE_COLOR)
                    .dashed(),
            );
        }
        let money = self.money();
        panel.format(move |v| money.full(v)).show(ui);
    }

    /// Visitors to paying teams, over the teams that signed up in the range.
    pub fn funnel_panel(&mut self, ui: &mut Ui) {
        let segment = self.segment();
        let (span, previous) = self.spans();
        let figures = Figures::new(&self.world);
        let stages = figures.funnel(segment, span.from, span.to, &SIGNUP_FUNNEL, 90);
        let before = self
            .view
            .compare
            .then(|| figures.funnel(segment, previous.from, previous.to, &SIGNUP_FUNNEL, 90));
        let paid = stages.last().map_or(0.0, |s| s.of_first);
        let mut subtitle = format!("{paid:.1}% paid");
        if let Some(last) = before.as_ref().and_then(|b| b.last()) {
            subtitle.push_str(&format!(", {:.1}% before", last.of_first));
        }
        FunnelPanel::new("Sign-up funnel", stages.iter().map(|s| (s.name.to_owned(), s.count as f64)))
            .subtitle(subtitle)
            .info("The teams that signed up in the range, and how far each got within 90 days: a project, a teammate invited, a paid plan.")
            .show(ui);
    }

    /// The latest changes, of the segment, newest first.
    fn feed_rows(&self, count: usize) -> Vec<(Change, Option<usize>)> {
        let segment = self.segment();
        self.world
            .latest
            .iter()
            .map(|change| (change.clone(), self.world.by_id.get(&change.team).copied()))
            .filter(|(change, team)| {
                segment
                    .region
                    .is_none_or(|r| team.is_some_and(|t| self.world.teams[t].region == r))
                    && segment.plan.is_none_or(|p| change.plan == p)
            })
            .take(count)
            .collect()
    }

    /// The live feed: the latest sign-ups, upgrades, orders and teams leaving, newest first, those
    /// since last read drawn stronger.
    pub fn feed_panel(&mut self, ui: &mut Ui, count: usize) {
        let money = self.money();
        let rows = self.feed_rows(count);
        let items: Vec<ActivityItem> = rows
            .iter()
            .map(|(change, team)| {
                let team = team.map(|t| &self.world.teams[t]);
                let said = describe(change, team);
                let mrr = if said.mrr == 0 {
                    String::new()
                } else {
                    format!(
                        ", {}{} MRR",
                        if said.mrr > 0 { "+" } else { "-" },
                        money.full(said.mrr.abs() as f64)
                    )
                };
                ActivityItem {
                    id: change.id.clone(),
                    text: format!("{}{mrr}", said.text),
                    when: ago(change.at, self.world.now),
                    icon: Some(said.icon),
                    tone: said.tone,
                    unread: change.at > self.prefs.feed_seen && !self.read.contains(&change.id),
                    action: team.map(|_| "Open".to_owned()),
                    ..Default::default()
                }
            })
            .collect();
        let unread = items.iter().filter(|i| i.unread).count();
        let paused = self.prefs.pace == Pace::Off;
        let state = if paused { StreamState::Off } else { self.stream.state };
        let subtitle = match state {
            StreamState::Live => format!("{unread} new"),
            StreamState::Off => "paused".into(),
            StreamState::Hidden => "waiting for the page".into(),
            StreamState::Full => "stopped at the limit".into(),
        };
        let mut mark_read = false;
        let mut toggle = false;
        let acted = DashboardPanel::new("Live")
            .subtitle(subtitle)
            .flush(true)
            .actions(|ui| {
                let (icon, label) = if paused {
                    (Icon::Play, "Resume the live stream")
                } else {
                    (Icon::Pause, "Pause the live stream")
                };
                toggle = ui.add(IconButton::new(icon, label).xs()).clicked();
                mark_read = ui
                    .add(
                        Button::new("Mark all read")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Xs)
                            .enabled(unread > 0),
                    )
                    .clicked();
            })
            .show(ui, |ui| {
                ActivityList::new(&items)
                    .head(false)
                    .words(ActivityWords {
                        empty: "Nothing has happened in this segment yet.".into(),
                        ..Default::default()
                    })
                    .show(ui)
            });
        if mark_read {
            self.prefs.feed_seen = self.now();
        }
        if toggle {
            self.toggle_stream();
        }
        if let Some(id) = acted.acted {
            if let Some((change, _)) = rows.iter().find(|(c, _)| c.id == id) {
                let path = format!("customers/{}", change.team);
                self.go(&path);
            }
            self.read.push(id);
        }
    }

    /// The teams paying most, with what they paid at the range's start.
    pub fn customers_panel(&mut self, ui: &mut Ui) {
        let segment = self.segment();
        let (span, _) = self.spans();
        let top: Vec<TopCustomer> = Figures::new(&self.world).top_customers(segment, &span, 8);
        let money = self.money();
        let mut columns = vec![
            Column::new("Team", 144.0).fill(),
            Column::new("Plan", 104.0),
            Column::new("Seats", 64.0).numeric(),
            Column::new("MRR", 88.0).numeric(),
        ];
        if self.view.compare {
            columns.push(Column::new("Change", 88.0).numeric());
        }
        let rows: Vec<Vec<Cell>> = top
            .iter()
            .map(|row| {
                let mut cells = vec![
                    Cell::Text(self.world.teams[row.team].name.clone()),
                    plan_chip(row.plan),
                    Cell::Text(row.seats.to_string()),
                    Cell::Text(money.full(row.mrr as f64)),
                ];
                if self.view.compare {
                    let said = if row.before == 0 {
                        "new".to_owned()
                    } else {
                        change(row.mrr as f64, row.before as f64).unwrap_or_else(|| "-".into())
                    };
                    let ink = if row.mrr >= row.before {
                        "muted-foreground"
                    } else {
                        "critical-ink"
                    };
                    cells.push(Cell::Toned(said, ink));
                }
                cells
            })
            .collect();
        let mut all = false;
        let mut opened = None;
        let empty = top.is_empty();
        let state = self.tables.entry("top-customers".into()).or_default();
        DashboardPanel::new("Top customers")
            .subtitle("by MRR")
            .flush(true)
            .actions(|ui| {
                all = ui
                    .add(
                        Button::new("All customers")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Xs),
                    )
                    .clicked();
            })
            .show(ui, |ui| {
                if empty {
                    ui.add_space(32.0);
                    ui.vertical_centered(|ui| ui.label("No paying team in this segment."));
                    ui.add_space(32.0);
                    return;
                }
                DataTable::new("top-customers", &columns, &rows, state)
                    .variant(TableVariant::List)
                    .framed(false)
                    .bar(false)
                    .footer(false)
                    .stack_below(420.0)
                    .show(ui);
                opened = state.opened.take();
            });
        if all {
            self.go("customers");
        }
        if let Some(row) = opened
            && let Some(one) = top.get(row)
        {
            let path = format!("customers/{}", self.world.teams[one.team].id);
            self.go(&path);
        }
    }

    /// Paid revenue by country on a map, each country a point sized and colored by it, with its
    /// change in a popup.
    pub fn countries_panel(&mut self, ui: &mut Ui) {
        use sokkel_egui_maps::{MapLegend, MapPopup, MapPopupRow, Marker, class_of, point_radius, sequential_colors};
        let segment = self.segment();
        let (span, previous) = self.spans();
        let figures = Figures::new(&self.world).countries(segment, &span, &previous);
        let most = figures.iter().map(|f| f.revenue).max().unwrap_or(0).max(1) as f64;
        let breaks: Vec<f64> = [0.1, 0.25, 0.5, 0.75]
            .iter()
            .map(|share| crate::seed::round(most * share))
            .collect();
        let total: i64 = figures.iter().map(|f| f.revenue).sum();
        let money = self.money();
        let range = range_name(&self.view.range);
        let compare = self.view.compare;
        let t = sokkel_egui::tokens(ui.ctx());
        let colors = sequential_colors(&t, "chart-2", breaks.len() + 1);
        let markers: Vec<Marker> = figures
            .iter()
            .map(|f| {
                Marker::new(f.at.1, f.at.0, "")
                    .id(f.name)
                    .color(colors[class_of(&breaks, f.revenue as f64)])
                    .radius(point_radius(f.revenue as f64, most, 4.0, 20.0))
            })
            .collect();
        if !self.map_framed {
            let (mut west, mut south, mut east, mut north) = (180.0f64, 90.0f64, -180.0f64, -90.0f64);
            for f in &figures {
                west = west.min(f.at.0);
                east = east.max(f.at.0);
                south = south.min(f.at.1);
                north = north.max(f.at.1);
            }
            self.map.frame(Some([west, south, east, north]));
            self.map_framed = true;
        }
        let map = &mut self.map;
        DashboardPanel::new("Revenue by country")
            .subtitle(format!("{}, {}", money.short(total as f64), range.to_lowercase()))
            .flush(true)
            .show(ui, |ui| {
                let room = sokkel_egui::body_room(ui);
                let height = 320.0 + room;
                sokkel_egui::took_room(ui, room);
                let response = map.show(ui, egui::vec2(ui.available_width(), height), &markers, &[]);
                MapLegend::new("Revenue")
                    .description(range.clone())
                    .ramp(colors.clone(), "Less", "More")
                    .show(ui, &response);
                let chosen = map
                    .selected
                    .clone()
                    .and_then(|id| figures.iter().find(|f| f.name == id).cloned());
                if let Some(chosen) = chosen {
                    let mut popup = MapPopup::new(chosen.name).description(chosen.region.name()).row(
                        MapPopupRow::new("Revenue", money.full(chosen.revenue as f64))
                            .color(colors[class_of(&breaks, chosen.revenue as f64)]),
                    );
                    if compare {
                        popup = popup
                            .row(MapPopupRow::new("Period before", money.full(chosen.before as f64)))
                            .row(MapPopupRow::new(
                                "Change",
                                change(chosen.revenue as f64, chosen.before as f64).unwrap_or_else(|| "new".into()),
                            ));
                    }
                    let share = if total == 0 {
                        0.0
                    } else {
                        chosen.revenue as f64 / total as f64 * 100.0
                    };
                    popup = popup.row(MapPopupRow::new("Share of all", format!("{share:.1}%")));
                    if popup.show(ui, map).closed {
                        map.selected = None;
                    }
                }
            });
    }

    /// Paid revenue over the range, by plan, as a donut.
    pub fn plans_panel(&mut self, ui: &mut Ui) {
        let segment = self.segment();
        let (span, _) = self.spans();
        let figures = Figures::new(&self.world);
        let slices: Vec<(String, f64, &'static str)> = [Plan::Team, Plan::Business]
            .iter()
            .map(|plan| {
                let of = crate::metrics::Segment {
                    plan: Some(*plan),
                    ..segment
                };
                (
                    plan.name().to_owned(),
                    figures.revenue(of, span.from, span.to) as f64,
                    plan.color(),
                )
            })
            .collect();
        let empty = slices.iter().all(|s| s.1 == 0.0);
        let money = self.money();
        DonutPanel::new("Revenue by plan", slices)
            .subtitle(range_name(&self.view.range).to_lowercase())
            .total("revenue")
            .format(move |v| money.short(v))
            .empty(empty)
            .show(ui);
    }

    /// Where MRR went over the range: what new teams added, what teams paying more and less
    /// changed, what teams leaving took, and the net change.
    pub fn movements_panel(&mut self, ui: &mut Ui) {
        let segment = self.segment();
        let (span, _) = self.spans();
        let figures = Figures::new(&self.world);
        let flow = figures.flows(segment, span.from, span.to);
        let start = figures.levels(segment, &[span.from]).mrr[0] as f64;
        let steps = vec![
            ("New".to_owned(), flow.new_mrr as f64),
            ("Expansion".to_owned(), flow.expansion as f64),
            ("Contraction".to_owned(), -(flow.contraction as f64)),
            ("Churn".to_owned(), -(flow.churn_mrr as f64)),
        ];
        let net: f64 = steps.iter().map(|s| s.1).sum();
        let money = self.money();
        WaterfallPanel::new("MRR movements", steps)
            .subtitle(format!("{} to {}", money.short(start), money.short(start + net)))
            .info("What teams starting to pay added to MRR over the range, what teams paying more or less changed, what teams leaving took, and the net change.")
            .total("Net")
            .format(move |v| money.short(v))
            .empty(segment.plan == Some(Plan::Free))
            .show(ui);
    }

    /// Retention by cohort: the share of each month's new paying teams still paying each month after.
    pub fn cohorts_panel(&mut self, ui: &mut Ui) {
        let segment = self.segment();
        let (span, _) = self.spans();
        let grid = Figures::new(&self.world).cohorts(segment, span.last);
        let cells: Vec<(usize, usize, f64)> = grid
            .cells
            .iter()
            .filter_map(|cell| {
                let x = grid.months.iter().position(|m| *m == cell.month)?;
                let y = grid.order.iter().position(|c| *c == cell.cohort)?;
                Some((x, y, cell.retained))
            })
            .collect();
        let empty = cells.is_empty();
        HeatmapPanel::new("Retention by cohort", grid.months.clone(), grid.order.clone(), cells)
            .subtitle("paying teams, by the month they first paid")
            .info("Each row is the teams that first paid in a month; each cell the share of them still paying that many months later.")
            .scale(["chart-2"])
            .unit("%")
            .height(320.0)
            .empty(empty)
            .show(ui);
    }

    /// Paid revenue by region, beside the period before.
    pub fn regions_panel(&mut self, ui: &mut Ui) {
        let segment = self.segment();
        let (span, previous) = self.spans();
        let figures = Figures::new(&self.world);
        let shown: Vec<_> = REGIONS
            .iter()
            .filter(|r| segment.region.is_none_or(|one| one == **r))
            .copied()
            .collect();
        let now = figures.revenue_by_region(segment, span.from, span.to);
        let mut panel = BarPanel::new("Revenue by region", shown.iter().map(|r| r.name()))
            .subtitle(range_name(&self.view.range).to_lowercase())
            .horizontal(true)
            .height(256.0)
            .empty(segment.plan == Some(Plan::Free))
            .series(Series::new("This period", shown.iter().map(|r| now[r] as f64)).color("chart-2"));
        if self.view.compare {
            let before = figures.revenue_by_region(segment, previous.from, previous.to);
            panel =
                panel.series(Series::new("Period before", shown.iter().map(|r| before[r] as f64)).color(BEFORE_COLOR));
        }
        let money = self.money();
        panel.format(move |v| money.full(v)).show(ui);
    }
}

/// How wide a widget starts once added.
pub fn widget_span(id: WidgetId) -> PanelSpan {
    match id {
        WidgetId::RevenueChart | WidgetId::Countries => PanelSpan::TwoThirds,
        WidgetId::Plans | WidgetId::Funnel | WidgetId::Feed => PanelSpan::Third,
        WidgetId::Customers | WidgetId::Movements | WidgetId::Regions => PanelSpan::Half,
        WidgetId::Cohorts => PanelSpan::Full,
        _ => PanelSpan::Sixth,
    }
}
