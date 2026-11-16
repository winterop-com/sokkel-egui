//! The range a dashboard covers and how it refreshes, after sokkel-ui's `TimeRangePicker` and
//! `RefreshControl` (`dashboard/TimeRange`).

use egui::{Popup, PopupCloseBehavior, Response, Sense, Stroke, StrokeKind, Ui, vec2};

use crate::{Button, ButtonSize, ButtonVariant, Icon, TextRole, icons, tokens};

/// The range a dashboard covers, as a trigger naming it (a calendar's glyph, the range, a chevron)
/// opening the list of presets; a preset chosen applies at once, as the web's `TimeRangePicker`.
/// `chosen` is the preset's id; the response says when it changed.
pub struct TimeRangePicker<'a> {
    chosen: &'a mut String,
    presets: &'a [(&'a str, &'a str)],
}

impl<'a> TimeRangePicker<'a> {
    /// `presets` are each an id and its label, such as `("15m", "Last 15 minutes")`.
    pub fn new(chosen: &'a mut String, presets: &'a [(&'a str, &'a str)]) -> Self {
        Self { chosen, presets }
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let label = self
            .presets
            .iter()
            .find(|(id, _)| *id == self.chosen.as_str())
            .map_or(self.chosen.clone(), |(_, label)| (*label).to_owned());
        let mut response = ui.add(
            Button::new(label)
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Sm)
                .icon(Icon::CalendarRange)
                .trailing(Icon::ChevronDown),
        );
        let mut picked = None;
        Popup::menu(&response)
            .frame(super::popup_frame(&t).inner_margin(egui::Margin::same(6)))
            .align(egui::RectAlign::BOTTOM_END)
            .gap(4.0)
            .width(160.0)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                for (id, label) in self.presets {
                    let chosen = *id == self.chosen.as_str();
                    let (row, hit) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
                    hit.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, chosen, *label));
                    let ground = if chosen {
                        Some(t.color("secondary"))
                    } else if hit.hovered() {
                        Some(t.color("hover"))
                    } else {
                        None
                    };
                    if let Some(ground) = ground {
                        ui.painter().rect_filled(row, t.radius("md"), ground);
                    }
                    let rich = TextRole::Body.rich(&t, *label);
                    let rich = if chosen { rich.variation("wght", 500.0) } else { rich };
                    let galley = egui::WidgetText::from(rich).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Truncate),
                        row.width() - 16.0,
                        TextRole::Body.font(&t),
                    );
                    ui.painter().galley(
                        egui::pos2(row.left() + 8.0, row.center().y - galley.size().y / 2.0),
                        galley,
                        t.color("popover-foreground"),
                    );
                    if hit.clicked() {
                        picked = Some((*id).to_owned());
                        ui.close();
                    }
                }
            });
        if let Some(id) = picked
            && id != *self.chosen
        {
            *self.chosen = id;
            response.mark_changed();
        }
        response
    }
}

/// The words a refresh control says.
#[derive(Clone, Debug)]
pub struct RefreshWords {
    pub refresh: String,
    pub every: String,
    pub off: String,
    pub pause: String,
    pub resume: String,
}

impl Default for RefreshWords {
    fn default() -> Self {
        Self {
            refresh: "Refresh".into(),
            every: "Refresh every".into(),
            off: "Off".into(),
            pause: "Pause".into(),
            resume: "Resume".into(),
        }
    }
}

/// What a refresh control was asked this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct RefreshAsked {
    /// Refresh now was pressed.
    pub refresh: bool,
    /// The interval or the pause changed.
    pub changed: bool,
}

/// Refreshing a dashboard, as the web's `RefreshControl`: when it last did, then in one bordered
/// group a button that refreshes now, a pause while an interval is set, and a menu of how often it
/// refreshes by itself (`0` is off).
pub struct RefreshControl<'a> {
    interval: &'a mut u32,
    paused: &'a mut bool,
    intervals: &'a [u32],
    updated: Option<String>,
    words: RefreshWords,
}

impl<'a> RefreshControl<'a> {
    pub fn new(interval: &'a mut u32, paused: &'a mut bool, intervals: &'a [u32]) -> Self {
        Self {
            interval,
            paused,
            intervals,
            updated: None,
            words: RefreshWords::default(),
        }
    }

    /// When it last refreshed, already written for a reader: `updated 09:00:05`.
    pub fn updated(mut self, updated: impl Into<String>) -> Self {
        self.updated = Some(updated.into());
        self
    }

