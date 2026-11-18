//! A one-line value in the code face, after sokkel-ui's `InlineCode`.

use egui::{Response, Sense, Ui, Widget, pos2, vec2};

use crate::{TextRole, tokens};

/// The ink and wash of an inline code value: plain, muted, or one of the status tones.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InlineCodeTone {
    #[default]
    Neutral,
    Muted,
    Critical,
    Warning,
    Info,
    Good,
}

/// A one-line value in the code face on a faint wash, as sokkel-ui's `InlineCode`: a run id, a
/// host, a path, a command. It never wraps; a value wider than its room is cut short with an
/// ellipsis, the whole of it said while pointed at. `bare` drops the wash and its 4 of padding,
/// leaving the tone's ink; `role` sets the face's size, the body's by default.
#[must_use = "add it with `ui.add(...)`"]
pub struct InlineCode {
    value: String,
    tone: InlineCodeTone,
    bare: bool,
    role: TextRole,
}

impl InlineCode {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            tone: InlineCodeTone::Neutral,
            bare: false,
            role: TextRole::Body,
        }
    }

    pub fn tone(mut self, tone: InlineCodeTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn bare(mut self, bare: bool) -> Self {
        self.bare = bare;
        self
    }

    /// The size of the text it stands among: `Help` for the web's `xs`.
    pub fn role(mut self, role: TextRole) -> Self {
        self.role = role;
        self
    }
}

impl Widget for InlineCode {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let (ink, wash) = match self.tone {
            InlineCodeTone::Neutral => (t.color("foreground"), t.color("muted")),
            InlineCodeTone::Muted => (t.color("muted-foreground"), t.color("muted")),
            InlineCodeTone::Critical => (t.color("critical-ink"), t.color_at("critical", 0.12)),
            InlineCodeTone::Warning => (t.color("warning-ink"), t.color_at("warning", 0.12)),
            InlineCodeTone::Info => (t.color("info-ink"), t.color_at("info", 0.12)),
            InlineCodeTone::Good => (t.color("good-ink"), t.color_at("good", 0.12)),
        };
        let spec = t.text(self.role.key());
        let font = crate::mono_font(ui.ctx(), crate::mono_size(&t, spec.size), spec.weight);
        let pad = if self.bare { 0.0 } else { 4.0 };
        let mut job = egui::text::LayoutJob::simple_singleline(self.value.clone(), font, ink);
        job.wrap = egui::text::TextWrapping::truncate_at_width((ui.available_width() - 2.0 * pad).max(1.0));
        let galley = ui.painter().layout_job(job);
        let height = spec.line_height.max(galley.size().y);
        let (rect, response) = ui.allocate_exact_size(vec2(galley.size().x + 2.0 * pad, height), Sense::hover());
        if !self.bare {
            ui.painter().rect_filled(rect, t.radius("mini"), wash);
        }
        let cut = galley.elided;
        ui.painter().galley(
            pos2(rect.left() + pad, rect.center().y - galley.size().y / 2.0),
            galley,
            ink,
        );
        if cut {
            response.on_hover_text(self.value)
        } else {
            response
        }
    }
}
