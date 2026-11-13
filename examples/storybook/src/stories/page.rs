use eframe::egui::Ui;
use sokkel_ui_egui::{Button, ButtonVariant, Icon, PageHeader, PageState, PageStateTone};

use super::{State, section};

pub fn page_header(_: &mut State, ui: &mut Ui) {
    section(ui, "A listing");
    PageHeader::new("Jobs")
        .count(5)
        .description("What runs, when it ran, and how it went.")
        .show(ui, |ui| {
            ui.add(Button::new("New").icon(Icon::Plus));
            ui.add(Button::new("Import").variant(ButtonVariant::Outline));
        });
    section(ui, "One thing, with its code");
    PageHeader::new("nightly-export").code("job_7f3a91").show(ui, |ui| {
        ui.add(Button::new("Run now").icon(Icon::Play));
    });
}

pub fn page_state(_: &mut State, ui: &mut Ui) {
    section(ui, "Loading");
    PageState::loading().show(ui);
    section(ui, "Refused, with its problems and a retry");
    PageState::refused("The token was revoked by ada at 09:12.")
        .problems(["scheduler: the token is not valid", "worker: no read access to harbor"])
        .retry(true)
        .show(ui);
    section(ui, "Not answering");
    PageState::unreachable("api.example.org did not answer within 10 seconds.")
        .retry(true)
        .show(ui);
    section(ui, "A part refused");
    PageState::refused("The run history is hidden from your role.")
        .tone(PageStateTone::Warning)
        .show(ui);
    section(ui, "Empty");
    PageState::empty("No runs yet. The first run of nightly-export starts at 02:00.").show(ui);
}
