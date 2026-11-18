//! The parts the status example brought into the library: each in its story, with Tidevann's data.

use eframe::egui::{self, Ui, vec2};
use sokkel_egui::{
    ActivityItem, ActivityList, ActivityTone, AppBar, Banner, BannerTone, BarList, BarListItem, Button, ButtonSize,
    ButtonVariant, CalendarEvent, Chip, ChipBar, ChipTone, Choice, ChoiceList, DashboardArrange, DashboardEditActions,
    DashboardGrid, DashboardLayoutItem, DashboardPanel, DashboardWidget, DateTimeField, Day, DescriptionList, FileKind,
    FilePreview, Gap, Icon, IconButton, KpiRow, LogLevel, LogLine, LogView, Markdown, Moment, PanelSpan, PersonRow,
    SearchField, Section, SplitPane, Stack, Stat, StatPanel, Switch, TagChips, TextField, TextRole, Threshold,
    Timeline, TimelineEvent, Tree, TreeNode, UptimeBar, UptimeDay, WeekGrid, form_bar, tokens,
};
use sokkel_egui_charts::{BarPanel, GaugePanel, Series, TimeSeriesPanel};

use super::{State, section};

/// Tidevann's mark, as sokkel-ui's stories draw it (`fixtures/TidevannMark`).
const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// The day the stories are set on, so they draw the same every time.
fn today() -> Day {
    Day::parse("2026-11-16").expect("a day")
}

pub fn icon_button(_: &mut State, ui: &mut Ui) {
    section(ui, "Beside a title");
    ui.horizontal(|ui| {
        let t = tokens(ui.ctx());
        ui.label(TextRole::Head.rich(&t, "nightly-export").color(t.color("foreground")));
        ui.add(IconButton::new(Icon::Pencil, "Rename nightly-export"));
        ui.add(IconButton::new(Icon::Copy, "Copy its id"));
        ui.add(IconButton::new(Icon::Trash2, "Delete nightly-export"));
    });
}

pub fn choice_list(state: &mut State, ui: &mut Ui) {
    ui.set_max_width(448.0);
    section(ui, "With a line under a choice");
    let choices = [
        Choice {
            value: "all",
            label: "Every service",
            description: None,
        },
        Choice {
            value: "some",
            label: "Some services",
            description: Some("Only the ones ticked below."),
        },
        Choice {
            value: "none",
            label: "No email",
            description: Some("The status page alone."),
        },
    ];
    ChoiceList::new("Hear about", &choices).show(ui, &mut state.choice);
}

pub fn date_time_field(state: &mut State, ui: &mut Ui) {
    ui.set_max_width(448.0);
    section(ui, "A window's start");
    DateTimeField::new("story-starts", "Starts", &mut state.starts, today())
        .min(sokkel_egui::Moment::new(today(), 9, 0))
        .show(ui);
    section(ui, "Before the earliest");
    let mut early = Some(sokkel_egui::Moment::new(today(), 8, 0));
    DateTimeField::new("story-early", "Starts", &mut early, today())
        .min(sokkel_egui::Moment::new(today(), 9, 0))
        .show(ui);
}

/// Ninety days of api, two of them with incidents.
fn days() -> Vec<UptimeDay> {
    (0..90)
        .map(|i| {
            let mut day = UptimeDay::new(today().plus(i - 89), Some(1.0));
            match i {
                41 => {
                    day.uptime = Some(0.9921);
                    day.status = Some("critical");
                    day.note = Some("Elevated error rates on the api".into());
                }
                86 => {
                    day.uptime = Some(0.9984);
                    day.status = Some("warning");
                    day.note = Some("Slow exports from storage".into());
                }
                12 => day.uptime = None,
                _ => {}
            }
            day
        })
        .collect()
}

pub fn uptime_bar(_: &mut State, ui: &mut Ui) {
    ui.set_max_width(560.0);
    section(ui, "The last ninety days");
    UptimeBar::new("story-uptime", &days()).show(ui);
}

