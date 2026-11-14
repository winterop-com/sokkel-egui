use eframe::egui::Ui;
use sokkel_ui_egui::{Button, ButtonSize, ButtonVariant, Chip, ChipTone, Icon, Switch, TextField, TextRole, tokens};

use super::{State, section};

pub fn button(_: &mut State, ui: &mut Ui) {
    section(ui, "Variants");
    ui.horizontal(|ui| {
        ui.add(Button::new("Run now"));
        ui.add(Button::new("Edit").variant(ButtonVariant::Outline));
        ui.add(Button::new("Duplicate").variant(ButtonVariant::Secondary));
        ui.add(Button::new("Skip").variant(ButtonVariant::Ghost));
        ui.add(Button::new("Delete").variant(ButtonVariant::Destructive));
        ui.add(Button::new("View logs").variant(ButtonVariant::Link));
    });
    section(ui, "With an icon");
    ui.horizontal(|ui| {
        ui.add(Button::new("Run now").icon(Icon::Play));
        ui.add(
            Button::new("Download")
                .icon(Icon::Download)
                .variant(ButtonVariant::Outline),
        );
        ui.add(Button::new("Copy").icon(Icon::Copy).variant(ButtonVariant::Secondary));
        ui.add(
            Button::new("Refresh")
                .icon(Icon::RefreshCw)
                .variant(ButtonVariant::Ghost),
        );
        ui.add(
            Button::new("Delete")
                .icon(Icon::Trash2)
                .variant(ButtonVariant::Destructive),
        );
        ui.add(
            Button::new("Upload")
                .icon(Icon::Upload)
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Sm),
        );
    });
    section(ui, "Sizes");
    for variant in [ButtonVariant::Default, ButtonVariant::Outline] {
        ui.horizontal(|ui| {
            for (size, label) in [
                (ButtonSize::Xs, "Extra small"),
                (ButtonSize::Sm, "Small"),
                (ButtonSize::Default, "Default"),
                (ButtonSize::Lg, "Large"),
            ] {
                ui.add(Button::new(label).variant(variant).size(size));
            }
        });
    }
    section(ui, "States");
    ui.horizontal(|ui| {
        ui.add(Button::new("Exporting").busy(true));
        ui.add(Button::new("Run now").reason("sync-inventory is already running"));
        ui.add(Button::new("Edit").variant(ButtonVariant::Outline).enabled(false));
    });
}

pub fn field(state: &mut State, ui: &mut Ui) {
    // a form's column, as the web's stories hold their fields (`max-w-md`)
    ui.set_max_width(448.0);
    section(ui, "Help");
    ui.add(TextField::new("Name", &mut state.job).help("Lowercase words joined by dashes."));
    section(ui, "Placeholder");
    ui.add(TextField::new("Owner", &mut state.empty).placeholder("ada"));
    section(ui, "Error");
    let invalid = state.schedule.contains("25:");
    ui.add(TextField::new("Schedule", &mut state.schedule).error(invalid.then_some("There is no hour 25.")));
}

pub fn switch(state: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    section(ui, "On and off");
    for (on, words, enabled) in [
        (&mut state.notify, "Notify ada on failure", true),
        (&mut state.retry, "Retry once", true),
        (&mut state.paused, "Pause rebuild-index (only an admin may)", false),
    ] {
        ui.horizontal(|ui| {
            ui.add(Switch::new(on, words).enabled(enabled));
            ui.label(TextRole::Body.rich(&t, words).color(t.color("foreground")));
        });
    }
}

pub fn chip(_: &mut State, ui: &mut Ui) {
    section(ui, "Tones");
    ui.horizontal(|ui| {
        ui.add(Chip::new("succeeded").tone(ChipTone::Good));
        ui.add(Chip::new("failed").tone(ChipTone::Critical));
        ui.add(Chip::new("running").tone(ChipTone::Info));
        ui.add(Chip::new("cancelled").tone(ChipTone::Warning));
        ui.add(Chip::new("queued"));
        ui.add(Chip::new("harbor").tone(ChipTone::Outline));
    });
    section(ui, "Chosen");
    ui.horizontal(|ui| {
        ui.add(Chip::new("succeeded").tone(ChipTone::Good).selected(true));
        ui.add(Chip::new("failed").tone(ChipTone::Critical).selected(true));
        ui.add(Chip::new("queued").selected(true));
        ui.add(Chip::new("larkspur").tone(ChipTone::Outline).selected(true));
    });
}
