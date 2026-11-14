//! A page's head and the states a page is in before its content: loading, refused, missing, not
//! answering, and empty, after sokkel-ui's `PageHeader` and `PageState`.

use egui::{Align, Frame, Layout, Margin, Stroke, Ui, vec2};

use super::spinner;
use crate::{Button, ButtonSize, ButtonVariant, Icon, TextRole, icons, tokens};

/// The content's own padding for a page to sit in, after sokkel-ui's `Page`: the density's page
/// inset across (the wide one where the room is 640 or wider, the phone-sized one below it), the
/// page top over it and 40 under it, its parts the page gap apart.
pub struct Page;

impl Page {
    pub fn show<R>(ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> R {
        let t = tokens(ui.ctx());
        // the room the page's scroller gives, its thin track included, as the web's `100cqi`
        let room = ui.available_width() + super::frame::SCROLL_GUTTER;
        let inset = if room >= 640.0 {
            t.density("page-inset-wide")
        } else {
            t.density("page-inset-narrow")
        };
        Frame::new()
            .inner_margin(Margin {
                left: inset as i8,
                right: inset as i8,
                top: t.density("page-top") as i8,
                bottom: 40,
            })
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = t.density("page-gap");
                content(ui)
            })
            .inner
    }
}

/// A page's head: its title (with a count, and a code under it), a description, and the page's
/// actions at the end of the row.
pub struct PageHeader {
    title: String,
    count: Option<String>,
    code: Option<String>,
    description: Option<String>,
}

impl PageHeader {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            count: None,
            code: None,
            description: None,
        }
    }

    /// How many the page lists, after its title in figures.
    pub fn count(mut self, count: impl ToString) -> Self {
        self.count = Some(count.to_string());
        self
    }

    /// The thing's code or id, in mono under the title.
    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Draw the head, with `actions` laid out from the row's end.
    pub fn show(self, ui: &mut Ui, actions: impl FnOnce(&mut Ui)) {
        let t = tokens(ui.ctx());
        let ink = t.color_or("page-title-ink", t.color("foreground"));
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                // the code right under the title, the description 4 under what is over it
                ui.spacing_mut().item_spacing.y = 0.0;
                // the title, and the count 8 after it standing on the title's baseline
                let title = egui::WidgetText::from(TextRole::PageTitle.rich(&t, &self.title)).into_galley(
                    ui,
                    Some(egui::TextWrapMode::Wrap),
                    ui.available_width(),
                    TextRole::PageTitle.font(&t),
                );
                let count = self.count.as_ref().map(|count| {
                    egui::WidgetText::from(TextRole::Help.rich(&t, count)).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Extend),
                        f32::INFINITY,
                        TextRole::Help.font(&t),
                    )
                });
                let width = title.size().x + count.as_ref().map_or(0.0, |c| 8.0 + c.size().x);
                let (row, _) = ui.allocate_exact_size(egui::vec2(width, title.size().y), egui::Sense::hover());
                let baseline = |galley: &egui::Galley| {
                    galley
                        .rows
                        .first()
                        .and_then(|r| r.glyphs.first().map(|g| r.pos.y + g.pos.y))
                        .unwrap_or(galley.size().y)
                };
                let line = baseline(&title);
                let title_width = title.size().x;
                if let Some(count) = count {
                    let at = egui::pos2(row.left() + title_width + 8.0, row.top() + line - baseline(&count));
                    ui.painter().galley(at, count, t.color("muted-foreground"));
                }
                ui.painter().galley(row.min, title, ink);
                if let Some(code) = &self.code {
                    let mono = egui::FontId::monospace(TextRole::Help.font(&t).size);
                    ui.label(egui::RichText::new(code).font(mono).color(t.color("muted-foreground")));
                }
                if let Some(description) = &self.description {
                    ui.add_space(4.0);
                    ui.set_max_width(640.0);
                    ui.label(TextRole::Body.rich(&t, description).color(t.color("muted-foreground")));
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                ui.add_space(0.0);
                actions(ui);
            });
        });
    }
}

/// Whether a failed read stops the page (`Critical`) or is about a part the page stands without.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PageStateTone {
    #[default]
    Critical,
    Warning,
}

/// The words a page state says, English by default.
#[derive(Clone, Debug)]
pub struct PageStateWords {
    pub loading: String,
    pub refused: String,
    pub missing: String,
    pub unreachable: String,
    pub part_refused: String,
    pub retry: String,
}

impl Default for PageStateWords {
    fn default() -> Self {
        Self {
            loading: "Loading".into(),
            refused: "The server refused this read".into(),
            missing: "Nothing is there".into(),
            unreachable: "The server is not answering".into(),
            part_refused: "Part of this page could not be read".into(),
            retry: "Try again".into(),
        }
    }
}

#[derive(Clone, Debug)]
enum Kind {
    Loading,
    Refused,
    Missing,
    Unreachable,
    Empty(String),
}

