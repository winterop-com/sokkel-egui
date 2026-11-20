use eframe::egui::{self, Ui};
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, Chip, ChipHue, ChipMenuAlign, ChipSize, ChipTone, ChipWords, Icon, MenuItem,
    Switch, SwitchField, TextField, TextRole, tokens,
};

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

pub fn field_issues(_: &mut State, ui: &mut Ui) {
    ui.set_max_width(448.0);
    section(ui, "A warning on a yes or no");
    let mut treated = Some(false);
    sokkel_egui::YesNoField::new("Water treated", &mut treated)
        .issues([(
            sokkel_egui::IssueTone::Warning,
            "Untreated river water: say why in Reason not treated.".to_owned(),
        )])
        .show(ui);
    section(ui, "An error and a warning");
    let mut reason = String::new();
    sokkel_egui::Field::new("Reason not treated")
        .issues([
            (
                sokkel_egui::IssueTone::Critical,
                "Say why the water was not treated.".to_owned(),
            ),
            (
                sokkel_egui::IssueTone::Warning,
                "Name the site that was asked.".to_owned(),
            ),
        ])
        .show(ui, |ui| {
            ui.add(sokkel_egui::Input::new(&mut reason).name("Reason not treated"));
        });
    section(ui, "A warning on a number");
    let mut failed = Some(4.0);
    ui.add(
        sokkel_egui::NumberField::optional("story-samples-failed", "Samples failed", &mut failed)
            .integer(true)
            .issues([(
                sokkel_egui::IssueTone::Warning,
                "More samples failed than the target allows.".to_owned(),
            )]),
    );
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
    section(ui, "As a field");
    // a form's column: each row across it, the switch at the right edge
    ui.scope(|ui| {
        ui.set_max_width(448.0);
        let (mut shown, mut give, mut none) = (true, true, false);
        ui.add(SwitchField::new(&mut shown, "Show on the status page"));
        ui.add(SwitchField::new(&mut give, "Give linus the open incident").explain("INC-41"));
        ui.add(
            SwitchField::new(&mut none, "Give linus the open incidents")
                .explain("grace has no open incident.")
                .enabled(false),
        );
    });
}

pub fn chip(_: &mut State, ui: &mut Ui) {
    let row = |ui: &mut Ui, add: &mut dyn FnMut(&mut Ui)| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            add(ui);
        });
    };
    section(ui, "Tones");
    row(ui, &mut |ui| {
        ui.add(Chip::new("nightly"));
        ui.add(Chip::new("healthy").tone(ChipTone::Good));
        ui.add(Chip::new("over budget").tone(ChipTone::Critical));
        ui.add(Chip::new("slow").tone(ChipTone::Warning));
        ui.add(Chip::new("new").tone(ChipTone::Info));
        ui.add(Chip::new("draft").tone(ChipTone::Outline));
        ui.add(Chip::new("no owner").tone(ChipTone::Absent));
    });
    section(ui, "Hues");
    row(ui, &mut |ui| {
        for (hue, word) in [
            (ChipHue::Chart1, "source"),
            (ChipHue::Chart2, "transform"),
            (ChipHue::Chart3, "check"),
            (ChipHue::Chart4, "sink"),
            (ChipHue::Chart5, "notice"),
        ] {
            ui.add(Chip::new(word).hue(hue));
        }
    });
    section(ui, "Kind hues");
    row(ui, &mut |ui| {
        for hue in &ChipHue::ALL[5..] {
            ui.add(Chip::new(hue.key()).hue(*hue));
        }
    });
    section(ui, "Mono and icons");
    row(ui, &mut |ui| {
        ui.add(Chip::new("nightly-export").mono(true));
        ui.add(Chip::new("main").mono(true).icon(Icon::GitBranch));
        ui.add(Chip::new("worker").icon(Icon::Server));
        ui.add(Chip::new("late 12 min").tone(ChipTone::Warning).icon(Icon::Clock));
        ui.add(Chip::new("no tags").tone(ChipTone::Absent).icon(Icon::Tag));
    });
    ui.add_space(4.0);
    ui.allocate_ui(egui::vec2(240.0, 20.0), |ui| {
        let long = "import-orders-from-the-harbor-warehouse-every-night";
        ui.add(Chip::new(long).mono(true).title(long));
    });
    section(ui, "Control size");
    row(ui, &mut |ui| {
        ui.add(
            Chip::new("api.example.org")
                .size(ChipSize::Control)
                .mono(true)
                .icon(Icon::Server)
                .link("#/services/api", false),
        );
        ui.add(
            Chip::new("connected")
                .size(ChipSize::Control)
                .tone(ChipTone::Good)
                .icon(Icon::Radio),
        );
    });
    ui.add_space(4.0);
    row(ui, &mut |ui| {
        for name in ["nightly", "hourly", "on demand"] {
            ui.add(
                Chip::new(name)
                    .size(ChipSize::Control)
                    .button()
                    .selected(name == "nightly"),
            );
        }
    });
}

