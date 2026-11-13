use eframe::egui::{Sense, Ui, vec2};
use sokkel_ui_egui::{
    Button, ButtonSize, ButtonVariant, Chip, ChipTone, Icon, Rail, RailEntry, RailGroup, Shell, ShellPart, StatusLine,
    StatusTone, TextRole, TopBar, tokens,
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
                sokkel_ui_egui::AccountMenu::new("ada")
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

pub fn shell(state: &mut State, ui: &mut Ui) {
    let width = ui.available_width();
    section(ui, "Open");
    frame(state, ui, vec2(width, 440.0), false);
    section(ui, "The rail collapsed");
    frame(state, ui, vec2(width, 360.0), true);
}
