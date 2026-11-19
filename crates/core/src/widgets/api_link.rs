//! A small chip naming the request behind a screen, after sokkel-ui's `ApiLink`.

use egui::{Response, Sense, Stroke, TextWrapMode, Ui, Widget, WidgetText, pos2, vec2};

use crate::{Icon, icons, tokens};

/// The words an API link says.
#[derive(Clone, Debug)]
pub struct ApiLinkWords {
    pub copy: String,
    pub copied: String,
    /// Said after the path to a screen reader when the chip opens the address.
    pub opens: String,
}

impl Default for ApiLinkWords {
    fn default() -> Self {
        Self {
            copy: "Copy the request".into(),
            copied: "Copied".into(),
            opens: "opens in a new tab".into(),
        }
    }
}

/// The request behind a screen, its method and path: `GET /api/jobs/nightly-export`, 24 tall on a
/// faint ground, and a button at its end that copies the method and the path (or `copy_value`).
/// The method is quiet for a read, in the warning ink for a write and the critical ink for a
/// delete. With `href` the path is a link; with `open` the whole chip is one link that opens
/// `href` (or the path) in a new tab, in place of the copy button. The path is cut to its room,
/// its whole text on hover.
#[must_use = "add it with `ui.add(...)`"]
pub struct ApiLink {
    method: String,
    path: String,
    href: Option<String>,
    open: bool,
    copy_value: Option<String>,
    words: ApiLinkWords,
}

impl ApiLink {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            method: "GET".into(),
            path: path.into(),
            href: None,
            open: false,
            copy_value: None,
            words: ApiLinkWords::default(),
        }
    }

    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.method = method.into().to_uppercase();
        self
    }

    /// Makes the path a link that opens in a new tab: the API's reference or the raw response.
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// The whole chip is one link that opens `href` (or the path) in a new tab, padded 8 at both
    /// ends, with an arrow after the path, in place of the copy button.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// What the button copies in place of the method and the path, such as a curl line.
    pub fn copy_value(mut self, value: impl Into<String>) -> Self {
        self.copy_value = Some(value.into());
        self
    }

    pub fn words(mut self, words: ApiLinkWords) -> Self {
        self.words = words;
        self
    }
}

/// The focus outline as the chip and its link draw it: `focus-ring-width` in `ring`, no offset.
fn ring(painter: &egui::Painter, t: &crate::Tokens, rect: egui::Rect, radius: egui::CornerRadius) {
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(t.length("focus-ring-width"), t.color("ring")),
        egui::StrokeKind::Outside,
    );
}

