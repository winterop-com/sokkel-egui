//! The search palette, as the web's `Console.tsx` builds it: the rail's pages, the jobs, a job to
//! run now, the look, putting the demo's data back, and signing out.

use eframe::egui;
use sokkel_egui::{
    AppearanceCommandWords, CommandGroup, CommandPalette, Icon, MenuItem, RailGroup, appearance_commands,
    place_commands, place_of, run_appearance_command,
};

use crate::app::RunsApp;
use crate::themes::CARRIED;

impl RunsApp {
    /// The palette while it is open, and what its command does.
    pub fn search_palette(&mut self, ctx: &egui::Context, rail: &[RailGroup]) {
        let Some((mut query, mut highlight)) = self.palette.take() else {
            return;
        };
        let mut groups: Vec<CommandGroup> = place_commands(rail, "Go to").into_iter().collect();
        groups.push(CommandGroup::new(
            "Jobs",
            self.backend.jobs.iter().map(|job| {
                MenuItem::action(&job.name)
                    .icon(Icon::ListChecks)
                    .hint(job.project.word())
                    .keywords([job.id.as_str()])
                    .id(format!("job/{}", job.id))
            }),
        ));
        groups.push(CommandGroup::new(
            "Run a job",
            self.backend.jobs.iter().map(|job| {
                MenuItem::action(format!("Run {} now", job.id))
                    .icon(Icon::Play)
                    .keywords(["start", "run now", job.name.as_str()])
                    .id(format!("run/{}", job.id))
            }),
        ));
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
        groups.push(CommandGroup::new(
            "Account",
            [MenuItem::action("Sign out").icon(Icon::LogOut).id("sign-out")],
        ));
        let outcome = CommandPalette::new("palette", &groups, &mut query, &mut highlight).show(ctx);
        let command = outcome
            .run
            .and_then(|(g, i)| groups[g].items[i].command_id().map(str::to_owned));
        if let Some(command) = command.as_deref() {
            let (kind, id) = command.split_once('/').unwrap_or((command, ""));
            match kind {
                "job" => self.go(&format!("jobs/{id}")),
                "run" => self.run_now(ctx, id),
                "demo-reset" => self.resetting = true,
                "sign-out" => self.sign_out(),
                _ => {
                    if let Some(place) = place_of(command) {
                        self.go(if place == "overview" { "" } else { place });
                    } else {
                        run_appearance_command(&mut self.appearance, command);
                    }
                }
            }
        }
        if !outcome.close {
            self.palette = Some((query, highlight));
        }
    }
}