pub fn chip_bar(state: &mut State, ui: &mut Ui) {
    section(ui, "Regions");
    let chips = [
        ("All", "All"),
        ("Europe", "Europe"),
        ("North America", "North America"),
        ("South America", "South America"),
        ("Africa", "Africa"),
        ("Asia", "Asia"),
        ("Oceania", "Oceania"),
    ];
    ChipBar::new("story-regions", &chips).show(ui, &mut state.region);
}

pub fn split_pane(_: &mut State, ui: &mut Ui) {
    section(ui, "Two panes, the rule dragged");
    let t = tokens(ui.ctx());
    let room = ui.available_rect_before_wrap();
    let height = 320.0_f32.min(room.height());
    ui.allocate_ui(vec2(room.width(), height), |ui| {
        ui.set_min_size(vec2(room.width(), height));
        SplitPane::new("story-split").default_share(0.6).show(
            ui,
            |ui| {
                ui.add_space(12.0);
                ui.label(
                    TextRole::Body
                        .rich(&t, "The sites on a map")
                        .color(t.color("muted-foreground")),
                );
            },
            |ui| {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label(
                        TextRole::Body
                            .rich(&t, "The sites in a table")
                            .color(t.color("muted-foreground")),
                    );
                });
            },
        );
    });
}

pub fn tag_chips(_: &mut State, ui: &mut Ui) {
    section(ui, "What an incident touches");
    let tags: Vec<String> = ["storage", "worker", "Frankfurt"]
        .into_iter()
        .map(String::from)
        .collect();
    ui.add(TagChips::new(&tags));
}

pub fn app_bar(_: &mut State, ui: &mut Ui) {
    section(ui, "A screen of its own");
    let trail = ["Tidevann status"];
    AppBar::new(&trail).mark(TIDEVANN_MARK).show(ui, |ui| {
        ui.add(IconButton::new(Icon::LayoutDashboard, "Open the board"));
        ui.add(
            Button::new("Subscribe")
                .icon(Icon::BellRing)
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Sm),
        );
    });
}

pub fn form_bar_story(_: &mut State, ui: &mut Ui) {
    ui.set_max_width(560.0);
    section(ui, "At a form's end");
    form_bar(ui, Some("No more than one email an update."), |ui| {
        ui.add(Button::new("Subscribe"));
    });
}

pub fn banner(_: &mut State, ui: &mut Ui) {
    for (title, tone, words) in [
        ("Good", BannerTone::Good, "All systems operational, as of 09:00:04"),
        ("Info", BannerTone::Info, "Maintenance in progress"),
        ("Warning", BannerTone::Warning, "Some systems are degraded"),
        ("Critical", BannerTone::Critical, "Some systems are down"),
    ] {
        section(ui, title);
        Banner::new(tone, words).show(ui);
    }
}

pub fn timeline(_: &mut State, ui: &mut Ui) {
    ui.set_max_width(640.0);
    let t = tokens(ui.ctx());
    section(ui, "An incident's updates");
    Timeline::new([
        TimelineEvent {
            title: "Exports are back to their usual speed. We are watching nightly-export finish.".into(),
            at: "9 minutes ago".into(),
            detail: Some("grace · Monitoring".into()),
            tone: Some(t.color("info")),
            ..Default::default()
        },
        TimelineEvent {
            title: "A full disk in Frankfurt slows writes to storage. We are moving exports to Oslo.".into(),
            at: "31 minutes ago".into(),
            detail: Some("grace · Identified".into()),
            tone: Some(t.color("warning")),
            ..Default::default()
        },
        TimelineEvent {
            title: "nightly-export is writing to storage at a third of its usual speed.".into(),
            at: "48 minutes ago".into(),
            detail: Some("grace · Investigating".into()),
            tone: Some(t.color("critical")),
            ..Default::default()
        },
    ])
    .show(ui);
    section(ui, "With icons");
    Timeline::new([
        TimelineEvent {
            icon: Some(Icon::PhoneCall),
            title: "linus to grace".into(),
            at: "2 days ago".into(),
            detail: Some("INC-41 is in monitoring; nightly-export should finish by morning.".into()),
            ..Default::default()
        },
        TimelineEvent {
            icon: Some(Icon::PhoneCall),
            title: "ada to linus".into(),
            at: "last week".into(),
            ..Default::default()
        },
    ])
    .show(ui);
}

