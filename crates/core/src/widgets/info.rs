//! A panel's info: the small Info mark beside a title, after sokkel-ui's panel heads, that says
//! what the panel counts in a tooltip.

use egui::{Pos2, Rect, Response, Sense, Ui, vec2};

use super::tooltip;
use crate::{Icon, icons, tokens};

/// The Info mark centered on `center`, 20 square to the pointer, its 12 point glyph muted and in
/// the foreground under the pointer, and `text` in a tooltip once the pointer rests on it.
/// `label` names it for a screen reader, as the web's `words.info`.
pub fn info_tip(ui: &mut Ui, center: Pos2, text: &str, label: &str) -> Response {
    let t = tokens(ui.ctx());
    let rect = Rect::from_center_size(center, vec2(20.0, 20.0));
    let id = ui.id().with(("info", center.x as i32, center.y as i32));
    let response = ui.interact(rect, id, Sense::hover());
    let ink = if response.hovered() {
        t.color("foreground")
    } else {
        t.color("muted-foreground")
    };
    icons::paint(ui.painter(), center, Icon::Info, 12.0, ink);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, label));
    tooltip(response, text)
}
