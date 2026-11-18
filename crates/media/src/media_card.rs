//! One album, film or episode in a grid, after sokkel-ui-media's `MediaCard` and `MediaGrid`.

use std::sync::Arc;

use egui::{
    Align, CornerRadius, DragAndDrop, FontSelection, Galley, Id, Painter, Rect, Response, Sense, Stroke, StrokeKind,
    TextWrapMode, Ui, pos2, text::LayoutJob, text::TextWrapping, vec2,
};

use sokkel_egui::{DragData, DropdownMenu, Icon, MenuItem, TextRole, Tokens, icons, mono_font, mono_size, tokens};

use crate::artwork::{Artwork, PicturePaint};

/// The words a media card says.
#[derive(Clone, Debug)]
pub struct MediaCardWords {
    pub play: String,
    pub playing: String,
    pub menu: String,
    /// `{percent}% watched`.
    pub watched: String,
    /// `Drag {title}`.
    pub drag: String,
}

impl Default for MediaCardWords {
    fn default() -> Self {
        Self {
            play: "Play".into(),
            playing: "Now playing".into(),
            menu: "More actions".into(),
            watched: "{percent}% watched".into(),
            drag: "Drag {title}".into(),
        }
    }
}

/// A cover's shape: square for an album, wide (16 by 9) for a video.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MediaShape {
    #[default]
    Square,
    Wide,
}

/// The cover above the words, or beside them, 160 wide.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MediaCardLayout {
    #[default]
    Stack,
    Row,
}

/// What a card was asked this frame.
#[derive(Clone, Debug)]
pub struct MediaCardResponse {
    /// The whole card.
    pub response: Response,
    /// The card was pressed, to open it (the web's `onSelect`).
    pub selected: bool,
    /// The play button was pressed (with `playable`).
    pub play: bool,
    /// The menu's action chosen, by its index in `menu`.
    pub menu: Option<usize>,
    /// The card is being dragged (with `drag`); its data is egui's drag payload meanwhile.
    pub dragging: bool,
}

/// One album, film or episode in a grid, after sokkel-ui-media's `MediaCard`: the cover first, on
/// `muted` in a `border` ring with the surface's corners, then its title in `text.label` on up to
/// `title_lines` lines and a line under it in `text.help`, muted. The whole card opens it, rising on
/// `elevation-pop` under the pointer; a play button rises over the cover on hover. A wide cover can
/// carry its length on the frame's corner (`badge`, in the code face on `shade/70`) and how much was
/// watched along its foot. `menu` adds a `More actions` button beside the title. A card with no
/// `cover` wears one drawn from `placeholder_seed`, the title by default; `cover` paints the app's
/// own picture into the frame.
#[must_use = "show it with `.show(ui)`"]
pub struct MediaCard<'a> {
    id_salt: Id,
    title: String,
    subtitle: Option<String>,
    cover: Option<PicturePaint<'a>>,
    placeholder_seed: Option<String>,
    shape: MediaShape,
    layout: MediaCardLayout,
    selectable: bool,
    playable: bool,
    playing: bool,
    badge: Option<String>,
    progress: Option<f32>,
    menu: &'a [MenuItem],
    title_lines: Option<usize>,
    drag: Option<DragData>,
    words: MediaCardWords,
    width: Option<f32>,
}

