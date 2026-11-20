use egui::{
    Color32, CursorIcon, Key, Modifiers, Rect, Response, Sense, Stroke, StrokeKind, TextWrapMode, Ui, Widget,
    WidgetText, pos2, vec2,
};

use crate::{DropdownMenu, Icon, MenuItem, TextRole, Tokens, icons, tokens};

/// How a chip says what it says, as sokkel-ui's `Chip` `tone`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChipTone {
    #[default]
    Neutral,
    Good,
    Critical,
    Warning,
    Info,
    /// Only edged, in `border`.
    Outline,
    /// A value that is not there: dashed in `border-strong` around muted words.
    Absent,
    /// A status token's wash and ink: `Status("waiting")` draws `status-waiting` behind
    /// `status-waiting-ink`, as sokkel-ui's `StatusChip` colors a state.
    Status(&'static str),
    /// A kind color, the same as [`Chip::hue`] with that kind: `Kind("teal")` is `ChipHue::Teal`.
    Kind(&'static str),
}

impl ChipTone {
    /// The tone's name in sokkel-design's `parts/chip` variants, for the semantic tones.
    fn key(self) -> Option<&'static str> {
        Some(match self {
            Self::Neutral => "neutral",
            Self::Good => "good",
            Self::Critical => "critical",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Outline => "outline",
            Self::Absent => "absent",
            Self::Status(_) | Self::Kind(_) => return None,
        })
    }
}

/// A category's color, worn in place of the tone, as sokkel-ui's `ChipHue`: one of the chart
/// palette's five, set by a theme with the charts', or a kind color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChipHue {
    Chart1,
    Chart2,
    Chart3,
    Chart4,
    Chart5,
    Blue,
    Green,
    Jade,
    Pink,
    Teal,
    Violet,
}

impl ChipHue {
    pub const ALL: [Self; 11] = [
        Self::Chart1,
        Self::Chart2,
        Self::Chart3,
        Self::Chart4,
        Self::Chart5,
        Self::Blue,
        Self::Green,
        Self::Jade,
        Self::Pink,
        Self::Teal,
        Self::Violet,
    ];

    /// The hue's name in sokkel-design's `parts/chip` variants (`hue-<key>`).
    pub fn key(self) -> &'static str {
        match self {
            Self::Chart1 => "1",
            Self::Chart2 => "2",
            Self::Chart3 => "3",
            Self::Chart4 => "4",
            Self::Chart5 => "5",
            Self::Blue => "blue",
            Self::Green => "green",
            Self::Jade => "jade",
            Self::Pink => "pink",
            Self::Teal => "teal",
            Self::Violet => "violet",
        }
    }

    /// The kind color by its name (`blue`, `teal`, ...).
    pub fn kind(name: &str) -> Option<Self> {
        Self::ALL.into_iter().skip(5).find(|hue| hue.key() == name)
    }
}

/// How tall a chip is, as sokkel-ui's `ChipSize`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChipSize {
    /// 20 tall in `text.badge`, in a line of text or a table cell.
    #[default]
    Default,
    /// As tall as a field or a button (`density.control`), a finger tall on a phone, in `text.xs`:
    /// a chip that stands alone as a control.
    Control,
}

/// Where a chip's menu lines up under it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChipMenuAlign {
    #[default]
    Start,
    Center,
    End,
}

/// The words a chip says, English by default.
#[derive(Clone, Debug)]
pub struct ChipWords {
    /// The remove button's name and tooltip (`chip.remove`).
    pub remove: String,
    /// Said after an external link chip's words, for a screen reader (`chip.newTab`).
    pub new_tab: String,
}

impl Default for ChipWords {
    fn default() -> Self {
        Self {
            remove: "Remove".into(),
            new_tab: "opens in a new tab".into(),
        }
    }
}

/// What a chip's presses did this frame.
pub struct ChipResponse {
    /// The face's response when it is a button, a link or a menu's trigger, else the chip's.
    pub response: Response,
    /// The face was pressed (a button chip's select, a toggle's flip, a link followed).
    pub pressed: bool,
    /// The remove button was pressed.
    pub removed: bool,
    /// The menu entry chosen, by its index.
    pub chosen: Option<usize>,
}

#[derive(Clone)]
enum Face {
    Plain,
    Button,
    Link { href: String, external: bool },
    Menu { items: Vec<MenuItem>, align: ChipMenuAlign },
}

