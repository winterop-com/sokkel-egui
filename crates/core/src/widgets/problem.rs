//! Something that went wrong, said for a person, after sokkel-ui's `Problem`.

use egui::{CornerRadius, Margin, Stroke, Ui, vec2};

use crate::{Icon, TextRole, icons, tokens};

/// How bad a problem is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProblemTone {
    #[default]
    Error,
    Warning,
}

/// The words a problem says.
#[derive(Clone, Debug)]
pub struct ProblemWords {
    pub technical: String,
}

impl Default for ProblemWords {
    fn default() -> Self {
        Self {
            technical: "Technical details".into(),
        }
    }
}

/// Something that went wrong, after sokkel-ui's `Problem`: what happened in one sentence at medium
/// weight after its tone's icon, then where and what a right value looks like, muted, its
/// backticked parts in the code face; the raw message a program produced folded away under the
/// technical details. On the tone at 6% with a rule of it at its start, 10 over and under, 12 in.
pub struct Problem {
    id: egui::Id,
    title: String,
    detail: Option<String>,
    technical: Option<String>,
    tone: ProblemTone,
    words: ProblemWords,
}

impl Problem {
    pub fn new(id: impl egui::AsIdSalt, title: impl Into<String>) -> Self {
        Self {
            id: egui::Id::unique(id),
            title: title.into(),
            detail: None,
            technical: None,
            tone: ProblemTone::Error,
            words: ProblemWords::default(),
        }
    }

    /// Where and how to fix it; its backticked parts are drawn as code.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// The raw message, shown as written under the technical details.
    pub fn technical(mut self, technical: impl Into<String>) -> Self {
        self.technical = Some(technical.into());
        self
    }

    pub fn tone(mut self, tone: ProblemTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn words(mut self, words: ProblemWords) -> Self {
        self.words = words;
        self
    }

    /// Draws it, with `actions` in a line under it, 24 in.
    pub fn show_with(self, ui: &mut Ui, actions: Option<&mut dyn FnMut(&mut Ui)>) -> egui::Response {
        let t = tokens(ui.ctx());
        let (key, icon) = match self.tone {
            ProblemTone::Error => ("critical", Icon::CircleAlert),
            ProblemTone::Warning => ("warning", Icon::TriangleAlert),
        };
        let radius = t.radius("control");
        let corner = CornerRadius {
            nw: 0,
            sw: 0,
            ne: radius.ne,
            se: radius.se,
        };
        let frame = egui::Frame::new()
            .fill(t.color_at(key, 0.06))
            .corner_radius(corner)
            .inner_margin(Margin {
                left: 12,
                right: 12,
                top: 10,
                bottom: 10,
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let line = TextRole::Body.line_height(&t);
                    let (slot, _) = ui.allocate_exact_size(vec2(16.0, line), egui::Sense::hover());
                    icons::paint(ui.painter(), slot.center(), icon, 16.0, t.color(key));
                    ui.add(
                        egui::Label::new(
                            TextRole::Body
                                .rich(&t, &self.title)
                                .variation("wght", 500.0)
                                .color(t.color("foreground")),
                        )
                        .wrap(),
                    );
                });
                if let Some(detail) = &self.detail {
                    ui.horizontal_top(|ui| {
                        ui.add_space(24.0);
                        ui.add(egui::Label::new(prose(ui, &t, detail)).wrap());
                    });
                }
                if let Some(technical) = &self.technical {
                    ui.horizontal_top(|ui| {
                        ui.add_space(20.0);
                        ui.vertical(|ui| {
                            super::Unfoldable::new(self.id.with("technical"), &self.words.technical)
                                .quiet(true)
                                .show(ui, |ui| {
                                    egui::Frame::new()
                                        .fill(t.color("muted"))
                                        .corner_radius(t.radius("sm"))
                                        .inner_margin(Margin::symmetric(8, 6))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            let mono = crate::mono_font(
                                                ui.ctx(),
                                                crate::mono_small_size(&t, TextRole::Help.font(&t).size),
                                                400.0,
                                            );
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(technical)
                                                        .font(mono)
                                                        .color(t.color("muted-foreground")),
                                                )
                                                .wrap(),
                                            );
                                        });
                                });
                        });
                    });
                }
                if let Some(actions) = actions {
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.add_space(24.0);
                        ui.spacing_mut().item_spacing.x = 8.0;
                        actions(ui);
                    });
                }
            });
        let rect = frame.response.rect;
        ui.painter()
            .vline(rect.left() + 1.0, rect.y_range(), Stroke::new(2.0, t.color(key)));
        frame.response
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        self.show_with(ui, None)
    }
}

/// Words with their backticked parts in the code face, muted, as the web's `ProseText`.
fn prose(ui: &Ui, t: &crate::Tokens, text: &str) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    for (index, part) in text.split('`').enumerate() {
        if part.is_empty() {
            continue;
        }
        let rich = if index % 2 == 1 {
            egui::RichText::new(part)
                .font(crate::mono_font(
                    ui.ctx(),
                    crate::mono_small_size(t, TextRole::Help.font(t).size),
                    400.0,
                ))
                .color(t.color("muted-foreground"))
        } else {
            TextRole::Body.rich(t, part).color(t.color("muted-foreground"))
        };
        rich.append_to(&mut job, ui.style(), egui::FontSelection::Default, egui::Align::Center);
    }
    job
}