impl<'a> MediaCard<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, title: impl Into<String>) -> Self {
        Self {
            id_salt: Id::unique(id_salt),
            title: title.into(),
            subtitle: None,
            cover: None,
            placeholder_seed: None,
            shape: MediaShape::Square,
            layout: MediaCardLayout::Stack,
            selectable: true,
            playable: false,
            playing: false,
            badge: None,
            progress: None,
            menu: &[],
            title_lines: None,
            drag: None,
            words: MediaCardWords::default(),
            width: None,
        }
    }

    /// Who made it, and when, or which episode it is.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// The artwork, painted by the app to fill the frame (the web's `cover` node), such as an
    /// [`Artwork`] or a scene drawn with the painter.
    pub fn cover(mut self, paint: impl Fn(&Painter, Rect) + 'a) -> Self {
        self.cover = Some(Box::new(paint));
        self
    }

    /// The stable id the drawn cover is chosen from when there is no `cover`; the title by default.
    pub fn placeholder_seed(mut self, seed: impl Into<String>) -> Self {
        self.placeholder_seed = Some(seed.into());
        self
    }

    pub fn shape(mut self, shape: MediaShape) -> Self {
        self.shape = shape;
        self
    }

    pub fn layout(mut self, layout: MediaCardLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Whether the card opens on a press (the web's `onSelect` or `href`); on by default.
    pub fn selectable(mut self, on: bool) -> Self {
        self.selectable = on;
        self
    }

    /// A play button over the cover, shown on hover (the web's `onPlay`).
    pub fn playable(mut self, on: bool) -> Self {
        self.playable = on;
        self
    }

    /// Marks the one playing now.
    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// A short mark on the frame's corner, such as the length of a video.
    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    /// How much was watched or heard, from 0 to 1.
    pub fn progress(mut self, progress: f32) -> Self {
        self.progress = Some(progress);
        self
    }

    /// Items for the card's menu.
    pub fn menu(mut self, items: &'a [MenuItem]) -> Self {
        self.menu = items;
        self
    }

    /// How many lines the title may take: two for a wide cover or a row, one otherwise.
    pub fn title_lines(mut self, lines: usize) -> Self {
        self.title_lines = Some(lines.max(1));
        self
    }

    /// What the card carries when it is dragged elsewhere, such as onto a timeline's track; it
    /// does not drag without it.
    pub fn drag(mut self, data: DragData) -> Self {
        self.drag = Some(data);
        self
    }

    pub fn words(mut self, words: MediaCardWords) -> Self {
        self.words = words;
        self
    }

    /// How wide the card is; the width the `ui` has left by default.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn show(self, ui: &mut Ui) -> MediaCardResponse {
        let t = tokens(ui.ctx());
        let id = ui.make_persistent_id(self.id_salt);
        let width = self.width.unwrap_or_else(|| ui.available_width());
        let row = self.layout == MediaCardLayout::Row;
        let lines = self
            .title_lines
            .unwrap_or(if row || self.shape == MediaShape::Wide { 2 } else { 1 });
        let cover_width = if row { 160.0 } else { width };
        let cover_height = match self.shape {
            MediaShape::Square => cover_width,
            MediaShape::Wide => cover_width * 9.0 / 16.0,
        };
        let has_menu = !self.menu.is_empty();
        let words_width = if row {
            (width - cover_width - 12.0).max(0.0)
        } else {
            width
        };
        // the words' column, less the menu's 28 that stands 6 past the card's end
        let text_width = (words_width - if has_menu { 28.0 - 6.0 + 4.0 } else { 0.0 }).max(0.0);
        let title = clamped(ui, &t, &self.title, text_width, lines);
        let title_clamped = title.elided;
        let subtitle = self.subtitle.as_ref().map(|s| {
            egui::WidgetText::from(TextRole::Help.rich(&t, s)).into_galley(
                ui,
                Some(if row {
                    TextWrapMode::Wrap
                } else {
                    TextWrapMode::Truncate
                }),
                text_width,
                TextRole::Help.font(&t),
            )
        });
        let words_height = title.galley.size().y + subtitle.as_ref().map_or(0.0, |s| s.size().y);
        let height = if row {
            cover_height.max(words_height)
        } else {
            cover_height + 10.0 + words_height
        };
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let frame = Rect::from_min_size(rect.min, vec2(cover_width, cover_height));
        let words_left = if row { frame.right() + 12.0 } else { rect.left() };
        let words_top = if row { rect.top() } else { frame.bottom() + 10.0 };

        // the whole card opens it, or drags it
        let sense = match (self.selectable, self.drag.is_some()) {
            (_, true) => Sense::click_and_drag(),
            (true, false) => Sense::click(),
            (false, false) => Sense::hover(),
        };
        let response = ui.interact(rect, id, sense);
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, self.selectable, &self.title));
        let mut out = MediaCardResponse {
            response: response.clone(),
            selected: self.selectable && response.clicked(),
            play: false,
            menu: None,
            dragging: false,
        };
        if let Some(data) = &self.drag {
            if response.drag_started() {
                DragAndDrop::set_payload(ui.ctx(), data.clone());
            }
            out.dragging = response.dragged() && DragAndDrop::has_payload_of_type::<DragData>(ui.ctx());
        }

        // the play button over the cover, and the menu's button beside the title
        let play_rect = match self.shape {
            MediaShape::Square => {
                Rect::from_min_size(frame.right_bottom() - vec2(8.0 + 40.0, 8.0 + 40.0), vec2(40.0, 40.0))
            }
            MediaShape::Wide => Rect::from_center_size(frame.center(), vec2(40.0, 40.0)),
        };
        let mut play_response = None;
        if self.playable && !self.playing {
            let r = ui.interact(play_rect, id.with("play"), Sense::click());
            let label = format!("{}: {}", self.words.play, self.title);
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
            out.play = r.clicked();
            play_response = Some(r.on_hover_text(self.words.play.clone()));
        }
        let mut menu_response = None;
        if has_menu {
            let trigger = Rect::from_min_size(pos2(rect.right() + 6.0 - 28.0, words_top - 4.0), vec2(28.0, 28.0));
            let r = ui.interact(trigger, id.with("menu"), Sense::click());
            let label = format!("{}: {}", self.words.menu, self.title);
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
            out.menu = DropdownMenu::new(self.menu).end(true).width(176.0).show(&r);
            menu_response = Some(r.on_hover_text(self.words.menu.clone()));
        }

        if !ui.is_rect_visible(rect) {
            return out;
        }
        let mut painter = ui.painter().clone();
        if out.dragging {
            painter.multiply_opacity(0.4);
        }
        let painter = &painter;
        let radius = t.radius("surface");
        let lifted = self.selectable && response.hovered();
        if lifted {
            painter.add(t.shadow("elevation-pop").as_shape(frame, radius));
        }
        painter.rect_filled(frame, radius, t.color("muted"));
        let cover = painter.with_clip_rect(frame.intersect(painter.clip_rect()));
        match &self.cover {
            Some(paint) => paint(&cover, frame),
            None => Artwork::new(self.placeholder_seed.as_deref().unwrap_or(&self.title))
                .wide(self.shape == MediaShape::Wide)
                .paint(&cover, frame),
        }
        let watched = self.progress.map(|p| (p.clamp(0.0, 1.0) * 100.0).round());
        if let Some(badge) = &self.badge {
            let spec = t.text("xs");
            let font = mono_font(ui.ctx(), mono_size(&t, spec.size), 400.0);
            let galley = painter.layout_no_wrap(badge.clone(), font, t.color("on-shade"));
            let bottom = if watched.is_some_and(|w| w > 0.0) { 10.0 } else { 6.0 };
            let size = vec2(galley.size().x + 12.0, 16.0 + 4.0);
            let r = Rect::from_min_size(frame.right_bottom() - vec2(6.0, bottom) - size, size);
            painter.rect_filled(r, t.radius("mini"), t.color_at("shade", 0.7));
            painter.galley(r.center() - galley.size() / 2.0, galley, t.color("on-shade"));
        }
        if self.playing {
            let center = match self.shape {
                MediaShape::Square => frame.right_bottom() - vec2(8.0 + 16.0, 8.0 + 16.0),
                MediaShape::Wide => frame.left_top() + vec2(8.0 + 16.0, 8.0 + 16.0),
            };
            cover.add(
                t.shadow("elevation-pop")
                    .as_shape(Rect::from_center_size(center, vec2(32.0, 32.0)), CornerRadius::same(16)),
            );
            cover.circle_filled(center, 16.0, t.color("primary"));
            icons::paint(&cover, center, Icon::Volume2, 16.0, t.color("primary-foreground"));
        }
        if let Some(r) = &play_response
            && (response.hovered() || r.hovered() || r.has_focus())
        {
            cover.add(t.shadow("elevation-pop").as_shape(play_rect, CornerRadius::same(20)));
            cover.circle_filled(play_rect.center(), 20.0, t.color("primary"));
            sokkel_egui::paint_play(
                &cover,
                play_rect.center() + vec2(1.0, 0.0),
                16.0,
                t.color("primary-foreground"),
            );
            focus_ring_round(painter, &t, play_rect, r);
        }
        if let Some(w) = watched
            && w > 0.0
        {
            let bar = Rect::from_min_max(pos2(frame.left(), frame.bottom() - 4.0), frame.right_bottom());
            cover.rect_filled(bar, 0.0, t.color_at("shade", 0.4));
            let lit = Rect::from_min_max(bar.min, pos2(bar.left() + bar.width() * w / 100.0, bar.bottom()));
            cover.rect_filled(lit, 0.0, t.color("primary"));
        }
        painter.rect_stroke(frame, radius, Stroke::new(1.0, t.color("border")), StrokeKind::Outside);
        if response.has_focus() {
            painter.rect_stroke(
                rect.expand(4.0),
                radius,
                Stroke::new(t.length("focus-ring-width"), t.color("ring")),
                StrokeKind::Outside,
            );
        }

        let title_ink = if self.playing {
            t.color("primary-ink")
        } else {
            t.color("foreground")
        };
        let title_height = title.galley.size().y;
        painter.galley(pos2(words_left, words_top), title.galley, title_ink);
        if let Some(sub) = subtitle {
            painter.galley(
                pos2(words_left, words_top + title_height),
                sub,
                t.color("muted-foreground"),
            );
        }
        if let Some(r) = &menu_response {
            let hot = r.hovered();
            if hot {
                painter.rect_filled(r.rect, t.radius("md"), t.color("hover"));
            }
            let ink = if hot {
                t.color("foreground")
            } else {
                t.color("muted-foreground")
            };
            icons::paint(painter, r.rect.center(), Icon::Ellipsis, 16.0, ink);
            if r.has_focus() {
                painter.rect_stroke(
                    r.rect.expand(t.length("focus-ring-offset")),
                    t.radius("md"),
                    Stroke::new(t.length("focus-ring-width"), t.color("ring")),
                    StrokeKind::Outside,
                );
            }
        }
        if title_clamped && response.hovered() {
            let _ = response.clone().on_hover_text(self.title.clone());
        }

        // a small card with the title follows the pointer while it drags
        if out.dragging
            && let Some(p) = ui.input(|i| i.pointer.hover_pos())
        {
            egui::Area::new(id.with("carried"))
                .order(egui::Order::Tooltip)
                .fixed_pos(p + vec2(12.0, 12.0))
                .interactable(false)
                .show(ui.ctx(), |ui| {
                    egui::Frame::new()
                        .fill(t.color("card"))
                        .stroke(Stroke::new(1.0, t.color("border")))
                        .corner_radius(t.radius("surface"))
                        .shadow(t.shadow("elevation-pop"))
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            ui.label(TextRole::Badge.rich(&t, &self.title).color(t.color("foreground")));
                        });
                });
        }
        out
    }
}

