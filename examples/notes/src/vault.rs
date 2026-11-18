//! What the vault's notes say about each other, as the web's `vault.ts`: their links and
//! backlinks, their tags, where they sit and in what order, and each note's outline, as sokkel-ui's
//! `markdownHeadings` reads it.

use std::collections::HashMap;
use std::ops::Range;

use sokkel_egui::{Icon, TreeNode};

use crate::model::{
    Folder, Note, Order, is_letter, is_number, is_space, locale_compare, lowercase, trim, trim_end, trim_start,
};
use crate::wiki_links::{WikiLink, mentions_of, tags_in, wiki_links_in};

/// A link from one note to another, with the line it stands in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Backlink {
    /// The id of the note linking.
    pub from: String,
    pub link: WikiLink,
    pub line: String,
}

/// A place a note's name is written in another note without a link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mention {
    /// The id of the note naming it.
    pub from: String,
    pub at: Range<usize>,
    pub line: String,
}

/// The vault read once: its notes and folders, and what they say about each other.
#[derive(Clone, Debug, Default)]
pub struct Vault {
    pub notes: Vec<Note>,
    pub folders: Vec<Folder>,
    by_id: HashMap<String, usize>,
    by_title: HashMap<String, usize>,
    folder_by_id: HashMap<String, usize>,
    links: HashMap<String, Vec<WikiLink>>,
    backlinks: HashMap<String, Vec<Backlink>>,
    tags_of: HashMap<String, Vec<String>>,
    /// Every tag in the order it is first met, with the ids of the notes under it.
    tagged: Vec<(String, Vec<String>)>,
    tag_at: HashMap<String, usize>,
}

impl Vault {
    pub fn note(&self, id: &str) -> Option<&Note> {
        self.by_id.get(id).map(|at| &self.notes[*at])
    }

    pub fn folder(&self, id: &str) -> Option<&Folder> {
        self.folder_by_id.get(id).map(|at| &self.folders[*at])
    }

    /// The note a link's name names, in any case.
    pub fn resolve(&self, target: &str) -> Option<&Note> {
        self.by_title.get(&lowercase(trim(target))).map(|at| &self.notes[*at])
    }

    /// A note's links out.
    pub fn links(&self, id: &str) -> &[WikiLink] {
        self.links.get(id).map_or(&[], Vec::as_slice)
    }

    /// The links into a note, one per note linking, at its first link.
    pub fn backlinks(&self, id: &str) -> &[Backlink] {
        self.backlinks.get(id).map_or(&[], Vec::as_slice)
    }

    /// A note's tags.
    pub fn tags_of(&self, id: &str) -> &[String] {
        self.tags_of.get(id).map_or(&[], Vec::as_slice)
    }

    /// The ids of the notes under a tag.
    pub fn tagged(&self, tag: &str) -> &[String] {
        self.tag_at.get(tag).map_or(&[], |at| self.tagged[*at].1.as_slice())
    }

    /// Every tag in the order it is first met, with the ids of the notes under it.
    pub fn tags(&self) -> &[(String, Vec<String>)] {
        &self.tagged
    }

    /// A folder's path from the top: Projects / Services; empty at the top of the vault.
    pub fn path(&self, folder: Option<&str>) -> String {
        let mut names: Vec<&str> = Vec::new();
        let mut at = folder;
        while let Some(id) = at {
            let Some(found) = self.folder(id) else { break };
            if names.len() > 32 {
                break;
            }
            names.insert(0, &found.name);
            at = found.parent.as_deref();
        }
        names.join(" / ")
    }
}

/// The line a range of a text stands in, cut to about `room` characters around the range.
pub fn line_around(text: &str, from: usize, to: usize, room: usize) -> String {
    // As `lastIndexOf('\n', from - 1) + 1` and `indexOf('\n', to)`.
    let upto = from.max(1).min(text.len());
    let start = text
        .get(..upto)
        .and_then(|before| before.rfind('\n'))
        .map_or(0, |at| at + 1);
    let end = text
        .get(to..)
        .and_then(|after| after.find('\n'))
        .map_or(text.len(), |at| at + to);
    let line = text.get(start..end).unwrap_or("");
    let chars: Vec<char> = line.chars().collect();
    if chars.len() <= room {
        return trim(line).to_owned();
    }
    let at = text.get(start..from).map_or(0, |before| before.chars().count());
    let left = at.saturating_sub(room / 3);
    let cut: String = chars[left.min(chars.len())..(left + room).min(chars.len())]
        .iter()
        .collect();
    format!(
        "{}{}{}",
        if left > 0 { "\u{2026}" } else { "" },
        trim(&cut),
        if left + room < chars.len() { "\u{2026}" } else { "" }
    )
}

