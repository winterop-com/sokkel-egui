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
    /// The page in a scroller of its own, as the shell's `main` scrolls a page taller than the
    /// window. A page that fills the window with a part that scrolls itself (a table) uses
    /// [`Page::show`] instead, so that part can take the room left.
    pub fn scroll<R>(ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> R {
        egui::ScrollArea::vertical()
            .id_salt("sokkel-page")
            .auto_shrink([false, false])
            .show(ui, |ui| Self::show(ui, content))
            .inner
    }

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
    back: Option<String>,
    chips: Vec<(String, crate::ChipTone)>,
    meta: Vec<String>,
    meta_row: bool,
    meta_line: Option<crate::MetaLine>,
    edit: Option<crate::InlineEdit>,
    edit_description: Option<crate::InlineEdit>,
}

/// What a page header was asked this frame.
#[derive(Clone, Debug, Default)]
pub struct PageHeaderResponse {
    /// Its back button was pressed.
    pub back: bool,
    /// The title saved in place, with [`PageHeader::edit_title`].
    pub renamed: Option<String>,
    /// The description saved in place, with [`PageHeader::edit_description`].
    pub described: Option<String>,
    /// A link in the [`PageHeader::meta_line`] was pressed: its index among the line's items.
    pub meta_link: Option<usize>,
}

