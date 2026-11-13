//! The primitives, each drawn from the tokens alone, after sokkel-ui's components of the same name.

mod button;
mod chip;
mod choice;
mod display;
mod frame;
mod overlay;
mod page;
mod settings;
mod switch;
mod table;
mod text_field;

pub use button::{Button, ButtonSize, ButtonVariant};
pub use chip::{Chip, ChipTone};
pub use choice::{Checkbox, Segmented, SegmentedVariant, Tabs, TabsVariant};
pub use display::{Alert, Badge, BadgeVariant, Card, Separator, Skeleton};
pub use frame::{Rail, RailEntry, RailGroup, Shell, ShellPart, StatusLine, StatusTone, TopBar};
pub use overlay::{Dialog, Select, tooltip, tooltip_shown};
pub use page::{PageHeader, PageState, PageStateTone, PageStateWords};
pub use settings::{SettingsDialog, SettingsOutcome, SettingsSection, SettingsWords, appearance_settings, setting_row};
pub use switch::Switch;
pub use table::{Cell, Column, DataTable, TableState, TableVariant, TableWords};
pub use text_field::{Input, TextField};

use egui::{Color32, CornerRadius, Painter, Rect, Response, Stroke, StrokeKind, Ui};

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

/// The busy spinner: three quarters of a ring turning, in `color`; asks for the next frame.
pub(crate) fn spinner(ui: &Ui, painter: &Painter, center: egui::Pos2, radius: f32, color: Color32) {
    let start = ui.input(|i| i.time) as f32 * 6.0;
    let points: Vec<_> = (0..=24)
        .map(|i| {
            let a = start + i as f32 / 24.0 * std::f32::consts::TAU * 0.75;
            center + radius * egui::vec2(a.cos(), a.sin())
        })
        .collect();
    painter.add(egui::Shape::line(points, Stroke::new(1.5, color)));
    ui.ctx().request_repaint();
}