/// The vault read: each note's links and tags, the links into each, and the notes by id and name.
pub fn index_vault(notes: Vec<Note>, folders: Vec<Folder>) -> Vault {
    let mut vault = Vault {
        by_id: notes
            .iter()
            .enumerate()
            .map(|(at, note)| (note.id.clone(), at))
            .collect(),
        by_title: notes
            .iter()
            .enumerate()
            .map(|(at, note)| (lowercase(&note.title), at))
            .collect(),
        folder_by_id: folders
            .iter()
            .enumerate()
            .map(|(at, folder)| (folder.id.clone(), at))
            .collect(),
        ..Vault::default()
    };
    for note in &notes {
        let out = wiki_links_in(&note.body);
        for link in &out {
            let Some(to) = vault
                .by_title
                .get(&lowercase(trim(&link.target)))
                .map(|at| &notes[*at].id)
            else {
                continue;
            };
            if *to == note.id {
                continue;
            }
            let list = vault.backlinks.entry(to.clone()).or_default();
            // One entry per note linking, at its first link.
            if !list.iter().any(|one| one.from == note.id) {
                list.push(Backlink {
                    from: note.id.clone(),
                    link: link.clone(),
                    line: line_around(&note.body, link.from, link.to, 90),
                });
            }
        }
        vault.links.insert(note.id.clone(), out);
        let tags = tags_in(&note.body);
        for tag in &tags {
            match vault.tag_at.get(tag) {
                Some(at) => vault.tagged[*at].1.push(note.id.clone()),
                None => {
                    vault.tag_at.insert(tag.clone(), vault.tagged.len());
                    vault.tagged.push((tag.clone(), vec![note.id.clone()]));
                }
            }
        }
        vault.tags_of.insert(note.id.clone(), tags);
    }
    vault.notes = notes;
    vault.folders = folders;
    vault
}

/// The places other notes write a note's name without linking it, one per note at its first.
pub fn unlinked_mentions(vault: &Vault, note: &Note) -> Vec<Mention> {
    vault
        .notes
        .iter()
        .filter(|other| other.id != note.id)
        .filter_map(|other| {
            let first = mentions_of(&other.body, &note.title).into_iter().next()?;
            Some(Mention {
                from: other.id.clone(),
                line: line_around(&other.body, first.start, first.end, 90),
                at: first,
            })
        })
        .collect()
}

/// Two names in reading order, numbers by their value.
fn by_name(a: &str, b: &str) -> std::cmp::Ordering {
    locale_compare(a, b, true)
}

/// Two places in the reader's own order; one without a place goes after those with one.
fn by_rank(a: Option<i64>, b: Option<i64>) -> std::cmp::Ordering {
    const LAST: i64 = 9_007_199_254_740_991;
    a.unwrap_or(LAST).cmp(&b.unwrap_or(LAST))
}

/// The folders directly in a folder (none for the top of the vault), in the vault's order.
pub fn folders_in<'a>(vault: &'a Vault, parent: Option<&str>, order: Order) -> Vec<&'a Folder> {
    let mut level: Vec<&Folder> = vault
        .folders
        .iter()
        .filter(|folder| folder.parent.as_deref() == parent)
        .collect();
    level.sort_by(|a, b| {
        let ranked = if order == Order::Manual {
            by_rank(a.rank, b.rank)
        } else {
            std::cmp::Ordering::Equal
        };
        ranked.then_with(|| by_name(&a.name, &b.name))
    });
    level
}

