//! A board: columns of cards, after sokkel-ui's `Board`, `BoardColumn` and `BoardCard`. A card is
//! dragged from anywhere on it to a place in any column: it fades where it was, a copy follows the
//! pointer, and a line shows where it lands; `Board::show` answers the move.

use egui::{
    Area, Color32, Frame, Id, Margin, Order, Rect, Response, Sense, Shadow, Stroke, TextStyle, TextWrapMode, Ui, Vec2,
    WidgetText, pos2, vec2,
};

use crate::{AvatarSize, Day, DropdownMenu, DueText, Icon, MenuItem, TextRole, icons, tokens};

/// How wide a board's column is.
pub const BOARD_COLUMN: f32 = 288.0;

/// A card dropped in a place it was not: the card's id, the column's it landed in, and its place
/// among that column's other cards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardMove {
    pub card: String,
    pub to: String,
    pub index: usize,
}

/// The card in hand: what it shows, where it came from, and where the pointer holds it.
#[derive(Clone)]
struct Held {
    card: BoardCard,
    column: String,
    grab: Vec2,
    size: Vec2,
}

/// Where the columns and their cards were drawn this frame, for a drop to find its place.
/// A card's id and where it was drawn.
type Placed = (String, Rect);

#[derive(Clone, Default)]
struct Places {
    columns: Vec<(String, Rect, Vec<Placed>)>,
}

fn board_id() -> Id {
    Id::new("sokkel-egui/board")
}

/// A board: its columns side by side, drawn by `columns`, and the drag of a card between them.
pub struct Board {
    id: Id,
}

impl Board {
    pub fn new(id: impl egui::AsId) -> Self {
        Self { id: Id::new(id) }
    }

    /// Draw the columns with `columns`; the card dropped this frame in a new place, if one was.
    pub fn show(self, ui: &mut Ui, columns: impl FnOnce(&mut Ui)) -> Option<BoardMove> {
        let ctx = ui.ctx().clone();
        let held_id = self.id.with("held");
        ctx.data_mut(|d| {
            d.insert_temp(board_id(), self.id);
            d.insert_temp(self.id.with("places"), Places::default());
        });
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            columns(ui);
        });
        ctx.data_mut(|d| d.remove::<Id>(board_id()));
        let held: Option<Held> = ctx.data(|d| d.get_temp(held_id));
        let held = held?;
        let places: Places = ctx.data(|d| d.get_temp(self.id.with("places"))).unwrap_or_default();
        let pointer = ctx.pointer_latest_pos()?;
        let t = tokens(&ctx);
        // where it lands: the column under the pointer, before the first card whose middle is below it
        let landing = places
            .columns
            .iter()
            .find(|(_, rect, _)| rect.x_range().contains(pointer.x))
            .map(|(column, rect, cards)| {
                let others: Vec<&(String, Rect)> = cards.iter().filter(|(id, _)| *id != held.card.id).collect();
                let index = others.iter().take_while(|(_, r)| r.center().y < pointer.y).count();
                let y = match (index.checked_sub(1).and_then(|i| others.get(i)), others.get(index)) {
                    (_, Some((_, next))) => next.top() - 4.0,
                    (Some((_, last)), None) => last.bottom() + 4.0,
                    (None, None) => rect.top() + 48.0,
                };
                ui.painter()
                    .clone()
                    .with_layer_id(egui::LayerId::new(Order::Foreground, self.id.with("line")))
                    .hline(
                        (rect.left() + 8.0)..=(rect.right() - 8.0),
                        y,
                        Stroke::new(2.0, t.color("primary")),
                    );
                (
                    column.clone(),
                    index,
                    others.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>(),
                )
            });
        // the copy that follows the pointer
        Area::new(self.id.with("overlay"))
            .order(Order::Tooltip)
            .fixed_pos(pointer - held.grab)
            .interactable(false)
            .show(&ctx, |ui| {
                ui.set_width(held.size.x);
                held.card.clone().face(ui, held.size.x, true);
            });
        if !ctx.input(|i| i.pointer.any_released()) {
            return None;
        }
        ctx.data_mut(|d| d.remove::<Held>(held_id));
        let (to, index, _) = landing?;
        // back where it was is no move
        let before = places
            .columns
            .iter()
            .find(|(column, _, _)| *column == held.column)
            .and_then(|(_, _, cards)| cards.iter().position(|(id, _)| *id == held.card.id));
        if to == held.column && before == Some(index) {
            return None;
        }
        Some(BoardMove {
            card: held.card.id.clone(),
            to,
            index,
        })
    }
}

/// One column of a board: a head with its icon, title and count, then its cards, on the muted
/// ground; `empty` says what an empty column holds.
pub struct BoardColumn {
    id: String,
    title: String,
    icon: Option<(Icon, Color32)>,
    count: usize,
    empty: String,
}

