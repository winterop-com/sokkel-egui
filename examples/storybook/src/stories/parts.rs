//! The parts the status example brought into the library: each in its story, with Tidevann's data.

use eframe::egui::{Ui, vec2};
use sokkel_ui_egui::{
    AppBar, Banner, BannerTone, Button, ButtonSize, ButtonVariant, Chip, ChipBar, ChipTone, Choice, ChoiceList,
    DashboardGrid, DashboardPanel, DateTimeField, Day, DescriptionList, Gap, Icon, IconButton, KpiRow, PanelSpan,
    PersonRow, Section, SplitPane, Stack, Stat, Switch, TagChips, TextField, TextRole, Timeline, TimelineEvent,
    UptimeBar, UptimeDay, form_bar, tokens,
};
use sokkel_ui_egui_charts::{Series, TimeSeriesPanel};

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
        .min(sokkel_ui_egui::Moment::new(today(), 9, 0))
        .show(ui);
    section(ui, "Before the earliest");
    let mut early = Some(sokkel_ui_egui::Moment::new(today(), 8, 0));
    DateTimeField::new("story-early", "Starts", &mut early, today())
        .min(sokkel_ui_egui::Moment::new(today(), 9, 0))
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
