//! Text written in markdown, after sokkel-ui's `Markdown` at its `message` scale: headings,
//! paragraphs, bullet and numbered lists, and in a sentence bold, italic and code. Links, quotes,
//! tables, code blocks and math are not read yet; their text shows as written.

use egui::{Align, FontId, FontSelection, RichText, Ui, text::LayoutJob};

use crate::{TextRole, Tokens, tokens};

/// One block of a markdown text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Heading(u8, String),
    Paragraph(String),
    /// A list's items, numbered from the first's number when `Some`.
    List(Option<u32>, Vec<String>),
}

/// The blocks of a markdown text: `#` headings, lists of `-`, `*` or `1.` items, and paragraphs
/// of the lines between blank ones.
pub fn blocks(text: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let flush = |paragraph: &mut Vec<&str>, out: &mut Vec<Block>| {
        if !paragraph.is_empty() {
            out.push(Block::Paragraph(paragraph.join(" ")));
            paragraph.clear();
        }
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush(&mut paragraph, &mut out);
            continue;
        }
        let hashes = trimmed.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hashes) && trimmed[hashes..].starts_with(' ') {
            flush(&mut paragraph, &mut out);
            out.push(Block::Heading(hashes as u8, trimmed[hashes..].trim().to_owned()));
            continue;
        }
        let bullet = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .map(|rest| (None, rest));
        let numbered = trimmed.split_once(". ").and_then(|(n, rest)| {
            n.parse::<u32>()
                .ok()
                .filter(|_| n.chars().all(|c| c.is_ascii_digit()))
                .map(|n| (Some(n), rest))
        });
        if let Some((number, rest)) = bullet.or(numbered) {
            flush(&mut paragraph, &mut out);
            match out.last_mut() {
                Some(Block::List(kind, items)) if kind.is_some() == number.is_some() => items.push(rest.to_owned()),
                _ => out.push(Block::List(number, vec![rest.to_owned()])),
            }
            continue;
        }
        paragraph.push(trimmed);
    }
    flush(&mut paragraph, &mut out);
    out
}

/// A run of a sentence and how it is set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
}

/// The runs of a sentence: `**bold**`, `*italic*` or `_italic_`, and `` `code` ``; a mark left
/// open is read as written.
pub fn runs(text: &str) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    let (mut bold, mut italic) = (false, false);
    let mut current = String::new();
    let push = |current: &mut String, out: &mut Vec<Run>, bold: bool, italic: bool, code: bool| {
        if !current.is_empty() {
            out.push(Run {
                text: std::mem::take(current),
                bold,
                italic,
                code,
            });
        }
    };
    let chars: Vec<char> = text.chars().collect();
    let closes = |from: usize, mark: &str| -> bool {
        let rest: String = chars[from..].iter().collect();
        rest.contains(mark)
    };
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '`' && closes(i + 1, "`") {
            push(&mut current, &mut out, bold, italic, false);
            let end = (i + 1..chars.len()).find(|j| chars[*j] == '`').unwrap_or(chars.len());
            current = chars[i + 1..end].iter().collect();
            push(&mut current, &mut out, false, false, true);
            i = end + 1;
            continue;
        }
        if c == '*' && chars.get(i + 1) == Some(&'*') && (bold || closes(i + 2, "**")) {
            push(&mut current, &mut out, bold, italic, false);
            bold = !bold;
            i += 2;
            continue;
        }
        if c == '*' && chars.get(i + 1) == Some(&'*') {
            // a pair of stars that opens nothing is read as written
            current.push_str("**");
            i += 2;
            continue;
        }
        if (c == '*' || c == '_') && (italic || closes(i + 1, &c.to_string())) {
            // an underscore inside a word is a word's, not a mark
            let inside = c == '_'
                && i > 0
                && chars[i - 1].is_alphanumeric()
                && chars.get(i + 1).is_some_and(|n| n.is_alphanumeric());
            if !inside {
                push(&mut current, &mut out, bold, italic, false);
                italic = !italic;
                i += 1;
                continue;
            }
        }
        current.push(c);
        i += 1;
    }
    push(&mut current, &mut out, bold, italic, false);
    out
}