/// A short value as a chip, after sokkel-ui's `Chip`: a tag on a job, a state with no run color, a
/// value that is absent. Its tone is neutral, a semantic color's wash with its ink, outline or
/// absent (dashed); [`Chip::hue`] gives it a category's color instead. It may be a button
/// ([`Chip::button`], a toggle with [`Chip::selected`]), a link ([`Chip::link`]), a menu's trigger
/// ([`Chip::menu`]), and may end in a remove button ([`Chip::removable`]). No press on it reaches a
/// row or a card around it. Show it with [`Chip::show`] to read its presses; `ui.add` returns the
/// face's response.
#[must_use = "add it with `ui.add(...)` or show it with `.show(ui)`"]
pub struct Chip {
    label: String,
    tone: ChipTone,
    hue: Option<ChipHue>,
    size: ChipSize,
    mono: bool,
    icon: Option<Icon>,
    title: Option<String>,
    select_label: Option<String>,
    selected: bool,
    face: Face,
    removable: bool,
    words: ChipWords,
    id_salt: Option<egui::Id>,
    default_open: bool,
}

impl Chip {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            tone: ChipTone::Neutral,
            hue: None,
            size: ChipSize::Default,
            mono: false,
            icon: None,
            title: None,
            select_label: None,
            selected: false,
            face: Face::Plain,
            removable: false,
            words: ChipWords::default(),
            id_salt: None,
            default_open: false,
        }
    }

    pub fn tone(mut self, tone: ChipTone) -> Self {
        self.tone = tone;
        self
    }

    /// A category's color in place of the tone: its wash at 12%, its edge at 30%, its ink.
    pub fn hue(mut self, hue: ChipHue) -> Self {
        self.hue = Some(hue);
        self
    }

    pub fn size(mut self, size: ChipSize) -> Self {
        self.size = size;
        self
    }

    /// The words in the code face, for an id, a tag or a key.
    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    /// Drawn before the words, at the chip's icon size (12, or 14 at `Control`).
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Said on hover: the whole value when the chip cuts it short, or what it means.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Makes the face a button; its press is [`ChipResponse::pressed`].
    pub fn button(mut self) -> Self {
        self.face = Face::Button;
        self
    }

    /// The button's or the link's name and tooltip, when a press does more than the words say
    /// ("Filter by harbor"); the words by default.
    pub fn select_label(mut self, label: impl Into<String>) -> Self {
        self.select_label = Some(label.into());
        self
    }

    /// A toggle's state: on a button chip it says it is pressed, and the chip wears an edge in its
    /// own ink over a stronger wash.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Makes the face a link to `href`, opened in this tab or, `external`, in a new one with the
    /// external mark after the words.
    pub fn link(mut self, href: impl Into<String>, external: bool) -> Self {
        self.face = Face::Link {
            href: href.into(),
            external,
        };
        self
    }

    /// Makes the face a menu's trigger with a chevron after the words; a press (or ArrowDown)
    /// opens `items` under the chip. It takes the place of [`Chip::button`] and [`Chip::link`].
    pub fn menu(mut self, items: impl IntoIterator<Item = MenuItem>, align: ChipMenuAlign) -> Self {
        self.face = Face::Menu {
            items: items.into_iter().collect(),
            align,
        };
        self
    }

    /// Open the menu the first time it is shown, as a story shows it.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Ends the chip in a remove button named by `words.remove`; its press is
    /// [`ChipResponse::removed`].
    pub fn removable(mut self, removable: bool) -> Self {
        self.removable = removable;
        self
    }

    pub fn words(mut self, words: ChipWords) -> Self {
        self.words = words;
        self
    }

    /// The id its focus and its menu are kept under, when the chip's place among its neighbors
    /// may move.
    pub fn id_salt(mut self, salt: impl egui::AsIdSalt) -> Self {
        self.id_salt = Some(egui::Id::unique(salt));
        self
    }

    pub fn show(self, ui: &mut Ui) -> ChipResponse {
        let t = tokens(ui.ctx());
        let control = self.size == ChipSize::Control;
        let height = if control {
            super::finger_tall(ui.ctx(), &t, t.density("control"))
        } else {
            20.0
        };
        let pad = if control { 10.0 } else { 6.0 };
        let icon_size = if control { 14.0 } else { 12.0 };
        let role = if control { TextRole::Help } else { TextRole::Badge };
        let trail = match &self.face {
            Face::Menu { .. } | Face::Link { external: true, .. } => 4.0 + icon_size,
            _ => 0.0,
        };
        let lead = if self.icon.is_some() { icon_size + 4.0 } else { 0.0 };
        // the remove button: its icon padded 2 all round, 2 in from the chip's end on each side
        let remove_w = if self.removable {
            2.0 + icon_size + 4.0 + 2.0
        } else {
            0.0
        };
        let end = if self.removable { remove_w } else { pad };
        let fixed = pad + lead + trail + end;
        let text = |wrap: TextWrapMode, room: f32| {
            let rich = if self.mono {
                egui::RichText::new(&self.label).font(crate::code_font(ui.ctx(), &t, role))
            } else {
                role.rich(&t, &self.label)
            };
            let font = role.font(&t);
            WidgetText::from(rich).into_galley(ui, Some(wrap), room, font)
        };
        let natural = text(TextWrapMode::Extend, f32::INFINITY);
        let room = ui.available_width().max(fixed + 8.0);
        let galley = if fixed + natural.size().x > room {
            text(TextWrapMode::Truncate, room - fixed)
        } else {
            natural
        };
        let width = fixed + galley.size().x;
        let (rect, chip) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let id = self.id_salt.unwrap_or(chip.id);
        let chip = match &self.title {
            Some(title) => chip.on_hover_text(title),
            None => chip,
        };
        let named = self.select_label.clone().unwrap_or_else(|| self.label.clone());

        // the face: the press, as tall as the chip
        let face_rect = Rect::from_min_max(
            rect.min,
            pos2(
                rect.right() - if self.removable { remove_w } else { 0.0 },
                rect.bottom(),
            ),
        );
        let face = match &self.face {
            Face::Plain => None,
            _ => {
                let face = ui.interact(face_rect, id.with("face"), Sense::click());
                let (kind, words) = match &self.face {
                    Face::Link { external: true, .. } => {
                        (egui::Role::Link, format!("{named} ({})", self.words.new_tab))
                    }
                    Face::Link { .. } => (egui::Role::Link, named.clone()),
                    Face::Menu { .. } => (egui::Role::Button, named.clone()),
                    _ => (egui::Role::Button, named.clone()),
                };
                let toggle = matches!(self.face, Face::Button) && self.selected;
                face.widget_info(|| {
                    let mut info = egui::WidgetInfo::labeled(kind, true, &words);
                    if toggle {
                        info.selected = Some(true);
                    }
                    info
                });
                if face.hovered() {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                let face = match &self.select_label {
                    Some(label) => face.on_hover_text(label),
                    None => face,
                };
                Some(face)
            }
        };
        let remove = self.removable.then(|| {
            let size = icon_size + 4.0;
            let r = Rect::from_center_size(pos2(rect.right() - 2.0 - size / 2.0, rect.center().y), vec2(size, size));
            let hit = ui.interact(r, id.with("remove"), Sense::click());
            let words = &self.words.remove;
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, words));
            if hit.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            hit.on_hover_text(&self.words.remove)
        });

        // Enter and Space press what has the focus and stop there, so no row around it sees them
        let hold = |response: &Response| {
            if response.has_focus() {
                ui.input_mut(|i| {
                    i.consume_key(Modifiers::NONE, Key::Enter);
                    i.consume_key(Modifiers::NONE, Key::Space);
                });
            }
        };
        let mut pressed = false;
        let mut chosen = None;
        if let Some(face) = &face {
            pressed = face.clicked();
            match &self.face {
                Face::Link { href, external } if pressed => {
                    let url = if *external {
                        egui::OpenUrl::new_tab(href)
                    } else {
                        egui::OpenUrl::same_tab(href)
                    };
                    ui.ctx().open_url(url);
                }
                Face::Menu { items, align } => {
                    let popup = face.id.with("menu");
                    if face.has_focus() && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)) {
                        egui::Popup::open_id(ui.ctx(), popup);
                    }
                    chosen = DropdownMenu::new(items)
                        .width(0.0)
                        .end(*align == ChipMenuAlign::End)
                        .center(*align == ChipMenuAlign::Center)
                        .default_open(self.default_open)
                        .show(face);
                }
                _ => {}
            }
            hold(face);
        }
        let removed = remove.as_ref().is_some_and(Response::clicked);
        if let Some(remove) = &remove {
            hold(remove);
        }

        if ui.is_rect_visible(rect) {
            let look = look(&t, self.tone, self.hue, self.selected);
            let painter = ui.painter();
            let radius = t.radius("chip");
            painter.rect(rect, radius, look.fill, Stroke::NONE, StrokeKind::Inside);
            if look.dashed {
                dashed_round_rect(painter, rect, radius, Stroke::new(1.0, look.edge));
            } else {
                painter.rect_stroke(rect, radius, Stroke::new(1.0, look.edge), StrokeKind::Inside);
            }
            let mut x = rect.left() + pad;
            if let Some(icon) = self.icon {
                icons::paint(
                    painter,
                    pos2(x + icon_size / 2.0, rect.center().y),
                    icon,
                    icon_size,
                    look.ink,
                );
                x += icon_size + 4.0;
            }
            let text_rect = Rect::from_min_size(pos2(x, rect.center().y - galley.size().y / 2.0), galley.size());
            let baseline = galley
                .rows
                .first()
                .and_then(|row| row.row.glyphs.first().map(|g| text_rect.top() + row.pos.y + g.pos.y));
            painter.galley(text_rect.min, galley, look.ink);
            if let (Some(face), Some(baseline)) = (&face, baseline)
                && face.hovered()
            {
                // `hover:underline underline-offset-2`
                let y = (baseline + 3.0).round() + 0.5;
                painter.hline(text_rect.x_range(), y, Stroke::new(1.0, look.ink));
            }
            let trail_icon = match &self.face {
                Face::Menu { .. } => Some(Icon::ChevronDown),
                Face::Link { external: true, .. } => Some(Icon::ArrowUpRight),
                _ => None,
            };
            if let Some(icon) = trail_icon {
                let center = pos2(text_rect.right() + 4.0 + icon_size / 2.0, rect.center().y);
                icons::paint(painter, center, icon, icon_size, look.ink.gamma_multiply(0.7));
            }
            if let Some(face) = &face {
                focus_outline(painter, &t, face_rect, radius, face);
            }
            if let Some(remove) = &remove {
                let state = if remove.hovered() { "hover" } else { "rest" };
                let paint = t.paint(&format!("chip.base.{state}.remove")).unwrap_or_default();
                let mini = t.radius("mini");
                if let Some(ground) = paint.ground {
                    painter.rect_filled(remove.rect, mini, ground);
                }
                let ink = look.ink.gamma_multiply(paint.opacity.unwrap_or(1.0));
                icons::paint(painter, remove.rect.center(), Icon::X, icon_size, ink);
                focus_outline(painter, &t, remove.rect, mini, remove);
            }
        }
        ChipResponse {
            response: face.unwrap_or(chip),
            pressed,
            removed,
            chosen,
        }
    }
}

