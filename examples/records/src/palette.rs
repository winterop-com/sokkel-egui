//! The search palette, as the web's `App.tsx` builds it: the rail's pages, the forms, the actions,
//! the look, putting the demo's data back, and the responses and households found by what is typed.

use eframe::egui;
use sokkel_egui::{
    AppearanceCommandWords, CommandGroup, CommandPalette, Icon, MenuItem, PaletteWords, RailGroup, appearance_commands,
    place_commands, place_of, run_appearance_command,
};

use crate::app::RecordsApp;
use crate::forms::{form_of, forms};
use crate::settings::CARRIED;

/// How many letters the search waits for, and how many of each kind it lists.
const MIN_QUERY: usize = 2;
const FOUND: usize = 8;

impl RecordsApp {
    /// The palette while it is open, and what its command does.
    pub fn search_palette(&mut self, ctx: &egui::Context, rail: &[RailGroup]) {
        let Some((mut query, mut highlight)) = self.palette.take() else {
            return;
        };
        let mut groups: Vec<CommandGroup> = place_commands(rail, "Pages").into_iter().collect();
        groups.push(CommandGroup::new(
            "Forms",
            forms().iter().map(|form| {
                MenuItem::action(form.title)
                    .icon(Icon::ClipboardList)
                    .hint(form.id)
                    .keywords([form.kind.label()])
                    .id(format!("form/{}", form.id))
            }),
        ));
        groups.push(CommandGroup::new(
            "Actions",
            [
                MenuItem::action("Send on every received response")
                    .icon(Icon::Send)
                    .keywords(["forward"])
                    .id("send-on"),
                MenuItem::action("Check the server again")
                    .icon(Icon::ServerCrash)
                    .id("check-server"),
                MenuItem::action("Settings")
                    .icon(Icon::Settings)
                    .keywords(["preferences", "theme"])
                    .id("settings"),
            ],
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
        let wanted = query.trim().to_lowercase();
        if wanted.chars().count() >= MIN_QUERY {
            let responses = self
                .backend
                .receipts
                .iter()
                .filter(|receipt| receipt.receipt.starts_with(&wanted))
                .take(FOUND)
                .map(|receipt| {
                    let title = form_of(&receipt.form_id).map_or(receipt.form_id.as_str(), |form| form.title);
                    MenuItem::action(title)
                        .icon(Icon::FileText)
                        .hint(&receipt.receipt[..receipt.receipt.len().min(12)])
                        .detail(receipt.state.key())
                        .id(format!("response/{}", receipt.receipt))
                });
            groups.push(CommandGroup::new("Responses", responses));
            let households = self
                .backend
                .households
                .iter()
                .filter(|home| home.identifier.to_lowercase().contains(&wanted))
                .take(FOUND)
                .map(|home| {
                    MenuItem::action(&home.identifier)
                        .icon(Icon::Users)
                        .detail(format!("{} people", home.size))
                        .id(format!("household/{}", home.id))
                });
            groups.push(CommandGroup::new("Households", households));
        }
        let words = PaletteWords {
            placeholder: "Go to a page, open a form, find a response".into(),
            ..Default::default()
        };
        let outcome = CommandPalette::new("palette", &groups, &mut query, &mut highlight)
            .words(words)
            .show(ctx);
        let command = outcome
            .run
            .and_then(|(g, i)| groups[g].items[i].command_id().map(str::to_owned));
        if let Some(command) = command.as_deref() {
            self.run_command(ctx, command);
        }
        if !outcome.close {
            self.palette = Some((query, highlight));
        }
    }

    fn run_command(&mut self, ctx: &egui::Context, command: &str) {
        let (kind, id) = command.split_once('/').unwrap_or((command, ""));
        match kind {
            "form" => self.go(&format!("forms/{id}")),
            "response" => self.go(&format!("responses/{id}")),
            "household" => self.go(&format!("people/{id}")),
            "send-on" => self.send_on(ctx),
            "check-server" => self.check_server(),
            "settings" => self.settings = Some("appearance".into()),
            "demo-reset" => self.resetting = true,
            _ => {
                if let Some(place) = place_of(command) {
                    self.go(if place == "overview" { "" } else { place });
                } else {
                    run_appearance_command(&mut self.appearance, command);
                }
            }
        }
    }
}
