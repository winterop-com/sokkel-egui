//! What opens over the page: a tooltip, a select's list, and a dialog, after sokkel-ui's `ui/`
//! tooltip, select and dialog.

use egui::{
    Align, Context, CornerRadius, Frame, Id, Layout, Margin, Popup, PopupCloseBehavior, PopupKind, Rect, RectAlign,
    Response, Sense, Stroke, StrokeKind, TextWrapMode, Ui, Widget, WidgetText, pos2, vec2,
};

use super::focus_ring;
use crate::{Button, ButtonVariant, Icon, Swatch, TextRole, icons, tokens};

/// How long the pointer rests on a part before its tooltip shows, in seconds.
const TOOLTIP_DELAY: f64 = 0.5;

/// Say `text` in a tooltip under `response` once the pointer has rested on it, or while it has
/// keyboard focus: the foreground ground with the background's ink, as sokkel-ui's tooltip.
pub fn tooltip(response: Response, text: impl Into<String>) -> Response {
    show_tooltip(response, text.into(), false)
}

/// The tooltip shown whatever the pointer does, for a story or a guided tour.
pub fn tooltip_shown(response: Response, text: impl Into<String>) -> Response {
    show_tooltip(response, text.into(), true)
}

fn show_tooltip(response: Response, text: String, always: bool) -> Response {
    let ctx = response.ctx.clone();
    let t = tokens(&ctx);
    let id = response.id.with("tooltip");
    let rest = ctx.data(|d| d.get_temp::<f64>(id));
    let now = ctx.input(|i| i.time);
    let held = tap_held(&ctx, &response, id, now, &t);
    let show = if held {
        true
    } else if response.hovered() {
        let since = rest.unwrap_or(now);
        ctx.data_mut(|d| d.insert_temp(id, since));
        if now - since < TOOLTIP_DELAY {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(TOOLTIP_DELAY));
        }
        now - since >= TOOLTIP_DELAY
    } else {
        ctx.data_mut(|d| d.remove::<f64>(id));
        response.has_focus()
    } || always;
    let frame = Frame::new()
        .fill(super::ground(&t, "tooltip.base.rest.box", "foreground"))
        .corner_radius(t.radius("md"))
        .inner_margin(Margin::symmetric(12, 6));
    let shown = Popup::from_response(&response)
        .kind(PopupKind::Tooltip)
        .open(show)
        .frame(frame)
        .gap(6.0)
        .align(RectAlign::BOTTOM)
        .show(|ui| {
            ui.set_max_width(320.0);
            ui.label(TextRole::Help.rich(&t, &text).color(t.color("background")));
        });
    // the arrow, 10 wide, from the tooltip's side that faces the trigger toward it
    if let Some(shown) = shown {
        let tip = shown.response.rect;
        let x = response.rect.center().x.clamp(tip.left() + 10.0, tip.right() - 10.0);
        let (edge, toward) = if tip.center().y > response.rect.center().y {
            (tip.top(), -1.0)
        } else {
            (tip.bottom(), 1.0)
        };
        let painter = ctx.layer_painter(shown.response.layer_id);
        painter.add(egui::Shape::convex_polygon(
            vec![
                pos2(x - 5.0, edge - toward),
                pos2(x + 5.0, edge - toward),
                pos2(x, edge + toward * 5.0),
            ],
            t.color("foreground"),
            Stroke::NONE,
        ));
    }
    response
}