pub fn description_list(state: &mut State, ui: &mut Ui) {
    ui.set_max_width(384.0);
    section(ui, "Stacked, values changed in place");
    let tags: Vec<String> = ["storage", "worker", "Frankfurt"]
        .into_iter()
        .map(String::from)
        .collect();
    let shown = &mut state.notify;
    DescriptionList::new()
        .item("Touches", |ui| {
            ui.add(TagChips::new(&tags));
        })
        .text("Opened", "Mon, Nov 16, 08:12")
        .text("Resolved", "Not yet")
        .item("On the status page", |ui| {
            ui.add(Switch::new(shown, "On the status page"));
        })
        .show(ui);
}

pub fn sections(_: &mut State, ui: &mut Ui) {
    ui.set_max_width(640.0);
    section(ui, "With a meta, a description and actions");
    Section::new("Slow exports from storage")
        .description("Affects storage, worker, Frankfurt")
        .actions(|ui| {
            ui.add(Chip::new("monitoring").tone(ChipTone::Info));
        })
        .show(ui, |ui| {
            let t = tokens(ui.ctx());
            ui.label(
                TextRole::Body
                    .rich(&t, "The section's body.")
                    .color(t.color("foreground")),
            );
        });
    section(ui, "People");
    Section::new("Now").meta("on call").flush(true).show(ui, |ui| {
        PersonRow::new("grace").detail("since Monday, 09:00").show(ui, |ui| {
            ui.add(Chip::new("on call").tone(ChipTone::Good));
        });
        PersonRow::new("linus").detail("escalation").show(ui, |_| {});
    });
}

pub fn stats(_: &mut State, ui: &mut Ui) {
    section(ui, "Key figures in a row");
    KpiRow::new([
        Stat::new("Services operational", "8 of 8"),
        Stat::new("Open incidents", 1).note("INC-41"),
        Stat::new("Uptime, 90 days", "99.91%"),
        Stat::new("Requests", "7,245").unit("/s").note("api p95 180 ms"),
    ])
    .show(ui);
}

/// Sixty readings, two seconds apart, of a calm service.
fn readings() -> (Vec<String>, Vec<f64>, Vec<f64>) {
    let x = (0..60)
        .map(|i| {
            let s = 8 * 3600 + 58 * 60 + 8 + i * 2;
            format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
        })
        .collect();
    let p95 = (0..60).map(|i| 176.0 + 8.0 * ((i as f64) * 0.4).sin()).collect();
    let p50 = (0..60).map(|i| 74.0 + 3.0 * ((i as f64) * 0.3).cos()).collect();
    (x, p95, p50)
}

pub fn dashboard(_: &mut State, ui: &mut Ui) {
    section(ui, "Panels in a grid");
    let (x, p95, p50) = readings();
    let days = days();
    DashboardGrid::new("story-dashboard").show(ui, |grid| {
        let x2 = x.clone();
        grid.cell(PanelSpan::Half, move |ui| {
            TimeSeriesPanel::new("Latency", x2)
                .subtitle("live, in milliseconds")
                .series(Series::new("p95", p95))
                .series(Series::new("p50", p50))
                .unit("ms")
                .label_every(15)
                .show(ui);
        });
        grid.cell(PanelSpan::Half, move |ui| {
            DashboardPanel::new("Uptime")
                .subtitle("99.92% over 90 days")
                .footer("Press a day to see what happened on it.")
                .show(ui, |ui| {
                    Stack::new(Gap::Lg).show(ui, |ui| {
                        UptimeBar::new("story-dashboard-uptime", &days).show(ui);
                    });
                });
        });
    });
}

pub fn text_area(state: &mut State, ui: &mut Ui) {
    ui.set_max_width(448.0);
    section(ui, "Several lines");
    ui.add(
        TextField::new("First update", &mut state.update)
            .placeholder("What you know now, and what happens next.")
            .multiline(true),
    );
}