impl BoardColumn {
    pub fn new(title: impl Into<String>, count: usize) -> Self {
        let title = title.into();
        Self {
            id: title.clone(),
            title,
            icon: None,
            count,
            empty: "Nothing here".into(),
        }
    }

    /// The id a move into this column answers with; its title by default.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
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
            pos2(rect.left() + 8.0, head.bottom() + 2.0),
            pos2(rect.right() - 8.0, rect.bottom() - 8.0),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(body)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(body.expand(4.0).intersect(ui.clip_rect()));
        child.spacing_mut().item_spacing.y = 8.0;
        let board = ui.ctx().data(|d| d.get_temp::<Id>(board_id()));
        // the cards register their places under the column's id as they draw
        if let Some(board) = board {
            ui.ctx().data_mut(|d| {
                let places = d.get_temp_mut_or_default::<Places>(board.with("places"));
                places.columns.push((self.id.clone(), rect, Vec::new()));
            });
        }
        if self.count == 0 {
            child.add_space(8.0);
            child.vertical_centered(|ui| {
                ui.label(TextRole::Help.rich(&t, &self.empty).color(t.color("muted-foreground")))
            });
        } else {
            child.push_id(&self.id, |ui| {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(Id::new("sokkel-egui/board-column"), self.id.clone()));
                cards(ui);
            });
        }
        response
    }
}

/// What a card was asked this frame: its response, and the menu entry chosen.
pub struct CardResponse {
    pub response: Response,
    /// The index of the entry chosen from the card's menu.
    pub chosen: Option<usize>,
}

/// A card on a board: its title on up to three lines, then its due date and its assignee; a menu
/// behind a button at its top end, shown while the card is hovered.
#[derive(Clone)]
pub struct BoardCard {
    id: String,
    title: String,
    assignee: Option<String>,
    due: Option<(Option<Day>, Day)>,
    done: bool,
    menu: Vec<MenuItem>,
    menu_label: String,
}

