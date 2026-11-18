//! Every tag with how many notes carry it, or one tag's notes, as the web's `pages/Tags.tsx`.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    BarList, BarListItem, Cell, Column, DataTable, Empty, PageHeader, Section, StackLayout, TableState, TableVariant,
    relative_time_short, tokens,
};

use crate::all_notes::{note_cells, note_columns, note_menu};
use crate::app::NotesApp;
use crate::model::Note;

/// A listing's sort: the notes column, most first; one tag's notes, newest change first.
const NOTES: usize = 1;
const CHANGED: usize = 4;

impl NotesApp {
    pub fn tags(&mut self, ui: &mut Ui, tag: Option<&str>) {
        match tag {
            Some(tag) => self.one_tag(ui, tag),
            None => self.every_tag(ui),
        }
    }

    fn every_tag(&mut self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let now = self.now();
        let vault = self.backend.vault();
        // each tag, the notes under it, and when one of them last changed
        let all: Vec<(String, usize, i64)> = vault
            .tags()
            .iter()
            .map(|(tag, ids)| {
                let latest = ids
                    .iter()
                    .filter_map(|id| vault.note(id))
                    .map(|n| n.updated)
                    .max()
                    .unwrap_or(0);
                (tag.clone(), ids.len(), latest)
            })
            .collect();
        let most = all.iter().map(|r| r.1).max().unwrap_or(1).max(1);
        let needle = self.tag_query.trim().to_lowercase();
        let shown: Vec<&(String, usize, i64)> = all
            .iter()
            .filter(|(tag, ..)| needle.is_empty() || format!("#{tag}").to_lowercase().contains(&needle))
            .collect();
        let columns = vec![
            Column::new("Tag", 192.0).fill().sortable(true),
            Column::new("Notes", 96.0).numeric().sortable(true),
            Column::new("Last used", 136.0).sortable(true),
        ];
        let rows: Vec<Vec<Cell>> = shown
            .iter()
            .map(|(tag, notes, latest)| {
                vec![
                    Cell::TagName(tag.clone()),
                    Cell::Number {
                        value: *notes as f64,
                        text: notes.to_string(),
                    },
                    Cell::Number {
                        value: *latest as f64,
                        text: relative_time_short((latest - now) as f64),
                    },
                ]
            })
            .collect();
        let keys: Vec<String> = shown.iter().map(|r| r.0.clone()).collect();
        let mut by_use: Vec<&(String, usize, i64)> = all.iter().collect();
        by_use.sort_by_key(|r| std::cmp::Reverse(r.1));
        let bars: Vec<BarListItem> = by_use
            .iter()
            .take(8)
            .map(|(tag, notes, _)| BarListItem {
                label: format!("#{tag}"),
                value: *notes as f64 / most as f64,
                chosen: false,
                note: None,
            })
            .collect();

        PageHeader::new("Tags")
            .count(format!("{} tags", all.len()))
            .description("Write # and a word in a note to tag it.")
            .show(ui, |_| {});
        let gap = t.density("section-gap");
        let mut opened = None;
        let mut query = std::mem::take(&mut self.tag_query);
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
            let side = 320.0;
            let main = ui.available_width() - side - gap;
            ui.allocate_ui_with_layout(egui::vec2(main, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(main);
                let state = self
                    .tables
                    .entry("tags".to_owned())
                    .or_insert_with(|| TableState::sorted(NOTES, false));
                DataTable::new("tags", &columns, &rows, state)
                    .variant(TableVariant::List)
                    .fits(true)
                    .stack_layout(StackLayout::Compact)
                    .keys(keys.iter())
                    .chevron(true)
                    .height(f32::INFINITY)
                    .search(&mut query)
                    .search_placeholder("Filter")
                    .empty(|ui| {
                        Empty::new("No tag is named like that.").show(ui, |_| {});
                    })
                    .footer(false)
                    .show(ui);
                opened = state.opened.take();
            });
            ui.allocate_ui_with_layout(egui::vec2(side, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                ui.set_width(side);
                Section::new("Most used").show(ui, |ui| {
                    let most = most as f64;
                    BarList::new(&bars)
                        .format(move |value| format!("{}", (value * most).round()))
                        .show(ui);
                });
            });
        });
        self.tag_query = query;
        if let Some(tag) = opened.and_then(|row| keys.get(row)) {
            self.go(&format!("tags/{tag}"));
        }
    }

    fn one_tag(&mut self, ui: &mut Ui, tag: &str) {
        let now = self.now();
        let vault = self.backend.vault();
        let notes: Vec<Note> = vault
            .tagged(tag)
            .iter()
            .filter_map(|id| vault.note(id))
            .cloned()
            .collect();
        let columns = note_columns();
        let rows: Vec<Vec<Cell>> = notes.iter().map(|n| note_cells(vault, n, now)).collect();
        let keys: Vec<String> = notes.iter().map(|n| n.id.clone()).collect();
        let header = PageHeader::new(format!("#{tag}"))
            .count(format!(
                "{} {}",
                notes.len(),
                if notes.len() == 1 { "note" } else { "notes" }
            ))
            .back("Tags")
            .description("The notes that carry this tag.")
            .show(ui, |_| {});
        let key = format!("tag-{tag}");
        let state = self
            .tables
            .entry(key.clone())
            .or_insert_with(|| TableState::sorted(CHANGED, false));
        let empty = format!("No note carries #{tag} now.");
        let out = DataTable::new(key.as_str(), &columns, &rows, state)
            .variant(TableVariant::List)
            .fits(true)
            .stack_layout(StackLayout::Compact)
            .keys(keys.iter())
            .actions(|_| note_menu())
            .height(f32::INFINITY)
            .empty(|ui| {
                Empty::new(empty).show(ui, |_| {});
            })
            .footer(false)
            .show(ui);
        let opened = state.opened.take();
        if header.back {
            self.go("tags");
        }
        if let Some(id) = opened.and_then(|row| keys.get(row)) {
            self.open_note(&id.clone());
        }
        if let Some((row, index)) = out.action
            && let Some(id) = keys.get(row).cloned()
        {
            self.note_action(&id, index);
        }
    }
}
