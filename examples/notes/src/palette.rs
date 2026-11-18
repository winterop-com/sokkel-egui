//! The search palette and the note switcher, as the web's `Notes.tsx` builds them: the app's own
//! commands, the rail's places, the recent notes and every note by name, the look and the demo's
//! data; and a palette of the notes alone, Mod-O, that makes a note it does not find.

use eframe::egui;
use sokkel_egui::{
    AppearanceCommandWords, CommandGroup, CommandPalette, Icon, MenuItem, PaletteWords, RailGroup, appearance_commands,
    place_commands, place_of, run_appearance_command,
};

use crate::app::NotesApp;
use crate::backend::{Asking, NewNote};
use crate::model::{day, locale_compare};
use crate::settings::CARRIED;

impl NotesApp {
    /// A group of notes to open, by their ids: each its name, its folder as a hint, its tags as
    /// words it is found by.
    fn switcher(&self, title: &str, ids: &[String]) -> CommandGroup {
        let vault = self.backend.vault();
        CommandGroup::new(
            title,
            ids.iter().filter_map(|id| vault.note(id)).map(|one| {
                let mut item = MenuItem::action(&one.title)
                    .icon(if one.day.is_some() {
                        Icon::CalendarDays
                    } else {
                        Icon::FileText
                    })
                    .keywords(vault.tags_of(&one.id).iter().map(String::as_str))
                    .id(format!("open/{}", one.id));
                let place = vault.path(one.folder.as_deref());
                if !place.is_empty() {
                    item = item.hint(place);
                }
                item
            }),
        )
    }

    /// The notes by name, in reading order.
    fn by_title(&self) -> Vec<String> {
        let mut notes: Vec<_> = self.backend.vault().notes.iter().collect();
        notes.sort_by(|a, b| locale_compare(&a.title, &b.title, false));
        notes.into_iter().map(|n| n.id.clone()).collect()
    }

    /// The folder of the note shown, where a new note or folder goes.
    fn here(&self) -> Option<String> {
        let id = (self.path.first().map(String::as_str) == Some("note"))
            .then(|| self.path.get(1))
            .flatten()?;
        self.backend.vault().note(id)?.folder.clone()
    }

    /// The palette while it is open, and what its command does.
    pub fn search_palette(&mut self, ctx: &egui::Context, rail: &[RailGroup]) {
        let Some((mut query, mut highlight)) = self.palette.take() else {
            return;
        };
        let panel = self.backend.prefs.panel;
        let mut groups = vec![CommandGroup::new(
            "Notes",
            [
                MenuItem::action("New note").icon(Icon::FilePlus).id("new-note"),
                MenuItem::action("New folder").icon(Icon::FolderPlus).id("new-folder"),
                MenuItem::action("Open today's note")
                    .icon(Icon::CalendarDays)
                    .keywords(["daily", "journal"])
                    .id("today"),
                MenuItem::action("Open a note by name")
                    .icon(Icon::FileText)
                    .shortcut("⌘O")
                    .id("switch"),
                MenuItem::action(if panel {
                    "Hide links and outline"
                } else {
                    "Show links and outline"
                })
                .icon(Icon::PanelRight)
                .keywords(["backlinks", "outline", "panel"])
                .id("panel"),
            ],
        )];
        groups.extend(place_commands(rail, "Go to"));
        let recent = self.backend.prefs.recent.clone();
        groups.push(self.switcher("Recent", &recent));
        groups.push(self.switcher("All notes", &self.by_title()));
        let themes: Vec<_> = sokkel_egui::themes()
            .filter(|theme| CARRIED.contains(&theme.name.as_str()))
            .collect();
        groups.extend(appearance_commands(
            &self.appearance,
            &themes,
            &AppearanceCommandWords::default(),
        ));
        groups.push(CommandGroup::new(
            "Demo",
            [MenuItem::action("Reset demo data")
                .icon(Icon::RotateCcw)
                .id("demo-reset")],
        ));
        let outcome = CommandPalette::new("palette", &groups, &mut query, &mut highlight).show(ctx);
        let command = outcome
            .run
            .and_then(|(g, i)| groups[g].items[i].command_id().map(str::to_owned));
        if let Some(command) = command.as_deref() {
            let here = self.here();
            match command {
                "new-note" => self.new_note_in(here.as_deref()),
                "new-folder" => self.ask(Asking::NewFolder { parent: here }),
                "today" => {
                    let today = day(0, self.now());
                    self.open_day(ctx, &today);
                }
                "switch" => self.switching = Some(Default::default()),
                "panel" => self.backend.set_prefs(|p| p.panel = !p.panel),
                "demo-reset" => self.resetting = true,
                _ => {
                    if let Some(id) = command.strip_prefix("open/") {
                        self.open_note(id);
                    } else if let Some(place) = place_of(command) {
                        self.go(place);
                    } else {
                        run_appearance_command(&mut self.appearance, command);
                    }
                }
            }
        }
        if !outcome.close {
            self.palette = Some((query, highlight));
        }
    }

    /// The note switcher while it is open: the recent notes and every note by name, and making a
    /// note by the name typed when none has it.
    pub fn note_switcher(&mut self, ctx: &egui::Context) {
        let Some((mut query, mut highlight)) = self.switching.take() else {
            return;
        };
        let recent = self.backend.prefs.recent.clone();
        let mut groups = vec![
            self.switcher("Recent", &recent),
            self.switcher("Notes", &self.by_title()),
        ];
        let typed = query.trim().to_owned();
        if !typed.is_empty() && self.backend.vault().resolve(&typed).is_none() {
            groups.push(CommandGroup::new(
                "Make",
                [MenuItem::action(format!("Make {typed}"))
                    .icon(Icon::FilePlus)
                    .detail("A new note by that name")
                    .id("make")],
            ));
        }
        let outcome = CommandPalette::new("switcher", &groups, &mut query, &mut highlight)
            .words(PaletteWords {
                placeholder: "Open a note by name".into(),
                no_match: "No note is named that.".into(),
                ..Default::default()
            })
            .show(ctx);
        let command = outcome
            .run
            .and_then(|(g, i)| groups[g].items[i].command_id().map(str::to_owned));
        match command.as_deref() {
            Some("make") => {
                let now = self.now();
                let folder = self.here();
                let note = self.backend.create_note(
                    NewNote {
                        title: Some(typed),
                        folder,
                        body: None,
                        day: None,
                    },
                    now,
                );
                self.open_note(&note.id);
            }
            Some(command) => {
                if let Some(id) = command.strip_prefix("open/") {
                    self.open_note(id);
                }
            }
            None => {}
        }
        if !outcome.close && command.is_none() {
            self.switching = Some((query, highlight));
        }
    }
}
