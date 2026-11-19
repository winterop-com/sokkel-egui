//! The card for one thing, after sokkel-ui's `ItemCard`, and a grid of them the reader puts in
//! order by dragging, after its `SortableGrid`.

use egui::{Rect, Response, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{Icon, TextRole, tokens};

/// What a card shows over its words, drawn in the room given.
type Media<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A fact at a card's foot: what it is, and its value.
pub struct CardFact {
    pub label: String,
    pub value: String,
}

/// The words an item card says.
#[derive(Clone, Debug)]
pub struct ItemCardWords {
    /// The menu button's name.
    pub menu: String,
}

impl Default for ItemCardWords {
    fn default() -> Self {
        Self {
            menu: "More actions".into(),
        }
    }
}

/// The card for one thing: a form, a project, a job, after sokkel-ui's `ItemCard`. Its title is
/// the thing's name, or its code when it has none ([`ItemCard::from_code`]), the code drawn once
/// in the code face; an icon on a tile may lead the title, and a status, a description (up to
/// three lines) and facts at the foot follow; a picture of it (`media`) can stand over the words.
/// With [`ItemCard::href`] or [`ItemCard::selectable`] the whole card is one control; `selected`
/// marks the chosen card of a set, and a menu sits at the head's end as its own control.
pub struct ItemCard<'a> {
    icon: Option<Icon>,
    media: Option<Media<'a>>,
    title_lines: usize,
    title: Option<String>,
    code: Option<String>,
    description: Option<String>,
    status: Option<super::section::Tools<'a>>,
    facts: Vec<CardFact>,
    selected: bool,
    selectable: bool,
    href: Option<String>,
    menu: Option<(&'a [crate::MenuItem], &'a mut Option<usize>)>,
    words: ItemCardWords,
}

