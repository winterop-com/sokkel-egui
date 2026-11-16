//! Short notes that come and go at the window's corner, after sokkel-ui's `Toaster` (base-ui's
//! toast): an app adds one with [`Toast::show`] and draws them all once a frame with [`Toaster`].

use egui::{Context, Id, Order, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2};

use crate::{Icon, IconButton, TextRole, icons, tokens};

/// What a toast says it is, shown by its icon: none for a plain one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToastKind {
    #[default]
    Plain,
    Success,
    Info,
    Warning,
    Error,
    Loading,
}

/// A toast to add: its kind, title and description, and how long it stays.
#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub kind: ToastKind,
    pub title: String,
    pub description: Option<String>,
    /// Seconds before it closes by itself, counted while it is not pointed at; `None` stays until
    /// closed. Five when not given, as base-ui's.
    pub timeout: Option<f64>,
    /// A button's words before the close, as the web's `actionProps`: pressed, the toast's id comes
    /// back from [`toast_acted`].
    pub action: Option<String>,
}

/// How long a toast takes to move, as the web's `transform 500ms`.
const MOVE: f64 = 0.5;
/// How far a toast is dragged down or right before it is let go, as base-ui's `SWIPE_THRESHOLD`.
const SWIPE: f32 = 40.0;

/// The web's `cubic-bezier(0.22, 1, 0.36, 1)`: fast away, settling softly.
fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let curve = |t: f32, a: f32, b: f32| 3.0 * (1.0 - t).powi(2) * t * a + 3.0 * (1.0 - t) * t * t * b + t.powi(3);
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for _ in 0..24 {
        let mid = (low + high) / 2.0;
        if curve(mid, 0.22, 0.36) < x {
            low = mid;
        } else {
            high = mid;
        }
    }
    curve((low + high) / 2.0, 1.0, 1.0)
}

/// A rect on its way from one place to another, eased over `MOVE`.
#[derive(Clone, Copy)]
struct Tween {
    from: Rect,
    to: Rect,
    since: f64,
}

impl Tween {
    fn at(&self, now: f64) -> Rect {
        let k = ease(((now - self.since) / MOVE) as f32);
        Rect::from_min_max(self.from.min.lerp(self.to.min, k), self.from.max.lerp(self.to.max, k))
    }

    fn done(&self, now: f64) -> bool {
        now - self.since >= MOVE
    }
}

/// A toast on its way out: where it stood, how far it goes, and whether its words showed.
#[derive(Clone)]
struct Leaving {
    toast: Toast,
    rect: Rect,
    to: Vec2,
    since: f64,
    words: bool,
}

/// The toasts shown, newest last, in the context's data: each with its time left, where each
/// stands, the ones leaving, those asked to close, and the one dragged and how far.
#[derive(Clone, Default)]
struct Shown {
    next: u64,
    toasts: Vec<(u64, Toast, f64)>,
    places: Vec<(u64, Tween)>,
    leaving: Vec<Leaving>,
    closing: Vec<u64>,
    dragged: Option<(u64, Vec2)>,
    /// A toast let go short of the swipe, springing back from where it was let go.
    back: Option<(u64, Vec2, f64)>,
    /// The toasts whose action was pressed, until [`toast_acted`] takes them.
    acted: Vec<u64>,
}

fn store() -> Id {
    Id::new("sokkel-toasts")
}

impl Toast {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            kind: ToastKind::Plain,
            title: title.into(),
            description: None,
            timeout: Some(5.0),
            action: None,
        }
    }

    pub fn success(title: impl Into<String>) -> Self {
        Self::new(title).kind(ToastKind::Success)
    }

    pub fn error(title: impl Into<String>) -> Self {
        Self::new(title).kind(ToastKind::Error)
    }

    pub fn kind(mut self, kind: ToastKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn timeout(mut self, seconds: Option<f64>) -> Self {
        self.timeout = seconds;
        self
    }

    /// An outline button with these words before the close, as the web's `ToastAction`.
    pub fn action(mut self, label: impl Into<String>) -> Self {
        self.action = Some(label.into());
        self
    }

    /// Shows the toast until it times out or is closed; returns its id, for [`close_toast`].
    pub fn show(self, ctx: &Context) -> u64 {
        ctx.data_mut(|d| {
            let shown = d.get_temp_mut_or_default::<Shown>(store());
            let id = shown.next;
            shown.next += 1;
            let left = self.timeout.unwrap_or(f64::INFINITY);
            shown.toasts.push((id, self, left));
            id
        })
    }
}

