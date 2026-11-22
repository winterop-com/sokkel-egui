//! Fields a form asks with: a value worked out from the others, after sokkel-ui's
//! `ComputedField`, and a yes or no, after its `YesNoField`.

use egui::{Ui, WidgetText};

use crate::{Button, ButtonSize, ButtonVariant, Field, Icon, Input, Segmented, TextRole, icons, tokens};

/// The words a worked-out field says.
#[derive(Clone, Debug)]
pub struct ComputedFieldWords {
    pub pending: String,
    pub computed: String,
}

impl Default for ComputedFieldWords {
    fn default() -> Self {
        Self {
            pending: "Worked out once the answers it reads are given".into(),
            computed: "worked out".into(),
        }
    }
}

/// A value worked out from the others: "worked out" after its label, the value with its unit in a
/// read-only input on the muted ground, and under it why, with a calculator before it and the
/// formula in the code face.
pub struct ComputedField {
    label: String,
    value: Option<String>,
    unit: Option<String>,
    why: String,
    formula: Option<String>,
    description: Option<String>,
    words: ComputedFieldWords,
}

impl ComputedField {
    pub fn new(label: impl Into<String>, value: Option<String>, why: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value,
            unit: None,
            why: why.into(),
            formula: None,
            description: None,
            words: ComputedFieldWords::default(),
        }
    }

    pub fn unit(mut self, unit: Option<impl Into<String>>) -> Self {
        self.unit = unit.map(Into::into);
        self
    }

    pub fn formula(mut self, formula: impl Into<String>) -> Self {
        self.formula = Some(formula.into());
        self
    }

    pub fn description(mut self, description: Option<impl Into<String>>) -> Self {
        self.description = description.map(Into::into);
        self
    }

    pub fn words(mut self, words: ComputedFieldWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let why = self.why;
        let formula = self.formula;
        let help_t = t.clone();
        let mut field = Field::new(&self.label)
            .hint(Some(&self.words.computed))
            .help_ui(move |ui| {
                let t = help_t;
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
                    let (slot, _) = ui.allocate_exact_size(egui::vec2(14.0, 18.0), egui::Sense::hover());
                    icons::paint(
                        ui.painter(),
                        slot.center(),
                        Icon::Calculator,
                        14.0,
                        t.color("muted-foreground"),
                    );
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.add(
                            egui::Label::new(TextRole::Help.rich(&t, &why).color(t.color("muted-foreground"))).wrap(),
                        );
                        if let Some(formula) = &formula {
                            let mono = crate::mono_font(
                                ui.ctx(),
                                crate::mono_small_size(&t, TextRole::Help.font(&t).size),
                                400.0,
                            );
                            ui.label(
                                egui::RichText::new(formula)
                                    .font(mono)
                                    .color(t.color("muted-foreground")),
                            );
                        }
                    });
                });
            });
        if let Some(description) = &self.description {
            field = field.description(description);
        }
        let mut shown = match (&self.value, &self.unit) {
            (None, _) => String::new(),
            (Some(v), None) => v.clone(),
            (Some(v), Some(u)) => format!("{v} {u}"),
        };
        field.show(ui, |ui| {
            ui.add(
                Input::new(&mut shown)
                    .name(&self.label)
                    .placeholder(&self.words.pending)
                    .read_only(true),
            );
        });
    }
}

/// The words a yes or no field says.
#[derive(Clone, Debug)]
pub struct YesNoFieldWords {
    pub yes: String,
    pub no: String,
    pub clear: String,
    pub unanswered: String,
}

impl Default for YesNoFieldWords {
    fn default() -> Self {
        Self {
            yes: "Yes".into(),
            no: "No".into(),
            clear: "Clear".into(),
            unanswered: "Not answered".into(),
        }
    }
}

/// A yes or no, or not answered: Yes and No as a segmented choice, "Not answered" beside them while
/// neither is chosen, and Clear once one is, unless the question is required. A question that keeps
/// no No (`yes_only`) offers Yes alone.
pub struct YesNoField<'a> {
    label: String,
    value: &'a mut Option<bool>,
    required: bool,
    yes_only: bool,
    description: Option<String>,
    help: Option<String>,
    error: Option<String>,
    issues: Vec<(super::IssueTone, String)>,
    words: YesNoFieldWords,
}

