//! Text written in markdown, after sokkel-ui's `Markdown`: headings, paragraphs, bullet, numbered
//! and task lists, quotes and callouts, fenced code with a copy button, rules, and in a sentence
//! bold, italic, struck and code, links, and with `links` the `[[links]]` between notes and their
//! `#tags`. Two scales: `message` (the default), whose headings stay within a reply, and
//! `document`, from a page title down. Tables, math and diagrams are not read; their text shows as
//! written.

mod inline;
mod parse;

use std::sync::Arc;

use egui::{
    Align, Color32, CornerRadius, FontSelection, Frame, Galley, Id, Margin, Pos2, Rect, Response, RichText, Sense,
    Shape, Stroke, StrokeKind, Ui, WidgetInfo, pos2, text::LayoutJob, vec2,
};

pub use inline::{Run, RunLink, runs};
pub use parse::{Block, Callout, ListItem, blocks, heading_id};

use crate::{Icon, IconButton, TextRole, Tokens, icons, mono_font, mono_size, tokens};
use inline::Reading;
use parse::UniqueIds;

/// Where the text is read, which sets its headings' sizes and the room between blocks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkdownScale {
    /// Within a message: the largest heading is a detail title.
    #[default]
    Message,
    /// A document's, from a page title down, its blocks parted as a page's.
    Document,
}

impl std::str::FromStr for MarkdownScale {
    type Err = String;

    /// The scale by the web's name for it: `message` or `document`.
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "message" => Ok(Self::Message),
            "document" => Ok(Self::Document),
            _ => Err(format!("no markdown scale `{name}`")),
        }
    }
}

/// The words markdown says: each callout's title, and its code blocks' copy button.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownWords {
    pub note: String,
    pub tip: String,
    pub important: String,
    pub warning: String,
    pub caution: String,
    /// The copy button's name.
    pub copy: String,
    /// The copy button's name for a moment after a press.
    pub copied: String,
}

impl Default for MarkdownWords {
    fn default() -> Self {
        Self {
            note: "Note".into(),
            tip: "Tip".into(),
            important: "Important".into(),
            warning: "Warning".into(),
            caution: "Caution".into(),
            copy: "Copy".into(),
            copied: "Copied".into(),
        }
    }
}

/// What a press on the text asked for, and where its headings stand.
#[derive(Debug)]
pub struct MarkdownResponse {
    /// The whole text's area.
    pub response: Response,
    /// A `[[link]]` pressed: the note it names, and the heading after its `#`.
    pub opened: Option<(String, Option<String>)>,
    /// A `#tag` pressed, named without its `#`.
    pub tag: Option<String>,
    /// An address pressed; it opens in a new tab too unless `open_urls(false)`.
    pub url: Option<String>,
    /// With `anchors`, each heading's id (as [`heading_id`] makes it, a repeated one numbered after
    /// the first) and where it stands, in order.
    pub headings: Vec<(String, Rect)>,
}

/// Text written in markdown, as sokkel-ui's `Markdown`: paragraphs and lists 8 apart, list items
/// 2, the room between blocks the larger of theirs as the web's margins collapse; headings from a
/// detail title down at the `message` scale, from a page title down at `document`.
pub struct Markdown<'a> {
    text: &'a str,
    scale: MarkdownScale,
    anchors: bool,
    exists: Option<&'a dyn Fn(&str) -> bool>,
    tags: bool,
    live: bool,
    highlight: &'a str,
    inline: Option<(TextRole, &'static str)>,
    reveal: Option<&'a str>,
    open_urls: bool,
    words: MarkdownWords,
}

