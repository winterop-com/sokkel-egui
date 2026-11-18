//! The settings every app has, as one dialog over the page: its sections in a column at the
//! start, the chosen section's rows, and a footer that resets the section and closes. After
//! sokkel-ui's `SettingsDialog` and `SettingRow`.

use egui::{Align, Context, CornerRadius, Frame, Id, Layout, Margin, Rect, Sense, Stroke, Ui, UiBuilder, pos2, vec2};

use crate::{
    Appearance, Button, ButtonVariant, Icon, Motion, Scheme, Segmented, SegmentedVariant, Select, TextRole, densities,
    icons, swatch, themes, tokens,
};

/// One section of the settings: an id the app keeps, a label and, when given, an icon in the column.
#[derive(Clone, Debug)]
pub struct SettingsSection {
    pub id: String,
    pub label: String,
    pub icon: Option<Icon>,
    /// The group title over this section and those after it with the same group.
    pub group: Option<String>,
    /// Whether the footer offers Reset to defaults while the section shows.
    pub reset: bool,
    /// The title over its rows, when it says more than its label (`Subscribers (2)`).
    pub heading: Option<String>,
}

impl SettingsSection {
    /// `icon` is an `Icon`, or `None` for a column of labels alone.
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: impl Into<Option<Icon>>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            group: None,
            reset: true,
            heading: None,
        }
    }

    /// The title over the section's rows in place of its label, as the web's section `heading`.
    pub fn heading(mut self, heading: impl Into<String>) -> Self {
        self.heading = Some(heading.into());
        self
    }

    /// Put the section under a group title in the column, as "Preferences" or "This demo".
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    /// False for a section with nothing to put back: the footer leaves out Reset to defaults.
    pub fn reset(mut self, reset: bool) -> Self {
        self.reset = reset;
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
    Id::unique("sokkel-egui/settings-rows")
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
    /// On a phone, the button in a section's head that goes back to the list of sections.
    pub back: String,
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
            back: "Back".into(),
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
    pub fn new(id: impl egui::AsIdSalt, sections: &'a [SettingsSection], active: &'a mut String) -> Self {
        Self {
            id: Id::unique(id),
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
        // 896 wide and 704 tall, at most the window less 32 across and 64 down; on a phone a sheet
        // over the whole window, its sections a list to go into
        let window = ctx.content_rect();
        let phone = super::is_phone(ctx);
        let (width, height) = if phone {
            (window.width(), window.height())
        } else {
            (
                896.0_f32.min(window.width() - 32.0),
                704.0_f32.min(window.height() - 64.0),
            )
        };
        // the footer: 16 over and under Done (a finger tall on a phone), and its rule
        let foot_height = 16.0 + super::finger_tall(ctx, &t, t.density("control")) + 16.0 + 1.0;
        let radius = if phone { CornerRadius::ZERO } else { t.radius("surface") };
        let frame = if phone {
            Frame::new().fill(t.color("popover"))
        } else {
            super::dialog_frame(&t)
        };
        let query_id = self.id.with("search");
        let mut query: String = ctx.data(|d| d.get_temp(query_id)).unwrap_or_default();
        // on a phone, whether a section is open (else the list of sections); it opens on the list
        let pass = ctx.cumulative_pass_nr();
        let seen_id = self.id.with("seen");
        let opening = ctx
            .data(|d| d.get_temp::<u64>(seen_id))
            .is_none_or(|last| last + 1 < pass);
        let in_section_id = self.id.with("in-section");
        let mut in_section = !opening && ctx.data(|d| d.get_temp::<bool>(in_section_id)).unwrap_or(false);
        let response = egui::Modal::new(self.id)
            .backdrop_color(t.color_at("shade", 0.1))
            .frame(frame)
            .show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
                let head = Rect::from_min_size(rect.min, vec2(width, 48.0));
                let foot = Rect::from_min_max(pos2(rect.left(), rect.bottom() - foot_height), rect.max);
                // the column at the start and the rows beside it; on a phone, one or the other across
                let nav_width = if phone { width } else { 224.0 };
                let nav = Rect::from_min_max(
                    pos2(rect.left(), head.bottom()),
                    pos2(rect.left() + nav_width, foot.top()),
                );
                let body = if phone {
                    Rect::from_min_max(pos2(rect.left(), head.bottom()), pos2(rect.right(), foot.top()))
                } else {
                    Rect::from_min_max(pos2(nav.right(), head.bottom()), pos2(rect.right(), foot.top()))
                };
                let show_nav = !phone || !in_section;
                let show_body = !phone || in_section;
                let painter = ui.painter().clone();
                let rule = Stroke::new(1.0, t.color("border"));

                // the head: the title and the close button; on a phone in a section, Back before the
                // title, the head padding 8 at its start
                painter.hline(head.x_range(), head.bottom() - 0.5, rule);
                let control = super::finger_tall(ctx, &t, 28.0);
                let mut title_x = head.left() + 16.0;
                if phone && in_section {
                    let back = Rect::from_min_size(
                        pos2(head.left() + 8.0, head.center().y - control / 2.0),
                        vec2(control, control),
                    );
                    let hit = ui.interact(back, self.id.with("back"), Sense::click());
                    hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.back));
                    if hit.hovered() {
                        painter.rect_filled(back, t.radius("control"), t.color("hover"));
                    }
                    icons::paint(
                        &painter,
                        back.center(),
                        Icon::ChevronLeft,
                        16.0,
                        t.color("popover-foreground"),
                    );
                    super::focus_ring(&painter, &t, back, t.radius("control"), &hit);
                    if hit.clicked() {
                        in_section = false;
                    }
                    title_x = back.right() + 4.0;
                }
                let title = egui::WidgetText::from(TextRole::DetailTitle.rich(&t, &self.words.title)).into_galley(
                    ui,
                    Some(egui::TextWrapMode::Truncate),
                    head.width() - 80.0,
                    TextRole::DetailTitle.font(&t),
                );
                painter.galley(
                    pos2(title_x, head.center().y - title.size().y / 2.0),
                    title,
                    t.color("popover-foreground"),
                );
                let close = Rect::from_min_size(
                    pos2(head.right() - 16.0 - control, head.center().y - control / 2.0),
                    vec2(control, control),
                );
                let close_hit = ui.interact(close, self.id.with("close"), Sense::click());
                close_hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.close));
                if close_hit.hovered() {
                    painter.rect_filled(close, t.radius("control"), t.color("hover"));
                }
                icons::paint(&painter, close.center(), Icon::X, 16.0, t.color("popover-foreground"));
                if close_hit.clicked() {
                    outcome.close = true;
                }

                // the column, on the dialog's own ground: the search, then the sections under
                // their group titles
                if !phone {
                    painter.vline(nav.right() - 0.5, nav.y_range(), rule);
                }
                let field = Rect::from_min_size(
                    pos2(nav.left() + 12.0, nav.top() + 12.0),
                    vec2(nav.width() - 24.0, super::finger_tall(ctx, &t, t.density("control"))),
                );
                if show_nav {
                    let mut child = ui.new_child(UiBuilder::new().max_rect(field));
                    let search = super::search_field(&mut child, &mut query, &self.words.search);
                    // from 768 the search takes the focus as the dialog opens; on a phone nothing
                    // does, so no keyboard rises
                    if opening && !phone {
                        search.request_focus();
                    }
                }
                ctx.data_mut(|d| d.insert_temp(seen_id, pass));
                // on a phone with words in the search, the matching rows show under it in place of
                // the sections
                let phone_search = phone && !in_section && !query.trim().is_empty();
                let mut y = field.bottom();
                let mut group: Option<&String> = None;
                let entries = if show_nav && !phone_search { self.sections } else { &[] };
                for (index, section) in entries.iter().enumerate() {
                    if section.group.is_some() && section.group.as_ref() != group {
                        group = section.group.as_ref();
                        // a group's title: 16 above, its line 20 tall, 4 and the entries' 2 under it
                        y += 16.0;
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
                        y += 26.0;
                    } else if section.group.is_none() {
                        // ungrouped sections: the first keeps the same 12 from the search as a group title
                        y += if index == 0 { 12.0 } else { 2.0 };
                    }
                    // 34 tall, a finger tall on a phone, where none is marked and each has a chevron
                    let entry = if phone { t.length("spacing-finger") } else { 34.0 };
                    let row = Rect::from_min_size(pos2(nav.left() + 12.0, y), vec2(nav.width() - 24.0, entry));
                    let hit = ui.interact(row, self.id.with(("section", &section.id)), Sense::click());
                    let active = !phone && *self.active == section.id && query.trim().is_empty();
                    hit.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, &section.label));
                    if hit.clicked() {
                        *self.active = section.id.clone();
                        query.clear();
                        in_section = true;
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
                    // a label alone stands 8 in, after an icon 34
                    let start = match section.icon {
                        Some(icon) => {
                            icons::paint(&painter, pos2(row.left() + 16.0, row.center().y), icon, 16.0, ink);
                            34.0
                        }
                        None => 8.0,
                    };
                    let role = if active { TextRole::Action } else { TextRole::Body };
                    let galley = egui::WidgetText::from(role.rich(&t, &section.label)).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Truncate),
                        row.width() - start - 10.0,
                        role.font(&t),
                    );
                    painter.galley(
                        pos2(row.left() + start, row.center().y - galley.size().y / 2.0),
                        galley,
                        ink,
                    );
                    if phone {
                        icons::paint(
                            &painter,
                            pos2(row.right() - 16.0, row.center().y),
                            Icon::ChevronRight,
                            16.0,
                            t.color("faint"),
                        );
                    }
                    y += row.height() + 2.0;
                }

                // the rows: the section shown, or every section's rows that match the search; 32 in,
                // 16 on a phone; under the search while a phone searches the list
                let across = if phone { 16.0 } else { 32.0 };
                let inner = if phone_search {
                    Rect::from_min_max(
                        pos2(body.left() + across, field.bottom() + 12.0),
                        body.max - vec2(across, 0.0),
                    )
                } else {
                    Rect::from_min_max(body.min + vec2(across, 20.0), body.max - vec2(across, 0.0))
                };
                if show_body || phone_search {
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
                                            heading: Some(
                                                section.heading.clone().unwrap_or_else(|| section.label.clone()),
                                            ),
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
                }

                // the footer: reset the section, the note, and Done
                painter.rect_filled(foot, CornerRadius { nw: 0, ne: 0, ..radius }, t.color_at("muted", 0.5));
                painter.hline(foot.x_range(), foot.top() + 0.5, rule);
                // Done at the end first, then Reset and the note in what it leaves, the note wrapping
                ui.scope_builder(
                    UiBuilder::new()
                        .max_rect(foot.shrink2(vec2(16.0, 0.0)))
                        .layout(Layout::right_to_left(Align::Center)),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        if ui.add(Button::new(self.words.done.clone())).clicked() {
                            outcome.close = true;
                        }
                        ui.add_space(8.0);
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            // no Reset on a phone's list of sections
                            let resets = query.trim().is_empty()
                                && (!phone || in_section)
                                && self
                                    .sections
                                    .iter()
                                    .find(|s| s.id == *self.active)
                                    .or(self.sections.first())
                                    .is_some_and(|s| s.reset);
                            if resets
                                && ui
                                    .add(Button::new(self.words.reset.clone()).variant(ButtonVariant::Ghost))
                                    .clicked()
                            {
                                outcome.reset = true;
                            }
                            ui.add(
                                egui::Label::new(
                                    TextRole::Help
                                        .rich(&t, &self.words.saved)
                                        .color(t.color("muted-foreground")),
                                )
                                .wrap(),
                            );
                        });
                    },
                );
            });
        ctx.data_mut(|d| {
            d.insert_temp(query_id, query);
            d.insert_temp(in_section_id, in_section);
        });
        outcome.close |= response.should_close();
        if outcome.close {
            // opened again later, the search is empty
            ctx.data_mut(|d| d.remove::<String>(query_id));
        }
        outcome
    }
}

