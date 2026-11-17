//! The search palette, as the web's `Metrics.tsx` builds it: the rail's places, every dashboard's
//! panels, the app's actions, the look, and putting the demo's data back.

use eframe::egui;
use sokkel_egui::{
    AppearanceCommandWords, CommandGroup, CommandPalette, Icon, MenuItem, RailGroup, appearance_commands,
    place_commands, place_of, run_appearance_command,
};

use crate::MetricsApp;
use crate::app::SETTINGS;
use crate::board::DetailsForm;
use crate::model::{PanelKind, kind_icon};

impl MetricsApp {
    /// The palette while it is open, and what its command does.
    pub(crate) fn search_palette(&mut self, ctx: &egui::Context, rail: &[RailGroup]) {
        let Some((mut query, mut highlight)) = self.palette.take() else {
            return;
        };
        let mut groups: Vec<CommandGroup> = place_commands(rail, "Go to").into_iter().collect();
        let panels: Vec<MenuItem> = self
            .dashboards
            .iter()
            .flat_map(|board| {
                board.panels.iter().map(|panel| {
                    MenuItem::action(&panel.title)
                        .icon(kind_icon(panel.kind))
                        .hint(&board.title)
                        .id(format!("panel/{}/{}", board.id, panel.id))
                })
            })
            .collect();
        groups.push(CommandGroup::new("Panels", panels));
        let on_board = self.dashboards.iter().find(|b| b.id == self.place).cloned();
        let mut actions = vec![MenuItem::action("New dashboard").icon(Icon::Plus).id("dashboard-new")];
        if let Some(board) = &on_board {
            actions.push(
                MenuItem::action("Add panel")
                    .icon(Icon::Plus)
                    .hint(&board.title)
                    .id("panel-add"),
            );
        }
        actions.push(MenuItem::action("Refresh now").icon(Icon::RefreshCw).id("refresh"));
        if self.refresh > 0 {
            actions.push(if self.paused {
                MenuItem::action("Resume auto refresh").icon(Icon::Play).id("pause")
            } else {
                MenuItem::action("Pause auto refresh").icon(Icon::Pause).id("pause")
            });
        }
        actions.push(MenuItem::action("New alert rule").icon(Icon::BellPlus).id("rule-new"));
        groups.push(CommandGroup::new("Actions", actions));
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
        let outcome = CommandPalette::new("palette", &groups, &mut query, &mut highlight).show(ctx);
        let command = outcome
            .run
            .and_then(|(g, i)| groups[g].items[i].command_id().map(str::to_owned));
        match command.as_deref() {
            Some("dashboard-new") => self.details = Some(DetailsForm::default()),
            Some("panel-add") => self.open_editor(None),
            Some("refresh") => self.view_at = self.at,
            Some("pause") => self.paused = !self.paused,
            Some("rule-new") => {
                self.alerts.rule = Some(crate::alerts::RuleForm {
                    editing: None,
                    draft: crate::alerts::fresh_rule(),
                });
            }
            Some("demo-reset") => self.resetting = true,
            Some(command) if command.starts_with("panel/") => {
                let mut parts = command.split('/').skip(1);
                if let (Some(board), Some(panel)) = (parts.next(), parts.next()) {
                    self.go(board);
                    let kind = self
                        .dashboards
                        .iter()
                        .find(|b| b.id == board)
                        .and_then(|b| b.panels.iter().find(|p| p.id == panel))
                        .map(|p| p.kind);
                    // a stat tile is brought into view; any other opens in its window
                    if kind == Some(PanelKind::Stat) {
                        self.panels.focus = Some(panel.to_owned());
                    } else {
                        self.panels.viewing = Some(panel.to_owned());
                    }
                }
            }
            Some(command) => {
                if let Some(place) = place_of(command) {
                    self.go(place);
                } else {
                    run_appearance_command(&mut self.appearance, command);
                }
            }
            None => {}
        }
        if !outcome.close {
            self.palette = Some((query, highlight));
        }
    }

    /// Go to a place: a page or a dashboard, or the settings over the page shown.
    pub fn go(&mut self, place: &str) {
        if place == SETTINGS {
            self.settings = Some("dashboards".into());
        } else if place != self.place {
            self.place = place.to_owned();
            self.editor = None;
        }
    }
}
