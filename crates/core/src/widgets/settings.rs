//! The settings every app has, as one dialog over the page: its sections in a column at the
//! start, the chosen section's rows, and a footer that resets the section and closes. After
//! sokkel-ui's `SettingsDialog` and `SettingRow`.

use egui::{
    Align, Context, CornerRadius, Frame, Id, Layout, Margin, Rect, Sense, Shadow, Stroke, Ui, UiBuilder, pos2, vec2,
};

use crate::{
    Appearance, Button, ButtonSize, ButtonVariant, Icon, Scheme, Segmented, SegmentedVariant, Select, TextRole,
    densities, icons, themes, tokens,
};

/// One section of the settings: an id the app keeps, a label and an icon in the column.
#[derive(Clone, Debug)]
pub struct SettingsSection {
    pub id: String,
    pub label: String,
    pub icon: Icon,
}

impl SettingsSection {
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: Icon) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
        }
    }
}

/// The words a settings dialog says, English by default.
#[derive(Clone, Debug)]
pub struct SettingsWords {
    pub title: String,
    pub close: String,
    pub reset: String,
    pub done: String,
    pub saved: String,
}

impl Default for SettingsWords {
    fn default() -> Self {
        Self {
            title: "Settings".into(),
            close: "Close".into(),
            reset: "Reset to defaults".into(),
            done: "Done".into(),
            saved: "Changes are kept as you make them".into(),
        }
    }
}

/// What a settings dialog was asked this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettingsOutcome {
    /// Done, Close, Escape or a press on the shade.
    pub close: bool,
    /// Reset to defaults, for the section shown.
    pub reset: bool,
}

/// The settings dialog: `sections` in a column, the active one's rows drawn by `rows`.
pub struct SettingsDialog<'a> {
    id: Id,
    sections: &'a [SettingsSection],
    active: &'a mut String,
    words: SettingsWords,
}

impl<'a> SettingsDialog<'a> {
    pub fn new(id: impl egui::AsId, sections: &'a [SettingsSection], active: &'a mut String) -> Self {
        Self {
            id: Id::new(id),
            sections,
            active,
            words: SettingsWords::default(),
        }
    }

    pub fn words(mut self, words: SettingsWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ctx: &Context, rows: impl FnOnce(&str, &mut Ui)) -> SettingsOutcome {
        let t = tokens(ctx);
        let mut outcome = SettingsOutcome::default();
        let (width, height) = (760.0, 520.0);
        let radius = t.radius("surface");
        let frame = Frame::new()
            .fill(t.color("popover"))
            .stroke(Stroke::new(1.0, t.color_at("foreground", 0.1)))
            .corner_radius(radius)
            .shadow(Shadow {
                offset: [0, 14],
                blur: 44,
                spread: 0,
                color: egui::Color32::from_black_alpha(if t.dark { 120 } else { 46 }),
            });
        let response = egui::Modal::new(self.id)
            .backdrop_color(t.color_at("shade", 0.1))
            .frame(frame)
            .show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
                let head = Rect::from_min_size(rect.min, vec2(width, 48.0));
                let foot = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 64.0), rect.max);
                let nav = Rect::from_min_max(pos2(rect.left(), head.bottom()), pos2(rect.left() + 208.0, foot.top()));
                let body = Rect::from_min_max(pos2(nav.right(), head.bottom()), pos2(rect.right(), foot.top()));
                let painter = ui.painter().clone();
                let rule = Stroke::new(1.0, t.color("border"));

                // the head: the title and the close button
                painter.hline(head.x_range(), head.bottom() - 0.5, rule);
                painter.text(
                    pos2(head.left() + 16.0, head.center().y),
                    egui::Align2::LEFT_CENTER,
                    &self.words.title,
                    TextRole::DetailTitle.font(&t),
                    t.color("popover-foreground"),
                );
                let close = Rect::from_center_size(pos2(head.right() - 28.0, head.center().y), vec2(28.0, 28.0));
                let close_hit = ui.interact(close, self.id.with("close"), Sense::click());
                close_hit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &self.words.close));
                if close_hit.hovered() {
                    painter.rect_filled(close, t.radius("control"), t.color("hover"));
                }
                icons::paint(&painter, close.center(), Icon::X, 16.0, t.color("muted-foreground"));
                if close_hit.clicked() {
                    outcome.close = true;
                }

                // the sections' column
                painter.rect_filled(nav, CornerRadius::ZERO, t.color_at("muted", 0.3));
                painter.vline(nav.right() - 0.5, nav.y_range(), rule);
                let mut y = nav.top() + 8.0;
                for section in self.sections {
                    let row = Rect::from_min_size(
                        pos2(nav.left() + 8.0, y),
                        vec2(nav.width() - 16.0, t.density("row") - 4.0),
                    );
                    let hit = ui.interact(row, self.id.with(("section", &section.id)), Sense::click());
                    let active = *self.active == section.id;
                    hit.widget_info(|| {
                        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, active, &section.label)
                    });
                    if hit.clicked() {
                        *self.active = section.id.clone();
                    }
                    if active {
                        painter.rect_filled(row, t.radius("md"), t.color("secondary"));
                    } else if hit.hovered() {
                        painter.rect_filled(row, t.radius("md"), t.color("hover"));
                    }
                    let ink = if active {
                        t.color("foreground")
                    } else {
                        t.color("muted-foreground")
                    };
                    icons::paint(
                        &painter,
                        pos2(row.left() + 18.0, row.center().y),
                        section.icon,
                        16.0,
                        ink,
                    );
                    let role = if active { TextRole::Action } else { TextRole::Body };
                    let galley = egui::WidgetText::from(role.rich(&t, &section.label)).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Truncate),
                        row.width() - 44.0,
                        role.font(&t),
                    );
                    painter.galley(
                        pos2(row.left() + 36.0, row.center().y - galley.size().y / 2.0),
                        galley,
                        ink,
                    );
                    y += row.height() + 2.0;
                }

                // the chosen section's rows
                let shown = self
                    .sections
                    .iter()
                    .find(|s| s.id == *self.active)
                    .or(self.sections.first());
                ui.scope_builder(
                    UiBuilder::new()
                        .max_rect(body.shrink2(vec2(32.0, 20.0)))
                        .layout(Layout::top_down(Align::Min)),
                    |ui| {
                        if let Some(section) = shown {
                            ui.label(
                                TextRole::DetailTitle
                                    .rich(&t, &section.label)
                                    .color(t.color("popover-foreground")),
                            );
                            ui.add_space(4.0);
                            rows(&section.id, ui);
                        }
                    },
                );

                // the footer: reset the section, the note, and Done
                painter.rect_filled(foot, CornerRadius { nw: 0, ne: 0, ..radius }, t.color_at("muted", 0.5));
                painter.hline(foot.x_range(), foot.top() + 0.5, rule);
                ui.scope_builder(
                    UiBuilder::new()
                        .max_rect(foot.shrink2(vec2(16.0, 0.0)))
                        .layout(Layout::left_to_right(Align::Center)),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        if ui
                            .add(
                                Button::new(self.words.reset.clone())
                                    .variant(ButtonVariant::Outline)
                                    .size(ButtonSize::Sm),
                            )
                            .clicked()
                        {
                            outcome.reset = true;
                        }
                        ui.label(
                            TextRole::Help
                                .rich(&t, &self.words.saved)
                                .color(t.color("muted-foreground")),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.add(Button::new(self.words.done.clone())).clicked() {
                                outcome.close = true;
                            }
                        });
                    },
                );
            });
        outcome.close |= response.should_close();
        outcome
    }
}

