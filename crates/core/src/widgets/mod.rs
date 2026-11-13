//! The primitives, each drawn from the tokens alone, after sokkel-ui's components of the same name.

mod button;
mod chip;
mod choice;
mod display;
mod frame;
mod switch;
mod text_field;

pub use button::{Button, ButtonSize, ButtonVariant};
pub use chip::{Chip, ChipTone};
pub use choice::{Checkbox, Segmented, SegmentedVariant, Tabs, TabsVariant};
pub use display::{Alert, Badge, BadgeVariant, Card, Separator, Skeleton};
pub use frame::{Rail, RailEntry, RailGroup, Shell, ShellPart, StatusLine, StatusTone, TopBar};
pub use switch::Switch;
pub use text_field::{Input, TextField};

use egui::{Color32, CornerRadius, Painter, Rect, Response, Stroke, StrokeKind};

use crate::Tokens;

/// How much a disabled control keeps, as `opacity-50` does.
pub(crate) const DISABLED: f32 = 0.5;

/// The keyboard focus outline: 2px in `ring`, 2px off the control, as `focus-visible:outline-2
/// outline-offset-2` draws it.
pub(crate) fn focus_ring(painter: &Painter, tokens: &Tokens, rect: Rect, radius: CornerRadius, response: &Response) {
    if response.has_focus() {
        let outer = CornerRadius::from(radius.nw.saturating_add(2));
        painter.rect_stroke(
            rect.expand(2.0),
            outer,
            Stroke::new(2.0, tokens.color("ring")),
            StrokeKind::Outside,
        );
    }
}

/// A color as a disabled control shows it.
pub(crate) fn dim(color: Color32, enabled: bool) -> Color32 {
    if enabled { color } else { color.gamma_multiply(DISABLED) }
}