impl Widget for Chip {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

/// The focus outline a chip's face and remove button wear, as `chip.base.focus.face` paints it.
fn focus_outline(painter: &egui::Painter, t: &Tokens, rect: Rect, radius: egui::CornerRadius, response: &Response) {
    if !response.has_focus() {
        return;
    }
    let (color, width, offset) = t
        .paint("chip.base.focus.face")
        .and_then(|paint| paint.outline)
        .unwrap_or((t.color("ring"), t.length("focus-ring-width"), 1.0));
    let outer = egui::CornerRadius::from(radius.nw.saturating_add(offset as u8));
    painter.rect_stroke(
        rect.expand(offset),
        outer,
        Stroke::new(width, color),
        StrokeKind::Outside,
    );
}

/// A rounded rectangle's edge drawn dashed, as `border-dashed` draws it.
fn dashed_round_rect(painter: &egui::Painter, rect: Rect, radius: egui::CornerRadius, stroke: Stroke) {
    let mut points = Vec::new();
    egui::epaint::tessellator::path::rounded_rectangle(
        &mut points,
        egui::epaint::RoundedRect::new(rect.shrink(0.5), radius),
    );
    if let Some(first) = points.first().copied() {
        points.push(first);
    }
    painter.extend(egui::Shape::dashed_line(&points, stroke, 3.0, 3.0));
}

/// A chip's colors: its wash, its ink and its edge, and whether the edge is dashed.
struct Look {
    fill: Color32,
    ink: Color32,
    edge: Color32,
    dashed: bool,
}

/// The chip's paint from sokkel-design's `chip.<variant>.<state>.container`, by tone or hue and
/// whether a toggle is on; a status tone reads the status tokens, which the spec does not name.
fn look(t: &Tokens, tone: ChipTone, hue: Option<ChipHue>, selected: bool) -> Look {
    let hue = hue.or(match tone {
        ChipTone::Kind(name) => ChipHue::kind(name),
        _ => None,
    });
    let none = Color32::TRANSPARENT;
    if let ChipTone::Status(key) = tone
        && hue.is_none()
    {
        let wash = format!("status-{key}");
        let ink = t.color(&format!("{wash}-ink"));
        return Look {
            fill: t.color_at(&wash, if selected { 0.3 } else { 0.15 }),
            ink,
            edge: if selected { ink } else { none },
            dashed: false,
        };
    }
    let variant = match hue {
        Some(hue) => format!("hue-{}", hue.key()),
        None => tone.key().unwrap_or("neutral").to_owned(),
    };
    let chosen = selected
        .then(|| t.paint(&format!("chip.{variant}.selected.container")))
        .flatten();
    let paint = chosen.unwrap_or_else(|| t.paint(&format!("chip.{variant}.rest.container")).unwrap_or_default());
    let ink = paint.ink.unwrap_or_else(|| t.color("foreground"));
    let mut look = Look {
        fill: paint.ground.unwrap_or(none),
        ink,
        edge: paint.edge.unwrap_or(none),
        dashed: tone == ChipTone::Absent && hue.is_none(),
    };
    // a toggle with no chosen paint of its own: edged in its ink, a hue over its ink at 20%
    if selected && chosen.is_none() {
        look.edge = ink;
        if hue.is_some() {
            look.fill = ink.gamma_multiply(0.2);
        }
    }
    look
}

/// Paint a chip at rest in `rect` with its words laid out in `galley`, 6 in from its start.
fn paint(painter: &egui::Painter, t: &Tokens, rect: Rect, galley: std::sync::Arc<egui::Galley>, tone: ChipTone) {
    let look = look(t, tone, None, false);
    let radius = t.radius("chip");
    painter.rect(rect, radius, look.fill, Stroke::NONE, StrokeKind::Inside);
    if look.dashed {
        dashed_round_rect(painter, rect, radius, Stroke::new(1.0, look.edge));
    } else {
        painter.rect_stroke(rect, radius, Stroke::new(1.0, look.edge), StrokeKind::Inside);
    }
    painter.galley(
        pos2(rect.left() + 6.0, rect.center().y - galley.size().y / 2.0),
        galley,
        look.ink,
    );
}

/// Paint a chip with its start at `left` on the line through `center_y`, with no widget of its own,
/// for a part that draws many (a table's cells); returns its width.
pub(crate) fn paint_at(ui: &Ui, left: f32, center_y: f32, text: &str, tone: ChipTone) -> f32 {
    let t = tokens(ui.ctx());
    let role = TextRole::Badge;
    let galley =
        WidgetText::from(role.rich(&t, text)).into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, role.font(&t));
    let rect = egui::Rect::from_min_size(pos2(left, center_y - 10.0), vec2(galley.size().x + 12.0, 20.0));
    paint(ui.painter(), &t, rect, galley, tone);
    rect.width()
}

