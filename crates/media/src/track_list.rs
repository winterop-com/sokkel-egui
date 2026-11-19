//! A list of tracks or episodes, after sokkel-ui-media's `TrackList`, `ReorderableTrackList` and
//! `DraggableTrackList`, with the window a long list draws (`track-window`).

use std::sync::Arc;

use egui::{
    Align, Color32, CornerRadius, DragAndDrop, FontSelection, Galley, Id, Key, LayerId, Order, Painter, Rect, Response,
    RichText, Sense, Shape, Stroke, StrokeKind, Style, Ui, UiBuilder, pos2, text::LayoutJob, text::TextWrapping, vec2,
};

use sokkel_egui::{
    DragData, Icon, TextRole, Tokens, format_remaining, format_time, group_label, icons, mono_font, mono_size,
    paint_play, tokens,
};

use crate::artwork::{Artwork, ArtworkCover};
use crate::media_card::MediaShape;

/// One row of a track list.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub id: String,
    pub title: String,
    /// A second line under the title, such as the artist where it differs from the album's.
    pub subtitle: Option<String>,
    /// The second line in the code face, as a time is written.
    pub mono_subtitle: bool,
    /// The number at the row's start, such as the track's place on its album.
    pub number: Option<String>,
    /// A drawn cover at the row's start, in place of the number.
    pub artwork: Option<ArtworkCover>,
    /// A quiet column before the length, such as the album; it folds away when the list is narrow.
    pub meta: Option<String>,
    /// Length in seconds.
    pub duration: f64,
    /// The disc of a set it is on, counted from 1; a list over more than one disc is headed by each.
    pub disc: Option<u32>,
}

impl Track {
    pub fn new(id: impl Into<String>, title: impl Into<String>, duration: f64) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            subtitle: None,
            mono_subtitle: false,
            number: None,
            artwork: None,
            meta: None,
            duration,
            disc: None,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// The second line in the code face, as a chapter's start is written.
    pub fn mono_subtitle(mut self, mono: bool) -> Self {
        self.mono_subtitle = mono;
        self
    }

    pub fn number(mut self, number: impl ToString) -> Self {
        self.number = Some(number.to_string());
        self
    }

    pub fn artwork(mut self, cover: ArtworkCover) -> Self {
        self.artwork = Some(cover);
        self
    }

    pub fn meta(mut self, meta: impl Into<String>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    pub fn disc(mut self, disc: u32) -> Self {
        self.disc = Some(disc);
        self
    }
}

/// The words a track list says.
#[derive(Clone, Debug)]
pub struct TrackListWords {
    /// `Play {title}`: a row's name.
    pub play: String,
    pub playing: String,
    /// `Disc {disc}`: the head over a disc's tracks.
    pub disc: String,
}

impl Default for TrackListWords {
    fn default() -> Self {
        Self {
            play: "Play {title}".into(),
            playing: "Now playing".into(),
            disc: "Disc {disc}".into(),
        }
    }
}

/// The words a reorderable list says beyond a track list's.
#[derive(Clone, Debug)]
pub struct ReorderableTrackListWords {
    /// `Move {title}`: the grip's name.
    pub move_: String,
    /// `{title} moved to {place} of {count}`: said once a row is moved from the keyboard.
    pub moved: String,
}

impl Default for ReorderableTrackListWords {
    fn default() -> Self {
        Self {
            move_: "Move {title}".into(),
            moved: "{title} moved to {place} of {count}".into(),
        }
    }
}

/// The words a draggable list says beyond a track list's.
#[derive(Clone, Debug)]
pub struct DraggableTrackListWords {
    /// `Drag {title}`: the grip's name.
    pub drag: String,
}

impl Default for DraggableTrackListWords {
    fn default() -> Self {
        Self {
            drag: "Drag {title}".into(),
        }
    }
}

/// What the current row shows of how far it has played: the time left in place of its length, or a
/// line along its foot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackProgress {
    Left,
    Bar,
}

/// The list in its own bordered surface, or on the ground it is placed on, as a side panel wants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrackListVariant {
    #[default]
    Boxed,
    Plain,
}

/// A list longer than this draws only the rows in view, and a few around them, unless `windowed`
/// says otherwise.
pub const TRACK_WINDOW_FROM: usize = 200;

/// Rows drawn above and below those in view, so a quick scroll never shows an empty stretch.
const OVERSCAN: usize = 12;

/// Whether each track opens a disc's head: the first of each disc, on a list over more than one disc.
pub fn disc_heads(tracks: &[Track]) -> Vec<bool> {
    let disc = |t: &Track| t.disc.unwrap_or(1);
    let many = tracks.iter().any(|t| disc(t) != tracks.first().map_or(1, disc));
    tracks
        .iter()
        .enumerate()
        .map(|(i, t)| many && (i == 0 || disc(&tracks[i - 1]) != disc(t)))
        .collect()
}