impl<'a> YesNoField<'a> {
    pub fn new(label: impl Into<String>, value: &'a mut Option<bool>) -> Self {
        Self {
            label: label.into(),
            value,
            required: false,
            yes_only: false,
            description: None,
            help: None,
            error: None,
            issues: Vec::new(),
            words: YesNoFieldWords::default(),
        }
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn yes_only(mut self, yes_only: bool) -> Self {
        self.yes_only = yes_only;
        self
    }

    pub fn description(mut self, description: Option<impl Into<String>>) -> Self {
        self.description = description.map(Into::into);
        self
    }

    pub fn help(mut self, help: Option<impl Into<String>>) -> Self {
        self.help = help.map(Into::into);
        self
    }

    pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    /// What is wrong with the answer, each in its tone, on the field's issue mark.
    pub fn issues(mut self, issues: impl IntoIterator<Item = (super::IssueTone, String)>) -> Self {
        self.issues = issues.into_iter().collect();
        self
    }

    pub fn words(mut self, words: YesNoFieldWords) -> Self {
        self.words = words;
        self
    }

    /// Draws the field; true when its answer changed.
    pub fn show(self, ui: &mut Ui) -> bool {
        let t = tokens(ui.ctx());
        let mut field = Field::new(&self.label)
            .required(self.required)
            .help_if(self.help.clone())
            .error(self.error.clone())
            .issues(self.issues.clone());
        if let Some(description) = &self.description {
            field = field.description(description);
        }
        let mut changed = false;
        field.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let mut options = vec![self.words.yes.clone()];
                if !self.yes_only {
                    options.push(self.words.no.clone());
                }
                let mut at = match *self.value {
                    Some(true) => 0,
                    Some(false) => 1,
                    None => usize::MAX,
                };
                if ui.add(Segmented::new(&mut at, options)).changed() {
                    *self.value = Some(at == 0);
                    changed = true;
                }
                if self.value.is_none() {
                    ui.label(
                        TextRole::Help
                            .rich(&t, &self.words.unanswered)
                            .color(t.color("muted-foreground")),
                    );
                } else if !self.required {
                    let clear = Button::new(&self.words.clear)
                        .icon(Icon::Eraser)
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Sm);
                    if ui.add(clear).clicked() {
                        *self.value = None;
                        changed = true;
                    }
                }
            });
        });
        changed
    }
}

/// One issue in a form's list: the field it belongs to, what it says, and whether it is only a
/// warning (allowed) rather than an error that stops the submit.
#[derive(Clone, Debug, Default)]
pub struct IssueEntry {
    pub label: String,
    pub text: String,
    pub warning: bool,
}

/// The words an issue count says: its counts in words, for its accessible name.
#[derive(Clone, Debug)]
pub struct IssueCountWords {
    /// How many issues stop the submit: `2 errors`.
    pub errors: fn(usize) -> String,
    /// How many warnings: `1 warning`.
    pub warnings: fn(usize) -> String,
}

impl Default for IssueCountWords {
    fn default() -> Self {
        Self {
            errors: |count| {
                let n = crate::numbers::locale_number(count as f64);
                if count == 1 {
                    format!("{n} error")
                } else {
                    format!("{n} errors")
                }
            },
            warnings: |count| {
                let n = crate::numbers::locale_number(count as f64);
                if count == 1 {
                    format!("{n} warning")
                } else {
                    format!("{n} warnings")
                }
            },
        }
    }
}

/// A form's issues as one control beside its verb, after sokkel-ui's `IssueCount`: a figure for
/// each kind, the errors in the critical ink and the warnings in the warning ink behind a rule, on
/// the tint of the worst. With a list it is a button that opens the issues above it, each going to
/// its field; with none it is a status. Nothing is drawn at zero. Answers with the index of the
/// issue chosen.
pub struct IssueCount<'a> {
    id: egui::Id,
    critical: usize,
    warning: usize,
    list: &'a [IssueEntry],
    default_open: bool,
    words: IssueCountWords,
}

impl<'a> IssueCount<'a> {
    /// The count of `list`'s issues, its errors and its warnings counted from it.
    pub fn new(id: impl egui::AsIdSalt, list: &'a [IssueEntry]) -> Self {
        Self {
            id: egui::Id::unique(id),
            critical: list.iter().filter(|e| !e.warning).count(),
            warning: list.iter().filter(|e| e.warning).count(),
            list,
            default_open: false,
            words: IssueCountWords::default(),
        }
    }

