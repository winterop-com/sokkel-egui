//! What happened, newest first, and labeled bars of shares: after sokkel-ui's `ActivityList` and
//! `BarList`.

use egui::{Align2, Rect, Sense, TextWrapMode, Ui, WidgetText, pos2, vec2};

use super::{Button, ButtonSize, ButtonVariant};
use crate::{Icon, TextRole, icons, tokens};

/// How an activity item reads, as sokkel-ui's item `tone`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActivityTone {
    Critical,
    Warning,
    Info,
    Good,
    #[default]
    Quiet,
}

impl ActivityTone {
    fn ink(self) -> &'static str {
        match self {
            Self::Critical => "critical-ink",
            Self::Warning => "warning-ink",
            Self::Info => "info-ink",
            Self::Good => "good-ink",
            Self::Quiet => "muted-foreground",
        }
    }
}

/// One thing that happened, as sokkel-ui's `ActivityItem`: what, when, how it reads, and the one
/// way to act on it.
#[derive(Clone, Debug, Default)]
pub struct ActivityItem {
    pub id: String,
    pub text: String,
    /// When it happened, as the app says it: `12 min ago`.
    pub when: String,
    /// The heading it is listed under, such as Today; items keep their order within it.
    pub group: Option<String>,
    pub icon: Option<Icon>,
    pub tone: ActivityTone,
    pub unread: bool,
    /// The label of the button that acts on it.
    pub action: Option<String>,
}

/// The words an activity list says.
#[derive(Clone, Debug)]
pub struct ActivityWords {
    pub title: String,
    pub mark_all_read: String,
    pub empty: String,
}

impl Default for ActivityWords {
    fn default() -> Self {
        Self {
            title: "Activity".into(),
            mark_all_read: "Mark all read".into(),
            empty: "Nothing new.".into(),
        }
    }
}

/// What an activity list was asked this frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActivityResponse {
    /// The item whose action was pressed, so it counts as read.
    pub acted: Option<String>,
    pub read_all: bool,
}

/// What happened, as sokkel-ui's `ActivityList`: newest first, in groups when given, the unread
/// ones on a wash, stronger, with a dot; each item's own action. `head` draws its title, its
/// unread count and Mark all read; off in a panel whose head says what it holds.
pub struct ActivityList<'a> {
    items: &'a [ActivityItem],
    head: bool,
    words: ActivityWords,
}

impl<'a> ActivityList<'a> {
    pub fn new(items: &'a [ActivityItem]) -> Self {
        Self {
            items,
            head: true,
            words: ActivityWords::default(),
        }
    }

    pub fn head(mut self, head: bool) -> Self {
        self.head = head;
        self
    }