impl<'a> Markdown<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            scale: MarkdownScale::Message,
            anchors: false,
            exists: None,
            tags: false,
            live: true,
            highlight: "",
            inline: None,
            reveal: None,
            open_urls: true,
            words: MarkdownWords::default(),
        }
    }

    /// Heading sizes and the room between blocks: within a message (the default), or a document's.
    pub fn scale(mut self, scale: MarkdownScale) -> Self {
        self.scale = scale;
        self
    }

    /// Lists each heading's id and place in the response, for a table of contents.
    pub fn anchors(mut self, anchors: bool) -> Self {
        self.anchors = anchors;
        self
    }

    /// Draws `[[Note]]`, `[[Note|words]]` and `[[Note#Heading]]` as links to notes, a link to a note
    /// `exists` says is not there drawn faint; a press comes back in `opened`.
    pub fn links(mut self, exists: &'a dyn Fn(&str) -> bool) -> Self {
        self.exists = Some(exists);
        self
    }

    /// Draws each `#tag` as a chip; a press comes back in `tag`. Off by default, as the web's tags
    /// are text without `onTag`.
    pub fn tags(mut self, tags: bool) -> Self {
        self.tags = tags;
        self
    }

    /// False draws every link as its words, a `[[link]]` as the words it shows, for text set inside
    /// something pressed as a whole.
    pub fn live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    /// Marks each place these words stand in the text, in any case and outside code, as a search's
    /// matches are marked.
    pub fn highlight(mut self, words: &'a str) -> Self {
        self.highlight = words;
        self
    }

    /// Draws the text as one run in its container's type, `role` in the `ink` color token: each
    /// block laid out as its text, for one line such as a backlink's.
    pub fn inline(mut self, role: TextRole, ink: &'static str) -> Self {
        self.inline = Some((role, ink));
        self
    }

    /// Scrolls the heading with this text or id into view on the frame it is drawn.
    pub fn reveal(mut self, heading: Option<&'a str>) -> Self {
        self.reveal = heading;
        self
    }

    /// False leaves an address pressed to the caller, in `url`, rather than opening it in a new tab.
    pub fn open_urls(mut self, open: bool) -> Self {
        self.open_urls = open;
        self
    }

    pub fn words(mut self, words: MarkdownWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> MarkdownResponse {
        let t = tokens(ui.ctx());
        let base = ui.next_auto_id();
        ui.skip_ahead_auto_ids(1);
        let reading = Reading {
            notes: self.exists.is_some() || !self.live,
            tags: self.tags && self.live,
        };
        let mut draw = Draw {
            metrics: Metrics::of(self.scale, &t),
            t: t.clone(),
            md: &self,
            reading,
            ids: UniqueIds::default(),
            base,
            count: 0,
            lead: 0.0,
            first_baseline: None,
            opened: None,
            tag: None,
            url: None,
            headings: Vec::new(),
        };
        let spacing = ui.spacing().item_spacing.y;
        let response = ui
            .scope(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                if let Some((role, ink)) = self.inline {
                    let text = flat(&blocks(self.text));
                    let style = Style {
                        role,
                        wght: 0.0,
                        ink: t.color(ink),
                        line_height: None,
                    };
                    draw.sentence(ui, &text, &style);
                } else {
                    draw.blocks(ui, &blocks(self.text), t.color("foreground"), None);
                }
                ui.spacing_mut().item_spacing.y = spacing;
            })
            .response;
        if let Some(url) = &draw.url
            && self.open_urls
        {
            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
        }
        MarkdownResponse {
            response,
            opened: draw.opened,
            tag: draw.tag,
            url: draw.url,
            headings: draw.headings,
        }
    }
}

/// The text of blocks as one line, each block's text parted from the next by a space.
fn flat(read: &[Block]) -> String {
    let mut out: Vec<String> = Vec::new();
    for block in read {
        match block {
            Block::Heading(_, text) | Block::Paragraph(text) => out.push(text.clone()),
            Block::List(_, items) => out.extend(items.iter().map(|item| flat(&item.blocks))),
            Block::Quote(_, inner) => out.push(flat(inner)),
            Block::Code(_, code) => out.push(format!("`{}`", code.replace('`', ""))),
            Block::Rule => {}
        }
    }
    out.retain(|s| !s.is_empty());
    out.join(" ")
}

