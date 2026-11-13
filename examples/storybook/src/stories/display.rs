use eframe::egui::{Ui, vec2};
use sokkel_ui_egui::{
    Alert, Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Card, Chip, ChipTone, Icon, Separator, Skeleton,
    TextRole, tokens,
};

use super::{State, section};

pub fn alert(_: &mut State, ui: &mut Ui) {
    ui.set_max_width(560.0);
    section(ui, "Default");
    ui.add(
        Alert::new("send-digest runs at 07:00")
            .icon(Icon::Info)
            .description("The digest goes to the harbor team once nightly-export has run."),
    );
    section(ui, "Destructive");
    ui.add(
        Alert::new("rebuild-index failed at validate")
            .icon(Icon::CircleAlert)
            .description("3 of 1,284 rows had no site. Fix them and run it again.")
            .destructive(true),
    );
    section(ui, "Title only");
    ui.add(Alert::new("Changes are saved as you type.").icon(Icon::Check));
}

pub fn badge(_: &mut State, ui: &mut Ui) {
    section(ui, "Variants");
    ui.horizontal(|ui| {
        ui.add(Badge::new("v2.4.0"));
        ui.add(Badge::new("worker").variant(BadgeVariant::Secondary));
        ui.add(Badge::new("deprecated").variant(BadgeVariant::Destructive));
        ui.add(Badge::new("scheduler").variant(BadgeVariant::Outline));
        ui.add(Badge::new("draft").variant(BadgeVariant::Ghost));
        ui.add(Badge::new("docs").variant(BadgeVariant::Link));
    });
    section(ui, "With an icon");
    ui.horizontal(|ui| {
        ui.add(Badge::new("verified").icon(Icon::BadgeCheck));
        ui.add(Badge::new("12 runs").icon(Icon::Clock).variant(BadgeVariant::Secondary));
        ui.add(
            Badge::new("3 failed")
                .icon(Icon::CircleAlert)
                .variant(BadgeVariant::Destructive),
        );
        ui.add(Badge::new("larkspur").icon(Icon::Folder).variant(BadgeVariant::Outline));
    });
}

pub fn card(_: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        ui.allocate_ui(vec2(340.0, 300.0), |ui| {
            Card::new()
                .title("nightly-export")
                .description("Exports every order of the day to the harbor bucket.")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(Chip::new("succeeded").tone(ChipTone::Good));
                        ui.label(
                            TextRole::Help
                                .rich(&t, "4m 12s, at 02:00")
                                .color(t.color("muted-foreground")),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.add(Button::new("Run now").size(ButtonSize::Sm));
                        ui.add(Button::new("Edit").variant(ButtonVariant::Outline).size(ButtonSize::Sm));
                    });
                });
        });
        ui.allocate_ui(vec2(340.0, 300.0), |ui| {
            Card::new()
                .title("Runs today")
                .bar(true)
                .footer("Updated a minute ago")
                .show(ui, |ui| {
                    for (job, tone, word) in [
                        ("nightly-export", ChipTone::Good, "succeeded"),
                        ("sync-inventory", ChipTone::Info, "running"),
                        ("rebuild-index", ChipTone::Critical, "failed"),
                    ] {
                        ui.horizontal(|ui| {
                            ui.label(TextRole::Body.rich(&t, job).color(t.color("foreground")));
                            ui.with_layout(eframe::egui::Layout::right_to_left(eframe::egui::Align::Center), |ui| {
                                ui.add(Chip::new(word).tone(tone));
                            });
                        });
                    }
                });
        });
    });
}

pub fn separator(_: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    ui.set_max_width(420.0);
    section(ui, "Across");
    ui.label(
        TextRole::SectionTitle
            .rich(&t, "sync-inventory")
            .color(t.color("foreground")),
    );
    ui.label(
        TextRole::Body
            .rich(&t, "Every hour, on the worker service.")
            .color(t.color("muted-foreground")),
    );
    ui.add(Separator::horizontal());
    section(ui, "Down");
    ui.horizontal(|ui| {
        for (i, word) in ["Runs", "Schedules", "Settings"].into_iter().enumerate() {
            if i > 0 {
                ui.add(Separator::vertical());
            }
            ui.label(TextRole::Body.rich(&t, word).color(t.color("foreground")));
        }
    });
}

pub fn skeleton(_: &mut State, ui: &mut Ui) {
    section(ui, "A row loading");
    for _ in 0..3 {
        ui.horizontal(|ui| {
            ui.add(Skeleton::new(32.0, 32.0).round());
            ui.vertical(|ui| {
                ui.add(Skeleton::new(220.0, 14.0));
                ui.add(Skeleton::new(140.0, 12.0));
            });
        });
        ui.add_space(8.0);
    }
}