impl<'a> ItemCard<'a> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            icon: None,
            media: None,
            title_lines: 2,
            title: Some(title.into()),
            code: None,
            description: None,
            status: None,
            facts: Vec::new(),
            selected: false,
            selectable: false,
            href: None,
            menu: None,
            words: ItemCardWords::default(),
        }
    }

    /// A card with no name: its code is its title, in the code face.
    pub fn from_code(code: impl Into<String>) -> Self {
        let mut card = Self::new(String::new());
        card.title = None;
        card.code = Some(code.into());
        card
    }

    /// The thing's addressable code, in the code face under the name.
    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Up to three lines about it, in the muted ink.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Chips and words in a row under the head that wraps, 8 apart.
    pub fn status(mut self, status: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.status = Some(Box::new(status));
        self
    }

    pub fn fact(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.facts.push(CardFact {
            label: label.into(),
            value: value.into(),
        });
        self
    }

    /// An icon before the title, on a 32 tile on the muted ground, as the web's `icon`.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// A picture of the thing over the words, as the web's `media`: drawn by `media` in a band as
    /// wide as the card and 9/16 of that tall, on the muted ground, clipped to it.
    pub fn media(mut self, media: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.media = Some(Box::new(media));
        self
    }

    /// The most lines the title takes, cut short with an ellipsis past them (2 by default, 1 to 3),
    /// its whole text on hover when cut, as the web's `titleLines`.
    pub fn title_lines(mut self, lines: usize) -> Self {
        self.title_lines = lines.clamp(1, 3);
        self
    }

    /// Marks the chosen card of a set: a primary edge with a 1px primary ring, and a check in the
    /// head.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// The whole card is one control whose press the app answers (the response's `clicked`), as
    /// the web's `onSelect`.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// The whole card is a link to `href`.
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// Menu items behind a button at the head's end, its own control over the card's; `chosen`
    /// is set to the index of the item picked.
    pub fn menu(mut self, items: &'a [crate::MenuItem], chosen: &'a mut Option<usize>) -> Self {
        self.menu = Some((items, chosen));
        self
    }

    pub fn words(mut self, words: ItemCardWords) -> Self {
        self.words = words;
        self
    }

    /// Draws the card `width` wide, at least `height` tall; its response is the press on it.
    pub fn show(self, ui: &mut Ui, width: f32, height: f32) -> Response {
        let t = tokens(ui.ctx());
        let inner = (width - 32.0).max(1.0);
        let interactive = self.selectable || self.href.is_some();
        // the head: the icon's tile 4 more than the head's 8 before the heading, the check and the
        // menu after it, 8 apart
        let lead = if self.icon.is_some() { 32.0 + 12.0 } else { 0.0 };
        let trail = if self.selected { 8.0 + 20.0 } else { 0.0 } + if self.menu.is_some() { 8.0 + 28.0 } else { 0.0 };
        let heading_width = (inner - lead - trail).max(1.0);
        let mono = crate::mono_font(ui.ctx(), crate::mono_size(&t, t.text("help").size), 400.0);
        // the heading: the name, cut short past its lines; with none, the code in the code face
        let (title_text, title_rich, title_font) = match &self.title {
            Some(title) => (
                title.clone(),
                TextRole::SectionTitle.rich(&t, title),
                TextRole::SectionTitle.font(&t),
            ),
            None => {
                let code = self.code.clone().unwrap_or_default();
                let size = crate::mono_size(&t, t.text("section-title").size);
                let font = crate::mono_font(ui.ctx(), size, 500.0);
                (code.clone(), egui::RichText::new(code).font(font.clone()), font)
            }
        };
        let mut job = (*WidgetText::from(title_rich).into_layout_job(
            ui.style(),
            egui::FontSelection::FontId(title_font),
            egui::Align::Min,
        ))
        .clone();
        job.wrap.max_width = heading_width;
        job.wrap.max_rows = self.title_lines;
        job.wrap.break_anywhere = false;
        job.wrap.overflow_character = Some('\u{2026}');
        let title = ui.fonts_mut(|f| f.layout_job(job));
        let media_height = if self.media.is_some() {
            (width * 9.0 / 16.0).round()
        } else {
            0.0
        };
        let code = self.code.as_ref().filter(|_| self.title.is_some()).map(|c| {
            WidgetText::from(egui::RichText::new(c).font(mono.clone())).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                heading_width,
                mono.clone(),
            )
        });
        // the description, up to three lines
        let description = self.description.as_ref().map(|d| {
            let mut job = (*WidgetText::from(TextRole::Body.rich(&t, d)).into_layout_job(
                ui.style(),
                egui::FontSelection::FontId(TextRole::Body.font(&t)),
                egui::Align::Min,
            ))
            .clone();
            job.wrap.max_width = inner;
            job.wrap.max_rows = 3;
            job.wrap.overflow_character = Some('\u{2026}');
            ui.fonts_mut(|f| f.layout_job(job))
        });
        // the facts: as many columns as fit, each at least 80, 12 apart
        let fact_columns = (((inner + 12.0) / (80.0 + 12.0)).floor() as usize).clamp(1, self.facts.len().max(1));
        let fact_width = (inner - 12.0 * (fact_columns - 1) as f32) / fact_columns as f32;
        let fact_rows = self.facts.len().div_ceil(fact_columns);
        let foot = if self.facts.is_empty() {
            0.0
        } else {
            1.0 + 10.0 + fact_rows as f32 * 36.0 + (fact_rows.saturating_sub(1)) as f32 * 12.0 + 10.0
        };
        // the card takes the press first, at last frame's height, so what lies over it (the
        // status, the menu) takes the pointer before it
        let origin = ui.cursor().min;
        let id = ui.next_auto_id().with("item-card");
        let last: f32 = ui.data(|d| d.get_temp(id.with("height"))).unwrap_or(height);
        let guess = Rect::from_min_size(origin, vec2(width, last.max(height)));
        let sense = if interactive { Sense::click() } else { Sense::hover() };
        let response = ui.interact(guess, id, sense);
        let name = title_text.clone();
        let role = if self.href.is_some() {
            egui::Role::Link
        } else if interactive {
            egui::Role::Button
        } else {
            egui::Role::Group
        };
        response.widget_info(|| egui::WidgetInfo::selected(role, true, self.selected, &name));
        if interactive && response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.clicked()
            && let Some(href) = &self.href
        {
            ui.ctx().open_url(egui::OpenUrl::new_tab(href));
        }
        let painter = ui.painter().clone();
        let ground_slot = painter.add(egui::Shape::Noop);
        let radius = t.radius("surface");
        let hot = interactive && response.hovered();
        // the picture over the words, on the muted ground under the card's top corners
        let media_slot = painter.add(egui::Shape::Noop);
        if let Some(media) = self.media {
            let band = Rect::from_min_size(origin, vec2(width, media_height));
            let top = egui::CornerRadius { sw: 0, se: 0, ..radius };
            painter.set(media_slot, egui::Shape::rect_filled(band, top, t.color("muted")));
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(band)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            child.set_clip_rect(band.intersect(ui.clip_rect()));
            media(&mut child);
        }
        let x = origin.x + 16.0;
        let mut y = origin.y + media_height + 16.0;
        let title_height = title.size().y;
        let cut = title.elided;
        let head_height = if self.icon.is_some() {
            title_height.max(32.0)
        } else {
            title_height
        };
        let title_ink = t.color("card-foreground");
        let title_at = if let Some(icon) = self.icon {
            let square = Rect::from_min_size(pos2(x, y), vec2(32.0, 32.0));
            painter.rect_filled(square, t.radius("lg"), t.color("muted"));
            crate::icons::paint(&painter, square.center(), icon, 16.0, t.color("foreground"));
            // a title of one line stands level with the tile's middle
            let at = if title_height < 32.0 {
                y + (32.0 - title_height) / 2.0
            } else {
                y
            };
            pos2(x + lead, at)
        } else {
            pos2(x, y)
        };
        let title_rect = Rect::from_min_size(title_at, title.size());
        painter.galley(title_at, title, title_ink);
        if cut {
            let hint = ui.interact(title_rect, id.with("title"), Sense::hover());
            hint.on_hover_text(title_text);
        }
        // the check and the menu at the head's end
        let mut end = origin.x + width - 16.0;
        let mut menu_chosen = None;
        if let Some((items, chosen)) = self.menu {
            let button = Rect::from_min_size(pos2(end - 28.0, y), vec2(28.0, 28.0));
            let trigger = ui.interact(button, id.with("menu"), Sense::click());
            trigger.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, self.words.menu.as_str()));
            let lit = trigger.hovered() || egui::Popup::is_id_open(ui.ctx(), trigger.id.with("dropdown"));
            if lit {
                painter.rect_filled(button, t.radius("md"), t.color("hover"));
            }
            crate::icons::paint(
                &painter,
                button.center(),
                Icon::Ellipsis,
                16.0,
                t.color(if lit { "foreground" } else { "muted-foreground" }),
            );
            super::focus_ring(&painter, &t, button, t.radius("md"), &trigger);
            if let Some(index) = crate::DropdownMenu::new(items).width(176.0).end(true).show(&trigger) {
                menu_chosen = Some(index);
            }
            if menu_chosen.is_some() {
                *chosen = menu_chosen;
            }
            end -= 28.0 + 8.0;
        }
        if self.selected {
            let center = pos2(end - 10.0, y + 10.0_f32.max(head_height.min(28.0) / 2.0));
            painter.circle_filled(center, 10.0, t.color("primary"));
            crate::icons::paint(&painter, center, Icon::Check, 14.0, t.color("primary-foreground"));
        }
        y += head_height;
        if let Some(code) = code {
            y += 4.0;
            let h = code.size().y;
            painter.galley(pos2(x, y), code, t.color("muted-foreground"));
            y += h;
        }
        if let Some(status) = self.status {
            // a row that wraps, 8 apart, in the small muted words
            y += 8.0;
            let line = Rect::from_min_size(pos2(x, y), vec2(inner, 20.0));
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(line)
                    .layout(egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true)),
            );
            child.spacing_mut().item_spacing = vec2(8.0, 8.0);
            child.spacing_mut().interact_size.y = 20.0;
            status(&mut child);
            y += child.min_rect().height().max(20.0);
        }
        if let Some(description) = description {
            y += 8.0;
            let h = description.size().y;
            painter.galley(pos2(x, y), description, t.color("muted-foreground"));
            y += h;
        }
        let natural = y + 16.0 - origin.y + foot;
        let rect = Rect::from_min_size(origin, vec2(width, height.max(natural)));
        ui.allocate_rect(rect, Sense::hover());
        if (rect.height() - last).abs() > 0.5 {
            ui.data_mut(|d| d.insert_temp(id.with("height"), rect.height()));
            ui.ctx().request_repaint();
        }
        painter.set(
            ground_slot,
            egui::Shape::rect_filled(rect, radius, if hot { t.color("hover") } else { t.color("card") }),
        );
        if !self.facts.is_empty() {
            let top = rect.bottom() - foot;
            painter.hline(rect.x_range(), top + 0.5, Stroke::new(1.0, t.color("border")));
            for (i, fact) in self.facts.iter().enumerate() {
                let fx = x + (i % fact_columns) as f32 * (fact_width + 12.0);
                let fy = top + 1.0 + 10.0 + (i / fact_columns) as f32 * (36.0 + 12.0);
                let one_line = |text: &str, role: TextRole| {
                    WidgetText::from(role.rich(&t, text)).into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        fact_width.max(1.0),
                        role.font(&t),
                    )
                };
                painter.galley(
                    pos2(fx, fy),
                    one_line(&fact.label, TextRole::Help),
                    t.color("muted-foreground"),
                );
                painter.galley(
                    pos2(fx, fy + 16.0),
                    one_line(&fact.value, TextRole::Body),
                    t.color("card-foreground"),
                );
            }
        }
        // the edge: primary with a 1px primary ring while selected; firmer under the pointer
        if self.selected {
            painter.rect_stroke(
                rect,
                radius,
                Stroke::new(1.0, t.color("primary")),
                egui::StrokeKind::Inside,
            );
            painter.rect_stroke(
                rect,
                radius,
                Stroke::new(1.0, t.color("primary")),
                egui::StrokeKind::Outside,
            );
        } else {
            let edge = if hot {
                t.color_at("foreground", 0.25)
            } else {
                t.color("border-strong")
            };
            painter.rect_stroke(rect, radius, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
        }
        if interactive {
            super::focus_ring(&painter, &t, rect, radius, &response);
        }
        let mut response = response;
        response.rect = rect;
        response
    }
}

