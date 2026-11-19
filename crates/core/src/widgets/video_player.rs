//! A video's player, after sokkel-ui's `media/VideoPlayer`: the picture at sixteen by nine on the
//! terminal ground, the cue of the captions over it, and the controls along its foot.

use egui::{
    Align, Color32, CornerRadius, Layout, Mesh, Painter, Rect, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder,
    WidgetInfo, pos2, vec2,
};

use super::media_slider::MediaTone;
use super::{
    DropdownMenu, MenuItem, PlayButton, PlayWords, Position, RateMenu, TransportButton, TransportSize, VolumeControl,
    VolumeWords,
};
use crate::captions::{CaptionStyle, caption_ground, caption_ink, caption_text_size};
use crate::{Icon, TextRole, icons, mono_font, mono_size, tokens};

/// The words the player says.
#[derive(Clone, Debug)]
pub struct VideoPlayerWords {
    pub play: String,
    pub pause: String,
    pub mute: String,
    pub unmute: String,
    pub volume: String,
    pub seek: String,
    pub speed: String,
    /// The quality menu's name.
    pub quality: String,
    pub captions: String,
    pub captions_off: String,
    pub picture_in_picture: String,
    pub exit_picture_in_picture: String,
    pub fullscreen: String,
    pub exit_fullscreen: String,
    pub back: String,
    pub forward: String,
}

impl Default for VideoPlayerWords {
    fn default() -> Self {
        Self {
            play: "Play".into(),
            pause: "Pause".into(),
            mute: "Mute".into(),
            unmute: "Unmute".into(),
            volume: "Volume".into(),
            seek: "Seek".into(),
            speed: "Playback speed".into(),
            quality: "Quality".into(),
            captions: "Captions".into(),
            captions_off: "Off".into(),
            picture_in_picture: "Picture in picture".into(),
            exit_picture_in_picture: "Exit picture in picture".into(),
            fullscreen: "Fullscreen".into(),
            exit_fullscreen: "Exit fullscreen".into(),
            back: "Back 10 seconds".into(),
            forward: "Forward 10 seconds".into(),
        }
    }
}

/// One rendition the viewer may pick, such as Auto or 720p: its value, what the menu and its button
/// say, and a second word in the menu, such as its bitrate.
#[derive(Clone, Debug, PartialEq)]
pub struct VideoQuality {
    pub value: String,
    pub label: String,
    pub detail: Option<String>,
}

/// What the player was asked this frame.
#[derive(Clone, Debug, Default)]
pub struct VideoPlayerResponse {
    pub response: Option<egui::Response>,
    /// Play or pause was asked: the picture pressed, the play button, or the big one.
    pub playing: Option<bool>,
    /// A second to go to: the position let go, or a skip.
    pub seek: Option<f64>,
    /// The second under the pointer while the position is dragged.
    pub scrub: Option<f64>,
    pub volume: Option<f32>,
    pub muted: Option<bool>,
    pub rate: Option<f64>,
    /// The quality picked, by its value.
    pub quality: Option<String>,
    /// The captions' track picked: `Some(None)` turned them off.
    pub track: Option<Option<usize>>,
    pub picture_in_picture: Option<bool>,
    pub fullscreen: Option<bool>,
}

type Paint<'a> = Box<dyn Fn(&Painter, Rect) + 'a>;
type Part<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// How long a skip of the back and forward buttons is, in seconds.
pub const VIDEO_SKIP: f64 = 10.0;