/// The toasts whose action was pressed since this was last asked, by the ids `Toast::show` gave;
/// each is closed as its action is pressed.
pub fn toast_acted(ctx: &Context) -> Vec<u64> {
    ctx.data_mut(|d| std::mem::take(&mut d.get_temp_mut_or_default::<Shown>(store()).acted))
}

/// Closes a toast before its time; it slides out, as one closed by its button.
pub fn close_toast(ctx: &Context, id: u64) {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Shown>(store()).closing.push(id));
    ctx.request_repaint();
}

/// The toasts shown, oldest first; one sliding out is gone already.
pub fn toasts(ctx: &Context) -> Vec<Toast> {
    let shown = ctx.data(|d| d.get_temp::<Shown>(store())).unwrap_or_default();
    shown
        .toasts
        .into_iter()
        .filter(|(id, ..)| !shown.closing.contains(id))
        .map(|(_, toast, _)| toast)
        .collect()
}

/// The words the toaster says.
#[derive(Clone, Debug)]
pub struct ToasterWords {
    pub close: String,
}

impl Default for ToasterWords {
    fn default() -> Self {
        Self {
            close: "Close toast".into(),
        }
    }
}

/// Draws the toasts, as sokkel-ui's `Toaster`: at the bottom right, 16 in, at most 384 wide (the
/// window's width less 16 a side below 640), the newest in front and two more peeking 12 above it,
/// each 10% smaller; pointed at, they stand apart, 12 between them, and their time stops. Each is
/// a popover with the surface's corners, its icon, title and description, and a close button.
/// They move as the web's, over half a second, quick then settling: a new one rises from below,
/// one closed or timed out sinks away, the rest close up; one dragged down or right past 40 is
/// swiped away that way, and one let go sooner springs back.
pub struct Toaster {
    words: ToasterWords,
    limit: usize,
}

impl Default for Toaster {
    fn default() -> Self {
        Self::new()
    }
}

impl Toaster {
    pub fn new() -> Self {
        Self {
            words: ToasterWords::default(),
            limit: 3,
        }
    }

