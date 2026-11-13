//! The settings every app has, as one dialog over the page: its sections in a column at the
//! start, the chosen section's rows, and a footer that resets the section and closes. After
//! sokkel-ui's `SettingsDialog` and `SettingRow`.

use egui::{
    Align, Context, CornerRadius, Frame, Id, Layout, Margin, Rect, Sense, Shadow, Stroke, Ui, UiBuilder, pos2, vec2,
};

use crate::{
    Appearance, Button, ButtonVariant, Icon, Scheme, Segmented, SegmentedVariant, Select, TextRole, densities, icons,
    themes, tokens,
};

/// One section of the settings: an id the app keeps, a label and an icon in the column.
#[derive(Clone, Debug)]
pub struct SettingsSection {
    pub id: String,
    pub label: String,
    pub icon: Icon,
    /// The group title over this section and those after it with the same group.
    pub group: Option<String>,
}

impl SettingsSection {
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: Icon) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            group: None,
        }
    }

    /// Put the section under a group title in the column, as "Preferences" or "This demo".
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }
}

/// What `setting_row` reads while a settings dialog draws its rows: the search, and the section
/// title still to draw over the first row that shows.
#[derive(Clone, Default)]
struct RowsState {
    query: String,
    heading: Option<String>,
    shown: usize,
}

fn rows_state_id() -> Id {
    Id::new("sokkel-ui-egui/settings-rows")
}

/// The words a settings dialog says, English by default.
#[derive(Clone, Debug)]
pub struct SettingsWords {
    pub title: String,
    pub close: String,
    pub reset: String,
    pub done: String,
    pub saved: String,
    pub search: String,
    pub no_match: String,
}