pub fn chip_actions(state: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let row = |ui: &mut Ui, add: &mut dyn FnMut(&mut Ui)| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            add(ui);
        });
    };
    section(ui, "Buttons with remove buttons");
    let mut removed = None;
    let mut filter = state.chip_filter.clone();
    row(ui, &mut |ui| {
        for tag in &state.chip_tags {
            let on = filter.as_deref() == Some(tag.as_str());
            let chip = Chip::new(tag)
                .mono(true)
                .tone(if on { ChipTone::Info } else { ChipTone::Neutral })
                .button()
                .select_label(if on {
                    "Clear the filter".to_owned()
                } else {
                    format!("Filter by {tag}")
                })
                .removable(true)
                .words(ChipWords {
                    remove: format!("Remove {tag}"),
                    ..Default::default()
                })
                .show(ui);
            if chip.pressed {
                filter = if on { None } else { Some(tag.clone()) };
            }
            if chip.removed {
                removed = Some(tag.clone());
            }
        }
        if state.chip_tags.is_empty() {
            ui.add(Chip::new("no tags").tone(ChipTone::Absent));
        }
    });
    state.chip_filter = filter;
    if let Some(tag) = removed {
        state.chip_tags.retain(|other| *other != tag);
    }
    ui.add_space(4.0);
    let said = match &state.chip_filter {
        Some(tag) => format!("Filtered by {tag}"),
        None => "No filter".to_owned(),
    };
    ui.label(TextRole::Body.rich(&t, said).color(t.color("muted-foreground")));
    section(ui, "Links");
    row(ui, &mut |ui| {
        ui.add(
            Chip::new("harbor")
                .mono(true)
                .link("#/projects/harbor", false)
                .select_label("Open harbor"),
        );
        ui.add(
            Chip::new("juniper")
                .mono(true)
                .hue(ChipHue::Teal)
                .link("#/projects/juniper", false),
        );
        ui.add(
            Chip::new("api guide")
                .icon(Icon::BookOpen)
                .link("https://docs.example.org/api", true),
        );
    });
    section(ui, "Toggles");
    let mut flipped = None;
    row(ui, &mut |ui| {
        for tag in ["harbor", "juniper", "larkspur", "nightly", "hourly", "critical-path"] {
            let on = state.chip_on.iter().any(|other| other == tag);
            if Chip::new(tag).mono(true).button().selected(on).show(ui).pressed {
                flipped = Some(tag);
            }
        }
    });
    if let Some(tag) = flipped {
        if state.chip_on.iter().any(|other| other == tag) {
            state.chip_on.retain(|other| other != tag);
        } else {
            state.chip_on.push(tag.to_owned());
        }
    }
    ui.add_space(4.0);
    row(ui, &mut |ui| {
        for (tone, word) in [
            (ChipTone::Neutral, "neutral"),
            (ChipTone::Good, "good"),
            (ChipTone::Critical, "critical"),
            (ChipTone::Warning, "warning"),
            (ChipTone::Info, "info"),
            (ChipTone::Outline, "outline"),
        ] {
            let on = matches!(tone, ChipTone::Critical | ChipTone::Info | ChipTone::Outline);
            ui.add(Chip::new(word).tone(tone).button().selected(on));
        }
    });
    ui.add_space(4.0);
    row(ui, &mut |ui| {
        for (index, hue) in ChipHue::ALL[5..].iter().enumerate() {
            ui.add(Chip::new(hue.key()).hue(*hue).button().selected(index % 2 == 0));
        }
    });
    ui.add_space(4.0);
    let showing = if state.chip_on.is_empty() {
        "Showing every project".to_owned()
    } else {
        format!("Showing {}", state.chip_on.join(", "))
    };
    ui.label(TextRole::Body.rich(&t, showing).color(t.color("muted-foreground")));
    section(ui, "Menu");
    row(ui, &mut |ui| {
        ui.add(
            Chip::new("harbor")
                .mono(true)
                .select_label("harbor, more")
                .menu(
                    [
                        MenuItem::action("Filter by harbor").icon(Icon::Filter),
                        MenuItem::action("Copy the name").icon(Icon::Copy),
                        MenuItem::action("Open the project").icon(Icon::SquareArrowOutUpRight),
                    ],
                    ChipMenuAlign::Start,
                )
                .default_open(true),
        );
        ui.add(
            Chip::new("juniper")
                .mono(true)
                .menu([MenuItem::action("Filter by juniper")], ChipMenuAlign::End),
        );
    });
}