    pub fn words(mut self, words: ToasterWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ctx: &Context) {
        let mut shown = ctx.data(|d| d.get_temp::<Shown>(store())).unwrap_or_default();
        if shown.toasts.is_empty() && shown.leaving.is_empty() {
            return;
        }
        let t = tokens(ctx);
        let window = ctx.content_rect();
        let width = if window.width() < 640.0 {
            window.width() - 32.0
        } else {
            384.0_f32.min(window.width() - 32.0)
        };
        let expanded_id = store().with("expanded");
        let expanded = ctx.data(|d| d.get_temp::<bool>(expanded_id)).unwrap_or(false);
        let from = shown.toasts.len().saturating_sub(self.limit);
        let front: Vec<(u64, Toast)> = shown.toasts[from..]
            .iter()
            .rev()
            .map(|(id, toast, _)| (*id, toast.clone()))
            .collect();

        let pad = 16.0;
        let gap = 12.0;
        let close_side = t.density("control");
        let text_width = |toast: &Toast| {
            let icon = if toast.kind == ToastKind::Plain {
                0.0
            } else {
                16.0 + gap
            };
            width - pad * 2.0 - icon - gap - close_side - action_width(ctx, &t, toast).map_or(0.0, |w| w + gap)
        };
        // each toast's words' height at the width it has
        let texts: Vec<f32> = front
            .iter()
            .map(|(_, toast)| {
                let w = text_width(toast);
                let title = ctx.fonts_mut(|f| {
                    f.layout(
                        toast.title.clone(),
                        TextRole::Action.font(&t),
                        t.color("popover-foreground"),
                        w,
                    )
                });
                let mut text = title.size().y;
                if let Some(description) = &toast.description {
                    let galley = ctx.fonts_mut(|f| {
                        f.layout(
                            description.clone(),
                            TextRole::Body.font(&t),
                            t.color("muted-foreground"),
                            w,
                        )
                    });
                    text += 4.0 + galley.size().y;
                }
                text
            })
            .collect();
        let heights: Vec<f32> = texts.iter().map(|text| pad * 2.0 + text.max(close_side)).collect();

        let right = window.right() - 16.0;
        let bottom = window.bottom() - 16.0;
        let left = right - width;
        let now = ctx.input(|i| i.time);
        // where each should stand, the front's first
        let mut targets = Vec::new();
        let mut y = bottom;
        for (index, height) in heights.iter().enumerate() {
            let rect = if expanded {
                let r = Rect::from_min_max(pos2(left, y - height), pos2(right, y));
                y -= height + gap;
                r
            } else {
                let scale = (1.0 - index as f32 * 0.1).max(0.0);
                let h = heights[0];
                let w = width * scale;
                // as the web's: lifted 12 a step and by what the scale took off, from the foot
                let foot = bottom - index as f32 * 12.0 - (1.0 - scale) * h;
                let center_x = left + width / 2.0;
                let top = foot - h * scale;
                Rect::from_min_max(pos2(center_x - w / 2.0, top), pos2(center_x + w / 2.0, foot))
            };
            targets.push(rect);
        }
        // each eases to its place; a new one rises from 150% of its height below it
        let mut places = Vec::new();
        let mut rects = Vec::new();
        for ((id, _), target) in front.iter().zip(&targets) {
            let tween = match shown.places.iter().find(|(i, _)| i == id) {
                Some((_, tween)) if tween.to == *target => *tween,
                Some((_, tween)) => Tween {
                    from: tween.at(now),
                    to: *target,
                    since: now,
                },
                None => Tween {
                    from: target.translate(vec2(0.0, target.height() * 1.5)),
                    to: *target,
                    since: now,
                },
            };
            let mut rect = tween.at(now);
            if let Some((i, offset)) = shown.dragged
                && i == *id
            {
                rect = rect.translate(offset);
            } else if let Some((i, from, since)) = shown.back
                && i == *id
            {
                rect = rect.translate(from * (1.0 - ease(((now - since) / MOVE) as f32)));
            }
            places.push((*id, tween));
            rects.push(rect);
        }

        let mut closed: Vec<(u64, Vec2)> = Vec::new();
        let mut pressed: Vec<u64> = Vec::new();
        let mut hovered = false;
        let mut swiped: Option<(u64, Vec2)> = None;
        let mut let_go: Option<(u64, Vec2)> = None;
        let dragged = shown.dragged;
        let area = egui::Area::new(store().with("area"))
            .order(Order::Tooltip)
            .fixed_pos(window.min)
            .interactable(true);
        area.show(ctx, |ui| {
            ui.set_clip_rect(window);
            let whole = targets
                .iter()
                .fold(Rect::NOTHING, |all, r| all.union(*r))
                .expand(gap / 2.0);
            hovered = ui.rect_contains_pointer(whole) || dragged.is_some();
            // the toasts' ground takes the pointer, so what is under them is not pressed through
            let _ = ui.interact(whole, store().with("ground"), Sense::hover());
            // the ones leaving behind the rest, each sliding the way it goes
            for leaving in &shown.leaving {
                let k = ease(((now - leaving.since) / MOVE) as f32);
                let rect = leaving.rect.translate(leaving.to * k);
                let _ = paint(ui, &t, rect, &leaving.toast, leaving.words, None, pad, gap, &text_width);
            }
            // behind ones first, so the front is drawn over them
            for (index, ((id, toast), rect)) in front.iter().zip(&rects).enumerate().rev() {
                let words = expanded || index == 0;
                // the toast itself is dragged, down or right, its close button aside
                if words {
                    let hold = ui.interact(*rect, store().with(("swipe", id)), Sense::drag());
                    if hold.dragged() {
                        let moved = dragged.filter(|(i, _)| i == id).map_or(Vec2::ZERO, |(_, o)| o) + hold.drag_delta();
                        swiped = Some((*id, vec2(moved.x.max(0.0), moved.y.max(0.0))));
                    } else if hold.drag_stopped()
                        && let Some((i, offset)) = dragged
                        && i == *id
                    {
                        let_go = Some((i, offset));
                    }
                }
                let close = if words { Some(self.words.close.as_str()) } else { None };
                let (shut, acted) = paint(
                    ui,
                    &t,
                    *rect,
                    toast,
                    words,
                    close.map(|c| (c, *id)),
                    pad,
                    gap,
                    &text_width,
                );
                if acted {
                    pressed.push(*id);
                }
                if shut || acted {
                    closed.push((*id, Vec2::ZERO));
                }
            }
        });
        if hovered != expanded {
            ctx.data_mut(|d| d.insert_temp(expanded_id, hovered));
        }
        shown.dragged = swiped;
        shown.acted.extend(pressed);
        if let Some((id, offset)) = let_go {
            if offset.y > SWIPE || offset.x > SWIPE {
                // past the swipe it carries on the way it was dragged, 150% of its size
                let rect = rects[front.iter().position(|(i, _)| *i == id).unwrap_or(0)];
                let far = if offset.y >= offset.x {
                    vec2(0.0, rect.height() * 1.5)
                } else {
                    vec2(rect.width() * 1.5, 0.0)
                };
                closed.push((id, far));
            } else {
                shown.back = Some((id, offset, now));
            }
        }
        if shown.back.is_some_and(|(_, _, since)| now - since >= MOVE) {
            shown.back = None;
        }

        let dt = ctx.input(|i| i.stable_dt as f64).min(0.1);
        if !hovered {
            for (_, _, left) in &mut shown.toasts {
                *left -= dt;
            }
        }
        // closed by its button, by the app, by its time or by a swipe: it slides out
        for (id, _, left) in &shown.toasts {
            if (*left <= 0.0 || shown.closing.contains(id)) && !closed.iter().any(|(i, _)| i == id) {
                closed.push((*id, Vec2::ZERO));
            }
        }
        for (id, far) in &closed {
            let Some(at) = shown.toasts.iter().position(|(i, ..)| i == id) else {
                continue;
            };
            let (_, toast, _) = shown.toasts.remove(at);
            let shown_at = front.iter().position(|(i, _)| i == id);
            let (rect, words) = match shown_at {
                Some(index) => (rects[index], expanded || index == 0),
                // past the three shown: it goes unseen
                None => continue,
            };
            let to = if *far == Vec2::ZERO {
                vec2(0.0, rect.height() * 1.5)
            } else {
                *far
            };
            shown.leaving.push(Leaving {
                toast,
                rect,
                to,
                since: now,
                words,
            });
        }
        shown.closing.clear();
        shown.leaving.retain(|l| now - l.since < MOVE);
        shown.places = places
            .into_iter()
            .filter(|(id, _)| shown.toasts.iter().any(|(i, ..)| i == id))
            .collect();

        let moving =
            !shown.leaving.is_empty() || shown.back.is_some() || shown.places.iter().any(|(_, tween)| !tween.done(now));
        if moving {
            ctx.request_repaint();
        } else if let Some(soonest) = shown
            .toasts
            .iter()
            .map(|(.., left)| *left)
            .filter(|left| left.is_finite())
            .reduce(f64::min)
        {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(soonest.clamp(0.0, 1.0)));
        }
        ctx.data_mut(|d| d.insert_temp(store(), shown));
    }
}