    /// Counts with nothing to open: the face is a status.
    pub fn counts(id: impl egui::AsIdSalt, critical: usize, warning: usize) -> Self {
        Self::new(id, &[]).critical(critical).warning(warning)
    }

    /// How many issues stop the submit.
    pub fn critical(mut self, count: usize) -> Self {
        self.critical = count;
        self
    }

    /// How many warnings.
    pub fn warning(mut self, count: usize) -> Self {
        self.warning = count;
        self
    }

    /// Start with the list open.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    pub fn words(mut self, words: IssueCountWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let tone = if self.critical > 0 {
            "critical"
        } else if self.warning > 0 {
            "warning"
        } else {
            return None;
        };
        let t = tokens(ui.ctx());
        let said = [
            (self.critical > 0).then(|| (self.words.errors)(self.critical)),
            (self.warning > 0).then(|| (self.words.warnings)(self.warning)),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");
        // each kind's figure: its mark, 4, its count, padded 8 across
        let figures: Vec<(Icon, egui::Color32, std::sync::Arc<egui::Galley>)> = [
            (self.critical, Icon::CircleAlert, "critical-ink"),
            (self.warning, Icon::TriangleAlert, "warning-ink"),
        ]
        .into_iter()
        .filter(|(count, ..)| *count > 0)
        .map(|(count, icon, ink)| {
            let digits = WidgetText::from(
                TextRole::Help
                    .rich(&t, count.to_string())
                    .variation("wght", t.weight(crate::Weight::Medium)),
            )
            .into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                TextRole::Help.font(&t),
            );
            (icon, t.color(ink), digits)
        })
        .collect();
        let width: f32 = figures.iter().map(|(.., d)| 8.0 + 14.0 + 4.0 + d.size().x + 8.0).sum();
        let opens = !self.list.is_empty();
        let mut height = t.density("control-sm");
        if opens && super::is_phone(ui.ctx()) {
            height = height.max(t.length("spacing-finger"));
        }
        let sense = if opens {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        };
        let (rect, face) = ui.allocate_exact_size(egui::vec2(width, height), sense);
        let role = if opens { egui::Role::Button } else { egui::Role::Status };
        face.widget_info(|| egui::WidgetInfo::labeled(role, true, &said));
        let open_id = self.id.with("open");
        let mut open: bool = opens && ui.data(|d| d.get_temp(open_id)).unwrap_or(self.default_open);
        let lit = opens && (face.hovered() || open);
        let radius = t.radius("control");
        let (rest, edge) = if tone == "critical" { (0.08, 0.4) } else { (0.08, 0.45) };
        ui.painter().rect(
            rect,
            radius,
            t.color_at(tone, if lit { 0.12 } else { rest }),
            egui::Stroke::new(1.0, t.color_at(tone, edge)),
            egui::StrokeKind::Inside,
        );
        let mut x = rect.left();
        for (index, (icon, ink, digits)) in figures.into_iter().enumerate() {
            if index > 0 {
                // the warnings' figure behind a rule in its own ink at 25 percent
                ui.painter().vline(
                    x + 0.5,
                    rect.y_range().shrink(1.0),
                    egui::Stroke::new(1.0, ink.gamma_multiply(0.25)),
                );
            }
            icons::paint(
                ui.painter(),
                egui::pos2(x + 8.0 + 7.0, rect.center().y),
                icon,
                14.0,
                ink,
            );
            let digits_width = digits.size().x;
            ui.painter().galley(
                egui::pos2(x + 8.0 + 14.0 + 4.0, rect.center().y - digits.size().y / 2.0),
                digits,
                ink,
            );
            x += 8.0 + 14.0 + 4.0 + digits_width + 8.0;
        }
        if face.has_focus() {
            ui.painter().rect_stroke(
                rect,
                radius,
                egui::Stroke::new(t.length("focus-ring-width"), t.color_at("ring", 0.5)),
                egui::StrokeKind::Outside,
            );
        }
        if !opens {
            return None;
        }
        let entry_id = |index: usize| self.id.with(("entry", index));
        let just_opened = face.clicked() && !open;
        if face.clicked() {
            open = !open;
            if open {
                // the focus on the first issue, once the list is laid out
                ui.data_mut(|d| d.insert_temp(self.id.with("focus-first"), true));
            }
        }
        let mut chosen = None;
        if open {
            let screen = ui.ctx().content_rect().width();
            let width = 352.0_f32.min(screen - 32.0).max(120.0);
            let focused = (0..self.list.len()).find(|i| ui.memory(|m| m.has_focus(entry_id(*i))));
            if let Some(at) = focused {
                let count = self.list.len();
                let (down, up) = ui.input_mut(|i| {
                    (
                        i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                        i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                    )
                });
                if down {
                    ui.memory_mut(|m| m.request_focus(entry_id((at + 1) % count)));
                } else if up {
                    ui.memory_mut(|m| m.request_focus(entry_id((at + count - 1) % count)));
                }
            }
            let area = egui::Area::new(self.id.with("list"))
                .order(egui::Order::Foreground)
                .pivot(egui::Align2::LEFT_BOTTOM)
                .fixed_pos(egui::pos2(rect.left(), rect.top() - 6.0))
                .show(ui.ctx(), |ui| {
                    super::popup_frame(&t).inner_margin(6.0).show(ui, |ui| {
                        let inner = width - 12.0;
                        ui.set_width(inner);
                        ui.spacing_mut().item_spacing.y = 4.0;
                        for (index, entry) in self.list.iter().enumerate() {
                            // its mark, 16, 8 before the field's label over the issue's words
                            let words_width = inner - 8.0 - 16.0 - 8.0 - 8.0;
                            let label = WidgetText::from(TextRole::Help.rich(&t, &entry.label)).into_galley(
                                ui,
                                Some(egui::TextWrapMode::Wrap),
                                words_width,
                                TextRole::Help.font(&t),
                            );
                            let sm = t.text("sm");
                            let text = WidgetText::from(
                                egui::RichText::new(&entry.text).size(sm.size).line_height(Some(20.0)),
                            )
                            .into_galley(
                                ui,
                                Some(egui::TextWrapMode::Wrap),
                                words_width,
                                egui::FontId::proportional(sm.size),
                            );
                            let height = 6.0 + label.size().y + text.size().y + 6.0;
                            let (row, _) = ui.allocate_exact_size(egui::vec2(inner, height), egui::Sense::hover());
                            let hit = ui.interact(row, entry_id(index), egui::Sense::click());
                            let name = format!("{}: {}", entry.label, entry.text);
                            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &name));
                            let first_id = self.id.with("focus-first");
                            if index == 0 && !ui.is_sizing_pass() && ui.data(|d| d.get_temp(first_id)).unwrap_or(false)
                            {
                                hit.request_focus();
                                ui.data_mut(|d| d.remove::<bool>(first_id));
                            }
                            if hit.has_focus() {
                                ui.memory_mut(|m| {
                                    m.set_focus_lock_filter(
                                        entry_id(index),
                                        egui::EventFilter {
                                            vertical_arrows: true,
                                            ..Default::default()
                                        },
                                    )
                                });
                            }
                            if hit.hovered() || hit.has_focus() {
                                ui.painter().rect_filled(row, t.radius("md"), t.color("hover"));
                            }
                            let (mark, ink) = if entry.warning {
                                (Icon::TriangleAlert, t.color("warning-ink"))
                            } else {
                                (Icon::CircleAlert, t.color("critical-ink"))
                            };
                            icons::paint(
                                ui.painter(),
                                egui::pos2(row.left() + 8.0 + 8.0, row.top() + 6.0 + 2.0 + 8.0),
                                mark,
                                16.0,
                                ink,
                            );
                            let x = row.left() + 8.0 + 16.0 + 8.0;
                            let label_height = label.size().y;
                            ui.painter()
                                .galley(egui::pos2(x, row.top() + 6.0), label, t.color("muted-foreground"));
                            ui.painter().galley(
                                egui::pos2(x, row.top() + 6.0 + label_height),
                                text,
                                t.color("popover-foreground"),
                            );
                            // the key that opened the list chooses nothing in it
                            if hit.clicked() && !just_opened {
                                chosen = Some(index);
                            }
                        }
                    });
                });
            let outside = ui.input(|i| i.pointer.any_pressed())
                && ui
                    .input(|i| i.pointer.interact_pos())
                    .is_some_and(|p| !area.response.rect.contains(p) && !rect.contains(p));
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                // shut, the focus back on the face
                open = false;
                face.request_focus();
            } else if outside || chosen.is_some() {
                open = false;
            }
        }
        ui.data_mut(|d| d.insert_temp(open_id, open));
        chosen
    }
}