/// The notes directly in a folder, in the vault's order: by name with daily notes newest first, or
/// the reader's own.
pub fn notes_in<'a>(vault: &'a Vault, folder: Option<&str>, order: Order, daily: Option<&str>) -> Vec<&'a Note> {
    let newest_first = folder.is_some() && folder == daily;
    let mut level: Vec<&Note> = vault
        .notes
        .iter()
        .filter(|note| note.folder.as_deref() == folder)
        .collect();
    level.sort_by(|a, b| {
        let ranked = if order == Order::Manual {
            by_rank(a.rank, b.rank)
        } else {
            std::cmp::Ordering::Equal
        };
        ranked.then_with(|| {
            if newest_first {
                by_name(&b.title, &a.title)
            } else {
                by_name(&a.title, &b.title)
            }
        })
    });
    level
}

/// Every note in a folder and the folders under it (the whole vault for none), in the tree's
/// order: a folder's notes after its folders, and at the top the notes outside every folder last.
pub fn notes_in_order<'a>(vault: &'a Vault, top: Option<&str>, order: Order, daily: Option<&str>) -> Vec<&'a Note> {
    let mut out = Vec::new();
    for folder in folders_in(vault, top, order) {
        out.extend(notes_in_order(vault, Some(&folder.id), order, daily));
    }
    out.extend(notes_in(vault, top, order, daily));
    out
}

/// The vault as a tree: its folders with their folders and notes, then the notes at the top. A
/// folder's node is `folder:<id>`, a note's its id.
pub fn vault_tree(vault: &Vault, daily: Option<&str>, order: Order) -> Vec<TreeNode> {
    fn level(vault: &Vault, parent: Option<&str>, daily: Option<&str>, order: Order) -> Vec<TreeNode> {
        let folders = folders_in(vault, parent, order).into_iter().map(|folder| {
            TreeNode::new(format!("folder:{}", folder.id), folder.name.clone())
                .icon(Icon::Folder)
                .open_icon(Icon::FolderOpen)
                .children(level(vault, Some(&folder.id), daily, order))
        });
        let notes = notes_in(vault, parent, order, daily).into_iter().map(|note| {
            TreeNode::new(note.id.clone(), note.title.clone()).icon(if note.day.is_none() {
                Icon::FileText
            } else {
                Icon::CalendarDays
            })
        });
        folders.chain(notes).collect()
    }
    level(vault, None, daily, order)
}

/// The id the top of the vault stands under where a folder is picked.
pub const ROOT: &str = "~vault";

/// The folders alone as a tree, under the top of the vault (`ROOT`, called `top`), for picking where
/// a note or a folder goes.
pub fn folder_tree(vault: &Vault, top: &str, order: Order) -> Vec<TreeNode> {
    fn level(vault: &Vault, parent: Option<&str>, order: Order) -> Vec<TreeNode> {
        folders_in(vault, parent, order)
            .into_iter()
            .map(|folder| {
                TreeNode::new(folder.id.clone(), folder.name.clone())
                    .icon(Icon::Folder)
                    .open_icon(Icon::FolderOpen)
                    .children(level(vault, Some(&folder.id), order))
            })
            .collect()
    }
    vec![
        TreeNode::new(ROOT, top)
            .icon(Icon::Folder)
            .open_icon(Icon::FolderOpen)
            .children(level(vault, None, order)),
    ]
}

/// A folder and every folder under it, the folder first.
pub fn folder_and_below(vault: &Vault, id: &str) -> Vec<String> {
    let mut out = vec![id.to_owned()];
    let mut grew = true;
    while grew {
        grew = false;
        for folder in &vault.folders {
            if let Some(parent) = &folder.parent
                && out.contains(parent)
                && !out.contains(&folder.id)
            {
                out.push(folder.id.clone());
                grew = true;
            }
        }
    }
    out
}

/// A text with every link to `from` renamed to `to`, keeping a heading and the words shown.
pub fn rename_links(text: &str, from: &str, to: &str) -> String {
    let lower = lowercase(from);
    let mut out = String::new();
    let mut at = 0;
    for link in wiki_links_in(text) {
        if lowercase(&link.target) != lower {
            continue;
        }
        out.push_str(&text[at..link.from]);
        out.push_str("[[");
        out.push_str(to);
        if let Some(heading) = &link.heading {
            out.push('#');
            out.push_str(heading);
        }
        if let Some(label) = &link.label {
            out.push('|');
            out.push_str(label);
        }
        out.push_str("]]");
        at = link.to;
    }
    out.push_str(&text[at..]);
    out
}

