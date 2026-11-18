//! The tools of a graph editor in one bar, after sokkel-ui-graph's `GraphToolbar`: undo and redo,
//! a tidy layout, the view's zoom and fit, and how many steps have a problem.

use std::sync::Arc;

use egui::{
    Align, FontSelection, Galley, Id, Painter, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetText,
    pos2, text::TextWrapping, vec2,
};
use sokkel_egui::{Icon, TextRole, Tokens, tokens};

use crate::view_buttons::GraphControlButton;

/// The bar's height: a button, the padding and the edge.
pub const TOOLBAR_HEIGHT: f32 = BUTTON + 2.0 * (PAD + 1.0);
/// The bar's margin from the corner of the canvas it stands over.
pub const TOOLBAR_MARGIN: f32 = 8.0;
/// One button's side.
const BUTTON: f32 = 28.0;
/// The padding inside the bar's edge, and the gap between its parts.
const PAD: f32 = 2.0;
const GAP: f32 = 2.0;
/// A divider's margins across and its height.
const DIVIDER_MARGIN: f32 = 2.0;
const DIVIDER_HEIGHT: f32 = 16.0;
/// The gap between the app's own buttons.
const APP_GAP: f32 = 4.0;
/// A glyph's size, and the problems pill's padding and gap.
const GLYPH: f32 = 14.0;
const PILL_PAD: f32 = 6.0;
const PILL_GAP: f32 = 6.0;
/// How much a disabled button keeps, as `disabled:opacity-40`.
const DISABLED: f32 = 0.4;

/// The words a graph toolbar says.
#[derive(Clone, Debug)]
pub struct GraphToolbarWords {
    /// The toolbar's accessible name.
    pub label: String,
    pub undo: String,
    pub redo: String,
    /// Lays the graph out again.
    pub layout: String,
    pub zoom_in: String,
    pub zoom_out: String,
    pub fit: String,
    /// How many things have a problem, `{count}` standing for the number.
    pub issues: String,
    /// What it says when nothing has one.
    pub clean: String,
}

impl Default for GraphToolbarWords {
    fn default() -> Self {
        Self {
            label: "Graph tools".into(),
            undo: "Undo".into(),
            redo: "Redo".into(),
            layout: "Tidy up the layout".into(),
            zoom_in: "Zoom in".into(),
            zoom_out: "Zoom out".into(),
            fit: "Fit the graph".into(),
            issues: "{count} to fix".into(),
            clean: "No problems".into(),
        }
    }
}

/// What the bar's buttons were pressed for this frame.
#[derive(Clone, Debug)]
pub struct GraphToolbarResponse {
    pub undo: bool,
    pub redo: bool,
    pub layout: bool,
    /// The problems count was pressed, to go to the first problem.
    pub issues: bool,
    pub zoom_in: bool,
    pub zoom_out: bool,
    pub fit: bool,
    /// The index of the app's button pressed.
    pub extra: Option<usize>,
    /// Each app button's rectangle on the screen.
    pub extra_rects: Vec<Rect>,
    /// The bar on the screen.
    pub rect: Rect,
}

impl Default for GraphToolbarResponse {
    fn default() -> Self {
        Self {
            undo: false,
            redo: false,
            layout: false,
            issues: false,
            zoom_in: false,
            zoom_out: false,
            fit: false,
            extra: None,
            extra_rects: Vec::new(),
            rect: Rect::NOTHING,
        }
    }
}

/// The tools of a graph editor in one bar: undo and redo, a tidy layout, the view's zoom and fit,
/// and how many steps have a problem, with the app's own buttons after them. Each group shows only
/// when it is given something to do. Drawn over a canvas as its `toolbar`, zoom and fit act on that
/// canvas; leave them out with `zoom(false)` when its corner's cluster carries them.
#[derive(Clone, Debug)]
pub struct GraphToolbar {
    history: Option<(bool, bool)>,
    layout: bool,
    issues: Option<usize>,
    issues_pressable: bool,
    zoom: bool,
    extra: Vec<GraphControlButton>,
    words: GraphToolbarWords,
}

impl Default for GraphToolbar {
    fn default() -> Self {
        Self::new()
    }
}

/// One part of the bar, in order.
enum Part {
    Button(usize, Icon, String, Option<String>, bool),
    Divider,
    Issues(usize),
    App(usize),
}

impl GraphToolbar {
    pub fn new() -> Self {
        Self {
            history: None,
            layout: false,
            issues: None,
            issues_pressable: false,
            zoom: true,
            extra: Vec::new(),
            words: GraphToolbarWords::default(),
        }
    }

    /// Undo and redo, each enabled when there is something to take back or bring back.
    pub fn history(mut self, can_undo: bool, can_redo: bool) -> Self {
        self.history = Some((can_undo, can_redo));
        self
    }

    /// The button that lays the graph out again.
    pub fn layout(mut self, layout: bool) -> Self {
        self.layout = layout;
        self
    }

    /// How many things have a problem; `pressable` makes a count above none a button.
    pub fn issues(mut self, issues: Option<usize>, pressable: bool) -> Self {
        self.issues = issues;
        self.issues_pressable = pressable;
        self
    }

