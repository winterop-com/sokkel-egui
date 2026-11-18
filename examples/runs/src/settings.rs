//! The person's settings as a dialog over the page, as the web's `pages/Settings.tsx`: how runs
//! open and their times read, what to hear of, the logs, the look, the scheduler, the account and
//! the demo's data.

use eframe::egui;
use sokkel_egui::{
    Appearance, Button, ButtonSize, ButtonVariant, ConfirmAnswer, ConfirmDialog, Icon, Segmented, SettingsDialog,
    SettingsSection, Switch, ThemeWords, setting_row, theme_settings_of,
};

use crate::app::RunsApp;
use crate::model::{OpenRuns, Prefs, Times};
use crate::themes::CARRIED;

impl RunsApp {
    /// The settings over the page while they are open, and the question before the demo's data
    /// goes back.
    pub fn settings_dialog(&mut self, ctx: &egui::Context) {
        if let Some(mut active) = self.settings.take() {
            let sections = [
                SettingsSection::new("general", "General", Icon::Settings2).group("Preferences"),
                SettingsSection::new("notifications", "Notifications", Icon::Bell).group("Preferences"),
                SettingsSection::new("logs", "Logs", Icon::ScrollText).group("Preferences"),
                SettingsSection::new("appearance", "Appearance", Icon::Palette).group("Preferences"),
                SettingsSection::new("scheduler", "Scheduler", Icon::CalendarClock).group("Workspace"),
                SettingsSection::new("account", "Account", Icon::UserRound)
                    .group("Workspace")
                    .reset(false),
                SettingsSection::new("demo", "Demo data", Icon::Database)
                    .group("Workspace")
                    .reset(false),
            ];
            let me = self.me();
            let mut prefs = self.prefs();
            let appearance = &mut self.appearance;
            let (mut sign_out, mut reset) = (false, false);
            let outcome =
                SettingsDialog::new("settings", &sections, &mut active).show(ctx, |section, ui| match section {
                    "general" => {
                        setting_row(
                            ui,
                            "Open a run",
                            "Pressing a run in a list shows it beside the list, or on a page of its own.",
                            |ui| {
                                let mut at = usize::from(prefs.open_runs == OpenRuns::Page);
                                if ui.add(Segmented::new(&mut at, ["Beside", "On a page"])).changed() {
                                    prefs.open_runs = if at == 0 { OpenRuns::Panel } else { OpenRuns::Page };
                                }
                            },
                        );
                        setting_row(
                            ui,
                            "Times",
                            "When runs started: how long ago, or the time on the UTC clock.",
                            |ui| {
                                let mut at = usize::from(prefs.times == Times::Clock);
                                if ui.add(Segmented::new(&mut at, ["How long ago", "Clock"])).changed() {
                                    prefs.times = if at == 0 { Times::Relative } else { Times::Clock };
                                }
                            },
                        );
                        setting_row(
                            ui,
                            "Confirm before cancelling a run",
                            "Ask once more before a run going is stopped.",
                            |ui| {
                                ui.add(Switch::new(
                                    &mut prefs.confirm_cancel,
                                    "Confirm before cancelling a run",
                                ));
                            },
                        );
                    }
                    "notifications" => {
                        setting_row(
                            ui,
                            "Tell me when any run fails",
                            "A note in the corner names the job and what its step said.",
                            |ui| {
                                ui.add(Switch::new(&mut prefs.notify_failures, "Tell me when any run fails"));
                            },
                        );
                        setting_row(
                            ui,
                            "Tell me when my runs finish",
                            "A note when a run you started succeeds or fails.",
                            |ui| {
                                ui.add(Switch::new(&mut prefs.notify_mine, "Tell me when my runs finish"));
                            },
                        );
                    }
                    "logs" => {
                        setting_row(
                            ui,
                            "Follow new lines",
                            "A run's log keeps its newest line in view until you scroll up.",
                            |ui| {
                                ui.add(Switch::new(&mut prefs.follow_log, "Follow new lines"));
                            },
                        );
                        setting_row(
                            ui,
                            "Wrap long lines",
                            "Long lines wrap onto more lines rather than scroll sideways.",
                            |ui| {
                                ui.add(Switch::new(&mut prefs.wrap_log, "Wrap long lines"));
                            },
                        );
                    }
                    "appearance" => theme_settings_of(ui, appearance, &CARRIED, true, &ThemeWords::default()),
                    "scheduler" => setting_row(
                        ui,
                        "Start runs on their schedules",
                        "Off, no schedule starts a run; runs started by hand still go.",
                        |ui| {
                            ui.add(Switch::new(&mut prefs.scheduler, "Start runs on their schedules"));
                        },
                    ),
                    "account" => setting_row(
                        ui,
                        &format!("Signed in as {}", me.word()),
                        "On tidevann.example.org. Signing out goes back to the sign-in page.",
                        |ui| {
                            let button = Button::new("Sign out")
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Sm);
                            sign_out = ui.add(button).clicked();
                        },
                    ),
                    _ => setting_row(
                        ui,
                        "Reset demo data",
                        "Puts back the jobs, two days of runs, the views and these settings, as the demo started.",
                        |ui| {
                            let button = Button::new("Reset")
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Sm);
                            reset = ui.add(button).clicked();
                        },
                    ),
                });
            if outcome.reset {
                let defaults = Prefs::default();
                match active.as_str() {
                    "general" => {
                        prefs.open_runs = defaults.open_runs;
                        prefs.times = defaults.times;
                        prefs.confirm_cancel = defaults.confirm_cancel;
                    }
                    "notifications" => {
                        prefs.notify_failures = defaults.notify_failures;
                        prefs.notify_mine = defaults.notify_mine;
                    }
                    "logs" => {
                        prefs.follow_log = defaults.follow_log;
                        prefs.wrap_log = defaults.wrap_log;
                    }
                    "scheduler" => prefs.scheduler = defaults.scheduler,
                    "appearance" => self.appearance = Appearance::default(),
                    _ => {}
                }
            }
            if sign_out {
                prefs.user = None;
            }
            if let Some(mine) = self.backend.prefs.first_mut() {
                *mine = prefs;
            }
            if reset {
                self.resetting = true;
            }
            if !outcome.close && !sign_out {
                self.settings = Some(active);
            }
        }
        if self.resetting {
            let question = ConfirmDialog::new("reset-demo", "Reset the demo data?", "Reset")
                .detail("Runs, jobs and views made since are gone, and the page loads again.")
                .destructive(true);
            match question.show(ctx) {
                Some(ConfirmAnswer::Confirm) => self.reset_demo(),
                Some(_) => self.resetting = false,
                None => {}
            }
        }
    }
}
