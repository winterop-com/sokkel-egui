//! The board's settings, as the web's `Settings`: a `SettingsDialog` over the page with how the
//! board reads and tells, the simulation, the status page and its subscribers, and the look.

use eframe::egui::Context;
use sokkel_ui_egui::{
    Button, ButtonSize, ButtonVariant, Cell, Column, ConfirmAnswer, ConfirmDialog, DataTable, Icon, Input, MenuItem,
    Segmented, Select, SettingsDialog, SettingsSection, StackLayout, StackPlace, Switch, TableVariant, TableWords,
    ThemeWords, Toast, ToastKind, setting_row, setting_row_below, setting_row_found_by, theme_settings,
};

use crate::data::{SERVICES, SITES};
use crate::live::ScenarioKind;
use crate::{Notify, Options, StatusApp, Weather, relative_time};

/// Where the settings dialog stands while it is open.
pub struct SettingsState {
    pub active: String,
    /// The service and the site the Demo's trouble goes to, by index.
    pub service: usize,
    pub site: usize,
    /// Reset demo data asked, waiting for its confirm.
    pub resetting: bool,
    pub subscribers: sokkel_ui_egui::TableState,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            active: "board".into(),
            service: SERVICES.iter().position(|s| s.id == "api").unwrap_or(0),
            site: SITES.iter().position(|s| s.id == "singapore").unwrap_or(0),
            resetting: false,
            subscribers: Default::default(),
        }
    }
}

/// The sections, in the web's groups.
fn sections(subscribers: usize) -> Vec<SettingsSection> {
    vec![
        SettingsSection::new("board", "Readings", Icon::MonitorCog).group("Board"),
        SettingsSection::new("notices", "Notices", Icon::Bell).group("Board"),
        SettingsSection::new("incidents", "Incidents", Icon::Siren).group("Board"),
        SettingsSection::new("simulation", "Simulation", Icon::CloudLightning).group("Demo"),
        SettingsSection::new("page", "Status page", Icon::Globe).group("Status page"),
        SettingsSection::new("subscribers", "Subscribers", Icon::Mail)
            .heading(format!("Subscribers ({subscribers})"))
            .group("Status page")
            .reset(false),
        SettingsSection::new("appearance", "Appearance", Icon::Palette).group("You"),
    ]
}

