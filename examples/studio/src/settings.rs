//! The reader's settings as a dialog over the page, as the web's `Settings.tsx`: the editor, the
//! look and the workspace.

use eframe::egui;
use sokkel_egui::{
    Appearance, Button, ButtonSize, ButtonVariant, ConfirmAnswer, ConfirmDialog, Icon, Segmented, SettingsDialog,
    SettingsSection, Switch, ThemeWords, setting_row, setting_row_found_by, theme_settings_of,
};

use crate::app::StudioApp;
use crate::backend::Backend;
use crate::model::{Prefs, TextSize, counted};

/// The themes the studio carries, whose styles the web's `app.css` imports.
pub const CARRIED: [&str; 4] = ["tidevann", "steel", "terminal", "paper"];

impl StudioApp {
    /// The settings over the page while they are open, and the question before the demo's data
    /// goes back.
    pub fn settings_dialog(&mut self, ctx: &egui::Context) {
        if let Some(mut active) = self.settings.take() {
            let sections = [
                SettingsSection::new("editor", "Editor", Icon::SquareCode).group("Studio"),
                SettingsSection::new("appearance", "Appearance", Icon::Palette).group("App"),
                SettingsSection::new("workspace", "Workspace", Icon::Database)
                    .group("App")
                    .reset(false),
            ];
            let mut prefs = self.backend.prefs.clone();
            let dirty = !self.backend.dirty().is_empty();
            let (files, runs) = (self.backend.files.len(), self.backend.runs.len());
            let appearance = &mut self.appearance;
            let (mut close_all, mut reset) = (false, false);
            let outcome =
                SettingsDialog::new("settings", &sections, &mut active).show(ctx, |section, ui| match section {
                    "editor" => {
                        setting_row_found_by(
                            ui,
                            "Text size",
                            "The editor's text, from the size of help text to a step over the body.",
                            &["font", "zoom"],
                            |ui| {
                                let sizes = [TextSize::Sm, TextSize::Md, TextSize::Lg];
                                let mut at = sizes.iter().position(|s| *s == prefs.size).unwrap_or(1);
                                if ui.add(Segmented::new(&mut at, ["Small", "Medium", "Large"])).changed() {
                                    prefs.size = sizes[at];
                                }
                            },
                        );
                        let flags: [(&str, &str, &[&str], &mut bool); 6] = [
                            (
                                "Wrap long lines",
                                "Long lines fold at the edge rather than scroll sideways.",
                                &[],
                                &mut prefs.wrap,
                            ),
                            (
                                "Line numbers",
                                "A number before each line, in the editors and the logs.",
                                &["gutter"],
                                &mut prefs.line_numbers,
                            ),
                            (
                                "Folding",
                                "Arrows in the gutter fold a block of YAML or JSON away.",
                                &["collapse"],
                                &mut prefs.folding,
                            ),
                            (
                                "Indent guides",
                                "A faint rule at each step of indent.",
                                &[],
                                &mut prefs.indent_guides,
                            ),
                            (
                                "Bracket colors",
                                "Brackets colored by depth, so a pair reads at a glance.",
                                &[],
                                &mut prefs.bracket_colors,
                            ),
                            (
                                "Keep the line shaded",
                                "The caret's line stays shaded while you work elsewhere, so you find your place again.",
                                &["highlight", "current line", "cursor"],
                                &mut prefs.line_wash,
                            ),
                        ];
                        for (title, description, keywords, on) in flags {
                            setting_row_found_by(ui, title, description, keywords, |ui| {
                                ui.add(Switch::new(on, title));
                            });
                        }
                    }
                    "appearance" => theme_settings_of(ui, appearance, &CARRIED, true, &ThemeWords::default()),
                    _ => {
                        let tabs = counted(prefs.tabs.len() as f64, "tab", None);
                        let said = if dirty {
                            format!("{tabs} open; those with changes not saved ask first.")
                        } else {
                            format!("{tabs} open.")
                        };
                        setting_row(ui, "Close every tab", &said, |ui| {
                            let button = Button::new("Close all")
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Sm);
                            close_all = ui.add_enabled(!prefs.tabs.is_empty(), button).clicked();
                        });
                        setting_row_found_by(
                            ui,
                            "Reset the demo data",
                            &format!(
                                "Puts back the workspace as it started ({} and {} now) and loads the page again.",
                                counted(files as f64, "file", None),
                                counted(runs as f64, "run", None)
                            ),
                            &["seed", "start over"],
                            |ui| {
                                let button = Button::new("Reset")
                                    .variant(ButtonVariant::Outline)
                                    .size(ButtonSize::Sm);
                                reset = ui.add(button).clicked();
                            },
                        );
                    }
                });
            if outcome.reset {
                let defaults = Prefs::default();
                match active.as_str() {
                    "editor" => {
                        prefs.wrap = defaults.wrap;
                        prefs.line_numbers = defaults.line_numbers;
                        prefs.size = defaults.size;
                        prefs.folding = defaults.folding;
                        prefs.indent_guides = defaults.indent_guides;
                        prefs.bracket_colors = defaults.bracket_colors;
                        prefs.line_wash = defaults.line_wash;
                    }
                    "appearance" => self.appearance = Appearance::default(),
                    _ => {}
                }
            }
            if prefs != self.backend.prefs {
                self.backend.set_prefs(|p| *p = prefs.clone());
            }
            if reset {
                self.resetting = true;
            }
            if close_all {
                let tabs = prefs.tabs.clone();
                self.close_tabs(&tabs);
            }
            if !outcome.close {
                self.settings = Some(active);
            }
        }
        if self.resetting {
            let question = ConfirmDialog::new("reset-demo", "Reset the demo data?", "Reset")
                .detail(
                    "Files, runs and settings made since are gone, changes not saved too, and the page loads again.",
                )
                .destructive(true);
            match question.show(ctx) {
                Some(ConfirmAnswer::Confirm) => self.reset_demo(),
                Some(_) => self.resetting = false,
                None => {}
            }
        }
    }

    /// The workspace as it started, and the app on its first tab.
    pub fn reset_demo(&mut self) {
        self.backend = Backend::seeded();
        self.resetting = false;
        self.tables.clear();
        self.views.clear();
        self.canvases.clear();
        self.go("");
    }
}