/// What a page shows in place of its content: loading, a failed read (refused, missing, or no
/// answer) with its detail, its problems and a retry, or empty. In a framed box by default.
pub struct PageState {
    kind: Kind,
    detail: Option<String>,
    problems: Vec<String>,
    tone: PageStateTone,
    framed: bool,
    retry: bool,
    words: PageStateWords,
}

impl PageState {
    fn of(kind: Kind) -> Self {
        Self {
            kind,
            detail: None,
            problems: Vec::new(),
            tone: PageStateTone::Critical,
            framed: true,
            retry: false,
            words: PageStateWords::default(),
        }
    }

    pub fn loading() -> Self {
        Self::of(Kind::Loading)
    }

    /// The server refused the read; `detail` says what it said.
    pub fn refused(detail: impl Into<String>) -> Self {
        Self::of(Kind::Refused).detail(detail)
    }

    /// The thing is not there (a 404): an absence, not a refusal.
    pub fn missing(detail: impl Into<String>) -> Self {
        Self::of(Kind::Missing).detail(detail)
    }

    /// No answer came.
    pub fn unreachable(detail: impl Into<String>) -> Self {
        Self::of(Kind::Unreachable).detail(detail)
    }

    pub fn empty(message: impl Into<String>) -> Self {
        Self::of(Kind::Empty(message.into()))
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// The problems a refusal listed, one line each.
    pub fn problems(mut self, problems: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.problems = problems.into_iter().map(Into::into).collect();
        self
    }

    pub fn tone(mut self, tone: PageStateTone) -> Self {
        self.tone = tone;
        self
    }

    /// No box of its own, for a state inside a panel.
    pub fn framed(mut self, framed: bool) -> Self {
        self.framed = framed;
        self
    }

    /// Offer Try again under a failed read.
    pub fn retry(mut self, retry: bool) -> Self {
        self.retry = retry;
        self
    }

    pub fn words(mut self, words: PageStateWords) -> Self {
        self.words = words;
        self
    }

    /// Draw the state; true when Try again was pressed.
    pub fn show(self, ui: &mut Ui) -> bool {
        let t = tokens(ui.ctx());
        let warning = self.tone == PageStateTone::Warning;
        let failed = matches!(self.kind, Kind::Refused | Kind::Missing | Kind::Unreachable);
        let (ground, edge) = match (self.framed, failed && warning) {
            (false, _) => (egui::Color32::TRANSPARENT, egui::Color32::TRANSPARENT),
            (true, true) => (t.color_at("warning", 0.08), t.color_at("warning", 0.35)),
            (true, false) => (t.color("card"), t.color("border-strong")),
        };
        let margin = if self.framed {
            Margin::symmetric(16, 24)
        } else {
            let pad = t.density("panel-pad") as i8;
            Margin::symmetric(pad, 12)
        };
        let mut pressed = false;
        Frame::new()
            .fill(ground)
            .stroke(Stroke::new(1.0, edge))
            .corner_radius(t.radius("surface"))
            .inner_margin(margin)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing = vec2(12.0, 4.0);
                    let muted = t.color("muted-foreground");
                    match &self.kind {
                        Kind::Loading => {
                            let (rect, _) = ui.allocate_exact_size(vec2(16.0, 20.0), egui::Sense::hover());
                            spinner(ui, ui.painter(), rect.center(), 6.0, muted);
                            ui.label(TextRole::Body.rich(&t, &self.words.loading).color(muted));
                        }
                        Kind::Empty(message) => {
                            ui.label(TextRole::Body.rich(&t, message).color(muted));
                        }
                        kind => {
                            let (icon, ink) = if warning {
                                (Icon::TriangleAlert, t.color("warning-ink"))
                            } else {
                                (Icon::CircleAlert, t.color("destructive"))
                            };
                            let (rect, _) = ui.allocate_exact_size(vec2(16.0, 22.0), egui::Sense::hover());
                            icons::paint(ui.painter(), rect.center(), icon, 16.0, ink);
                            ui.vertical(|ui| {
                                let heading = match kind {
                                    Kind::Unreachable => &self.words.unreachable,
                                    Kind::Missing => &self.words.missing,
                                    _ if warning => &self.words.part_refused,
                                    _ => &self.words.refused,
                                };
                                ui.label(TextRole::Action.rich(&t, heading).color(t.color("foreground")));
                                if let Some(detail) = &self.detail {
                                    ui.label(TextRole::Help.rich(&t, detail).color(muted));
                                }
                                for problem in &self.problems {
                                    ui.label(TextRole::Help.rich(&t, format!("•  {problem}")).color(muted));
                                }
                                if self.retry {
                                    ui.add_space(8.0);
                                    let button = Button::new(self.words.retry.clone())
                                        .variant(ButtonVariant::Outline)
                                        .size(ButtonSize::Sm);
                                    pressed = ui.add(button).clicked();
                                }
                            });
                        }
                    }
                });
            });
        pressed
    }
}