impl StatusApp {
    /// The settings dialog while it is open, and the reset demo confirm over it.
    pub(crate) fn settings_dialog(&mut self, ctx: &Context) {
        if !self.settings {
            return;
        }
        let mut state = std::mem::take(&mut self.settings_state);
        let sections = sections(self.subscribe.subscribers.len());
        let going = self.live_world.scenarios.len();
        let now = (self.start + self.elapsed) as f64 * 1000.0;
        let mut trouble: Option<(ScenarioKind, usize)> = None;
        let mut calm = false;
        let mut to_public = false;
        let mut removed: Option<usize> = None;
        let subscriber_rows: Vec<Vec<Cell>> = self
            .subscribe
            .subscribers
            .iter()
            .map(|one| {
                vec![
                    Cell::Text(one.email.clone()),
                    if one.services.is_empty() {
                        Cell::Text("every service".into())
                    } else {
                        Cell::Tags(one.services.iter().map(|s| s.to_string()).collect())
                    },
                    Cell::Text(relative_time(one.at - now)),
                ]
            })
            .collect();
        let (options, live, appearance) = (&mut self.options, &mut self.live, &mut self.appearance);
        let SettingsState {
            active,
            service,
            site,
            resetting,
            subscribers,
        } = &mut state;
        let outcome = SettingsDialog::new("settings", &sections, active).show(ctx, |section, ui| match section {
            "board" => {
                setting_row_found_by(
                    ui,
                    "Live readings",
                    "Take a reading of every service and site on a timer. Off holds the board still.",
                    &["pause", "resume"],
                    |ui| {
                        ui.add(Switch::new(live, "Live readings"));
                    },
                );
                setting_row_found_by(
                    ui,
                    "A reading every",
                    "How often the tiles, charts, map and states update.",
                    &["refresh", "interval"],
                    |ui| {
                        let mut at = [1, 2, 5].iter().position(|s| *s == options.interval).unwrap_or(1);
                        if ui.add(Segmented::new(&mut at, ["1 s", "2 s", "5 s"])).changed() {
                            options.interval = [1, 2, 5][at];
                        }
                    },
                );
                setting_row(ui, "Clock", "How times of day are written.", |ui| {
                    let mut at = usize::from(options.twelve_hour);
                    if ui.add(Segmented::new(&mut at, ["24-hour", "12-hour"])).changed() {
                        options.twelve_hour = at == 1;
                    }
                });
            }
            "notices" => {
                setting_row_below(
                    ui,
                    "Tell me when",
                    "A notice in the corner when a service or a site changes state, and when an incident opens or is resolved.",
                    &["toast", "notification"],
                    |ui| {
                        let all = [Notify::All, Notify::Outages, Notify::Off];
                        let mut at = all.iter().position(|n| *n == options.notify).unwrap_or(0);
                        if ui
                            .add(Segmented::new(&mut at, ["Anything changes", "Outages", "Never"]))
                            .changed()
                        {
                            options.notify = all[at];
                        }
                    },
                );
                setting_row(ui, "A test notice", "Shows what a notice looks like.", |ui| {
                    let button = Button::new("Show one").variant(ButtonVariant::Outline).size(ButtonSize::Sm);
                    if ui.add(button).clicked() {
                        Toast::new("A test notice")
                            .kind(ToastKind::Info)
                            .description("Notices show here.")
                            .timeout(Some(4.0))
                            .show(ui.ctx());
                    }
                });
            }
            "incidents" => {
                setting_row(
                    ui,
                    "Open incidents by themselves",
                    "When a service or a site goes wrong and no open incident covers it, open one, given to whoever is on call.",
                    |ui| {
                        ui.add(Switch::new(&mut options.auto_incidents, "Open incidents by themselves"));
                    },
                );
                setting_row(
                    ui,
                    "On-call responder",
                    "Moves an incident the board opened through identified, monitoring and resolved as the service recovers, until a person posts.",
                    |ui| {
                        ui.add(Switch::new(&mut options.responder, "On-call responder"));
                    },
                );
            }
            "simulation" => {
                let things = if going == 1 { "thing is" } else { "things are" };
                setting_row_below(
                    ui,
                    "How often things go wrong",
                    &format!("{going} {things} going wrong now."),
                    &[],
                    |ui| {
                        let all = [Weather::Calm, Weather::Normal, Weather::Stormy];
                        let mut at = all.iter().position(|w| *w == options.weather).unwrap_or(1);
                        if ui
                            .add(Segmented::new(&mut at, ["Rarely", "Now and then", "Often"]))
                            .changed()
                        {
                            options.weather = all[at];
                        }
                    },
                );
                setting_row_below(
                    ui,
                    "Trouble on a service",
                    "An error spike or a slow drift, now, on the service chosen.",
                    &[],
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let names: Vec<&str> = SERVICES.iter().map(|s| s.id).collect();
                        ui.add(Select::new(service, names, "Service").width(136.0));
                        let small = |word: &'static str| Button::new(word).variant(ButtonVariant::Outline).size(ButtonSize::Sm);
                        if ui.add(small("Error spike")).clicked() {
                            trouble = Some((ScenarioKind::Spike, *service));
                        }
                        if ui.add(small("Slow drift")).clicked() {
                            trouble = Some((ScenarioKind::Drift, *service));
                        }
                    },
                );
                setting_row_below(
                    ui,
                    "Trouble in a region",
                    "A site falls behind for a while, then recovers.",
                    &[],
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let names: Vec<&str> = SITES.iter().map(|s| s.city).collect();
                        ui.add(Select::new(site, names, "Site").width(160.0));
                        let start = Button::new("Start").variant(ButtonVariant::Outline).size(ButtonSize::Sm);
                        if ui.add(start).clicked() {
                            trouble = Some((ScenarioKind::Region, *site));
                        }
                    },
                );
                setting_row(ui, "Calm everything down", "Ends whatever is going wrong now.", |ui| {
                    let button = Button::new("Calm down").variant(ButtonVariant::Outline).size(ButtonSize::Sm);
                    calm = ui.add_enabled(going > 0, button).clicked();
                });
                setting_row(
                    ui,
                    "Reset demo data",
                    "Puts the incidents, maintenance, deploys, rotation and subscribers back as they were.",
                    |ui| {
                        let button = Button::new("Reset").variant(ButtonVariant::Outline).size(ButtonSize::Sm);
                        if ui.add(button).clicked() {
                            *resetting = true;
                        }
                    },
                );
            }
            "page" => {
                setting_row_below(ui, "Title", "What the status page is called at its top.", &[], |ui| {
                    ui.add(Input::new(&mut options.page_title).width(320.0));
                });
                setting_row(ui, "Show the last 90 days", "A bar a day under each service.", |ui| {
                    ui.add(Switch::new(&mut options.show_uptime, "Show the last 90 days"));
                });
                setting_row(
                    ui,
                    "Past incidents",
                    "How many days back the page lists what happened.",
                    |ui| {
                        let mut at = usize::from(options.history_days == 14);
                        if ui.add(Segmented::new(&mut at, ["7 days", "14 days"])).changed() {
                            options.history_days = if at == 1 { 14 } else { 7 };
                        }
                    },
                );
                setting_row(ui, "Open the status page", "What customers see.", |ui| {
                    let button = Button::new("Open").variant(ButtonVariant::Outline).size(ButtonSize::Sm);
                    to_public = ui.add(button).clicked();
                });
            }
            "subscribers" => {
                setting_row_below(
                    ui,
                    "Who hears of incidents",
                    "People subscribe on the status page; remove one here.",
                    &["email", "unsubscribe"],
                    |ui| {
                        let columns = [
                            Column::new("Email", 192.0).fill(),
                            Column::new("Hears about", 128.0).stack(StackPlace::Secondary),
                            Column::new("Since", 128.0).stack(StackPlace::Secondary),
                        ];
                        let output = DataTable::new("subscribers", &columns, &subscriber_rows, subscribers)
                            .variant(TableVariant::List)
                            .stack_layout(StackLayout::Labeled)
                            .actions(|_| vec![MenuItem::action("Remove").icon(Icon::Trash2).destructive()])
                            .words(TableWords {
                                empty: "Nobody has subscribed yet.".into(),
                                ..Default::default()
                            })
                            .footer(false)
                            .show(ui);
                        if let Some((row, _)) = output.action {
                            removed = Some(row);
                        }
                    },
                );
            }
            _ => theme_settings(ui, appearance, true, &ThemeWords::default()),
        });
        if outcome.reset {
            let fresh = Options::default();
            match state.active.as_str() {
                "board" => {
                    self.live = true;
                    self.options.interval = fresh.interval;
                    self.options.twelve_hour = fresh.twelve_hour;
                }
                "notices" => self.options.notify = fresh.notify,
                "incidents" => {
                    self.options.auto_incidents = fresh.auto_incidents;
                    self.options.responder = fresh.responder;
                }
                "simulation" => self.options.weather = fresh.weather,
                "page" => {
                    self.options.page_title = fresh.page_title;
                    self.options.show_uptime = fresh.show_uptime;
                    self.options.history_days = fresh.history_days;
                }
                "appearance" => self.appearance = Default::default(),
                _ => {}
            }
        }
        if let Some((kind, target)) = trouble {
            self.live_world.trigger(kind, Some(target), now);
        }
        if calm {
            self.live_world.calm_down();
        }
        if let Some(row) = removed
            && row < self.subscribe.subscribers.len()
        {
            self.subscribe.subscribers.remove(row);
        }
        if state.resetting {
            let dialog = ConfirmDialog::new("reset-demo", "Reset the demo data?", "Reset")
                .detail(
                    "Every incident, window, deploy, shift, handover and subscriber goes back to how the demo started.",
                )
                .destructive(true);
            match dialog.show(ctx) {
                Some(ConfirmAnswer::Confirm) => {
                    self.reset_demo();
                    return;
                }
                Some(_) => state.resetting = false,
                None => {}
            }
        }
        self.settings_state = state;
        if outcome.close {
            self.settings = false;
        }
        if to_public {
            self.settings = false;
            self.go("public");
        }
    }

    /// Everything the demo keeps back as it started, the look and the settings kept.
    pub fn reset_demo(&mut self) {
        let appearance = self.appearance.clone();
        let options = self.options.clone();
        let desktop = self.desktop;
        *self = Self::on(appearance);
        self.options = options;
        self.desktop = desktop;
    }
}
