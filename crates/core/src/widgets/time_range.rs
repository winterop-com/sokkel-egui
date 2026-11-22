//! The range a dashboard covers and how it refreshes, after sokkel-ui's `TimeRangePicker` and
//! `RefreshControl` (`dashboard/TimeRange`).

use egui::{Popup, PopupCloseBehavior, Response, Sense, Stroke, StrokeKind, Ui, vec2};

use crate::{Button, ButtonSize, ButtonVariant, Day, Icon, RangeCalendar, TextRole, icons, tokens};

/// The words a time range picker says.
#[derive(Clone, Debug)]
pub struct TimeRangeWords {
    pub custom: String,
    pub apply: String,
}

impl Default for TimeRangeWords {
    fn default() -> Self {
        Self {
            custom: "Custom range".into(),
            apply: "Apply".into(),
        }
    }
}

/// A span of days said for a reader, as the web's `rangeLabel`: `Nov 3 to Nov 17, 2026`.
pub fn range_label(from: Day, to: Day) -> String {
    format!("{} to {}", from.short(), to.medium())
}

/// The range a dashboard covers, as a trigger naming it (a calendar's glyph, the range, a chevron)
/// opening the list of presets; a preset chosen applies at once, as the web's `TimeRangePicker`.
/// `chosen` is the preset's id; the response says when it changed.
pub struct TimeRangePicker<'a> {
    chosen: &'a mut String,
    presets: &'a [(&'a str, &'a str)],
    custom: Option<(&'a mut Option<(Day, Day)>, Day)>,
    words: TimeRangeWords,
}

impl<'a> TimeRangePicker<'a> {
    /// `presets` are each an id and its label, such as `("15m", "Last 15 minutes")`.
    pub fn new(chosen: &'a mut String, presets: &'a [(&'a str, &'a str)]) -> Self {
        Self {
            chosen,
            presets,
            custom: None,
            words: TimeRangeWords::default(),
        }
    }

    /// A custom span of days beside the presets, picked on two months and applied with Apply, as
    /// the web's: `span` holds it, `chosen` empty while it is the range; `today` is marked.
    pub fn custom(mut self, span: &'a mut Option<(Day, Day)>, today: Day) -> Self {
        self.custom = Some((span, today));
        self
    }

    pub fn words(mut self, words: TimeRangeWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let span = self.custom.as_ref().and_then(|(span, _)| **span);
        let label = match (self.chosen.is_empty(), span) {
            (true, Some((from, to))) => range_label(from, to),
            _ => self
                .presets
                .iter()
                .find(|(id, _)| *id == self.chosen.as_str())
                .map_or(self.chosen.clone(), |(_, label)| (*label).to_owned()),
        };
        let mut response = ui.add(
            Button::new(label)
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Sm)
                .icon(Icon::CalendarRange)
                .trailing(Icon::ChevronDown),
        );
        let mut picked = None;
        let mut applied = None;
        let draft_id = response.id.with("draft");
        let custom = self.custom.as_ref().map(|(span, today)| (**span, *today));
        let presets = self.presets;
        let chosen_now = self.chosen.clone();
        let words = self.words.clone();
        Popup::menu(&response)
            .frame(super::popup_frame(&t).inner_margin(egui::Margin::ZERO))
            .align(egui::RectAlign::BOTTOM_END)
            .gap(4.0)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                ui.horizontal_top(|ui| {
                    // the presets in a column of 160, a rule after it
                    let column = ui.vertical(|ui| {
                        ui.set_width(160.0);
                        egui::Frame::new().inner_margin(egui::Margin::same(6)).show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            if let Some(id) = preset_rows(ui, &t, presets, &chosen_now) {
                                picked = Some(id);
                                ui.close();
                            }
                        });
                    });
                    let Some((span, today)) = custom else {
                        return;
                    };
                    let rule = column.response.rect;
                    ui.painter()
                        .vline(rule.right(), rule.y_range(), Stroke::new(1.0, t.color("border")));
                    ui.vertical(|ui| {
                        let mut draft: (Option<Day>, Option<Day>) = ui
                            .data(|d| d.get_temp(draft_id))
                            .unwrap_or(span.map_or((None, None), |(from, to)| (Some(from), Some(to))));
                        egui::Frame::new()
                            .inner_margin(egui::Margin {
                                left: 12,
                                right: 12,
                                top: 10,
                                bottom: 0,
                            })
                            .show(ui, |ui| {
                                ui.label(
                                    TextRole::Help
                                        .rich(&t, &words.custom)
                                        .variation("wght", t.weight(crate::Weight::Medium))
                                        .color(t.color("muted-foreground")),
                                );
                            });
                        ui.add(RangeCalendar::new(
                            draft_id.with("calendar"),
                            &mut draft.0,
                            &mut draft.1,
                            today,
                        ));
                        // the footer: a rule, then Apply at the end once both days are chosen
                        let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                        ui.painter()
                            .hline(line.x_range(), line.center().y, Stroke::new(1.0, t.color("border")));
                        egui::Frame::new().inner_margin(egui::Margin::same(8)).show(ui, |ui| {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let ready = draft.0.is_some() && draft.1.is_some();
                                let apply = Button::new(&words.apply).size(ButtonSize::Sm).enabled(ready);
                                if ui.add(apply).clicked()
                                    && let (Some(from), Some(to)) = draft
                                {
                                    applied = Some((from, to));
                                    ui.close();
                                }
                            });
                        });
                        ui.data_mut(|d| d.insert_temp(draft_id, draft));
                    });
                });
            });
        if let Some(span) = applied
            && let Some((held, _)) = self.custom
        {
            *held = Some(span);
            self.chosen.clear();
            ui.data_mut(|d| d.remove::<(Option<Day>, Option<Day>)>(draft_id));
            response.mark_changed();
        }
        if let Some(id) = picked
            && id != *self.chosen
        {
            *self.chosen = id;
            response.mark_changed();
        }
        response
    }
}

/// The presets as rows 32 tall, the chosen one on the secondary ground; the one pressed.
fn preset_rows(ui: &mut Ui, t: &crate::Tokens, presets: &[(&str, &str)], chosen: &str) -> Option<String> {
    let mut picked = None;
    for (id, label) in presets {
        let is_chosen = *id == chosen;
        let (row, hit) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
        hit.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, is_chosen, *label));
        let ground = if is_chosen {
            Some(t.color("secondary"))
        } else if hit.hovered() {
            Some(t.color("hover"))
        } else {
            None
        };
        if let Some(ground) = ground {
            ui.painter().rect_filled(row, t.radius("md"), ground);
        }
        let rich = TextRole::Body.rich(t, *label);
        let rich = if is_chosen {
            rich.variation("wght", t.weight(crate::Weight::Medium))
        } else {
            rich
        };
        let galley = egui::WidgetText::from(rich).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            row.width() - 16.0,
            TextRole::Body.font(t),
        );
        ui.painter().galley(
            egui::pos2(row.left() + 8.0, row.center().y - galley.size().y / 2.0),
            galley,
            t.color("popover-foreground"),
        );
        if hit.clicked() {
            picked = Some((*id).to_owned());
        }
    }
    picked
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
                    let hit = ui.interact(rect, ui.scope_id().with(("refresh", id)), Sense::click());
                    hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
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
                                egui::WidgetInfo::selected(egui::Role::RadioButton, true, chosen, &words)
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
