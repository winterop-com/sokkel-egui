//! Short notes that come and go at the window's corner, after sokkel-ui's `Toaster` (base-ui's
//! toast): an app adds one with [`Toast::show`] and draws them all once a frame with [`Toaster`].

use egui::{Context, Id, Order, Rect, Sense, Stroke, StrokeKind, UiBuilder, pos2, vec2};

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
}

/// The toasts shown, newest last, in the context's data.
#[derive(Clone, Default)]
struct Shown {
    next: u64,
    toasts: Vec<(u64, Toast, f64)>,
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

/// Closes a toast before its time.
pub fn close_toast(ctx: &Context, id: u64) {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Shown>(store())
            .toasts
            .retain(|(i, ..)| *i != id)
    });
}

/// The toasts shown, oldest first.
pub fn toasts(ctx: &Context) -> Vec<Toast> {
    ctx.data(|d| d.get_temp::<Shown>(store()))
        .unwrap_or_default()
        .toasts
        .into_iter()
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
        if shown.toasts.is_empty() {
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
            width - pad * 2.0 - icon - gap - close_side
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
        let mut closed = Vec::new();
        let mut hovered = false;
        let area = egui::Area::new(store().with("area"))
            .order(Order::Tooltip)
            .fixed_pos(window.min)
            .interactable(true);
        area.show(ctx, |ui| {
            ui.set_clip_rect(window);
            // where each stands, the front's first
            let mut rects = Vec::new();
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
                rects.push(rect);
            }
            let whole = rects
                .iter()
                .fold(Rect::NOTHING, |all, r| all.union(*r))
                .expand(gap / 2.0);
            hovered = ui.rect_contains_pointer(whole);
            let frame = t.radius("surface");
            let shadow = t.shadow("elevation-pop");
            // behind ones first, so the front is drawn over them
            for (index, ((id, toast), rect)) in front.iter().zip(&rects).enumerate().rev() {
                let painter = ui.painter();
                painter.add(shadow.as_shape(*rect, frame));
                painter.rect(
                    *rect,
                    frame,
                    t.color("popover"),
                    Stroke::new(1.0, t.color("border")),
                    StrokeKind::Inside,
                );
                if !expanded && index > 0 {
                    continue;
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
                    icons::paint(painter, pos2(x + 8.0, content.center().y), glyph, 16.0, ink);
                    x += 16.0 + gap;
                }
                let w = text_width(toast);
                let mut job_ui = ui.new_child(
                    UiBuilder::new()
                        .max_rect(Rect::from_min_size(pos2(x, content.top()), vec2(w, content.height())))
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                let lift = ((content.height() - texts[index]) / 2.0).max(0.0);
                job_ui.add_space(lift);
                job_ui.spacing_mut().item_spacing.y = 4.0;
                job_ui.add(
                    egui::Label::new(
                        TextRole::Action
                            .rich(&t, &toast.title)
                            .color(t.color("popover-foreground")),
                    )
                    .wrap(),
                );
                if let Some(description) = &toast.description {
                    job_ui.add(
                        egui::Label::new(TextRole::Body.rich(&t, description).color(t.color("muted-foreground")))
                            .wrap(),
                    );
                }
                let close = Rect::from_center_size(
                    pos2(content.right() - close_side / 2.0, content.center().y),
                    vec2(close_side, close_side),
                );
                let mut close_ui = ui.new_child(UiBuilder::new().max_rect(close).id_salt(("toast-close", id)));
                if close_ui.add(IconButton::new(Icon::X, &self.words.close)).clicked() {
                    closed.push(*id);
                }
            }
            // the toasts' ground takes the pointer, so what is under them is not pressed through
            let _ = ui.interact(whole, store().with("ground"), Sense::hover());
        });
        if hovered != expanded {
            ctx.data_mut(|d| d.insert_temp(expanded_id, hovered));
        }

        let dt = ctx.input(|i| i.stable_dt as f64).min(0.1);
        shown.toasts.retain(|(id, _, left)| !closed.contains(id) && *left > 0.0);
        if !hovered {
            for (_, _, left) in &mut shown.toasts {
                *left -= dt;
            }
        }
        if let Some(soonest) = shown
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