/// The room around a block: above and below it.
type Room = (f32, f32);

/// What a scale sets: its headings' roles, room and weight; and the prose's leading.
struct Metrics {
    /// Levels 1 to 4: the role, the room around, and a weight of their own (0 for the role's).
    headings: [(TextRole, Room, f32); 4],
    /// The prose's relaxed leading, `leading-relaxed`, for body text.
    relaxed: f32,
}

/// `my-2`: paragraphs and lists.
const PARAGRAPH: Room = (8.0, 8.0);
/// `my-2`: quotes.
const QUOTE: Room = (8.0, 8.0);
/// `my-3`: callouts and code blocks.
const CALLOUT: Room = (12.0, 12.0);
const CODE: Room = (12.0, 12.0);
/// `my-4`: rules.
const RULE: Room = (16.0, 16.0);
/// `my-0.5`: list items.
const ITEM_GAP: f32 = 2.0;
/// `leading-relaxed`: a line 1.625 times its text's size.
const RELAXED: f32 = 1.625;
/// `pl-5`: a list's items stand this far in, their markers in the room before.
const LIST_INDENT: f32 = 20.0;
/// `border-l-2 pl-3`: a quote's rule and the room after it.
const QUOTE_RULE: f32 = 2.0;
const QUOTE_INDENT: f32 = 14.0;
/// `px-1`: a chip's room each side of its words.
const CHIP_PAD: f32 = 4.0;
/// `underline-offset-2`: a link's underline below the baseline, the face's own drop with it.
const UNDERLINE_DROP: f32 = 3.0;
/// `[&_[id]]:scroll-mt-4`: a revealed heading stands this far below the top.
const SCROLL_MARGIN: f32 = 16.0;
/// A bullet's room before it in its list's indent.
const BULLET_INSET: f32 = 3.0;
/// A task's box, as the browser draws a checkbox: 13 square, 4 before and 3 after it, its foot 3
/// above the baseline.
const TASK_BOX: f32 = 13.0;

impl Metrics {
    fn of(scale: MarkdownScale, t: &Tokens) -> Self {
        let relaxed = (TextRole::Body.font(t).size * RELAXED).round();
        let headings = match scale {
            MarkdownScale::Message => [
                (TextRole::DetailTitle, (20.0, 8.0), 0.0),
                (TextRole::SectionTitle, (16.0, 8.0), 600.0),
                (TextRole::SectionTitle, (12.0, 6.0), 0.0),
                (TextRole::Head, (12.0, 4.0), 0.0),
            ],
            MarkdownScale::Document => [
                (TextRole::PageTitle, (24.0, 12.0), 0.0),
                (TextRole::DetailTitle, (20.0, 8.0), 0.0),
                (TextRole::SectionTitle, (16.0, 6.0), 0.0),
                (TextRole::Head, (12.0, 4.0), 0.0),
            ],
        };
        Self { headings, relaxed }
    }

    fn heading(&self, level: u8) -> (TextRole, Room, f32) {
        self.headings[(level.clamp(1, 4) - 1) as usize]
    }
}

/// How a sentence is set.
struct Style {
    role: TextRole,
    /// A weight of its own on the `wght` axis; 0 for the role's.
    wght: f32,
    ink: Color32,
    /// A line height of its own; `None` for the role's.
    line_height: Option<f32>,
}

/// One run's place in a laid-out sentence, by character, and how it is drawn beyond its glyphs.
struct Span {
    from: usize,
    to: usize,
    /// A ground under the words: its color, corners and the room each side.
    ground: Option<(Color32, CornerRadius, f32)>,
    underline: Option<Color32>,
    link: Option<RunLink>,
    words: String,
}

/// One row's piece of a span: the glyphs' box from the face's top to its foot, and the baseline.
pub(crate) struct Piece {
    pub(crate) rect: Rect,
    pub(crate) baseline: f32,
}

