//! A JSON value as a tree that folds, after sokkel-ui's `JsonTree`.

use egui::{Sense, Stroke, Ui, pos2, vec2};
use serde_json::Value;

use crate::{Icon, TextRole, icons, tokens};

/// The words a JSON tree says: how many keys a folded object holds, and the step that shows more.
#[derive(Clone, Debug)]
pub struct JsonTreeWords {
    /// `{count} keys`, `{count} key` for one.
    pub keys: String,
    pub key: String,
    /// `Show {count} more`.
    pub more: String,
}

impl Default for JsonTreeWords {
    fn default() -> Self {
        Self {
            keys: "{count} keys".into(),
            key: "{count} key".into(),
            more: "Show {count} more".into(),
        }
    }
}

/// A JSON value as a tree in the code face at the small size, on lines 20 tall: an object or a list
/// opens and folds under a chevron, a folded one saying how many it holds; open, its entries stand
/// under a rule, 16 in. Strings are green, numbers blue, true and false violet, null muted. Nested
/// values open `open` levels deep at first, and a long list or object shows `limit` entries with a
/// step that shows as many more.
pub struct JsonTree<'a> {
    id: egui::Id,
    value: &'a Value,
    open: usize,
    limit: usize,
    words: JsonTreeWords,
}

impl<'a> JsonTree<'a> {
    pub fn new(id: impl egui::AsIdSalt, value: &'a Value) -> Self {
        Self {
            id: egui::Id::unique(id),
            value,
            open: 2,
            limit: 100,
            words: JsonTreeWords::default(),
        }
    }

    /// How many levels stand open at first.
    pub fn open(mut self, open: usize) -> Self {
        self.open = open;
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }

    pub fn words(mut self, words: JsonTreeWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            self.node(ui, self.id, None, self.value, 0);
        })
        .response
    }

    fn node(&self, ui: &mut Ui, id: egui::Id, name: Option<&str>, value: &Value, depth: usize) {
        let t = tokens(ui.ctx());
        let mono = crate::mono_font(
            ui.ctx(),
            crate::mono_small_size(&t, TextRole::Help.font(&t).size),
            400.0,
        );
        let text = |s: &str, ink: &str| egui::RichText::new(s).font(mono.clone()).color(t.color(ink));
        let line = |ui: &mut Ui, parts: Vec<egui::RichText>, lead: f32| -> egui::Response {
            let mut job = egui::text::LayoutJob::default();
            for part in parts {
                part.line_height(Some(20.0)).append_to(
                    &mut job,
                    ui.style(),
                    egui::FontSelection::Default,
                    egui::Align::Center,
                );
            }
            job.wrap.max_width = (ui.available_width() - lead).max(1.0);
            let galley = ui.fonts_mut(|f| f.layout_job(job));
            let (rect, response) =
                ui.allocate_exact_size(vec2(ui.available_width(), galley.size().y.max(20.0)), Sense::hover());
            ui.painter()
                .galley(pos2(rect.left() + lead, rect.top()), galley, t.color("foreground"));
            response
        };
        let key = name.map(|n| text(&format!("{n}: "), "foreground"));
        let (entries, list): (Vec<(String, &Value)>, bool) = match value {
            Value::Array(items) => (
                items.iter().enumerate().map(|(i, v)| (i.to_string(), v)).collect(),
                true,
            ),
            Value::Object(map) => (map.iter().map(|(k, v)| (k.clone(), v)).collect(), false),
            scalar => {
                let shown = match scalar {
                    Value::Null => text("null", "muted-foreground"),
                    Value::String(s) => text(&format!("\"{s}\""), "kind-green-ink"),
                    Value::Number(n) => text(&n.to_string(), "kind-blue-ink"),
                    Value::Bool(b) => text(&b.to_string(), "kind-violet-ink"),
                    _ => text("", "muted-foreground"),
                };
                line(ui, key.into_iter().chain([shown]).collect(), 16.0);
                return;
            }
        };
        let open_id = id.with("open");
        let mut expanded: bool = ui.data(|d| d.get_temp(open_id)).unwrap_or(depth < self.open);
        let (start, end) = if list { ("[", "]") } else { ("{", "}") };
        let mut parts: Vec<egui::RichText> = key.into_iter().collect();
        parts.push(text(start, "muted-foreground"));
        if !expanded {
            let said = if list {
                entries.len().to_string()
            } else {
                let words = if entries.len() == 1 {
                    &self.words.key
                } else {
                    &self.words.keys
                };
                words.replace("{count}", &entries.len().to_string())
            };
            parts.push(text(&format!(" {said} "), "faint"));
            parts.push(text(end, "muted-foreground"));
        }
        let head = line(ui, parts, 14.0);
        let hit = ui.interact(head.rect, id.with("toggle"), Sense::click());
        hit.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, expanded, name.unwrap_or(start)));
        if hit.hovered() {
            ui.painter().rect_filled(head.rect, t.radius("sm"), t.color("hover"));
        }
        icons::paint(
            ui.painter(),
            pos2(head.rect.left() + 7.0, head.rect.center().y),
            if expanded {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            },
            14.0,
            t.color("muted-foreground"),
        );
        if hit.clicked() {
            expanded = !expanded;
            ui.data_mut(|d| d.insert_temp(open_id, expanded));
        }
        if !expanded {
            return;
        }
        let shown_id = id.with("shown");
        let mut shown: usize = ui.data(|d| d.get_temp(shown_id)).unwrap_or(self.limit);
        let body = ui
            .horizontal(|ui| {
                ui.add_space(7.0 + 4.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (child, item) in entries.iter().take(shown) {
                        let label = (!list).then_some(child.as_str());
                        self.node(ui, id.with(child), label, item, depth + 1);
                    }
                    let step = self.limit.min(entries.len().saturating_sub(shown));
                    if step > 0 {
                        let said = self.words.more.replace("{count}", &step.to_string());
                        let more = line(ui, vec![text(&said, "primary-ink")], 16.0);
                        let press = ui.interact(more.rect, id.with("more"), Sense::click());
                        if press.clicked() {
                            shown += self.limit;
                            ui.data_mut(|d| d.insert_temp(shown_id, shown));
                        }
                    }
                })
            })
            .response;
        ui.painter().vline(
            body.rect.left() + 7.0,
            body.rect.y_range(),
            Stroke::new(1.0, t.color("border")),
        );
        line(ui, vec![text(end, "muted-foreground")], 16.0);
    }
}