/// One setting: its title and description at the start, its control at the end, a rule under it.
pub fn setting_row(ui: &mut Ui, title: &str, description: &str, control: impl FnOnce(&mut Ui)) {
    let t = tokens(ui.ctx());
    Frame::new().inner_margin(Margin::symmetric(0, 16)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_max_width((ui.available_width() - 240.0).max(160.0));
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(TextRole::Label.rich(&t, title).color(t.color("popover-foreground")));
                ui.label(TextRole::Body.rich(&t, description).color(t.color("muted-foreground")));
            });
            ui.with_layout(Layout::right_to_left(Align::Min), control);
        });
    });
    let rect = ui.available_rect_before_wrap();
    ui.painter()
        .hline(rect.x_range(), rect.top(), Stroke::new(1.0, t.color("border")));
}

/// The appearance rows every app's settings have: the theme, the scheme and the density.
pub fn appearance_settings(ui: &mut Ui, appearance: &mut Appearance) {
    setting_row(
        ui,
        "Theme",
        "How the app looks: its grounds, its inks and its corners.",
        |ui| {
            let all: Vec<_> = themes().collect();
            let mut chosen = all.iter().position(|theme| theme.name == appearance.theme).unwrap_or(0);
            if ui
                .add(Select::new(&mut chosen, all.iter().map(|t| t.label.clone()), "Theme").width(200.0))
                .changed()
            {
                appearance.theme = all[chosen].name.clone();
            }
        },
    );
    setting_row(ui, "Scheme", "Light, dark, or as the system is set.", |ui| {
        let schemes = [Scheme::Light, Scheme::Dark, Scheme::System];
        let mut chosen = schemes.iter().position(|s| *s == appearance.scheme).unwrap_or(2);
        if ui
            .add(Segmented::new(&mut chosen, ["Light", "Dark", "System"]).variant(SegmentedVariant::Track))
            .changed()
        {
            appearance.scheme = schemes[chosen];
        }
    });
    setting_row(ui, "Density", "How much room rows and controls take.", |ui| {
        let levels = densities();
        let mut chosen = levels
            .iter()
            .position(|level| *level == appearance.density)
            .unwrap_or(0);
        let labels = levels.iter().map(|level| {
            let mut label = level.to_string();
            label[..1].make_ascii_uppercase();
            label
        });
        if ui
            .add(Segmented::new(&mut chosen, labels).variant(SegmentedVariant::Track))
            .changed()
        {
            appearance.density = levels[chosen].to_owned();
        }
    });
}