/// Where each row of a windowed list starts, in pixels from the list's top, with the disc's head
/// over a row that opens one, and the list's whole height: rows `row` tall and heads `head` tall.
pub fn track_tops(heads: &[bool], row: f32, head: f32) -> (Vec<f32>, f32) {
    let mut tops = Vec::with_capacity(heads.len());
    let mut at = 0.0;
    for opens in heads {
        tops.push(at);
        at += if *opens { head } else { 0.0 } + row;
    }
    (tops, at)
}

/// The rows a windowed list draws for a view from `start`, `height` tall: those in it and a dozen
/// either side, as the first index and one past the last.
pub fn track_range(tops: &[f32], start: f32, height: f32) -> (usize, usize) {
    if tops.is_empty() {
        return (0, 0);
    }
    // the last row starting at or above the view's top, found by halving
    let (mut low, mut high) = (0, tops.len() - 1);
    while low < high {
        let middle = (low + high).div_ceil(2);
        if tops[middle] <= start {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let mut last = low;
    while last < tops.len() && tops[last] < start + height {
        last += 1;
    }
    (low.saturating_sub(OVERSCAN), (last + OVERSCAN).min(tops.len()))
}

/// What a track list was asked this frame.
#[derive(Clone, Debug)]
pub struct TrackListResponse {
    pub response: Response,
    /// The index of the row pressed to play (the web's `onPlay`).
    pub played: Option<usize>,
    /// The index of a linked row pressed to open its page (with `linked`, the web's `href`).
    pub opened: Option<usize>,
    /// A row moved from the first index to the second (a reorderable list's `onMove`).
    pub moved: Option<(usize, usize)>,
    /// The index of the row being dragged elsewhere (a draggable list); its [`DragData`] is egui's
    /// drag payload meanwhile.
    pub dragging: Option<usize>,
}

type RowActions<'a> = Box<dyn FnMut(&mut Ui, &Track, usize) + 'a>;

/// What a row ends in after its own actions.
enum Mode {
    Plain,
    Reorder(ReorderableTrackListWords),
    Drag {
        kind: String,
        words: DraggableTrackListWords,
    },
}

/// A reorder in hand: the row lifted, and where the pointer took it.
#[derive(Clone, Copy, Debug)]
struct Lifted {
    from: usize,
    grab: f32,
}

/// A list of tracks or episodes, after sokkel-ui-media's `TrackList`: the number or a small picture,
/// the title in `text.body` over an optional second line in `text.help`, a quiet meta column, and
/// the length in the code face at the end.
///
/// The `current` row is drawn on `secondary` with its title in `primary-ink`, its number replaced
/// by a speaker while `playing` and by a play glyph while paused. `Boxed`, the default, draws the
/// list on `card` in a `border-strong` ring with the surface's corners, its rows `border` apart;
/// `Plain` leaves it on the ground it is placed on. `artwork` sets the picture's shape. `actions`
/// draws a row's own controls at its end, after the length, such as a star and a menu; they stay
/// in view. Pointing at a row's track washes the row in `hover` while it plays on a press
/// (`playable`).
///
/// The current row is kept in view, brought to the middle of the scroller around the list when it
/// changes and is out of view; `follow(false)` leaves the scrolling to the reader. With the
/// `position` played into it, `progress` shows how far the current row has played. An album of
/// several discs gives each track its `disc`: the list then heads each disc it reaches.
///
/// A list longer than [`TRACK_WINDOW_FROM`] rows, or any with `windowed`, draws only the rows in
/// view of the scroller and a dozen either side; the current row is always drawn.
#[must_use = "show it with `.show(ui)`"]
pub struct TrackList<'a> {
    id: Id,
    tracks: &'a [Track],
    current: Option<&'a str>,
    playing: bool,
    position: Option<f64>,
    progress: Option<TrackProgress>,
    follow: bool,
    playable: bool,
    linked: bool,
    actions: Option<RowActions<'a>>,
    variant: TrackListVariant,
    artwork: MediaShape,
    windowed: Option<bool>,
    words: TrackListWords,
}

impl<'a> TrackList<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, tracks: &'a [Track]) -> Self {
        Self {
            id: Id::unique(id_salt),
            tracks,
            current: None,
            playing: false,
            position: None,
            progress: None,
            follow: true,
            playable: true,
            linked: false,
            actions: None,
            variant: TrackListVariant::Boxed,
            artwork: MediaShape::Square,
            windowed: None,
            words: TrackListWords::default(),
        }
    }

    /// The id of the row that is playing or paused.
    pub fn current(mut self, id: Option<&'a str>) -> Self {
        self.current = id;
        self
    }

    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// Seconds played into the current row, which `progress` shows.
    pub fn position(mut self, seconds: f64) -> Self {
        self.position = Some(seconds);
        self
    }

    pub fn progress(mut self, progress: TrackProgress) -> Self {
        self.progress = Some(progress);
        self
    }

    /// Whether the current row is kept in view as it changes; on by default.
    pub fn follow(mut self, follow: bool) -> Self {
        self.follow = follow;
        self
    }

    /// Whether a press on a row plays it (the web's `onPlay`); on by default.
    pub fn playable(mut self, playable: bool) -> Self {
        self.playable = playable;
        self
    }

    /// Each row opens its own page on a press (the web's `href`), its number or picture the button
    /// that plays it.
    pub fn linked(mut self, linked: bool) -> Self {
        self.linked = linked;
        self
    }

    /// A row's own controls, drawn left to right at its end.
    pub fn actions(mut self, actions: impl FnMut(&mut Ui, &Track, usize) + 'a) -> Self {
        self.actions = Some(Box::new(actions));
        self
    }

    pub fn variant(mut self, variant: TrackListVariant) -> Self {
        self.variant = variant;
        self
    }

    /// The pictures' shape: square for covers, wide for video frames.
    pub fn artwork(mut self, shape: MediaShape) -> Self {
        self.artwork = shape;
        self
    }

    /// Whether only the rows in view are drawn; by default once the list is longer than
    /// [`TRACK_WINDOW_FROM`].
    pub fn windowed(mut self, windowed: bool) -> Self {
        self.windowed = Some(windowed);
        self
    }

    pub fn words(mut self, words: TrackListWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> TrackListResponse {
        self.show_as(ui, Mode::Plain)
    }

    fn show_as(mut self, ui: &mut Ui, mode: Mode) -> TrackListResponse {
        let t = tokens(ui.ctx());
        let tracks = self.tracks;
        let count = tracks.len();
        let boxed = self.variant == TrackListVariant::Boxed;
        let reorder = matches!(mode, Mode::Reorder(_));
        let heads = if reorder {
            vec![false; count]
        } else {
            disc_heads(tracks)
        };
        let line = if boxed { 1.0 } else { 0.0 };
        let head_height = 10.0 + TextRole::Help.line_height(&t) + 6.0 + line;
        let heights: Vec<f32> = tracks
            .iter()
            .enumerate()
            .map(|(i, track)| row_height(&t, track) + if i + 1 < count { line } else { 0.0 })
            .collect();
        // where each row's head (or the row) starts, from the list's top inside its ring
        let mut tops = Vec::with_capacity(count);
        let mut at = 0.0;
        for i in 0..count {
            tops.push(at);
            at += if heads[i] { head_height } else { 0.0 } + heights[i];
        }
        let inner = at;
        let width = ui.available_width();
        let (outer, response) = ui.allocate_exact_size(vec2(width, inner + 2.0 * line), Sense::hover());
        let radius = t.radius("surface");
        if boxed {
            ui.painter().rect(
                outer,
                radius,
                t.color("card"),
                Stroke::new(1.0, t.color("border-strong")),
                StrokeKind::Inside,
            );
        }
        let origin = outer.top() + line;
        let row_top = |i: usize| origin + tops[i] + if heads[i] { head_height } else { 0.0 };
        let current = self
            .current
            .and_then(|id| tracks.iter().position(|track| track.id == id));

        // the rows to draw: all, or those in view and a dozen either side
        let windowed = !reorder && self.windowed.unwrap_or(count > TRACK_WINDOW_FROM);
        let mut shown: Vec<usize> = if windowed {
            let clip = ui.clip_rect();
            let (first, last) = track_range(&tops, clip.top() - origin, clip.height());
            (first..last).collect()
        } else {
            (0..count).collect()
        };
        if let Some(c) = current
            && !shown.contains(&c)
        {
            shown.push(c);
            shown.sort_unstable();
        }

        let mut out = TrackListResponse {
            response,
            played: None,
            opened: None,
            moved: None,
            dragging: None,
        };

        // a reorder in hand: the row lifted, where it would land, and the rows moved aside
        let lifted_id = self.id.with("lifted");
        let mut lifted: Option<Lifted> = ui.data(|d| d.get_temp(lifted_id));
        if lifted.is_some_and(|l| l.from >= count) {
            lifted = None;
        }
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let landing = lifted.zip(pointer).map(|(l, p)| {
            let center = p.y - l.grab + heights[l.from] / 2.0;
            (0..count)
                .filter(|j| *j != l.from)
                .filter(|j| row_top(*j) + heights[*j] / 2.0 < center)
                .count()
        });
        let held = lifted;
        let sizes = &heights;
        let shift = move |i: usize| -> f32 {
            let heights = sizes;
            match (held, landing) {
                (Some(l), Some(to)) if i != l.from => {
                    if l.from < i && i <= to {
                        -heights[l.from]
                    } else if to <= i && i < l.from {
                        heights[l.from]
                    } else {
                        0.0
                    }
                }
                _ => 0.0,
            }
        };

        let widths_id = self.id.with("action-widths");
        let mut widths: Vec<f32> = ui.data(|d| d.get_temp(widths_id)).unwrap_or_default();
        widths.resize(count, 0.0);
        let has_actions = self.actions.is_some() || !matches!(mode, Mode::Plain);
        let pad = if boxed { t.density("panel-pad") } else { 8.0 };
        let narrow = width < 448.0;
        let style = ui.style().clone();

        for &i in &shown {
            let track = &tracks[i];
            if heads[i] {
                let head = Rect::from_min_size(pos2(outer.left(), origin + tops[i]), vec2(width, head_height));
                if ui.is_rect_visible(head) {
                    if boxed {
                        ui.painter()
                            .hline(head.x_range(), head.bottom() - 0.5, Stroke::new(1.0, t.color("border")));
                    }
                    let left = if boxed { pad } else { 8.0 };
                    let top = if i > 0 && !boxed { 16.0 } else { 10.0 };
                    let label = Rect::from_min_max(
                        pos2(head.left() + left, head.top() + top),
                        pos2(head.right() - 8.0, head.bottom() - 6.0 - line),
                    );
                    let mut child = ui.new_child(UiBuilder::new().max_rect(label));
                    let disc = track.disc.unwrap_or(1).to_string();
                    group_label(&mut child, &self.words.disc.replace("{disc}", &disc));
                }
            }
            let height = heights[i];
            let rect = Rect::from_min_size(
                pos2(outer.left() + line, row_top(i) + shift(i)),
                vec2(width - 2.0 * line, height - if i + 1 < count { line } else { 0.0 }),
            );
            if lifted.is_some_and(|l| l.from == i) {
                // its place is held empty while the copy follows the pointer
                continue;
            }
            let visible = ui.is_rect_visible(rect);
            let now = current == Some(i);
            let row_id = self.id.with(("row", &track.id));
            if boxed && i + 1 < count && visible {
                ui.painter()
                    .hline(rect.x_range(), rect.bottom() + 0.5, Stroke::new(1.0, t.color("border")));
            }
            let wash = ui.painter().add(Shape::Noop);

            // the row's own controls at its end, then the grip
            let right = rect.right() - 8.0;
            let mut body_right = right;
            let mut grip: Option<Response> = None;
            if has_actions && visible {
                let room = widths[i];
                let area = Rect::from_min_max(pos2(right - room, rect.top()), pos2(right, rect.bottom()));
                let mut child = ui.new_child(
                    UiBuilder::new()
                        .id_salt(("actions", &track.id))
                        .max_rect(area.with_max_x(right + 400.0))
                        .layout(egui::Layout::left_to_right(Align::Center)),
                );
                child.spacing_mut().item_spacing.x = 2.0;
                if let Some(actions) = self.actions.as_mut() {
                    actions(&mut child, track, i);
                }
                match &mode {
                    Mode::Plain => {}
                    Mode::Reorder(words) => {
                        grip = Some(grip_button(
                            &mut child,
                            &t,
                            row_id,
                            &words.move_.replace("{title}", &track.title),
                        ));
                    }
                    Mode::Drag { words, .. } if !narrow => {
                        grip = Some(grip_button(
                            &mut child,
                            &t,
                            row_id,
                            &words.drag.replace("{title}", &track.title),
                        ));
                    }
                    Mode::Drag { .. } => {}
                }
                let used = child.min_rect().width().max(0.0);
                widths[i] = if child.min_rect().is_positive() { used } else { 0.0 };
                if widths[i] > 0.0 {
                    body_right = right - widths[i] - 8.0;
                }
                if (used - room).abs() > 0.5 {
                    ui.ctx().request_repaint();
                }
            }
            if !visible && !now {
                continue;
            }

            // the body: the button that plays it, or the link and its lead
            let body = Rect::from_min_max(rect.min, pos2(body_right, rect.bottom()));
            let dragging_rows = matches!(mode, Mode::Drag { .. });
            let label = self.words.play.replace("{title}", &track.title);
            let lead_rect = lead_rect(track, self.artwork, rect, pad);
            let (body_response, lead_response) = if self.linked {
                let lead_hit = Rect::from_min_max(rect.min, pos2(lead_rect.right() + 6.0, rect.bottom()));
                let lead = self
                    .playable
                    .then(|| ui.interact(lead_hit, row_id.with("lead"), Sense::click()));
                let link = Rect::from_min_max(pos2(lead_hit.right(), rect.top()), body.max);
                let sense = if dragging_rows {
                    Sense::click_and_drag()
                } else {
                    Sense::click()
                };
                (ui.interact(link, row_id.with("body"), sense), lead)
            } else {
                let sense = match (self.playable, dragging_rows) {
                    (_, true) => Sense::click_and_drag(),
                    (true, false) => Sense::click(),
                    (false, false) => Sense::hover(),
                };
                (ui.interact(body, row_id.with("body"), sense), None)
            };
            body_response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
            if let Some(lead) = &lead_response {
                lead.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
                if lead.clicked() {
                    out.played = Some(i);
                }
            }
            if body_response.clicked() {
                if self.linked {
                    out.opened = Some(i);
                } else if self.playable {
                    out.played = Some(i);
                }
            }
            let pressable = self.playable || self.linked;
            if pressable && (body_response.hovered() || lead_response.as_ref().is_some_and(Response::hovered)) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }

            // a draggable list's row carries its track elsewhere
            let mut faded = false;
            if let Mode::Drag { kind, .. } = &mode {
                let data = DragData {
                    kind: kind.clone(),
                    label: track.title.clone(),
                    duration: Some(track.duration),
                    value: track.id.clone(),
                };
                let started = body_response.drag_started() || grip.as_ref().is_some_and(Response::drag_started);
                if started {
                    DragAndDrop::set_payload(ui.ctx(), data);
                }
                let held = body_response.dragged() || grip.as_ref().is_some_and(Response::dragged);
                if held && DragAndDrop::has_payload_of_type::<DragData>(ui.ctx()) {
                    out.dragging = Some(i);
                    faded = true;
                    carried(ui, &t, row_id, &track.title);
                }
            }

            // a reorderable list's grip lifts the row, and its arrows move it a place
            if let (Mode::Reorder(_), Some(g)) = (&mode, &grip) {
                if g.drag_started()
                    && let Some(p) = pointer
                {
                    lifted = Some(Lifted {
                        from: i,
                        grab: p.y - rect.top(),
                    });
                }
                if g.has_focus() {
                    let (up, down) = ui.input_mut(|input| {
                        (
                            input.consume_key(egui::Modifiers::NONE, Key::ArrowUp),
                            input.consume_key(egui::Modifiers::NONE, Key::ArrowDown),
                        )
                    });
                    if up && i > 0 {
                        out.moved = Some((i, i - 1));
                    } else if down && i + 1 < count {
                        out.moved = Some((i, i + 1));
                    }
                }
            }

            let hot = pressable && (body_response.hovered() || lead_response.as_ref().is_some_and(Response::hovered));
            let ground = if now {
                Some(t.color("secondary"))
            } else if hot || body_response.has_focus() {
                Some(t.color("hover"))
            } else {
                None
            };
            if let Some(ground) = ground {
                let corners = row_corners(&t, boxed, i, count);
                ui.painter().set(wash, Shape::rect_filled(rect, corners, ground));
            }
            let mut painter = ui.painter().clone();
            if faded {
                painter.multiply_opacity(0.4);
            }
            let row = RowPaint {
                track,
                now,
                playing: now && self.playing,
                played: if now { self.position } else { None },
                progress: self.progress,
                shape: self.artwork,
                pad,
                list_width: width,
            };
            row.paint(&painter, &style, &t, rect, body_right);

            // the current row is brought into view when it changes
            if now && self.follow {
                let followed_id = self.id.with("followed");
                let followed: Option<String> = ui.data(|d| d.get_temp(followed_id));
                if followed.as_deref() != Some(track.id.as_str()) {
                    if !ui.clip_rect().contains_rect(rect) {
                        ui.scroll_to_rect(rect, Some(Align::Center));
                    }
                    ui.data_mut(|d| d.insert_temp(followed_id, track.id.clone()));
                }
            }
        }

        // the lifted copy follows the pointer, and lands where it is let go
        if let (Some(l), Some(p)) = (lifted, pointer) {
            let track = &tracks[l.from];
            let height = heights[l.from] - if l.from + 1 < count { line } else { 0.0 };
            let rect = Rect::from_min_size(pos2(outer.left(), p.y - l.grab), vec2(width, height));
            let layer = LayerId::new(Order::Tooltip, self.id.with("lifted-layer"));
            let painter = ui.ctx().layer_painter(layer);
            painter.add(t.shadow("elevation-pop").as_shape(rect, t.radius("surface")));
            painter.rect(
                rect,
                t.radius("surface"),
                t.color("card"),
                Stroke::new(1.0, t.color("border")),
                StrokeKind::Inside,
            );
            let row = RowPaint {
                track,
                now: current == Some(l.from),
                playing: current == Some(l.from) && self.playing,
                played: None,
                progress: None,
                shape: self.artwork,
                pad: 8.0,
                list_width: width,
            };
            let right = rect.right() - 8.0 - widths[l.from] - 8.0;
            row.paint(&painter, &style, &t, rect, right);
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            let (released, escaped) = ui.input(|i| (i.pointer.any_released(), i.key_pressed(Key::Escape)));
            if released || escaped {
                if released
                    && let Some(to) = landing
                    && to != l.from
                {
                    out.moved = Some((l.from, to));
                }
                lifted = None;
            }
        }
        ui.data_mut(|d| {
            d.insert_temp(widths_id, widths);
            match lifted {
                Some(l) => {
                    d.insert_temp(lifted_id, l);
                }
                None => d.remove::<Lifted>(lifted_id),
            }
        });
        out
    }
}