/// One setting: its title and description at the start, its control at the end, a rule under it.
/// In a settings dialog that is searching, a row whose words do not match draws nothing; the first
/// row of a section that shows draws the section's title over it.
pub fn setting_row(ui: &mut Ui, title: &str, description: &str, control: impl FnOnce(&mut Ui)) {
    setting_row_found_by(ui, title, description, &[], control);
}

/// A `setting_row` the settings' search also finds by `keywords`, such as the names of the
/// options its control offers.
pub fn setting_row_found_by(
    ui: &mut Ui,
    title: &str,
    description: &str,
    keywords: &[&str],
    control: impl FnOnce(&mut Ui),
) {
    setting_row_laid(ui, title, description, keywords, false, None, control);
}

/// A setting whose control stands under its words, 12 down at its own width, as sokkel-ui's
/// `SettingRow` with `layout="below"`: for a wide control, such as a choice of several or a field.
pub fn setting_row_below(
    ui: &mut Ui,
    title: &str,
    description: &str,
    keywords: &[&str],
    control: impl FnOnce(&mut Ui),
) {
    setting_row_laid(ui, title, description, keywords, true, None, control);
}

/// One setting with all of sokkel-ui's `SettingRow` options: the words the search also finds it
/// by, the control under the words, and a note across the row under both, for what the setting
/// needs or changes elsewhere.
pub struct SettingRow<'a> {
    title: &'a str,
    description: &'a str,
    keywords: &'a [&'a str],
    below: bool,
    note: Option<String>,
}

