//! Nodes in levels that open and shut, after sokkel-ui's `Tree`: one chosen at a time, a search
//! keeping the rows that match and those above them, the arrow keys walking the rows.

use std::collections::HashSet;

use egui::{Id, Key, Modifiers, Sense, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{Icon, TextRole, icons, tokens};

/// One node of a tree, as sokkel-ui's `TreeNode`: what it answers with, what it is called, and
/// what is under it.
#[derive(Clone, Debug, Default)]
pub struct TreeNode {
    pub id: String,
    pub label: String,
    /// A short fact at the row's right end: a count, a size.
    pub meta: Option<String>,
    pub children: Vec<TreeNode>,
    /// An icon before the label, such as a folder or a file.
    pub icon: Option<Icon>,
    /// The icon an open branch shows in place of `icon`, such as an open folder.
    pub open_icon: Option<Icon>,
}

impl TreeNode {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            ..Default::default()
        }
    }

    pub fn meta(mut self, meta: impl Into<String>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn open_icon(mut self, icon: Icon) -> Self {
        self.open_icon = Some(icon);
        self
    }

    pub fn children(mut self, children: Vec<TreeNode>) -> Self {
        self.children = children;
        self
    }

    /// Whether it opens: it has children.
    pub fn is_branch(&self) -> bool {
        !self.children.is_empty()
    }
}

/// A visible row: the node, how deep it is (the top level is 1), and its parent.
#[derive(Clone, Debug)]
pub struct TreeRow<'a> {
    pub node: &'a TreeNode,
    pub level: usize,
    pub parent: Option<&'a str>,
}

/// The rows shown, as sokkel-ui's `visibleRows`: every top-level node and the children of each
/// open branch; with `matches`, only the nodes it holds, every branch among them open.
pub fn visible_rows<'a>(
    nodes: &'a [TreeNode],
    open: &HashSet<String>,
    matches: Option<&HashSet<String>>,
) -> Vec<TreeRow<'a>> {
    fn walk<'a>(
        list: &'a [TreeNode],
        level: usize,
        parent: Option<&'a str>,
        open: &HashSet<String>,
        matches: Option<&HashSet<String>>,
        rows: &mut Vec<TreeRow<'a>>,
    ) {
        for node in list {
            if matches.is_some_and(|m| !m.contains(&node.id)) {
                continue;
            }
            rows.push(TreeRow { node, level, parent });
            if node.is_branch() && (matches.is_some() || open.contains(&node.id)) {
                walk(&node.children, level + 1, Some(&node.id), open, matches, rows);
            }
        }
    }
    let mut rows = Vec::new();
    walk(nodes, 1, None, open, matches, &mut rows);
    rows
}

/// The ids of every node whose label, or one of whose descendants' labels, holds the query, with
/// their ancestors, as sokkel-ui's `treeMatches`.
pub fn tree_matches(nodes: &[TreeNode], query: &str) -> HashSet<String> {
    fn visit(node: &TreeNode, needle: &str, path: &mut Vec<String>, keep: &mut HashSet<String>) -> bool {
        let own = node.label.to_lowercase().contains(needle);
        path.push(node.id.clone());
        let mut below = false;
        for child in &node.children {
            below = visit(child, needle, path, keep) || below;
        }
        path.pop();
        if own || below {
            keep.insert(node.id.clone());
            keep.extend(path.iter().cloned());
        }
        own || below
    }
    let needle = query.trim().to_lowercase();
    let mut keep = HashSet::new();
    for node in nodes {
        visit(node, &needle, &mut Vec::new(), &mut keep);
    }
    keep
}

/// The ids of a node's ancestors, from the top level down to its parent, or None when the tree
/// does not hold it.
pub fn tree_ancestors(nodes: &[TreeNode], id: &str) -> Option<Vec<String>> {
    fn visit(list: &[TreeNode], id: &str, path: &mut Vec<String>) -> Option<Vec<String>> {
        for node in list {
            if node.id == id {
                return Some(path.clone());
            }
            path.push(node.id.clone());
            let found = visit(&node.children, id, path);
            path.pop();
            if found.is_some() {
                return found;
            }
        }
        None
    }
    visit(nodes, id, &mut Vec::new())
}