/// The words a repeated group says.
#[derive(Clone, Debug)]
pub struct RepeatedGroupWords {
    pub add: String,
    pub remove: String,
    /// What each row is called: `Item 1`, given the row's index.
    pub item: fn(usize) -> String,
}

impl Default for RepeatedGroupWords {
    fn default() -> Self {
        Self {
            add: "Add".into(),
            remove: "Remove".into(),
            item: |index| format!("Item {}", index + 1),
        }
    }
}

/// What a repeated group was asked this frame.
#[derive(Default)]
pub struct RepeatedOutcome {
    pub added: bool,
    pub removed: Option<usize>,
}

/// A question answered more than once, after sokkel-ui's `RepeatedGroup`: its label with Add at
/// the end, a description under it, and a box per answer with the answer's name and Remove over
/// the row's own controls. Remove goes once `min` rows are left.
pub struct RepeatedGroup {
    label: String,
    description: Option<String>,
    required: bool,
    rows: usize,
    min: usize,
    words: RepeatedGroupWords,
}

impl RepeatedGroup {
    pub fn new(label: impl Into<String>, rows: usize) -> Self {
        Self {
            label: label.into(),
            description: None,
            required: false,
            rows,
            min: 0,
            words: RepeatedGroupWords::default(),
        }
    }