impl<'a> SettingRow<'a> {
    pub fn new(title: &'a str, description: &'a str) -> Self {
        Self {
            title,
            description,
            keywords: &[],
            below: false,
            note: None,
        }
    }

    pub fn keywords(mut self, keywords: &'a [&'a str]) -> Self {
        self.keywords = keywords;
        self
    }

    /// The control under the words, as the web's `layout="below"`.
    pub fn below(mut self) -> Self {
        self.below = true;
        self
    }

    /// Small muted words on a `muted` ground under the row, 8 down, as wide as they need.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub fn show(self, ui: &mut Ui, control: impl FnOnce(&mut Ui)) {
        setting_row_laid(
            ui,
            self.title,
            self.description,
            self.keywords,
            self.below,
            self.note.as_deref(),
            control,
        );
    }
}

fn setting_row_laid(
    ui: &mut Ui,
    title: &str,
    description: &str,
    keywords: &[&str],
    below: bool,
    note: Option<&str>,
    control: impl FnOnce(&mut Ui),
) {
    let t = tokens(ui.ctx());
    let rule = |ui: &mut Ui| {
        let (rule, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
        ui.painter()
            .hline(rule.x_range(), rule.center().y, Stroke::new(1.0, t.color("border")));
    };
    // in the settings dialog, a rule between two rows: none over the first or under the last
    let mut ruled_after = true;
    if let Some(mut state) = ui.ctx().data(|d| d.get_temp::<RowsState>(rows_state_id())) {
        let words = format!("{title} {description} {}", keywords.join(" ")).to_lowercase();
        let matches = state.query.is_empty() || words.contains(&state.query);
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
        if state.shown > 0 {
            rule(ui);
        }
        ruled_after = false;
        state.shown += 1;
        ui.ctx().data_mut(|d| d.insert_temp(rows_state_id(), state));
    }
    let note_of = |ui: &mut Ui| {
        if let Some(note) = note {
            ui.add_space(8.0);
            Frame::new()
                .fill(t.color("muted"))
                .corner_radius(t.radius("control"))
                .inner_margin(Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.add(egui::Label::new(TextRole::Help.rich(&t, note).color(t.color("muted-foreground"))).wrap());
                });
        }
    };
    Frame::new().inner_margin(Margin::symmetric(0, 16)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            setting_row_body(ui, &t, title, description, below, control);
            note_of(ui);
        });
    });
    // outside the dialog, the rule under the row, a line of its own
    if ruled_after {
        rule(ui);
    }
}

