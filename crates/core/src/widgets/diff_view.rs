//! Two versions of a text as one list of lines, after sokkel-ui's `DiffView`: the line diff
//! (`diffLines`), the rows it draws with the unchanged lines far from a change folded (`foldDiff`),
//! and the view, a code block of those rows with the removed lines marked `−` on a critical wash and
//! the added `+` on a good one, the counts at its top end.

use egui::{Align2, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use serde::Serialize;

use crate::{Icon, TextRole, icons, tokens};

/// What a line of a diff is: kept, added in the new text, or removed from the old.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffKind {
    Same,
    Added,
    Removed,
}

/// One line of a line diff.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub text: String,
}

fn lines_of(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    text.strip_suffix('\n').unwrap_or(text).split('\n').collect()
}

/// The lines that turn `before` into `after`, by the longest run of lines the two share.
pub fn diff_lines(before: &str, after: &str) -> Vec<DiffLine> {
    let a = lines_of(before);
    let b = lines_of(after);
    let cols = b.len() + 1;
    let mut shared = vec![0u32; (a.len() + 1) * cols];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            shared[i * cols + j] = if a[i] == b[j] {
                shared[(i + 1) * cols + j + 1] + 1
            } else {
                shared[(i + 1) * cols + j].max(shared[i * cols + j + 1])
            };
        }
    }
    let line = |kind, text: &str| DiffLine {
        kind,
        text: text.to_owned(),
    };
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            out.push(line(DiffKind::Same, a[i]));
            i += 1;
            j += 1;
        } else if shared[(i + 1) * cols + j] >= shared[i * cols + j + 1] {
            out.push(line(DiffKind::Removed, a[i]));
            i += 1;
        } else {
            out.push(line(DiffKind::Added, b[j]));
            j += 1;
        }
    }
    out.extend(a[i..].iter().map(|text| line(DiffKind::Removed, text)));
    out.extend(b[j..].iter().map(|text| line(DiffKind::Added, text)));
    out
}

/// A row of a folded diff: a line drawn, with its place among the lines, or a run of unchanged
/// lines folded into one row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum DiffRow {
    Line { line: DiffLine, index: usize },
    Fold { start: usize, count: usize },
}

/// The rows of a diff with the unchanged lines more than `context` lines from any change folded, a
/// run of them into one row; a run in `opened`, by its start, is drawn whole, and so is a run of
/// one line, since its row would take the same room.
pub fn fold_diff(lines: &[DiffLine], context: usize, opened: &[usize]) -> Vec<DiffRow> {
    let near_change = |at: usize| {
        let from = at.saturating_sub(context);
        let to = at.saturating_add(context).min(lines.len().saturating_sub(1));
        (from..=to).any(|k| lines.get(k).is_some_and(|one| one.kind != DiffKind::Same))
    };
    let near: Vec<bool> = (0..lines.len()).map(near_change).collect();
    let mut rows = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if near[index] {
            rows.push(DiffRow::Line {
                line: lines[index].clone(),
                index,
            });
            index += 1;
            continue;
        }
        let mut end = index;
        while end < lines.len() && !near[end] {
            end += 1;
        }
        let count = end - index;
        if count > 1 && !opened.contains(&index) {
            rows.push(DiffRow::Fold { start: index, count });
        } else {
            for (at, line) in lines.iter().enumerate().take(end).skip(index) {
                rows.push(DiffRow::Line {
                    line: line.clone(),
                    index: at,
                });
            }
        }
        index = end;
    }
    rows
}

/// How many lines a diff adds and removes.
pub fn diff_counts(lines: &[DiffLine]) -> (usize, usize) {
    (
        lines.iter().filter(|one| one.kind == DiffKind::Added).count(),
        lines.iter().filter(|one| one.kind == DiffKind::Removed).count(),
    )
}

/// The words a diff view says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffViewWords {
    /// Said in place of the counts when the two texts are the same.
    pub unchanged: String,
    /// The row standing for one folded unchanged line.
    pub folded_one: String,
    /// The row standing for a folded run of unchanged lines, `{count}` of them.
    pub folded_other: String,
}