/// A track list the listener puts in their own order, such as a playlist or what a queue plays
/// next, after sokkel-ui-media's `ReorderableTrackList`: a [`TrackList`] whose rows each end in a
/// grip after their own actions. Dragging the grip lifts a copy of the row and moves the rest aside
/// to show where it lands; letting go reports the move, and Escape puts it back. With the grip
/// focused the up and down arrows move the row one place, the focus following it. It draws no disc
/// heads and every row however long the list.
#[must_use = "show it with `.show(ui)`"]
pub struct ReorderableTrackList<'a> {
    list: TrackList<'a>,
    words: ReorderableTrackListWords,
}

impl<'a> ReorderableTrackList<'a> {
    pub fn new(list: TrackList<'a>) -> Self {
        Self {
            list,
            words: ReorderableTrackListWords::default(),
        }
    }

    pub fn words(mut self, words: ReorderableTrackListWords) -> Self {
        self.words = words;
        self
    }

    /// Shows the list; `moved` holds a move, both indexes into the tracks.
    pub fn show(self, ui: &mut Ui) -> TrackListResponse {
        self.list.show_as(ui, Mode::Reorder(self.words))
    }
}

/// A track list whose rows are dragged elsewhere, such as a song onto a playlist in the rail, after
/// sokkel-ui-media's `DraggableTrackList`: each row drags as a whole once the pointer moves a few
/// pixels, the title on a small card following it, carrying a [`DragData`] of `kind` whose value is
/// the track's id; a grip ends each row, folded away on a narrow list. A press still plays the row.
/// The list keeps its own order.
#[must_use = "show it with `.show(ui)`"]
pub struct DraggableTrackList<'a> {
    list: TrackList<'a>,
    kind: String,
    words: DraggableTrackListWords,
}