/// A week of Tidevann's runs: two overlapping on Monday night, one skipped on Thursday.
pub fn week_grid(_: &mut State, ui: &mut Ui) {
    let days: Vec<Day> = (0..7).map(|d| today().plus(d)).collect();
    let run = |id: &str, title: &str, day: i32, from: (u32, u32), to: (u32, u32), color: &'static str| CalendarEvent {
        id: id.into(),
        title: title.into(),
        start: Moment::new(today().plus(day), from.0, from.1),
        end: Moment::new(today().plus(day), to.0, to.1),
        color,
        detail: None,
        muted: false,
    };
    let mut events = vec![
        run("export-mon", "nightly-export", 0, (2, 0), (2, 45), "blue"),
        run("index-mon", "rebuild-index", 0, (2, 30), (4, 0), "violet"),
        run("sync-tue", "sync-inventory", 1, (6, 30), (6, 50), "green"),
        run("digest-wed", "send-digest", 2, (7, 0), (7, 15), "pink"),
        run("orders-wed", "import-orders", 2, (8, 0), (9, 30), "teal"),
        run("index-thu", "rebuild-index", 3, (3, 0), (4, 30), "violet"),
        run("export-fri", "nightly-export", 4, (2, 0), (2, 45), "blue"),
    ];
    events[5].muted = true;
    events[4].detail = Some("juniper".into());
    ui.allocate_ui(vec2(ui.available_width(), 560.0), |ui| {
        WeekGrid::new("story-week", &days, &events)
            .now(Moment::new(today().plus(2), 5, 0))
            .scroll_to_hour(1)
            .show(ui);
    });
}

/// Tidevann's harbor and juniper buckets as a tree.
fn bucket_nodes() -> Vec<TreeNode> {
    let file = |path: &str, name: &str, size: &str| {
        TreeNode::new(format!("{path}/{name}"), name)
            .icon(FileKind::of_name(name).icon())
            .meta(size)
    };
    let folder = |id: &str, name: &str, children: Vec<TreeNode>| {
        TreeNode::new(id, name)
            .icon(Icon::Folder)
            .open_icon(Icon::FolderOpen)
            .meta(children.len().to_string())
            .children(children)
    };
    vec![
        folder(
            "harbor",
            "harbor-exports",
            vec![
                folder(
                    "harbor/nightly-export",
                    "nightly-export",
                    vec![
                        file("harbor/nightly-export", "orders.csv", "532 B"),
                        file("harbor/nightly-export", "summary.json", "180 B"),
                        file("harbor/nightly-export", "run.log", "413 B"),
                    ],
                ),
                file("harbor", "README.md", "131 B"),
            ],
        ),
        folder(
            "juniper",
            "juniper-inventory",
            vec![
                file("juniper", "stock.csv", "1.1 KB"),
                file("juniper", "warehouses.json", "128 B"),
            ],
        ),
    ]
}

pub fn tree(state: &mut State, ui: &mut Ui) {
    ui.set_max_width(360.0);
    let nodes = bucket_nodes();
    section(ui, "Open");
    Tree::new("story-tree", &nodes, &mut state.tree_chosen, &mut state.tree_open).show(ui);
    section(ui, "Searched");
    let mut open = std::collections::HashSet::new();
    let mut chosen = None;
    SearchField::new("story-tree-search", &mut state.tree_query)
        .placeholder("Search")
        .show(ui);
    let query = if state.tree_query.is_empty() {
        "csv"
    } else {
        state.tree_query.as_str()
    };
    Tree::new("story-tree-searched", &nodes, &mut chosen, &mut open)
        .query(query)
        .show(ui);
}

pub fn files(_: &mut State, ui: &mut Ui) {
    ui.set_max_width(640.0);
    section(ui, "A table");
    FilePreview::new("story-csv", "stock.csv", FileKind::Table)
        .text("sku,warehouse,on_hand\nsku-01000,oslo-1,212\nsku-01037,porto-2,48\nsku-01074,kobe-1,305\n")
        .show(ui);
    section(ui, "Code");
    FilePreview::new("story-code", "schedule.toml", FileKind::Code)
        .text("[rebuild-index]\ndays = [\"mon\", \"thu\"]\nat = \"03:00\"\nminutes = 90\n")
        .show(ui);
    section(ui, "No preview");
    FilePreview::new("story-archive", "index.tar.gz", FileKind::Archive).show(ui);
}

