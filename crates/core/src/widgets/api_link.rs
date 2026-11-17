//! A small chip naming the request behind a screen, after sokkel-ui's `ApiLink`.

use egui::{Response, Sense, Stroke, TextWrapMode, Ui, Widget, WidgetText, pos2, vec2};

use crate::{Icon, TextRole, icons, tokens};

/// The words an API link says.
#[derive(Clone, Debug)]
pub struct ApiLinkWords {
    pub copy: String,
    pub copied: String,
}

impl Default for ApiLinkWords {
    fn default() -> Self {
        Self {
            copy: "Copy the request".into(),
            copied: "Copied".into(),
        }
    }
}

/// The request behind a screen, its method and path: `GET /forms/FRM-SITE`, 24 tall on a faint
/// ground, and a button at its end that copies the method and the path (or `copy_value`). The
/// method is quiet for a read, in the warning ink for a write and the critical ink for a delete.
#[must_use = "add it with `ui.add(...)`"]
pub struct ApiLink {
    method: String,
    path: String,
    copy_value: Option<String>,
    words: ApiLinkWords,
}

impl ApiLink {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            method: "GET".into(),
            path: path.into(),
            copy_value: None,
            words: ApiLinkWords::default(),
        }
    }

    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.method = method.into().to_uppercase();
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

impl Widget for ApiLink {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let mono = egui::FontId::monospace(TextRole::Help.font(&t).size);
        let tone = match self.method.as_str() {
            "DELETE" => t.color("critical-ink"),
            "POST" | "PUT" | "PATCH" => t.color("warning-ink"),
            _ => t.color("muted-foreground"),
        };
        let lay = |ui: &Ui, text: &str| {
            WidgetText::from(egui::RichText::new(text).font(mono.clone())).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                mono.clone(),
            )
        };
        let method = lay(ui, &self.method);
        let path = lay(ui, &self.path);
        // 8 in, the method, 6, the path, 6, a 24 copy button at the end
        let width = 8.0 + method.size().x + 6.0 + path.size().x + 6.0 + 24.0;
        let (rect, response) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
        let painter = ui.painter();
        painter.rect(
            rect,
            t.radius("chip"),
            t.color_at("muted", 0.5),
            Stroke::new(1.0, t.color("border")),
            egui::StrokeKind::Inside,
        );
        let mut x = rect.left() + 8.0;
        let method_width = method.size().x;
        painter.galley(pos2(x, rect.center().y - method.size().y / 2.0), method, tone);
        x += method_width + 6.0;
        painter.galley(
            pos2(x, rect.center().y - path.size().y / 2.0),
            path,
            t.color("foreground"),
        );
        let button = egui::Rect::from_min_size(pos2(rect.right() - 24.0, rect.top()), vec2(24.0, 24.0));
        let copied_id = response.id.with("copied");
        let copied_at: Option<f64> = ui.data(|d| d.get_temp(copied_id));
        let now = ui.input(|i| i.time);
        let copied = copied_at.is_some_and(|at| now - at < 1.5);
        let hit = ui.interact(button, response.id.with("copy"), Sense::click());
        let label = if copied { &self.words.copied } else { &self.words.copy };
        hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
        if hit.hovered() {
            ui.painter()
                .rect_filled(button.shrink(1.0), t.radius("chip"), t.color("hover"));
        }
        icons::paint(
            ui.painter(),
            button.center(),
            if copied { Icon::Check } else { Icon::Copy },
            12.0,
            t.color("muted-foreground"),
        );
        super::focus_ring(ui.painter(), &t, button, t.radius("chip"), &hit);
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
        hit.on_hover_text(label.clone()).union(response)
    }
}
