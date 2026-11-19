//! A line diff of two texts, as sokkel-ui's `DiffView` makes one (`diffLines` and `foldDiff`): the
//! lines that turn one into the other by the longest run of lines they share, and the rows it draws
//! with the unchanged lines far from any change folded. The history and CQL versions compare a
//! version with the editor's text with it. The library's diff view has no line diff of its own yet,
//! so it lives here for now.

use serde::Serialize;

/// What a line of a diff is: kept, added in the new text, or removed from the old.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffKind {
    Same,
    Added,
    Removed,
}

/// One line of a line diff.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
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
/// one line.
pub fn fold_diff(lines: &[DiffLine], context: usize, opened: &[usize]) -> Vec<DiffRow> {
    let near_change = |at: usize| {
        let from = at.saturating_sub(context);
        let to = (at + context).min(lines.len().saturating_sub(1));
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