/// The arranged dashboard as it comes.
pub fn arranged_default() -> Vec<DashboardLayoutItem> {
    vec![
        DashboardLayoutItem::new("latency", PanelSpan::Half),
        DashboardLayoutItem::new("uptime", PanelSpan::Half),
        DashboardLayoutItem::new("runs", PanelSpan::Third),
        DashboardLayoutItem::new("failed", PanelSpan::Third),
        DashboardLayoutItem::new("queued", PanelSpan::Third),
    ]
}

fn arranged_label(item: &DashboardLayoutItem) -> String {
    match item.id.split('-').next().unwrap_or_default() {
        "latency" => "Latency",
        "uptime" => "Uptime",
        "runs" => "Runs today",
        "failed" => "Failed",
        "queued" => "Queued",
        _ => "Note",
    }
    .into()
}

pub fn arranged(state: &mut State, ui: &mut Ui) {
    // the story opens in edit mode, so its picture shows the tools
    if !state.arrange_edit.editing() {
        let kept = state.arranged.clone();
        state.arrange_edit.edit(&kept);
    }
    let default = arranged_default();
    let row = vec2(ui.available_width(), 32.0);
    ui.allocate_ui_with_layout(row, egui::Layout::right_to_left(egui::Align::Center), |ui| {
        DashboardEditActions::new(&mut state.arrange_edit, &mut state.arranged)
            .default_layout(&default)
            .show(ui);
    });
    ui.add_space(12.0);
    let palette = [
        DashboardWidget::new("note", "Note").icon(Icon::StickyNote),
        DashboardWidget::new("runs", "Runs today").icon(Icon::Activity),
    ];
    let editing = state.arrange_edit.editing();
    let layout = state.arrange_edit.layout(&mut state.arranged);
    let (x, p95, p50) = readings();
    let days = days();
    let label = arranged_label;
    let shown = DashboardArrange::new("story-arranged", layout, &label)
        .arranging(editing)
        .palette(&palette)
        .show(ui, |ui, item| match item.id.split('-').next().unwrap_or_default() {
            "latency" => TimeSeriesPanel::new("Latency", x.clone())
                .subtitle("in milliseconds")
                .series(Series::new("p95", p95.clone()))
                .series(Series::new("p50", p50.clone()))
                .unit("ms")
                .label_every(15)
                .show(ui),
            "uptime" => {
                DashboardPanel::new("Uptime")
                    .subtitle("99.92% over 90 days")
                    .show(ui, |ui| {
                        UptimeBar::new(("story-arranged-uptime", &item.id), &days).show(ui);
                    });
            }
            _ => {
                let figure = match item.id.split('-').next().unwrap_or_default() {
                    "runs" => "148",
                    "failed" => "3",
                    "queued" => "12",
                    _ => "Rerun nightly-export once the api is back.",
                };
                DashboardPanel::new(arranged_label(item)).show(ui, |ui| {
                    let t = tokens(ui.ctx());
                    ui.label(TextRole::Statistic.rich(&t, figure).color(t.color("foreground")));
                });
            }
        });
    if let Some((widget, index)) = shown.added {
        let layout = state.arrange_edit.layout(&mut state.arranged);
        let id = format!("{widget}-{}", layout.len());
        let span = palette
            .iter()
            .find(|w| w.id == widget)
            .map_or(PanelSpan::Third, |w| w.span);
        layout.insert(index.min(layout.len()), DashboardLayoutItem::new(id, span));
    }
}

