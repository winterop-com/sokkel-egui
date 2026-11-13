use egui::{Color32, Response, Sense, Stroke, StrokeKind, TextWrapMode, Ui, Widget, WidgetText, pos2, vec2};

use crate::{TextRole, tokens};

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
        let (wash, rest, chosen, ink) = match self.tone {
            ChipTone::Neutral => (
                "faint",
                0.15,
                0.35,
                if self.selected {
                    "foreground"
                } else {
                    "muted-foreground"
                },
            ),
            ChipTone::Good => ("good", 0.15, 0.3, "good-ink"),
            ChipTone::Critical => ("critical", 0.15, 0.3, "critical-ink"),
            ChipTone::Warning => ("warning", 0.15, 0.3, "warning-ink"),
            ChipTone::Info => ("info", 0.15, 0.3, "info-ink"),
            ChipTone::Outline => ("faint", 0.0, 0.25, "foreground"),
        };
        let ink = t.color(ink);
        let fill = t.color_at(wash, if self.selected { chosen } else { rest });
        let border = match (self.selected, self.tone) {
            (true, _) => ink,
            (false, ChipTone::Outline) => t.color("border"),
            (false, _) => Color32::TRANSPARENT,
        };
        let role = TextRole::Badge;
        let galley = WidgetText::from(role.rich(&t, &self.label)).into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            role.font(&t),
        );
        let (rect, response) = ui.allocate_exact_size(vec2(galley.size().x + 12.0, 20.0), Sense::hover());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &self.label));
        if ui.is_rect_visible(rect) {
            let radius = t.radius("chip");
            let painter = ui.painter();
            painter.rect(rect, radius, fill, Stroke::new(1.0, border), StrokeKind::Inside);
            painter.galley(
                pos2(rect.left() + 6.0, rect.center().y - galley.size().y / 2.0),
                galley,
                ink,
            );
        }
        response
    }
}
