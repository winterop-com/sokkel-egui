//! Nodes in levels that open and shut, after sokkel-ui's `Tree`: one chosen at a time, a search
//! keeping the rows that match and those above them, the arrow keys walking the rows.

use std::collections::HashSet;

use egui::{Id, Key, Modifiers, Sense, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{DropdownMenu, Icon, MenuItem, TextRole, icons, tokens};

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
    /// A row's menu button.
    pub actions: String,
}

impl Default for TreeWords {
    fn default() -> Self {
        Self {
            empty: "Nothing matches.".into(),
            actions: "Actions".into(),
        }
    }
}

/// What a tree did this frame: the node chosen, if one was, and a node moved by dragging.
#[derive(Clone, Debug, Default)]
pub struct TreeResponse {
    pub chosen: Option<String>,
    /// A node dropped in a new place: its id, the branch it goes into (`None` for the top level),
    /// and its place among that branch's nodes once it has left its old one.
    pub moved: Option<(String, Option<String>, usize)>,
    /// An entry pressed in a row's menu: the node's id and the entry's index.
    pub action: Option<(String, usize)>,
}

/// A copy of `nodes` with `id` taken from its place and put into `into` (the top level for
/// `None`) at `index`, as sokkel-ui's `moveTreeNode`.
pub fn move_tree_node(nodes: &[TreeNode], id: &str, into: Option<&str>, index: usize) -> Vec<TreeNode> {
    fn take(list: &mut Vec<TreeNode>, id: &str) -> Option<TreeNode> {
        if let Some(at) = list.iter().position(|n| n.id == id) {
            return Some(list.remove(at));
        }
        list.iter_mut().find_map(|n| take(&mut n.children, id))
    }
    fn find<'a>(list: &'a mut [TreeNode], id: &str) -> Option<&'a mut TreeNode> {
        for node in list {
            if node.id == id {
                return Some(node);
            }
            if let Some(found) = find(&mut node.children, id) {
                return Some(found);
            }
        }
        None
    }
    let mut out = nodes.to_vec();
    let Some(node) = take(&mut out, id) else { return out };
    let list = match into {
        None => &mut out,
        Some(parent) => match find(&mut out, parent) {
            Some(parent) => &mut parent.children,
            None => return nodes.to_vec(),
        },
    };
    let at = index.min(list.len());
    list.insert(at, node);
    out
}

/// Whether a node may go into a branch (`None` for the top level).
pub type CanMove<'a> = &'a dyn Fn(&str, Option<&str>) -> bool;

/// What a node's menu offers, or `None` for a node without one.
pub type NodeActions<'a> = &'a dyn Fn(&TreeNode) -> Option<Vec<MenuItem>>;

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
    movable: Option<CanMove<'a>>,
    actions: Option<NodeActions<'a>>,
    choosing: bool,
}

impl<'a> Tree<'a> {
    pub fn new(
        id: impl std::hash::Hash + std::fmt::Debug,
        nodes: &'a [TreeNode],
        selected: &'a mut Option<String>,
        open: &'a mut HashSet<String>,
    ) -> Self {
        Self {
            id: Id::unique(id),
            nodes,
            selected,
            open,
            query: "",
            words: TreeWords::default(),
            movable: None,
            actions: None,
            choosing: true,
        }
    }

    /// Rows dragged to another place, where `can_move` lets them go; the drop comes back as the
    /// response's `moved`, for the caller to keep.
    pub fn movable(mut self, can_move: CanMove<'a>) -> Self {
        self.movable = Some(can_move);
        self
    }

    /// A menu on each row that has one, as the web's `actions`: a 24 button at the row's end, 4
    /// from it, seen while the row is pointed at or its menu is open; the press comes back as the
    /// response's `action`.
    pub fn actions(mut self, actions: NodeActions<'a>) -> Self {
        self.actions = Some(actions);
        self
    }

