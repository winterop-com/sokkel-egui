//! What opens over the page: a tooltip, a select's list, and a dialog, after sokkel-ui's `ui/`
//! tooltip, select and dialog.

use egui::{
    Align, Context, CornerRadius, Frame, Id, Layout, Margin, Popup, PopupCloseBehavior, PopupKind, Rect, RectAlign,
    Response, Sense, Shadow, Stroke, StrokeKind, TextWrapMode, Ui, Widget, WidgetText, pos2, vec2,
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
    let show = if response.hovered() {
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
        .fill(t.color("foreground"))
        .corner_radius(t.radius("md"))
        .inner_margin(Margin::symmetric(12, 6));
    Popup::from_response(&response)
        .kind(PopupKind::Tooltip)
        .open(show)
        .frame(frame)
        .gap(4.0)
        .align(RectAlign::BOTTOM)
        .show(|ui| {
            ui.set_max_width(320.0);
            ui.label(TextRole::Help.rich(&t, &text).color(t.color("background")));
        });
    response
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
        }
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
    fn lead(&self) -> f32 {
        if self.swatches.is_some() { 36.0 } else { 0.0 }
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
        let height = t.density("control");
        let (rect, mut response) = ui.allocate_exact_size(vec2(self.width, height), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, &self.label));
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
        let current = self.options.get(*self.chosen).cloned().unwrap_or_default();
        let lead = self.lead();
        if let Some(swatches) = &self.swatches {
            paint_swatch(
                painter,
                &t,
                pos2(rect.left() + 10.0, rect.center().y),
                swatches.get(*self.chosen).copied().flatten(),
            );
        }
        let galley = WidgetText::from(TextRole::Body.rich(&t, current)).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            rect.width() - 40.0 - lead,
            TextRole::Body.font(&t),
        );
        painter.galley(
            pos2(rect.left() + 10.0 + lead, rect.center().y - galley.size().y / 2.0),
            galley,
            t.color("foreground"),
        );
        icons::paint(
            painter,
            pos2(rect.right() - 16.0, rect.center().y),
            Icon::ChevronDown,
            16.0,
            t.color("muted-foreground"),
        );
        focus_ring(painter, &t, rect, radius, &response);

        let frame = Frame::new()
            .fill(t.color("popover"))
            .stroke(Stroke::new(1.0, t.color_at("foreground", 0.1)))
            .corner_radius(t.radius("surface"))
            .shadow(Shadow {
                offset: [0, 4],
                blur: 12,
                spread: 0,
                color: egui::Color32::from_black_alpha(if t.dark { 90 } else { 24 }),
            })
            .inner_margin(Margin::same(4));
        let mut picked = None;
        Popup::menu(&response)
            .id(popup_id)
            .frame(frame)
            .gap(4.0)
            .width(self.width)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for (index, option) in self.options.iter().enumerate() {
                    let (row, hit) = ui.allocate_exact_size(vec2(ui.available_width(), height - 4.0), Sense::click());
                    let chosen = index == *self.chosen;
                    hit.widget_info(|| {
                        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, chosen, option)
                    });
                    if hit.hovered() || hit.has_focus() {
                        ui.painter().rect_filled(row, t.radius("md"), t.color("hover"));
                    }
                    if let Some(swatches) = &self.swatches {
                        paint_swatch(
                            ui.painter(),
                            &t,
                            pos2(row.left() + 8.0, row.center().y),
                            swatches.get(index).copied().flatten(),
                        );
                    }
                    let galley = WidgetText::from(TextRole::Body.rich(&t, option)).into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        row.width() - 40.0 - lead,
                        TextRole::Body.font(&t),
                    );
                    ui.painter().galley(
                        pos2(row.left() + 8.0 + lead, row.center().y - galley.size().y / 2.0),
                        galley,
                        t.color("popover-foreground"),
                    );
                    if chosen {
                        icons::paint(
                            ui.painter(),
                            pos2(row.right() - 14.0, row.center().y),
                            Icon::Check,
                            16.0,
                            t.color("popover-foreground"),
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
}

impl Dialog {
    pub fn new(id: impl egui::AsId, title: impl Into<String>) -> Self {
        Self {
            id: Id::new(id),
            title: title.into(),
            description: None,
            width: 384.0,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

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
        let frame = Frame::new()
            .fill(t.color("popover"))
            .stroke(Stroke::new(1.0, t.color_at("foreground", 0.1)))
            .corner_radius(t.radius("surface"))
            .shadow(Shadow {
                offset: [0, 14],
                blur: 44,
                spread: 0,
                color: egui::Color32::from_black_alpha(if t.dark { 120 } else { 46 }),
            });
        let response = egui::Modal::new(self.id)
            .backdrop_color(t.color_at("shade", 0.1))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(self.width);
                ui.spacing_mut().item_spacing.y = 0.0;
                Frame::new().inner_margin(Margin::same(16)).show(ui, |ui| {
                    ui.set_width(self.width - 32.0);
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
                        body(ui);
                    }
                });
                // the footer band: muted at half, a rule above, the surface's lower corners
                let (band, _) = ui.allocate_exact_size(vec2(self.width, 64.0), Sense::hover());
                let radius = t.radius("surface");
                ui.painter()
                    .rect_filled(band, CornerRadius { nw: 0, ne: 0, ..radius }, t.color_at("muted", 0.5));
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
}

/// A question with one answer to confirm, after sokkel-ui's `ConfirmDialog`: the title is the
/// question ("Reset the demo data?"), a line under it says what the answer does, and the footer
/// holds Cancel and the answer, drawn destructive when it is one.
pub struct ConfirmDialog {
    id: Id,
    question: String,
    detail: Option<String>,
    confirm: String,
    destructive: bool,
    busy: Option<String>,
    cancel: String,
}

impl ConfirmDialog {
    pub fn new(id: impl egui::AsId, question: impl Into<String>, confirm: impl Into<String>) -> Self {
        Self {
            id: Id::new(id),
            question: question.into(),
            detail: None,
            confirm: confirm.into(),
            destructive: false,
            busy: None,
            cancel: "Cancel".into(),
        }
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
        let mut dialog = Dialog::new(self.id, self.question.clone());
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
            let cancel = Button::new(self.cancel.clone()).variant(ButtonVariant::Ghost);
            if ui.add_enabled(!busy, cancel).clicked() {
                answer = Some(ConfirmAnswer::Cancel);
            }
        });
        answer.or((closed && !busy).then_some(ConfirmAnswer::Cancel))
    }
}