/// A tap by touch or pen on the trigger, as 0.28.1's `tap`: a press that ends on the trigger
/// shows the tooltip and holds it for `tip-hold`, or until the next press anywhere or a second tap
/// on the trigger. A mouse press, hover and focus are left as they were. It reads the pointer's
/// place, not the trigger's sense, so a shut button's reason shows on a tap too.
fn tap_held(ctx: &Context, response: &Response, id: Id, now: f64, t: &crate::Tokens) -> bool {
    #[derive(Clone, Copy, Default)]
    struct Tap {
        /// A touch press began on the trigger.
        pressed: bool,
        /// Held until this time.
        until: Option<f64>,
        /// The press that began ends the hold, and its release does not open it again.
        closing: bool,
    }
    let key = id.with("tap");
    let mut tap: Tap = ctx.data(|d| d.get_temp(key)).unwrap_or_default();
    let (pressed, released, touch, at) = ctx.input(|i| {
        (
            i.pointer.any_pressed(),
            i.pointer.any_released(),
            i.any_touches(),
            i.pointer.interact_pos(),
        )
    });
    let on_trigger = at.is_some_and(|p| response.rect.contains(p));
    if pressed {
        if tap.until.is_some() {
            // the next press anywhere ends the hold; on the trigger, its release does not reopen it
            tap.until = None;
            tap.closing = on_trigger;
        } else if touch && on_trigger {
            tap.pressed = true;
        }
    }
    if released {
        if tap.pressed && on_trigger && !tap.closing {
            let hold = f64::from(t.duration("tip-hold").unwrap_or(4000.0)) / 1000.0;
            tap.until = Some(now + hold);
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(hold));
        }
        tap.pressed = false;
        tap.closing = false;
    }
    if tap.until.is_some_and(|until| now >= until) {
        tap.until = None;
    }
    ctx.data_mut(|d| d.insert_temp(key, tap));
    tap.until.is_some()
}

/// One choice from a list in a popup under a trigger that looks like an input: `field` ground,
/// `input` edge, the chosen option's label and a chevron.
#[must_use = "add it with `ui.add(...)`"]
pub struct Select<'a> {
    chosen: &'a mut usize,
    options: Vec<String>,
    label: String,
    width: f32,
    default_open: bool,
    swatches: Option<Vec<Option<Swatch>>>,
    dots: Option<Vec<egui::Color32>>,
    small: bool,
    bare: bool,
    codes: Option<Vec<String>>,
    placeholder: Option<String>,
}

impl<'a> Select<'a> {
    /// `label` names the select for assistive technology.
    pub fn new(
        chosen: &'a mut usize,
        options: impl IntoIterator<Item = impl Into<String>>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            chosen,
            options: options.into_iter().map(Into::into).collect(),
            label: label.into(),
            width: 220.0,
            default_open: false,
            swatches: None,
            dots: None,
            small: false,
            bare: false,
            codes: None,
            placeholder: None,
        }
    }

    /// A code after each option's label, small in the code face and quiet, as a period's id
    /// after its name.
    pub fn codes(mut self, codes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.codes = Some(codes.into_iter().map(Into::into).collect());
        self
    }

    /// Muted words in the trigger while no option is chosen.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// The small size, as the web's `size="sm"`: `control-sm` tall, its words in the help role.
    pub fn small(mut self, small: bool) -> Self {
        self.small = small;
        self
    }

    /// No ground or edge of its own, for a select set into another part's frame, such as a
    /// variable's chip.
    pub fn bare(mut self, bare: bool) -> Self {
        self.bare = bare;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// A sample before each option's label, as a theme's colors; `None` draws an empty dashed box.
    pub fn swatches(mut self, swatches: impl IntoIterator<Item = Option<Swatch>>) -> Self {
        self.swatches = Some(swatches.into_iter().collect());
        self
    }

    /// Open the list the first time the select is shown, as a story shows it.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }
}

impl Select<'_> {
    /// How far the label moves for the swatch before it: 28 and a gap of 8.
    /// A dot of 10 before each option's label in its color, as sokkel-ui's `Dot` in a select's items.
    pub fn dots(mut self, dots: impl IntoIterator<Item = egui::Color32>) -> Self {
        self.dots = Some(dots.into_iter().collect());
        self
    }

    fn lead(&self) -> f32 {
        if self.swatches.is_some() {
            36.0
        } else if self.dots.is_some() {
            18.0
        } else {
            0.0
        }
    }
}

/// A theme's sample, 28 by 16 from `left_center`: its background with its text, accent and third
/// chart color as dots, or a dashed empty box for no theme.
fn paint_swatch(painter: &egui::Painter, t: &crate::Tokens, left_center: egui::Pos2, swatch: Option<Swatch>) {
    let rect = Rect::from_min_size(pos2(left_center.x, left_center.y - 8.0), vec2(28.0, 16.0));
    let control = t.radius("control");
    let radius = CornerRadius::same(control.nw / 2);
    match swatch {
        Some(swatch) => {
            painter.rect(
                rect,
                radius,
                swatch.ground,
                Stroke::new(1.0, swatch.edge),
                StrokeKind::Inside,
            );
            for (index, dot) in swatch.dots.into_iter().enumerate() {
                painter.circle_filled(
                    pos2(rect.center().x + (index as f32 - 1.0) * 8.0, rect.center().y),
                    3.0,
                    dot,
                );
            }
        }
        None => {
            let shape = egui::Shape::dashed_line(
                &[
                    rect.left_top(),
                    rect.right_top(),
                    rect.right_bottom(),
                    rect.left_bottom(),
                    rect.left_top(),
                ],
                Stroke::new(1.0, t.color("border-strong")),
                3.0,
                2.0,
            );
            painter.extend(shape);
        }
    }
}