/// The frame's focus outline round a round button.
fn focus_ring_round(painter: &Painter, t: &Tokens, rect: Rect, response: &Response) {
    if response.has_focus() {
        let offset = t.length("focus-ring-offset");
        painter.circle_stroke(
            rect.center(),
            rect.width() / 2.0 + offset + t.length("focus-ring-width") / 2.0,
            Stroke::new(t.length("focus-ring-width"), t.color("ring")),
        );
    }
}

struct Clamped {
    galley: Arc<Galley>,
    elided: bool,
}

/// The title in `text.label` on at most `lines` lines, cut with an ellipsis, as `line-clamp` does.
fn clamped(ui: &Ui, t: &Tokens, title: &str, width: f32, lines: usize) -> Clamped {
    let mut job = LayoutJob::default();
    TextRole::Label
        .rich(t, title)
        .append_to(&mut job, ui.style(), FontSelection::Default, Align::Min);
    job.wrap = TextWrapping {
        max_width: width,
        max_rows: lines,
        break_anywhere: false,
        overflow_character: Some('\u{2026}'),
    };
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let elided = galley.elided;
    Clamped { galley, elided }
}

/// Media cards in a grid that fills its width, after sokkel-ui-media's `MediaGrid`: as many columns
/// as fit, each at least `min` wide (152 for square covers, 240 for wide ones), 20 apart and the rows
/// 24 apart.
pub struct MediaGrid {
    shape: MediaShape,
    min: Option<f32>,
}

