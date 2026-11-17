//! A date and a time typed as one, after sokkel-ui's `DateTimeField`.

use egui::{Id, Rect, Response, Sense, Ui, vec2};

use crate::{Calendar, Day, Icon, Input, TextRole, icons, tokens};

/// A moment on the wall clock with no zone: a day and the minutes into it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Moment {
    pub day: Day,
    pub minutes: u32,
}

impl Moment {
    pub fn new(day: Day, hour: u32, minute: u32) -> Self {
        Self {
            day,
            minutes: hour * 60 + minute,
        }
    }

    /// As the field writes it, and reads it back: `2026-11-17 02:00`.
    pub fn written(self) -> String {
        format!("{} {:02}:{:02}", self.day, self.minutes / 60, self.minutes % 60)
    }

    /// The moment `text` writes, or `None` where it is not a date and a time.
    pub fn read(text: &str) -> Option<Self> {
        let (day, time) = text.trim().split_once(' ')?;
        let day = Day::parse(day.trim())?;
        let (hour, minute) = time.trim().split_once(':')?;
        let (hour, minute) = (hour.parse::<u32>().ok()?, minute.parse::<u32>().ok()?);
        (hour < 24 && minute < 60).then(|| Self::new(day, hour, minute))
    }
}

/// The words a date and time field says.
#[derive(Clone, Debug)]
pub struct DateTimeFieldWords {
    pub unreadable: String,
    /// `{}` a moment written as the field writes it.
    pub example: String,
    /// `{}` the earliest moment.
    pub under: String,
    pub pick: String,
    pub times: String,
}

impl Default for DateTimeFieldWords {
    fn default() -> Self {
        Self {
            unreadable: "Enter a date and time".into(),
            example: "As in {}.".into(),
            under: "Enter {} or later".into(),
            pick: "Choose a date and time".into(),
            times: "Times".into(),
        }
    }
}

/// What a date and time field was asked this frame.
pub struct DateTimeFieldResponse {
    pub response: Response,
    /// Its moment changed, by typing or picking.
    pub changed: bool,
    /// What is wrong with what is typed, as the field says it under itself; `None` when it is right.
    pub issue: Option<String>,
}

/// A date and a time typed as one, `YYYY-MM-DD HH:MM`, with a button at its end opening a calendar
/// and, beside it, the day's times every `step` minutes. Text that is not a date and a time keeps
/// the moment as it was and is said under the field; a moment before `min` is kept and said too.
/// Picking a day keeps the time and leaves the picker open; picking a time closes it.
pub struct DateTimeField<'a> {
    id: Id,
    label: String,
    value: &'a mut Option<Moment>,
    min: Option<Moment>,
    today: Day,
    step: u32,
    words: DateTimeFieldWords,
}

#[derive(Clone, Default)]
struct Held {
    text: String,
    /// The moment the text was last written from, so a change from outside writes it again.
    from: Option<Moment>,
    open: bool,
}

impl<'a> DateTimeField<'a> {
    pub fn new(id: impl egui::AsIdSalt, label: impl Into<String>, value: &'a mut Option<Moment>, today: Day) -> Self {
        Self {
            id: Id::unique(id),
            label: label.into(),
            value,
            min: None,
            today,
            step: 15,
            words: DateTimeFieldWords::default(),
        }
    }

    /// The earliest moment it takes.
    pub fn min(mut self, min: Moment) -> Self {
        self.min = Some(min);
        self
    }

    /// Minutes between the times listed beside the calendar; 15 by default.
    pub fn step(mut self, step: u32) -> Self {
        self.step = step.max(1);
        self
    }

