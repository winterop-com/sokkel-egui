//! Beside a note, as the web's `NotePanel.tsx`: what links to it, where it is named without a
//! link, what it links to, its tags and its outline.

use eframe::egui::Ui;
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, ContentsEntry, Icon, Item, ItemSize, Markdown, SidePanel, TableOfContents,
    TableOfContentsWords, TextRole, item_group, panel_section, tokens,
};

use crate::model::Note;
use crate::vault::{Vault, markdown_headings, unlinked_mentions};

/// What the panel asked this frame.
#[derive(Clone, Debug, Default)]
pub struct PanelAsked {
    /// A note to open, by its id.
    pub open: Option<String>,
    /// A name linked to that has no note yet, to make.
    pub make: Option<String>,
    /// A tag to show.
    pub tag: Option<String>,
    /// The first place another note names this one, to turn into a link: that note's id, and
    /// the range of its text.
    pub link: Option<(String, std::ops::Range<usize>)>,
    /// A heading of the outline to go to, by its id.
    pub heading: Option<String>,
}

/// A muted sentence where a section holds nothing.
fn none(ui: &mut Ui, words: &str) {
    let t = tokens(ui.ctx());
    let galley = eframe::egui::WidgetText::from(TextRole::Body.rich(&t, words)).into_galley(
        ui,
        Some(eframe::egui::TextWrapMode::Wrap),
        ui.available_width(),
        TextRole::Body.font(&t),
    );
    let (rect, _) = ui.allocate_exact_size(galley.size(), eframe::egui::Sense::hover());
    ui.painter().galley(rect.min, galley, t.color("muted-foreground"));
}

/// The panel beside `note`, its text `body` (a draft, while one is kept); `tab` is the tab shown.
pub fn note_panel(ui: &mut Ui, vault: &Vault, note: &Note, body: &str, tab: &mut usize) -> PanelAsked {
    let mut asked = PanelAsked::default();
    let backlinks = vault.backlinks(&note.id);
    let mentions = unlinked_mentions(vault, note);
    let mut targets: Vec<String> = Vec::new();
    for link in vault.links(&note.id) {
        if !targets.iter().any(|t| t.to_lowercase() == link.target.to_lowercase()) {
            targets.push(link.target.clone());
        }
    }
    let tags = vault.tags_of(&note.id);
    let headings: Vec<ContentsEntry> = markdown_headings(body, &[1, 2, 3, 4])
        .into_iter()
        .map(|h| ContentsEntry {
            id: h.id,
            label: h.label,
            level: h.level,
        })
        .collect();
    let icon_of = |n: &Note| {
        if n.day.is_some() {
            Icon::CalendarDays
        } else {
            Icon::FileText
        }
    };

    SidePanel::new("note-panel", &["Links", "Outline"], tab).show(ui, |tab, ui| {
        if tab == 1 {
            eframe::egui::Frame::new()
                .inner_margin(eframe::egui::Margin::same(16))
                .show(ui, |ui| {
                    if headings.is_empty() {
                        none(ui, "No headings. Start a line with # to make one.");
                    } else {
                        asked.heading = TableOfContents::new(&headings)
                            .words(TableOfContentsWords {
                                title: note.title.clone(),
                            })
                            .show(ui);
                    }
                });
            return;
        }
        panel_section(ui, &format!("Linked mentions {}", backlinks.len()), |ui| {
            if backlinks.is_empty() {
                none(ui, "No note links here yet.");
                return;
            }
            item_group(ui, |ui| {
                for backlink in backlinks {
                    let Some(from) = vault.note(&backlink.from) else {
                        continue;
                    };
                    let row = Item::new(&from.title)
                        .icon(icon_of(from))
                        .description_with(|ui| {
                            Markdown::new(&backlink.line)
                                .inline(TextRole::Help, "muted-foreground")
                                .live(false)
                                .highlight(backlink.link.label.as_deref().unwrap_or(&backlink.link.target))
                                .show(ui);
                        })
                        .size(ItemSize::Xs)
                        .pressable(true)
                        .show(ui, |_| {});
                    if row.clicked() {
                        asked.open = Some(from.id.clone());
                    }
                }
            });
        });
        panel_section(ui, &format!("Unlinked mentions {}", mentions.len()), |ui| {
            if mentions.is_empty() {
                none(ui, "No note names it without a link.");
                return;
            }
            item_group(ui, |ui| {
                for mention in &mentions {
                    let Some(from) = vault.note(&mention.from) else {
                        continue;
                    };
                    let mut link = false;
                    let row = Item::new(&from.title)
                        .icon(icon_of(from))
                        .description_with(|ui| {
                            Markdown::new(&mention.line)
                                .inline(TextRole::Help, "muted-foreground")
                                .live(false)
                                .highlight(&note.title)
                                .show(ui);
                        })
                        .size(ItemSize::Xs)
                        .pressable(true)
                        .show(ui, |ui| {
                            link = ui
                                .add(
                                    Button::new("Link")
                                        .icon(Icon::Link2)
                                        .size(ButtonSize::Xs)
                                        .variant(ButtonVariant::Outline),
                                )
                                .clicked();
                        });
                    if link {
                        asked.link = Some((from.id.clone(), mention.at.clone()));
                    } else if row.clicked() {
                        asked.open = Some(from.id.clone());
                    }
                }
            });
        });
        panel_section(ui, &format!("Links out {}", targets.len()), |ui| {
            if targets.is_empty() {
                none(ui, "This note links nowhere. Type [[ to link one.");
                return;
            }
            item_group(ui, |ui| {
                for target in &targets {
                    match vault.resolve(target) {
                        None => {
                            let mut make = false;
                            Item::new(target)
                                .icon(Icon::FileX)
                                .description("Not made yet")
                                .size(ItemSize::Xs)
                                .show(ui, |ui| {
                                    make = ui
                                        .add(
                                            Button::new("Make")
                                                .icon(Icon::FilePlus)
                                                .size(ButtonSize::Xs)
                                                .variant(ButtonVariant::Outline),
                                        )
                                        .clicked();
                                });
                            if make {
                                asked.make = Some(target.clone());
                            }
                        }
                        Some(to) => {
                            let mut item = Item::new(&to.title)
                                .icon(icon_of(to))
                                .size(ItemSize::Xs)
                                .pressable(true);
                            if to.folder.is_some() {
                                item = item.description(vault.path(to.folder.as_deref()));
                            }
                            if item.show(ui, |_| {}).clicked() {
                                asked.open = Some(to.id.clone());
                            }
                        }
                    }
                }
            });
        });
        panel_section(ui, "Tags", |ui| {
            if tags.is_empty() {
                none(ui, "No tags. Write # and a word to tag the note.");
                return;
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = eframe::egui::vec2(4.0, 4.0);
                for tag in tags {
                    let pressed = ui
                        .add(
                            Button::new(format!("#{tag}"))
                                .size(ButtonSize::Xs)
                                .variant(ButtonVariant::Outline),
                        )
                        .clicked();
                    if pressed {
                        asked.tag = Some(tag.clone());
                    }
                }
            });
        });
    });
    asked
}
