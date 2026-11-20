use eframe::egui::Ui;
use sokkel_egui::{Button, ButtonVariant, Icon, PageHeader, PageState, PageStateTone};

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

pub fn page_header_editable(_: &mut State, ui: &mut Ui) {
    section(
        ui,
        "A title and a description edited in place, the count beside the title",
    );
    PageHeader::new("Runs in October")
        .count("248 rows, 7 columns")
        .description("How Tidevann's jobs ran through the month, one row a run.")
        .edit_title("Name", |value| {
            value.trim().is_empty().then(|| "A dataset needs a name.".to_owned())
        })
        .edit_description("Description", "Add a description")
        .show(ui, |ui| {
            ui.add(Button::new("New chart").icon(Icon::ChartColumn));
        });
    section(ui, "No description yet");
    PageHeader::new("Harbor arrivals")
        .count("1,080 rows")
        .edit_title("Name", |_| None)
        .edit_description("Description", "Add a description")
        .show(ui, |_| {});
}

pub fn page_state(_: &mut State, ui: &mut Ui) {
    section(ui, "Loading");
    PageState::loading().show(ui);
    section(ui, "Refused, with its problems and a retry");
    PageState::refused("The token was revoked by ada at 09:12. Sign in again, or set `api.token` in the service.")
        .problems([
            "scheduler: the token is not valid",
            "worker: no `runs:read` access to harbor",
        ])
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