    pub fn description(mut self, description: Option<impl Into<String>>) -> Self {
        self.description = description.map(Into::into);
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn min(mut self, min: usize) -> Self {
        self.min = min;
        self
    }

    pub fn words(mut self, words: RepeatedGroupWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui, mut row: impl FnMut(&mut Ui, usize)) -> RepeatedOutcome {
        let t = tokens(ui.ctx());
        let mut out = RepeatedOutcome::default();
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.horizontal(|ui| {
                ui.set_min_height(t.density("control-sm"));
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(TextRole::Label.rich(&t, &self.label).color(t.color("foreground")));
                if self.required {
                    ui.label(TextRole::Label.rich(&t, "*").color(t.color("critical-ink")));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let add = Button::new(&self.words.add)
                        .icon(Icon::Plus)
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Sm);
                    out.added = ui.add(add).clicked();
                });
            });
            if let Some(description) = &self.description {
                ui.add_space(-4.0);
                ui.add(
                    egui::Label::new(TextRole::Help.rich(&t, description).color(t.color("muted-foreground"))).wrap(),
                );
            }
            for index in 0..self.rows {
                egui::Frame::new()
                    .stroke(egui::Stroke::new(1.0, t.color("border")))
                    .corner_radius(t.radius("surface"))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = 12.0;
                        let name = (self.words.item)(index);
                        ui.horizontal(|ui| {
                            ui.set_height(24.0);
                            ui.label(
                                TextRole::Help
                                    .rich(&t, &name)
                                    .variation("wght", t.weight(crate::Weight::Medium))
                                    .color(t.color("muted-foreground")),
                            );
                            if self.rows > self.min {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let remove = Button::new(&self.words.remove)
                                        .icon(Icon::X)
                                        .variant(ButtonVariant::Ghost)
                                        .size(ButtonSize::Sm)
                                        .muted(true);
                                    if ui.add(remove).clicked() {
                                        out.removed = Some(index);
                                    }
                                });
                            }
                        });
                        row(ui, index);
                    });
            }
        });
        out
    }
}

/// The words a time field says.
#[derive(Clone, Debug)]
pub struct TimeFieldWords {
    pub unreadable: String,
    pub pick: String,
}

impl Default for TimeFieldWords {
    fn default() -> Self {
        Self {
            unreadable: "Enter a time, as in 14:30.".into(),
            pick: "Choose a time".into(),
        }
    }
}

/// A time of day typed on the 24-hour clock, `hh:mm`, with a clock at its end opening the day's
/// times every `step` minutes, after sokkel-ui's `TimeField` with `clock={24}`. Text that is not a
/// time keeps the value as it was and is said.
pub struct TimeField<'a> {
    id: egui::Id,
    label: String,
    value: &'a mut Option<String>,
    step: u32,
    required: bool,
    description: Option<String>,
    help: Option<String>,
    error: Option<String>,
    words: TimeFieldWords,
}

#[derive(Clone, Default)]
struct TimeHeld {
    text: String,
    from: Option<String>,
    open: bool,
}

