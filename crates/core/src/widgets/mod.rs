//! The primitives, each drawn from the tokens alone, after sokkel-ui's components of the same name.

mod board;
mod button;
mod calendar;
mod chip;
mod chip_bar;
mod choice;
mod commands;
mod dashboard;
mod datetime;
mod display;
mod empty;
mod fields;
mod frame;
mod layout;
mod menu;
mod overlay;
mod page;
mod person;
mod section;
mod settings;
mod stat;
mod switch;
mod table;
mod table_edit;
mod text_field;
mod time;
mod uptime;

pub use board::{BOARD_COLUMN, Board, BoardCard, BoardColumn, BoardMove, CardResponse};
pub use button::{Button, ButtonSize, ButtonVariant, IconButton};
pub use calendar::{Calendar, CalendarWords, WeekStart, calendar_forget};
pub use chip::{Chip, ChipTone, TagChips};
pub use chip_bar::ChipBar;
pub use choice::{Checkbox, Choice, ChoiceList, Segmented, SegmentedVariant, Tabs, TabsVariant, checkbox_list};
pub use commands::{AppearanceCommandWords, appearance_commands, place_commands, place_of, run_appearance_command};
pub use dashboard::{DashboardGrid, DashboardPanel, GridCells, PanelSpan, cell_height, report_natural_height};
pub use datetime::{DateTimeField, DateTimeFieldResponse, DateTimeFieldWords, Moment};
pub use display::{Alert, Badge, BadgeVariant, Banner, BannerTone, Card, Separator, Skeleton};
pub use empty::Empty;
pub(crate) use fields::search_field;
pub use fields::{DueState, DueText, QuickAdd, SearchField, SwitchField, due_state, field, form_bar, toolbar};
pub use frame::{
    AppBar, DesktopWindow, Rail, RailEntry, RailGroup, RailWords, Shell, ShellPart, ShellWords, StatusLine, StatusTone,
    TopBar, desktop_viewport, is_phone,
};
pub use layout::{AsideWidth, Columns, Gap, Split, SplitPane, Stack};
pub use menu::{CommandGroup, CommandPalette, DropdownMenu, MenuItem, PaletteOutcome, PaletteWords, palette_shortcut};
pub use overlay::{ConfirmAnswer, ConfirmDialog, Dialog, Select, tooltip, tooltip_shown};
pub use page::{Page, PageHeader, PageHeaderResponse, PageState, PageStateTone, PageStateWords};
pub use person::{AccountAction, AccountMenu, AccountWords, AvatarSize, PersonAvatar, initials, kind_of};
pub use section::{DescriptionList, PersonRow, Section, Timeline, TimelineEvent};
pub use settings::{
    SettingsDialog, SettingsOutcome, SettingsSection, SettingsWords, ThemeWords, setting_row, setting_row_found_by,
    theme_settings,
};
pub use stat::{KpiRow, Stat, sparkline};
pub use switch::Switch;
pub use table::{
    Cell, CellEdit, Column, DataTable, RowActions, StackLayout, StackPlace, TableOutput, TableState, TableVariant,
    TableWords, Validate,
};
pub use table_edit::{EditWords, Editor};
pub use text_field::{Input, TextField};
pub use time::relative_time;
pub use uptime::{UptimeBar, UptimeBarWords, UptimeDay, format_uptime, overall_uptime, uptime_color};

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

/// The keyboard focus outline drawn inside a control, for one at the window's edge whose outline
/// outside would be cut off (the top bar's buttons on a phone, the spec's `phone+focus`):
/// `focus-ring-width` in `ring`, `focus-ring-offset` inside the control's edge.
pub(crate) fn focus_ring_inset(
    painter: &Painter,
    tokens: &Tokens,
    rect: Rect,
    radius: CornerRadius,
    response: &Response,
) {
    if response.has_focus() {
        let width = tokens.length("focus-ring-width");
        let inset = tokens.length("focus-ring-offset") + width;
        let inner = CornerRadius::from(radius.nw.saturating_sub(inset as u8));
        painter.rect_stroke(
            rect.shrink(inset),
            inner,
            Stroke::new(width, tokens.color("ring")),
            StrokeKind::Outside,
        );
    }
}

/// A paint's ground, as sokkel-design resolves it for a part in a state; the token `or` names
/// where a theme lacks the paint (the paint check fails a paint egui reads that is missing).
pub(crate) fn ground(tokens: &Tokens, key: &str, or: &str) -> Color32 {
    tokens
        .paint(key)
        .and_then(|p| p.ground)
        .unwrap_or_else(|| tokens.color(or))
}

/// A paint's edge, as `ground`.
pub(crate) fn edge(tokens: &Tokens, key: &str, or: &str) -> Color32 {
    tokens
        .paint(key)
        .and_then(|p| p.edge)
        .unwrap_or_else(|| tokens.color(or))
}

/// A paint's ink, as `ground`.
pub(crate) fn ink(tokens: &Tokens, key: &str, or: &str) -> Color32 {
    tokens
        .paint(key)
        .and_then(|p| p.ink)
        .unwrap_or_else(|| tokens.color(or))
}

/// The paint keys egui's parts read, for the paint check.
pub const PAINTS_READ: &[&str] = &[
    "dropdown-menu.base.rest.popup",
    "dropdown-menu.base.highlighted.item",
    "dialog.base.rest.box",
    "dialog.base.rest.backdrop",
    "dialog.base.rest.footer",
    "tooltip.base.rest.box",
    "select.base.highlighted.row",
    "text-field.base.rest.control",
    "text-field.base.focus.control",
    "text-field.base.invalid.control",
    "text-field.base.invalid+focus.control",
];

/// A popup's surface (a select's list, a menu, a submenu, the account menu): the popover's
/// ground, the surface's corners, a 1px `border-strong` edge and `elevation-pop`'s shadow.
pub(crate) fn popup_frame(tokens: &Tokens) -> egui::Frame {
    egui::Frame::new()
        .fill(ground(tokens, "dropdown-menu.base.rest.popup", "popover"))
        .stroke(Stroke::new(
            1.0,
            edge(tokens, "dropdown-menu.base.rest.popup", "border-strong"),
        ))
        .corner_radius(tokens.radius("surface"))
        .shadow(tokens.shadow("elevation-pop"))
}

/// A boxed dialog's surface (a dialog, the settings dialog, the palette): a popup's, wearing
/// `elevation-dialog`, or `elevation-pop` until a theme has the dialog's own.
pub(crate) fn dialog_frame(tokens: &Tokens) -> egui::Frame {
    let shadow = if tokens.shadows("elevation-dialog").is_empty() {
        tokens.shadow("elevation-pop")
    } else {
        tokens.shadow("elevation-dialog")
    };
    popup_frame(tokens).shadow(shadow)
}

/// A control's height on this window: `height`, and at least a finger on a phone, as the web's
/// `max-md:min-h-finger` (buttons, inputs, search fields, a select's trigger).
pub(crate) fn finger_tall(ctx: &egui::Context, tokens: &Tokens, height: f32) -> f32 {
    if is_phone(ctx) {
        height.max(tokens.length("spacing-finger"))
    } else {
        height
    }
}

/// How tall a one-line row of a list of choices (a menu, a select's list) is: a control tall, as
/// the density sets it, and a finger tall on a phone.
pub(crate) fn choice_row_height(ctx: &egui::Context, tokens: &Tokens) -> f32 {
    if is_phone(ctx) {
        tokens.length("spacing-finger")
    } else {
        tokens.density("control")
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
