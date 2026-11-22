//! An "on this page" list of headings, after sokkel-ui's `TableOfContents`.

use egui::{Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{TextRole, tokens};

/// One entry: the heading's id, its text and its level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentsEntry {
    pub id: String,
    pub label: String,
    pub level: usize,
}

/// The words a table of contents says.
#[derive(Clone, Debug)]
pub struct TableOfContentsWords {
    pub title: String,
}

impl Default for TableOfContentsWords {
    fn default() -> Self {
        Self {
            title: "On this page".into(),
        }
    }
}

/// An "on this page" list beside a long page, as sokkel-ui's `TableOfContents`: a group label over
/// the headings on a rule in the border's color, the deeper ones 12 further in, the one the reader
/// is in marked with a 2 bar in the primary color at medium weight. A press comes back as the
/// heading's id, for the caller to scroll to.
pub struct TableOfContents<'a> {
    entries: &'a [ContentsEntry],
    active: Option<&'a str>,
    words: TableOfContentsWords,
}

impl<'a> TableOfContents<'a> {
    pub fn new(entries: &'a [ContentsEntry]) -> Self {
        Self {
            entries,
            active: None,
            words: TableOfContentsWords::default(),
        }
    }

    /// The id of the heading the reader is in.
    pub fn active(mut self, active: Option<&'a str>) -> Self {
        self.active = active;
        self
    }

    pub fn words(mut self, words: TableOfContentsWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }
        let t = tokens(ui.ctx());
        let top = self.entries.iter().map(|e| e.level).min().unwrap_or(1);
        let mut jumped = None;
        let spacing = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 8.0;
        super::group_label(ui, &self.words.title);
        ui.spacing_mut().item_spacing.y = 0.0;
        let width = ui.available_width();
        let role = TextRole::Body;
        let line = (role.font(&t).size * 1.375).round();
        let first = ui.cursor().top();
        for entry in self.entries {
            let on = self.active == Some(entry.id.as_str());
            let indent = (entry.level - top) as f32 * 12.0 + 12.0;
            let mut rich = role.rich(&t, &entry.label).line_height(Some(line));
            if on {
                rich = rich.variation("wght", t.weight(crate::Weight::Medium));
            }
            let galley = WidgetText::from(rich).into_galley(
                ui,
                Some(TextWrapMode::Wrap),
                (width - indent - 8.0).max(1.0),
                role.font(&t),
            );
            let (rect, hit) = ui.allocate_exact_size(vec2(width, galley.size().y + 8.0), Sense::click());
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, true, &entry.label));
            let ink = if on || hit.hovered() {
                t.color("foreground")
            } else {
                t.color("muted-foreground")
            };
            let bar = if on {
                Some(t.color("primary"))
            } else if hit.hovered() {
                Some(t.color("border-strong"))
            } else {
                None
            };
            if let Some(bar) = bar {
                ui.painter()
                    .vline(rect.left() + 0.5, rect.y_range(), Stroke::new(2.0, bar));
            }
            ui.painter()
                .galley(pos2(rect.left() + indent, rect.top() + 4.0), galley, ink);
            if hit.clicked() {
                jumped = Some(entry.id.clone());
            }
        }
        let last = ui.cursor().top();
        let left = ui.min_rect().left();
        ui.painter()
            .vline(left + 0.5, first..=last, Stroke::new(1.0, t.color("border")));
        ui.spacing_mut().item_spacing.y = spacing;
        jumped
    }
}
