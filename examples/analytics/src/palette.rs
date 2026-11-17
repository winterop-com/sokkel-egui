//! The search palette, as the web's `Analytics.tsx` builds it: the rail's places, what was saved,
//! the things to make, the view's range and comparing, the live stream, the look, putting the
//! demo's data back, and the teams found by name.

use eframe::egui;
use sokkel_egui::{
    AppearanceCommandWords, CommandGroup, CommandPalette, Icon, MenuItem, PaletteWords, RailGroup, appearance_commands,
    place_commands, place_of, run_appearance_command,
};

use crate::app::AnalyticsApp;
use crate::model::{Pace, TeamStatus};
use crate::view::{PRESETS, Range};
use crate::world::standing;

/// How many letters the teams' search waits for, and how many teams it lists.
const MIN_QUERY: usize = 2;
const FOUND: usize = 8;

impl AnalyticsApp {
    /// The palette while it is open, and what its command does.
    pub fn search_palette(&mut self, ctx: &egui::Context, rail: &[RailGroup]) {
        let Some((mut query, mut highlight)) = self.palette.take() else {
            return;
        };
        let mut groups: Vec<CommandGroup> = place_commands(rail, "Go to").into_iter().collect();
        let mut saved: Vec<MenuItem> = Vec::new();
        saved.extend(self.invoices.views.iter().map(|one| {
            MenuItem::action(&one.name)
                .icon(Icon::Filter)
                .hint("Invoices")
                .keywords(["view", "invoices"])
                .id(format!("view/{}", one.id))
        }));
        saved.extend(self.funnels.saved.iter().map(|one| {
            MenuItem::action(&one.name)
                .icon(Icon::ListFilter)
                .hint("Funnel")
                .keywords(["funnel"])
                .id(format!("funnel/{}", one.id))
        }));
        saved.extend(self.reports.saved.iter().map(|one| {
            MenuItem::action(&one.name)
                .icon(Icon::FileText)
                .hint("Report")
                .keywords(["report"])
                .id(format!("report/{}", one.id))
        }));
        groups.push(CommandGroup::new("Saved", saved));
        groups.push(CommandGroup::new(
            "Create",
            [
                MenuItem::action("New funnel").icon(Icon::Plus).id("new-funnel"),
                MenuItem::action("New report").icon(Icon::Plus).id("new-report"),
                MenuItem::action("New goal").icon(Icon::Plus).id("new-goal"),
            ],
        ));
        let paused = self.prefs.pace == Pace::Off;
        let mut view: Vec<MenuItem> = PRESETS
            .iter()
            .map(|preset| {
                MenuItem::action(format!("Show {}", preset.label.to_lowercase()))
                    .icon(Icon::CalendarRange)
                    .keywords(["range", "period"])
                    .id(format!("range/{}", preset.id))
            })
            .collect();
        view.push(
            MenuItem::action(if self.view.compare {
                "Stop comparing with the period before"
            } else {
                "Compare with the period before"
            })
            .icon(Icon::GitCompareArrows)
            .id("compare"),
        );
        view.push(
            MenuItem::action(if paused {
                "Resume the live stream"
            } else {
                "Pause the live stream"
            })
            .icon(if paused { Icon::Play } else { Icon::Pause })
            .keywords(["live", "events"])
            .id("stream"),
        );
        groups.push(CommandGroup::new("View", view));
        let themes: Vec<_> = sokkel_egui::themes().filter(|theme| theme.name == "tidevann").collect();
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
        // the teams whose name holds what is typed, the ones paying most first
        let wanted = query.trim().to_lowercase();
        if wanted.chars().count() >= MIN_QUERY {
            let money = self.money();
            let now = self.world.now;
            let mut found: Vec<_> = self
                .world
                .teams
                .iter()
                .filter(|team| team.name.to_lowercase().contains(&wanted))
                .map(|team| (team, standing(team, now)))
                .collect();
            found.sort_by_key(|(_, now)| std::cmp::Reverse(now.mrr));
            let results = found.iter().take(FOUND).map(|(team, now)| {
                let left = if now.status == TeamStatus::Churned {
                    ", left"
                } else {
                    ""
                };
                let mut row = MenuItem::action(&team.name)
                    .icon(Icon::Users)
                    .detail(format!("{}{left}, {}", now.plan.name(), team.country))
                    .id(format!("customer/{}", team.id));
                if now.mrr > 0 {
                    row = row.hint(format!("{} a month", money.full(now.mrr as f64)));
                }
                row
            });
            groups.push(CommandGroup::new("Customers", results));
        }
        let words = PaletteWords {
            searching: "Looking".into(),
            ..Default::default()
        };
        let outcome = CommandPalette::new("palette", &groups, &mut query, &mut highlight)
            .words(words)
            .show(ctx);
        let command = outcome
            .run
            .and_then(|(g, i)| groups[g].items[i].command_id().map(str::to_owned));
        if let Some(command) = command.as_deref() {
            self.run_command(command);
        }
        if !outcome.close {
            self.palette = Some((query, highlight));
        }
    }

    fn run_command(&mut self, command: &str) {
        let (kind, id) = command.split_once('/').unwrap_or((command, ""));
        match kind {
            "view" => {
                self.go("revenue/invoices");
                self.apply_view(id);
            }
            "funnel" => self.go(&format!("funnels/{id}")),
            "report" => self.go(&format!("reports/{id}")),
            "customer" => self.go(&format!("customers/{id}")),
            "new-funnel" => self.go("funnels/new"),
            "new-report" => self.go("reports/new"),
            "new-goal" => {
                self.go("goals");
                if self.page() == "goals" {
                    self.open_new_goal();
                }
            }
            "range" => self.view.range = Range::preset(id),
            "compare" => self.view.compare = !self.view.compare,
            "stream" => self.toggle_stream(),
            "demo-reset" => self.resetting = true,
            _ => {
                if let Some(place) = place_of(command) {
                    self.go(if place == "overview" { "" } else { place });
                } else {
                    run_appearance_command(&mut self.appearance, command);
                }
            }
        }
    }
}
