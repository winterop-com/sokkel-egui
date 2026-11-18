//! The reader's settings as a dialog over the page, as the web's `pages/Settings.tsx`: the editor,
//! the daily notes, the look and the vault.

use eframe::egui;
use sokkel_egui::{
    Appearance, Button, ButtonSize, ButtonVariant, CodeEditor, CodeLanguage, ConfirmAnswer, ConfirmDialog, Icon,
    Segmented, Select, SettingsDialog, SettingsSection, Switch, ThemeWords, setting_row, setting_row_below,
    setting_row_found_by, theme_settings_of,
};

use crate::app::NotesApp;
use crate::backend::Backend;
use crate::model::{FontSize, Order, Prefs, TEMPLATE, View};

/// The themes the app carries, as the web's `themes.ts`: the only ones it offers.
pub const CARRIED: [&str; 4] = ["tidevann", "paper", "contrast", "ocean"];

/// How long the template rests after typing stops before it is kept, as the web's 500 ms.
const TEMPLATE_REST: f64 = 0.5;

impl NotesApp {
    /// The settings over the page while they are open, and the question before the demo's data
    /// goes back.
    pub fn settings_dialog(&mut self, ctx: &egui::Context) {
        if let Some(mut active) = self.settings.take() {
            let sections = [
                SettingsSection::new("editor", "Editor", Icon::PenLine).group("Notes"),
                SettingsSection::new("daily", "Daily notes", Icon::CalendarDays).group("Notes"),
                SettingsSection::new("appearance", "Appearance", Icon::Palette).group("App"),
                SettingsSection::new("vault", "Vault", Icon::Database).group("App"),
            ];
            let mut prefs = self.backend.prefs.clone();
            let mut order = self.backend.order();
            let vault = self.backend.vault();
            let notes = vault.notes.len();
            // the folders by their path, the top of the vault first
            let mut folders: Vec<(String, String)> = vault
                .folders
                .iter()
                .map(|f| (f.id.clone(), vault.path(Some(&f.id))))
                .collect();
            folders.sort_by(|a, b| a.1.cmp(&b.1));
            let now_time = ctx.input(|i| i.time);
            let template = &mut self.template_draft;
            let appearance = &mut self.appearance;
            let mut reset = false;
            let mut template_typed = false;
            let outcome =
                SettingsDialog::new("settings", &sections, &mut active).show(ctx, |section, ui| match section {
                    "editor" => {
                        setting_row_found_by(
                            ui,
                            "Text size",
                            "The editor's text, from the size of help text to a step over the body.",
                            &["font", "zoom"],
                            |ui| {
                                let sizes = [FontSize::Sm, FontSize::Md, FontSize::Lg];
                                let mut at = sizes.iter().position(|s| *s == prefs.font_size).unwrap_or(1);
                                if ui.add(Segmented::new(&mut at, ["Small", "Medium", "Large"])).changed() {
                                    prefs.font_size = sizes[at];
                                }
                            },
                        );
                        setting_row(
                            ui,
                            "Wrap long lines",
                            "Long lines fold at the edge rather than scroll sideways.",
                            |ui| {
                                ui.add(Switch::new(&mut prefs.wrap, "Wrap long lines"));
                            },
                        );
                        setting_row(ui, "Line numbers", "A number before each line of the editor.", |ui| {
                            ui.add(Switch::new(&mut prefs.line_numbers, "Line numbers"));
                        });
                        setting_row_found_by(
                            ui,
                            "Open notes in",
                            "What a note shows when it is opened: the editor, both side by side, or the page to read.",
                            &["preview", "reading", "split"],
                            |ui| {
                                let views = [View::Edit, View::Both, View::Preview];
                                let mut at = views.iter().position(|v| *v == prefs.default_view).unwrap_or(1);
                                if ui.add(Segmented::new(&mut at, ["Edit", "Both", "Read"])).changed() {
                                    prefs.default_view = views[at];
                                }
                            },
                        );
                    }
                    "daily" => {
                        setting_row(ui, "Folder", "Where a new day's note goes.", |ui| {
                            let mut at = prefs
                                .daily_folder
                                .as_ref()
                                .and_then(|id| folders.iter().position(|f| &f.0 == id))
                                .map_or(0, |i| i + 1);
                            let names = std::iter::once("Top of the vault".to_owned())
                                .chain(folders.iter().map(|f| f.1.clone()));
                            ui.add(Select::new(&mut at, names, "Folder for daily notes").small(true));
                            prefs.daily_folder = at.checked_sub(1).map(|i| folders[i].0.clone());
                        });
                        setting_row_below(
                            ui,
                            "Template",
                            "What a new day's note starts with. {{date}} becomes the day, {{long}} the day written out.",
                            &[],
                            |ui| {
                                let (text, _) = template.get_or_insert_with(|| (prefs.template.clone(), None));
                                let out = CodeEditor::new("daily-template", text)
                                    .label("Daily note template")
                                    .language(CodeLanguage::Markdown)
                                    .wrap(true)
                                    .line_numbers(false)
                                    .height(192.0)
                                    .show(ui);
                                template_typed = out.changed;
                            },
                        );
                    }
                    "appearance" => theme_settings_of(ui, appearance, &CARRIED, true, &ThemeWords::default()),
                    _ => {
                        setting_row_found_by(
                            ui,
                            "Order",
                            "How each folder lists its notes and folders: by name, or in the order you drag them into.",
                            &["sort", "drag", "manual"],
                            |ui| {
                                let mut at = usize::from(order == Order::Manual);
                                if ui.add(Segmented::new(&mut at, ["Name", "Manual"])).changed() {
                                    order = if at == 0 { Order::Name } else { Order::Manual };
                                }
                            },
                        );
                        setting_row(
                            ui,
                            "Close every tab",
                            &format!("{} notes are open as tabs.", prefs.tabs.len()),
                            |ui| {
                                let button = Button::new("Close all")
                                    .variant(ButtonVariant::Outline)
                                    .size(ButtonSize::Sm)
                                    .enabled(!prefs.tabs.is_empty());
                                if ui.add(button).clicked() {
                                    prefs.tabs.clear();
                                    prefs.active = None;
                                }
                            },
                        );
                        setting_row(
                            ui,
                            "Forget the recent notes",
                            &format!("{} notes are listed as recent.", prefs.recent.len()),
                            |ui| {
                                let button = Button::new("Forget")
                                    .variant(ButtonVariant::Outline)
                                    .size(ButtonSize::Sm)
                                    .enabled(!prefs.recent.is_empty());
                                if ui.add(button).clicked() {
                                    prefs.recent.clear();
                                }
                            },
                        );
                        setting_row_found_by(
                            ui,
                            "Reset the demo data",
                            &format!(
                                "Puts back the vault as it started, {notes} notes now, and loads the page again."
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
            // the template is kept once typing has rested a moment
            if let Some((text, typed_at)) = &mut self.template_draft {
                if template_typed {
                    *typed_at = Some(now_time);
                }
                match *typed_at {
                    Some(at) if now_time - at >= TEMPLATE_REST => {
                        prefs.template = text.clone();
                        *typed_at = None;
                    }
                    Some(_) => ctx.request_repaint_after(std::time::Duration::from_millis(100)),
                    None => {}
                }
            }
            if outcome.reset {
                let defaults = Prefs::default();
                match active.as_str() {
                    "editor" => {
                        prefs.font_size = defaults.font_size;
                        prefs.wrap = defaults.wrap;
                        prefs.line_numbers = defaults.line_numbers;
                        prefs.default_view = defaults.default_view;
                    }
                    "daily" => {
                        prefs.daily_folder = defaults.daily_folder;
                        prefs.template = TEMPLATE.to_owned();
                        // put back from here, the template's editor follows
                        self.template_draft = None;
                    }
                    "vault" => order = defaults.order.unwrap_or(Order::Manual),
                    "appearance" => self.appearance = Appearance::default(),
                    _ => {}
                }
            }
            prefs.order = Some(order);
            if prefs != self.backend.prefs {
                self.backend.set_prefs(|p| *p = prefs);
            }
            if reset {
                self.resetting = true;
            }
            if outcome.close {
                self.template_draft = None;
            } else {
                self.settings = Some(active);
            }
        }
        if self.resetting {
            let question = ConfirmDialog::new("reset-demo", "Reset the demo data?", "Reset")
                .detail("Notes, folders and settings made since are gone, and the page loads again.")
                .destructive(true);
            match question.show(ctx) {
                Some(ConfirmAnswer::Confirm) => self.reset_demo(),
                Some(_) => self.resetting = false,
                None => {}
            }
        }
    }

    /// The vault as it started, and the app on All notes.
    pub fn reset_demo(&mut self) {
        let now = self.now();
        self.backend = Backend::seeded(now);
        self.resetting = false;
        self.tables.clear();
        self.go("notes");
    }
}
