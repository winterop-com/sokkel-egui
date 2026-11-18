//! What goes with the thing an action removes or changes, after sokkel-ui's `ConsequenceList`.

use egui::{Rect, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{Icon, TextRole, icons, tokens};

/// One kind of thing an action takes with it: 12 runs, 3 schedules.
#[derive(Clone, Debug)]
pub struct Consequence {
    pub label: String,
    /// How many go, at the row's end.
    pub count: Option<usize>,
    /// One line on what that means, under the label.
    pub detail: Option<String>,
    pub icon: Option<Icon>,
}

impl Consequence {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            count: None,
            detail: None,
            icon: None,
        }
    }

    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// The words a consequence list says.
#[derive(Clone, Debug)]
pub struct ConsequenceListWords {
    /// The heading over things that are lost.
    pub heading: String,
    /// The heading over what an action changes, moves or keeps.
    pub changed: String,
}

impl Default for ConsequenceListWords {
    fn default() -> Self {
        Self {
            heading: "Also deleted".into(),
            changed: "What this changes".into(),
        }
    }
}

/// What goes with the thing an action removes or changes, as sokkel-ui's `ConsequenceList`: a small
/// heading at medium weight (Also deleted in the critical ink, or What this changes, muted) over a
/// row per kind, each its icon, its label, a line under it and its count at the end, 12 across and
/// 8 over and under, parted by rules, in a box on the critical wash for things lost. It sits in a
/// confirm's body, before the verb.
pub struct ConsequenceList<'a> {
    items: &'a [Consequence],
    critical: bool,
    heading: Option<String>,
    words: ConsequenceListWords,
}

impl<'a> ConsequenceList<'a> {
    pub fn new(items: &'a [Consequence]) -> Self {
        Self {
            items,
            critical: true,
            heading: None,
            words: ConsequenceListWords::default(),
        }
    }

    /// Only moved or kept, or facts, rather than lost: the neutral box and heading.
    pub fn neutral(mut self) -> Self {
        self.critical = false;
        self
    }

    pub fn heading(mut self, heading: impl Into<String>) -> Self {
        self.heading = Some(heading.into());
        self
    }

    pub fn words(mut self, words: ConsequenceListWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        if self.items.is_empty() {
            return;
        }
        let t = tokens(ui.ctx());
        let heading = self.heading.clone().unwrap_or_else(|| {
            if self.critical {
                self.words.heading.clone()
            } else {
                self.words.changed.clone()
            }
        });
        let spacing = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 6.0;
        let ink = if self.critical {
            t.color("critical-ink")
        } else {
            t.color("muted-foreground")
        };
        ui.label(TextRole::Help.rich(&t, heading).variation("wght", 500.0).color(ink));
        let width = ui.available_width();
        let rows: Vec<(f32, Option<std::sync::Arc<egui::Galley>>, std::sync::Arc<egui::Galley>)> =
            self.items
                .iter()
                .map(|item| {
                    let room = width - 24.0 - if item.icon.is_some() { 26.0 } else { 0.0 } - 40.0;
                    let label = WidgetText::from(TextRole::Body.rich(&t, &item.label).line_height(Some(20.0)))
                        .into_galley(ui, Some(TextWrapMode::Wrap), room.max(1.0), TextRole::Body.font(&t));
                    let detail = item.detail.as_ref().map(|d| {
                        WidgetText::from(TextRole::Help.rich(&t, d)).into_galley(
                            ui,
                            Some(TextWrapMode::Wrap),
                            room.max(1.0),
                            TextRole::Help.font(&t),
                        )
                    });
                    let height = 16.0 + label.size().y + detail.as_ref().map_or(0.0, |d| 2.0 + d.size().y);
                    (height, detail, label)
                })
                .collect();
        let total: f32 = rows.iter().map(|r| r.0).sum();
        let (rect, _) = ui.allocate_exact_size(vec2(width, total), Sense::hover());
        let radius = t.radius("control");
        let (ground, edge) = if self.critical {
            (t.color_at("critical", 0.04), t.color_at("critical", 0.3))
        } else {
            (egui::Color32::TRANSPARENT, t.color("border"))
        };
        ui.painter()
            .rect(rect, radius, ground, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
        let mut y = rect.top();
        for (index, (item, (height, detail, label))) in self.items.iter().zip(rows).enumerate() {
            let row = Rect::from_min_size(pos2(rect.left(), y), vec2(width, height));
            if index > 0 {
                ui.painter()
                    .hline(row.x_range(), row.top() + 0.5, Stroke::new(1.0, t.color("border")));
            }
            let mut x = row.left() + 12.0;
            if let Some(icon) = item.icon {
                icons::paint(
                    ui.painter(),
                    pos2(x + 8.0, row.top() + 8.0 + 10.0),
                    icon,
                    16.0,
                    t.color("muted-foreground"),
                );
                x += 16.0 + 10.0;
            }
            let label_height = label.size().y;
            ui.painter()
                .galley(pos2(x, row.top() + 8.0), label, t.color("foreground"));
            if let Some(detail) = detail {
                ui.painter().galley(
                    pos2(x, row.top() + 8.0 + label_height + 2.0),
                    detail,
                    t.color("muted-foreground"),
                );
            }
            if let Some(count) = item.count {
                let spec = t.text(TextRole::Body.key());
                let mono = crate::mono_font(ui.ctx(), crate::mono_size(&t, spec.size), spec.weight);
                let galley = ui
                    .painter()
                    .layout_no_wrap(super::table::thousands(count), mono, t.color("foreground"));
                ui.painter().galley(
                    pos2(row.right() - 12.0 - galley.size().x, row.top() + 8.0),
                    galley,
                    t.color("foreground"),
                );
            }
            y += height;
        }
        ui.spacing_mut().item_spacing.y = spacing;
    }
}
