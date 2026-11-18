//! The vault searched in full, as the web's `pages/SearchPage.tsx`: every note holding the words,
//! with the line each says them in, narrowed by kind and folder.

use eframe::egui::Ui;
use sokkel_egui::{PageHeader, SearchResults, SearchResultsWords};

use crate::app::NotesApp;
use crate::search::search_vault;

impl NotesApp {
    pub fn search_page(&mut self, ui: &mut Ui) {
        PageHeader::new("Search")
            .description("Every note holding all the words, in its name or its text.")
            .show(ui, |_| {});
        let found = search_vault(self.backend.vault(), &self.search_query, &self.search_chosen);
        let mut query = std::mem::take(&mut self.search_query);
        let out = SearchResults::new(&mut query, &found.groups)
            .facets(&found.facets, &self.search_chosen)
            .words(SearchResultsWords {
                placeholder: "Search the vault".into(),
                empty: Box::new(|typed| {
                    if typed.is_empty() {
                        "Type a word to search every note.".to_owned()
                    } else {
                        format!("No note holds \u{201c}{typed}\u{201d}.")
                    }
                }),
                ..Default::default()
            })
            .show(ui);
        self.search_query = query;
        if out.changed {
            // the address follows the query, with no step in the history for each key
            let typed = self.search_query.trim();
            self.path = if typed.is_empty() {
                vec!["search".into()]
            } else {
                vec!["search".into(), self.search_query.clone()]
            };
        }
        if let Some(chosen) = out.chosen {
            self.search_chosen = chosen;
        }
        if let Some((_, id)) = out.opened {
            self.open_note(&id);
        }
    }
}
