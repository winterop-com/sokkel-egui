//! The dialogs any page opens for a video, as the web's `dialogs.tsx` draws them: Save to, with
//! Watch later and the viewer's playlists ticked and a field for a new one, and Share, with the
//! video's link and Start at.

use eframe::egui;
use sokkel_egui::{Checkbox, DialogShell, Gap, Icon, IconButton, Input, QuickAdd, Stack};

use crate::act::Act;
use crate::app::VideoApp;
use crate::catalog::video_by_id;
use crate::dialogs::{save_entries, share, validate_name};

/// The site's own address before its `#`, as the web's `location.href` gives it.
pub const SITE: &str = "https://tidevann.example.org/video/";

impl VideoApp {
    /// Save to and Share while one is open.
    pub fn dialogs(&mut self, ctx: &egui::Context) {
        let mut acts = Vec::new();
        if let Some(id) = self.saving.clone()
            && let Some(video) = video_by_id(&id)
        {
            let entries = save_entries(&self.data, video);
            let data = &self.data;
            let name = &mut self.views.save_name;
            let answer = DialogShell::new("save-to", "Save to", "Done")
                .description(video.title)
                .width(384.0)
                .show(ctx, |ui| {
                    Stack::new(Gap::Sm).show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            for entry in &entries {
                                let mut on = entry.checked;
                                let label = format!("{}  {}", entry.label, entry.description);
                                if ui.add(Checkbox::new(&mut on, label).field()).changed() {
                                    acts.push(Act::SaveToggle(entry.id.clone(), id.clone(), on));
                                }
                            }
                        });
                        let made = QuickAdd::new("save-new", name)
                            .placeholder("New playlist")
                            .validate(|typed| validate_name(data, typed).map(str::to_owned))
                            .show(ui);
                        if let Some(made) = made {
                            acts.push(Act::SaveNew(made, id.clone()));
                        }
                    });
                });
            if answer.is_some() {
                self.saving = None;
            }
        }
        if let Some((id, at, from_here)) = self.sharing.clone()
            && let Some(video) = video_by_id(&id)
        {
            let shared = share(SITE, video, at, from_here);
            let mut link = shared.link.clone();
            let mut start = from_here;
            let mut copied = false;
            let answer = DialogShell::new("share", "Share", "Copy link")
                .description(video.title)
                .width(384.0)
                .show(ctx, |ui| {
                    Stack::new(Gap::Sm).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            let width = ui.available_width() - 38.0;
                            ui.add(Input::new(&mut link).width(width));
                            copied = ui.add(IconButton::new(Icon::Copy, "Copy").md().outline()).clicked();
                        });
                        if let Some(words) = &shared.start_at {
                            ui.add(Checkbox::new(&mut start, words).field());
                        }
                    });
                });
            if start != from_here {
                self.sharing = Some((id.clone(), at, start));
            }
            if copied || answer == Some(true) {
                ctx.copy_text(shared.link.clone());
                acts.push(Act::Toast("Link copied".into()));
            }
            if answer.is_some() {
                self.sharing = None;
            }
        }
        for act in acts {
            self.apply(ctx, act);
        }
    }
}
