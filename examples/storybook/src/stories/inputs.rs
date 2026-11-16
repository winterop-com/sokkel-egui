use eframe::egui::Ui;
use sokkel_egui::{Checkbox, Input, Segmented, SegmentedVariant, Tabs, TabsVariant, TextRole, tokens};

use super::{State, section};

pub fn checkbox(state: &mut State, ui: &mut Ui) {
    section(ui, "Checked and not");
    ui.add(Checkbox::new(&mut state.notify, "Notify ada on failure"));
    ui.add(Checkbox::new(&mut state.retry, "Retry once"));
    section(ui, "Partly chosen");
    let mut some = true;
    ui.add(Checkbox::new(&mut some, "2 of 5 steps").indeterminate(true));
    section(ui, "Disabled");
    let (mut on, mut off) = (true, false);
    ui.add(Checkbox::new(&mut on, "Keep logs for 30 days").enabled(false));
    ui.add(Checkbox::new(&mut off, "Delete runs after a day").enabled(false));
    section(ui, "As a field");
    // a to-do list's column: each row as wide as its room, the label wrapping inside it
    ui.scope(|ui| {
        ui.set_max_width(320.0);
        ui.spacing_mut().item_spacing.y = 8.0;
        let (mut first, mut second) = (false, true);
        ui.add(Checkbox::new(&mut first, "Review the larkspur sync schedule with grace").field());
        ui.add(Checkbox::new(&mut second, "Rotate the api keys for harbor").field());
    });
}

pub fn input(state: &mut State, ui: &mut Ui) {
    // a form's column, as the web's stories hold their fields (`max-w-md`)
    ui.set_max_width(448.0);
    section(ui, "Plain");
    ui.add(Input::new(&mut state.job));
    section(ui, "Placeholder");
    ui.add(Input::new(&mut state.empty).placeholder("Search jobs"));
    section(ui, "Invalid");
    ui.add(Input::new(&mut state.schedule).invalid(true));
    section(ui, "Disabled");
    let mut fixed = "worker".to_owned();
    ui.add(Input::new(&mut fixed).enabled(false));
}

pub fn tabs(state: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let tabs = ["Runs", "Schedules", "Settings"];
    section(ui, "Default");
    ui.add(Tabs::new(&mut state.tab, tabs));
    ui.label(
        TextRole::Body
            .rich(&t, format!("{} of nightly-export.", tabs[state.tab]))
            .color(t.color("muted-foreground")),
    );
    section(ui, "Line");
    ui.add(Tabs::new(&mut state.tab, tabs).variant(TabsVariant::Line));
}

pub fn segmented(state: &mut State, ui: &mut Ui) {
    let options = ["Day", "Week", "Month", "Year"];
    for (title, variant) in [
        ("Track", SegmentedVariant::Track),
        ("Tint", SegmentedVariant::Tint),
        ("Outline", SegmentedVariant::Outline),
        ("Solid", SegmentedVariant::Solid),
    ] {
        section(ui, title);
        ui.add(Segmented::new(&mut state.period, options).variant(variant));
    }
}
