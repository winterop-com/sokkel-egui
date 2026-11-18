//! Every note in the vault, or in one folder and the folders under it, as the web's
//! `pages/AllNotes.tsx`: a field that makes a note, and the notes newest change first.

use eframe::egui::Ui;
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, Cell, Column, DataTable, Icon, IconButton, MenuItem, PageHeader, QuickAdd,
    StackLayout, TableState, TableVariant, relative_time_short,
};

use crate::app::NotesApp;
use crate::backend::NewNote;
use crate::model::{Note, Order};
use crate::vault::{Vault, folder_and_below, notes_in_order};

/// The name column's sort: newest change first, as the web's default sort.
const CHANGED: usize = 4;

/// What a listing shows of a note: its name, where it is, its tags, the links into it and when it
/// changed, as the web's `noteColumns`.
pub fn note_columns() -> Vec<Column> {
    // the web's widths: a list's cells have no padding of their own
    vec![
        Column::new("Name", 224.0).fill_share(2.0).sortable(true),
        Column::new("Folder", 128.0).fill().sortable(true),
        Column::new("Tags", 128.0).fill(),
        Column::new("Links in", 96.0).numeric().sortable(true),
        Column::new("Changed", 136.0).sortable(true),
    ]
}

/// A note's cells, in the columns' order.
pub fn note_cells(vault: &Vault, note: &Note, now: i64) -> Vec<Cell> {
    let path = vault.path(note.folder.as_deref());
    vec![
        Cell::IconName {
            icon: if note.day.is_some() {
                Icon::CalendarDays
            } else {
                Icon::FileText
            },
            name: note.title.clone(),
        },
        Cell::Muted(if path.is_empty() { "Vault".into() } else { path }),
        Cell::Tags(vault.tags_of(&note.id).iter().map(|t| format!("#{t}")).collect()),
        Cell::Number {
            value: vault.backlinks(&note.id).len() as f64,
            text: vault.backlinks(&note.id).len().to_string(),
        },
        Cell::Number {
            value: note.updated as f64,
            text: relative_time_short((note.updated - now) as f64),
        },
    ]
}

/// What a note's row offers in its menu.
pub fn note_menu() -> Vec<MenuItem> {
    vec![
        MenuItem::action("Open").icon(Icon::FileText),
        MenuItem::action("Rename").icon(Icon::Pencil),
        MenuItem::action("Move to").icon(Icon::FolderInput),
        MenuItem::Separator,
        MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
    ]
}

impl NotesApp {
    pub fn all_notes(&mut self, ui: &mut Ui, folder: Option<&str>) {
        let now = self.now();
        let order = self.backend.order();
        let vault = self.backend.vault();
        let manual = order == Order::Manual;
        let inside: Vec<Note> = if manual {
            notes_in_order(vault, folder, order, self.backend.prefs.daily_folder.as_deref())
                .into_iter()
                .cloned()
                .collect()
        } else {
            match folder {
                None => vault.notes.clone(),
                Some(id) => {
                    let within = folder_and_below(vault, id);
                    vault
                        .notes
                        .iter()
                        .filter(|n| n.folder.as_ref().is_some_and(|f| within.contains(f)))
                        .cloned()
                        .collect()
                }
            }
        };
        let name = match folder {
            None => "All notes".to_owned(),
            Some(id) => vault.folder(id).map_or("Folder".to_owned(), |f| f.name.clone()),
        };
        let description = folder.map(|id| vault.path(Some(id)));
        let rows: Vec<Vec<Cell>> = inside.iter().map(|n| note_cells(vault, n, now)).collect();
        let keys: Vec<String> = inside.iter().map(|n| n.id.clone()).collect();
        let count = format!("{} {}", inside.len(), if inside.len() == 1 { "note" } else { "notes" });
        let mut header = PageHeader::new(&name).count(count);
        if let Some(path) = description {
            header = header.description(path);
        }
        let mut new_folder = false;
        header.show(ui, |ui| {
            new_folder = ui
                .add(
                    Button::new("New folder")
                        .icon(Icon::FolderPlus)
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm),
                )
                .clicked();
            if folder.is_some() {
                ui.add(IconButton::new(Icon::Trash2, "Delete folder"));
                ui.add(IconButton::new(Icon::FolderInput, "Move folder"));
                ui.add(IconButton::new(Icon::Pencil, "Rename folder"));
            }
        });
        let placeholder = if folder.is_none() {
            "New note".to_owned()
        } else {
            format!("New note in {name}")
        };
        let made = QuickAdd::new("new-note", &mut self.quick)
            .placeholder(placeholder)
            .width(ui.available_width())
            .show(ui);
        let columns = note_columns();
        let key = format!("notes-{}", folder.unwrap_or("all"));
        let state = self.tables.entry(key.clone()).or_insert_with(|| {
            if manual {
                TableState::default()
            } else {
                TableState::sorted(CHANGED, false)
            }
        });
        let out = DataTable::new(key.as_str(), &columns, &rows, state)
            .variant(TableVariant::List)
            .fits(true)
            .stack_layout(StackLayout::Compact)
            .keys(keys.iter())
            .reorder(manual)
            .actions(|_| note_menu())
            .height(f32::INFINITY)
            .footer(false)
            .show(ui);
        let opened = state.opened.take();
        if let Some(title) = made.filter(|t| !t.trim().is_empty()) {
            let note = self.backend.create_note(
                NewNote {
                    title: Some(title.trim().to_owned()),
                    folder: folder.map(str::to_owned),
                    body: None,
                    day: None,
                },
                now,
            );
            self.open_note(&note.id);
        }
        if let Some(id) = opened.and_then(|row| keys.get(row)) {
            self.open_note(&id.clone());
        }
        if let Some((row, 0)) = out.action
            && let Some(id) = keys.get(row).cloned()
        {
            self.open_note(&id);
        }
        let _ = new_folder;
    }
}