/// One heading of a markdown text: its level, its text, and the id `anchors` gives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownHeading {
    pub id: String,
    pub label: String,
    pub level: usize,
}

/// The id a heading is given under `anchors`: its text lowercased, spaces as dashes, punctuation
/// dropped.
pub fn heading_id(text: &str) -> String {
    let lower = lowercase(text);
    let kept: String = trim(&lower)
        .chars()
        .filter(|c| is_letter(*c) || is_number(*c) || is_space(*c) || *c == '-')
        .collect();
    let mut out = String::new();
    let mut spaced = false;
    for c in kept.chars() {
        if is_space(c) {
            if !spaced {
                out.push('-');
            }
            spaced = true;
        } else {
            out.push(c);
            spaced = false;
        }
    }
    out
}

/// The text with each `[words](address)` link written as its words alone.
fn link_words(text: &str) -> String {
    let find = |c: char, from: usize| -> Option<usize> { text.get(from..)?.find(c).map(|at| at + from) };
    let mut out = String::new();
    let mut at = 0;
    let mut close: Option<usize> = None;
    let mut end: Option<usize> = None;
    let mut open = find('[', 0);
    while let Some(start) = open {
        open = find('[', start + 1);
        if start < at {
            continue;
        }
        if close.is_none_or(|close| close <= start) {
            close = find(']', start + 1);
        }
        let Some(shut) = close else { break };
        if text.as_bytes().get(shut + 1) != Some(&b'(') {
            continue;
        }
        if end.is_none_or(|end| end <= shut) {
            end = find(')', shut + 2);
        }
        let Some(done) = end else { break };
        out.push_str(&text[at..start]);
        out.push_str(&text[start + 1..shut]);
        at = done + 1;
    }
    out.push_str(&text[at..]);
    out
}

/// A setext heading's underline: `===` under a level 1 heading, `---` under a level 2 one; the mark
/// it is drawn with.
fn setext(line: &str) -> Option<char> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let mark = rest.chars().next().filter(|c| *c == '=' || *c == '-')?;
    trim_start(rest.trim_start_matches(mark)).is_empty().then_some(mark)
}

/// A line written as a `#` heading: its level and its text without the closing `#`s, or none.
fn atx_heading(line: &str) -> Option<(usize, String)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let after = &line[indent..];
    let level = after.len() - after.trim_start_matches('#').len();
    if !(1..=6).contains(&level) {
        return None;
    }
    let space = after[level..].chars().next().filter(|c| is_space(*c))?;
    let rest = &after[level + space.len_utf8()..];
    let body = trim_start(rest);
    let text = if body.is_empty() {
        rest.chars().next_back().map_or(String::new(), String::from)
    } else {
        body.to_owned()
    };
    let mut end = trim_end(&text).len();
    while end > 0 && text.as_bytes()[end - 1] == b'#' {
        end -= 1;
    }
    let end = trim_end(&text[..end]).len();
    let first = text.chars().next().map_or(0, char::len_utf8);
    let text = text[..end.max(first)].to_owned();
    if text.is_empty() || text.contains(['\n', '\r', '\u{2028}', '\u{2029}']) {
        return None;
    }
    Some((level, text))
}

/// Whether a line starts as a list item, a quote or a table row.
fn starts_block(line: &str) -> bool {
    let rest = trim_start(line);
    let after = match rest.chars().next() {
        Some('-' | '*' | '+' | '>' | '|') => &rest[1..],
        Some(c) if c.is_ascii_digit() => {
            let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            match rest[digits..].chars().next() {
                Some('.' | ')') => &rest[digits + 1..],
                _ => return false,
            }
        }
        _ => return false,
    };
    after.chars().next().is_none_or(is_space)
}

/// Whether a line opens or closes a fence: ```` ``` ```` or `~~~` after spaces.
fn fences(line: &str) -> bool {
    let rest = trim_start(line);
    rest.starts_with("```") || rest.starts_with("~~~")
}