impl Default for DiffViewWords {
    fn default() -> Self {
        Self {
            unchanged: "No changes".into(),
            folded_one: "1 unchanged line".into(),
            folded_other: "{count} unchanged lines".into(),
        }
    }
}

/// The diff view; see the module's documentation. Without a title its counts float at the block's
/// top end; a folded row opens its lines on a press.
#[must_use = "show it with `.show(ui)`"]
pub struct DiffView<'a> {
    id: egui::Id,
    before: &'a str,
    after: &'a str,
    title: Option<String>,
    small: bool,
    context: usize,
    words: DiffViewWords,
}

impl<'a> DiffView<'a> {
    pub fn new(id: impl egui::AsIdSalt, before: &'a str, after: &'a str) -> Self {
        Self {
            id: egui::Id::unique(id),
            before,
            after,
            title: None,
            small: false,
            context: 3,
            words: DiffViewWords::default(),
        }
    }

    /// A head row with the title, in the code face, and the counts beside it.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// The small size, as the web's `size="sm"`: the text at `xs`, the block 10 in.
    pub fn small(mut self, small: bool) -> Self {
        self.small = small;
        self
    }

    /// How many unchanged lines stay drawn on each side of a change; `usize::MAX` draws every line.
    pub fn context(mut self, context: usize) -> Self {
        self.context = context;
        self
    }

    pub fn words(mut self, words: DiffViewWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let lines = diff_lines(self.before, self.after);
        // the runs opened, kept with the lines they were opened in
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            lines.hash(&mut h);
            h.finish()
        };
        let open_id = self.id.with("opened");
        let mut opened: Vec<usize> = ui
            .data(|d| d.get_temp::<(u64, Vec<usize>)>(open_id))
            .filter(|(of, _)| *of == key)
            .map(|(_, starts)| starts)
            .unwrap_or_default();
        let rows = fold_diff(&lines, self.context, &opened);
        let (added, removed) = diff_counts(&lines);

        let spec = if self.small { t.text("xs") } else { t.text("sm") };
        let size = spec.size;
        // leading-relaxed
        let line_height = (size * 1.625).round();
        let pad = if self.small { 10.0 } else { 12.0 };
        let mono = egui::FontId::monospace(crate::mono_size(&t, size));
        let sans = egui::FontId::proportional(size);
        let head = if self.title.is_some() { line_height + pad } else { 0.0 };
        let width = ui.available_width();
        let height = head + pad * 2.0 + line_height * rows.len().max(1) as f32;
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let painter = ui.painter_at(rect);
        let radius = t.radius("surface");
        painter.rect_filled(rect, radius, t.color("editor-ground"));

