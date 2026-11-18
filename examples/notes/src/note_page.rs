//! One note, as the web's `pages/NotePage.tsx`: the open notes as tabs over it, its name to rename
//! in place, and its markdown beside the page it makes.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    CodeEditor, CodeLanguage, CodeSize, CodeVariant, DocumentTab, DocumentTabs, DocumentTabsWords, DropdownMenu,
    EditPreview, EditPreviewView, Icon, IconButton, InlineEdit, Markdown, MarkdownScale, MenuItem, PaneHeader,
    Segmented, section_title_weight,
};

use crate::app::NotesApp;
use crate::backend::{Follow, NewNote, TabClosed};
use crate::model::{FontSize, View, bad_title};

/// The views the switch offers, in its order.
const VIEWS: [View; 3] = [View::Edit, View::Both, View::Preview];

/// The view after this one, as Mod-E steps through them.
fn next_view(view: View) -> View {
    match view {
        View::Edit => View::Both,
        View::Both => View::Preview,
        View::Preview => View::Edit,
    }
}

impl NotesApp {
    pub fn note_page(&mut self, ui: &mut Ui, id: &str) {
        let now = self.now();
        // a note shown is open as a tab, and the one shown, as the web's page does on mounting
        if self.backend.prefs.active.as_deref() != Some(id) {
            self.backend.show(id);
        }
        let vault = self.backend.vault();
        let Some(note) = vault.note(id).cloned() else { return };
        let prefs = self.backend.prefs.clone();
        let view = prefs.views.get(id).copied().unwrap_or(prefs.default_view);
        let tabs: Vec<DocumentTab> = prefs
            .tabs
            .iter()
            .filter_map(|tab| vault.note(tab))
            .map(|one| {
                DocumentTab::new(&one.id, &one.title)
                    .icon(if one.day.is_some() {
                        Icon::CalendarDays
                    } else {
                        Icon::FileText
                    })
                    .dirty(self.backend.draft(&one.id).is_some())
            })
            .collect();
        let recent: Vec<(String, String, bool)> = prefs
            .recent
            .iter()
            .filter(|r| r.as_str() != id)
            .filter_map(|r| vault.note(r).map(|n| (n.id.clone(), n.title.clone(), n.day.is_some())))
            .collect();
        let links: Vec<String> = vault
            .notes
            .iter()
            .filter(|one| one.id != id)
            .map(|one| one.title.clone())
            .collect();
        let others: Vec<String> = links.clone();
        let mut text = self.backend.draft(id).map_or_else(|| note.body.clone(), str::to_owned);

        // the keys this page answers: Mod-E steps the view, Mod-S saves now
        let (step, save) = ui.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::E),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::S),
            )
        });
        let mut set_view = step.then(|| next_view(view));

        let spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut open_recent = None;
        let mut new_note = false;
        let tabs_out = DocumentTabs::new("note-tabs", &tabs, Some(id))
            .closable(true)
            .words(DocumentTabsWords {
                label: "Open notes".into(),
                ..Default::default()
            })
            .show(ui, |ui| {
                new_note = ui.add(IconButton::new(Icon::FilePlus, "New note").xs()).clicked();
                let trigger = ui.add(IconButton::new(Icon::History, "Recent notes").xs());
                let mut items = vec![MenuItem::Label("Recent".into())];
                if recent.is_empty() {
                    items.push(MenuItem::action("No other note opened yet").disabled(true));
                } else {
                    items.extend(recent.iter().map(|(_, title, day)| {
                        MenuItem::action(title).icon(if *day { Icon::CalendarDays } else { Icon::FileText })
                    }));
                }
                if let Some(chosen) = DropdownMenu::new(&items).end(true).width(256.0).show(&trigger) {
                    open_recent = recent.get(chosen.wrapping_sub(1)).map(|(id, ..)| id.clone());
                }
            });

        let mut chosen_view = VIEWS.iter().position(|v| *v == view).unwrap_or(1);
        let mut renamed = None;
        let mut more = None;
        let weight = section_title_weight(ui);
        let note_title = note.title.clone();
        PaneHeader::new()
            .title_with(|ui| {
                renamed = InlineEdit::new(("note-name", id), "Name", &note_title)
                    .weight(weight)
                    .validate(move |title| {
                        if title.trim() == note_title {
                            None
                        } else {
                            let taken: Vec<&str> = others.iter().map(String::as_str).collect();
                            bad_title(title, &taken)
                        }
                    })
                    .show(ui);
            })
            .show(ui, |ui| {
                let trigger = ui.add(IconButton::new(Icon::Ellipsis, "More"));
                let items = [
                    MenuItem::action("Rename").icon(Icon::Pencil),
                    MenuItem::action("Move to").icon(Icon::FolderInput),
                    MenuItem::action("Close tab").icon(Icon::FileText),
                    MenuItem::Separator,
                    MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
                ];
                more = DropdownMenu::new(&items).end(true).width(192.0).show(&trigger);
                ui.add_space(4.0);
                if ui
                    .add(Segmented::new(&mut chosen_view, ["Edit", "Both", "Read"]))
                    .changed()
                {
                    set_view = Some(VIEWS[chosen_view]);
                }
            });

        let mut edited = false;
        let mut link_opened = None;
        let mut tag_opened = None;
        let reveal = self.reveal.take();
        let mut followed = None;
        let body = text.clone();
        let view_shown = match set_view.unwrap_or(view) {
            View::Edit => EditPreviewView::Edit,
            View::Both => EditPreviewView::Both,
            View::Preview => EditPreviewView::Preview,
        };
        EditPreview::new(("notes-split", id)).view(view_shown).show(
            ui,
            |ui| {
                let out = CodeEditor::new(("note-editor", id), &mut text)
                    .label(format!("{} as markdown", note.title))
                    .language(CodeLanguage::Markdown)
                    .variant(CodeVariant::Pane)
                    .links(&links)
                    .wrap(prefs.wrap)
                    .line_numbers(prefs.line_numbers)
                    .size(match prefs.font_size {
                        FontSize::Sm => CodeSize::Sm,
                        FontSize::Md => CodeSize::Md,
                        FontSize::Lg => CodeSize::Lg,
                    })
                    .search(true)
                    .placeholder("Write in markdown; [[ links to a note, # tags it")
                    .show(ui);
                edited = out.changed;
                followed = out.followed;
            },
            |ui| {
                let exists = |target: &str| vault.resolve(target).is_some();
                let out = Markdown::new(&body)
                    .scale(MarkdownScale::Document)
                    .anchors(true)
                    .links(&exists)
                    .tags(true)
                    .reveal(reveal.as_deref())
                    .show(ui);
                link_opened = out.opened;
                tag_opened = out.tag;
            },
        );
        ui.spacing_mut().item_spacing = spacing;

        // what was asked, once the page is drawn
        if edited {
            self.backend.edit(id, &text, now);
        }
        if save {
            self.backend.flush(Some(id), now);
        }
        if let Some(next) = set_view {
            self.backend.set_view(id, next);
        }
        if let Some(title) = renamed
            && let Err(reason) = self.backend.rename_note(id, &title, now)
        {
            sokkel_egui::Toast::error(reason).show(ui.ctx());
        }
        if let Some((target, _heading)) = link_opened {
            self.follow(&target, Some(id));
        }
        if let Some(tag) = tag_opened {
            self.go(&format!("tags/{tag}"));
        }
        if let Some(target) = followed {
            self.follow(&target, Some(id));
        }
        if let Some(open) = tabs_out.chosen.filter(|t| t != id) {
            self.open_note(&open);
        }
        if let Some(open) = open_recent {
            self.open_note(&open);
        }
        if new_note {
            let made = self.backend.new_note(note.folder.as_deref(), now);
            self.open_note(&made.id);
        }
        let mut close = tabs_out.closed;
        if more == Some(2) {
            close = Some(id.to_owned());
        }
        if let Some(closing) = close
            && let TabClosed::Active { next } = self.backend.close_tab(&closing, now)
        {
            match next {
                Some(next) => self.open_note(&next),
                None => self.go("notes"),
            }
        }
    }

    /// Follows a link's name: to its note, or makes the note it names and opens it.
    pub fn follow(&mut self, target: &str, from: Option<&str>) {
        let now = self.now();
        match self.backend.follow(target, from) {
            Follow::Open { id } => self.open_note(&id),
            Follow::Create { title, folder } => {
                let made = self.backend.create_note(
                    NewNote {
                        title: Some(title),
                        folder,
                        body: None,
                        day: None,
                    },
                    now,
                );
                self.open_note(&made.id);
            }
        }
    }
}