impl Default for SettingsWords {
    fn default() -> Self {
        Self {
            title: "Settings".into(),
            close: "Close".into(),
            reset: "Reset to defaults".into(),
            done: "Done".into(),
            saved: "Changes are saved as you make them".into(),
            search: "Search".into(),
            no_match: "No setting matches".into(),
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

    /// Draw the dialog; `rows` draws a section's rows with `setting_row`, called for the section shown
    /// or, while the search holds words, for every section, its rows narrowed to those that match.
    pub fn show(self, ctx: &Context, mut rows: impl FnMut(&str, &mut Ui)) -> SettingsOutcome {
        let t = tokens(ctx);
        let mut outcome = SettingsOutcome::default();
        // 896 wide and as tall as the window leaves under the overlay's inset, at most 704
        let window = ctx.content_rect();
        let (width, height) = (
            896.0_f32.min(window.width() - 32.0),
            704.0_f32.min(window.height() - 96.0),
        );
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
        let query_id = self.id.with("search");
        let mut query: String = ctx.data(|d| d.get_temp(query_id)).unwrap_or_default();
        let response = egui::Modal::new(self.id)
            .backdrop_color(t.color_at("shade", 0.1))
            .frame(frame)
            .show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
                let head = Rect::from_min_size(rect.min, vec2(width, 48.0));
                let foot = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 65.0), rect.max);
                let nav = Rect::from_min_max(pos2(rect.left(), head.bottom()), pos2(rect.left() + 224.0, foot.top()));
                let body = Rect::from_min_max(pos2(nav.right(), head.bottom()), pos2(rect.right(), foot.top()));
                let painter = ui.painter().clone();
                let rule = Stroke::new(1.0, t.color("border"));

                // the head: the title and the close button
                painter.hline(head.x_range(), head.bottom() - 0.5, rule);
                let title = egui::WidgetText::from(TextRole::DetailTitle.rich(&t, &self.words.title)).into_galley(
                    ui,
                    Some(egui::TextWrapMode::Truncate),
                    head.width() - 80.0,
                    TextRole::DetailTitle.font(&t),
                );
                painter.galley(
                    pos2(head.left() + 16.0, head.center().y - title.size().y / 2.0),
                    title,
                    t.color("popover-foreground"),
                );
                let close = Rect::from_center_size(pos2(head.right() - 30.0, head.center().y), vec2(28.0, 28.0));
                let close_hit = ui.interact(close, self.id.with("close"), Sense::click());
                close_hit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &self.words.close));
                if close_hit.hovered() {
                    painter.rect_filled(close, t.radius("control"), t.color("hover"));
                }
                icons::paint(&painter, close.center(), Icon::X, 16.0, t.color("popover-foreground"));
                if close_hit.clicked() {
                    outcome.close = true;
                }

                // the column: the search, then the sections under their group titles
                painter.rect_filled(nav, CornerRadius::ZERO, t.color_at("muted", 0.3));
                painter.vline(nav.right() - 0.5, nav.y_range(), rule);
                let field = Rect::from_min_size(
                    pos2(nav.left() + 12.0, nav.top() + 12.0),
                    vec2(nav.width() - 24.0, 32.0),
                );
                let mut child = ui.new_child(UiBuilder::new().max_rect(field));
                let search = super::search_field(&mut child, &mut query, &self.words.search);
                // the search takes the focus as the dialog opens, as the web's does
                let opened_id = self.id.with("opened");
                if !ui.ctx().data(|d| d.get_temp::<bool>(opened_id).unwrap_or(false)) {
                    ui.ctx().data_mut(|d| d.insert_temp(opened_id, true));
                    search.request_focus();
                }
                let mut y = field.bottom();
                let mut group: Option<&String> = None;
                for (index, section) in self.sections.iter().enumerate() {
                    if section.group.is_some() && section.group.as_ref() != group {
                        group = section.group.as_ref();
                        // a group's title: 12 above, its line 20 tall, 10 before its sections
                        y += 12.0;
                        let label = egui::WidgetText::from(TextRole::Badge.rich(&t, group.unwrap_or(&String::new())))
                            .into_galley(
                                ui,
                                Some(egui::TextWrapMode::Truncate),
                                nav.width() - 32.0,
                                TextRole::Badge.font(&t),
                            );
                        painter.galley(
                            pos2(nav.left() + 20.0, y + 10.0 - label.size().y / 2.0),
                            label,
                            t.color("muted-foreground"),
                        );
                        y += 30.0;
                    } else if section.group.is_none() {
                        // ungrouped sections: the first keeps the same 12 from the search as a group title
                        y += if index == 0 { 12.0 } else { 2.0 };
                    }
                    let row = Rect::from_min_size(pos2(nav.left() + 12.0, y), vec2(nav.width() - 24.0, 34.0));
                    let hit = ui.interact(row, self.id.with(("section", &section.id)), Sense::click());
                    let active = *self.active == section.id && query.trim().is_empty();
                    hit.widget_info(|| {
                        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, active, &section.label)
                    });
                    if hit.clicked() {
                        *self.active = section.id.clone();
                        query.clear();
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
                        pos2(row.left() + 16.0, row.center().y),
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
                        pos2(row.left() + 34.0, row.center().y - galley.size().y / 2.0),
                        galley,
                        ink,
                    );
                    y += row.height() + 2.0;
                }

                // the rows: the section shown, or every section's rows that match the search
                let inner = Rect::from_min_max(body.min + vec2(32.0, 20.0), body.max - vec2(32.0, 0.0));
                ui.scope_builder(
                    UiBuilder::new().max_rect(inner).layout(Layout::top_down(Align::Min)),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt(self.id.with("rows"))
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                let searching = !query.trim().is_empty();
                                let sections: Vec<&SettingsSection> = if searching {
                                    self.sections.iter().collect()
                                } else {
                                    self.sections.iter().filter(|s| s.id == *self.active).take(1).collect()
                                };
                                let sections = if sections.is_empty() {
                                    self.sections.iter().take(1).collect()
                                } else {
                                    sections
                                };
                                let mut shown = 0;
                                for section in sections {
                                    let state = RowsState {
                                        query: query.trim().to_lowercase(),
                                        heading: Some(section.label.clone()),
                                        shown: 0,
                                    };
                                    ui.ctx().data_mut(|d| d.insert_temp(rows_state_id(), state));
                                    rows(&section.id, ui);
                                    shown += ui
                                        .ctx()
                                        .data(|d| d.get_temp::<RowsState>(rows_state_id()))
                                        .map_or(0, |s| s.shown);
                                }
                                ui.ctx().data_mut(|d| d.remove::<RowsState>(rows_state_id()));
                                if shown == 0 && searching {
                                    ui.add_space(8.0);
                                    ui.label(
                                        TextRole::Body
                                            .rich(&t, &self.words.no_match)
                                            .color(t.color("muted-foreground")),
                                    );
                                }
                                ui.add_space(20.0);
                            });
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
                        ui.spacing_mut().item_spacing.x = 8.0;
                        if ui
                            .add(Button::new(self.words.reset.clone()).variant(ButtonVariant::Ghost))
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
        ctx.data_mut(|d| d.insert_temp(query_id, query));
        outcome.close |= response.should_close();
        if outcome.close {
            // opened again later, the search is empty and takes the focus again
            ctx.data_mut(|d| {
                d.remove::<bool>(self.id.with("opened"));
                d.remove::<String>(query_id);
            });
        }
        outcome
    }
}

/// One setting: its title and description at the start, its control at the end, a rule under it.
/// In a settings dialog that is searching, a row whose words do not match draws nothing; the first
/// row of a section that shows draws the section's title over it.
pub fn setting_row(ui: &mut Ui, title: &str, description: &str, control: impl FnOnce(&mut Ui)) {
    let t = tokens(ui.ctx());
    if let Some(mut state) = ui.ctx().data(|d| d.get_temp::<RowsState>(rows_state_id())) {
        let matches = state.query.is_empty() || format!("{title} {description}").to_lowercase().contains(&state.query);
        if !matches {
            return;
        }
        if let Some(heading) = state.heading.take() {
            // the section's title, 28 tall: a detail title and 4 under it
            ui.label(
                TextRole::DetailTitle
                    .rich(&t, heading)
                    .color(t.color("popover-foreground")),
            );
            ui.add_space(4.0);
        }
        state.shown += 1;
        ui.ctx().data_mut(|d| d.insert_temp(rows_state_id(), state));
    }
    Frame::new().inner_margin(Margin::symmetric(0, 16)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        // the control from the end first, then the words in what it leaves, 32 short of it
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            control(ui);
            ui.add_space(32.0);
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(TextRole::Label.rich(&t, title).color(t.color("popover-foreground")));
                ui.add(
                    egui::Label::new(TextRole::Body.rich(&t, description).color(t.color("muted-foreground"))).wrap(),
                );
            });
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
