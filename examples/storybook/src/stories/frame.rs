use eframe::egui::{Id, Sense, Stroke, Ui, UiBuilder, vec2};
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, Chip, ChipTone, Icon, Rail, RailEntry, RailGroup, Shell, ShellPart, SidePanel,
    StatusLine, StatusTone, TextRole, TopBar, panel_section, tokens,
};

use super::{State, section};

/// Tidevann's places, grouped as its rail groups them.
fn places() -> Vec<RailGroup> {
    vec![
        RailGroup::new(None, [RailEntry::new("overview", "Overview", Icon::LayoutDashboard)]),
        RailGroup::new(
            Some("Operate"),
            [
                RailEntry::new("runs", "Runs", Icon::Play).count(12),
                RailEntry::new("schedules", "Schedules", Icon::CalendarClock),
            ],
        ),
        RailGroup::new(Some("Build"), [RailEntry::new("jobs", "Jobs", Icon::Box)]),
        RailGroup::new(Some("System"), [RailEntry::new("settings", "Settings", Icon::Settings)]),
    ]
}

/// One app frame at `size`: the rail, the top bar with the trail, a page of runs and the foot.
fn frame(state: &mut State, ui: &mut Ui, size: eframe::egui::Vec2, collapsed: bool) {
    let t = tokens(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let groups = places();
    let place = groups
        .iter()
        .flat_map(|g| &g.entries)
        .find(|e| e.id == state.place)
        .map_or("Runs".to_owned(), |e| e.label.clone());
    Shell::new()
        .collapsed(collapsed)
        .framed(true)
        .show(ui, rect, |part, ui| match part {
            ShellPart::Rail => {
                ui.add(Rail::new("Tidevann", &groups, &mut state.place).collapsed(collapsed));
            }
            ShellPart::RailFoot => {
                sokkel_egui::AccountMenu::new("ada")
                    .place("tidevann.example.org")
                    .show(ui);
            }
            ShellPart::Top => {
                TopBar::new(&["Tidevann", &place]).show(ui, |ui| {
                    ui.add(Button::new("New").icon(Icon::Plus).size(ButtonSize::Sm));
                    ui.add(
                        Button::new("Search")
                            .icon(Icon::Search)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm),
                    );
                });
            }
            ShellPart::Page => {
                let inset = t.density("page-inset-wide");
                ui.add_space(t.density("page-top"));
                ui.horizontal(|ui| {
                    ui.add_space(inset);
                    ui.vertical(|ui| {
                        ui.label(TextRole::PageTitle.rich(&t, &place).color(t.color("foreground")));
                        ui.add_space(12.0);
                        eframe::egui::Grid::new(("runs", collapsed))
                            .num_columns(2)
                            .spacing(vec2(32.0, 12.0))
                            .show(ui, |ui| {
                                for (job, word, tone) in [
                                    ("nightly-export", "succeeded", ChipTone::Good),
                                    ("sync-inventory", "running", ChipTone::Info),
                                    ("rebuild-index", "failed", ChipTone::Critical),
                                    ("send-digest", "queued", ChipTone::Neutral),
                                ] {
                                    ui.label(TextRole::Body.rich(&t, job).color(t.color("foreground")));
                                    ui.add(Chip::new(word).tone(tone));
                                    ui.end_row();
                                }
                            });
                    });
                });
            }
            ShellPart::Foot => {
                ui.add(
                    StatusLine::new("sync-inventory is running")
                        .tone(StatusTone::Live)
                        .live(true)
                        .fact("api, worker, scheduler")
                        .fact("v2.4.0"),
                );
            }
        });
}

/// One side panel `width` wide and 200 tall, its tabs `tabs` with `icons`, the shown tab held in
/// memory under `id`.
fn side_panel(ui: &mut Ui, id: &str, width: f32, tabs: &[&str], icons: &[Icon]) {
    let t = tokens(ui.ctx());
    let key = Id::unique(("story-side-panel", id));
    let mut active = ui.data(|d| d.get_temp::<usize>(key)).unwrap_or(1);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 200.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, t.color("sidebar"));
    ui.painter().rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, t.color("border-strong")),
        eframe::egui::StrokeKind::Inside,
    );
    let mut inner = ui.new_child(UiBuilder::new().max_rect(rect));
    inner.set_clip_rect(rect);
    SidePanel::new(id, tabs, &mut active)
        .icons(icons)
        .show(&mut inner, |tab, ui| {
            panel_section(ui, tabs[tab], |ui| {
                ui.label(
                    TextRole::Body
                        .rich(&t, "nightly-export ran at 02:00 UTC")
                        .color(t.color("foreground")),
                );
            });
        });
    ui.data_mut(|d| d.insert_temp(key, active));
}

pub fn side_panel_story(_: &mut State, ui: &mut Ui) {
    let tabs = ["Details", "Steps", "Log", "Outputs"];
    let icons = [Icon::Info, Icon::ListChecks, Icon::ScrollText, Icon::Package];
    section(ui, "Labels that fit");
    side_panel(ui, "wide", 352.0, &tabs, &icons);
    section(ui, "Too narrow: each tab its icon, its label in a tooltip");
    side_panel(ui, "icons", 220.0, &tabs, &icons);
    section(ui, "Too narrow without icons: the labels scroll");
    side_panel(ui, "scroll", 220.0, &tabs, &[]);
}

pub fn status_line_story(_: &mut State, ui: &mut Ui) {
    let line = |ui: &mut Ui, width: f32| {
        let (rect, _) = ui.allocate_exact_size(vec2(width, 28.0), Sense::hover());
        let mut inner = ui.new_child(UiBuilder::new().max_rect(rect));
        inner.add(
            StatusLine::new("sync-inventory is running")
                .tone(StatusTone::Live)
                .fact("api, worker, scheduler")
                .item("next", "send-digest at 09:15 UTC")
                .item_code("version", "v2.4.0-rc.3+build.20261122"),
        );
    };
    section(ui, "Room for every fact");
    line(ui, 960.0);
    section(ui, "Narrow: the last fact alone gives way");
    line(ui, 560.0);
}

pub fn shell(state: &mut State, ui: &mut Ui) {
    let width = ui.available_width();
    section(ui, "Open");
    frame(state, ui, vec2(width, 440.0), false);
    section(ui, "The rail collapsed");
    frame(state, ui, vec2(width, 360.0), true);
}
