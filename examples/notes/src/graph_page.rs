//! The vault as a network of its notes and their links, as the web's `pages/Graph.tsx`, or the
//! notes around the one open; a dot opens its note.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use eframe::egui::{self, Ui};
use sokkel_egui::{Dot, PageHeader, Segmented, SwitchField, TextRole, tokens, toolbar};
use sokkel_egui_graph::{KindColor, NetworkGraph, NetworkLink, NetworkNode, NetworkWords};

use crate::app::NotesApp;
use crate::model::Note;
use crate::vault::Vault;

/// The folder at the top of the vault a note sits under, which colors it.
fn top_folder(vault: &Vault, note: &Note) -> Option<String> {
    let mut at = note.folder.clone();
    let mut steps = 0;
    while let Some(id) = at.clone() {
        if steps >= 32 {
            break;
        }
        match vault.folder(&id).and_then(|f| f.parent.clone()) {
            None => return Some(id),
            Some(parent) => at = Some(parent),
        }
        steps += 1;
    }
    at
}

/// The notes within `depth` links of one note, either way.
fn around(vault: &Vault, id: &str, depth: usize) -> HashSet<String> {
    let mut near: HashSet<String> = HashSet::from([id.to_owned()]);
    for _ in 0..depth {
        for note in &vault.notes {
            for link in vault.links(&note.id) {
                let Some(to) = vault.resolve(&link.target) else {
                    continue;
                };
                if near.contains(&note.id) && !near.contains(&to.id) {
                    near.insert(to.id.clone());
                } else if near.contains(&to.id) && !near.contains(&note.id) {
                    near.insert(note.id.clone());
                }
            }
        }
    }
    near
}

impl NotesApp {
    pub fn graph_page(&mut self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let vault = self.backend.vault();
        let prefs = &self.backend.prefs;
        let current = prefs.active.clone().filter(|id| vault.note(id).is_some());
        let local = self.graph_around && current.is_some();

        // the top folders by name, each a kind color in turn
        let mut tops: Vec<(String, String)> = vault
            .folders
            .iter()
            .filter(|f| f.parent.is_none())
            .map(|f| (f.id.clone(), f.name.clone()))
            .collect();
        tops.sort_by(|a, b| a.1.cmp(&b.1));
        let colors: BTreeMap<String, KindColor> = tops
            .iter()
            .enumerate()
            .map(|(i, (id, _))| (id.clone(), KindColor::ALL[i % KindColor::ALL.len()]))
            .collect();

        let keep = current.as_deref().filter(|_| local).map(|id| around(vault, id, 2));
        let shown: Vec<&Note> = vault
            .notes
            .iter()
            .filter(|n| keep.as_ref().is_none_or(|k| k.contains(&n.id)))
            .collect();
        let ids: HashSet<&str> = shown.iter().map(|n| n.id.as_str()).collect();
        let mut links: Vec<NetworkLink> = Vec::new();
        let mut ghosts: Vec<NetworkNode> = Vec::new();
        let mut ghost_ids: BTreeSet<String> = BTreeSet::new();
        for note in &shown {
            for link in vault.links(&note.id) {
                match vault.resolve(&link.target) {
                    Some(to) => {
                        if ids.contains(to.id.as_str()) && to.id != note.id {
                            links.push(NetworkLink::new(&note.id, &to.id));
                        }
                    }
                    None if self.graph_missing => {
                        let id = format!("missing:{}", link.target.to_lowercase());
                        if ghost_ids.insert(id.clone()) {
                            ghosts.push(NetworkNode::new(&id, &link.target).faint(true));
                        } else if let Some(ghost) = ghosts.iter_mut().find(|g| g.id == id) {
                            // a later spelling names the ghost, as the web's map keeps the last
                            ghost.label = link.target.clone();
                        }
                        links.push(NetworkLink::new(&note.id, &id));
                    }
                    None => {}
                }
            }
        }
        let linked: HashSet<&str> = links
            .iter()
            .flat_map(|l| [l.source.as_str(), l.target.as_str()])
            .collect();
        let mut nodes: Vec<NetworkNode> = shown
            .iter()
            .filter(|n| prefs.orphans || linked.contains(n.id.as_str()))
            .map(|n| {
                let color = top_folder(vault, n).and_then(|top| colors.get(&top).copied());
                NetworkNode::new(&n.id, &n.title).color(color)
            })
            .collect();
        nodes.extend(ghosts);

        PageHeader::new("Graph")
            .count(format!("{} notes, {} links", nodes.len(), links.len()))
            .description(
                "Each dot a note, colored by its folder, larger the more it links; point at one to see its \
                 neighbors, drag it to pull them along, press it to open it.",
            )
            .show(ui, |_| {});

        let mut orphans = prefs.orphans;
        let mut missing = self.graph_missing;
        let mut scope = usize::from(local);
        toolbar(
            ui,
            |ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                ui.add(SwitchField::new(&mut orphans, "Notes with no link"));
                ui.add(SwitchField::new(&mut missing, "Notes not made yet"));
            },
            |ui| {
                let off = current.is_none().then(|| "Open a note first.".to_owned());
                ui.add(Segmented::new(&mut scope, ["Whole vault", "Around the open note"]).disabled([None, off]));
            },
        );

        // the graph takes the room the legend leaves, never under 320
        let legend_height = TextRole::Help.line_height(&t);
        let gap = 8.0;
        ui.spacing_mut().item_spacing.y = 0.0;
        let height = (ui.available_height() - gap - legend_height).max(320.0);
        let size = egui::vec2(ui.available_width(), height);
        let out = NetworkGraph::new("notes-graph", &nodes, &links)
            .current(current.as_deref())
            .size(size)
            .words(NetworkWords {
                label: "The notes and their links".into(),
                empty: "No note to show.".into(),
                ..Default::default()
            })
            .show(ui);
        ui.add_space(gap);
        let row = egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true);
        ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), legend_height), row, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(16.0, 4.0);
            for (id, name) in &tops {
                let color = colors.get(id).map_or(t.color("muted-foreground"), |c| c.fill(&t));
                ui.add(Dot::new(color, name));
            }
            ui.add(Dot::new(t.color("muted-foreground"), "At the top"));
            ui.add(Dot::new(
                t.color("muted-foreground").gamma_multiply(0.5),
                "Not made yet",
            ));
        });

        if orphans != self.backend.prefs.orphans {
            self.backend.set_prefs(|p| p.orphans = orphans);
        }
        self.graph_missing = missing;
        self.graph_around = scope == 1;
        if let Some(id) = out.pressed {
            match id.strip_prefix("missing:") {
                Some(_) => {
                    let label = nodes
                        .iter()
                        .find(|n| n.id == id)
                        .map(|n| n.label.clone())
                        .unwrap_or_default();
                    self.follow(&label, None);
                }
                None => self.open_note(&id),
            }
        }
    }
}