/// A video's player, after sokkel-ui's `VideoPlayer`: the picture the app paints (`picture`, the
/// poster before it starts and the frame at the time after) at sixteen by nine on the `terminal`
/// ground with the surface's corners, the cue of the captions over it in the `caption-*` tokens, and
/// the controls along its foot on a fade from `terminal/90`: the position, then the title at the
/// start, back, play and forward in the middle, and the volume, the captions, the speed, the
/// quality, the app's own `actions`, picture in picture and full screen at the end. The controls
/// show while it is paused or pointed at. A `cover` (what plays next, over a dimmed picture) takes
/// the place of the controls, and an `overlay` (the stats) stands at the top start. The player
/// holds no time of its own: the app moves it and hears what was asked in the response.
#[must_use = "show it with `.show(ui)`"]
pub struct VideoPlayer<'a> {
    id_salt: egui::Id,
    picture: Option<Paint<'a>>,
    title: Option<String>,
    time: f64,
    duration: f64,
    playing: bool,
    started: bool,
    volume: f32,
    muted: bool,
    rate: f64,
    rates: &'a [f64],
    qualities: &'a [VideoQuality],
    quality: Option<String>,
    tracks: &'a [String],
    track: Option<usize>,
    cue: Option<String>,
    caption_style: CaptionStyle,
    actions: Option<Part<'a>>,
    cover: Option<Part<'a>>,
    overlay: Option<Part<'a>>,
    picture_in_picture: Option<bool>,
    fullscreen: bool,
    height: Option<f32>,
    words: VideoPlayerWords,
}