/// `hh:mm` on the 24-hour clock, or `None`.
pub fn read_time(text: &str) -> Option<String> {
    let (hour, minute) = text.trim().split_once(':')?;
    let (hour, minute) = (hour.trim().parse::<u32>().ok()?, minute.trim().parse::<u32>().ok()?);
    (hour < 24 && minute < 60).then(|| format!("{hour:02}:{minute:02}"))
}

impl<'a> TimeField<'a> {
    pub fn new(id: impl egui::AsIdSalt, label: impl Into<String>, value: &'a mut Option<String>) -> Self {
        Self {
            id: egui::Id::unique(id),
            label: label.into(),
            value,
            step: 30,
            required: false,
            description: None,
            help: None,
            error: None,
            words: TimeFieldWords::default(),
        }
    }

    pub fn step(mut self, minutes: u32) -> Self {
        self.step = minutes.max(1);
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn description(mut self, description: Option<impl Into<String>>) -> Self {
        self.description = description.map(Into::into);
        self
    }

    pub fn help(mut self, help: Option<impl Into<String>>) -> Self {
        self.help = help.map(Into::into);
        self
    }

    pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    /// Draws the field; true when its time changed.
    pub fn show(self, ui: &mut Ui) -> bool {
        let t = tokens(ui.ctx());
        let mut held: TimeHeld = ui.data(|d| d.get_temp(self.id)).unwrap_or_default();
        if held.from != *self.value {
            held.text = self.value.clone().unwrap_or_default();
            held.from = self.value.clone();
        }
        let unreadable = !held.text.trim().is_empty() && read_time(&held.text).is_none();
        let issue = if unreadable {
            Some(self.words.unreadable.clone())
        } else {
            self.error.clone()
        };
        let mut field = Field::new(&self.label)
            .required(self.required)
            .help_if(self.help.clone())
            .error(issue);
        if let Some(description) = &self.description {
            field = field.description(description);
        }
        let mut changed = false;
        field.show(ui, |ui| {
            let before = held.text.clone();
            let input = ui.add(Input::new(&mut held.text).name(&self.label).placeholder("hh:mm"));
            let button = egui::Rect::from_center_size(
                egui::pos2(input.rect.right() - 18.0, input.rect.center().y),
                egui::vec2(28.0, 28.0),
            );
            let hit = ui.interact(button, self.id.with("pick"), egui::Sense::click());
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.pick));
            if hit.hovered() {
                ui.painter().rect_filled(button, t.radius("md"), t.color("hover"));
            }
            icons::paint(
                ui.painter(),
                button.center(),
                Icon::Clock,
                14.0,
                t.color("muted-foreground"),
            );
            if hit.clicked() {
                held.open = !held.open;
            }
            if held.text != before {
                let next = read_time(&held.text);
                if (next.is_some() || held.text.trim().is_empty()) && next != *self.value {
                    *self.value = next.clone();
                    held.from = next;
                    changed = true;
                }
            }
            if held.open {
                let area = egui::Area::new(self.id.with("times"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(egui::pos2(input.rect.right() - 128.0, input.rect.bottom() + 4.0))
                    .show(ui.ctx(), |ui| {
                        super::popup_frame(&t).show(ui, |ui| {
                            egui::ScrollArea::vertical().max_height(256.0).show(ui, |ui| {
                                ui.set_width(128.0 - 16.0);
                                ui.spacing_mut().item_spacing.y = 0.0;
                                let mut minutes = 0;
                                while minutes < 24 * 60 {
                                    let time = format!("{:02}:{:02}", minutes / 60, minutes % 60);
                                    let chosen = self.value.as_deref() == Some(time.as_str());
                                    let (row, hit) = ui.allocate_exact_size(
                                        egui::vec2(ui.available_width(), t.density("control")),
                                        egui::Sense::click(),
                                    );
                                    hit.widget_info(|| {
                                        egui::WidgetInfo::selected(egui::Role::Button, true, chosen, &time)
                                    });
                                    if chosen {
                                        ui.painter().rect_filled(row, t.radius("md"), t.color("primary"));
                                    } else if hit.hovered() {
                                        ui.painter().rect_filled(row, t.radius("md"), t.color("accent"));
                                    }
                                    ui.painter().text(
                                        row.center(),
                                        egui::Align2::CENTER_CENTER,
                                        &time,
                                        TextRole::Body.font(&t),
                                        if chosen {
                                            t.color("primary-foreground")
                                        } else {
                                            t.color("popover-foreground")
                                        },
                                    );
                                    if hit.clicked() {
                                        *self.value = Some(time.clone());
                                        held.text = time.clone();
                                        held.from = Some(time);
                                        held.open = false;
                                        changed = true;
                                    }
                                    minutes += self.step;
                                }
                            });
                        });
                    });
                let outside = ui.input(|i| i.pointer.any_pressed())
                    && ui
                        .input(|i| i.pointer.interact_pos())
                        .is_some_and(|p| !area.response.rect.contains(p) && !input.rect.contains(p));
                if outside || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    held.open = false;
                }
            }
        });
        ui.data_mut(|d| d.insert_temp(self.id, held));
        changed
    }
}