impl Widget for Select<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let height = super::finger_tall(
            ui.ctx(),
            &t,
            t.density(if self.small { "control-sm" } else { "control" }),
        );
        let role = if self.small { TextRole::Help } else { TextRole::Body };
        // across the row it is given where that row asks it to fill (a setting's control under its words)
        let width = if ui.layout().main_justify {
            ui.available_width()
        } else {
            self.width
        };
        let (rect, mut response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, &self.label));
        response = response.accessible_name(&self.label);
        let popup_id = response.id.with("list");
        let opened_once = response.id.with("opened-once");
        if self.default_open && !ui.ctx().data(|d| d.get_temp::<bool>(opened_once).unwrap_or(false)) {
            ui.ctx().data_mut(|d| d.insert_temp(opened_once, true));
            Popup::open_id(ui.ctx(), popup_id);
        }
        let open = Popup::is_id_open(ui.ctx(), popup_id);
        let radius = t.radius("control");
        let painter = ui.painter();
        let edge = if open || response.has_focus() {
            t.color("ring")
        } else {
            t.color("input")
        };
        let ground = if response.hovered() && !open {
            t.color("hover")
        } else {
            t.color("field")
        };
        if self.bare {
            if ground != t.color("field") {
                painter.rect_filled(rect, CornerRadius::ZERO, ground);
            }
        } else {
            painter.rect(
                rect,
                radius,
                t.color("field"),
                Stroke::new(1.0, edge),
                StrokeKind::Inside,
            );
            if ground != t.color("field") {
                painter.rect_filled(rect.shrink(1.0), radius, ground);
            }
        }
        let current = self.options.get(*self.chosen).cloned();
        let unset = current.is_none();
        let current = current.unwrap_or_else(|| self.placeholder.clone().unwrap_or_default());
        let lead = self.lead();
        if let Some(dot) = self.dots.as_ref().and_then(|d| d.get(*self.chosen)) {
            painter.circle_filled(pos2(rect.left() + 10.0 + 5.0, rect.center().y), 5.0, *dot);
        }
        if let Some(swatches) = &self.swatches {
            paint_swatch(
                painter,
                &t,
                pos2(rect.left() + 10.0, rect.center().y),
                swatches.get(*self.chosen).copied().flatten(),
            );
        }
        let (pad, chevron) = if self.small { (8.0, 14.0) } else { (10.0, 16.0) };
        let galley = WidgetText::from(role.rich(&t, current)).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            rect.width() - 30.0 - pad - lead,
            role.font(&t),
        );
        let label_width = galley.size().x;
        painter.galley(
            pos2(rect.left() + pad + lead, rect.center().y - galley.size().y / 2.0),
            galley,
            if unset {
                t.color("muted-foreground")
            } else {
                t.color("foreground")
            },
        );
        let code_of = |index: usize| self.codes.as_ref().and_then(|c| c.get(index)).cloned();
        let mono = egui::FontId::monospace(TextRole::Help.font(&t).size);
        if let Some(code) = code_of(*self.chosen) {
            painter.text(
                pos2(rect.left() + pad + lead + label_width + 8.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                code,
                mono.clone(),
                t.color("muted-foreground"),
            );
        }
        icons::paint(
            painter,
            pos2(rect.right() - pad - chevron / 2.0 + 2.0, rect.center().y),
            Icon::ChevronDown,
            chevron,
            t.color("muted-foreground"),
        );
        if !self.bare {
            focus_ring(painter, &t, rect, radius, &response);
        }

        let frame = super::popup_frame(&t).inner_margin(Margin::same(4));
        let mut picked = None;
        Popup::menu(&response)
            .id(popup_id)
            .frame(frame)
            .gap(4.0)
            .width(rect.width())
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let row_height = super::choice_row_height(ui.ctx(), &t);
                for (index, option) in self.options.iter().enumerate() {
                    let (row, hit) = ui.allocate_exact_size(vec2(ui.available_width(), row_height), Sense::click());
                    let chosen = index == *self.chosen;
                    hit.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, chosen, option));
                    // lit in the accent, as every list of choices is
                    let lit = hit.hovered() || hit.has_focus();
                    let ink = if lit {
                        super::ink(&t, "select.base.highlighted.row", "accent-foreground")
                    } else {
                        t.color("popover-foreground")
                    };
                    if lit {
                        ui.painter().rect_filled(
                            row,
                            t.radius("md"),
                            super::ground(&t, "select.base.highlighted.row", "accent"),
                        );
                    }
                    if let Some(dot) = self.dots.as_ref().and_then(|d| d.get(index)) {
                        ui.painter()
                            .circle_filled(pos2(row.left() + 8.0 + 5.0, row.center().y), 5.0, *dot);
                    }
                    if let Some(swatches) = &self.swatches {
                        paint_swatch(
                            ui.painter(),
                            &t,
                            pos2(row.left() + 8.0, row.center().y),
                            swatches.get(index).copied().flatten(),
                        );
                    }
                    let galley = WidgetText::from(role.rich(&t, option)).into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        row.width() - 40.0 - lead,
                        role.font(&t),
                    );
                    let option_width = galley.size().x;
                    ui.painter().galley(
                        pos2(row.left() + 8.0 + lead, row.center().y - galley.size().y / 2.0),
                        galley,
                        ink,
                    );
                    if let Some(code) = code_of(index) {
                        ui.painter().text(
                            pos2(row.left() + 8.0 + lead + option_width + 8.0, row.center().y),
                            egui::Align2::LEFT_CENTER,
                            code,
                            mono.clone(),
                            t.color("muted-foreground"),
                        );
                    }
                    if chosen {
                        icons::paint(
                            ui.painter(),
                            pos2(row.right() - 14.0, row.center().y),
                            Icon::Check,
                            16.0,
                            ink,
                        );
                    }
                    if hit.clicked() {
                        picked = Some(index);
                        ui.close();
                    }
                }
            });
        if let Some(index) = picked
            && index != *self.chosen
        {
            *self.chosen = index;
            response.mark_changed();
        }
        response
    }
}