/// Where a span's characters stand in a galley painted at `origin`, one piece per row.
pub(crate) fn pieces(galley: &Galley, origin: Pos2, from: usize, to: usize) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut at = 0;
    for placed in &galley.rows {
        let glyphs = &placed.row.glyphs;
        let mut found: Option<Piece> = None;
        for (k, glyph) in glyphs.iter().enumerate() {
            let c = at + k;
            if c < from || c >= to {
                continue;
            }
            let top = origin.y + placed.pos.y + glyph.pos.y - glyph.font_ascent;
            let left = origin.x + placed.pos.x + glyph.pos.x;
            let rect = Rect::from_min_max(
                pos2(left, top),
                pos2(left + glyph.advance_width, top + glyph.font_height),
            );
            match &mut found {
                Some(piece) => piece.rect = piece.rect.union(rect),
                None => {
                    found = Some(Piece {
                        rect,
                        baseline: origin.y + placed.pos.y + glyph.pos.y,
                    })
                }
            }
        }
        out.extend(found);
        at += glyphs.len() + usize::from(placed.ends_with_newline);
    }
    out
}

/// The state of one drawing: tokens, settings, and what was pressed and found.
struct Draw<'a> {
    t: Arc<Tokens>,
    metrics: Metrics,
    md: &'a Markdown<'a>,
    reading: Reading,
    ids: UniqueIds,
    base: Id,
    /// Sentences drawn so far, so each link's id is its own.
    count: usize,
    /// Room before the next sentence's first line, for a task's box.
    lead: f32,
    /// The first baseline the next sentence sets, when asked for.
    first_baseline: Option<f32>,
    opened: Option<(String, Option<String>)>,
    tag: Option<String>,
    url: Option<String>,
    headings: Vec<(String, Rect)>,
}