/// The node with this id, anywhere in the tree.
pub fn tree_node<'a>(nodes: &'a [TreeNode], id: &str) -> Option<&'a TreeNode> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let Some(found) = tree_node(&node.children, id) {
            return Some(found);
        }
    }
    None
}

/// The words a tree says.
#[derive(Clone, Debug)]
pub struct TreeWords {
    /// Said when a search keeps no row.
    pub empty: String,
}

impl Default for TreeWords {
    fn default() -> Self {
        Self {
            empty: "Nothing matches.".into(),
        }
    }
}

/// What a tree did this frame: the node chosen, if one was.
#[derive(Clone, Debug, Default)]
pub struct TreeResponse {
    pub chosen: Option<String>,
}

/// A tree of nodes, one chosen at a time. A branch opens from its chevron, from Right on it, or by
/// choosing it; Left shuts it or goes to its parent; Up, Down, Home and End walk the rows. With a
/// query the rows matching it and those above them show, every branch among them open.
pub struct Tree<'a> {
    id: Id,
    nodes: &'a [TreeNode],
    selected: &'a mut Option<String>,
    open: &'a mut HashSet<String>,
    query: &'a str,
    words: TreeWords,
}

impl<'a> Tree<'a> {
    pub fn new(
        id: impl std::hash::Hash + std::fmt::Debug,
        nodes: &'a [TreeNode],
        selected: &'a mut Option<String>,
        open: &'a mut HashSet<String>,
    ) -> Self {
        Self {
            id: Id::new(id),
            nodes,
            selected,
            open,
            query: "",
            words: TreeWords::default(),
        }
    }

    /// Keep the rows whose label holds these words, and those above them.
    pub fn query(mut self, query: &'a str) -> Self {
        self.query = query;
        self
    }