/// A dialog over the page: the shade over everything else, a box in the popover ground with its
/// title and description, its body, and a footer band holding its actions at the end.
pub struct Dialog {
    id: Id,
    title: String,
    description: Option<String>,
    width: f32,
    /// A full-screen sheet on a phone; a confirm dialog stays a box sized to its words.
    sheet: bool,
}

impl Dialog {
    pub fn new(id: impl egui::AsIdSalt, title: impl Into<String>) -> Self {
        Self {
            id: Id::unique(id),
            title: title.into(),
            description: None,
            width: 512.0,
            sheet: true,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// The box's width, 512 by default (sokkel-ui's `md`): 384 `sm`, 672 `lg`, 896 `xl`. It is at
    /// most the window less 32.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Show the dialog; `body` draws its content and `footer` its actions, laid out from the end.
    /// True when it asks to close: Escape, or a press on the shade.
    pub fn show(self, ctx: &Context, body: impl FnOnce(&mut Ui), footer: impl FnOnce(&mut Ui)) -> bool {
        self.draw(ctx, Some(body), footer)
    }

    /// The dialog with no body: its title and description, 16 over the footer.
    fn draw(self, ctx: &Context, body: Option<impl FnOnce(&mut Ui)>, footer: impl FnOnce(&mut Ui)) -> bool {
        let t = tokens(ctx);
        // below 768 a sheet over the whole window, with no corners, edge or shadow
        let sheet = self.sheet && super::is_phone(ctx);
        let frame = if sheet {
            Frame::new().fill(super::ground(&t, "dialog.base.rest.box", "popover"))
        } else {
            super::dialog_frame(&t)
        };
        let response = egui::Modal::new(self.id)
            .backdrop_color(
                t.paint("dialog.base.rest.backdrop")
                    .and_then(|p| p.ground)
                    .unwrap_or_else(|| t.color_at("shade", 0.1)),
            )
            .frame(frame)
            .show(ctx, |ui| {
                let window = ctx.content_rect();
                let width = if sheet {
                    window.width()
                } else {
                    self.width.min(window.width() - 32.0)
                };
                // a sheet's body scrolls in all the window leaves; a box's in the window less 64
                let room_down = if sheet { 0.0 } else { 64.0 };
                // the footer: 16 over and under a control (a finger tall on a phone), and its rule
                let band_height = 1.0 + 16.0 + super::finger_tall(ctx, &t, t.density("control")) + 16.0;
                let top = ui.cursor().top();
                ui.set_width(width);
                ui.spacing_mut().item_spacing.y = 0.0;
                Frame::new().inner_margin(Margin::same(16)).show(ui, |ui| {
                    ui.set_width(width - 32.0);
                    ui.spacing_mut().item_spacing.y = 8.0;
                    ui.label(
                        TextRole::DetailTitle
                            .rich(&t, &self.title)
                            .color(t.color("popover-foreground")),
                    );
                    if let Some(description) = &self.description {
                        ui.label(TextRole::Body.rich(&t, description).color(t.color("muted-foreground")));
                    }
                    if let Some(body) = body {
                        ui.add_space(8.0);
                        // the box at most the window less 64 tall: the body scrolls in what is left
                        let room = window.height() - room_down - (ui.cursor().top() - top) - 16.0 - band_height;
                        egui::ScrollArea::vertical()
                            .id_salt(self.id.with("body"))
                            .max_height(room.max(t.density("control")))
                            .auto_shrink([false, true])
                            .show(ui, body);
                    }
                });
                // a sheet's footer stands at the window's foot
                if sheet {
                    let used = ui.cursor().top() - top;
                    ui.add_space((window.height() - used - band_height).max(0.0));
                }
                // the footer band: muted at half, a rule above, the surface's lower corners
                let (band, _) = ui.allocate_exact_size(vec2(width, band_height), Sense::hover());
                let radius = if sheet { CornerRadius::ZERO } else { t.radius("surface") };
                ui.painter().rect_filled(
                    band,
                    CornerRadius { nw: 0, ne: 0, ..radius },
                    t.paint("dialog.base.rest.footer")
                        .and_then(|p| p.ground)
                        .unwrap_or_else(|| t.color_at("muted", 0.5)),
                );
                ui.painter()
                    .hline(band.x_range(), band.top() + 0.5, Stroke::new(1.0, t.color("border")));
                let inner = Rect::from_min_max(band.min + vec2(16.0, 0.0), band.max - vec2(16.0, 0.0));
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(inner)
                        .layout(Layout::right_to_left(Align::Center)),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        footer(ui);
                    },
                );
            });
        response.should_close()
    }
}