/// Paint a tag (sokkel-ui's `TagChips`): a chip in `tone` in the code face in the badge's text, its
/// start at `left` on the line through `center_y`; returns its width.
pub(crate) fn paint_tag_at(ui: &Ui, left: f32, center_y: f32, text: &str, tone: ChipTone) -> f32 {
    let t = tokens(ui.ctx());
    // in the chip's `text-badge`, its size and weight, as the web's mono chip
    let mono = crate::code_font(ui.ctx(), &t, TextRole::Badge);
    let galley = WidgetText::from(egui::RichText::new(text).font(mono.clone())).into_galley(
        ui,
        Some(TextWrapMode::Extend),
        f32::INFINITY,
        mono,
    );
    let rect = egui::Rect::from_min_size(pos2(left, center_y - 10.0), vec2(galley.size().x + 12.0, 20.0));
    paint(ui.painter(), &t, rect, galley, tone);
    rect.width()
}

/// Tags, after sokkel-ui's `TagChips`: each a neutral chip in the code face in the badge's text,
/// 20 tall, 4 apart, wrapping onto lines 4 apart where they run out of room.
pub struct TagChips<'a> {
    tags: &'a [String],
}

impl<'a> TagChips<'a> {
    pub fn new(tags: &'a [String]) -> Self {
        Self { tags }
    }
}

impl egui::Widget for TagChips<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = tokens(ui.ctx());
        let mono = crate::code_font(ui.ctx(), &t, TextRole::Badge);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            ui.spacing_mut().interact_size.y = 20.0;
            for tag in self.tags {
                let width = ui
                    .painter()
                    .layout_no_wrap(tag.clone(), mono.clone(), t.color("foreground"))
                    .size()
                    .x
                    + 12.0;
                let (rect, response) = ui.allocate_exact_size(vec2(width, 20.0), egui::Sense::hover());
                response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, tag));
                paint_tag_at(ui, rect.left(), rect.center().y, tag, ChipTone::Neutral);
            }
        })
        .response
    }
}
