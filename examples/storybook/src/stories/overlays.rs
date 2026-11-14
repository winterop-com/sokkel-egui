use eframe::egui::Ui;
use sokkel_ui_egui::{
    Button, ButtonVariant, Dialog, Icon, Select, Switch, TextField, TextRole, Toast, ToastKind, Toaster, tokens,
    tooltip, tooltip_shown,
};

use super::{State, section};

pub fn dialog(state: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    section(ui, "Rename a job");
    if ui
        .add(Button::new("Rename").icon(Icon::Pencil).variant(ButtonVariant::Outline))
        .clicked()
    {
        state.dialog_open = true;
    }
    if state.dialog_open {
        let mut close = false;
        let job = &mut state.job;
        let notify = &mut state.notify;
        let asked = Dialog::new("rename", "Rename nightly-export")
            .description("Runs keep the name they ran under.")
            .show(
                ui.ctx(),
                |ui| {
                    ui.add(TextField::new("Name", job).width(352.0));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add(Switch::new(notify, "Notify ada on failure"));
                        ui.label(
                            TextRole::Body
                                .rich(&t, "Notify ada on failure")
                                .color(t.color("popover-foreground")),
                        );
                    });
                },
                |ui| {
                    if ui.add(Button::new("Rename")).clicked() {
                        close = true;
                    }
                    if ui.add(Button::new("Cancel").variant(ButtonVariant::Outline)).clicked() {
                        close = true;
                    }
                },
            );
        if asked || close {
            state.dialog_open = false;
        }
    }
}

pub fn select(state: &mut State, ui: &mut Ui) {
    let periods = ["Last hour", "Today", "Last 7 days", "Last 30 days"];
    section(ui, "Closed");
    ui.add(Select::new(&mut state.period, periods, "Period"));
    section(ui, "Open");
    ui.add(Select::new(&mut state.service, ["api", "worker", "scheduler"], "Service").default_open(true));
}

pub fn tooltips(_: &mut State, ui: &mut Ui) {
    section(ui, "On hover and focus");
    ui.horizontal(|ui| {
        let copy = ui.add(Button::new("Copy").icon(Icon::Copy).variant(ButtonVariant::Outline));
        tooltip(copy, "Copy the run's id");
    });
    section(ui, "Shown");
    ui.horizontal(|ui| {
        let retry = ui.add(
            Button::new("Retry")
                .icon(Icon::RefreshCw)
                .variant(ButtonVariant::Outline),
        );
        tooltip_shown(retry, "Run rebuild-index again from validate");
    });
    ui.add_space(40.0);
}

/// Toasts at the window's corner: three standing at the start, so they are seen without waiting,
/// and a button that adds one more.
pub fn toasts(_: &mut State, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let seeded = ui.id().with("toasts-seeded");
    if !ui.data(|d| d.get_temp::<bool>(seeded).unwrap_or(false)) {
        ui.data_mut(|d| d.insert_temp(seeded, true));
        Toast::new("Overview deleted").timeout(None).show(&ctx);
        Toast::error("Could not save the dashboard")
            .description("The name is another dashboard's.")
            .timeout(None)
            .show(&ctx);
        Toast::success("Operations made").timeout(None).show(&ctx);
    }
    section(ui, "Toasts");
    if ui
        .add(Button::new("Notify").icon(Icon::Bell).variant(ButtonVariant::Outline))
        .clicked()
    {
        Toast::new("nightly-export succeeded")
            .kind(ToastKind::Info)
            .description("Its run took 4 min 12 s.")
            .show(&ctx);
    }
    Toaster::new().show(&ctx);
}