/// The headings of a markdown text at the given levels, outside fenced code, in order: those
/// written with `#` and those underlined with `===` or `---`. Their ids are those `anchors` gives,
/// a repeated heading numbered after the first.
pub fn markdown_headings(content: &str, levels: &[usize]) -> Vec<MarkdownHeading> {
    let mut out = Vec::new();
    let mut taken: HashMap<String, usize> = HashMap::new();
    let mut unique = |id: String| -> String {
        let mut out = id.clone();
        while taken.contains_key(&out) {
            let count = taken.get(&id).copied().unwrap_or(0) + 1;
            taken.insert(id.clone(), count);
            out = format!("{id}-{count}");
        }
        taken.insert(out.clone(), 0);
        out
    };
    let mut fenced = false;
    let lines: Vec<&str> = content.split('\n').collect();
    for (index, line) in lines.iter().enumerate() {
        if fences(line) {
            fenced = !fenced;
        }
        if fenced {
            continue;
        }
        let atx = atx_heading(line);
        let under = setext(lines.get(index + 1).copied().unwrap_or(""));
        let before = if index == 0 { "" } else { lines[index - 1] };
        // A setext heading is one line of text after a blank one, never a list item, a quote or a table row.
        let is_setext = atx.is_none()
            && under.is_some()
            && !trim(line).is_empty()
            && trim(before).is_empty()
            && !starts_block(line);
        if atx.is_none() && !is_setext {
            continue;
        }
        let (level, text) = match atx {
            Some((level, text)) => (level, text),
            None => (if under == Some('=') { 1 } else { 2 }, (*line).to_owned()),
        };
        let words: String = link_words(&text)
            .chars()
            .filter(|c| !matches!(c, '`' | '*' | '_' | '~'))
            .collect();
        let label = trim(&words).to_owned();
        let id = unique(heading_id(&label));
        if levels.contains(&level) {
            out.push(MarkdownHeading { id, label, level });
        }
    }
    out
}

/// A note's outline, as the panel beside it lists it: its headings at levels 1 to 4.
pub fn outline(body: &str) -> Vec<MarkdownHeading> {
    markdown_headings(body, &[1, 2, 3, 4])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heading(id: &str, label: &str, level: usize) -> MarkdownHeading {
        MarkdownHeading {
            id: id.to_owned(),
            label: label.to_owned(),
            level,
        }
    }

    #[test]
    fn headings_read_as_the_web_reads_them() {
        let text = "# Title\n\n## Steps\n\n### Sub **bold** `code`\n\nText\n===\n\nOther\n---\n\n- item\n---\n\n## Steps\n## Steps\n#### Deep [link](http://x) ##\n```\n# not\n```\n#nospace\n#    \n# #\n## trailing ###   ";
        assert_eq!(
            outline(text),
            [
                heading("title", "Title", 1),
                heading("steps", "Steps", 2),
                heading("sub-bold-code", "Sub bold code", 3),
                heading("text", "Text", 1),
                heading("other", "Other", 2),
                heading("steps-1", "Steps", 2),
                heading("steps-2", "Steps", 2),
                heading("deep-link", "Deep link", 4),
                heading("", "", 1),
                heading("-1", "#", 1),
                heading("trailing", "trailing", 2),
            ]
        );
        let ids: Vec<String> = markdown_headings(text, &[2, 3]).into_iter().map(|one| one.id).collect();
        assert_eq!(
            ids,
            ["steps", "sub-bold-code", "other", "steps-1", "steps-2", "trailing"]
        );
    }

    #[test]
    fn links_rename_and_lines_cut_as_the_web_does() {
        assert_eq!(
            rename_links("[[Old]] [[old#H|w]] [[Older]] [[ old ]]", "Old", "New"),
            "[[New]] [[New#H|w]] [[Older]] [[New]]"
        );
        assert_eq!(line_around("short line\nnext", 2, 4, 90), "short line");
        let long = format!("{} word {}", "x".repeat(200), "y".repeat(200));
        assert_eq!(
            line_around(&long, 201, 205, 90),
            format!("\u{2026}{} word {}\u{2026}", "x".repeat(29), "y".repeat(55))
        );
        let edge = format!("{}{}", "a".repeat(50), "b".repeat(80));
        assert_eq!(
            line_around(&edge, 10, 11, 90),
            format!("{}{}\u{2026}", "a".repeat(50), "b".repeat(40))
        );
    }
}