impl Widget for ApiLink {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let size = crate::mono_size(&t, t.text("help").size);
        let line = t.text("help").line_height;
        let tone = match self.method.as_str() {
            "DELETE" => t.color("critical-ink"),
            "POST" | "PUT" | "PATCH" => t.color("warning-ink"),
            _ => t.color("muted-foreground"),
        };
        let lay = |ui: &Ui, text: &str, weight: f32, width: f32| {
            let font = crate::mono_font(ui.ctx(), size, weight);
            WidgetText::from(egui::RichText::new(text).font(font.clone()).line_height(Some(line))).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                width.max(1.0),
                font,
            )
        };
        let method = lay(ui, &self.method, 500.0, f32::INFINITY);
        let copy_side = t.density("control-xs");
        let room = ui.available_width();
        // 8 in, the method, 6, the path, then 6 and the copy button, or with open 6, the arrow, 8;
        // a linked path carries its arrow 2 after it
        let end = if self.open { 6.0 + 12.0 + 8.0 } else { 6.0 + copy_side };
        let path_arrow = if self.href.is_some() && !self.open {
            2.0 + 12.0
        } else {
            0.0
        };
        let fixed = 8.0 + method.size().x + 6.0 + path_arrow + end;
        let path = lay(ui, &self.path, 400.0, room - fixed);
        let width = (fixed + path.size().x).min(room);
        let sense = if self.open { Sense::click() } else { Sense::hover() };
        let (rect, response) = ui.allocate_exact_size(vec2(width, 24.0), sense);
        let radius = t.radius("chip");
        let target = self.href.clone().unwrap_or_else(|| self.path.clone());
        if self.open {
            let name = format!("{} {}, {}", self.method, self.path, self.words.opens);
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, true, &name));
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(&target));
            }
        } else {
            let name = format!("{} {}", self.method, self.path);
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &name));
        }
        let ground = if self.open && response.hovered() {
            t.color("hover")
        } else {
            t.color_at("muted", 0.5)
        };
        let painter = ui.painter().clone();
        painter.rect(
            rect,
            radius,
            ground,
            Stroke::new(1.0, t.color("border")),
            egui::StrokeKind::Inside,
        );
        let mut x = rect.left() + 8.0;
        let method_width = method.size().x;
        painter.galley(pos2(x, rect.center().y - method.size().y / 2.0), method, tone);
        x += method_width + 6.0;
        let path_rect = egui::Rect::from_min_size(pos2(x, rect.center().y - path.size().y / 2.0), path.size());
        let cut = path.elided;
        let ink = t.color("foreground");
        painter.galley(path_rect.min, path.clone(), ink);
        x = path_rect.right();
        let mut response = response;
        if self.open {
            icons::paint(
                &painter,
                pos2(x + 6.0 + 6.0, rect.center().y),
                Icon::ArrowUpRight,
                12.0,
                t.color("muted-foreground"),
            );
            if response.has_focus() {
                ring(&painter, &t, rect, radius);
            }
            return response.on_hover_text(self.path);
        }
        // the path: a link with its arrow when it has an address, else words with their whole text
        // on hover
        if let Some(href) = &self.href {
            let link_rect = egui::Rect::from_min_max(path_rect.min, pos2(x + path_arrow, path_rect.max.y));
            let link = ui.interact(link_rect, response.id.with("path"), Sense::click());
            let name = format!("{}, {}", self.path, self.words.opens);
            link.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, true, &name));
            icons::paint(
                &painter,
                pos2(x + 2.0 + 6.0, rect.center().y),
                Icon::ArrowUpRight,
                12.0,
                ink,
            );
            if link.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                painter.hline(path_rect.x_range(), path_rect.bottom() - 2.0, Stroke::new(1.0, ink));
            }
            if link.has_focus() {
                ring(&painter, &t, link_rect, t.radius("mini"));
            }
            if link.clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(href));
            }
            link.on_hover_text(self.path.clone());
        } else {
            let words = ui.interact(path_rect, response.id.with("path"), Sense::hover());
            if cut || words.hovered() {
                words.on_hover_text(self.path.clone());
            }
        }
        // the copy button: the button part's `icon-xs`, at the chip's end
        let copied_id = response.id.with("copied");
        let copied_at: Option<f64> = ui.data(|d| d.get_temp(copied_id));
        let now = ui.input(|i| i.time);
        let copied = copied_at.is_some_and(|at| now - at < 1.5);
        let label = if copied { &self.words.copied } else { &self.words.copy };
        let button = egui::Rect::from_min_size(pos2(rect.right() - copy_side, rect.top()), vec2(copy_side, 24.0));
        let hit = ui
            .scope_builder(egui::UiBuilder::new().max_rect(button), |ui| {
                ui.add(super::IconButton::new(if copied { Icon::Check } else { Icon::Copy }, label.clone()).xs())
            })
            .inner;
        if hit.clicked() {
            let value = self
                .copy_value
                .clone()
                .unwrap_or_else(|| format!("{} {}", self.method, self.path));
            ui.ctx().copy_text(value);
            ui.data_mut(|d| d.insert_temp(copied_id, now));
        }
        if copied {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
        }
        response = response.union(hit);
        response
    }
}