/// Draws one toast in `rect`: the popover, and with `words` its icon, title, description and, given
/// `close`, its close button; returns whether the close button was pressed.
#[allow(clippy::too_many_arguments)]
fn paint(
    ui: &mut Ui,
    t: &crate::Tokens,
    rect: Rect,
    toast: &Toast,
    words: bool,
    close: Option<(&str, u64)>,
    pad: f32,
    gap: f32,
    text_width: &dyn Fn(&Toast) -> f32,
) -> (bool, bool) {
    let frame = t.radius("surface");
    let painter = ui.painter().clone();
    painter.add(t.shadow("elevation-pop").as_shape(rect, frame));
    painter.rect(
        rect,
        frame,
        t.color("popover"),
        Stroke::new(1.0, t.color("border")),
        StrokeKind::Inside,
    );
    if !words {
        return (false, false);
    }
    let content = rect.shrink(pad);
    let mut x = content.left();
    let icon = match toast.kind {
        ToastKind::Plain => None,
        ToastKind::Success => Some((Icon::CircleCheck, t.color("popover-foreground"))),
        ToastKind::Info => Some((Icon::Info, t.color("popover-foreground"))),
        ToastKind::Warning => Some((Icon::TriangleAlert, t.color("popover-foreground"))),
        ToastKind::Error => Some((Icon::OctagonX, t.color("destructive"))),
        ToastKind::Loading => Some((Icon::LoaderCircle, t.color("popover-foreground"))),
    };
    if let Some((glyph, ink)) = icon {
        icons::paint(&painter, pos2(x + 8.0, content.center().y), glyph, 16.0, ink);
        x += 16.0 + gap;
    }
    let w = text_width(toast);
    // through the roles' rich text, so each keeps its weight
    let title = egui::WidgetText::from(TextRole::Action.rich(t, &toast.title)).into_galley(
        ui,
        Some(egui::TextWrapMode::Wrap),
        w,
        TextRole::Action.font(t),
    );
    let description = toast.description.as_ref().map(|d| {
        egui::WidgetText::from(TextRole::Body.rich(t, d)).into_galley(
            ui,
            Some(egui::TextWrapMode::Wrap),
            w,
            TextRole::Body.font(t),
        )
    });
    let text = title.size().y + description.as_ref().map_or(0.0, |g| 4.0 + g.size().y);
    let mut y = content.top() + ((content.height() - text) / 2.0).max(0.0);
    let title_height = title.size().y;
    painter.galley(Pos2::new(x, y), title, t.color("popover-foreground"));
    y += title_height + 4.0;
    if let Some(galley) = description {
        painter.galley(Pos2::new(x, y), galley, t.color("muted-foreground"));
    }
    // the close button, `icon-sm`, at the content's right edge, as the web's `ToastClose`
    let side = t.density("control-sm");
    let place = Rect::from_center_size(pos2(content.right() - side / 2.0, content.center().y), vec2(side, side));
    // the action, an outline small button, `gap` before the close
    let mut acted = false;
    if let (Some(label), Some(width)) = (&toast.action, action_width(ui.ctx(), t, toast)) {
        let spot = Rect::from_center_size(
            pos2(place.left() - gap - width / 2.0, content.center().y),
            vec2(width, t.density("control-sm")),
        );
        let id = close.map_or(0, |(_, id)| id);
        let mut action_ui = ui.new_child(UiBuilder::new().max_rect(spot).id_salt(("toast-action", id)));
        let button = crate::Button::new(label.as_str())
            .variant(crate::ButtonVariant::Outline)
            .size(crate::ButtonSize::Sm);
        acted = action_ui.put(spot, button).clicked() && close.is_some();
    }
    let shut = match close {
        Some((label, id)) => {
            let mut close_ui = ui.new_child(UiBuilder::new().max_rect(place).id_salt(("toast-close", id)));
            close_ui.put(place, IconButton::new(Icon::X, label)).clicked()
        }
        None => {
            icons::paint(&painter, place.center(), Icon::X, 16.0, t.color("muted-foreground"));
            false
        }
    };
    (shut, acted)
}

/// How wide a toast's action button is: its words in a small button's type, 10 in each side.
fn action_width(ctx: &Context, t: &crate::Tokens, toast: &Toast) -> Option<f32> {
    let label = toast.action.as_ref()?;
    let width = ctx.fonts_mut(|f| {
        f.layout_no_wrap(label.clone(), TextRole::Badge.font(t), egui::Color32::PLACEHOLDER)
            .size()
            .x
    });
    Some((width + 20.0).ceil())
}
