//! Zoom in, zoom out and fit in one flush cluster, with the app's own buttons after them, after
//! sokkel-ui-graph's `GraphViewButtons` and `GraphControlButton`.

use egui::{CornerRadius, Id, Painter, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, vec2};
use sokkel_egui::{Icon, tokens};

/// One button's side.
pub const CONTROL_BUTTON: f32 = 28.0;
/// The cluster's margin from the corner of the canvas it stands in.
pub const CONTROLS_MARGIN: f32 = 8.0;
/// A glyph's size on a button.
const GLYPH: f32 = 14.0;
/// How much a disabled button keeps, as `disabled:opacity-40`.
const DISABLED: f32 = 0.4;

/// The words the view's buttons say.
#[derive(Clone, Debug)]
pub struct GraphViewWords {
    pub zoom_in: String,
    pub zoom_out: String,
    pub fit: String,
    /// What the zoom-in button's tooltip says in place of its name, such as how else the canvas
    /// zooms.
    pub zoom_in_hint: Option<String>,
    pub zoom_out_hint: Option<String>,
    pub fit_hint: Option<String>,
}

impl Default for GraphViewWords {
    fn default() -> Self {
        Self {
            zoom_in: "Zoom in".into(),
            zoom_out: "Zoom out".into(),
            fit: "Fit the graph".into(),
            zoom_in_hint: None,
            zoom_out_hint: None,
            fit_hint: None,
        }
    }
}

/// A button of the app's in the view's cluster, drawn flush as the zoom buttons are: a glyph,
/// named by `label`, with a tooltip saying the label, or `tooltip` in its place. Hand it to a
/// canvas's `extra_controls` or to `GraphViewButtons::extra`.
#[derive(Clone, Debug)]
pub struct GraphControlButton {
    pub icon: Icon,
    pub label: String,
    pub tooltip: Option<String>,
    pub disabled: bool,
}

impl GraphControlButton {
    pub fn new(icon: Icon, label: impl Into<String>) -> Self {
        Self {
            icon,
            label: label.into(),
            tooltip: None,
            disabled: false,
        }
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Which way the cluster's buttons stand.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonsOrientation {
    /// Side by side, as under a fitted canvas.
    #[default]
    Row,
    /// One over another, as in a stage's corner.
    Column,
}

/// What the cluster's buttons were pressed for this frame, and where its app buttons stand, so an
/// overlay can open beside the one that opened it.
#[derive(Clone, Debug)]
pub struct ViewButtonsResponse {
    pub zoom_in: bool,
    pub zoom_out: bool,
    pub fit: bool,
    /// The index of the app's button pressed.
    pub extra: Option<usize>,
    /// Each app button's rectangle on the screen.
    pub extra_rects: Vec<Rect>,
    /// The cluster's rectangle on the screen.
    pub rect: Rect,
}

impl Default for ViewButtonsResponse {
    fn default() -> Self {
        Self {
            zoom_in: false,
            zoom_out: false,
            fit: false,
            extra: None,
            extra_rects: Vec::new(),
            rect: Rect::NOTHING,
        }
    }
}

/// Zoom in, zoom out and fit in one flush cluster, with the app's own buttons after them: the
/// cluster a canvas draws in its corner, for an app to draw in a panel's head as well.
pub struct GraphViewButtons<'a> {
    id: Id,
    orientation: ButtonsOrientation,
    words: GraphViewWords,
    extra: &'a [GraphControlButton],
    idle: bool,
    hide_when_idle: bool,
}

