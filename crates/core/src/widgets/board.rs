//! A board: columns of cards, after sokkel-ui's `Board`, `BoardColumn` and `BoardCard`.

use egui::{Color32, Frame, Margin, Rect, Response, Sense, Shadow, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{AvatarSize, Day, DueText, Icon, TextRole, icons, tokens};

/// How wide a board's column is.
pub const BOARD_COLUMN: f32 = 288.0;

/// One column of a board: a head with its icon, title and count, then its cards, on the muted
/// ground; `empty` says what an empty column holds.
pub struct BoardColumn {
    title: String,
    icon: Option<(Icon, Color32)>,
    count: usize,
    empty: String,
}

impl BoardColumn {
    pub fn new(title: impl Into<String>, count: usize) -> Self {
        Self {
            title: title.into(),
            icon: None,
            count,
            empty: "Nothing here".into(),
        }
    }

    pub fn icon(mut self, icon: Icon, color: Color32) -> Self {
        self.icon = Some((icon, color));
        self
    }

    pub fn empty(mut self, empty: impl Into<String>) -> Self {
        self.empty = empty.into();
        self
    }

    /// Draw the column `height` tall; `cards` draws its cards one under another.
    pub fn show(self, ui: &mut Ui, height: f32, cards: impl FnOnce(&mut Ui)) -> Response {
        let t = tokens(ui.ctx());
        let ground = t.color_at("muted", if t.dark { 0.3 } else { 0.5 });
        let (rect, response) = ui.allocate_exact_size(vec2(BOARD_COLUMN, height), Sense::hover());
        ui.painter().rect_filled(rect, t.radius("surface"), ground);
        // the head: icon, title, count
        let head = Rect::from_min_size(rect.min + vec2(6.0, 4.0), vec2(rect.width() - 12.0, 36.0));
        let mut x = head.left() + 6.0;
        if let Some((icon, color)) = self.icon {
            icons::paint(ui.painter(), pos2(x + 8.0, head.center().y), icon, 16.0, color);
            x += 24.0;
        }
        let title = WidgetText::from(TextRole::Action.rich(&t, &self.title)).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            head.width() - 80.0,
            TextRole::Action.font(&t),
        );
        let title_w = title.size().x;
        ui.painter().galley(
            pos2(x, head.center().y - title.size().y / 2.0),
            title,
            t.color("foreground"),
        );
        x += title_w + 8.0;
        let count = WidgetText::from(TextRole::Help.rich(&t, self.count.to_string())).into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            TextRole::Help.font(&t),
        );
        let chip = Rect::from_min_size(pos2(x, head.center().y - 10.0), vec2(count.size().x + 12.0, 20.0));
        ui.painter().rect_filled(chip, t.radius("chip"), t.color("secondary"));
        ui.painter().galley(
            pos2(chip.left() + 6.0, chip.center().y - count.size().y / 2.0),
            count,
            t.color("muted-foreground"),
        );
        // the cards, clipped to the column
        let body = Rect::from_min_max(
            pos2(rect.left() + 8.0, head.bottom() + 4.0),
            pos2(rect.right() - 8.0, rect.bottom() - 8.0),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(body)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(body.expand(4.0).intersect(ui.clip_rect()));
        child.spacing_mut().item_spacing.y = 8.0;
        if self.count == 0 {
            child.add_space(8.0);
            child.vertical_centered(|ui| {
                ui.label(TextRole::Help.rich(&t, &self.empty).color(t.color("muted-foreground")))
            });
        } else {
            cards(&mut child);
        }
        response
    }
}

/// A card on a board: its title on up to three lines, then its due date and its assignee.
pub struct BoardCard {
    title: String,
    assignee: Option<String>,
    due: Option<(Option<Day>, Day)>,
    done: bool,
}

impl BoardCard {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            assignee: None,
            due: None,
            done: false,
        }
    }

    pub fn assignee(mut self, name: impl Into<String>) -> Self {
        self.assignee = Some(name.into());
        self
    }

    pub fn due(mut self, date: Option<Day>, now: Day) -> Self {
        self.due = Some((date, now));
        self
    }

    pub fn done(mut self, done: bool) -> Self {
        self.done = done;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        let frame = Frame::new()
            .fill(t.color("card"))
            .stroke(Stroke::new(1.0, t.color("border-strong")))
            .corner_radius(t.radius("surface"))
            .inner_margin(Margin::same(12))
            .shadow(Shadow {
                offset: [0, 1],
                blur: 2,
                spread: 0,
                color: Color32::from_black_alpha(if t.dark { 40 } else { 14 }),
            });
        let response = frame
            .show(ui, |ui| {
                ui.set_width(width - 26.0);
                ui.spacing_mut().item_spacing.y = 8.0;
                // the title at the section title's size, on lines 20 tall, at most three
                let spec = t.text("section-title");
                let rich = egui::RichText::new(&self.title)
                    .size(spec.size)
                    .line_height(Some(20.0))
                    .variation("wght", spec.weight)
                    .color(t.color("card-foreground"));
                // wrapped a little short of the card, as the web's title leaves room for its menu
                ui.allocate_ui(vec2(width - 26.0 - 64.0, 60.0), |ui| {
                    ui.add(egui::Label::new(rich).wrap())
                });
                ui.horizontal(|ui| {
                    if let Some((date, now)) = self.due {
                        ui.add(DueText::new(date, now).done(self.done).icon(true));
                    }
                    if let Some(name) = &self.assignee {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let (rect, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
                            super::person::paint_avatar(ui, &t, rect, name, AvatarSize::Xs, false);
                        });
                    }
                });
            })
            .response;
        response.interact(Sense::click())
    }
}