/// What a sortable grid was asked this frame.
#[derive(Default)]
pub struct SortableOutcome {
    /// The index of the item pressed.
    pub pressed: Option<usize>,
    /// An item moved from the first index to the second: by a drag let go, an arrow key, or
    /// Escape putting a picked-up card back.
    pub moved: Option<(usize, usize)>,
}

/// What takes the grab in a sortable grid: the whole card, or a grip the app draws in it with
/// [`SortableGrid::grip`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortableHandle {
    #[default]
    Item,
    Grip,
}

/// The words a sortable grid says.
#[derive(Clone, Debug)]
pub struct SortableWords {
    /// A grip's name: `Move harbor`.
    pub move_item: fn(&str) -> String,
    /// What a screen reader hears after a move: `harbor moved to 1 of 3`.
    pub moved: fn(&str, usize, usize) -> String,
}

impl Default for SortableWords {
    fn default() -> Self {
        Self {
            move_item: |item| format!("Move {item}"),
            moved: |item, place, count| {
                format!(
                    "{item} moved to {} of {}",
                    crate::numbers::locale_number(place as f64),
                    crate::numbers::locale_number(count as f64)
                )
            },
        }
    }
}

/// A grid's drag in progress: the card taken, where the pointer took it, and whether Escape put
/// it back (the release then moves nothing).
#[derive(Clone, Copy, Debug)]
struct SortDrag {
    from: usize,
    grab: egui::Vec2,
    cancelled: bool,
}

