//! One node chosen from a tree in a popover under a trigger, after sokkel-ui's `TreeSelect`.

use std::collections::HashSet;

use egui::{Id, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{Icon, TextRole, Tree, TreeNode, icons, tokens, tree_ancestors, tree_node};

/// The words a tree select says.
#[derive(Clone, Debug)]
pub struct TreeSelectWords {
    pub placeholder: String,
    pub search: String,
    pub clear: String,
}

impl Default for TreeSelectWords {
    fn default() -> Self {
        Self {
            placeholder: "Choose".into(),
            search: "Search".into(),
            clear: "Clear".into(),
        }
    }
}

#[derive(Clone, Default)]
struct Held {
    open: bool,
    query: String,
    unfolded: HashSet<String>,
    seeded: bool,
}

/// A trigger that looks like an input, naming the chosen node and the path above it, opening a
/// popover with a search over a tree; choosing a node closes it. `expanded` are the branches
/// unfolded the first time it opens.
pub struct TreeSelect<'a> {
    id: Id,
    nodes: &'a [TreeNode],
    value: &'a mut Option<String>,
    expanded: Vec<String>,
    invalid: bool,
    words: TreeSelectWords,
}

impl<'a> TreeSelect<'a> {
    pub fn new(id: impl egui::AsIdSalt, nodes: &'a [TreeNode], value: &'a mut Option<String>) -> Self {
        Self {
            id: Id::unique(id),
            nodes,
            value,
            expanded: Vec::new(),
            invalid: false,
            words: TreeSelectWords::default(),
        }
    }

    pub fn expanded(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.expanded = ids.into_iter().map(Into::into).collect();
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn words(mut self, words: TreeSelectWords) -> Self {
        self.words = words;
        self
    }

    /// Draws the select; true when the choice changed.
    pub fn show(self, ui: &mut Ui) -> bool {
        let t = tokens(ui.ctx());
        let mut held: Held = ui.data(|d| d.get_temp(self.id)).unwrap_or_default();
        let width = ui.available_width();
        let height = t.density("control");
        let (rect, trigger) = ui.allocate_exact_size(vec2(width, height), Sense::click());
        let chosen = self.value.as_deref().and_then(|id| tree_node(self.nodes, id));
        let label = chosen.map_or_else(|| self.words.placeholder.clone(), |n| n.label.clone());
        trigger.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, &label));
        let painter = ui.painter();
        let edge = if self.invalid {
            t.color("critical")
        } else if trigger.has_focus() || held.open {
            t.color("ring")
        } else {
            t.color("input")
        };
        painter.rect(
            rect,
            t.radius("control"),
            t.color("field"),
            Stroke::new(1.0, edge),
            egui::StrokeKind::Inside,
        );
        let mut x = rect.left() + 10.0;
        let room = rect.width() - 20.0 - 16.0 - 8.0;
        let main = WidgetText::from(TextRole::Body.rich(&t, &label)).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            room,
            TextRole::Body.font(&t),
        );
        let main_width = main.size().x;
        painter.galley(
            pos2(x, rect.center().y - main.size().y / 2.0),
            main,
            if chosen.is_some() {
                t.color("foreground")
            } else {
                t.color("muted-foreground")
            },
        );
        x += main_width + 6.0;
        if let Some(id) = self.value.as_deref()
            && let Some(path) = tree_ancestors(self.nodes, id)
        {
            let names: Vec<String> = path
                .iter()
                .filter_map(|a| tree_node(self.nodes, a).map(|n| n.label.clone()))
                .collect();
            if !names.is_empty() {
                let trail = WidgetText::from(TextRole::Help.rich(&t, names.join(" / "))).into_galley(
                    ui,
                    Some(TextWrapMode::Truncate),
                    (rect.right() - 34.0 - x).max(1.0),
                    TextRole::Help.font(&t),
                );
                painter.galley(
                    pos2(x, rect.center().y - trail.size().y / 2.0),
                    trail,
                    t.color("muted-foreground"),
                );
            }
        }
        icons::paint(
            painter,
            pos2(rect.right() - 10.0 - 8.0, rect.center().y),
            Icon::ChevronsUpDown,
            16.0,
            t.color("muted-foreground"),
        );
        if trigger.clicked() {
            held.open = !held.open;
        }
        let mut changed = false;
        if held.open {
            if !held.seeded {
                held.unfolded.extend(self.expanded.iter().cloned());
                held.seeded = true;
            }
            let width = rect.width().max(288.0);
            let mut chosen_now = self.value.clone();
            let area = egui::Area::new(self.id.with("popover"))
                .order(egui::Order::Foreground)
                .fixed_pos(pos2(rect.left(), rect.bottom() + 4.0))
                .show(ui.ctx(), |ui| {
                    super::popup_frame(&t).inner_margin(0.0).show(ui, |ui| {
                        ui.set_width(width);
                        ui.spacing_mut().item_spacing.y = 0.0;
                        // the search across the top, a rule under it
                        ui.horizontal(|ui| {
                            ui.set_height(36.0);
                            ui.add_space(10.0);
                            let (slot, _) = ui.allocate_exact_size(vec2(16.0, 36.0), Sense::hover());
                            icons::paint(
                                ui.painter(),
                                slot.center(),
                                Icon::Search,
                                16.0,
                                t.color("muted-foreground"),
                            );
                            ui.add_space(8.0);
                            let edit = egui::TextEdit::singleline(&mut held.query)
                                .frame(egui::Frame::NONE)
                                .font(TextRole::Body.font(&t))
                                .hint_text(
                                    TextRole::Body
                                        .rich(&t, &self.words.search)
                                        .color(t.color("muted-foreground")),
                                )
                                .align(egui::Align2::LEFT_CENTER)
                                .desired_width(width - 10.0 - 16.0 - 8.0 - 70.0);
                            ui.add(edit).accessible_name(&self.words.search);
                            if chosen_now.is_some() {
                                let clear = ui.add(
                                    egui::Button::new(
                                        TextRole::Help
                                            .rich(&t, &self.words.clear)
                                            .color(t.color("muted-foreground")),
                                    )
                                    .frame(false),
                                );
                                if clear.clicked() {
                                    chosen_now = None;
                                }
                            }
                        });
                        let rule = ui.available_rect_before_wrap();
                        ui.painter()
                            .hline(rule.x_range(), rule.top(), Stroke::new(1.0, t.color("border")));
                        egui::ScrollArea::vertical()
                            .id_salt(self.id.with("rows"))
                            .max_height(320.0)
                            .show(ui, |ui| {
                                egui::Frame::new().inner_margin(4.0).show(ui, |ui| {
                                    let query = held.query.clone();
                                    let mut picked = chosen_now.clone();
                                    let out =
                                        Tree::new(self.id.with("tree"), self.nodes, &mut picked, &mut held.unfolded)
                                            .query(&query)
                                            .show(ui);
                                    if let Some(id) = out.chosen {
                                        chosen_now = Some(id);
                                        held.open = false;
                                        held.query.clear();
                                    }
                                });
                            });
                    });
                });
            if chosen_now != *self.value {
                *self.value = chosen_now;
                changed = true;
            }
            let outside = ui.input(|i| i.pointer.any_pressed())
                && ui
                    .input(|i| i.pointer.interact_pos())
                    .is_some_and(|p| !area.response.rect.contains(p) && !rect.contains(p));
            if outside || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                held.open = false;
            }
        }
        ui.data_mut(|d| d.insert_temp(self.id, held));
        changed
    }
}