fn setting_row_body(
    ui: &mut Ui,
    t: &crate::Tokens,
    title: &str,
    description: &str,
    below: bool,
    control: impl FnOnce(&mut Ui),
) {
    {
        if below {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(TextRole::Action.rich(t, title).color(t.color("popover-foreground")));
            ui.add(egui::Label::new(TextRole::Body.rich(t, description).color(t.color("muted-foreground"))).wrap());
            ui.add_space(10.0);
            // one line, as tall as the control
            ui.horizontal(control);
            return;
        }
        // narrower than 512, the control under the words, 12 down, a select across the row
        if ui.available_width() < 512.0 {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(TextRole::Action.rich(t, title).color(t.color("popover-foreground")));
            ui.add(egui::Label::new(TextRole::Body.rich(t, description).color(t.color("muted-foreground"))).wrap());
            ui.add_space(10.0);
            ui.with_layout(
                Layout::left_to_right(Align::Min)
                    .with_main_justify(true)
                    .with_main_align(Align::Min),
                control,
            );
            return;
        }
        // the control from the end first, then the words in what it leaves, 32 short of it
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            control(ui);
            ui.add_space(32.0);
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(TextRole::Action.rich(t, title).color(t.color("popover-foreground")));
                ui.add(egui::Label::new(TextRole::Body.rich(t, description).color(t.color("muted-foreground"))).wrap());
            });
        });
    }
}