impl<'a> DraggableTrackList<'a> {
    pub fn new(list: TrackList<'a>, kind: impl Into<String>) -> Self {
        Self {
            list,
            kind: kind.into(),
            words: DraggableTrackListWords::default(),
        }
    }

    pub fn words(mut self, words: DraggableTrackListWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> TrackListResponse {
        self.list.show_as(
            ui,
            Mode::Drag {
                kind: self.kind,
                words: self.words,
            },
        )
    }
}

/// A row's height: a row of the density at least, and room for its picture or its two lines with
/// 6 above and below.
fn row_height(t: &Tokens, track: &Track) -> f32 {
    let words = TextRole::Body.line_height(t)
        + if track.subtitle.is_some() {
            TextRole::Help.line_height(t)
        } else {
            0.0
        };
    let picture = if track.artwork.is_some() { 36.0 } else { 0.0 };
    t.density("row").max(words.max(picture) + 12.0)
}

/// The rounding a row's wash takes: the box's corners on its first and last row.
fn row_corners(t: &Tokens, boxed: bool, i: usize, count: usize) -> CornerRadius {
    if !boxed {
        return t.radius("control");
    }
    let r = t.radius("surface").nw.saturating_sub(1);
    CornerRadius {
        nw: if i == 0 { r } else { 0 },
        ne: if i == 0 { r } else { 0 },
        sw: if i + 1 == count { r } else { 0 },
        se: if i + 1 == count { r } else { 0 },
    }
}

/// Where a row's lead sits: its picture (36 square, or 64 by 36 wide) or its number's 24.
fn lead_rect(track: &Track, shape: MediaShape, row: Rect, pad: f32) -> Rect {
    let size = match (track.artwork.is_some(), shape) {
        (true, MediaShape::Square) => vec2(36.0, 36.0),
        (true, MediaShape::Wide) => vec2(64.0, 36.0),
        (false, _) => vec2(24.0, 20.0),
    };
    Rect::from_min_size(pos2(row.left() + pad, row.center().y - size.y / 2.0), size)
}

/// The grip at a row's end: a ghost `icon-sm` square holding Lucide's grip in the muted ink, which
/// drags.
fn grip_button(ui: &mut Ui, t: &Tokens, row: Id, label: &str) -> Response {
    let side = t.density("control-sm");
    let (rect, _) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
    let response = ui.interact(rect, row.with("grip"), Sense::click_and_drag());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    let hot = response.hovered() || response.dragged();
    let radius = CornerRadius::same(t.radius("control").nw.min(12));
    if hot {
        ui.painter().rect_filled(rect, radius, t.color("hover"));
        ui.ctx().set_cursor_icon(if response.dragged() {
            egui::CursorIcon::Grabbing
        } else {
            egui::CursorIcon::Grab
        });
    }
    let ink = if hot {
        t.color("foreground")
    } else {
        t.color("muted-foreground")
    };
    icons::paint(ui.painter(), rect.center(), Icon::GripVertical, 16.0, ink);
    if response.has_focus() {
        let offset = t.length("focus-ring-offset");
        ui.painter().rect_stroke(
            rect.expand(offset),
            radius,
            Stroke::new(t.length("focus-ring-width"), t.color("ring")),
            StrokeKind::Outside,
        );
    }
    response.on_hover_text(label.to_owned())
}

/// The small card with a dragged row's title, following the pointer.
fn carried(ui: &Ui, t: &Tokens, id: Id, title: &str) {
    let Some(p) = ui.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    egui::Area::new(id.with("carried"))
        .order(Order::Tooltip)
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
                    ui.label(TextRole::Badge.rich(t, title).color(t.color("foreground")));
                });
        });
}