    pub fn words(mut self, words: RefreshWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> RefreshAsked {
        let t = tokens(ui.ctx());
        let mut asked = RefreshAsked::default();
        let written = |seconds: u32| {
            if seconds == 0 {
                self.words.off.clone()
            } else if seconds < 60 {
                format!("{seconds}s")
            } else {
                format!("{}m", seconds / 60)
            }
        };
        let side = t.density("control-sm");
        let pausable = *self.interval > 0;
        let menu_label = written(*self.interval);
        let menu_galley =
            ui.painter()
                .layout_no_wrap(menu_label.clone(), TextRole::Badge.font(&t), t.color("foreground"));
        let menu_width = 8.0 + menu_galley.size().x + 4.0 + 14.0 + 8.0;
        // an icon alone in a small button: 10 either side of a 16 glyph's room, as `size="sm"`
        let button = 10.0 + 16.0 + 10.0;
        let width = button + if pausable { button } else { 0.0 } + menu_width;
        let updated_width = self.updated.as_ref().map_or(0.0, |u| {
            ui.painter()
                .layout_no_wrap(u.clone(), TextRole::Help.font(&t), t.color("muted-foreground"))
                .size()
                .x
                + 8.0
        });
        // left to right in a row of its own, whichever way the row around it runs
        ui.allocate_ui_with_layout(
            vec2(updated_width + width, side),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if let Some(updated) = &self.updated {
                    ui.label(TextRole::Help.rich(&t, updated).color(t.color("muted-foreground")));
                }
                let (group, _) = ui.allocate_exact_size(vec2(width, side), Sense::hover());
                let painter = ui.painter().clone();
                let radius = t.radius("control");
                let mut x = group.left();
                let mut part = |ui: &mut Ui, w: f32, label: &str, id: &str| {
                    let rect = egui::Rect::from_min_size(egui::pos2(x, group.top()), vec2(w, side));
                    x += w;
                    let hit = ui.interact(rect, ui.id().with(("refresh", id)), Sense::click());
                    hit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
                    if hit.hovered() {
                        painter.rect_filled(rect.shrink(1.0), radius, t.color("hover"));
                    }
                    (rect, hit)
                };
                let (rect, hit) = part(ui, button, &self.words.refresh, "now");
                icons::paint(&painter, rect.center(), Icon::RefreshCw, 14.0, t.color("foreground"));
                asked.refresh = hit.on_hover_text(&self.words.refresh).clicked();
                if pausable {
                    let label = if *self.paused {
                        &self.words.resume
                    } else {
                        &self.words.pause
                    };
                    let (rect, hit) = part(ui, button, label, "pause");
                    painter.vline(rect.left() + 0.5, rect.y_range(), Stroke::new(1.0, t.color("border")));
                    let icon = if *self.paused { Icon::Play } else { Icon::Pause };
                    icons::paint(&painter, rect.center(), icon, 14.0, t.color("foreground"));
                    if hit.on_hover_text(label.as_str()).clicked() {
                        *self.paused = !*self.paused;
                        asked.changed = true;
                    }
                }
                let (rect, hit) = part(ui, menu_width, &self.words.every, "every");
                painter.vline(rect.left() + 0.5, rect.y_range(), Stroke::new(1.0, t.color("border")));
                let ink = if pausable && *self.paused {
                    t.color("muted-foreground")
                } else {
                    t.color("foreground")
                };
                painter.galley(
                    egui::pos2(rect.left() + 8.0, rect.center().y - menu_galley.size().y / 2.0),
                    menu_galley,
                    ink,
                );
                icons::paint(
                    &painter,
                    egui::pos2(rect.right() - 8.0 - 7.0, rect.center().y),
                    Icon::ChevronDown,
                    14.0,
                    t.color("muted-foreground"),
                );
                painter.rect_stroke(group, radius, Stroke::new(1.0, t.color("border")), StrokeKind::Inside);
                let mut picked = None;
                Popup::menu(&hit)
                    .frame(super::popup_frame(&t).inner_margin(egui::Margin::same(4)))
                    .align(egui::RectAlign::BOTTOM_END)
                    .gap(4.0)
                    .width(144.0)
                    .show(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for seconds in self.intervals {
                            let chosen = *seconds == *self.interval;
                            let words = written(*seconds);
                            let (row, hit) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
                            hit.widget_info(|| {
                                egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, chosen, &words)
                            });
                            if hit.hovered() {
                                ui.painter().rect_filled(row, t.radius("md"), t.color("accent"));
                            }
                            if chosen {
                                icons::paint(
                                    ui.painter(),
                                    egui::pos2(row.left() + 14.0, row.center().y),
                                    Icon::Check,
                                    16.0,
                                    t.color("popover-foreground"),
                                );
                            }
                            ui.painter().text(
                                egui::pos2(row.left() + 28.0, row.center().y),
                                egui::Align2::LEFT_CENTER,
                                &words,
                                TextRole::Body.font(&t),
                                t.color("popover-foreground"),
                            );
                            if hit.clicked() {
                                picked = Some(*seconds);
                                ui.close();
                            }
                        }
                    });
                if let Some(seconds) = picked
                    && seconds != *self.interval
                {
                    *self.interval = seconds;
                    asked.changed = true;
                }
            },
        );
        asked
    }
}