impl Draw<'_> {
    fn room(&self, block: &Block, paragraph: Room) -> Room {
        match block {
            Block::Heading(level, _) => self.metrics.heading(*level).1,
            Block::Paragraph(_) | Block::List(..) => paragraph,
            Block::Quote(None, _) => QUOTE,
            Block::Quote(Some(_), _) => CALLOUT,
            Block::Code(..) => CODE,
            Block::Rule => RULE,
        }
    }

    /// Blocks one under another in `ink`, the room between them as the scale sets it; `paragraph`
    /// gives paragraphs room of their own, as a callout's are 4 apart.
    fn blocks(&mut self, ui: &mut Ui, read: &[Block], ink: Color32, paragraph: Option<Room>) {
        let paragraph = paragraph.unwrap_or(PARAGRAPH);
        let mut below: Option<f32> = None;
        for block in read {
            let (above, after) = self.room(block, paragraph);
            if let Some(below) = below {
                ui.add_space(below.max(above));
            }
            self.block(ui, block, ink);
            below = Some(after);
        }
    }

    fn block(&mut self, ui: &mut Ui, block: &Block, ink: Color32) {
        let t = self.t.clone();
        match block {
            Block::Heading(level, text) => {
                let (role, _, wght) = self.metrics.heading(*level);
                let style = Style {
                    role,
                    wght,
                    ink,
                    line_height: None,
                };
                let (rect, words) = self.sentence(ui, text, &style);
                let id = self.ids.take(heading_id(&words));
                if let Some(reveal) = self.md.reveal
                    && (heading_id(reveal) == id || reveal == words)
                {
                    ui.scroll_to_rect(rect.with_min_y(rect.top() - SCROLL_MARGIN), Some(Align::TOP));
                }
                if self.md.anchors {
                    self.headings.push((id, rect));
                }
            }
            Block::Paragraph(text) => {
                let style = Style {
                    role: TextRole::Body,
                    wght: 0.0,
                    ink,
                    line_height: Some(self.metrics.relaxed),
                };
                self.sentence(ui, text, &style);
            }
            Block::List(start, items) => self.list(ui, *start, items, ink),
            Block::Quote(None, inner) => {
                let shown = Frame::NONE
                    .inner_margin(Margin {
                        left: QUOTE_INDENT as i8,
                        ..Margin::ZERO
                    })
                    .show(ui, |ui| self.blocks(ui, inner, t.color("muted-foreground"), None));
                let rect = shown.response.rect;
                ui.painter().rect_filled(
                    Rect::from_min_size(rect.min, vec2(QUOTE_RULE, rect.height())),
                    0.0,
                    t.color("border-strong"),
                );
            }
            Block::Quote(Some(kind), inner) => self.callout(ui, *kind, inner),
            Block::Code(language, code) => self.code(ui, language, code),
            Block::Rule => {
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                ui.painter().rect_filled(rect, 0.0, t.color("border"));
            }
        }
    }

    fn list(&mut self, ui: &mut Ui, start: Option<u32>, items: &[ListItem], ink: Color32) {
        let t = self.t.clone();
        let width = ui.available_width();
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                ui.add_space(ITEM_GAP);
            }
            let marker = match start {
                Some(n) => format!("{}.", n + i as u32),
                None => "\u{2022}".to_owned(),
            };
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let (spot, _) = ui.allocate_exact_size(vec2(LIST_INDENT, 1.0), Sense::hover());
                if item.task.is_some() {
                    // the box, then the space the web sets after it
                    let space = ui.fonts_mut(|f| f.glyph_width(&TextRole::Body.font(&t), ' '));
                    self.lead = 4.0 + TASK_BOX + 3.0 + space;
                }
                // the item's first baseline, kept apart from an outer item's
                let outer = self.first_baseline.replace(f32::NAN);
                ui.vertical(|ui| {
                    ui.set_max_width(width - LIST_INDENT);
                    self.blocks(ui, &item.blocks, ink, None);
                });
                let baseline = std::mem::replace(&mut self.first_baseline, outer).filter(|b| b.is_finite());
                self.lead = 0.0;
                let font = TextRole::Body.font(&t);
                // the marker on the item's first baseline: a numbered one and a space against
                // the text, a bullet 3 into the room, as the browser sets them
                let shown = if start.is_some() { format!("{marker} ") } else { marker };
                let galley = ui.painter().layout_no_wrap(shown, font, ink);
                let ascent = galley
                    .rows
                    .first()
                    .and_then(|r| r.row.glyphs.first())
                    .map_or(0.0, |g| g.pos.y);
                let top = baseline.map_or(spot.top(), |b| b - ascent);
                let left = if start.is_some() {
                    spot.right() - galley.size().x
                } else {
                    spot.left() + BULLET_INSET
                };
                ui.painter().galley(pos2(left, top), galley, ink);
                if let (Some(checked), Some(baseline)) = (item.task, baseline) {
                    let rect = Rect::from_min_size(
                        pos2(spot.right() + 4.0, baseline - 3.0 - TASK_BOX),
                        vec2(TASK_BOX, TASK_BOX),
                    );
                    self.task_box(ui, rect, checked);
                }
            });
        }
    }

    /// A task's box, as a disabled checkbox: checked filled in the accent with its check.
    fn task_box(&self, ui: &Ui, rect: Rect, checked: bool) {
        let t = &self.t;
        let dim = |c: Color32| super::dim(c, false);
        let (fill, edge) = if checked {
            (t.color("primary"), t.color("primary"))
        } else if t.dark {
            (t.color_at("input", 0.3), t.color("input"))
        } else {
            (Color32::TRANSPARENT, t.color("input"))
        };
        let painter = ui.painter();
        painter.rect(
            rect,
            t.radius("sm"),
            dim(fill),
            Stroke::new(1.0, dim(edge)),
            StrokeKind::Inside,
        );
        if checked {
            icons::paint(
                painter,
                rect.center(),
                Icon::Check,
                TASK_BOX - 2.0,
                dim(t.color("primary-foreground")),
            );
        }
    }

    /// A quote whose first line names a callout: a framed block in the kind's tint, its icon and
    /// title in the kind's ink, then the quote's blocks 4 apart.
    fn callout(&mut self, ui: &mut Ui, kind: Callout, inner: &[Block]) {
        let t = self.t.clone();
        let words = &self.md.words;
        let (icon, tone, ink, frame, title) = match kind {
            Callout::Note => (Icon::Info, "info", "info-ink", 0.45, &words.note),
            Callout::Tip => (Icon::Lightbulb, "good", "good-ink", 0.45, &words.tip),
            Callout::Important => (
                Icon::MessageSquareWarning,
                "kind-violet",
                "kind-violet-ink",
                0.45,
                &words.important,
            ),
            Callout::Warning => (Icon::TriangleAlert, "warning", "warning-ink", 0.5, &words.warning),
            Callout::Caution => (Icon::CircleAlert, "critical", "critical-ink", 0.45, &words.caution),
        };
        let title = title.clone();
        let text_ink = t.color("foreground");
        Frame::NONE
            .fill(t.color_at(tone, 0.08))
            .stroke(Stroke::new(1.0, t.color_at(tone, frame)))
            .corner_radius(t.radius("surface"))
            .inner_margin(Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let (spot, _) = ui.allocate_exact_size(vec2(16.0, 16.0 + 2.0), Sense::hover());
                    icons::paint(
                        ui.painter(),
                        pos2(spot.center().x, spot.top() + 2.0 + 8.0),
                        icon,
                        16.0,
                        t.color(ink),
                    );
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        let style = Style {
                            role: TextRole::Body,
                            wght: 500.0,
                            ink: t.color(ink),
                            line_height: Some(self.metrics.relaxed),
                        };
                        self.sentence(ui, &title.replace(['*', '_', '`', '[', '#'], ""), &style);
                        if !inner.is_empty() {
                            ui.add_space(4.0);
                            self.blocks(ui, inner, text_ink, Some((4.0, 4.0)));
                        }
                    });
                });
            });
    }

    /// Fenced code in the code face on the editor's ground, framed, scrolled sideways when wide,
    /// with a copy button over its corner under the pointer.
    fn code(&mut self, ui: &mut Ui, language: &str, code: &str) {
        let t = self.t.clone();
        self.count += 1;
        let id = self.base.with(("code", self.count));
        let body = TextRole::Body.font(&t).size;
        let font = mono_font(ui.ctx(), mono_size(&t, body), 400.0);
        let under = ui.painter().add(Shape::Noop);
        let shown = Frame::NONE
            .fill(t.color("editor-ground"))
            .stroke(Stroke::new(1.0, t.color("input")))
            .corner_radius(t.radius("surface"))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                egui::ScrollArea::horizontal().id_salt(id).show(ui, |ui| {
                    let text = if code.is_empty() { " " } else { code };
                    let format = |color: Color32| egui::TextFormat {
                        font_id: font.clone(),
                        color,
                        line_height: Some(self.metrics.relaxed),
                        ..Default::default()
                    };
                    // a language the fence names is colored as the editor colors it
                    let tokens = super::code_editor::fence_language(language)
                        .map(|language| super::code_editor::highlight_tokens(language, text))
                        .unwrap_or_default();
                    let mut job = LayoutJob::default();
                    let mut at = 0;
                    for token in tokens {
                        if token.from < at || token.to > text.len() {
                            continue;
                        }
                        if token.from > at {
                            job.append(&text[at..token.from], 0.0, format(t.color("foreground")));
                        }
                        let ink = token
                            .kind
                            .ink()
                            .map_or_else(|| t.color("foreground"), |key| t.color(key));
                        job.append(&text[token.from..token.to], 0.0, format(ink));
                        at = token.to;
                    }
                    if at < text.len() {
                        job.append(&text[at..], 0.0, format(t.color("foreground")));
                    }
                    let galley = ui.painter().layout_job(job);
                    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
                    ui.painter().galley(rect.min, galley, t.color("foreground"));
                });
            });
        let rect = shown.response.rect;
        if t.dark {
            // the editor's ground washes the input's dark ground rather than replacing it
            ui.painter().set(
                under,
                Shape::rect_filled(rect, t.radius("surface"), t.color_at("input", 0.3)),
            );
        }
        let copied_id = id.with("copied");
        let now = ui.input(|i| i.time);
        let copied = ui
            .data(|d| d.get_temp::<f64>(copied_id))
            .is_some_and(|at| now - at < 1.5);
        let hovered = ui.rect_contains_pointer(rect);
        if hovered || copied {
            let side = t.density("control-xs");
            let spot = Rect::from_min_size(pos2(rect.right() - 8.0 - side, rect.top() + 8.0), vec2(side, side));
            let radius = t.radius("control");
            let radius = CornerRadius::same(radius.nw.min(10));
            ui.painter().rect(
                spot,
                radius,
                t.color("background"),
                Stroke::new(1.0, t.color_at("foreground", 0.1)),
                StrokeKind::Outside,
            );
            let words = &self.md.words;
            let (icon, label) = if copied {
                (Icon::Check, &words.copied)
            } else {
                (Icon::Copy, &words.copy)
            };
            let pressed = ui
                .put(spot, IconButton::new(icon, label.clone()).xs())
                .on_hover_text(label);
            if pressed.clicked() {
                ui.ctx().copy_text(code.to_owned());
                ui.data_mut(|d| d.insert_temp(copied_id, now));
            }
            if copied {
                ui.ctx().request_repaint_after_secs(0.25);
            }
        }
    }

    /// The runs of a sentence as the drawing reads them: links dropped where they are not live.
    fn runs(&self, text: &str) -> Vec<Run> {
        let mut read = if self.reading.notes || self.reading.tags {
            inline::read(text, self.reading)
        } else {
            runs(text)
        };
        if !self.md.live {
            for run in &mut read {
                run.link = None;
            }
        }
        read
    }

    /// One sentence wrapped in the room there is: its glyphs, the grounds under its chips and marks,
    /// its links' underlines, and a press on a link kept. Returns its area and its words.
    fn sentence(&mut self, ui: &mut Ui, text: &str, style: &Style) -> (Rect, String) {
        let t = self.t.clone();
        self.count += 1;
        let size = style.role.font(&t).size;
        let line_height = style.line_height.unwrap_or_else(|| style.role.line_height(&t));
        let link_ink = t.color("primary-ink");
        let mut job = LayoutJob::default();
        let mut spans: Vec<Span> = Vec::new();
        let mut at = 0;
        let mut pad = std::mem::take(&mut self.lead);
        for run in self.runs(text) {
            let pieces = if run.code || self.md.highlight.trim().is_empty() {
                vec![(run.text.clone(), false)]
            } else {
                inline::marks(&run.text, self.md.highlight)
            };
            for (piece, marked) in pieces {
                if piece.is_empty() {
                    continue;
                }
                let missing = matches!(&run.link, Some(RunLink::Note { target, .. }) if self.md.exists.is_some_and(|exists| !exists(target)));
                let mut ink = match &run.link {
                    Some(_) => link_ink,
                    None => style.ink,
                };
                if marked {
                    ink = t.color("primary-ink");
                }
                if missing {
                    ink = ink.gamma_multiply(0.6);
                }
                let rich = if run.code {
                    RichText::new(&piece)
                        .font(mono_font(ui.ctx(), mono_size(&t, size), 400.0))
                        .color(ink)
                        .line_height(Some(line_height))
                } else {
                    let mut rich = style.role.rich(&t, &piece).color(ink);
                    if style.line_height.is_some() {
                        rich = rich.line_height(Some(line_height));
                    }
                    if run.bold {
                        rich = rich.variation("wght", 600.0);
                    } else if style.wght > 0.0 {
                        rich = rich.variation("wght", style.wght);
                    }
                    if run.italic {
                        rich = rich.italics();
                    }
                    if run.strike {
                        rich = rich.strikethrough();
                    }
                    rich
                };
                rich.append_to(&mut job, ui.style(), FontSelection::Default, Align::Min);
                let chip = run.code || matches!(run.link, Some(RunLink::Tag(_)));
                if let Some(section) = job.sections.last_mut() {
                    section.leading_space += pad + if chip { CHIP_PAD } else { 0.0 };
                }
                pad = if chip { CHIP_PAD } else { 0.0 };
                let ground = if run.code {
                    Some((t.color("muted"), t.radius("sm"), CHIP_PAD))
                } else if matches!(run.link, Some(RunLink::Tag(_))) {
                    Some((t.color_at("primary", 0.12), t.radius("sm"), CHIP_PAD))
                } else if marked {
                    Some((t.color_at("primary", 0.15), t.radius("mini"), 0.0))
                } else {
                    None
                };
                let underline = match &run.link {
                    Some(RunLink::Note { .. } | RunLink::Url(_)) => Some(ink),
                    _ => None,
                };
                let count = piece.chars().count();
                spans.push(Span {
                    from: at,
                    to: at + count,
                    ground,
                    underline,
                    link: run.link.clone(),
                    words: piece,
                });
                at += count;
            }
        }
        if job.sections.is_empty() {
            style
                .role
                .rich(&t, " ")
                .append_to(&mut job, ui.style(), FontSelection::Default, Align::Min);
        }
        job.wrap.max_width = ui.available_width();
        let galley = ui.painter().layout_job(job);
        let words = galley.text().to_owned();
        let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(egui::Role::Label, true, galley.text()));
        if let Some(first) = &mut self.first_baseline
            && !first.is_finite()
        {
            *first = galley
                .rows
                .first()
                .and_then(|r| r.row.glyphs.first().map(|g| rect.top() + r.pos.y + g.pos.y))
                .unwrap_or(rect.top());
        }
        if !ui.is_rect_visible(rect) {
            return (rect, words);
        }
        let origin = rect.min;
        for span in &spans {
            if let Some((color, radius, pad)) = span.ground {
                for piece in pieces(&galley, origin, span.from, span.to) {
                    ui.painter()
                        .rect_filled(piece.rect.expand2(vec2(pad, 0.0)), radius, color);
                }
            }
        }
        ui.painter().galley(origin, galley.clone(), style.ink);
        for span in &spans {
            if let Some(color) = span.underline {
                for piece in pieces(&galley, origin, span.from, span.to) {
                    let y = (piece.baseline + UNDERLINE_DROP).round() + 0.5;
                    ui.painter()
                        .hline(piece.rect.left()..=piece.rect.right(), y, Stroke::new(1.0, color));
                }
            }
        }
        // a press on a link: runs of one link side by side are one link
        let mut k = 0;
        while k < spans.len() {
            let Some(link) = spans[k].link.clone() else {
                k += 1;
                continue;
            };
            let mut end = k + 1;
            while end < spans.len() && spans[end].link.as_ref() == Some(&link) {
                end += 1;
            }
            let words: String = spans[k..end].iter().map(|s| s.words.as_str()).collect();
            for (row, piece) in pieces(&galley, origin, spans[k].from, spans[end - 1].to)
                .iter()
                .enumerate()
            {
                let pad = spans[k].ground.map_or(0.0, |g| g.2);
                let hit = ui.interact(
                    piece.rect.expand2(vec2(pad, 0.0)),
                    self.base.with((self.count, k, row)),
                    Sense::click(),
                );
                // a link that wraps is named once, on its first row
                if row == 0 {
                    hit.widget_info(|| WidgetInfo::labeled(egui::Role::Link, true, &words));
                }
                if hit.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if hit.clicked() {
                    match &link {
                        RunLink::Note { target, heading } => self.opened = Some((target.clone(), heading.clone())),
                        RunLink::Tag(tag) => self.tag = Some(tag.clone()),
                        RunLink::Url(url) => self.url = Some(url.clone()),
                    }
                }
            }
            k = end;
        }
        (rect, words)
    }
}