impl PageHeader {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            count: None,
            code: None,
            description: None,
            back: None,
            chips: Vec::new(),
            meta: Vec::new(),
            meta_row: false,
            meta_line: None,
            edit: None,
            edit_description: None,
        }
    }

    /// Small facts under the title and the description, 8 down, as the web's `meta` holding a
    /// `MetaLine`: its items at the text size with `sm`, a link among them answered as
    /// [`PageHeaderResponse::meta_link`].
    pub fn meta_line(mut self, line: crate::MetaLine) -> Self {
        self.meta_line = Some(line);
        self
    }

    /// The title edited where it stands, as sokkel-ui's `InlineEdit` with `variant="title"` in the
    /// header's `title`: `label` names its editor, and `validate` refuses a title with a message.
    /// The title saved comes back as [`PageHeaderResponse::renamed`].
    pub fn edit_title(mut self, label: impl Into<String>, validate: impl Fn(&str) -> Option<String> + 'static) -> Self {
        self.edit = Some(
            crate::InlineEdit::new(("page-title", &self.title), label, self.title.clone())
                .title()
                .validate(validate),
        );
        self
    }

    /// The description edited where it stands, as sokkel-ui's `InlineEdit` with
    /// `editor="multiline"` in the header's `description`: `label` names its editor, and
    /// `placeholder` stands for an empty one. The description saved comes back as
    /// [`PageHeaderResponse::described`].
    pub fn edit_description(mut self, label: impl Into<String>, placeholder: impl Into<String>) -> Self {
        let value = self.description.take().unwrap_or_default();
        self.edit_description = Some(
            crate::InlineEdit::new(("page-description", &self.title), label, value)
                .multiline(true)
                .placeholder(placeholder),
        );
        self
    }

    /// A back button before the title, named `label` ("Back"): the way to the page this one is in.
    pub fn back(mut self, label: impl Into<String>) -> Self {
        self.back = Some(label.into());
        self
    }

    /// A chip under the description, 8 down: the thing's state (sokkel-ui's `meta` with a
    /// `StatusChip`); each call adds one more on the line.
    pub fn chip(mut self, label: impl Into<String>, tone: crate::ChipTone) -> Self {
        self.chips.push((label.into(), tone));
        self
    }

    /// Words after the chips on their line, muted, such as how long the thing has been open; each
    /// call adds more, 6 after the last.
    pub fn meta(mut self, meta: impl Into<String>) -> Self {
        self.meta.push(meta.into());
        self
    }

    /// The chips and the meta as one tight row, 6 apart, as an app's `Row gap="xs"` in the
    /// web's `meta`; 12 apart, the header's own gap, otherwise.
    pub fn meta_row(mut self, row: bool) -> Self {
        self.meta_row = row;
        self
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
    pub fn show(mut self, ui: &mut Ui, actions: impl FnOnce(&mut Ui)) -> PageHeaderResponse {
        let t = tokens(ui.ctx());
        let ink = t.color_or("page-title-ink", t.color("foreground"));
        let mut answer = PageHeaderResponse::default();
        ui.horizontal_top(|ui| {
            // the back button on the title's line, the title 8 after it
            if let Some(label) = &self.back {
                ui.spacing_mut().item_spacing.x = 8.0;
                // beside an editable title, centered on its words below the 2 it is padded down
                let line = TextRole::PageTitle.font(&t).size * 1.25 + if self.edit.is_some() { 4.0 } else { 0.0 };
                let (slot, _) = ui.allocate_exact_size(egui::vec2(28.0, line), egui::Sense::hover());
                let button = egui::Rect::from_center_size(slot.center(), egui::vec2(28.0, 28.0));
                let hit = ui.interact(button, ui.scope_id().with("page-back"), egui::Sense::click());
                hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
                if hit.hovered() {
                    ui.painter().rect_filled(button, t.radius("control"), t.color("hover"));
                }
                icons::paint(
                    ui.painter(),
                    button.center(),
                    Icon::ChevronLeft,
                    16.0,
                    t.color("muted-foreground"),
                );
                super::focus_ring(ui.painter(), &t, button, t.radius("control"), &hit);
                answer.back = hit.clicked();
            }
            let block = ui.vertical(|ui| {
                // the code right under the title, the description 4 under what is over it
                ui.spacing_mut().item_spacing.y = 0.0;
                // an editable title stands as its own inline edit
                let edit = self.edit.take();
                if let Some(edit) = edit {
                    // the count 8 after the title's editor, standing on its line
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        answer.renamed = edit.show(ui);
                        if let Some(count) = &self.count {
                            // on the title's baseline: its line's foot less the count's
                            let title = t.text(TextRole::PageTitle.key()).line_height;
                            let help = t.text(TextRole::Help.key()).line_height;
                            ui.vertical(|ui| {
                                ui.add_space((2.0 + title - help - 2.0).max(0.0));
                                ui.label(TextRole::Help.rich(&t, count).color(t.color("muted-foreground")));
                            });
                        }
                    });
                } else {
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
                }
                if let Some(code) = &self.code {
                    let mono = egui::FontId::monospace(TextRole::Help.font(&t).size);
                    ui.label(egui::RichText::new(code).font(mono).color(t.color("muted-foreground")));
                }
                if let Some(edit) = self.edit_description.take() {
                    ui.add_space(4.0);
                    let zero = ui.fonts_mut(|f| f.glyph_width(&TextRole::Body.font(&t), '0'));
                    ui.set_max_width(65.0 * zero);
                    answer.described = edit.show(ui);
                }
                if let Some(description) = &self.description {
                    ui.add_space(4.0);
                    // `max-w-prose`: 65 widths of "0" in the body's type
                    let zero = ui.fonts_mut(|f| f.glyph_width(&TextRole::Body.font(&t), '0'));
                    ui.set_max_width(65.0 * zero);
                    ui.label(TextRole::Body.rich(&t, description).color(t.color("muted-foreground")));
                }
                if !self.chips.is_empty() || !self.meta.is_empty() {
                    // the chips and the meta on one line, 8 down: 12 apart as the header's own
                    // `gap-x-3`, or 6 where the app gives them as one `Row gap="xs"`
                    ui.add_space(8.0);
                    // the line as tall as its chips, not a control's height: set before the row is made
                    let kept = ui.spacing().interact_size.y;
                    ui.spacing_mut().interact_size.y = 0.0;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().interact_size.y = kept;
                        ui.spacing_mut().item_spacing.x = if self.meta_row { 6.0 } else { 12.0 };
                        for (label, tone) in &self.chips {
                            ui.add(crate::Chip::new(label.clone()).tone(*tone));
                        }
                        for meta in &self.meta {
                            ui.label(TextRole::Help.rich(&t, meta).color(t.color("muted-foreground")));
                        }
                    });
                    ui.spacing_mut().interact_size.y = kept;
                }
                if let Some(line) = self.meta_line.take() {
                    // as the web's meta row in the text's own line height, 6 under what is over it
                    ui.add_space(6.0);
                    answer.meta_link = line.show_links(ui).pressed;
                }
            });
            // the actions at the row's end, 8 apart, level with the title block's foot
            let height = block.response.rect.height();
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), height),
                Layout::right_to_left(Align::Max),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    actions(ui);
                },
            );
        });
        answer
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