pub fn dashboard_panels(_: &mut State, ui: &mut Ui) {
    const LATENCY: [Threshold; 3] = [
        Threshold::base("good"),
        Threshold::from(200.0, "warning"),
        Threshold::from(300.0, "critical"),
    ];
    const CPU: [Threshold; 3] = [
        Threshold::base("good"),
        Threshold::from(70.0, "warning"),
        Threshold::from(85.0, "critical"),
    ];
    let runs = [38.0, 41.0, 44.0, 40.0, 46.0, 49.0, 45.0, 51.0, 48.0, 53.0, 55.0, 52.0];
    let latency = [
        150.0, 162.0, 171.0, 190.0, 214.0, 236.0, 228.0, 241.0, 256.0, 249.0, 262.0, 271.0,
    ];
    DashboardGrid::new("story-dashboard-panels").show(ui, |grid| {
        grid.cell(PanelSpan::Sixth, move |ui| {
            StatPanel::new("Runs", Some(55.0))
                .unit("/min")
                .color("chart-1")
                .sparkline(&runs)
                .delta("+12%")
                .good(Some(true))
                .show(ui);
        });
        grid.cell(PanelSpan::Sixth, move |ui| {
            StatPanel::new("api latency p95", Some(271.0))
                .unit("ms")
                .thresholds(&LATENCY)
                .sparkline(&latency)
                .delta("+80%")
                .good(Some(false))
                .show(ui);
        });
        grid.cell(PanelSpan::Sixth, |ui| {
            StatPanel::new("Queue depth", None).show(ui);
        });
        grid.cell(PanelSpan::Sixth, |ui| {
            GaugePanel::new("Worker CPU", Some(74.0))
                .unit("%")
                .thresholds(&CPU)
                .show(ui);
        });
        grid.cell(PanelSpan::Third, |ui| {
            BarPanel::new(
                "Runs by job",
                [
                    "nightly-export",
                    "sync-inventory",
                    "rebuild-index",
                    "send-digest",
                    "import-orders",
                ],
            )
            .subtitle("since midnight")
            .series(Series::new("Runs", [52.0, 47.0, 44.0, 49.0, 48.0]))
            .horizontal(true)
            .show(ui);
        });
    });
}

pub fn activity_list(_: &mut State, ui: &mut Ui) {
    ui.set_max_width(448.0);
    let item = |id: &str, text: &str, when: &str, group: &str, icon, tone, unread, action: Option<&str>| ActivityItem {
        id: id.into(),
        text: text.into(),
        when: when.into(),
        group: Some(group.into()),
        icon: Some(icon),
        tone,
        unread,
        action: action.map(Into::into),
    };
    let items = [
        item(
            "a",
            "nightly-export failed at upload in harbor",
            "4 min ago",
            "Today",
            Icon::CircleX,
            ActivityTone::Critical,
            true,
            Some("Open"),
        ),
        item(
            "b",
            "grace deployed worker 2.14.0",
            "10 min ago",
            "Today",
            Icon::Rocket,
            ActivityTone::Info,
            true,
            None,
        ),
        item(
            "c",
            "sync-inventory succeeded in juniper",
            "1 h ago",
            "Today",
            Icon::CircleCheck,
            ActivityTone::Good,
            false,
            None,
        ),
        item(
            "d",
            "Oslo is slow",
            "18 h ago",
            "Yesterday",
            Icon::TriangleAlert,
            ActivityTone::Warning,
            false,
            None,
        ),
    ];
    egui::Frame::new()
        .fill(tokens(ui.ctx()).color("card"))
        .stroke(egui::Stroke::new(1.0, tokens(ui.ctx()).color("border")))
        .corner_radius(tokens(ui.ctx()).radius("surface"))
        .show(ui, |ui| {
            ActivityList::new(&items).show(ui);
        });
}

pub fn bar_list(_: &mut State, ui: &mut Ui) {
    ui.set_max_width(448.0);
    let items = [
        BarListItem {
            label: "harbor".into(),
            value: 0.41,
            chosen: true,
            note: Some("98 runs".into()),
        },
        BarListItem {
            label: "juniper".into(),
            value: 0.34,
            chosen: false,
            note: Some("82 runs".into()),
        },
        BarListItem {
            label: "larkspur".into(),
            value: 0.25,
            chosen: false,
            note: Some("60 runs".into()),
        },
    ];
    BarList::new(&items).show(ui);
}