/// What a confirm dialog was answered this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmAnswer {
    /// Cancel, Escape or a press on the shade.
    Cancel,
    Confirm,
    /// Stop waiting, or Escape, once a busy answer has run `busy-release`: the dialog lets the work
    /// go, and the app stops it if it can and says how it ended elsewhere.
    StopWaiting,
}

/// A question with one answer to confirm, after sokkel-ui's `ConfirmDialog`: the title is the
/// question ("Reset the demo data?"), a line under it says what the answer does, and the footer
/// holds Cancel and the answer, drawn destructive when it is one. While the answer is busy the
/// dialog holds; after `busy-release` (5 seconds) Cancel says "Stop waiting" and lets it go, as
/// Escape does, while the shade still holds and the answer stays busy.
pub struct ConfirmDialog {
    id: Id,
    question: String,
    detail: Option<String>,
    confirm: String,
    destructive: bool,
    busy: Option<String>,
    cancel: String,
    stop_waiting: String,
    busy_release: Option<f32>,
}

impl ConfirmDialog {
    pub fn new(id: impl egui::AsIdSalt, question: impl Into<String>, confirm: impl Into<String>) -> Self {
        Self {
            id: Id::unique(id),
            question: question.into(),
            detail: None,
            confirm: confirm.into(),
            destructive: false,
            busy: None,
            cancel: "Cancel".into(),
            stop_waiting: "Stop waiting".into(),
            busy_release: None,
        }
    }

    /// The word Cancel says once a busy answer has run `busy-release`.
    pub fn stop_waiting(mut self, words: impl Into<String>) -> Self {
        self.stop_waiting = words.into();
        self
    }