    /// False leaves zoom and fit out, for a canvas that keeps its own buttons in its corner.
    pub fn zoom(mut self, zoom: bool) -> Self {
        self.zoom = zoom;
        self
    }

    /// The app's own buttons, after the tools.
    pub fn extra(mut self, extra: Vec<GraphControlButton>) -> Self {
        self.extra = extra;
        self
    }

    pub fn words(mut self, words: GraphToolbarWords) -> Self {
        self.words = words;
        self
    }

    /// The parts the bar draws; `view` when it stands over a canvas with something to zoom.
    fn parts(&self, ctx: &egui::Context, view: bool) -> Vec<Part> {
        let mac = ctx.os() == egui::os::OperatingSystem::Mac;
        let keys = |redo: bool| match (mac, redo) {
            (true, false) => "Cmd+Z",
            (true, true) => "Shift+Cmd+Z",
            (false, false) => "Ctrl+Z",
            (false, true) => "Shift+Ctrl+Z",
        };
        let mut groups: Vec<Vec<Part>> = Vec::new();
        if let Some((can_undo, can_redo)) = self.history {
            groups.push(vec![
                Part::Button(
                    0,
                    Icon::Undo2,
                    self.words.undo.clone(),
                    Some(keys(false).into()),
                    !can_undo,
                ),
                Part::Button(
                    1,
                    Icon::Redo2,
                    self.words.redo.clone(),
                    Some(keys(true).into()),
                    !can_redo,
                ),
            ]);
        }
        let tidies = self.layout && view;
        let zooms = self.zoom && view;
        if tidies || zooms {
            let mut group = Vec::new();
            if tidies {
                group.push(Part::Button(2, Icon::Network, self.words.layout.clone(), None, false));
            }
            if zooms {
                group.push(Part::Button(3, Icon::ZoomOut, self.words.zoom_out.clone(), None, false));
                group.push(Part::Button(4, Icon::ZoomIn, self.words.zoom_in.clone(), None, false));
                group.push(Part::Button(5, Icon::Scan, self.words.fit.clone(), None, false));
            }
            groups.push(group);
        }
        if let Some(count) = self.issues {
            groups.push(vec![Part::Issues(count)]);
        }
        if !self.extra.is_empty() {
            groups.push((0..self.extra.len()).map(Part::App).collect());
        }
        let mut parts = Vec::new();
        for (k, group) in groups.into_iter().enumerate() {
            if k > 0 {
                parts.push(Part::Divider);
            }
            parts.extend(group);
        }
        parts
    }

    /// The problems pill's glyph, words and their galley.
    fn pill(&self, ui: &Ui, t: &Tokens, count: usize) -> (Icon, Arc<Galley>, bool) {
        let bad = count > 0;
        let words = if bad {
            self.words.issues.replace("{count}", &count.to_string())
        } else {
            self.words.clean.clone()
        };
        let rich = if bad {
            TextRole::Badge.rich(t, words)
        } else {
            TextRole::Help.rich(t, words)
        };
        let icon = if bad { Icon::CircleAlert } else { Icon::CircleCheck };
        (icon, galley(ui, rich), bad)
    }

    /// How wide one part is.
    fn part_width(&self, ui: &Ui, t: &Tokens, part: &Part) -> f32 {
        match part {
            Part::Button(..) | Part::App(_) => BUTTON,
            Part::Divider => 2.0 * DIVIDER_MARGIN + 1.0,
            Part::Issues(count) => {
                let (_, words, _) = self.pill(ui, t, *count);
                2.0 * PILL_PAD + GLYPH + PILL_GAP + words.size().x.ceil()
            }
        }
    }

    /// The bar's size, for a view that keeps room for it; `view` as in `show_at`.
    pub fn size(&self, ui: &Ui, view: bool) -> Vec2 {
        let t = tokens(ui.ctx());
        let parts = self.parts(ui.ctx(), view);
        vec2(self.width(ui, &t, &parts), TOOLBAR_HEIGHT)
    }

    fn width(&self, ui: &Ui, t: &Tokens, parts: &[Part]) -> f32 {
        let mut width = 2.0 * (PAD + 1.0);
        let mut last_app = false;
        for (k, part) in parts.iter().enumerate() {
            let app = matches!(part, Part::App(_));
            if k > 0 {
                // the app's buttons keep their own gap; every other part the bar's
                width += if app && last_app { APP_GAP } else { GAP };
            }
            width += self.part_width(ui, t, part);
            last_app = app;
        }
        width
    }

    /// Draw the bar where the layout puts it, as a toolbar outside a canvas.
    pub fn show(self, ui: &mut Ui, id: Id) -> GraphToolbarResponse {
        let size = self.size(ui, false);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter().clone();
        self.show_at(ui, &painter, rect.min, id, false)
    }