/// `rich` on one line at most `width` wide, cut with an ellipsis.
pub(crate) fn one_line(painter: &Painter, style: &Style, rich: RichText, width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::default();
    rich.append_to(&mut job, style, FontSelection::Default, Align::Min);
    job.wrap = TextWrapping {
        max_width: width.max(1.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('\u{2026}'),
    };
    painter.layout_job(job)
}

/// What a row draws, apart from its controls.
struct RowPaint<'t> {
    track: &'t Track,
    now: bool,
    playing: bool,
    played: Option<f64>,
    progress: Option<TrackProgress>,
    shape: MediaShape,
    pad: f32,
    list_width: f32,
}

impl RowPaint<'_> {
    /// The lead, the two lines, the meta column and the length, between the row's start and
    /// `right`; with `bar`, how far it has played along its foot.
    fn paint(&self, painter: &Painter, style: &Style, t: &Tokens, rect: Rect, right: f32) {
        let track = self.track;
        let lead = lead_rect(track, self.shape, rect, self.pad);
        let mark = |painter: &Painter, center: egui::Pos2, ink: Color32| {
            if self.playing {
                icons::paint(painter, center, Icon::Volume2, 14.0, ink);
            } else {
                paint_play(painter, center + vec2(0.5, 0.0), 14.0, ink);
            }
        };
        if let Some(cover) = track.artwork {
            let radius = t.radius("mini");
            painter.rect_filled(lead, radius, t.color("muted"));
            Artwork::of(cover)
                .wide(self.shape == MediaShape::Wide)
                .paint(painter, lead);
            if self.now {
                painter.rect_filled(lead, radius, t.color("scrim"));
                mark(painter, lead.center(), t.color("terminal-foreground"));
            }
        } else if self.now {
            mark(
                painter,
                pos2(lead.right() - 7.0, lead.center().y),
                t.color("primary-ink"),
            );
        } else if let Some(number) = &track.number {
            let galley = painter.layout_no_wrap(number.clone(), TextRole::Help.font(t), t.color("muted-foreground"));
            painter.galley(
                pos2(lead.right() - galley.size().x, rect.center().y - galley.size().y / 2.0),
                galley,
                t.color("muted-foreground"),
            );
        }

        // the length at the end, at least 48 wide
        let mono = mono_font(painter.ctx(), mono_size(t, t.text("xs").size), 400.0);
        let length = painter.layout_no_wrap(
            match (self.progress, self.played) {
                (Some(TrackProgress::Left), Some(at)) => format_remaining(at, track.duration),
                _ => format_time(track.duration),
            },
            mono.clone(),
            t.color("muted-foreground"),
        );
        let length_left = right - length.size().x.max(48.0);
        painter.galley(
            pos2(right - length.size().x, rect.center().y - length.size().y / 2.0),
            length,
            t.color("muted-foreground"),
        );

        // the quiet meta column, once the list is as wide as `@xl`
        let text_left = lead.right() + 12.0;
        let mut words_right = length_left - 12.0;
        if let Some(meta) = &track.meta
            && self.list_width >= 576.0
        {
            let room = ((right - text_left) * 0.28).max(0.0);
            let left = length_left - 12.0 - room;
            let galley = one_line(
                painter,
                style,
                TextRole::Body.rich(t, meta).color(t.color("muted-foreground")),
                room,
            );
            painter.galley(
                pos2(left, rect.center().y - galley.size().y / 2.0),
                galley,
                t.color("muted-foreground"),
            );
            words_right = left - 12.0;
        }

        // the title over its second line
        let room = (words_right - text_left).max(0.0);
        let title = if self.now {
            TextRole::Body
                .rich(t, &track.title)
                .variation("wght", 500.0)
                .color(t.color("primary-ink"))
        } else {
            TextRole::Body.rich(t, &track.title).color(t.color("foreground"))
        };
        let title = one_line(painter, style, title, room);
        let sub = track.subtitle.as_ref().map(|s| {
            let rich = if track.mono_subtitle {
                egui::RichText::new(s)
                    .font(mono)
                    .line_height(Some(TextRole::Help.line_height(t)))
            } else {
                TextRole::Help.rich(t, s)
            };
            one_line(painter, style, rich.color(t.color("muted-foreground")), room)
        });
        let block = TextRole::Body.line_height(t) + sub.as_ref().map_or(0.0, |_| TextRole::Help.line_height(t));
        let top = rect.center().y - block / 2.0;
        painter.galley(pos2(text_left, top), title, t.color("foreground"));
        if let Some(sub) = sub {
            painter.galley(
                pos2(text_left, top + TextRole::Body.line_height(t)),
                sub,
                t.color("muted-foreground"),
            );
        }

        if self.progress == Some(TrackProgress::Bar)
            && let Some(at) = self.played
            && track.duration > 0.0
        {
            let share = (at / track.duration).clamp(0.0, 1.0) as f32;
            let bar = Rect::from_min_size(pos2(rect.left(), rect.bottom() - 2.0), vec2(rect.width() * share, 2.0));
            painter.rect_filled(bar, CornerRadius::ZERO, t.color("primary"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set() -> Vec<Track> {
        vec![
            Track::new("1-1", "Low Water", 214.0).number(1).disc(1),
            Track::new("1-2", "Harbor Wall", 247.0).number(2).disc(1),
            Track::new("2-1", "Spring Tide", 198.0).number(1).disc(2),
            Track::new("2-2", "High Water", 312.0).number(2).disc(2),
        ]
    }

    #[test]
    fn heads_each_disc_of_a_set_where_its_tracks_begin() {
        assert_eq!(disc_heads(&set()), [true, false, true, false]);
    }

    #[test]
    fn draws_no_head_for_a_list_on_one_disc_numbered_or_not() {
        assert_eq!(disc_heads(&set()[..2]), [false, false]);
        let plain: Vec<Track> = set().into_iter().map(|t| Track { disc: None, ..t }).collect();
        assert_eq!(disc_heads(&plain), [false; 4]);
    }

    #[test]
    fn counts_a_track_without_a_disc_as_on_the_first() {
        let mut mixed = vec![Track {
            disc: None,
            ..set()[0].clone()
        }];
        mixed.extend_from_slice(&set()[2..]);
        assert_eq!(disc_heads(&mixed), [true, true, false]);
    }

    #[test]
    fn places_each_row_under_the_head_of_the_disc_it_opens() {
        let (tops, height) = track_tops(&[true, false, true, false], 48.0, 36.0);
        assert_eq!(tops, [0.0, 84.0, 132.0, 216.0]);
        assert_eq!(height, 264.0);
    }

    #[test]
    fn draws_the_rows_in_view_and_a_dozen_either_side() {
        let (tops, _) = track_tops(&[false; 1000], 50.0, 0.0);
        assert_eq!(track_range(&tops, 0.0, 400.0), (0, 20));
        assert_eq!(track_range(&tops, 10_000.0, 400.0), (188, 220));
        assert_eq!(track_range(&tops, -300.0, 400.0), (0, 14));
        assert_eq!(track_range(&tops, 49_900.0, 400.0), (986, 1000));
    }
}
