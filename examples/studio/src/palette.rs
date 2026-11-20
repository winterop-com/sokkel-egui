//! The search palette and the file switcher, as the web's `Studio.tsx` builds them: the studio's
//! commands, the rail's places, every file and table, the look and the demo's data; and a palette of
//! the files alone, Mod-P.

use eframe::egui;
use sokkel_egui::{
    AppearanceCommandWords, CommandGroup, CommandPalette, Icon, MenuItem, PaletteWords, RailGroup, appearance_commands,
    place_commands, place_of, run_appearance_command,
};

use crate::app::{StudioApp, kind_icon};
use crate::backend::Asking;
use crate::js::locale_compare;
use crate::model::{FileKind, KINDS, Project, WorkFile, counted};
use crate::settings::CARRIED;
use crate::tables::tables_with;

/// A group of files to open: each its name, its project as a hint, found by its project and kind.
fn file_commands(title: &str, files: &[WorkFile]) -> CommandGroup {
    CommandGroup::new(
        title,
        files.iter().map(|one| {
            MenuItem::action(&one.name)
                .icon(kind_icon(one.kind))
                .hint(one.project.name())
                .keywords([one.project.name(), one.kind.name()])
                .id(format!("open/{}", one.id))
        }),
    )
}

impl StudioApp {
    /// The files by name, in reading order.
    fn by_name(&self) -> Vec<WorkFile> {
        let mut files = self.backend.files.clone();
        files.sort_by(|a, b| locale_compare(&a.name, &b.name, false));
        files
    }

    /// The palette while it is open, and what its command does.
    pub fn search_palette(&mut self, ctx: &egui::Context, rail: &[RailGroup], project: Project, kind: FileKind) {
        let Some((mut query, mut highlight)) = self.palette.take() else {
            return;
        };
        let dirty = !self.backend.dirty().is_empty();
        let mut studio = vec![
            MenuItem::action("Open a file")
                .icon(Icon::FolderOpen)
                .shortcut("⌘P")
                .id("open-file"),
        ];
        for k in KINDS {
            studio.push(
                MenuItem::action(format!("New {}", k.label.to_lowercase()))
                    .icon(Icon::FilePlus)
                    .keywords(["make", "create", k.extension])
                    .id(format!("new/{}", k.kind.name())),
            );
        }
        studio.push(
            MenuItem::action("Save all")
                .icon(Icon::Save)
                .shortcut("⇧⌘S")
                .disabled(!dirty)
                .id("save-all"),
        );
        let mut groups = vec![CommandGroup::new("Studio", studio)];
        groups.extend(place_commands(rail, "Go to"));
        groups.push(file_commands("Files", &self.by_name()));
        let tables = tables_with(&self.backend.runs);
        groups.push(CommandGroup::new(
            "Tables",
            tables.iter().map(|table| {
                MenuItem::action(&table.name)
                    .icon(Icon::Sheet)
                    .hint(counted(table.rows.len() as f64, "row", None))
                    .keywords(["table", "sql"])
                    .id(format!("open/table:{}", table.name))
            }),
        ));
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
            let now = self.now();
            match command {
                "open-file" => self.switching = Some(Default::default()),
                "save-all" => self.backend.save_all(now),
                "demo-reset" => self.resetting = true,
                _ => {
                    if let Some(id) = command.strip_prefix("open/") {
                        self.open_tab(id);
                    } else if let Some(name) = command.strip_prefix("new/") {
                        let kind = KINDS.iter().find(|k| k.kind.name() == name).map_or(kind, |k| k.kind);
                        self.backend.asking = Some(Asking::New { project, kind });
                    } else if let Some(place) = place_of(command) {
                        match place {
                            "workspace" => self.go(""),
                            other => self.go(other),
                        }
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

    /// The file switcher while it is open: the open files, then every file by name.
    pub fn file_switcher(&mut self, ctx: &egui::Context) {
        let Some((mut query, mut highlight)) = self.switching.take() else {
            return;
        };
        let open: Vec<WorkFile> = self
            .backend
            .prefs
            .tabs
            .iter()
            .filter_map(|id| self.backend.file(id).cloned())
            .collect();
        let mut groups = Vec::new();
        if !open.is_empty() {
            groups.push(file_commands("Open", &open));
        }
        groups.push(file_commands("Files", &self.by_name()));
        let outcome = CommandPalette::new("switcher", &groups, &mut query, &mut highlight)
            .words(PaletteWords {
                placeholder: "Open a file by name".into(),
                no_match: "No file is named that.".into(),
                ..Default::default()
            })
            .show(ctx);
        let command = outcome
            .run
            .and_then(|(g, i)| groups[g].items[i].command_id().map(str::to_owned));
        if let Some(id) = command.as_deref().and_then(|c| c.strip_prefix("open/")) {
            self.open_tab(id);
        } else if !outcome.close {
            self.switching = Some((query, highlight));
        }
    }
}