pub fn log_view(_: &mut State, ui: &mut Ui) {
    let line = |at: &str, level, text: &str, project: &str, ms: &str| LogLine {
        at: Some(at.into()),
        level: Some(level),
        source: Some("worker".into()),
        text: text.into(),
        fields: vec![("project".into(), project.into()), ("ms".into(), ms.into())],
    };
    let lines = [
        line("09:00:01", LogLevel::Info, "picked up run", "harbor", "12"),
        line("09:00:02", LogLevel::Debug, "heartbeat", "harbor", "4"),
        line("09:00:04", LogLevel::Info, "step finished", "juniper", "88"),
        line(
            "09:00:05",
            LogLevel::Warning,
            "slow transform for sync-inventory, 6s",
            "juniper",
            "142",
        ),
        line("09:00:07", LogLevel::Info, "wrote 1,204 rows", "larkspur", "61"),
        line(
            "09:00:08",
            LogLevel::Error,
            "nightly-export failed at upload: connection reset",
            "harbor",
            "177",
        ),
        line("09:00:10", LogLevel::Info, "uploaded the archive", "harbor", "35"),
    ];
    LogView::new("story-log", &lines).height(200.0).show(ui);
}

pub fn markdown(_: &mut State, ui: &mut Ui) {
    ui.set_max_width(448.0);
    Markdown::new(
        "### Hand-over

The person on call keeps an eye on **Recent failures** and hands over at *09:00*, after `nightly-export` is done.

1. Read the log of the worker
2. Retry what failed overnight
3. Write down what is left here

- Press **Edit** to move widgets
- Drag a widget in from the palette",
    )
    .show(ui);
}

pub fn inline_code(_: &mut State, ui: &mut Ui) {
    use sokkel_egui::{InlineCode, InlineCodeTone};
    section(ui, "Tones");
    ui.horizontal(|ui| {
        ui.add(InlineCode::new("run_toowb0pi"));
        ui.add(InlineCode::new("null").tone(InlineCodeTone::Muted));
        ui.add(InlineCode::new("exit 1").tone(InlineCodeTone::Critical));
        ui.add(InlineCode::new("retrying").tone(InlineCodeTone::Warning));
        ui.add(InlineCode::new("queued").tone(InlineCodeTone::Info));
        ui.add(InlineCode::new("exit 0").tone(InlineCodeTone::Good));
    });
    section(ui, "Bare and small");
    ui.horizontal(|ui| {
        ui.add(InlineCode::new("duration_ms").bare(true));
        ui.add(InlineCode::new("api.example.org").role(sokkel_egui::TextRole::Help));
    });
    section(ui, "Cut short");
    ui.allocate_ui(vec2(160.0, 24.0), |ui| {
        ui.add(InlineCode::new(
            "https://storage.example.org/juniper/import-orders/run_toowb0pi.csv",
        ));
    });
}

pub fn windowed_pane(_: &mut State, ui: &mut Ui) {
    use sokkel_egui::{WindowedPane, WindowedPaneWhere, WindowedPaneWords, windowed_pane_button};
    let held = ui.scope_id().with("story-windowed-open");
    let mut open = ui.data(|d| d.get_temp::<bool>(held)).unwrap_or(true);
    let words = WindowedPaneWords::default();
    ui.set_max_width(480.0);
    let mut expand = false;
    WindowedPane::new("story-windowed", "nightly-export.yaml")
        .keep_pane(true)
        .aside(|ui| {
            ui.label("The step kinds and what each takes.");
        })
        .show(ui, &mut open, |ui, place| {
            ui.horizontal(|ui| {
                ui.label(if place == WindowedPaneWhere::Pane {
                    "In the pane"
                } else {
                    "In the window"
                });
                if place == WindowedPaneWhere::Pane {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        expand = windowed_pane_button(ui, &words);
                    });
                }
            });
            ui.label("name: nightly-export");
            ui.label("schedule: every night at 02:00");
        });
    if expand {
        open = true;
    }
    ui.data_mut(|d| d.insert_temp(held, open));
}