impl<'a> GraphViewButtons<'a> {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            orientation: ButtonsOrientation::Row,
            words: GraphViewWords::default(),
            extra: &[],
            idle: false,
            hide_when_idle: false,
        }
    }

    pub fn orientation(mut self, orientation: ButtonsOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn words(mut self, words: GraphViewWords) -> Self {
        self.words = words;
        self
    }

    /// The app's own buttons after the zoom buttons.
    pub fn extra(mut self, extra: &'a [GraphControlButton]) -> Self {
        self.extra = extra;
        self
    }

    /// Nothing to zoom: the zoom buttons are disabled, or left out with `hide_when_idle`.
    pub fn idle(mut self, idle: bool) -> Self {
        self.idle = idle;
        self
    }

    /// Leave the zoom buttons out while there is nothing to zoom, rather than disabling them.
    pub fn hide_when_idle(mut self, hide: bool) -> Self {
        self.hide_when_idle = hide;
        self
    }

    /// How many buttons the cluster holds.
    fn count(&self) -> usize {
        let zoom = if self.idle && self.hide_when_idle { 0 } else { 3 };
        zoom + self.extra.len()
    }

    /// The cluster's size, its edge included.
    pub fn size(&self) -> Vec2 {
        let along = self.count() as f32 * CONTROL_BUTTON + 2.0;
        match self.orientation {
            ButtonsOrientation::Row => vec2(along, CONTROL_BUTTON + 2.0),
            ButtonsOrientation::Column => vec2(CONTROL_BUTTON + 2.0, along),
        }
    }

    /// Draw the cluster in the corner of `frame`, `CONTROLS_MARGIN` in from its bottom right.
    pub fn show_in_corner(self, ui: &Ui, painter: &Painter, frame: Rect) -> ViewButtonsResponse {
        let size = self.size();
        let min = frame.right_bottom() - size - Vec2::splat(CONTROLS_MARGIN);
        self.show_at(ui, painter, Rect::from_min_size(min, size))
    }

    /// Draw the cluster where the layout puts it.
    pub fn show(self, ui: &mut Ui) -> ViewButtonsResponse {
        let (rect, _) = ui.allocate_exact_size(self.size(), Sense::hover());
        let painter = ui.painter().clone();
        self.show_at(ui, &painter, rect)
    }

    /// Draw the cluster in `group`, as big as `size` says.
    fn show_at(self, ui: &Ui, painter: &Painter, group: Rect) -> ViewButtonsResponse {
        let t = tokens(ui.ctx());
        let mut answer = ViewButtonsResponse {
            rect: group,
            ..Default::default()
        };
        if self.count() == 0 {
            return answer;
        }
        let corners = t.radius("control");
        for shadow in t.shadows("elevation-node") {
            painter.add(shadow.as_shape(group, corners));
        }
        painter.rect(
            group,
            corners,
            t.color("card"),
            Stroke::new(1.0, t.color("border")),
            StrokeKind::Inside,
        );
        let zoom = !(self.idle && self.hide_when_idle);
        let mut buttons: Vec<(Icon, &str, Option<&str>, bool)> = Vec::new();
        if zoom {
            buttons.push((
                Icon::ZoomIn,
                &self.words.zoom_in,
                self.words.zoom_in_hint.as_deref(),
                self.idle,
            ));
            buttons.push((
                Icon::ZoomOut,
                &self.words.zoom_out,
                self.words.zoom_out_hint.as_deref(),
                self.idle,
            ));
            buttons.push((Icon::Scan, &self.words.fit, self.words.fit_hint.as_deref(), self.idle));
        }
        for extra in self.extra {
            buttons.push((extra.icon, &extra.label, extra.tooltip.as_deref(), extra.disabled));
        }
        let first_extra = if zoom { 3 } else { 0 };
        for (k, (icon, word, hint, disabled)) in buttons.into_iter().enumerate() {
            let step = k as f32 * CONTROL_BUTTON;
            let offset = match self.orientation {
                ButtonsOrientation::Row => vec2(1.0 + step, 1.0),
                ButtonsOrientation::Column => vec2(1.0, 1.0 + step),
            };
            let spot = Rect::from_min_size(group.min + offset, Vec2::splat(CONTROL_BUTTON));
            let sense = if disabled { Sense::hover() } else { Sense::click() };
            let response = ui.interact(spot, self.id.with(("view-button", k)), sense);
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, !disabled, word));
            if k > 0 {
                match self.orientation {
                    ButtonsOrientation::Row => {
                        painter.vline(spot.left() - 0.5, spot.y_range(), Stroke::new(1.0, t.color("border")));
                    }
                    ButtonsOrientation::Column => {
                        painter.hline(spot.x_range(), spot.top() - 0.5, Stroke::new(1.0, t.color("border")));
                    }
                }
            }
            let ink = if disabled {
                t.color("muted-foreground").gamma_multiply(DISABLED)
            } else if response.hovered() {
                painter.rect_filled(spot, CornerRadius::ZERO, t.color("hover"));
                t.color("foreground")
            } else {
                t.color("muted-foreground")
            };
            sokkel_egui::icons::paint(painter, spot.center(), icon, GLYPH, ink);
            let said = hint.filter(|h| !h.is_empty()).unwrap_or(word);
            let clicked = response.on_hover_text(said).clicked();
            if k >= first_extra {
                answer.extra_rects.push(spot);
                if clicked {
                    answer.extra = Some(k - first_extra);
                }
            } else if clicked {
                match k {
                    0 => answer.zoom_in = true,
                    1 => answer.zoom_out = true,
                    _ => answer.fit = true,
                }
            }
        }
        answer
    }
}