    pub fn words(mut self, words: DateTimeFieldWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> DateTimeFieldResponse {
        let t = tokens(ui.ctx());
        let mut held: Held = ui.data(|d| d.get_temp(self.id)).unwrap_or_default();
        if held.from != *self.value || (held.text.is_empty() && self.value.is_some()) {
            held.text = self.value.map(Moment::written).unwrap_or_default();
            held.from = *self.value;
        }
        let mut changed = false;
        let width = ui.available_width();
        let inner = ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.label(TextRole::Label.rich(&t, &self.label).color(t.color("foreground")));
            let typed_before = held.text.clone();
            let read = Moment::read(&held.text);
            let issue = match (read, self.min) {
                (None, _) if !held.text.trim().is_empty() => Some(format!(
                    "{} {}",
                    self.words.unreadable,
                    self.words.example.replace(
                        "{}",
                        &Moment::new(self.min.map_or(self.today, |m| m.day), 14, 30).written()
                    )
                )),
                (Some(at), Some(min)) if at < min => Some(self.words.under.replace("{}", &min.written())),
                _ => None,
            };
            let field = ui.add(
                Input::new(&mut held.text)
                    .name(&self.label)
                    .width(width)
                    .invalid(issue.is_some()),
            );
            // the picker's button inside the field's end, 28 square
            let button = Rect::from_center_size(
                egui::pos2(field.rect.right() - 4.0 - 14.0, field.rect.center().y),
                vec2(28.0, 28.0),
            );
            let hit = ui.interact(button, self.id.with("pick"), Sense::click());
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.pick));
            if hit.hovered() {
                ui.painter().rect_filled(button, t.radius("md"), t.color("hover"));
            }
            icons::paint(
                ui.painter(),
                button.center(),
                Icon::CalendarClock,
                14.0,
                t.color("muted-foreground"),
            );
            super::focus_ring(ui.painter(), &t, button, t.radius("md"), &hit);
            if hit.clicked() {
                held.open = !held.open;
            }
            if held.text != typed_before
                && let Some(at) = Moment::read(&held.text)
            {
                *self.value = Some(at);
                held.from = Some(at);
                changed = true;
            }
            if let Some(issue) = &issue {
                ui.label(TextRole::Help.rich(&t, issue).color(t.color("critical-ink")));
            }
            if held.open {
                let picked = self.picker(ui, field.rect, &mut held);
                if let Some(at) = picked {
                    *self.value = Some(at);
                    held.text = at.written();
                    held.from = Some(at);
                    changed = true;
                }
            }
            (field, issue)
        });
        ui.data_mut(|d| d.insert_temp(self.id, held));
        let (response, issue) = inner.inner;
        DateTimeFieldResponse {
            response,
            changed,
            issue,
        }
    }

    /// The picker under the field: the calendar, and beside it the times; a moment picked.
    fn picker(&self, ui: &Ui, field: Rect, held: &mut Held) -> Option<Moment> {
        let t = tokens(ui.ctx());
        let mut picked = None;
        let current = *self.value;
        let area = egui::Area::new(self.id.with("picker"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(field.left(), field.bottom() + 4.0))
            .show(ui.ctx(), |ui| {
                super::popup_frame(&t).show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        let mut day = current.map(|m| m.day);
                        let before = day;
                        ui.add(Calendar::new(self.id.with("calendar"), &mut day, self.today));
                        if day != before
                            && let Some(day) = day
                        {
                            // a day picked keeps the time, midnight where there is none
                            picked = Some(Moment {
                                day,
                                minutes: current.map_or(0, |m| m.minutes),
                            });
                        }
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .id_salt(self.id.with("times"))
                            .max_height(Calendar::size().y)
                            .show(ui, |ui| {
                                // the times one under another, whatever the row around the list lays out
                                ui.vertical(|ui| {
                                    ui.set_width(112.0 - 16.0);
                                    ui.spacing_mut().item_spacing.y = 0.0;
                                    let row = t.density("control");
                                    let day = picked.map(|m| m.day).or(current.map(|m| m.day)).unwrap_or(self.today);
                                    let mut minutes = 0;
                                    while minutes < 24 * 60 {
                                        let at = Moment { day, minutes };
                                        let chosen = current == Some(at);
                                        let shut = self.min.is_some_and(|min| at < min);
                                        let (rect, hit) = ui.allocate_exact_size(
                                            vec2(ui.available_width(), row),
                                            if shut { Sense::hover() } else { Sense::click() },
                                        );
                                        let words = format!("{:02}:{:02}", minutes / 60, minutes % 60);
                                        hit.widget_info(|| {
                                            egui::WidgetInfo::selected(egui::Role::Button, !shut, chosen, &words)
                                        });
                                        if chosen {
                                            ui.painter().rect_filled(rect, t.radius("md"), t.color("primary"));
                                        } else if hit.hovered() && !shut {
                                            ui.painter().rect_filled(rect, t.radius("md"), t.color("accent"));
                                        }
                                        let ink = if chosen {
                                            t.color("primary-foreground")
                                        } else if shut {
                                            t.color("faint")
                                        } else {
                                            t.color("popover-foreground")
                                        };
                                        ui.painter().text(
                                            rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            &words,
                                            TextRole::Body.font(&t),
                                            ink,
                                        );
                                        if chosen && held.open {
                                            hit.scroll_to_me(Some(egui::Align::Center));
                                        }
                                        if hit.clicked() {
                                            picked = Some(at);
                                            held.open = false;
                                        }
                                        minutes += self.step;
                                    }
                                });
                            });
                    });
                });
            });
        // a press outside the picker and the field closes it
        if ui.input(|i| i.pointer.any_pressed())
            && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
            && !area.response.rect.contains(pos)
            && !field.contains(pos)
        {
            held.open = false;
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            held.open = false;
        }
        picked
    }
}