impl BoardCard {
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        Self {
            id: title.clone(),
            title,
            assignee: None,
            due: None,
            done: false,
            menu: Vec::new(),
            menu_label: "More actions".into(),
        }
    }

    /// The id a move of this card answers with; its title by default.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
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

    /// The entries behind the card's menu button.
    pub fn menu(mut self, items: Vec<MenuItem>) -> Self {
        self.menu = items;
        self
    }

    /// The menu button's name, "More actions" by default.
    pub fn menu_label(mut self, label: impl Into<String>) -> Self {
        self.menu_label = label.into();
        self
    }

    /// The card's face, `width` wide: frame, title and foot. `lifted` for the copy in hand.
    fn face(self, ui: &mut Ui, width: f32, lifted: bool) -> Response {
        let t = tokens(ui.ctx());
        let frame = Frame::new()
            .fill(t.color("card"))
            .stroke(Stroke::new(1.0, t.color("border-strong")))
            .corner_radius(t.radius("surface"))
            .inner_margin(Margin::same(12))
            .shadow(Shadow {
                offset: [0, if lifted { 8 } else { 1 }],
                blur: if lifted { 24 } else { 2 },
                spread: 0,
                color: Color32::from_black_alpha(if t.dark { 40 } else { 14 } + if lifted { 30 } else { 0 }),
            });
        frame
            .show(ui, |ui| {
                ui.set_width(width - 26.0);
                ui.spacing_mut().item_spacing.y = 8.0;
                // the title at the section title's size, on lines 20 tall, at most three
                let spec = t.text("section-title");
                let rich = |text: &str| {
                    egui::RichText::new(text)
                        .size(spec.size)
                        .line_height(Some(20.0))
                        .variation("wght", spec.weight)
                        .color(t.color("card-foreground"))
                };
                let broken = break_like_web(&self.title, width - 26.0 - 64.0, |text| {
                    WidgetText::from(rich(text))
                        .into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, TextStyle::Body)
                        .size()
                        .x
                });
                let rich = rich(&broken);
                // wrapped a little short of the card, as the web's title leaves room for its menu,
                // each line 20 tall
                let galley = WidgetText::from(rich).into_galley(
                    ui,
                    Some(TextWrapMode::Wrap),
                    width - 26.0 - 64.0,
                    TextStyle::Body,
                );
                let lines = galley.rows.len().clamp(1, 3);
                let (title, _) = ui.allocate_exact_size(vec2(width - 26.0, 20.0 * lines as f32), Sense::hover());
                ui.painter()
                    .with_clip_rect(title)
                    .galley(title.min, galley, t.color("card-foreground"));
                // the foot: one line 20 tall, the due date in the help's type, the avatar at the end
                let (foot, _) = ui.allocate_exact_size(vec2(width - 26.0, 20.0), Sense::hover());
                let mut row = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(foot)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                if let Some((date, now)) = self.due {
                    row.add(DueText::new(date, now).done(self.done).icon(true).role(TextRole::Help));
                }
                if let Some(name) = &self.assignee {
                    let avatar = Rect::from_min_size(pos2(foot.right() - 20.0, foot.top()), vec2(20.0, 20.0));
                    super::person::paint_avatar(ui, &t, avatar, name, AvatarSize::Xs, false);
                }
            })
            .response
    }

    pub fn show(self, ui: &mut Ui) -> CardResponse {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        let board = ui.ctx().data(|d| d.get_temp::<Id>(board_id()));
        let column: Option<String> = ui.ctx().data(|d| d.get_temp(Id::new("sokkel-egui/board-column")));
        let held_id = board.map(|b| b.with("held"));
        let in_hand = held_id
            .and_then(|id| ui.ctx().data(|d| d.get_temp::<Held>(id)))
            .is_some_and(|held| held.card.id == self.id);
        // where it was, faded while it is in hand
        let face = ui
            .scope(|ui| {
                if in_hand {
                    ui.multiply_opacity(0.4);
                }
                self.clone().face(ui, width, false)
            })
            .inner;
        let rect = face.rect;
        let response = ui.interact(rect, ui.id().with(("card", &self.id)), Sense::click_and_drag());
        if response.hovered() && !in_hand {
            ui.painter().rect_stroke(
                rect,
                t.radius("surface"),
                Stroke::new(1.0, t.color_at("foreground", 0.25)),
                egui::StrokeKind::Inside,
            );
        }
        if let (Some(board), Some(column)) = (board, &column) {
            ui.ctx().data_mut(|d| {
                let places = d.get_temp_mut_or_default::<Places>(board.with("places"));
                if let Some(entry) = places.columns.iter_mut().rev().find(|(id, _, _)| id == column) {
                    entry.2.push((self.id.clone(), rect));
                }
            });
            if response.drag_started()
                && let (Some(id), Some(pointer)) = (held_id, response.interact_pointer_pos())
            {
                let held = Held {
                    card: self.clone(),
                    column: column.clone(),
                    grab: pointer - rect.min,
                    size: rect.size(),
                };
                ui.ctx().data_mut(|d| d.insert_temp(id, held));
            }
            if response.dragged() || response.hovered() {
                ui.ctx().set_cursor_icon(if response.dragged() {
                    egui::CursorIcon::Grabbing
                } else {
                    egui::CursorIcon::Grab
                });
            }
        }
        // the menu: a button 28 square, 6 in from the card's end and 8 down, shown while hovered
        let mut chosen = None;
        if !self.menu.is_empty() && !in_hand {
            let button = Rect::from_min_size(pos2(rect.right() - 34.0, rect.top() + 8.0), vec2(28.0, 28.0));
            let trigger = ui.interact(button, ui.id().with(("card-menu", &self.id)), Sense::click());
            trigger.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &self.menu_label));
            let open = egui::Popup::is_id_open(ui.ctx(), trigger.id.with("menu"));
            let shown = open || ui.rect_contains_pointer(rect);
            if shown {
                if open || trigger.hovered() {
                    ui.painter().rect_filled(button, t.radius("md"), t.color("hover"));
                }
                let ink = if open || trigger.hovered() {
                    t.color("foreground")
                } else {
                    t.color("muted-foreground")
                };
                icons::paint(ui.painter(), button.center(), Icon::Ellipsis, 16.0, ink);
                // the grip beside it says the card drags
                icons::paint(
                    ui.painter(),
                    pos2(button.left() - 21.0, button.center().y),
                    Icon::GripVertical,
                    14.0,
                    t.color("muted-foreground"),
                );
            }
            chosen = DropdownMenu::new(&self.menu).width(192.0).end(true).show(&trigger);
        }
        CardResponse { response, chosen }
    }
}

/// `text` with a line break wherever a browser would wrap it at `width`: after a space or a hyphen,
/// at the last one that keeps the line within `width`. egui breaks after a hyphen only where a line
/// has no space, so "send-digest" stays whole where the web puts "send-" at a line's end.
fn break_like_web(text: &str, width: f32, measure: impl Fn(&str) -> f32) -> String {
    // the pieces a line may end after: each word with its space, or up to and with its hyphen
    let mut pieces: Vec<&str> = Vec::new();
    let mut start = 0;
    for (i, c) in text.char_indices() {
        if c == ' ' || c == '-' {
            pieces.push(&text[start..i + c.len_utf8()]);
            start = i + c.len_utf8();
        }
    }
    if start < text.len() {
        pieces.push(&text[start..]);
    }
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for piece in pieces {
        let tried = format!("{line}{piece}");
        if !line.is_empty() && measure(tried.trim_end()) > width {
            lines.push(line.trim_end().to_owned());
            line = piece.to_owned();
        } else {
            line = tried;
        }
    }
    if !line.is_empty() {
        lines.push(line.trim_end().to_owned());
    }
    lines.join("\n")
}