/// The words the appearance rows say, English by default, as sokkel-ui's `ThemePickerWords`.
#[derive(Clone, Debug)]
pub struct ThemeWords {
    pub theme: String,
    pub theme_hint: String,
    pub scheme: String,
    pub scheme_hint: String,
    pub scheme_system_hint: String,
    pub dark: String,
    pub light: String,
    pub system: String,
    pub density: String,
    pub density_hint: String,
    pub motion: String,
    pub motion_hint: String,
    pub motion_system: String,
    pub motion_on: String,
    pub motion_off: String,
}

impl Default for ThemeWords {
    fn default() -> Self {
        Self {
            theme: "Theme".into(),
            theme_hint: "The colors of surfaces, lines, text and the accent.".into(),
            scheme: "Scheme".into(),
            scheme_hint: "Dark or light.".into(),
            scheme_system_hint: "Dark, light, or as the device is set.".into(),
            dark: "Dark".into(),
            light: "Light".into(),
            system: "System".into(),
            density: "Density".into(),
            density_hint: "How much fits on a screen.".into(),
            motion: "Motion".into(),
            motion_hint: "Whether parts move as they open and change.".into(),
            motion_system: "System".into(),
            motion_on: "On".into(),
            motion_off: "Off".into(),
        }
    }
}

/// The appearance rows of a settings section, after sokkel-ui's `themeSettings`: the theme in a
/// select with each theme's sample, the scheme (the system's as a third when `system`), the
/// density and the motion, each found in the settings' search by its options' names too.
pub fn theme_settings(ui: &mut Ui, appearance: &mut Appearance, system: bool, words: &ThemeWords) {
    theme_rows(ui, appearance, system, None, false, words);
}

/// The appearance rows offering only the `carried` themes (by name, in the catalog's order) in
/// the select, as `themeSettings` given the app's `themes`.
pub fn theme_settings_of(ui: &mut Ui, appearance: &mut Appearance, carried: &[&str], system: bool, words: &ThemeWords) {
    theme_rows(ui, appearance, system, Some(carried), false, words);
}

/// The appearance rows with the theme as a list under its words, as `themeSettings` with
/// `themeList`: each of the `carried` themes (by name, in the catalog's order) a row with its sample, its
/// name and what it looks like, the chosen one marked.
pub fn theme_settings_listed(
    ui: &mut Ui,
    appearance: &mut Appearance,
    carried: &[&str],
    system: bool,
    words: &ThemeWords,
) {
    theme_rows(ui, appearance, system, Some(carried), true, words);
}

