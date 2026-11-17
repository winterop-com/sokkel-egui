//! The person's settings as a dialog over the page, as the web's `Settings.tsx`: the workspace's
//! name, the currency and the opening range, the live stream, the notes in the corner, the look,
//! and the demo's data.

use eframe::egui;
use sokkel_egui::{
    Appearance, Button, ButtonSize, ButtonVariant, ConfirmAnswer, ConfirmDialog, Icon, InlineEdit, Segmented, Select,
    SettingRow, SettingsDialog, SettingsSection, Switch, ThemeWords, setting_row, setting_row_found_by, theme_settings,
};

use crate::app::{AnalyticsApp, Money};
use crate::model::{Currency, Pace, Prefs};
use crate::stream::LIVE_LIMIT;
use crate::view::{PRESETS, Range};

const CURRENCIES: [Currency; 3] = [Currency::Usd, Currency::Eur, Currency::Nok];
const PACES: [(Pace, &str); 3] = [
    (Pace::Fast, "Every second or two"),
    (Pace::Slow, "Every few seconds"),
    (Pace::Off, "Paused"),
];
const BIG_ORDERS: [i64; 4] = [0, 100, 500, 1000];

impl AnalyticsApp {
    /// The settings over the page while they are open, and the question before the demo's data
    /// goes back.
    pub fn settings_dialog(&mut self, ctx: &egui::Context) {
        if self.settings {
            let sections = [
                SettingsSection::new("general", "General", Icon::Settings2).group("Preferences"),
                SettingsSection::new("live", "Live data", Icon::Radio).group("Preferences"),
                SettingsSection::new("notifications", "Notifications", Icon::Bell).group("Preferences"),
                SettingsSection::new("appearance", "Appearance", Icon::Palette).group("Preferences"),
                SettingsSection::new("demo", "Demo data", Icon::Database)
                    .group("Workspace")
                    .reset(false),
            ];
            let before = self.prefs.clone();
            let live = self.world.live_count;
            let money = Money(self.prefs.currency);
            let mut active = std::mem::take(&mut self.settings_section);
            if active.is_empty() {
                active = "general".into();
            }
            let (prefs, appearance) = (&mut self.prefs, &mut self.appearance);
            let mut reset_data = false;
            let outcome = SettingsDialog::new("settings", &sections, &mut active).show(ctx, |section, ui| match section {
                "general" => {
                    setting_row(ui, "Workspace name", "What the rail and the overview call the business.", |ui| {
                        let edit = InlineEdit::new("workspace-name", "Workspace name", prefs.workspace.clone())
                            .validate(|v| v.trim().is_empty().then(|| "A workspace needs a name.".to_owned()));
                        if let Some(name) = edit.show(ui) {
                            prefs.workspace = name.trim().to_owned();
                        }
                    });
                    setting_row_found_by(
                        ui,
                        "Currency",
                        "Every figure of money in this currency, at a fixed rate from US dollars.",
                        &["money", "dollar", "euro", "krone"],
                        |ui| {
                            let mut at = CURRENCIES.iter().position(|c| *c == prefs.currency).unwrap_or(0);
                            if ui.add(Segmented::new(&mut at, CURRENCIES.map(Currency::code))).changed() {
                                prefs.currency = CURRENCIES[at];
                            }
                        },
                    );
                    setting_row(ui, "Opening range", "The range a page opens on when its address names none.", |ui| {
                        let mut at = PRESETS.iter().position(|p| p.id == prefs.range).unwrap_or(0);
                        let labels = PRESETS.iter().map(|p| p.label);
                        if ui.add(Select::new(&mut at, labels, "Opening range").width(150.0)).changed() {
                            prefs.range = PRESETS[at].id.to_owned();
                        }
                    });
                    setting_row(
                        ui,
                        "Compare with the period before",
                        "Changes on the tiles and dashed lines for the period before, unless the address says otherwise.",
                        |ui| {
                            ui.add(Switch::new(&mut prefs.compare, "Compare with the period before"));
                        },
                    );
                }
                "live" => {
                    SettingRow::new(
                        "Live stream",
                        "How often a sign-up, an upgrade, an order or a team leaving arrives while the app is open.",
                    )
                    .keywords(&["stream", "pause", "speed", "events"])
                    .below()
                    .note(format!(
                        "{} of {} live changes held; at the limit the stream stops until the demo data are reset.",
                        sokkel_egui::format_value(live as f64, Some(0)),
                        sokkel_egui::format_value(LIVE_LIMIT as f64, Some(0)),
                    ))
                    .show(ui, |ui| {
                        let mut at = PACES.iter().position(|(p, _)| *p == prefs.pace).unwrap_or(0);
                        if ui.add(Segmented::new(&mut at, PACES.map(|(_, l)| l))).changed() {
                            prefs.pace = PACES[at].0;
                        }
                    });
                }
                "notifications" => {
                    setting_row(
                        ui,
                        "Tell me about big orders",
                        "A note in the corner for each upgrade or order that bills at least this much at once.",
                        |ui| {
                            let mut at = BIG_ORDERS.iter().position(|v| *v == prefs.big_order).unwrap_or(2);
                            let labels = BIG_ORDERS.map(|v| if v == 0 { "Never".to_owned() } else { money.full(v as f64) });
                            if ui.add(Segmented::new(&mut at, labels)).changed() {
                                prefs.big_order = BIG_ORDERS[at];
                            }
                        },
                    );
                    setting_row_found_by(
                        ui,
                        "Tell me when a paying team leaves",
                        "A note in the corner naming the team and what it paid.",
                        &["churn", "cancel", "lost"],
                        |ui| {
                            ui.add(Switch::new(&mut prefs.notify_churn, "Tell me when a paying team leaves"));
                        },
                    );
                }
                "appearance" => theme_settings(ui, appearance, true, &ThemeWords::default()),
                _ => setting_row(
                    ui,
                    "Reset demo data",
                    "Takes away every live change, saved view, funnel, report and goal made since, and these settings, as the demo started.",
                    |ui| {
                        let reset = Button::new("Reset").size(ButtonSize::Sm).variant(ButtonVariant::Outline);
                        if ui.add(reset).clicked() {
                            reset_data = true;
                        }
                    },
                ),
            });
            if outcome.reset {
                let defaults = Prefs::default();
                match active.as_str() {
                    "general" => {
                        self.prefs.workspace = defaults.workspace;
                        self.prefs.currency = defaults.currency;
                        self.prefs.range = defaults.range;
                        self.prefs.compare = defaults.compare;
                    }
                    "live" => self.prefs.pace = defaults.pace,
                    "notifications" => {
                        self.prefs.big_order = defaults.big_order;
                        self.prefs.notify_churn = defaults.notify_churn;
                    }
                    "appearance" => self.appearance = Appearance::default(),
                    _ => {}
                }
            }
            // a page opens on the preferences when its address names none: the view follows
            if self.prefs.range != before.range {
                self.view.range = Range::preset(&self.prefs.range);
            }
            if self.prefs.compare != before.compare {
                self.view.compare = self.prefs.compare;
            }
            if reset_data {
                self.resetting = true;
            }
            if outcome.close {
                self.settings = false;
            }
            self.settings_section = active;
        }
        if self.resetting {
            let question = ConfirmDialog::new("reset-demo", "Reset the demo data?", "Reset")
                .detail("Everything changed since the demo started is gone, and the page loads again.")
                .destructive(true);
            match question.show(ctx) {
                Some(ConfirmAnswer::Confirm) => self.reset_demo(),
                Some(_) => self.resetting = false,
                None => {}
            }
        }
    }

    /// Everything as the demo started: the world, what was saved, and the settings; the look stays.
    fn reset_demo(&mut self) {
        let (appearance, desktop) = (self.appearance.clone(), self.desktop);
        let path = self.view.path.clone();
        *self = Self::at(self.now(), appearance);
        self.desktop = desktop;
        self.view.path = path;
    }
}
