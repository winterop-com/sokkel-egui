use eframe::egui::Ui;
use sokkel_egui::{
    Button, ButtonVariant, CommandGroup, CommandPalette, DropdownMenu, Icon, MenuItem, TextRole, tokens,
};

use super::{State, section};

pub fn dropdown(state: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    section(ui, "The account menu");
    let items = [
        MenuItem::Label("ada@example.org".into()),
        MenuItem::action("Profile").icon(Icon::User),
        MenuItem::action("Settings").icon(Icon::Settings).shortcut("⌘,"),
        MenuItem::action("Keyboard shortcuts")
            .icon(Icon::Keyboard)
            .shortcut("?"),
        MenuItem::Separator,
        MenuItem::action("Sign out").icon(Icon::LogOut).destructive(),
    ];
    let trigger = ui.add(
        Button::new("ada")
            .icon(Icon::CircleUser)
            .variant(ButtonVariant::Outline),
    );
    if let Some(index) = DropdownMenu::new(&items).default_open(true).show(&trigger)
        && let MenuItem::Action { label, .. } = &items[index]
    {
        state.last_command = label.clone();
    }
    ui.add_space(220.0);
    if !state.last_command.is_empty() {
        ui.label(
            TextRole::Help
                .rich(&t, format!("Pressed: {}", state.last_command))
                .color(t.color("muted-foreground")),
        );
    }
}

/// Tidevann's commands, as its palette groups them.
pub fn commands() -> Vec<CommandGroup> {
    vec![
        CommandGroup::new(
            "Go to",
            [
                MenuItem::action("Overview").icon(Icon::LayoutDashboard),
                MenuItem::action("Runs").icon(Icon::Play).shortcut("G R"),
                MenuItem::action("Schedules").icon(Icon::CalendarClock),
                MenuItem::action("Jobs").icon(Icon::Box).shortcut("G J"),
            ],
        ),
        CommandGroup::new(
            "Runs",
            [
                MenuItem::action("Run nightly-export now").icon(Icon::Play),
                MenuItem::action("Pause every schedule").icon(Icon::Pause),
                MenuItem::action("Cancel queued runs").icon(Icon::CircleX).destructive(),
            ],
        ),
    ]
}

pub fn palette(state: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    section(ui, "Mod K opens it over the page");
    if ui
        .add(Button::new("Search").icon(Icon::Search).variant(ButtonVariant::Outline))
        .clicked()
    {
        state.palette_open = true;
    }
    if !state.last_command.is_empty() {
        ui.label(
            TextRole::Help
                .rich(&t, format!("Ran: {}", state.last_command))
                .color(t.color("muted-foreground")),
        );
    }
    if state.palette_open {
        let groups = commands();
        let outcome = CommandPalette::new(
            "story-palette",
            &groups,
            &mut state.palette_query,
            &mut state.palette_highlight,
        )
        .show(ui.ctx());
        if let Some((g, i)) = outcome.run
            && let MenuItem::Action { label, .. } = &groups[g].items[i]
        {
            state.last_command = label.clone();
        }
        if outcome.close {
            state.palette_open = false;
        }
    }
}