impl<'a> VideoPlayer<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt) -> Self {
        Self {
            id_salt: egui::Id::unique(id_salt),
            picture: None,
            title: None,
            time: 0.0,
            duration: 0.0,
            playing: false,
            started: false,
            volume: 1.0,
            muted: false,
            rate: 1.0,
            rates: &super::MEDIA_RATES,
            qualities: &[],
            quality: None,
            tracks: &[],
            track: None,
            cue: None,
            caption_style: crate::captions::CAPTION_STYLE,
            actions: None,
            cover: None,
            overlay: None,
            picture_in_picture: None,
            fullscreen: false,
            height: None,
            words: VideoPlayerWords::default(),
        }
    }

    /// Paints the picture into the frame: the poster before it starts, the frame at the time after.
    pub fn picture(mut self, paint: impl Fn(&Painter, Rect) + 'a) -> Self {
        self.picture = Some(Box::new(paint));
        self
    }

    /// The title at the controls' start.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Where it is and how long it is, in seconds.
    pub fn position(mut self, time: f64, duration: f64) -> Self {
        self.time = time;
        self.duration = duration;
        self
    }

    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// Whether it has played at all; until then the big play button stands over a hidden bar.
    pub fn started(mut self, started: bool) -> Self {
        self.started = started;
        self
    }

    pub fn volume(mut self, volume: f32, muted: bool) -> Self {
        self.volume = volume;
        self.muted = muted;
        self
    }

    pub fn rate(mut self, rate: f64) -> Self {
        self.rate = rate;
        self
    }

    pub fn rates(mut self, rates: &'a [f64]) -> Self {
        self.rates = rates;
        self
    }

    /// The renditions offered, and the one chosen by its value.
    pub fn qualities(mut self, qualities: &'a [VideoQuality], chosen: impl Into<String>) -> Self {
        self.qualities = qualities;
        self.quality = Some(chosen.into());
        self
    }

    /// The captions' tracks by their labels, and the one showing.
    pub fn tracks(mut self, tracks: &'a [String], showing: Option<usize>) -> Self {
        self.tracks = tracks;
        self.track = showing;
        self
    }

    /// The cue showing now, and how the captions look.
    pub fn cue(mut self, cue: Option<String>, style: CaptionStyle) -> Self {
        self.cue = cue.filter(|cue| !cue.is_empty());
        self.caption_style = style;
        self
    }

    /// The app's own buttons among the controls, before picture in picture, such as theater mode.
    pub fn actions(mut self, actions: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.actions = Some(Box::new(actions));
        self
    }

    /// Over the dimmed picture in the controls' place, centered, such as `UpNext`.
    pub fn cover(mut self, cover: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.cover = Some(Box::new(cover));
        self
    }

    /// At the picture's top start, such as `VideoStatsOverlay`.
    pub fn overlay(mut self, overlay: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.overlay = Some(Box::new(overlay));
        self
    }

    /// Offers picture in picture, on or off.
    pub fn picture_in_picture(mut self, on: bool) -> Self {
        self.picture_in_picture = Some(on);
        self
    }

    /// Filling the screen: no corners, as tall as it is given.
    pub fn fullscreen(mut self, fullscreen: bool) -> Self {
        self.fullscreen = fullscreen;
        self
    }

    /// As tall as this, the picture kept at its shape in the middle; sixteen by nine of the width
    /// by default.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    pub fn words(mut self, words: VideoPlayerWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(mut self, ui: &mut Ui) -> VideoPlayerResponse {
        let t = tokens(ui.ctx());
        let cover = self.cover.take();
        let overlay = self.overlay.take();
        let mut out = VideoPlayerResponse::default();
        let width = ui.available_width();
        let height = self.height.unwrap_or(width * 9.0 / 16.0);
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
        let id = ui.make_persistent_id(self.id_salt);
        response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, self.title.as_deref().unwrap_or("")));
        let radius = if self.fullscreen {
            CornerRadius::ZERO
        } else {
            t.radius("surface")
        };
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, radius, t.color("terminal"));
        // the picture at its own shape in the middle, as `object-contain` holds it
        let frame = {
            let fit = (rect.width() / 16.0).min(rect.height() / 9.0);
            Rect::from_center_size(rect.center(), vec2(fit * 16.0, fit * 9.0))
        };
        if let Some(paint) = &self.picture {
            let inner = painter.with_clip_rect(frame.intersect(rect));
            paint(&inner, frame);
        }

        let covered = cover.is_some();
        let pointed = ui.rect_contains_pointer(rect);
        let menu_open = ui.data(|d| d.get_temp::<bool>(id.with("menu"))).unwrap_or(false);
        let shown = !covered && (!self.playing || pointed || menu_open);

        // a press on the picture plays or pauses, two fill the screen
        if !covered {
            if response.double_clicked() {
                out.fullscreen = Some(!self.fullscreen);
            } else if response.clicked() {
                out.playing = Some(!self.playing);
            }
        }

        // the cue, 64 over the foot while the controls show and 2.5% of the width (16 at the least)
        // while they hide
        if let Some(cue) = &self.cue {
            let style = &self.caption_style;
            let size = caption_text_size(&t, style, rect.width());
            let font = egui::FontId::new(size, TextRole::Body.font(&t).family);
            let galley = painter.layout(cue.clone(), font, caption_ink(&t, style), rect.width() * 0.85);
            let bottom = if shown {
                rect.bottom() - 64.0
            } else {
                rect.bottom() - (rect.width() * 0.025).max(16.0)
            };
            let at = pos2(rect.center().x - galley.size().x / 2.0, bottom - galley.size().y);
            let pad = vec2(size * 0.3, size * 0.05);
            let ground = caption_ground(&t, style);
            if ground != Color32::TRANSPARENT {
                painter.rect_filled(
                    Rect::from_min_size(at - pad, galley.size() + pad * 2.0),
                    t.radius("mini"),
                    ground,
                );
            }
            let edge = t.color("caption-edge");
            match style.edge {
                "outline" => {
                    let w = (size * 0.055).max(1.0);
                    for (dx, dy) in [
                        (-w, 0.0),
                        (w, 0.0),
                        (0.0, -w),
                        (0.0, w),
                        (-0.7 * w, -0.7 * w),
                        (0.7 * w, -0.7 * w),
                        (-0.7 * w, 0.7 * w),
                        (0.7 * w, 0.7 * w),
                    ] {
                        painter.galley_with_override_text_color(at + vec2(dx, dy), galley.clone(), edge);
                    }
                }
                "shadow" => {
                    painter.galley_with_override_text_color(
                        at + vec2(size * 0.05, size * 0.07),
                        galley.clone(),
                        edge.gamma_multiply(0.9),
                    );
                }
                _ => {}
            }
            painter.galley(at, galley, caption_ink(&t, style));
        }

        // the big play button, before it has played and while the controls hide
        if !covered && !self.started && !self.playing && !shown {
            let big = Rect::from_center_size(rect.center(), vec2(56.0, 56.0));
            let hit = ui.interact(big, id.with("big-play"), Sense::click());
            hit.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, &self.words.play));
            let ground = if hit.hovered() { 0.85 } else { 0.7 };
            painter.add(t.shadow("elevation-pop").as_shape(big, t.radius("surface")));
            painter.rect_filled(big, t.radius("surface"), t.color_at("terminal", ground));
            super::paint_play(
                &painter,
                big.center() + vec2(1.0, 0.0),
                24.0,
                t.color("terminal-foreground"),
            );
            if hit.clicked() {
                out.playing = Some(true);
            }
        }

        if shown {
            self.controls(ui, rect, id, &mut out);
        }

        if let Some(cover) = cover {
            painter.rect_filled(rect, radius, t.color_at("terminal", 0.75));
            let pad = if rect.width() >= 512.0 { 12.0 } else { 8.0 };
            let inner = rect.shrink(pad);
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(inner)
                    .layout(Layout::centered_and_justified(egui::Direction::TopDown)),
            );
            child.set_clip_rect(rect.intersect(ui.clip_rect()));
            cover(&mut child);
        }

        if let Some(overlay) = overlay {
            let pad = if rect.width() >= 512.0 { 12.0 } else { 8.0 };
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(rect.shrink(pad))
                    .layout(Layout::top_down(Align::Min)),
            );
            child.set_clip_rect(rect.intersect(ui.clip_rect()));
            overlay(&mut child);
        }
        // the frame's square corners cut back into the page's ground, as `overflow-hidden` rounds
        // the picture and the fade under the controls
        cut_corners(&ui.painter_at(rect), rect, radius, t.color("background"));
        out.response = Some(response);
        out
    }

    /// The controls along the foot: the fade, the position, and the row of buttons.
    fn controls(self, ui: &mut Ui, rect: Rect, id: egui::Id, out: &mut VideoPlayerResponse) {
        let t = tokens(ui.ctx());
        let wide = rect.width() >= 512.0;
        let side_pad = if wide { 12.0 } else { 8.0 };
        let button = t.density("control");
        let row_bottom = rect.bottom() - 6.0;
        let row = Rect::from_min_max(
            pos2(rect.left() + side_pad, row_bottom - button),
            pos2(rect.right() - side_pad, row_bottom),
        );
        let line = Rect::from_min_max(
            pos2(row.left() + 4.0, row.top() - 24.0),
            pos2(row.right() - 4.0, row.top() - 4.0),
        );
        // the fade: terminal at 90% at the foot, 55% halfway, nothing 40 over the position
        let fade = Rect::from_min_max(pos2(rect.left(), line.top() - 40.0), rect.max);
        let painter = ui.painter_at(rect);
        let mut mesh = Mesh::default();
        let stops = [
            (0.0, t.color_at("terminal", 0.0)),
            (0.5, t.color_at("terminal", 0.55)),
            (1.0, t.color_at("terminal", 0.9)),
        ];
        for pair in stops.windows(2) {
            let (top, from) = pair[0];
            let (bottom, to) = pair[1];
            let band = Rect::from_min_max(
                pos2(fade.left(), fade.top() + fade.height() * top),
                pos2(fade.right(), fade.top() + fade.height() * bottom),
            );
            let at = mesh.vertices.len() as u32;
            mesh.colored_vertex(band.left_top(), from);
            mesh.colored_vertex(band.right_top(), from);
            mesh.colored_vertex(band.right_bottom(), to);
            mesh.colored_vertex(band.left_bottom(), to);
            mesh.add_triangle(at, at + 1, at + 2);
            mesh.add_triangle(at, at + 2, at + 3);
        }
        painter.add(Shape::mesh(mesh));

        // the position
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(line)
                .layout(Layout::left_to_right(Align::Center)),
        );
        let position = Position::new(id.with("position"), self.time, self.duration, &self.words.seek)
            .tone(MediaTone::Overlay)
            .height(line.height())
            .show(&mut child);
        if let Some(at) = position.seek {
            out.seek = Some(at);
        }
        out.scrub = position.scrub;

        let play_words = PlayWords {
            play: self.words.play.clone(),
            pause: self.words.pause.clone(),
        };
        // the middle: back, play and forward, 6 apart
        let gap = 6.0;
        let small = t.density("control-sm");
        let middle_width = small * 2.0 + button + gap * 2.0;
        let middle = Rect::from_center_size(row.center(), vec2(middle_width, button));
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(middle)
                .layout(Layout::left_to_right(Align::Center)),
        );
        child.spacing_mut().item_spacing.x = gap;
        if child
            .add(TransportButton::new(Icon::RotateCcw, &self.words.back).tone(MediaTone::Overlay))
            .clicked()
        {
            out.seek = Some((self.time - VIDEO_SKIP).max(0.0));
        }
        if child
            .add(PlayButton::new(self.playing).tone(MediaTone::Overlay).words(play_words))
            .clicked()
        {
            out.playing = Some(!self.playing);
        }
        if child
            .add(TransportButton::new(Icon::RotateCw, &self.words.forward).tone(MediaTone::Overlay))
            .clicked()
        {
            out.seek = Some((self.time + VIDEO_SKIP).min(self.duration));
        }

        // the start: the title, 6 in, cut short before the middle
        if let Some(title) = &self.title
            && wide
        {
            let room = (middle.left() - 8.0 - row.left() - 12.0).max(0.0);
            let galley = egui::WidgetText::from(TextRole::Action.rich(&t, title)).into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                room,
                TextRole::Action.font(&t),
            );
            painter.galley(
                pos2(row.left() + 6.0, row.center().y - galley.size().y / 2.0),
                galley,
                t.color("terminal-foreground"),
            );
        }

        // the end, right to left, 2 apart
        let end = Rect::from_min_max(pos2(middle.right() + 8.0, row.top()), row.max);
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(end)
                .layout(Layout::right_to_left(Align::Center)),
        );
        child.spacing_mut().item_spacing.x = 2.0;
        let ui = &mut child;
        let mut menu_open = false;
        if ghost_icon(
            ui,
            if self.fullscreen {
                Icon::Minimize
            } else {
                Icon::Maximize
            },
            if self.fullscreen {
                &self.words.exit_fullscreen
            } else {
                &self.words.fullscreen
            },
        )
        .clicked()
        {
            out.fullscreen = Some(!self.fullscreen);
        }
        if let Some(on) = self.picture_in_picture
            && rect.width() >= 384.0
            && ghost_icon(
                ui,
                Icon::PictureInPicture2,
                if on {
                    &self.words.exit_picture_in_picture
                } else {
                    &self.words.picture_in_picture
                },
            )
            .clicked()
        {
            out.picture_in_picture = Some(!on);
        }
        // the app's own, laid out from the end as the rest of this side is
        if let Some(actions) = self.actions
            && wide
        {
            actions(ui);
        }
        if wide
            && self.qualities.len() > 1
            && let Some(chosen) = self
                .quality
                .as_ref()
                .and_then(|value| self.qualities.iter().find(|one| &one.value == value))
                .or(self.qualities.first())
        {
            let spec = t.text("xs");
            let font = mono_font(ui.ctx(), mono_size(&t, spec.size), 400.0);
            let galley = ui
                .painter()
                .layout_no_wrap(chosen.label.clone(), font, Color32::PLACEHOLDER);
            let (r, hit) = ui.allocate_exact_size(vec2((galley.size().x + 12.0).max(32.0), button), Sense::click());
            let label = format!("{}: {}", self.words.quality, chosen.label);
            hit.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, &label));
            if hit.hovered() {
                ui.painter()
                    .rect_filled(r, t.radius("control"), t.color_at("terminal-foreground", 0.15));
            }
            ui.painter()
                .galley(r.center() - galley.size() / 2.0, galley, t.color("terminal-foreground"));
            let items: Vec<MenuItem> = self
                .qualities
                .iter()
                .map(|one| {
                    let mut item = MenuItem::action(&one.label);
                    if let Some(detail) = &one.detail {
                        item = item.hint(detail);
                    }
                    if one.value == chosen.value {
                        item = item.icon(Icon::Check);
                    }
                    item
                })
                .collect();
            if let Some(index) = DropdownMenu::new(&items).end(true).width(160.0).show(&hit) {
                out.quality = Some(self.qualities[index].value.clone());
            }
            menu_open |= egui::Popup::is_id_open(ui.ctx(), hit.id.with("menu"));
        }
        if wide
            && self.rates.len() > 1
            && let Some(rate) = RateMenu::new(self.rate, &self.words.speed)
                .rates(self.rates)
                .tone(MediaTone::Overlay)
                .size(TransportSize::Md)
                .show(ui)
        {
            out.rate = Some(rate);
        }
        if !self.tracks.is_empty() {
            let hit = ui.add(
                TransportButton::new(Icon::Captions, &self.words.captions)
                    .tone(MediaTone::Overlay)
                    .pressed(self.track.is_some()),
            );
            let mut items = vec![MenuItem::action(&self.words.captions_off)];
            items.extend(self.tracks.iter().map(MenuItem::action));
            let checked = self.track.map_or(0, |at| at + 1);
            if let MenuItem::Action { icon, .. } = &mut items[checked] {
                *icon = Some(Icon::Check);
            }
            if let Some(index) = DropdownMenu::new(&items).end(true).width(160.0).show(&hit) {
                out.track = Some(index.checked_sub(1));
            }
        }
        let volume = VolumeControl::new(
            id.with("volume"),
            if self.muted { 0.0 } else { self.volume },
            self.muted,
        )
        .tone(MediaTone::Overlay)
        .compact(rect.width() < 768.0)
        .words(VolumeWords {
            mute: self.words.mute.clone(),
            unmute: self.words.unmute.clone(),
            volume: self.words.volume.clone(),
        });
        // the volume: its button, and its 96 slider 4 after where there is room
        let compact = rect.width() < 768.0;
        let room = small + if compact { 0.0 } else { 4.0 + 96.0 };
        let heard = ui
            .allocate_ui_with_layout(vec2(room, button), Layout::left_to_right(Align::Center), |ui| {
                volume.show(ui)
            })
            .inner;
        if heard.volume.is_some() {
            out.volume = heard.volume;
        }
        if heard.muted.is_some() {
            out.muted = heard.muted;
        }
        ui.data_mut(|d| d.insert_temp(id.with("menu"), menu_open));
    }
}

/// A ghost button over the picture holding an icon, `control` square.
fn ghost_icon(ui: &mut Ui, icon: Icon, label: &str) -> egui::Response {
    let t = tokens(ui.ctx());
    let side = t.density("control");
    let (rect, response) = ui.allocate_exact_size(vec2(side, side), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, label));
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, t.radius("control"), t.color_at("terminal-foreground", 0.15));
    }
    super::focus_ring(ui.painter(), &t, rect, t.radius("control"), &response);
    icons::paint(ui.painter(), rect.center(), icon, 16.0, t.color("terminal-foreground"));
    response.on_hover_text(label)
}

/// Covers a frame's square corners outside its rounded ones with the page's ground, so a picture
/// painted to the square reads rounded.
fn cut_corners(painter: &Painter, rect: Rect, radius: CornerRadius, ground: Color32) {
    let r = f32::from(radius.nw);
    if r <= 0.0 {
        return;
    }
    // a thick ring outside the rounded outline, clipped to the frame, hides the square corners
    painter.rect_stroke(
        rect.expand(r),
        CornerRadius::same((r * 2.0) as u8),
        Stroke::new(r * 2.0, ground),
        StrokeKind::Middle,
    );
}
