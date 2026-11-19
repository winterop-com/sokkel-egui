//! The palette, as the web's `Home.tsx` builds it: the rail's places, the projects' own commands,
//! the look and the demo's data.

use eframe::egui;
use sokkel_egui::{
    AppearanceCommandWords, CommandGroup, CommandPalette, Icon, MenuItem, RailGroup, appearance_commands,
    place_commands, place_of, run_appearance_command,
};

use crate::app::CutApp;
use crate::dialogs::Asking;
use crate::settings::CARRIED;

impl CutApp {
    /// The palette while it is open, and what its command does. `project` is the one open in the
    /// editor, which it can rename and duplicate.
    pub fn search_palette(&mut self, ctx: &egui::Context, rail: &[RailGroup], project: Option<String>) {
        let Some((mut query, mut highlight)) = self.palette.take() else {
            return;
        };
        let mut groups: Vec<CommandGroup> = place_commands(rail, "Go to").into_iter().collect();
        let mut own = vec![MenuItem::action("New project").icon(Icon::Plus).id("new")];
        if project.is_some() {
            own.push(
                MenuItem::action("Rename this project")
                    .icon(Icon::PencilLine)
                    .id("rename"),
            );
            own.push(
                MenuItem::action("Duplicate this project")
                    .icon(Icon::Copy)
                    .id("duplicate"),
            );
        }
        groups.push(CommandGroup::new("Projects", own));
        let themes: Vec<_> = sokkel_egui::themes()
            .filter(|theme| CARRIED.contains(&theme.name.as_str()))
            .collect();
        groups.extend(appearance_commands(
            &self.appearance,
            &themes,
            &AppearanceCommandWords::default(),
        ));
        groups.push(CommandGroup::new(
            "Demo",
            [MenuItem::action("Reset demo data")
                .icon(Icon::RotateCcw)
                .id("demo-reset")],
        ));
        let outcome = CommandPalette::new("palette", &groups, &mut query, &mut highlight).show(ctx);
        let command = outcome
            .run
            .and_then(|(g, i)| groups[g].items[i].command_id().map(str::to_owned));
        if let Some(command) = command.as_deref() {
            match (command, &project) {
                ("new", _) => self.ask(Asking::New),
                ("rename", Some(id)) => self.ask(Asking::Rename(id.clone())),
                ("duplicate", Some(id)) => {
                    let id = id.clone();
                    self.duplicate(ctx, &id);
                }
                ("demo-reset", _) => self.resetting = true,
                _ => match place_of(command) {
                    Some("projects") => self.go("projects"),
                    Some("settings") => self.go("settings"),
                    Some(id) => self.go(&format!("p/{id}")),
                    None => {
                        run_appearance_command(&mut self.appearance, command);
                    }
                },
            }
        }
        if !outcome.close {
            self.palette = Some((query, highlight));
        }
    }
}