fn theme_rows(
    ui: &mut Ui,
    appearance: &mut Appearance,
    system: bool,
    carried: Option<&[&str]>,
    listed: bool,
    words: &ThemeWords,
) {
    let all: Vec<_> = match carried {
        Some(names) => themes().filter(|theme| names.contains(&theme.name.as_str())).collect(),
        None => themes().collect(),
    };
    let labels: Vec<&str> = all.iter().map(|theme| theme.label.as_str()).collect();
    if listed {
        SettingRow::new(&words.theme, &words.theme_hint)
            .keywords(&labels)
            .below()
            .show(ui, |ui| {
                let dark = tokens(ui.ctx()).dark;
                let choices: Vec<super::Choice> = all
                    .iter()
                    .map(|theme| super::Choice {
                        value: &theme.name,
                        label: &theme.label,
                        description: theme.description.as_deref(),
                    })
                    .collect();
                let mut value = appearance.theme.clone();
                let list = super::ChoiceList::new(&words.theme, &choices)
                    .swatches(all.iter().map(|theme| swatch(&theme.name, dark)));
                if list.show(ui, &mut value) {
                    appearance.theme = value;
                }
            });
    }
    if !listed {
        setting_row_found_by(ui, &words.theme, &words.theme_hint, &labels, |ui| {
            let dark = tokens(ui.ctx()).dark;
            let mut chosen = all.iter().position(|theme| theme.name == appearance.theme).unwrap_or(0);
            let select = Select::new(&mut chosen, labels.iter().copied(), words.theme.clone())
                .width(224.0)
                .swatches(all.iter().map(|theme| swatch(&theme.name, dark)));
            if ui.add(select).changed() {
                appearance.theme = all[chosen].name.clone();
            }
        });
    }
    let mut schemes = vec![
        (Scheme::Dark, words.dark.as_str()),
        (Scheme::Light, words.light.as_str()),
    ];
    if system {
        schemes.push((Scheme::System, words.system.as_str()));
    }
    let names: Vec<&str> = schemes.iter().map(|(_, name)| *name).collect();
    let hint = if system {
        &words.scheme_system_hint
    } else {
        &words.scheme_hint
    };
    setting_row_found_by(ui, &words.scheme, hint, &names, |ui| {
        let mut chosen = schemes.iter().position(|(s, _)| *s == appearance.scheme).unwrap_or(0);
        if ui
            .add(Segmented::new(&mut chosen, names.iter().copied()).variant(SegmentedVariant::Track))
            .changed()
        {
            appearance.scheme = schemes[chosen].0;
        }
    });
    let levels = densities();
    let level_names: Vec<String> = levels
        .iter()
        .map(|level| {
            let mut label = level.to_string();
            label[..1].make_ascii_uppercase();
            label
        })
        .collect();
    let level_words: Vec<&str> = level_names.iter().map(String::as_str).collect();
    setting_row_found_by(ui, &words.density, &words.density_hint, &level_words, |ui| {
        let mut chosen = levels
            .iter()
            .position(|level| *level == appearance.density)
            .unwrap_or(0);
        if ui
            .add(Segmented::new(&mut chosen, level_words.iter().copied()).variant(SegmentedVariant::Track))
            .changed()
        {
            appearance.density = levels[chosen].to_owned();
        }
    });
    let motions = [
        (Motion::System, words.motion_system.as_str()),
        (Motion::On, words.motion_on.as_str()),
        (Motion::Off, words.motion_off.as_str()),
    ];
    setting_row_found_by(ui, &words.motion, &words.motion_hint, &["animation"], |ui| {
        let mut chosen = motions.iter().position(|(m, _)| *m == appearance.motion).unwrap_or(0);
        if ui
            .add(Segmented::new(&mut chosen, motions.iter().map(|(_, name)| *name)).variant(SegmentedVariant::Track))
            .changed()
        {
            appearance.motion = motions[chosen].0;
        }
    });
}