    /// How long, in milliseconds, a busy answer holds the dialog; `busy-release` by default.
    pub fn busy_release(mut self, ms: f32) -> Self {
        self.busy_release = Some(ms);
        self
    }

    /// One line on what the answer does, where the question does not say it.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Draw the answer as a destructive one.
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }

    /// While the answer is carried out: the answer says `label` and nothing can be pressed.
    pub fn busy(mut self, label: Option<String>) -> Self {
        self.busy = label;
        self
    }

    /// The word on the button that leaves the question unanswered.
    pub fn cancel(mut self, cancel: impl Into<String>) -> Self {
        self.cancel = cancel.into();
        self
    }

    /// Show the question; `Some` once it is answered.
    pub fn show(self, ctx: &Context) -> Option<ConfirmAnswer> {
        let mut answer = None;
        let busy = self.busy.is_some();
        // how long the answer has been busy, on the context's clock, to release the dialog after a wait
        let since = self.id.with("busy-since");
        let now = ctx.input(|i| i.time);
        let released = if busy {
            let started = ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(since, || now));
            let wait = f64::from(
                self.busy_release
                    .or_else(|| crate::tokens(ctx).duration("busy-release"))
                    .unwrap_or(5000.0),
            ) / 1000.0;
            let left = started + wait - now;
            if left > 0.0 {
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
            }
            left <= 0.0
        } else {
            ctx.data_mut(|d| d.remove_temp::<f64>(since));
            false
        };
        // sokkel-ui's ConfirmDialog is `sm`, and a box on a phone too
        let mut dialog = Dialog::new(self.id, self.question.clone()).width(384.0);
        dialog.sheet = false;
        if let Some(detail) = &self.detail {
            dialog = dialog.description(detail.clone());
        }
        let closed = dialog.draw(ctx, None::<fn(&mut Ui)>, |ui| {
            let variant = if self.destructive {
                ButtonVariant::Destructive
            } else {
                ButtonVariant::Default
            };
            let label = self.busy.clone().unwrap_or_else(|| self.confirm.clone());
            if ui.add(Button::new(label).variant(variant).busy(busy)).clicked() && !busy {
                answer = Some(ConfirmAnswer::Confirm);
            }
            let words = if released { &self.stop_waiting } else { &self.cancel };
            let cancel = Button::new(words.clone()).variant(ButtonVariant::Ghost);
            if ui.add_enabled(!busy || released, cancel).clicked() {
                answer = Some(if released {
                    ConfirmAnswer::StopWaiting
                } else {
                    ConfirmAnswer::Cancel
                });
            }
        });
        if released && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            answer = answer.or(Some(ConfirmAnswer::StopWaiting));
        }
        answer.or((closed && !busy).then_some(ConfirmAnswer::Cancel))
    }
}

/// What the unsaved-changes question says, as sokkel-ui's `UnsavedChangesWords`.
#[derive(Clone, Debug)]
pub struct UnsavedChangesWords {
    pub question: String,
    pub detail: String,
    /// The answer that goes back to the changes.
    pub keep: String,
    /// The answer that lets them go.
    pub discard: String,
}

impl Default for UnsavedChangesWords {
    /// The question a form asks before it closes with changes.
    fn default() -> Self {
        Self {
            question: "Discard your changes?".into(),
            detail: "What you changed in this form has not been saved.".into(),
            keep: "Keep editing".into(),
            discard: "Discard".into(),
        }
    }
}

impl UnsavedChangesWords {
    /// The question asked before the app leaves a page with unsaved changes, as `LEAVE_WORDS`.
    pub fn leave() -> Self {
        Self {
            question: "Leave without saving?".into(),
            detail: "What you changed on this page has not been saved.".into(),
            keep: "Stay".into(),
            discard: "Leave".into(),
        }
    }
}

/// The question asked before unsaved changes are let go, after sokkel-ui's `UnsavedChanges`: a
/// confirm dialog whose destructive answer discards them. True to let them go, false to keep them,
/// `None` while it waits.
pub fn unsaved_changes(ctx: &Context, id: impl egui::AsIdSalt, words: &UnsavedChangesWords) -> Option<bool> {
    ConfirmDialog::new(id, &words.question, &words.discard)
        .detail(&words.detail)
        .cancel(&words.keep)
        .destructive(true)
        .show(ctx)
        .map(|answer| answer == ConfirmAnswer::Confirm)
}