        // the rows, each the block's width, the text 28 in and the mark 12 in
        let mut y = rect.top() + head + pad;
        for row in &rows {
            let band = Rect::from_min_size(pos2(rect.left(), y), vec2(rect.width(), line_height));
            match row {
                DiffRow::Line { line, .. } => {
                    let (wash, mark) = match line.kind {
                        DiffKind::Same => (None, None),
                        DiffKind::Added => (Some(t.color_at("good", 0.12)), Some(("+", t.color("good")))),
                        DiffKind::Removed => (
                            Some(t.color_at("critical", 0.12)),
                            Some(("\u{2212}", t.color("destructive"))),
                        ),
                    };
                    if let Some(wash) = wash {
                        painter.rect_filled(band, CornerRadius::ZERO, wash);
                    }
                    if let Some((sign, color)) = mark {
                        painter.text(
                            pos2(band.left() + 12.0, band.center().y),
                            Align2::LEFT_CENTER,
                            sign,
                            mono.clone(),
                            color,
                        );
                    }
                    painter.text(
                        pos2(band.left() + 28.0, band.center().y),
                        Align2::LEFT_CENTER,
                        &line.text,
                        mono.clone(),
                        t.color("foreground"),
                    );
                }
                DiffRow::Fold { start, count } => {
                    let hit = ui.interact(band, self.id.with(("fold", *start)), Sense::click());
                    let said = if *count == 1 {
                        self.words.folded_one.clone()
                    } else {
                        self.words.folded_other.replace("{count}", &count.to_string())
                    };
                    hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &said));
                    let (ground, ink) = if hit.hovered() {
                        (t.color("muted"), t.color("foreground"))
                    } else {
                        (t.color_at("muted", 0.6), t.color("muted-foreground"))
                    };
                    painter.rect_filled(band, CornerRadius::ZERO, ground);
                    icons::paint(
                        &painter,
                        pos2(band.left() + 8.0 + 7.0, band.center().y),
                        Icon::UnfoldVertical,
                        14.0,
                        ink,
                    );
                    painter.text(
                        pos2(band.left() + 28.0, band.center().y),
                        Align2::LEFT_CENTER,
                        said,
                        sans.clone(),
                        ink,
                    );
                    if hit.clicked() {
                        opened.push(*start);
                        ui.data_mut(|d| d.insert_temp(open_id, (key, opened.clone())));
                    }
                }
            }
            y += line_height;
        }

        // the counts: in the head beside the title, or floating at the block's top end
        let counts: Vec<(String, egui::Color32, egui::FontId)> = if added == 0 && removed == 0 {
            vec![(self.words.unchanged.clone(), t.color("muted-foreground"), sans.clone())]
        } else {
            let mut out = Vec::new();
            if added > 0 {
                out.push((format!("+{added}"), t.color("good"), mono.clone()));
            }
            if removed > 0 {
                out.push((format!("-{removed}"), t.color("critical"), mono.clone()));
            }
            out
        };
        let galleys: Vec<_> = counts
            .into_iter()
            .map(|(text, color, font)| (ui.painter().layout_no_wrap(text, font, color), color))
            .collect();
        let gap = 8.0;
        let words_width: f32 =
            galleys.iter().map(|(g, _)| g.size().x).sum::<f32>() + gap * galleys.len().saturating_sub(1) as f32;
        let edge = if self.small { 6.0 } else { 8.0 };
        let (mut x, middle) = match &self.title {
            Some(title) => {
                let middle = rect.top() + pad + line_height / 2.0;
                let font = TextRole::Label.font(&t);
                painter.text(
                    pos2(rect.left() + pad, middle),
                    Align2::LEFT_CENTER,
                    title,
                    egui::FontId::monospace(crate::mono_size(&t, font.size)),
                    t.color("foreground"),
                );
                (rect.right() - pad - words_width, middle)
            }
            None => {
                let pill = Rect::from_min_size(
                    pos2(rect.right() - edge - words_width - 10.0, rect.top() + edge),
                    vec2(words_width + 10.0, line_height + 2.0),
                );
                painter.rect(
                    pill,
                    t.radius("control"),
                    t.color_at("background", 0.9),
                    Stroke::new(1.0, t.color_at("foreground", 0.1)),
                    StrokeKind::Inside,
                );
                (pill.left() + 8.0, pill.center().y)
            }
        };
        for (galley, color) in galleys {
            let w = galley.size().x;
            painter.galley(pos2(x, middle - galley.size().y / 2.0), galley, color);
            x += w + gap;
        }
        painter.rect_stroke(rect, radius, Stroke::new(1.0, t.color("input")), StrokeKind::Inside);
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_diff_folds_the_far_unchanged_lines() {
        let before = (1..=20).map(|n| format!("line {n}")).collect::<Vec<_>>().join("\n");
        let after = before.replace("line 10", "line ten");
        let lines = diff_lines(&before, &after);
        assert_eq!(diff_counts(&lines), (1, 1));
        let rows = fold_diff(&lines, 3, &[]);
        assert_eq!(rows.first(), Some(&DiffRow::Fold { start: 0, count: 6 }));
        assert_eq!(rows.last(), Some(&DiffRow::Fold { start: 14, count: 7 }));
        // a run opened is drawn whole
        let opened = fold_diff(&lines, 3, &[0]);
        assert!(matches!(opened[0], DiffRow::Line { index: 0, .. }));
    }
}