    pub fn words(mut self, words: ActivityWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ActivityResponse {
        let t = tokens(ui.ctx());
        let mut response = ActivityResponse::default();
        let width = ui.available_width();
        let spacing = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 0.0;
        let unread = self.items.iter().filter(|i| i.unread).count();
        if self.head {
            let (head, _) = ui.allocate_exact_size(vec2(width, t.density("panel-head")), Sense::hover());
            let title = if unread > 0 {
                format!("{}  {unread}", self.words.title)
            } else {
                self.words.title.clone()
            };
            ui.painter().text(
                pos2(head.left() + 12.0, head.center().y),
                Align2::LEFT_CENTER,
                title,
                TextRole::Label.font(&t),
                t.color("foreground"),
            );
            ui.painter().hline(
                head.x_range(),
                head.bottom() - 0.5,
                egui::Stroke::new(1.0, t.color("border")),
            );
            let mut end = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(head.shrink2(vec2(12.0, 0.0)))
                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
            );
            let button = Button::new(&self.words.mark_all_read)
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::Xs)
                .enabled(unread > 0);
            if end.add(button).clicked() {
                response.read_all = true;
            }
        }
        if self.items.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    TextRole::Body
                        .rich(&t, &self.words.empty)
                        .color(t.color("muted-foreground")),
                );
            });
            ui.add_space(24.0);
            ui.spacing_mut().item_spacing.y = spacing;
            return response;
        }
        ui.add_space(4.0);
        let mut group: Option<&str> = None;
        for item in self.items {
            if let Some(heading) = item.group.as_deref()
                && group != Some(heading)
            {
                let (rect, _) = ui.allocate_exact_size(vec2(width, 26.0), Sense::hover());
                ui.painter().text(
                    pos2(rect.left() + 12.0, rect.bottom() - 4.0),
                    Align2::LEFT_BOTTOM,
                    heading,
                    TextRole::Help.font(&t),
                    t.color("faint"),
                );
            }
            group = item.group.as_deref();
            // 4 in from the list, 8 more to the item's words, the mark 16 before them
            let left = 4.0 + 8.0 + if item.icon.is_some() { 16.0 + 10.0 } else { 0.0 };
            let action = item.action.as_ref().map(|label| {
                let galley = WidgetText::from(TextRole::Help.rich(&t, label)).into_galley(
                    ui,
                    Some(TextWrapMode::Extend),
                    f32::INFINITY,
                    TextRole::Help.font(&t),
                );
                galley.size().x + 16.0
            });
            let right = 4.0 + 8.0 + action.map_or(0.0, |w| w + 10.0) + if item.unread { 8.0 + 10.0 } else { 0.0 };
            let room = (width - left - right).max(1.0);
            let mut words = TextRole::Body.rich(&t, &item.text);
            if item.unread {
                words = words.variation("wght", 500.0);
            }
            let text = WidgetText::from(words).into_galley(ui, Some(TextWrapMode::Wrap), room, TextRole::Body.font(&t));
            let when = WidgetText::from(TextRole::Help.rich(&t, &item.when)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                room,
                TextRole::Help.font(&t),
            );
            let height = 8.0 + text.size().y + when.size().y + 8.0;
            let (outer, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
            let row = Rect::from_min_max(
                pos2(outer.left() + 4.0, outer.top()),
                pos2(outer.right() - 4.0, outer.bottom()),
            );
            if item.unread {
                ui.painter().rect_filled(row, t.radius("md"), t.color_at("muted", 0.6));
            }
            if let Some(icon) = item.icon {
                icons::paint(
                    ui.painter(),
                    pos2(row.left() + 8.0 + 8.0, row.top() + 8.0 + 10.0),
                    icon,
                    16.0,
                    t.color(item.tone.ink()),
                );
            }
            let x = outer.left() + left;
            let text_height = text.size().y;
            ui.painter()
                .galley(pos2(x, row.top() + 8.0), text, t.color("foreground"));
            ui.painter().galley(
                pos2(x, row.top() + 8.0 + text_height),
                when,
                t.color("muted-foreground"),
            );
            let mut end = row.right() - 8.0;
            if item.unread {
                ui.painter()
                    .circle_filled(pos2(end - 4.0, row.top() + 12.0), 4.0, t.color("info"));
                end -= 18.0;
            }
            if let (Some(label), Some(w)) = (&item.action, action) {
                let spot = Rect::from_min_size(pos2(end - w, row.top() + 6.0), vec2(w, t.density("control-xs")));
                let pressed = ui
                    .new_child(egui::UiBuilder::new().max_rect(spot))
                    .add(Button::new(label).variant(ButtonVariant::Outline).size(ButtonSize::Xs))
                    .clicked();
                if pressed {
                    response.acted = Some(item.id.clone());
                }
            }
        }
        ui.add_space(4.0);
        ui.spacing_mut().item_spacing.y = spacing;
        response
    }
}

/// One bar of a bar list, as sokkel-ui's `BarListItem`: its label and its share, from 0 to 1.
#[derive(Clone, Debug, Default)]
pub struct BarListItem {
    pub label: String,
    /// The share, from 0 to 1.
    pub value: f64,
    /// The bar the answer is: drawn in the accent, the others quiet.
    pub chosen: bool,
    /// Words said after the value, such as the counts behind the share.
    pub note: Option<String>,
}

