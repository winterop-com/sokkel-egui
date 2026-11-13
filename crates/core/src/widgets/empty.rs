//! What a place says while it holds nothing, after sokkel-ui's `primitives/empty`: an icon in a
//! muted box, a title, a line on what to do, and its actions under them, in the middle of its room.

use std::sync::Arc;

use egui::{Galley, Rect, Sense, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{Icon, TextRole, icons, tokens};

/// An empty state: `Empty::new("Nothing in Today").icon(..).description(..).show(ui, |ui| ..)`.
pub struct Empty {
    title: String,
    icon: Option<Icon>,
    description: Option<String>,
}

/// The most a line of the empty state runs, as `max-w-sm`.
const MOST: f32 = 384.0;

impl Empty {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            icon: None,
            description: None,
        }
    }

    /// The icon in a 32 box on the muted ground over the title.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The line under the title, in lines as even as they can be, as `text-balance` sets them.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Draw it 24 in from the top of the room `ui` gives, centered across it; `actions` draws what
    /// goes under it, such as a button, centered too.
    pub fn show<R>(self, ui: &mut Ui, actions: impl FnOnce(&mut Ui) -> R) -> R {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        let left = ui.cursor().left();
        let center = left + width / 2.0;
        ui.add_space(24.0);
        if let Some(icon) = self.icon {
            let (rect, _) = ui.allocate_exact_size(vec2(width, 32.0), Sense::hover());
            let tile = Rect::from_center_size(pos2(center, rect.center().y), vec2(32.0, 32.0));
            ui.painter().rect_filled(tile, t.radius("lg"), t.color("muted"));
            icons::paint(ui.painter(), tile.center(), icon, 16.0, t.color("foreground"));
            // the icon's own margin under it, then the header's gap, as the web's lines fall
            ui.add_space(8.0 + 8.0 + 2.0);
        }
        // tight, as `tracking-tight`: a fortieth of the size closer
        let size = TextRole::Label.font(&t).size;
        let rich = TextRole::Label.rich(&t, &self.title).extra_letter_spacing(-size / 40.0);
        let title =
            WidgetText::from(rich).into_galley(ui, Some(TextWrapMode::Wrap), MOST.min(width), TextRole::Label.font(&t));
        centered(ui, title, center, t.color("foreground"));
        if let Some(description) = &self.description {
            ui.add_space(8.0 + 3.0);
            let rich = TextRole::Body.rich(&t, description).line_height(Some(24.0));
            let galley = balanced(ui, rich, MOST.min(width), TextRole::Body.font(&t));
            centered(ui, galley, center, t.color("muted-foreground"));
        }
        ui.add_space(15.0);
        ui.vertical_centered(|ui| actions(ui)).inner
    }
}

/// A galley laid on a line of its own, centered on `center`.
fn centered(ui: &mut Ui, galley: Arc<Galley>, center: f32, ink: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), galley.size().y), Sense::hover());
    ui.painter()
        .galley(pos2(center - galley.size().x / 2.0, rect.top()), galley, ink);
}

/// Text wrapped in as many lines as `most` wide needs, each as near the others' length as can be:
/// the narrowest width that keeps that many lines.
fn balanced(ui: &Ui, rich: egui::RichText, most: f32, font: egui::FontId) -> Arc<Galley> {
    let lay = |w: f32| WidgetText::from(rich.clone()).into_galley(ui, Some(TextWrapMode::Wrap), w, font.clone());
    let widest = lay(most);
    let lines = widest.rows.len();
    if lines < 2 {
        return widest;
    }
    let (mut low, mut high) = (most / lines as f32, most);
    while high - low > 1.0 {
        let mid = (low + high) / 2.0;
        if lay(mid).rows.len() > lines {
            low = mid;
        } else {
            high = mid;
        }
    }
    lay(high)
}