/// The words a period picker says.
#[derive(Clone, Debug)]
pub struct PeriodPickerWords {
    pub other: String,
    pub typed: String,
    pub placeholder: String,
}

impl Default for PeriodPickerWords {
    fn default() -> Self {
        Self {
            other: "Other period".into(),
            typed: "Period id".into(),
            placeholder: "Choose a period".into(),
        }
    }
}

/// A reporting period of one type, after sokkel-ui's `PeriodPicker`: the recent periods in a
/// select, each named with its id after it, and Other period, which opens a field to type any id
/// in, said wrong in a sentence until it names a period of the type. Answers with the issue typed.
pub struct PeriodPicker<'a> {
    id: egui::Id,
    kind: crate::periods::PeriodType,
    value: &'a mut Option<String>,
    today: crate::Day,
    width: Option<f32>,
    words: PeriodPickerWords,
}

impl<'a> PeriodPicker<'a> {
    pub fn new(
        id: impl egui::AsIdSalt,
        kind: crate::periods::PeriodType,
        value: &'a mut Option<String>,
        today: crate::Day,
    ) -> Self {
        Self {
            id: egui::Id::unique(id),
            kind,
            value,
            today,
            width: None,
            words: PeriodPickerWords::default(),
        }
    }

    /// The picker this wide rather than across its room, as one among a section's actions.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn words(mut self, words: PeriodPickerWords) -> Self {
        self.words = words;
        self
    }

    /// Draws the picker; the issue with a typed id, if any.
    pub fn show(self, ui: &mut Ui) -> Option<String> {
        use crate::periods::{parse_period, period_issue, recent_periods};
        let periods = recent_periods(self.kind, self.today, self.kind.recent());
        let typing_id = self.id.with("typing");
        let (mut typing, mut text): (bool, String) = ui.data(|d| d.get_temp(typing_id)).unwrap_or_else(|| {
            let listed = self.value.as_ref().is_none_or(|v| periods.iter().any(|p| p.id == *v));
            (
                !listed,
                if listed {
                    String::new()
                } else {
                    self.value.clone().unwrap_or_default()
                },
            )
        });
        let mut labels: Vec<String> = periods.iter().map(|p| p.label.clone()).collect();
        labels.push(self.words.other.clone());
        let mut codes: Vec<String> = periods.iter().map(|p| p.id.clone()).collect();
        codes.push(String::new());
        let mut at = if typing {
            periods.len()
        } else {
            self.value
                .as_ref()
                .and_then(|v| periods.iter().position(|p| p.id == *v))
                .unwrap_or(usize::MAX)
        };
        let mut issue = None;
        let width = self.width.unwrap_or_else(|| ui.available_width());
        ui.allocate_ui_with_layout(egui::vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let picked = ui.add(
                crate::Select::new(&mut at, labels, &self.words.placeholder)
                    .codes(codes)
                    .placeholder(&self.words.placeholder)
                    .width(width),
            );
            if picked.changed() {
                if at == periods.len() {
                    typing = true;
                    *self.value = parse_period(&text, Some(self.kind)).map(|p| p.id);
                } else if let Some(period) = periods.get(at) {
                    typing = false;
                    *self.value = Some(period.id.clone());
                }
            }
            if typing {
                let before = text.clone();
                let mono_hint = self.kind.example();
                ui.add(Input::new(&mut text).name(&self.words.typed).placeholder(mono_hint));
                if text != before {
                    *self.value = parse_period(&text, Some(self.kind)).map(|p| p.id);
                }
                if !text.trim().is_empty() {
                    issue = period_issue(&text, self.kind);
                }
            }
        });
        ui.data_mut(|d| d.insert_temp(typing_id, (typing, text)));
        issue
    }
}