impl MediaGrid {
    pub fn new(shape: MediaShape) -> Self {
        Self { shape, min: None }
    }

    /// The narrowest a card is drawn.
    pub fn min(mut self, min: f32) -> Self {
        self.min = Some(min);
        self
    }

    /// The columns for a width, and each column's width.
    pub fn columns(&self, width: f32) -> (usize, f32) {
        let narrowest = self.min.unwrap_or(match self.shape {
            MediaShape::Square => 152.0,
            MediaShape::Wide => 240.0,
        });
        let gap = 20.0;
        let narrowest = narrowest.min(width);
        let count = (((width + gap) / (narrowest + gap)).floor() as usize).max(1);
        (count, (width - gap * (count - 1) as f32) / count as f32)
    }

    /// Lay out `count` cards, `card` drawing the one at an index into a `ui` as wide as its column.
    pub fn show(self, ui: &mut Ui, count: usize, mut card: impl FnMut(&mut Ui, usize)) {
        let width = ui.available_width();
        let (columns, column) = self.columns(width);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(20.0, 24.0);
            for start in (0..count).step_by(columns) {
                ui.horizontal_top(|ui| {
                    for index in start..(start + columns).min(count) {
                        ui.allocate_ui_with_layout(vec2(column, 0.0), egui::Layout::top_down(Align::Min), |ui| {
                            ui.set_width(column);
                            card(ui, index);
                        });
                    }
                });
            }
        });
    }
}