    /// Rows that are not chosen, as the web's `selection="none"`: a press opens a branch.
    pub fn choosing(mut self, choosing: bool) -> Self {
        self.choosing = choosing;
        self
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
        let drag_id = self.id.with("dragging");
        let mut dragging: Option<String> = ui.data(|d| d.get_temp(drag_id));
        let mut rects: Vec<egui::Rect> = Vec::with_capacity(rows.len());
        for (index, row) in rows.iter().enumerate() {
            let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
            rects.push(rect);
            let id = self.id.with(&row.node.id);
            let sense = if self.movable.is_some() {
                Sense::click_and_drag()
            } else {
                Sense::click()
            };
            let hit = ui.interact(rect, id, sense);
            if hit.drag_started() && self.movable.is_some() {
                dragging = Some(row.node.id.clone());
            }
            let branch = row.node.is_branch();
            let expanded = branch && (matches.is_some() || self.open.contains(&row.node.id));
            let chosen = self.selected.as_deref() == Some(row.node.id.as_str());
            hit.widget_info(|| {
                let mut info = egui::WidgetInfo::selected(egui::Role::Button, true, chosen, &row.node.label);
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
            let menu = self.actions.and_then(|actions| actions(row.node));
            let mut right = rect.right() - if menu.is_some() { 4.0 } else { 8.0 };
            if let Some(items) = &menu {
                let button = egui::Rect::from_center_size(pos2(right - 12.0, center), vec2(24.0, 24.0));
                let trigger = ui.interact(button, id.with("actions"), Sense::click());
                trigger.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &self.words.actions));
                let open = egui::Popup::is_id_open(ui.ctx(), trigger.id.with("menu"));
                if hit.hovered() || trigger.hovered() || open {
                    if trigger.hovered() || open {
                        ui.painter().rect_filled(button, t.radius("sm"), t.color("hover"));
                    }
                    icons::paint(ui.painter(), button.center(), Icon::Ellipsis, 16.0, muted);
                }
                if let Some(chosen) = DropdownMenu::new(items).end(true).width(192.0).show(&trigger) {
                    response.action = Some((row.node.id.clone(), chosen));
                }
                right -= 24.0 + 6.0;
            }
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
                if self.choosing {
                    *self.selected = Some(row.node.id.clone());
                    response.chosen = Some(row.node.id.clone());
                    if branch && !expanded && matches.is_none() {
                        toggles.push(row.node.id.clone());
                    }
                } else if branch && matches.is_none() {
                    toggles.push(row.node.id.clone());
                }
                hit.request_focus();
            }
        }
        // a row dragged: where it would land, drawn as a line, and kept on release
        if let (Some(from), Some(can_move)) = (dragging.clone(), self.movable) {
            let pointer = ui.input(|i| i.pointer.interact_pos());
            let target = pointer.and_then(|p| {
                let at = rects.iter().position(|r| r.contains(p))?;
                Some((at, p.y > rects[at].center().y))
            });
            let landing = target.and_then(|(at, below)| {
                let dragged = rows.iter().find(|r| r.node.id == from)?;
                let onto = &rows[at];
                if onto.node.id == from {
                    return None;
                }
                let siblings = |parent: Option<&str>| -> Vec<&TreeNode> {
                    match parent {
                        None => self.nodes.iter().collect(),
                        Some(p) => tree_node(self.nodes, p)
                            .map(|n| n.children.iter().collect())
                            .unwrap_or_default(),
                    }
                };
                // a top-level node goes among the top level, by the top-level row it is dropped on
                let (into, place, line) = if dragged.level == 1 {
                    let mut top = at;
                    while rows[top].level > 1 {
                        top -= 1;
                    }
                    let list = siblings(None);
                    let pos = list.iter().position(|n| n.id == rows[top].node.id)?;
                    let after =
                        below && onto.level == 1 && !(onto.node.is_branch() && self.open.contains(&onto.node.id));
                    let place = if after || onto.level > 1 { pos + 1 } else { pos };
                    (None, place, if after { rects[at].bottom() } else { rects[top].top() })
                } else if onto.node.is_branch() && onto.level < dragged.level {
                    (Some(onto.node.id.clone()), 0, rects[at].bottom())
                } else {
                    let parent = onto.parent?;
                    let list = siblings(Some(parent));
                    let pos = list.iter().position(|n| n.id == onto.node.id)?;
                    let place = if below { pos + 1 } else { pos };
                    (
                        Some(parent.to_owned()),
                        place,
                        if below { rects[at].bottom() } else { rects[at].top() },
                    )
                };
                if !can_move(&from, into.as_deref()) {
                    return None;
                }
                // its place once it has left its old one in the same list
                let old = siblings(into.as_deref()).iter().position(|n| n.id == from);
                let place = match old {
                    Some(old) if old < place => place - 1,
                    _ => place,
                };
                Some((into, place, line, onto.level.min(dragged.level)))
            });
            if let Some((_, _, line, level)) = &landing {
                let left = rects[0].left() + (*level as f32 - 1.0) * 16.0 + 4.0;
                ui.painter().hline(
                    left..=rects[0].right(),
                    *line,
                    egui::Stroke::new(2.0, t.color("primary")),
                );
            }
            if ui.input(|i| i.pointer.any_released()) {
                if let Some((into, place, _, _)) = landing {
                    response.moved = Some((from, into, place));
                }
                dragging = None;
            }
        }
        ui.data_mut(|d| match &dragging {
            Some(id) => {
                d.insert_temp(drag_id, id.clone());
            }
            None => d.remove::<String>(drag_id),
        });
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

#[cfg(test)]
mod moving {
    use super::{TreeNode, move_tree_node};

    #[test]
    fn a_node_moves_into_another_branch_and_within_its_own() {
        let nodes = vec![
            TreeNode::new("a", "A").children(vec![TreeNode::new("a1", "A1"), TreeNode::new("a2", "A2")]),
            TreeNode::new("b", "B").children(vec![TreeNode::new("b1", "B1")]),
        ];
        let moved = move_tree_node(&nodes, "a2", Some("b"), 0);
        assert_eq!(
            moved[1].children.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
            ["a2", "b1"]
        );
        let moved = move_tree_node(&nodes, "a1", Some("a"), 1);
        assert_eq!(
            moved[0].children.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
            ["a2", "a1"]
        );
        let moved = move_tree_node(&nodes, "b", None, 0);
        assert_eq!(moved.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(), ["b", "a"]);
    }
}