/// The words a date field says.
#[derive(Clone, Debug)]
pub struct DateFieldWords {
    pub unreadable: String,
    /// `{}` a day written as the field writes it.
    pub example: String,
    pub pick: String,
}

impl Default for DateFieldWords {
    fn default() -> Self {
        Self {
            unreadable: "Enter a date".into(),
            example: "As in {}.".into(),
            pick: "Choose a date".into(),
        }
    }
}

/// A day typed as `YYYY-MM-DD`, with a button at its end opening a calendar to pick from, as
/// sokkel-ui's `DateField`. Text that is not a date keeps the day as it was and is said under the
/// field; an app's own refusal (`error`) is said there too. Picking a day closes the calendar.
pub struct DateField<'a> {
    id: Id,
    label: String,
    value: &'a mut Option<Day>,
    today: Day,
    error: Option<String>,
    words: DateFieldWords,
}

#[derive(Clone, Default)]
struct HeldDay {
    text: String,
    from: Option<Day>,
    open: bool,
}

impl<'a> DateField<'a> {
    pub fn new(id: impl egui::AsIdSalt, label: impl Into<String>, value: &'a mut Option<Day>, today: Day) -> Self {
        Self {
            id: Id::unique(id),
            label: label.into(),
            value,
            today,
            error: None,
            words: DateFieldWords::default(),
        }
    }

    /// What is wrong with the day, as the app says it under the field.
    pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    pub fn words(mut self, words: DateFieldWords) -> Self {
        self.words = words;
        self
    }

    /// Draws the field; true when its day changed, by typing or picking.
    pub fn show(self, ui: &mut Ui) -> bool {
        let t = tokens(ui.ctx());
        let mut held: HeldDay = ui.data(|d| d.get_temp(self.id)).unwrap_or_default();
        if held.from != *self.value || (held.text.is_empty() && self.value.is_some()) {
            held.text = self.value.map(|d| d.to_string()).unwrap_or_default();
            held.from = *self.value;
        }
        let mut changed = false;
        let width = ui.available_width();
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.label(TextRole::Label.rich(&t, &self.label).color(t.color("foreground")));
            let typed_before = held.text.clone();
            let unreadable = !held.text.trim().is_empty() && Day::parse(held.text.trim()).is_none();
            let issue = if unreadable {
                Some(format!(
                    "{} {}",
                    self.words.unreadable,
                    self.words.example.replace("{}", &self.today.to_string())
                ))
            } else {
                self.error.clone()
            };
            let field = ui.add(
                Input::new(&mut held.text)
                    .name(&self.label)
                    .placeholder("yyyy-mm-dd")
                    .width(width)
                    .invalid(issue.is_some()),
            );
            let button = Rect::from_center_size(
                egui::pos2(field.rect.right() - 4.0 - 14.0, field.rect.center().y),
                vec2(28.0, 28.0),
            );
            let hit = ui.interact(button, self.id.with("pick"), Sense::click());
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.pick));
            if hit.hovered() {
                ui.painter().rect_filled(button, t.radius("md"), t.color("hover"));
            }
            icons::paint(
                ui.painter(),
                button.center(),
                Icon::CalendarDays,
                14.0,
                t.color("muted-foreground"),
            );
            super::focus_ring(ui.painter(), &t, button, t.radius("md"), &hit);
            if hit.clicked() {
                held.open = !held.open;
            }
            if held.text != typed_before
                && let Some(day) = Day::parse(held.text.trim())
            {
                *self.value = Some(day);
                held.from = Some(day);
                changed = true;
            }
            if let Some(issue) = &issue {
                ui.label(TextRole::Help.rich(&t, issue).color(t.color("critical-ink")));
            }
            if held.open {
                let mut day = *self.value;
                let area = egui::Area::new(self.id.with("picker"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(egui::pos2(field.rect.left(), field.rect.bottom() + 4.0))
                    .show(ui.ctx(), |ui| {
                        super::popup_frame(&t).show(ui, |ui| {
                            ui.add(Calendar::new(self.id.with("calendar"), &mut day, self.today));
                        });
                    });
                if day != *self.value
                    && let Some(day) = day
                {
                    *self.value = Some(day);
                    held.text = day.to_string();
                    held.from = Some(day);
                    held.open = false;
                    changed = true;
                }
                let outside = ui.input(|i| i.pointer.any_pressed())
                    && ui
                        .input(|i| i.pointer.interact_pos())
                        .is_some_and(|pos| !area.response.rect.contains(pos) && !field.rect.contains(pos));
                if outside || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    held.open = false;
                }
            }
        });
        ui.data_mut(|d| d.insert_temp(self.id, held));
        changed
    }
}
