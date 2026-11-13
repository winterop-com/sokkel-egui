use eframe::egui::{self, Frame, Margin, Sense, Stroke, Ui, vec2};
use sokkel_ui_egui::{
    Button, ButtonSize, ButtonVariant, Chip, ChipTone, Switch, TextField, TextRole, scope, themes, tokens,
};

use super::{State, section};

pub fn sample(state: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    ui.label(
        TextRole::Eyebrow
            .rich(&t, "TIDEVANN")
            .color(t.color("muted-foreground")),
    );
    ui.label(TextRole::PageTitle.rich(&t, "Jobs").color(t.color("foreground")));
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add(Button::new("Run now"));
        ui.add(Button::new("Edit").variant(ButtonVariant::Outline));
        ui.add(Button::new("Delete").variant(ButtonVariant::Destructive));
    });
    section(ui, "Last runs");
    for (job, state_word, tone, when) in [
        ("nightly-export", "succeeded", ChipTone::Good, "4m 12s, 02:00"),
        ("sync-inventory", "running", ChipTone::Info, "started 3m ago"),
        ("rebuild-index", "failed", ChipTone::Critical, "at validate, 01:14"),
        ("send-digest", "queued", ChipTone::Neutral, "next at 07:00"),
    ] {
        ui.horizontal(|ui| {
            ui.add_sized(
                [180.0, 20.0],
                eframe::egui::Label::new(TextRole::Body.rich(&t, job).color(t.color("foreground"))),
            );
            ui.add(Chip::new(state_word).tone(tone));
            ui.label(TextRole::Help.rich(&t, when).color(t.color("muted-foreground")));
        });
    }
    section(ui, "Settings");
    ui.horizontal_top(|ui| {
        ui.add(TextField::new("Name", &mut state.job).help("Lowercase words joined by dashes."));
        ui.add_space(16.0);
        let invalid = state.schedule.contains("25:");
        ui.add(TextField::new("Schedule", &mut state.schedule).error(invalid.then_some("There is no hour 25.")));
    });
    ui.horizontal(|ui| {
        ui.add(Switch::new(&mut state.notify, "Notify ada on failure"));
        ui.label(
            TextRole::Body
                .rich(&t, "Notify ada on failure")
                .color(t.color("foreground")),
        );
    });
}

/// Every theme as a tile, each drawn in its own scope in the scheme the toolbar picks, then the
/// other scheme beneath.
pub fn all_themes(_: &mut State, ui: &mut Ui) {
    let dark = tokens(ui.ctx()).dark;
    for (title, scheme) in [("In this scheme", dark), ("In the other scheme", !dark)] {
        section(ui, title);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(16.0, 16.0);
            for theme in themes() {
                // a tile's size is known before it is drawn, so the row can wrap before it
                ui.allocate_ui(vec2(306.0, 196.0), |ui| {
                    scope(ui, &theme.name, Some(scheme), |ui| {
                        tile(ui, &theme.label, theme.description.as_deref())
                    })
                });
            }
        });
    }
}

/// One theme's tile: its ground, its name and line, the colors it is known by, and a few parts.
fn tile(ui: &mut Ui, label: &str, description: Option<&str>) {
    let t = tokens(ui.ctx());
    Frame::new()
        .fill(t.color("background"))
        .stroke(Stroke::new(1.0, t.color("border-strong")))
        .corner_radius(t.radius("surface"))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(276.0);
            ui.vertical(|ui| tile_body(ui, label, description));
        });
}

fn tile_body(ui: &mut Ui, label: &str, description: Option<&str>) {
    let t = tokens(ui.ctx());
    {
        {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            ui.label(TextRole::DetailTitle.rich(&t, label).color(t.color("foreground")));
            ui.add(
                egui::Label::new(
                    TextRole::Help
                        .rich(&t, description.unwrap_or_default())
                        .color(t.color("muted-foreground")),
                )
                .truncate(),
            );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for key in [
                    "primary",
                    "secondary",
                    "accent",
                    "muted",
                    "good",
                    "warning",
                    "critical",
                    "chart-1",
                    "chart-2",
                    "chart-3",
                ] {
                    let (rect, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
                    ui.painter().rect(
                        rect,
                        t.radius("chip"),
                        t.color(key),
                        Stroke::new(1.0, t.color("border")),
                        egui::StrokeKind::Inside,
                    );
                }
            });
            ui.horizontal(|ui| {
                ui.add(Button::new("Run now").size(ButtonSize::Sm));
                ui.add(Button::new("Edit").variant(ButtonVariant::Outline).size(ButtonSize::Sm));
                let mut on = true;
                ui.add(Switch::new(&mut on, "Notify"));
            });
            ui.horizontal(|ui| {
                ui.add(Chip::new("succeeded").tone(ChipTone::Good));
                ui.add(Chip::new("running").tone(ChipTone::Info));
                ui.add(Chip::new("failed").tone(ChipTone::Critical));
            });
        }
    }
}