    pub fn words(mut self, words: TreeWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> TreeResponse {
        let t = tokens(ui.ctx());
        let matches = (!self.query.trim().is_empty()).then(|| tree_matches(self.nodes, self.query));
        let rows = visible_rows(self.nodes, self.open, matches.as_ref());
        let mut response = TreeResponse::default();
        if rows.is_empty() {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    TextRole::Body
                        .rich(&t, &self.words.empty)
                        .color(t.color("muted-foreground")),
                );
            });
            ui.add_space(24.0);
            return response;
        }
        let height = 32.0;
        let width = ui.available_width();
        let radius = t.radius("md");
        let mut toggles: Vec<String> = Vec::new();
        let mut focus_row: Option<usize> = None;
        let mut any_focus = None;
        let spacing = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 0.0;
        for (index, row) in rows.iter().enumerate() {
            let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
            let id = self.id.with(&row.node.id);
            let hit = ui.interact(rect, id, Sense::click());
            let branch = row.node.is_branch();
            let expanded = branch && (matches.is_some() || self.open.contains(&row.node.id));
            let chosen = self.selected.as_deref() == Some(row.node.id.as_str());
            hit.widget_info(|| {
                let mut info =
                    egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, chosen, &row.node.label);
                info.selected = Some(chosen);
                info
            });
            if hit.has_focus() {
                any_focus = Some(index);
            }
            let painter = ui.painter();
            if chosen {
                let wash = if hit.hovered() { 0.18 } else { 0.12 };
                painter.rect_filled(rect, radius, t.color_at("primary", wash));
            } else if hit.hovered() {
                painter.rect_filled(rect, radius, t.color("hover"));
            }
            super::focus_ring_inset(painter, &t, rect, radius, &hit);
            let mut x = rect.left() + (row.level - 1) as f32 * 16.0 + 4.0;
            let center = rect.center().y;
            let muted = t.color("muted-foreground");
            if branch {
                let chevron = egui::Rect::from_center_size(pos2(x + 10.0, center), vec2(20.0, 20.0));
                let tap = ui.interact(chevron, id.with("chevron"), Sense::click());
                let color = if tap.hovered() { t.color("foreground") } else { muted };
                let icon = if expanded {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                };
                icons::paint(ui.painter(), chevron.center(), icon, 14.0, color);
                if tap.clicked() && matches.is_none() {
                    toggles.push(row.node.id.clone());
                }
            }
            x += 20.0 + 6.0;
            let painter = ui.painter();
            let icon = if expanded {
                row.node.open_icon.or(row.node.icon)
            } else {
                row.node.icon
            };
            if let Some(icon) = icon {
                icons::paint(painter, pos2(x + 8.0, center), icon, 16.0, muted);
                x += 16.0 + 6.0;
            }
            let mut right = rect.right() - 8.0;
            if let Some(meta) = &row.node.meta {
                let galley = WidgetText::from(TextRole::Help.rich(&t, meta)).into_galley(
                    ui,
                    Some(TextWrapMode::Extend),
                    f32::INFINITY,
                    TextRole::Help.font(&t),
                );
                right -= galley.size().x;
                ui.painter()
                    .galley(pos2(right, center - galley.size().y / 2.0), galley, muted);
                right -= 6.0;
            }
            let mut label = TextRole::Body.rich(&t, &row.node.label);
            if chosen {
                label = label.variation("wght", 500.0);
            }
            let galley = WidgetText::from(label).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                (right - x).max(1.0),
                TextRole::Body.font(&t),
            );
            let ink = if chosen {
                t.color("primary-ink")
            } else {
                t.color("foreground")
            };
            ui.painter()
                .galley(pos2(x, center - galley.size().y / 2.0), galley, ink);
            if hit.clicked() {
                *self.selected = Some(row.node.id.clone());
                response.chosen = Some(row.node.id.clone());
                if branch && !expanded && matches.is_none() {
                    toggles.push(row.node.id.clone());
                }
                hit.request_focus();
            }
        }
        // the keys walk the row with the focus
        if let Some(index) = any_focus {
            let row = &rows[index];
            let branch = row.node.is_branch();
            let expanded = branch && (matches.is_some() || self.open.contains(&row.node.id));
            ui.input_mut(|i| {
                let none = Modifiers::NONE;
                if i.consume_key(none, Key::ArrowDown) && index + 1 < rows.len() {
                    focus_row = Some(index + 1);
                } else if i.consume_key(none, Key::ArrowUp) && index > 0 {
                    focus_row = Some(index - 1);
                } else if i.consume_key(none, Key::Home) {
                    focus_row = Some(0);
                } else if i.consume_key(none, Key::End) {
                    focus_row = Some(rows.len() - 1);
                } else if i.consume_key(none, Key::ArrowRight) {
                    if branch && !expanded {
                        toggles.push(row.node.id.clone());
                    } else if expanded && index + 1 < rows.len() {
                        focus_row = Some(index + 1);
                    }
                } else if i.consume_key(none, Key::ArrowLeft) {
                    if expanded && matches.is_none() {
                        toggles.push(row.node.id.clone());
                    } else if let Some(parent) = row.parent {
                        focus_row = rows.iter().position(|r| r.node.id == parent);
                    }
                } else if i.consume_key(none, Key::Enter) || i.consume_key(none, Key::Space) {
                    response.chosen = Some(row.node.id.clone());
                }
            });
        }
        if let Some(chosen) = &response.chosen {
            *self.selected = Some(chosen.clone());
        }
        if let Some(to) = focus_row {
            ui.memory_mut(|m| m.request_focus(self.id.with(&rows[to].node.id)));
        }
        ui.spacing_mut().item_spacing.y = spacing;
        for id in toggles {
            if !self.open.remove(&id) {
                self.open.insert(id);
            }
        }
        response
    }
}