/// Items in a grid of columns at least `min_width` wide and 12 apart, filling the row, after
/// sokkel-ui's `SortableGrid`. A card is dragged as a whole (or by a grip the app draws in it): its
/// cell stays at 40 percent where it will land, the others moving aside, and a copy follows the
/// pointer; letting go moves it and Escape puts it back. With a card focused the arrows move it a
/// place, the focus following it, and Space or Enter picks it up (Escape then puts it back); a
/// screen reader hears each move. Each item is drawn by `item` into its cell at the row's height,
/// and answers with its press.
pub struct SortableGrid<'a> {
    id: egui::Id,
    count: usize,
    min_width: f32,
    label: String,
    item_label: Option<Box<dyn Fn(usize) -> String + 'a>>,
    handle: SortableHandle,
    kind: Option<String>,
    disabled: bool,
    words: SortableWords,
}

impl<'a> SortableGrid<'a> {
    pub fn new(id: impl egui::AsIdSalt, count: usize, label: impl Into<String>) -> Self {
        Self {
            id: egui::Id::unique(id),
            count,
            min_width: 192.0,
            label: label.into(),
            item_label: None,
            handle: SortableHandle::Item,
            kind: None,
            disabled: false,
            words: SortableWords::default(),
        }
    }

    /// The least width of a column; 192 (`12rem`) by default.
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = width;
        self
    }

    /// What a screen reader and a grip call the card at an index.
    pub fn item_label(mut self, label: impl Fn(usize) -> String + 'a) -> Self {
        self.item_label = Some(Box::new(label));
        self
    }

    /// The whole card takes the grab (the default), or a grip drawn with [`SortableGrid::grip`].
    pub fn handle(mut self, handle: SortableHandle) -> Self {
        self.handle = handle;
        self
    }

    /// What a target outside the grid takes its cards as: a dragged card carries a
    /// [`crate::DragData`] of this kind, its value the card's index.
    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    /// Nothing moves.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn words(mut self, words: SortableWords) -> Self {
        self.words = words;
        self
    }

    /// The grip of the card at `index` in the grid made from `id` (the same salt the grid takes),
    /// for a grid whose handle is [`SortableHandle::Grip`]: a grip glyph, 16, taking the grab and
    /// the keys, named `label`.
    pub fn grip(ui: &mut Ui, id: impl egui::AsIdSalt, index: usize, label: &str) -> Response {
        let t = tokens(ui.ctx());
        let grip_id = egui::Id::unique(id).with(("grip", index));
        let side = t.density("control-xs");
        let (rect, _) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
        let grip = ui.interact(rect, grip_id, Sense::click_and_drag());
        grip.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
        let hot = grip.hovered() || grip.dragged();
        if hot {
            ui.painter().rect_filled(rect, t.radius("control"), t.color("hover"));
            ui.ctx().set_cursor_icon(if grip.dragged() {
                egui::CursorIcon::Grabbing
            } else {
                egui::CursorIcon::Grab
            });
        }
        crate::icons::paint(
            ui.painter(),
            rect.center(),
            Icon::GripVertical,
            16.0,
            t.color(if hot { "foreground" } else { "muted-foreground" }),
        );
        super::focus_ring(ui.painter(), &t, rect, t.radius("control"), &grip);
        grip
    }

    fn label_of(&self, index: usize) -> String {
        self.item_label
            .as_ref()
            .map_or_else(|| (index + 1).to_string(), |label| label(index))
    }

    pub fn show(self, ui: &mut Ui, mut item: impl FnMut(&mut Ui, usize, f32, f32) -> Response) -> SortableOutcome {
        let gap = 12.0;
        let width = ui.available_width();
        let columns = (((width + gap) / (self.min_width + gap)).floor() as usize).max(1);
        let cell = (width - gap * (columns - 1) as f32) / columns as f32;
        let rows = self.count.div_ceil(columns);
        // each row as tall as its tallest card, as the web's grid stretches them
        let heights_id = self.id.with("heights");
        let mut heights: Vec<f32> = ui.data(|d| d.get_temp(heights_id)).unwrap_or_default();
        heights.resize(rows, 0.0);
        let drag_id = self.id.with("dragging");
        let picked_id = self.id.with("picked");
        let refocus_id = self.id.with("refocus");
        let said_id = self.id.with("said");
        let mut drag: Option<SortDrag> = ui.data(|d| d.get_temp(drag_id));
        let mut picked: Option<(usize, usize)> = ui.data(|d| d.get_temp(picked_id));
        let refocus: Option<usize> = ui.data_mut(|d| d.remove_temp(refocus_id));
        let mut said: Option<String> = ui.data(|d| d.get_temp(said_id));
        if self.disabled {
            drag = None;
            picked = None;
        }
        let mut out = SortableOutcome::default();
        let origin = ui.cursor().min;
        // the slots: each place in the grid, fixed whatever order the cards stand in
        let mut slot_top = Vec::with_capacity(rows);
        let mut y = origin.y;
        for height in &heights {
            slot_top.push(y);
            y += height + gap;
        }
        let slot = |index: usize| {
            let (row, column) = (index / columns, index % columns);
            Rect::from_min_size(
                pos2(origin.x + column as f32 * (cell + gap), slot_top[row]),
                vec2(cell, heights[row]),
            )
        };
        let pointer = ui.input(|i| i.pointer.interact_pos());
        // while a card is dragged, the slot under the pointer is where it lands
        let target = drag.filter(|d| !d.cancelled).map(|d| {
            pointer
                .and_then(|p| (0..self.count).find(|i| slot(*i).expand(gap / 2.0).contains(p)))
                .unwrap_or(d.from)
        });
        // the order drawn: the dragged card moved to where it lands, the others aside
        let mut order: Vec<usize> = (0..self.count).collect();
        if let (Some(d), Some(to)) = (drag, target) {
            let taken = order.remove(d.from);
            order.insert(to.min(order.len()), taken);
        }
        let mut measured = vec![0.0_f32; rows];
        let mut focus_ids = Vec::with_capacity(self.count);
        let mut lifted_rect = None;
        for (place, &index) in order.iter().enumerate() {
            let at = slot(place);
            let row = place / columns;
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(self.id.with(index))
                    .max_rect(Rect::from_min_size(at.min, vec2(cell, f32::INFINITY))),
            );
            let dragging = drag.is_some_and(|d| d.from == index && !d.cancelled);
            if dragging {
                child.set_opacity(0.4);
            }
            let response = item(&mut child, index, cell, heights[row]);
            measured[row] = measured[row].max(response.rect.height());
            if dragging {
                lifted_rect = Some(response.rect);
            }
            let label = self.label_of(index);
            // the grab: the whole card, or the app's grip
            let (grab, focus_id) = match self.handle {
                SortableHandle::Item => {
                    let id = self.id.with(("cell", index));
                    let sense = if self.disabled {
                        Sense::hover()
                    } else {
                        Sense::click_and_drag()
                    };
                    let grab = ui.interact(response.rect, id, sense);
                    let name = (self.words.move_item)(&label);
                    grab.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ListItem, !self.disabled, &name));
                    if !self.disabled && grab.hovered() && drag.is_none() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                    }
                    if grab.clicked_by(egui::PointerButton::Primary) || response.clicked() {
                        out.pressed = Some(index);
                    }
                    if grab.has_focus() {
                        super::focus_ring(
                            ui.painter(),
                            &tokens(ui.ctx()),
                            response.rect,
                            tokens(ui.ctx()).radius("surface"),
                            &grab,
                        );
                    }
                    (Some(grab), id)
                }
                SortableHandle::Grip => {
                    if response.clicked() {
                        out.pressed = Some(index);
                    }
                    let id = self.id.with(("grip", index));
                    (ui.ctx().read_response(id), id)
                }
            };
            focus_ids.push((index, focus_id));
            if let Some(grab) = grab
                && !self.disabled
                && grab.drag_started()
            {
                let grabbed_at = pointer.unwrap_or(response.rect.min);
                drag = Some(SortDrag {
                    from: index,
                    grab: grabbed_at - response.rect.min,
                    cancelled: false,
                });
                if let Some(kind) = &self.kind {
                    egui::DragAndDrop::set_payload(
                        ui.ctx(),
                        crate::DragData {
                            kind: kind.clone(),
                            label: label.clone(),
                            duration: None,
                            value: index.to_string(),
                        },
                    );
                }
            }
        }
        let bottom = origin.y + measured.iter().sum::<f32>() + gap * rows.saturating_sub(1) as f32;
        let (grid_rect, grid) = ui.allocate_exact_size(vec2(width, (bottom - origin.y).max(0.0)), Sense::hover());
        let _ = grid_rect;
        grid.widget_info(|| egui::WidgetInfo::labeled(egui::Role::List, true, &self.label));
        // the copy that follows the pointer
        if let (Some(d), Some(rect), Some(p)) = (drag.filter(|d| !d.cancelled), lifted_rect, pointer) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            let at = p - d.grab;
            egui::Area::new(self.id.with("lifted"))
                .order(egui::Order::Tooltip)
                .fixed_pos(at)
                .interactable(false)
                .show(ui.ctx(), |ui| {
                    ui.set_width(rect.width());
                    let row = (0..self.count).position(|i| i == d.from).map_or(0, |i| i / columns);
                    item(ui, d.from, rect.width(), heights.get(row).copied().unwrap_or(0.0));
                });
            ui.ctx().request_repaint();
        }
        // Escape puts a dragged card back; letting go moves it where it lands
        if let Some(d) = drag.as_mut() {
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                d.cancelled = true;
            }
            if ui.input(|i| i.pointer.any_released()) {
                if !d.cancelled
                    && let Some(to) = target.filter(|to| *to != d.from)
                {
                    out.moved = Some((d.from, to));
                    said = Some((self.words.moved)(&self.label_of(d.from), to + 1, self.count));
                }
                drag = None;
            }
        }
        // the keys, on the focused card (or grip)
        if !self.disabled && self.count > 0 {
            let focused = focus_ids
                .iter()
                .find(|(_, id)| ui.memory(|m| m.has_focus(*id)))
                .map(|(index, _)| *index);
            if let Some(at) = focused {
                let focus_of = |index: usize| focus_ids.iter().find(|(i, _)| *i == index).map(|(_, id)| *id);
                if let Some(id) = focus_of(at) {
                    ui.memory_mut(|m| {
                        m.set_focus_lock_filter(
                            id,
                            egui::EventFilter {
                                horizontal_arrows: true,
                                vertical_arrows: true,
                                escape: true,
                                ..Default::default()
                            },
                        )
                    });
                }
                let none = egui::Modifiers::NONE;
                let (back, on, take, escape) = ui.input_mut(|i| {
                    (
                        i.consume_key(none, egui::Key::ArrowUp) || i.consume_key(none, egui::Key::ArrowLeft),
                        i.consume_key(none, egui::Key::ArrowDown) || i.consume_key(none, egui::Key::ArrowRight),
                        i.consume_key(none, egui::Key::Space) || i.consume_key(none, egui::Key::Enter),
                        picked.is_some() && i.consume_key(none, egui::Key::Escape),
                    )
                });
                let step = if back && at > 0 {
                    Some(at - 1)
                } else if on && at + 1 < self.count {
                    Some(at + 1)
                } else {
                    None
                };
                if let Some(to) = step {
                    out.moved = Some((at, to));
                    said = Some((self.words.moved)(&self.label_of(at), to + 1, self.count));
                    if let Some((from, _)) = picked {
                        picked = Some((from, to));
                    }
                    // the focus follows the card to its new place
                    ui.data_mut(|d| d.insert_temp(refocus_id, to));
                } else if take {
                    picked = match picked {
                        Some(_) => None,
                        None => Some((at, at)),
                    };
                } else if escape && let Some((from, _)) = picked {
                    if from != at {
                        out.moved = Some((at, from));
                        said = Some((self.words.moved)(&self.label_of(at), from + 1, self.count));
                        ui.data_mut(|d| d.insert_temp(refocus_id, from));
                    }
                    picked = None;
                }
            }
            if let Some(index) = refocus
                && let Some((_, id)) = focus_ids.iter().find(|(i, _)| *i == index)
            {
                ui.memory_mut(|m| m.request_focus(*id));
            }
        }
        // what a screen reader hears after a move: a polite live region
        if let Some(words) = &said {
            let status = ui.interact(
                Rect::from_min_size(origin, vec2(0.0, 0.0)),
                self.id.with("status"),
                Sense::hover(),
            );
            status.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Status, true, words));
            ui.ctx().accesskit_node_builder(status.id, |node| {
                node.set_live(egui::accesskit::Live::Polite);
            });
        }
        ui.data_mut(|d| {
            d.insert_temp(heights_id, measured);
            match drag {
                Some(state) => {
                    d.insert_temp(drag_id, state);
                }
                None => d.remove::<SortDrag>(drag_id),
            }
            match picked {
                Some(state) => {
                    d.insert_temp(picked_id, state);
                }
                None => d.remove::<(usize, usize)>(picked_id),
            }
            if let Some(words) = &said {
                d.insert_temp(said_id, words.clone());
            }
        });
        if drag.is_none() && self.kind.is_some() && ui.input(|i| i.pointer.any_released()) {
            egui::DragAndDrop::clear_payload(ui.ctx());
        }
        out
    }
}