/// Labeled bars, each a share from 0 to 1, as sokkel-ui's `BarList`: the labels, bars and values
/// in three columns (a fourth for the notes), the chosen one in the primary color, the rest
/// quiet; a value written as a whole percent. The bars keep at least two fifths of the width.
pub struct BarList<'a> {
    items: &'a [BarListItem],
    format: Box<dyn Fn(f64) -> String + 'a>,
}

impl<'a> BarList<'a> {
    pub fn new(items: &'a [BarListItem]) -> Self {
        Self {
            items,
            format: Box::new(|v| format!("{}%", (v * 100.0).round())),
        }
    }

    /// How a value is written; a whole percent by default.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'a) -> Self {
        self.format = Box::new(format);
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        let gap = 12.0;
        let measure = |text: &str, role: TextRole| {
            WidgetText::from(role.rich(&t, text))
                .into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, role.font(&t))
                .size()
                .x
        };
        let labels = self
            .items
            .iter()
            .map(|i| measure(&i.label, TextRole::Body))
            .fold(0.0, f32::max);
        let values = self
            .items
            .iter()
            .map(|i| measure(&(self.format)(i.value), TextRole::Help))
            .fold(0.0, f32::max);
        let notes = self
            .items
            .iter()
            .filter_map(|i| i.note.as_ref())
            .map(|n| measure(n, TextRole::Help))
            .fold(0.0_f32, f32::max);
        let noted = notes > 0.0;
        let inner = width - 12.0;
        let gaps = gap * if noted { 3.0 } else { 2.0 };
        // the bars at least two fifths; the label and the note cut short to fit the rest
        let free = (inner - gaps - values - inner * 0.4).max(0.0);
        let (label_w, note_w) = if labels + notes <= free {
            (labels, notes)
        } else {
            let share = free / (labels + notes).max(1.0);
            (labels * share, notes * share)
        };
        let bar_w = inner - gaps - values - label_w - note_w;
        let spacing = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 0.0;
        for item in self.items {
            // 4 over and under a line of the body's words, as the web's `py-1` row
            let tall = TextRole::Body.line_height(&t) + 8.0;
            let (row, _) = ui.allocate_exact_size(vec2(width, tall), Sense::hover());
            let painter = ui.painter();
            let middle = row.center().y;
            let ink = if item.chosen {
                t.color("foreground")
            } else {
                t.color("muted-foreground")
            };
            let mut x = row.left() + 6.0;
            let mut label = TextRole::Body.rich(&t, &item.label);
            if item.chosen {
                label = label.variation("wght", 500.0);
            }
            let galley = WidgetText::from(label).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                label_w.max(1.0),
                TextRole::Body.font(&t),
            );
            painter.galley(pos2(x, middle - galley.size().y / 2.0), galley, ink);
            x += label_w + gap;
            let track = Rect::from_min_size(pos2(x, middle - 3.0), vec2(bar_w, 6.0));
            painter.rect_filled(track, t.radius("mini"), t.color("muted"));
            let share = if item.value.is_finite() {
                item.value.clamp(0.0, 1.0)
            } else {
                0.0
            } as f32;
            let fill = if item.chosen {
                t.color("primary")
            } else {
                t.color_at("muted-foreground", 0.4)
            };
            painter.rect_filled(
                Rect::from_min_size(track.min, vec2(bar_w * share, 6.0)),
                t.radius("mini"),
                fill,
            );
            x += bar_w + gap;
            painter.text(
                pos2(x + values, middle),
                Align2::RIGHT_CENTER,
                (self.format)(item.value),
                TextRole::Help.font(&t),
                ink,
            );
            x += values + gap;
            if let Some(note) = &item.note {
                let galley = WidgetText::from(TextRole::Help.rich(&t, note)).into_galley(
                    ui,
                    Some(TextWrapMode::Truncate),
                    note_w.max(1.0),
                    TextRole::Help.font(&t),
                );
                ui.painter().galley(
                    pos2(x, middle - galley.size().y / 2.0),
                    galley,
                    t.color("muted-foreground"),
                );
            }
        }
        ui.spacing_mut().item_spacing.y = spacing;
    }
}
