use egui::{Color32, Response, Sense, Stroke, StrokeKind, TextWrapMode, Ui, Widget, WidgetText, pos2, vec2};

use crate::{TextRole, Tokens, tokens};

/// How a chip says what it says, as sokkel-ui's `Chip` `tone`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChipTone {
    #[default]
    Neutral,
    Good,
    Critical,
    Warning,
    Info,
    Outline,
    /// A status token's wash and ink: `Status("waiting")` draws `status-waiting` behind
    /// `status-waiting-ink`, as sokkel-ui's `StatusChip` colors a state.
    Status(&'static str),
    /// A categorical kind token, as sokkel-ui's `KIND_CHIP`: `Kind("teal")` draws `kind-teal` at
    /// 12% behind `kind-teal-ink`, edged in `kind-teal` at 30%.
    Kind(&'static str),
}

/// A short label in a line of text or a table cell, 20 tall: the tone's color at 15% behind its ink.
#[must_use = "add it with `ui.add(...)`"]
pub struct Chip {
    label: String,
    tone: ChipTone,
    selected: bool,
}

impl Chip {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            tone: ChipTone::Neutral,
            selected: false,
        }
    }

    pub fn tone(mut self, tone: ChipTone) -> Self {
        self.tone = tone;
        self
    }

    /// The stronger wash and the edge a chosen toggle wears.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl Widget for Chip {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let role = TextRole::Badge;
        let galley = WidgetText::from(role.rich(&t, &self.label)).into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            role.font(&t),
        );
        let (rect, response) = ui.allocate_exact_size(vec2(galley.size().x + 12.0, 20.0), Sense::hover());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &self.label));
        if ui.is_rect_visible(rect) {
            paint(ui.painter(), &t, rect, galley, self.tone, self.selected);
        }
        response
    }
}

/// A chip's colors: its wash, its ink and its edge, by tone and whether it is chosen.
fn colors(t: &Tokens, tone: ChipTone, selected: bool) -> (Color32, Color32, Color32) {
    let (wash, rest, chosen, ink) = match tone {
        ChipTone::Neutral => (
            "faint",
            0.15,
            0.35,
            if selected { "foreground" } else { "muted-foreground" },
        ),
        ChipTone::Good => ("good", 0.15, 0.3, "good-ink"),
        ChipTone::Critical => ("critical", 0.15, 0.3, "critical-ink"),
        ChipTone::Warning => ("warning", 0.15, 0.3, "warning-ink"),
        ChipTone::Info => ("info", 0.15, 0.3, "info-ink"),
        ChipTone::Outline => ("faint", 0.0, 0.25, "foreground"),
        ChipTone::Status(_) | ChipTone::Kind(_) => ("", 0.15, 0.3, ""),
    };
    if let ChipTone::Kind(key) = tone {
        let kind = format!("kind-{key}");
        let ink = t.color(&format!("{kind}-ink"));
        let fill = t.color_at(&kind, if selected { 0.3 } else { 0.12 });
        let edge = if selected { ink } else { t.color_at(&kind, 0.3) };
        return (fill, ink, edge);
    }
    let (ink, fill) = if let ChipTone::Status(key) = tone {
        let wash = format!("status-{key}");
        (
            t.color(&format!("{wash}-ink")),
            t.color_at(&wash, if selected { chosen } else { rest }),
        )
    } else {
        (t.color(ink), t.color_at(wash, if selected { chosen } else { rest }))
    };
    let edge = match (selected, tone) {
        (true, _) => ink,
        (false, ChipTone::Outline) => t.color("border"),
        (false, _) => Color32::TRANSPARENT,
    };
    (fill, ink, edge)
}

fn paint(
    painter: &egui::Painter,
    t: &Tokens,
    rect: egui::Rect,
    galley: std::sync::Arc<egui::Galley>,
    tone: ChipTone,
    selected: bool,
) {
    let (fill, ink, edge) = colors(t, tone, selected);
    painter.rect(rect, t.radius("chip"), fill, Stroke::new(1.0, edge), StrokeKind::Inside);
    painter.galley(
        pos2(rect.left() + 6.0, rect.center().y - galley.size().y / 2.0),
        galley,
        ink,
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
    paint(ui.painter(), &t, rect, galley, tone, false);
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
    paint(ui.painter(), &t, rect, galley, tone, false);
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