    /// Draw the bar with its top left at `at`; `view` draws zoom, fit and the tidy layout, as over
    /// a canvas with something on it.
    pub fn show_at(self, ui: &Ui, painter: &Painter, at: Pos2, id: Id, view: bool) -> GraphToolbarResponse {
        let t = tokens(ui.ctx());
        let parts = self.parts(ui.ctx(), view);
        let rect = Rect::from_min_size(at, vec2(self.width(ui, &t, &parts), TOOLBAR_HEIGHT));
        let mut answer = GraphToolbarResponse {
            rect,
            ..Default::default()
        };
        let bar = ui.interact(rect, id.with("toolbar"), Sense::hover());
        bar.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Unknown, true, &self.words.label));
        let corners = t.radius("control");
        for shadow in t.shadows("elevation-node") {
            painter.add(shadow.as_shape(rect, corners));
        }
        painter.rect(
            rect,
            corners,
            t.color("card"),
            Stroke::new(1.0, t.color("border")),
            StrokeKind::Inside,
        );
        let mut x = rect.left() + PAD + 1.0;
        let top = rect.top() + PAD + 1.0;
        let mut last_app = false;
        for (k, part) in parts.iter().enumerate() {
            let app = matches!(part, Part::App(_));
            if k > 0 {
                x += if app && last_app { APP_GAP } else { GAP };
            }
            last_app = app;
            let width = self.part_width(ui, &t, part);
            let spot = Rect::from_min_size(pos2(x, top), vec2(width, BUTTON));
            x += width;
            match part {
                Part::Divider => {
                    painter.vline(
                        spot.center().x,
                        (spot.center().y - DIVIDER_HEIGHT / 2.0)..=(spot.center().y + DIVIDER_HEIGHT / 2.0),
                        Stroke::new(1.0, t.color("border")),
                    );
                }
                Part::Button(which, icon, label, keys, disabled) => {
                    let tip = match keys {
                        Some(keys) => format!("{label}  {keys}"),
                        None => label.clone(),
                    };
                    if button(
                        ui,
                        painter,
                        &t,
                        spot,
                        id.with(("tool", which)),
                        *icon,
                        label,
                        &tip,
                        *disabled,
                    ) {
                        match which {
                            0 => answer.undo = true,
                            1 => answer.redo = true,
                            2 => answer.layout = true,
                            3 => answer.zoom_out = true,
                            4 => answer.zoom_in = true,
                            _ => answer.fit = true,
                        }
                    }
                }
                Part::App(k) => {
                    let extra = &self.extra[*k];
                    let tip = extra.tooltip.clone().unwrap_or_else(|| extra.label.clone());
                    answer.extra_rects.push(spot);
                    if button(
                        ui,
                        painter,
                        &t,
                        spot,
                        id.with(("app-tool", k)),
                        extra.icon,
                        &extra.label,
                        &tip,
                        extra.disabled,
                    ) {
                        answer.extra = Some(*k);
                    }
                }
                Part::Issues(count) => {
                    let (icon, words, bad) = self.pill(ui, &t, *count);
                    let ink = if bad {
                        t.color("critical-ink")
                    } else {
                        t.color("muted-foreground")
                    };
                    let said = words.text().to_owned();
                    if bad && self.issues_pressable {
                        let response = ui.interact(spot, id.with("issues"), Sense::click());
                        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &said));
                        if response.hovered() {
                            painter.rect_filled(spot, t.radius("chip"), t.color("hover"));
                        }
                        answer.issues = response.clicked();
                    } else {
                        ui.interact(spot, id.with("issues"), Sense::hover())
                            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &said));
                    }
                    let glyph = pos2(spot.left() + PILL_PAD + GLYPH / 2.0, spot.center().y);
                    sokkel_egui::icons::paint(painter, glyph, icon, GLYPH, ink);
                    let left = spot.left() + PILL_PAD + GLYPH + PILL_GAP;
                    painter.galley(pos2(left, spot.center().y - words.size().y / 2.0), words, ink);
                }
            }
        }
        answer
    }
}

/// One of the bar's buttons: a glyph on the bar's ground, lit under the pointer; true when pressed.
#[allow(clippy::too_many_arguments)]
fn button(
    ui: &Ui,
    painter: &Painter,
    t: &Tokens,
    spot: Rect,
    id: Id,
    icon: Icon,
    label: &str,
    tip: &str,
    disabled: bool,
) -> bool {
    let sense = if disabled { Sense::hover() } else { Sense::click() };
    let response = ui.interact(spot, id, sense);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, !disabled, label));
    let ink = if disabled {
        t.color("muted-foreground").gamma_multiply(DISABLED)
    } else if response.hovered() {
        painter.rect_filled(spot, t.radius("chip"), t.color("hover"));
        t.color("foreground")
    } else {
        t.color("muted-foreground")
    };
    sokkel_egui::icons::paint(painter, spot.center(), icon, GLYPH, ink);
    if disabled {
        return false;
    }
    response.on_hover_text(tip).clicked()
}

/// Words laid out in one line, in their role's face.
pub(crate) fn galley(ui: &Ui, rich: RichText) -> Arc<Galley> {
    WidgetText::from(rich).into_galley_impl(
        ui.ctx(),
        &ui.ctx().global_style(),
        TextWrapping::no_max_width(),
        FontSelection::Default,
        Align::Min,
    )
}