/// A sentence's runs as one layout, in `role`'s type at the prose's relaxed leading.
fn sentence(ui: &Ui, t: &Tokens, text: &str, role: TextRole, wght: f32, width: f32) -> LayoutJob {
    let size = role.font(t).size;
    let mut job = LayoutJob::default();
    for run in runs(text) {
        let rich = if run.code {
            RichText::new(&run.text)
                .font(FontId::monospace(TextRole::Help.font(t).size))
                .background_color(t.color("muted"))
                .color(t.color("foreground"))
        } else {
            let mut rich = role
                .rich(t, &run.text)
                .color(t.color("foreground"))
                .line_height(Some((size * 1.625).round()));
            if run.bold {
                rich = rich.variation("wght", 600.0);
            } else if wght > 0.0 {
                rich = rich.variation("wght", wght);
            }
            if run.italic {
                rich = rich.italics();
            }
            rich
        };
        rich.append_to(&mut job, ui.style(), FontSelection::Default, Align::Min);
    }
    job.wrap.max_width = width;
    job
}

/// Text written in markdown, as sokkel-ui's `Markdown` at its `message` scale: paragraphs and
/// lists 8 apart, list items 2 apart and 20 in, headings from a detail title down.
pub struct Markdown<'a> {
    text: &'a str,
}

impl<'a> Markdown<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text }
    }

    pub fn show(self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        let spacing = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 0.0;
        for (index, block) in blocks(self.text).into_iter().enumerate() {
            let first = index == 0;
            match block {
                Block::Heading(level, text) => {
                    let (role, above, below, wght) = match level {
                        1 => (TextRole::DetailTitle, 20.0, 8.0, 0.0),
                        2 => (TextRole::SectionTitle, 16.0, 8.0, 600.0),
                        3 => (TextRole::SectionTitle, 12.0, 6.0, 0.0),
                        _ => (TextRole::Head, 12.0, 4.0, 0.0),
                    };
                    if !first {
                        ui.add_space(above);
                    }
                    ui.add(egui::Label::new(sentence(ui, &t, &text, role, wght, width)).wrap());
                    ui.add_space(below);
                }
                Block::Paragraph(text) => {
                    if !first {
                        ui.add_space(8.0);
                    }
                    ui.add(egui::Label::new(sentence(ui, &t, &text, TextRole::Body, 0.0, width)).wrap());
                }
                Block::List(start, items) => {
                    if !first {
                        ui.add_space(8.0);
                    }
                    for (i, item) in items.iter().enumerate() {
                        if i > 0 {
                            ui.add_space(4.0);
                        }
                        let marker = match start {
                            Some(n) => format!("{}.", n + i as u32),
                            None => "\u{2022}".to_owned(),
                        };
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            let (spot, _) = ui.allocate_exact_size(egui::vec2(20.0, 1.0), egui::Sense::hover());
                            // the marker on the item's first line, in the text's own ink
                            let line = ui.fonts_mut(|f| f.row_height(&TextRole::Body.font(&t)));
                            ui.painter().text(
                                egui::pos2(spot.right() - 6.0, spot.top() + line / 2.0),
                                egui::Align2::RIGHT_CENTER,
                                marker,
                                TextRole::Body.font(&t),
                                t.color("foreground"),
                            );
                            ui.add(egui::Label::new(sentence(ui, &t, item, TextRole::Body, 0.0, width - 20.0)).wrap());
                        });
                    }
                }
            }
        }
        ui.spacing_mut().item_spacing.y = spacing;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_are_read() {
        let read =
            blocks("### Hand-over\n\nThe person on call\nhands over.\n\n1. Read the log\n2. Retry\n- one\n- two");
        assert_eq!(
            read,
            [
                Block::Heading(3, "Hand-over".into()),
                Block::Paragraph("The person on call hands over.".into()),
                Block::List(Some(1), vec!["Read the log".into(), "Retry".into()]),
                Block::List(None, vec!["one".into(), "two".into()]),
            ]
        );
    }

    #[test]
    fn runs_are_read() {
        let read = runs("Press **Edit** to move `nightly-export`, *now*; a sync_inventory stays, ** stays");
        let texts: Vec<(&str, bool, bool, bool)> = read
            .iter()
            .map(|r| (r.text.as_str(), r.bold, r.italic, r.code))
            .collect();
        assert_eq!(
            texts,
            [
                ("Press ", false, false, false),
                ("Edit", true, false, false),
                (" to move ", false, false, false),
                ("nightly-export", false, false, true),
                (", ", false, false, false),
                ("now", false, true, false),
                ("; a sync_inventory stays, ** stays", false, false, false),
            ]
        );
    }
}
