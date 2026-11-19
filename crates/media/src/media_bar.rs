//! The player along the foot of a screen, after sokkel-ui-media's `MediaBar`, its parts
//! (`MediaBarParts`), its arrangements (`MediaBarLayouts`) and its fullscreen view
//! (`MediaFullscreen`).

use egui::{
    Align, Color32, CornerRadius, FontSelection, Id, Key, Layout, Mesh, Order, Painter, Rect, RichText, Sense, Shape,
    Stroke, Ui, UiBuilder, pos2, text::LayoutJob, vec2,
};

use sokkel_egui::{
    Icon, LiveMark, MEDIA_RATES, MediaRange, PlayButton, PlayWords, Position, RateMenu, Segmented, TextRole,
    TransportButton, TransportGlyph, TransportSize, VolumeControl, VolumeWords, icons, scope, tokens,
};

use crate::artwork::{Artwork, ArtworkCover};
use crate::bar_growth::{LINE, PHONE, TALL, WIDE, glow, grows, panel_growth, rows_growth, stage_growth};
pub use crate::bar_growth::{MediaBarLayout, Tier, rows_height_for};
use crate::queue::RepeatMode;
use crate::track_list::one_line;

/// How far the skip buttons move unless told otherwise, in seconds.
pub const MEDIA_SKIP: f64 = 15.0;

/// The most of the window's height the bar takes, whatever its size.
const WINDOW_SHARE: f32 = 0.5;

/// How far a skip moves, in seconds: one interval both ways, or each way its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediaSkip {
    pub back: f64,
    pub forward: f64,
}

impl From<f64> for MediaSkip {
    fn from(seconds: f64) -> Self {
        Self {
            back: seconds,
            forward: seconds,
        }
    }
}

/// A look for the visualizer an app offers in the fullscreen view's picker, by its own id and name.
#[derive(Clone, Debug, PartialEq)]
pub struct MediaVisualizerOption {
    pub id: String,
    pub label: String,
}

impl MediaVisualizerOption {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

/// The words the player says.
#[derive(Clone, Debug)]
pub struct MediaBarWords {
    pub play: String,
    pub pause: String,
    pub previous: String,
    pub next: String,
    /// `Back {seconds} seconds`.
    pub skip_back: String,
    /// `Forward {seconds} seconds`.
    pub skip_forward: String,
    pub speed: String,
    pub live: String,
    pub shuffle: String,
    pub repeat_off: String,
    pub repeat_all: String,
    pub repeat_one: String,
    pub mute: String,
    pub unmute: String,
    pub volume: String,
    pub seek: String,
    pub resize: String,
    /// `Track {number} of {count}`, and `Track {number}` without a count.
    pub track_of: String,
    pub track: String,
    pub fullscreen: String,
    pub exit_fullscreen: String,
    pub now_playing: String,
    pub lyrics: String,
    pub visualizer_kind: String,
}

impl Default for MediaBarWords {
    fn default() -> Self {
        Self {
            play: "Play".into(),
            pause: "Pause".into(),
            previous: "Previous".into(),
            next: "Next".into(),
            skip_back: "Back {seconds} seconds".into(),
            skip_forward: "Forward {seconds} seconds".into(),
            speed: "Playback speed".into(),
            live: "Live".into(),
            shuffle: "Shuffle".into(),
            repeat_off: "Repeat".into(),
            repeat_all: "Repeat all".into(),
            repeat_one: "Repeat one".into(),
            mute: "Mute".into(),
            unmute: "Unmute".into(),
            volume: "Volume".into(),
            seek: "Seek".into(),
            resize: "Resize the player".into(),
            track_of: "Track {number} of {count}".into(),
            track: "Track {number}".into(),
            fullscreen: "Fullscreen".into(),
            exit_fullscreen: "Exit fullscreen".into(),
            now_playing: "Now playing".into(),
            lyrics: "Lyrics".into(),
            visualizer_kind: "Visualizer".into(),
        }
    }
}

/// What the player was asked this frame; each is what the web's handler of the same name hears.
#[derive(Clone, Debug, Default)]
pub struct MediaBarResponse {
    /// Play or pause asked (`onPlayingChange`).
    pub playing: Option<bool>,
    /// A seek, in seconds from the start.
    pub seek: Option<f64>,
    /// The time under the pointer while the scrubber is dragged.
    pub scrub: Option<f64>,
    pub volume: Option<f32>,
    pub muted: Option<bool>,
    pub previous: bool,
    pub next: bool,
    /// A skip by the seconds given, negative for back.
    pub skip: Option<f64>,
    pub rate: Option<f64>,
    pub shuffle: Option<bool>,
    pub repeat: Option<RepeatMode>,
    /// The height the top edge was dragged to, or put back to.
    pub size: Option<f32>,
    pub fullscreen: Option<bool>,
    pub lyrics_shown: Option<bool>,
    /// The id of the visualizer's look picked in the fullscreen view.
    pub visualizer_option: Option<String>,
}

type Draw<'a> = Box<dyn FnMut(&mut Ui) + 'a>;
type Picture<'a> = Box<dyn Fn(&Painter, Rect) + 'a>;
/// The visualizer as the arrangements are handed it.
type Paint<'v> = Option<&'v dyn Fn(&Painter, Rect)>;

/// The player along the foot of a screen, for audio, after sokkel-ui-media's `MediaBar`. As one
/// line it holds what is playing at the left (the cover, the title in `text.label`, the artist and
/// the album in `text.help`), the transport over the position in the middle, and the speed, the
/// volume, the app's `extra` controls and the fullscreen button at the right, on `sidebar` under a
/// `border-strong` line. Under 672 of its own width it folds into two rows.
///
/// Its top edge drags to resize it, a double click putting it back; dragged taller than about 96
/// it grows by its `layout` (rows, panel or stage), each with the track's `waveform` as its
/// scrubber and the `visualizer` rising faintly behind the words. It never takes more than half the
/// window. The corner button opens the fullscreen view: a dark room in every theme with the
/// visualizer at full strength, a picker of the app's `visualizer_options`, the `lyrics` shown in
/// the visualizer's place from a button of their own, and `fullscreen_actions` under the title;
/// Escape leaves it.
///
/// It is controlled: it draws the `position` it is given and reports what was asked in its
/// response. `size`, `fullscreen` and `lyrics_shown` are held by the bar itself unless given.
#[must_use = "show it with `.show(ui)`"]
pub struct MediaBar<'a> {
    id: Id,
    title: String,
    subtitle: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    year: Option<String>,
    track_number: Option<usize>,
    track_count: Option<usize>,
    artwork: Option<ArtworkCover>,
    placeholder_seed: Option<String>,
    playing: bool,
    position: f64,
    duration: f64,
    live: bool,
    rate: f64,
    rates: &'a [f64],
    rate_menu: bool,
    buffered: &'a [MediaRange],
    waveform: Option<&'a [f32]>,
    visualizer: Option<Picture<'a>>,
    volume: f32,
    muted: bool,
    mutable: bool,
    previous: bool,
    next: bool,
    shuffle: Option<bool>,
    repeat: Option<RepeatMode>,
    skip: Option<MediaSkip>,
    extra: Option<Draw<'a>>,
    layout: MediaBarLayout,
    size: Option<f32>,
    resizable: bool,
    min_size: f32,
    max_size: f32,
    default_size: Option<f32>,
    expandable: bool,
    fullscreen: Option<bool>,
    lyrics: Option<Draw<'a>>,
    lyrics_shown: Option<bool>,
    fullscreen_actions: Option<Draw<'a>>,
    visualizer_options: &'a [MediaVisualizerOption],
    visualizer_option: Option<&'a str>,
    words: MediaBarWords,
}

impl<'a> MediaBar<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, title: impl Into<String>) -> Self {
        Self {
            id: Id::unique(id_salt),
            title: title.into(),
            subtitle: None,
            artist: None,
            album: None,
            year: None,
            track_number: None,
            track_count: None,
            artwork: None,
            placeholder_seed: None,
            playing: false,
            position: 0.0,
            duration: 0.0,
            live: false,
            rate: 1.0,
            rates: &MEDIA_RATES,
            rate_menu: false,
            buffered: &[],
            waveform: None,
            visualizer: None,
            volume: 1.0,
            muted: false,
            mutable: true,
            previous: false,
            next: false,
            shuffle: None,
            repeat: None,
            skip: None,
            extra: None,
            layout: MediaBarLayout::Rows,
            size: None,
            resizable: true,
            min_size: LINE,
            max_size: 360.0,
            default_size: None,
            expandable: true,
            fullscreen: None,
            lyrics: None,
            lyrics_shown: None,
            fullscreen_actions: None,
            visualizer_options: &[],
            visualizer_option: None,
            words: MediaBarWords::default(),
        }
    }

    /// The line under the title in the one-line bar; by default the artist and the album.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn artist(mut self, artist: impl Into<String>) -> Self {
        self.artist = Some(artist.into());
        self
    }

    pub fn album(mut self, album: impl Into<String>) -> Self {
        self.album = Some(album.into());
        self
    }

    pub fn year(mut self, year: impl ToString) -> Self {
        self.year = Some(year.to_string());
        self
    }

    /// Its place in what is playing, said as "Track 13 of 16" over the large title.
    pub fn track(mut self, number: usize, count: Option<usize>) -> Self {
        self.track_number = Some(number);
        self.track_count = count;
        self
    }

    pub fn artwork(mut self, cover: ArtworkCover) -> Self {
        self.artwork = Some(cover);
        self
    }

    /// A stable id, such as the album's, that a cover is drawn from when there is no `artwork`.
    pub fn placeholder_seed(mut self, seed: impl Into<String>) -> Self {
        self.placeholder_seed = Some(seed.into());
        self
    }

    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// Seconds from the start, and the length; a length of zero disables seeking.
    pub fn position(mut self, position: f64, duration: f64) -> Self {
        self.position = position;
        self.duration = duration;
        self
    }

    /// A live stream: a Live mark in place of the position, and nothing seeks.
    pub fn live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    /// The speed menu, written as "1x", is drawn only with this.
    pub fn rate(mut self, rate: f64) -> Self {
        self.rate = rate;
        self.rate_menu = true;
        self
    }

    pub fn rates(mut self, rates: &'a [f64]) -> Self {
        self.rates = rates;
        self
    }

    pub fn buffered(mut self, ranges: &'a [MediaRange]) -> Self {
        self.buffered = ranges;
        self
    }

    /// The track's loudness from start to end, drawn as the grown bar's scrubber.
    pub fn waveform(mut self, levels: &'a [f32]) -> Self {
        self.waveform = Some(levels);
        self
    }

    /// A live picture of the sound, painted at full strength; faint behind the words once the bar
    /// grows, at full strength in the fullscreen view.
    pub fn visualizer(mut self, paint: impl Fn(&Painter, Rect) + 'a) -> Self {
        self.visualizer = Some(Box::new(paint));
        self
    }

    /// From 0 to 1, and whether it is muted.
    pub fn volume(mut self, volume: f32, muted: bool) -> Self {
        self.volume = volume;
        self.muted = muted;
        self
    }

    /// Whether the mute button works (the web's `onMutedChange`); on by default.
    pub fn mutable(mut self, mutable: bool) -> Self {
        self.mutable = mutable;
        self
    }

    /// Whether previous and next work; drawn disabled without.
    pub fn steps(mut self, previous: bool, next: bool) -> Self {
        self.previous = previous;
        self.next = next;
        self
    }

    /// Shuffle, drawn only when given.
    pub fn shuffle(mut self, shuffle: bool) -> Self {
        self.shuffle = Some(shuffle);
        self
    }

    /// Repeat, drawn only when given; a press asks for the next mode.
    pub fn repeat(mut self, repeat: RepeatMode) -> Self {
        self.repeat = Some(repeat);
        self
    }

    /// Buttons that move back and forward by an interval written on them.
    pub fn skip(mut self, skip: impl Into<MediaSkip>) -> Self {
        self.skip = Some(skip.into());
        self
    }

    /// Controls at the bar's end, such as a queue button.
    pub fn extra(mut self, draw: impl FnMut(&mut Ui) + 'a) -> Self {
        self.extra = Some(Box::new(draw));
        self
    }

    pub fn layout(mut self, layout: MediaBarLayout) -> Self {
        self.layout = layout;
        self
    }

    /// The height, held from outside with the response's `size`.
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn min_size(mut self, size: f32) -> Self {
        self.min_size = size;
        self
    }

    pub fn max_size(mut self, size: f32) -> Self {
        self.max_size = size;
        self
    }

    /// The height the bar starts at and a double click on the edge returns to.
    pub fn default_size(mut self, size: f32) -> Self {
        self.default_size = Some(size);
        self
    }

    /// Whether the bar draws the button that opens the fullscreen view; on by default.
    pub fn expandable(mut self, expandable: bool) -> Self {
        self.expandable = expandable;
        self
    }

    /// Whether the fullscreen view is open, held from outside with the response's `fullscreen`.
    pub fn fullscreen(mut self, open: bool) -> Self {
        self.fullscreen = Some(open);
        self
    }

    /// The words of what plays, such as a [`Lyrics`](crate::Lyrics), for the fullscreen view.
    pub fn lyrics(mut self, draw: impl FnMut(&mut Ui) + 'a) -> Self {
        self.lyrics = Some(Box::new(draw));
        self
    }

    pub fn lyrics_shown(mut self, shown: bool) -> Self {
        self.lyrics_shown = Some(shown);
        self
    }

    /// Controls for what plays in the fullscreen view, such as a star, after the line under its title.
    pub fn fullscreen_actions(mut self, draw: impl FnMut(&mut Ui) + 'a) -> Self {
        self.fullscreen_actions = Some(Box::new(draw));
        self
    }

    /// The app's own looks for the visualizer, which the fullscreen view's picker lists, and the
    /// one shown (the first by default).
    pub fn visualizer_options(mut self, options: &'a [MediaVisualizerOption], chosen: Option<&'a str>) -> Self {
        self.visualizer_options = options;
        self.visualizer_option = chosen;
        self
    }

    pub fn words(mut self, words: MediaBarWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> MediaBarResponse {
        let t = tokens(ui.ctx());
        let id = self.id;
        let size_id = id.with("size");
        let open_id = id.with("open");
        let lyrics_id = id.with("lyrics");
        let room = ui.ctx().content_rect().height();
        let top = self.min_size.max(self.max_size.min((room * WINDOW_SHARE).floor()));
        let own: Option<f32> = ui.data(|d| d.get_temp(size_id));
        let held = self.size.or(own).or(self.default_size).unwrap_or(self.min_size);
        let shown = top.min(self.min_size.max(held));
        let grown = grows(self.layout, shown);
        let open = self
            .fullscreen
            .unwrap_or_else(|| ui.data(|d| d.get_temp(open_id)).unwrap_or(false));
        let lyrics_shown = self
            .lyrics_shown
            .unwrap_or_else(|| ui.data(|d| d.get_temp(lyrics_id)).unwrap_or(false));

        let place = self.track_number.map(|n| match self.track_count {
            Some(c) => self
                .words
                .track_of
                .replace("{number}", &n.to_string())
                .replace("{count}", &c.to_string()),
            None => self.words.track.replace("{number}", &n.to_string()),
        });
        let mut deck = Deck {
            id,
            words: self.words,
            title: self.title,
            subtitle: self.subtitle,
            artist: self.artist,
            album: self.album,
            year: self.year,
            place,
            artwork: self.artwork,
            seed: self.placeholder_seed,
            playing: self.playing,
            at: if self.duration > 0.0 {
                self.position.clamp(0.0, self.duration)
            } else {
                self.position.max(0.0)
            },
            duration: self.duration,
            live: self.live,
            buffered: self.buffered,
            waveform: self.waveform,
            level: if self.muted { 0.0 } else { self.volume },
            muted: self.muted,
            mutable: self.mutable,
            previous: self.previous,
            next: self.next,
            skip: self.skip,
            rate: self.rate,
            rates: self.rates,
            rate_menu: self.rate_menu,
            shuffle: self.shuffle,
            repeat: self.repeat,
            extra: self.extra,
            out: MediaBarResponse::default(),
        };

        // the bar: as tall as its size, or as its one line needs
        let width = ui.available_width();
        // the wide line: the transport over the time's row, 8 over and under, under the bar's rule
        let line_needs = if width >= WIDE { 71.0 } else { 95.0 };
        let height = if grown { shown } else { shown.max(line_needs) };
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        ui.painter().rect_filled(rect, CornerRadius::ZERO, t.color("sidebar"));
        ui.painter().hline(
            rect.x_range(),
            rect.top() + 0.5,
            Stroke::new(1.0, t.color("border-strong")),
        );
        let body = Rect::from_min_max(pos2(rect.left(), rect.top() + 1.0), rect.max);
        let expand = self.expandable;
        let visualizer = if open { None } else { self.visualizer.as_deref() };
        let mut bar = ui.new_child(UiBuilder::new().id_salt(id.with("bar")).max_rect(body));
        bar.set_clip_rect(body.intersect(ui.clip_rect()));
        let asked_open = if !grown {
            line_bar(&mut bar, &mut deck, body, expand)
        } else {
            match self.layout {
                MediaBarLayout::Rows => rows_bar(&mut bar, &mut deck, body, shown, visualizer, expand),
                MediaBarLayout::Panel => panel_bar(&mut bar, &mut deck, body, shown, visualizer, expand),
                MediaBarLayout::Stage => stage_bar(&mut bar, &mut deck, body, shown, visualizer, expand),
            }
        };

        // the top edge resizes it, a double click putting it back
        if self.resizable {
            let edge = Rect::from_min_max(
                pos2(rect.left(), rect.top() - 4.0),
                pos2(rect.right(), rect.top() + 4.0),
            );
            let handle = ui.interact(edge, id.with("resize"), Sense::click_and_drag());
            handle.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Slider, true, &deck.words.resize));
            if handle.hovered() || handle.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                ui.painter()
                    .hline(rect.x_range(), rect.top() + 0.5, Stroke::new(2.0, t.color("ring")));
            }
            let mut asked = None;
            if handle.dragged() {
                let delta = handle.drag_delta().y;
                let from: f32 = ui.data(|d| d.get_temp(id.with("drag"))).unwrap_or(height);
                let next = (from - delta).clamp(self.min_size, top);
                ui.data_mut(|d| d.insert_temp(id.with("drag"), next));
                asked = Some(next.round());
            }
            if handle.drag_stopped() {
                ui.data_mut(|d| d.remove::<f32>(id.with("drag")));
            }
            if handle.double_clicked() {
                asked = Some(self.default_size.unwrap_or(self.min_size));
            }
            if let Some(next) = asked {
                if self.size.is_none() {
                    ui.data_mut(|d| d.insert_temp(size_id, next));
                }
                deck.out.size = Some(next);
            }
        }

        // the fullscreen view
        let set_open = |ui: &Ui, deck: &mut Deck, next: bool| {
            if self.fullscreen.is_none() {
                ui.data_mut(|d| d.insert_temp(open_id, next));
            }
            deck.out.fullscreen = Some(next);
        };
        if asked_open {
            set_open(ui, &mut deck, true);
        }
        if open {
            let view = Room {
                layout: self.layout,
                visualizer: self.visualizer.as_deref(),
                lyrics: self.lyrics,
                lyrics_shown,
                actions: self.fullscreen_actions,
                options: self.visualizer_options,
                option: self
                    .visualizer_option
                    .map(str::to_owned)
                    .or_else(|| ui.data(|d| d.get_temp::<String>(id.with("option"))))
                    .or_else(|| self.visualizer_options.first().map(|o| o.id.clone())),
            };
            let asked = view.show(ui, &mut deck);
            if asked.leave || ui.input(|i| i.key_pressed(Key::Escape)) {
                set_open(ui, &mut deck, false);
            }
            if let Some(shown) = asked.lyrics {
                if self.lyrics_shown.is_none() {
                    ui.data_mut(|d| d.insert_temp(lyrics_id, shown));
                }
                deck.out.lyrics_shown = Some(shown);
            }
            if let Some(option) = asked.option {
                if self.visualizer_option.is_none() {
                    ui.data_mut(|d| d.insert_temp(id.with("option"), option.clone()));
                }
                deck.out.visualizer_option = Some(option);
            }
        }
        deck.out
    }
}

/// What every arrangement of the player draws from: what plays, where it is, and what was asked.
struct Deck<'a> {
    id: Id,
    words: MediaBarWords,
    title: String,
    subtitle: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    year: Option<String>,
    place: Option<String>,
    artwork: Option<ArtworkCover>,
    seed: Option<String>,
    playing: bool,
    at: f64,
    duration: f64,
    live: bool,
    buffered: &'a [MediaRange],
    waveform: Option<&'a [f32]>,
    level: f32,
    muted: bool,
    mutable: bool,
    previous: bool,
    next: bool,
    skip: Option<MediaSkip>,
    rate: f64,
    rates: &'a [f64],
    rate_menu: bool,
    shuffle: Option<bool>,
    repeat: Option<RepeatMode>,
    extra: Option<Draw<'a>>,
    out: MediaBarResponse,
}

/// The side of a transport button and of the play button at a size.
fn sides(ui: &Ui, size: TransportSize) -> (f32, f32) {
    let t = tokens(ui.ctx());
    match size {
        TransportSize::Sm => (t.density("control-sm"), t.density("control")),
        TransportSize::Md => (t.density("control"), t.density("control")),
        TransportSize::Lg => (t.density("control-lg"), 56.0),
    }
}

impl Deck<'_> {
    /// The line under the title: the subtitle, or the artist and the album.
    fn by_line(&self) -> Option<String> {
        self.subtitle
            .clone()
            .or_else(|| meta(&[self.artist.as_deref(), self.album.as_deref()]))
    }

    /// Who made it, or the subtitle.
    fn by(&self) -> Option<String> {
        self.artist.clone().or_else(|| self.subtitle.clone())
    }

    fn skipping(&self) -> bool {
        self.skip.is_some()
    }

    /// How wide the steps are at a size, with shuffle and repeat when `toggles`.
    fn steps_width(&self, ui: &Ui, size: TransportSize, gap: f32, toggles: bool) -> f32 {
        let (side, play) = sides(ui, size);
        let mut buttons = vec![side, play, side];
        if self.skipping() {
            buttons.extend([side, side]);
        }
        if toggles {
            let (toggle, _) = sides(
                ui,
                if size == TransportSize::Lg {
                    TransportSize::Md
                } else {
                    size
                },
            );
            if self.shuffle.is_some() {
                buttons.push(toggle);
            }
            if self.repeat.is_some() {
                buttons.push(toggle);
            }
        }
        buttons.iter().sum::<f32>() + gap * (buttons.len() - 1) as f32
    }

    /// Shuffle, the steps and repeat (with `toggles`), left to right from `ui`'s cursor.
    fn transport(&mut self, ui: &mut Ui, size: TransportSize, toggles: bool) {
        let toggle_size = if size == TransportSize::Lg {
            TransportSize::Md
        } else {
            size
        };
        if toggles {
            self.shuffle_button(ui, toggle_size);
        }
        self.steps(ui, size);
        if toggles {
            self.repeat_button(ui, toggle_size);
        }
    }

    /// Previous, play and next, between the skip buttons when the deck has an interval.
    fn steps(&mut self, ui: &mut Ui, size: TransportSize) {
        if let Some(skip) = self.skip {
            let label = self
                .words
                .skip_back
                .replace("{seconds}", &skip.back.round().to_string());
            let glyph = TransportGlyph::Skip {
                seconds: skip.back,
                back: true,
            };
            if ui
                .add(TransportButton::new(glyph, label).size(size).enabled(!self.live))
                .clicked()
            {
                self.out.skip = Some(-skip.back);
            }
        }
        if step_button(ui, size, &self.words.previous, self.previous, true).clicked() {
            self.out.previous = true;
        }
        let words = PlayWords {
            play: self.words.play.clone(),
            pause: self.words.pause.clone(),
        };
        if ui.add(PlayButton::new(self.playing).size(size).words(words)).clicked() {
            self.out.playing = Some(!self.playing);
        }
        if step_button(ui, size, &self.words.next, self.next, false).clicked() {
            self.out.next = true;
        }
        if let Some(skip) = self.skip {
            let label = self
                .words
                .skip_forward
                .replace("{seconds}", &skip.forward.round().to_string());
            let glyph = TransportGlyph::Skip {
                seconds: skip.forward,
                back: false,
            };
            if ui
                .add(TransportButton::new(glyph, label).size(size).enabled(!self.live))
                .clicked()
            {
                self.out.skip = Some(skip.forward);
            }
        }
    }

    fn shuffle_button(&mut self, ui: &mut Ui, size: TransportSize) {
        if let Some(on) = self.shuffle
            && ui
                .add(
                    TransportButton::new(Icon::Shuffle, &self.words.shuffle)
                        .pressed(on)
                        .size(size),
                )
                .clicked()
        {
            self.out.shuffle = Some(!on);
        }
    }

    fn repeat_button(&mut self, ui: &mut Ui, size: TransportSize) {
        let Some(mode) = self.repeat else { return };
        let (icon, label) = match mode {
            RepeatMode::One => (Icon::Repeat1, &self.words.repeat_one),
            RepeatMode::All => (Icon::Repeat, &self.words.repeat_all),
            RepeatMode::Off => (Icon::Repeat, &self.words.repeat_off),
        };
        if ui
            .add(
                TransportButton::new(icon, label)
                    .pressed(mode != RepeatMode::Off)
                    .size(size),
            )
            .clicked()
        {
            self.out.repeat = Some(mode.next());
        }
    }

    /// The speed menu, drawn only with a rate and never on a live stream.
    fn speed(&mut self, ui: &mut Ui, size: TransportSize) {
        if !self.rate_menu || self.live {
            return;
        }
        if let Some(rate) = RateMenu::new(self.rate, &self.words.speed)
            .rates(self.rates)
            .size(size)
            .show(ui)
        {
            self.out.rate = Some(rate);
        }
    }

    fn volume(&mut self, ui: &mut Ui, compact: bool) {
        let words = VolumeWords {
            mute: self.words.mute.clone(),
            unmute: self.words.unmute.clone(),
            volume: self.words.volume.clone(),
        };
        let asked = VolumeControl::new(self.id.with("volume"), self.level, self.muted)
            .mutable(self.mutable)
            .compact(compact)
            .words(words)
            .show(ui);
        if asked.muted.is_some() {
            self.out.muted = asked.muted;
        }
        if asked.volume.is_some() {
            self.out.volume = asked.volume;
        }
    }

    fn extras(&mut self, ui: &mut Ui) {
        if let Some(extra) = self.extra.as_mut() {
            extra(ui);
        }
    }

    /// The position: a slider, or with `levels` the waveform, between the times; the live mark on
    /// a live stream.
    fn scrub(&mut self, ui: &mut Ui, height: f32, large: bool, waveform: bool) {
        if self.live {
            ui.add(LiveMark::new(&self.words.live).large(large));
            return;
        }
        let mut position = Position::new(self.id.with("position"), self.at, self.duration, &self.words.seek)
            .buffered(self.buffered)
            .height(height)
            .large(large);
        if waveform {
            position = position.waveform(self.waveform.unwrap_or(&[]));
        }
        let asked = position.show(ui);
        if asked.seek.is_some() {
            self.out.seek = asked.seek;
        }
        if asked.scrub.is_some() {
            self.out.scrub = asked.scrub;
        }
    }
}

/// The parts of a line joined as `MetaLine` joins them, by a middle dot.
fn meta(parts: &[Option<&str>]) -> Option<String> {
    let kept: Vec<&str> = parts.iter().flatten().copied().filter(|p| !p.is_empty()).collect();
    (!kept.is_empty()).then(|| kept.join(" \u{b7} "))
}

/// Previous or next: a transport button with Lucide's skip glyph filled, as `fill-current` fills it.
fn step_button(ui: &mut Ui, size: TransportSize, label: &str, enabled: bool, back: bool) -> egui::Response {
    let t = tokens(ui.ctx());
    let icon = if back { Icon::SkipBack } else { Icon::SkipForward };
    let response = ui.add(TransportButton::new(icon, label).size(size).enabled(enabled));
    let glyph = if size == TransportSize::Lg { 20.0 } else { 16.0 };
    let s = glyph / 24.0;
    let c = response.rect.center();
    let at = |x: f32, y: f32| c + vec2((x - 12.0) * s, (y - 12.0) * s);
    let ink = if enabled {
        t.color("foreground")
    } else {
        t.color("foreground").gamma_multiply(0.5)
    };
    // lucide skip-back: the triangle 19 20, 9 12, 19 4; skip-forward mirrors it
    let points = if back {
        vec![at(19.0, 20.0), at(9.0, 12.0), at(19.0, 4.0)]
    } else {
        vec![at(5.0, 4.0), at(15.0, 12.0), at(5.0, 20.0)]
    };
    ui.painter()
        .add(Shape::convex_polygon(points, ink, Stroke::new(2.0 * s, ink)));
    response
}

/// A child placed at `rect`, laid out by `layout`.
fn place<R>(ui: &mut Ui, rect: Rect, layout: Layout, add: impl FnOnce(&mut Ui) -> R) -> R {
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect).layout(layout));
    child.spacing_mut().item_spacing.x = 8.0;
    add(&mut child)
}

/// A row of controls laid out left to right, `width` wide and centered on `center_x`, its middle on
/// `y`.
fn centered_row<R>(
    ui: &mut Ui,
    center_x: f32,
    y: f32,
    width: f32,
    height: f32,
    gap: f32,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let rect = Rect::from_center_size(pos2(center_x, y), vec2(width + 1.0, height));
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    child.spacing_mut().item_spacing.x = gap;
    add(&mut child)
}

/// A row of controls ending at `right`, its middle on `y`, in the order given; past the room's
/// start (`_left`) when it must, as the web's `justify-end` row runs. Gives the width it took.
fn end_row(ui: &mut Ui, right: f32, y: f32, _left: f32, height: f32, add: impl FnOnce(&mut Ui)) -> f32 {
    // laid out left to right from where last frame's width put it, so the order reads as given
    let key = ui.next_auto_id().with("end-row");
    let width: f32 = ui.data(|d| d.get_temp(key)).unwrap_or(0.0);
    let start = right - width;
    let rect = Rect::from_min_max(
        pos2(start, y - height / 2.0),
        pos2(right.max(start + width), y + height / 2.0),
    );
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect.with_max_x(rect.max.x + 400.0))
            .layout(Layout::left_to_right(Align::Center)),
    );
    child.spacing_mut().item_spacing.x = 8.0;
    add(&mut child);
    let used = if child.min_rect().is_positive() {
        child.min_rect().width()
    } else {
        0.0
    };
    if (used - width).abs() > 0.5 {
        ui.data_mut(|d| d.insert_temp(key, used));
        ui.ctx().request_repaint();
    }
    ui.skip_ahead_auto_ids(1);
    used
}

/// The cover, square and filled edge to edge: the drawn artwork, one drawn from the seed, or a note
/// glyph in `faint` on `muted`.
fn cover(painter: &Painter, deck: &Deck, rect: Rect, radius: CornerRadius) {
    let t = tokens(painter.ctx());
    painter.rect_filled(rect, radius, t.color("muted"));
    let art = match (deck.artwork, &deck.seed) {
        (Some(cover), _) => Some(Artwork::of(cover)),
        (None, Some(seed)) => Some(Artwork::new(seed)),
        (None, None) => None,
    };
    match art {
        Some(art) => {
            // the corners are cut by painting the picture into a rounded mask's bounds
            art.paint(painter, rect);
            if radius != CornerRadius::ZERO {
                round_corners(painter, rect, radius, t.color("sidebar"));
            }
        }
        None => icons::paint(
            painter,
            rect.center(),
            Icon::Music,
            (rect.width() * 0.4).min(48.0),
            t.color("faint"),
        ),
    }
}

/// Paints `ground` over the four corners outside a rounded rect, so a picture painted square reads
/// as rounded on that ground.
fn round_corners(painter: &Painter, rect: Rect, radius: CornerRadius, ground: Color32) {
    let r = f32::from(radius.nw);
    if r <= 0.0 {
        return;
    }
    let corners = [
        (rect.left_top(), vec2(1.0, 1.0)),
        (rect.right_top(), vec2(-1.0, 1.0)),
        (rect.right_bottom(), vec2(-1.0, -1.0)),
        (rect.left_bottom(), vec2(1.0, -1.0)),
    ];
    for (corner, inward) in corners {
        let center = corner + inward * r;
        let mut mesh = Mesh::default();
        mesh.colored_vertex(corner, ground);
        let steps = 6;
        for i in 0..=steps {
            let a = std::f32::consts::FRAC_PI_2 * i as f32 / steps as f32;
            let p = center - vec2(inward.x * r * a.cos(), inward.y * r * a.sin());
            mesh.colored_vertex(p, ground);
            if i > 0 {
                mesh.add_triangle(0, i as u32, i as u32 + 1);
            }
        }
        painter.add(Shape::mesh(mesh));
    }
}

/// The visualizer behind the words at `strength`.
fn backdrop(painter: &Painter, visualizer: Paint<'_>, rect: Rect, strength: f32) {
    if let Some(paint) = visualizer
        && strength > 0.0
    {
        let mut faint = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
        faint.multiply_opacity(strength);
        paint(&faint, rect);
    }
}

/// The fullscreen button.
fn expand_button(ui: &mut Ui, deck: &Deck) -> bool {
    ui.add(TransportButton::new(Icon::Maximize2, &deck.words.fullscreen))
        .clicked()
}

/// The fullscreen button in the top right corner of `rect`.
fn corner_expand(ui: &mut Ui, deck: &Deck, rect: Rect) -> bool {
    let t = tokens(ui.ctx());
    let side = t.density("control-sm");
    let at = Rect::from_min_size(pos2(rect.right() - 6.0 - side, rect.top() + 6.0), vec2(side, side));
    place(ui, at, Layout::left_to_right(Align::Center), |ui| {
        expand_button(ui, deck)
    })
}

/// A text laid out with `rich`, wrapped to `width`.
fn wrapped(ui: &Ui, rich: RichText, width: f32, rows: usize) -> std::sync::Arc<egui::Galley> {
    let mut job = LayoutJob::default();
    rich.append_to(&mut job, ui.style(), FontSelection::Default, Align::Min);
    job.wrap = egui::text::TextWrapping {
        max_width: width.max(1.0),
        max_rows: rows,
        break_anywhere: false,
        overflow_character: (rows != usize::MAX).then_some('\u{2026}'),
    };
    ui.painter().layout_job(job)
}

/// The line in the accent over a large title: the place, the album and the year, as a group label.
fn eyebrow(ui: &Ui, deck: &Deck, width: f32, large: bool) -> Option<std::sync::Arc<egui::Galley>> {
    let t = tokens(ui.ctx());
    let line = meta(&[deck.place.as_deref(), deck.album.as_deref(), deck.year.as_deref()])?;
    let role = if large { TextRole::Body } else { TextRole::Help };
    let size = t.text(role.key()).size;
    let rich = role
        .rich(&t, line.to_uppercase())
        .variation("wght", 500.0)
        .extra_letter_spacing(size * 0.025)
        .color(t.color("primary-ink"));
    Some(one_line(ui.painter(), ui.style(), rich, width))
}

/// What plays, saying as much as the tier allows, its foot on `rect`'s foot.
fn words(ui: &Ui, deck: &Deck, tier: Tier, rect: Rect, center: bool) {
    let t = tokens(ui.ctx());
    let width = rect.width();
    let mut lines: Vec<std::sync::Arc<egui::Galley>> = Vec::new();
    let painter = ui.painter();
    match tier {
        Tier::Line => {
            let mut job = LayoutJob::default();
            TextRole::Body
                .rich(&t, &deck.title)
                .variation("wght", 600.0)
                .color(t.color("foreground"))
                .append_to(&mut job, ui.style(), FontSelection::Default, Align::Min);
            if let Some(by) = deck.by() {
                TextRole::Body
                    .rich(&t, format!(" \u{b7} {by}"))
                    .color(t.color("muted-foreground"))
                    .append_to(&mut job, ui.style(), FontSelection::Default, Align::Min);
            }
            job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(1.0));
            lines.push(painter.layout_job(job));
        }
        Tier::Pair => {
            lines.push(one_line(
                painter,
                ui.style(),
                TextRole::DetailTitle.rich(&t, &deck.title).color(t.color("foreground")),
                width,
            ));
            if let Some(by) = deck.by_line() {
                lines.push(one_line(
                    painter,
                    ui.style(),
                    TextRole::Help.rich(&t, by).color(t.color("muted-foreground")),
                    width,
                ));
            }
        }
        Tier::Liner | Tier::Poster => {
            let poster = tier == Tier::Poster;
            if let Some(brow) = eyebrow(ui, deck, width, false) {
                lines.push(brow);
            }
            let (role, key) = if poster {
                (TextRole::Statistic, "stat")
            } else {
                (TextRole::PageTitle, "page")
            };
            let size = t.text(key).size;
            lines.push(one_line(
                painter,
                ui.style(),
                role.rich(&t, &deck.title)
                    .variation("wght", 600.0)
                    .extra_letter_spacing(-0.025 * size)
                    .color(t.color("foreground")),
                width,
            ));
            if let Some(by) = deck.by() {
                let role = if poster { TextRole::DetailTitle } else { TextRole::Body };
                let rich = role.rich(&t, by).variation("wght", 400.0);
                lines.push(one_line(
                    painter,
                    ui.style(),
                    rich.color(t.color("muted-foreground")),
                    width,
                ));
            }
        }
    }
    let total: f32 = lines.iter().map(|g| g.size().y).sum();
    let mut y = rect.bottom() - total;
    for galley in lines {
        let x = if center {
            rect.center().x - galley.size().x / 2.0
        } else {
            rect.left()
        };
        let h = galley.size().y;
        painter.galley(pos2(x, y), galley, t.color("foreground"));
        y += h;
    }
}

/// The one-line bar, drawn by every layout until it grows. Gives whether fullscreen was asked.
fn line_bar(ui: &mut Ui, deck: &mut Deck, rect: Rect, expand: bool) -> bool {
    let t = tokens(ui.ctx());
    let wide = rect.width() >= WIDE;
    let mut asked = false;
    let line = deck.by_line();
    let words_at = |ui: &Ui, deck: &Deck, left: f32, right: f32, y: f32| {
        let width = (right - left).max(0.0);
        let title = one_line(
            ui.painter(),
            ui.style(),
            TextRole::Label.rich(&t, &deck.title).color(t.color("foreground")),
            width,
        );
        let sub = line.as_ref().map(|l| {
            one_line(
                ui.painter(),
                ui.style(),
                TextRole::Help.rich(&t, l).color(t.color("muted-foreground")),
                width,
            )
        });
        let block = title.size().y + sub.as_ref().map_or(0.0, |s| s.size().y);
        let top = y - block / 2.0;
        let h = title.size().y;
        ui.painter().galley(pos2(left, top), title, t.color("foreground"));
        if let Some(sub) = sub {
            ui.painter()
                .galley(pos2(left, top + h), sub, t.color("muted-foreground"));
        }
    };
    if wide {
        let inner = rect.shrink2(vec2(12.0, 8.0));
        let gap = 24.0;
        let unit = (inner.width() - 2.0 * gap) / 3.6;
        let first = Rect::from_min_size(inner.min, vec2(unit, inner.height()));
        let middle = Rect::from_min_size(pos2(first.right() + gap, inner.top()), vec2(unit * 1.6, inner.height()));
        let last = Rect::from_min_max(pos2(middle.right() + gap, inner.top()), inner.max);
        // what plays, the cover growing with the bar
        // 48 on the one line (70 under the rule), growing as the bar is dragged taller
        let side = (rect.height() - 22.0).max(48.0);
        let art = Rect::from_min_size(pos2(first.left(), first.center().y - side / 2.0), vec2(side, side));
        cover(ui.painter(), deck, art, t.radius("mini"));
        words_at(ui, deck, art.right() + 12.0, first.right(), first.center().y);
        // the transport over the position
        // the time's row 20 tall, 2 under the transport, as the web's `text-xs` times beside the track
        let (_, play) = sides(ui, TransportSize::Sm);
        let block = play + 2.0 + 20.0;
        let top = middle.center().y - block / 2.0;
        let steps = deck.steps_width(ui, TransportSize::Sm, 8.0, true);
        centered_row(ui, middle.center().x, top + play / 2.0, steps, play, 8.0, |ui| {
            deck.transport(ui, TransportSize::Sm, true);
        });
        // the position lays its row out a control tall, so it is given that much around the middle
        let control = ui.spacing().interact_size.y;
        let position = Rect::from_center_size(
            pos2(middle.center().x, top + play + 2.0 + 10.0),
            vec2(middle.width(), control),
        );
        place(ui, position, Layout::left_to_right(Align::Center), |ui| {
            deck.scrub(ui, 16.0, false, false);
        });
        // the speed, the volume, the extra controls and the fullscreen button
        end_row(ui, last.right(), last.center().y, last.left(), play, |ui| {
            deck.speed(ui, TransportSize::Sm);
            deck.volume(ui, false);
            deck.extras(ui);
            if expand {
                asked = expand_button(ui, deck);
            }
        });
    } else {
        // two rows: what plays beside the transport, over the position
        let inner = rect.shrink2(vec2(12.0, 10.0));
        let side = 40.0;
        let row = Rect::from_min_size(inner.min, vec2(inner.width(), side));
        let steps = deck.steps_width(ui, TransportSize::Sm, 4.0, false);
        let (_, play) = sides(ui, TransportSize::Sm);
        centered_row(ui, row.right() - steps / 2.0, row.center().y, steps, play, 4.0, |ui| {
            deck.steps(ui, TransportSize::Sm);
        });
        let art = Rect::from_min_size(row.min, vec2(side, side));
        cover(ui.painter(), deck, art, t.radius("mini"));
        words_at(ui, deck, art.right() + 12.0, row.right() - steps - 12.0, row.center().y);
        let foot_y = row.bottom() + 6.0 + 14.0;
        let used = end_row(ui, inner.right(), foot_y, inner.left(), play, |ui| {
            deck.speed(ui, TransportSize::Sm);
            deck.extras(ui);
            if expand {
                asked = expand_button(ui, deck);
            }
        });
        let position = Rect::from_min_max(
            pos2(inner.left(), foot_y - 8.0),
            pos2(inner.right() - used - 8.0, foot_y + 8.0),
        );
        place(ui, position, Layout::left_to_right(Align::Center), |ui| {
            deck.scrub(ui, 16.0, false, false);
        });
    }
    asked
}

/// `rows`: the cover across both rows; the words, the transport and the volume in the top row; the
/// waveform as wide as the rest in the bottom row.
fn rows_bar(ui: &mut Ui, deck: &mut Deck, rect: Rect, height: f32, visualizer: Paint<'_>, expand: bool) -> bool {
    let t = tokens(ui.ctx());
    let narrow = rect.width() < WIDE;
    let rows = rows_growth(height, rect.width());
    let inner = Rect::from_min_max(
        pos2(rect.left() + 12.0, rect.top() + rows.pad),
        pos2(rect.right() - 12.0, rect.bottom() - rows.pad),
    );
    let mut asked = false;
    let mut left = inner.left();
    if !narrow {
        let art = Rect::from_min_size(
            pos2(left, inner.center().y - rows.cover / 2.0),
            vec2(rows.cover, rows.cover),
        );
        let radius = if rows.cover >= 160.0 {
            t.radius("surface")
        } else {
            t.radius("mini")
        };
        cover(ui.painter(), deck, art, radius);
        left = art.right() + 16.0;
    }
    let column = Rect::from_min_max(pos2(left, inner.top()), inner.max);
    let head = Rect::from_min_max(column.min, pos2(column.right(), column.bottom() - rows.wave - rows.gap));
    let wave = Rect::from_min_max(pos2(column.left(), column.bottom() - rows.wave), column.max);
    backdrop(
        ui.painter(),
        visualizer,
        Rect::from_min_max(pos2(head.left(), head.top() - 12.0), head.max),
        glow(height),
    );
    let size = if height >= TALL {
        TransportSize::Md
    } else {
        TransportSize::Sm
    };
    let (_, play) = sides(ui, size);
    let foot = head.bottom() - play / 2.0;
    let steps = deck.steps_width(ui, size, 8.0, !narrow);
    let controls_right = head.right();
    let words_room = if narrow {
        let used = end_row(ui, controls_right, foot, head.left(), play, |ui| {
            deck.extras(ui);
            if expand && !rows.corner {
                asked |= expand_button(ui, deck);
            }
        });
        let steps_right = controls_right - used - if used > 0.0 { 8.0 } else { 0.0 };
        centered_row(ui, steps_right - steps / 2.0, foot, steps, play, 8.0, |ui| {
            deck.transport(ui, size, false);
        });
        let mut words_left = head.left();
        let art = Rect::from_min_size(
            pos2(words_left, head.bottom() - rows.cover.max(0.0)),
            vec2(rows.cover.max(0.0), rows.cover.max(0.0)),
        );
        if rows.cover > 0.0 {
            cover(ui.painter(), deck, art, t.radius("mini"));
            words_left = art.right() + 12.0;
        }
        Rect::from_min_max(
            pos2(words_left, head.top()),
            pos2(steps_right - steps - 12.0, head.bottom()),
        )
    } else {
        // three columns: the words, the transport centered, the volume at the end
        let gap = 24.0;
        let third = (head.width() - 2.0 * gap - steps) / 2.0;
        let middle_x = head.left() + third + gap + steps / 2.0;
        let middle_x = if rows.compact_volume {
            head.center().x.max(middle_x)
        } else {
            middle_x
        };
        centered_row(ui, middle_x, foot, steps, play, 8.0, |ui| {
            deck.transport(ui, size, true)
        });
        end_row(ui, controls_right, foot, middle_x + steps / 2.0 + gap, play, |ui| {
            deck.speed(ui, TransportSize::Sm);
            deck.volume(ui, rows.compact_volume);
            deck.extras(ui);
            if expand && !rows.corner {
                asked |= expand_button(ui, deck);
            }
        });
        Rect::from_min_max(head.min, pos2(middle_x - steps / 2.0 - gap, head.bottom()))
    };
    words(ui, deck, rows.tier, words_room, false);
    let scrub = if narrow && deck.rate_menu {
        let used = end_row(ui, wave.right(), wave.center().y, wave.left(), play, |ui| {
            deck.speed(ui, TransportSize::Sm)
        });
        Rect::from_min_max(wave.min, pos2(wave.right() - used - 8.0, wave.bottom()))
    } else {
        wave
    };
    place(ui, scrub, Layout::left_to_right(Align::Center), |ui| {
        deck.scrub(ui, rows.wave, false, true);
    });
    if expand && rows.corner {
        asked |= corner_expand(ui, deck, rect);
    }
    asked
}

/// `panel`: the cover flush and as tall as the bar; the words and the waveform over the transport,
/// which runs along the foot with the volume and the extra controls at its end.
fn panel_bar(ui: &mut Ui, deck: &mut Deck, rect: Rect, height: f32, visualizer: Paint<'_>, expand: bool) -> bool {
    let t = tokens(ui.ctx());
    let narrow = rect.width() < WIDE;
    let foot = t.length("spacing-shell-foot");
    let panel = panel_growth(height, rect.width(), foot);
    let mut asked = false;
    let mut left = rect.left();
    if !narrow {
        let art = Rect::from_min_size(
            pos2(left, rect.center().y - panel.cover / 2.0),
            vec2(panel.cover, panel.cover),
        );
        cover(ui.painter(), deck, art, CornerRadius::ZERO);
        left = art.right();
    }
    let room = Rect::from_min_max(pos2(left, rect.top()), pos2(rect.right(), rect.bottom() - foot));
    let inner = Rect::from_min_max(
        pos2(room.left() + 20.0, room.top() + 8.0),
        pos2(room.right() - 20.0, room.bottom() - 4.0),
    );
    let wave = Rect::from_min_max(pos2(inner.left(), inner.bottom() - panel.wave), inner.max);
    let head = Rect::from_min_max(inner.min, pos2(inner.right(), wave.top() - panel.gap));
    backdrop(
        ui.painter(),
        visualizer,
        Rect::from_min_max(pos2(room.left(), room.top()), pos2(room.right(), head.bottom())),
        glow(height),
    );
    let mut words_left = head.left();
    if narrow && panel.cover > 0.0 {
        let art = Rect::from_min_size(
            pos2(words_left, head.bottom() - panel.cover),
            vec2(panel.cover, panel.cover),
        );
        cover(ui.painter(), deck, art, t.radius("mini"));
        words_left = art.right() + 12.0;
    }
    let words_right = head.right() - if expand { 32.0 } else { 0.0 };
    words(
        ui,
        deck,
        panel.tier,
        Rect::from_min_max(pos2(words_left, head.top()), pos2(words_right, head.bottom())),
        false,
    );
    place(ui, wave, Layout::left_to_right(Align::Center), |ui| {
        deck.scrub(ui, panel.wave, false, true)
    });
    if expand {
        asked |= corner_expand(ui, deck, room);
    }
    // the foot: shuffle, repeat and the transport, the volume and the extra controls at its end
    let foot_rect = Rect::from_min_max(
        pos2(left + 12.0, rect.bottom() - foot),
        pos2(rect.right() - 12.0, rect.bottom()),
    );
    let (_, play) = sides(ui, TransportSize::Sm);
    let y = foot_rect.center().y;
    if narrow {
        let steps = deck.steps_width(ui, TransportSize::Sm, 4.0, false);
        centered_row(ui, foot_rect.center().x, y, steps, play, 4.0, |ui| {
            deck.steps(ui, TransportSize::Sm)
        });
        end_row(
            ui,
            foot_rect.right(),
            y,
            foot_rect.center().x + steps / 2.0,
            play,
            |ui| {
                deck.speed(ui, TransportSize::Sm);
                deck.extras(ui);
            },
        );
    } else {
        let steps = deck.steps_width(ui, TransportSize::Sm, 4.0, true);
        place(
            ui,
            Rect::from_min_size(pos2(foot_rect.left(), y - play / 2.0), vec2(steps + 1.0, play)),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                deck.shuffle_button(ui, TransportSize::Sm);
                deck.repeat_button(ui, TransportSize::Sm);
                deck.steps(ui, TransportSize::Sm);
            },
        );
        end_row(ui, foot_rect.right(), y, foot_rect.left() + steps, play, |ui| {
            deck.speed(ui, TransportSize::Sm);
            deck.volume(ui, panel.compact_volume);
            deck.extras(ui);
        });
    }
    asked
}

/// `stage`: the cover at the left, the stage centered on the bar (the words over a wide waveform,
/// the transport under it), the volume at the foot of the right column.
fn stage_bar(ui: &mut Ui, deck: &mut Deck, rect: Rect, height: f32, visualizer: Paint<'_>, expand: bool) -> bool {
    let t = tokens(ui.ctx());
    let narrow = rect.width() < WIDE;
    let stage = stage_growth(height, rect.width());
    let mut asked = false;
    let behind = Rect::from_min_max(
        pos2(
            rect.left() + if narrow { 0.0 } else { stage.cover + 2.0 * stage.pad },
            rect.top(),
        ),
        pos2(rect.right(), rect.bottom() - stage.under),
    );
    backdrop(ui.painter(), visualizer, behind, glow(height));
    let inner = rect.shrink(stage.pad);
    let middle = if narrow {
        inner
    } else {
        let art = Rect::from_min_size(
            pos2(inner.left(), inner.center().y - stage.cover / 2.0),
            vec2(stage.cover, stage.cover),
        );
        let radius = if stage.cover >= 160.0 {
            t.radius("surface")
        } else {
            t.radius("mini")
        };
        cover(ui.painter(), deck, art, radius);
        Rect::from_min_max(
            pos2(inner.left() + stage.side + 24.0, inner.top()),
            pos2(inner.right() - stage.side - 24.0, inner.bottom()),
        )
    };
    let size = if stage.tall {
        TransportSize::Md
    } else {
        TransportSize::Sm
    };
    let controls = 36.0;
    let row = Rect::from_min_max(pos2(middle.left(), middle.bottom() - controls), middle.max);
    let wave = Rect::from_min_max(
        pos2(middle.left(), row.top() - stage.gap - stage.wave),
        pos2(middle.right(), row.top() - stage.gap),
    );
    let pad_x = if expand { 32.0 } else { 0.0 };
    let words_rect = Rect::from_min_max(
        pos2(middle.left() + pad_x, middle.top()),
        pos2(middle.right() - pad_x, wave.top() - stage.gap),
    );
    words(ui, deck, stage.tier, words_rect, true);
    place(ui, wave, Layout::left_to_right(Align::Center), |ui| {
        deck.scrub(ui, stage.wave, false, true)
    });
    let (_, play) = sides(ui, size);
    let steps = deck.steps_width(ui, size, 8.0, !narrow);
    centered_row(ui, row.center().x, row.center().y, steps, play, 8.0, |ui| {
        deck.transport(ui, size, !narrow)
    });
    if narrow {
        end_row(
            ui,
            row.right(),
            row.center().y,
            row.center().x + steps / 2.0,
            play,
            |ui| {
                deck.speed(ui, TransportSize::Sm);
                deck.extras(ui);
            },
        );
    } else {
        let side = Rect::from_min_max(
            pos2(inner.right() - stage.side, row.top()),
            pos2(inner.right(), row.bottom()),
        );
        end_row(ui, side.right(), side.center().y, side.left(), play, |ui| {
            deck.speed(ui, TransportSize::Sm);
            deck.volume(ui, stage.compact_volume);
            deck.extras(ui);
        });
    }
    if expand {
        asked |= corner_expand(ui, deck, rect);
    }
    asked
}

/// What the fullscreen view was asked.
#[derive(Default)]
struct RoomAsked {
    leave: bool,
    lyrics: Option<bool>,
    option: Option<String>,
}

/// The fullscreen view's makings.
struct Room<'a, 'v> {
    layout: MediaBarLayout,
    visualizer: Paint<'v>,
    lyrics: Option<Draw<'a>>,
    lyrics_shown: bool,
    actions: Option<Draw<'a>>,
    options: &'a [MediaVisualizerOption],
    option: Option<String>,
}

/// The view's width under which the rows view puts its transport under what plays.
const TABLET: f32 = 1280.0;

impl Room<'_, '_> {
    /// The player's now-playing view over the whole window, after sokkel-ui-media's
    /// `MediaFullscreen`: a dark room in every theme (the theme's own dark tokens), arranged after
    /// the bar's layout, with the visualizer's picker, the lyrics button and the way out in the
    /// corner.
    fn show(mut self, ui: &mut Ui, deck: &mut Deck) -> RoomAsked {
        let theme = tokens(ui.ctx()).theme.clone();
        let screen = ui.ctx().content_rect();
        let mut asked = RoomAsked::default();
        egui::Area::new(deck.id.with("fullscreen"))
            .order(Order::Foreground)
            .fixed_pos(screen.min)
            .show(ui.ctx(), |ui| {
                scope(ui, &theme, Some(true), |ui| {
                    let t = tokens(ui.ctx());
                    let (rect, _) = ui.allocate_exact_size(screen.size(), Sense::click());
                    ui.painter()
                        .rect_filled(rect, CornerRadius::ZERO, t.color("background"));
                    let phone = rect.width() < PHONE;
                    let worded = self.lyrics.is_some() && self.lyrics_shown;
                    match self.layout {
                        MediaBarLayout::Rows => self.rows_view(ui, deck, rect, phone, worded),
                        MediaBarLayout::Panel => self.panel_view(ui, deck, rect, phone, worded),
                        MediaBarLayout::Stage => self.stage_view(ui, deck, rect, phone, worded),
                    }
                    self.corner(ui, deck, rect, phone, worded, &mut asked);
                });
            });
        asked
    }

    /// The corner: the speed on a phone, the lyrics button, the visualizer's picker and the way out.
    fn corner(&mut self, ui: &mut Ui, deck: &mut Deck, rect: Rect, phone: bool, worded: bool, asked: &mut RoomAsked) {
        let t = tokens(ui.ctx());
        let side = t.density("control-sm");
        let y = rect.top() + 12.0 + side / 2.0;
        end_row(ui, rect.right() - 12.0, y, rect.center().x, side, |ui| {
            if phone {
                deck.speed(ui, TransportSize::Md);
            }
            if self.lyrics.is_some()
                && ui
                    .add(TransportButton::new(Icon::MicVocal, &deck.words.lyrics).pressed(self.lyrics_shown))
                    .clicked()
            {
                asked.lyrics = Some(!self.lyrics_shown);
            }
            if !self.options.is_empty() && self.visualizer.is_some() && !worded {
                let at = self
                    .option
                    .as_deref()
                    .and_then(|o| self.options.iter().position(|x| x.id == o))
                    .unwrap_or(0);
                if phone {
                    let label = format!("{}: {}", deck.words.visualizer_kind, self.options[at].label);
                    if ui.add(TransportButton::new(Icon::AudioLines, label)).clicked() {
                        asked.option = Some(self.options[(at + 1) % self.options.len()].id.clone());
                    }
                } else {
                    let mut chosen = at;
                    if ui
                        .add(Segmented::new(
                            &mut chosen,
                            self.options.iter().map(|o| o.label.clone()),
                        ))
                        .changed()
                    {
                        asked.option = Some(self.options[chosen].id.clone());
                    }
                }
            }
            if ui
                .add(TransportButton::new(Icon::Minimize2, &deck.words.exit_fullscreen))
                .clicked()
            {
                asked.leave = true;
            }
        });
    }

    /// The lyrics while shown, or the visualizer, in `rect`.
    fn middle(&mut self, ui: &mut Ui, rect: Rect, worded: bool) {
        if worded {
            if let Some(lyrics) = self.lyrics.as_mut() {
                let mut child = ui.new_child(UiBuilder::new().max_rect(rect));
                child.set_clip_rect(rect);
                lyrics(&mut child);
            }
        } else if let Some(paint) = self.visualizer {
            paint(&ui.painter().with_clip_rect(rect), rect);
        }
    }

    /// What plays at a screen's size: the eyebrow, the title never cut short (a size smaller when it
    /// would run past two lines, three on a phone), and the line under it with the actions at its
    /// end. Its foot on `bottom`; gives its height.
    #[allow(clippy::too_many_arguments)]
    fn screen_words(
        &mut self,
        ui: &mut Ui,
        deck: &Deck,
        left: f32,
        right: f32,
        bottom: f32,
        phone: bool,
        center: bool,
    ) -> f32 {
        let t = tokens(ui.ctx());
        let width = (right - left).max(1.0);
        let title = |role: TextRole, key: &str, weight: f32, tight: bool| {
            let size = t.text(key).size;
            let rich = role
                .rich(&t, &deck.title)
                .variation("wght", weight)
                .color(t.color("foreground"));
            if tight {
                rich.extra_letter_spacing(-0.025 * size)
            } else {
                rich
            }
        };
        let sizes: Vec<RichText> = if phone {
            vec![
                title(TextRole::PageTitle, "page", 600.0, true),
                title(TextRole::DetailTitle, "detail-title", 600.0, false),
            ]
        } else {
            vec![
                title(TextRole::MediaTitle, "media-title", 600.0, true),
                title(TextRole::Statistic, "stat", 600.0, true),
                title(TextRole::PageTitle, "page", 600.0, true),
            ]
        };
        let most = if phone { 3 } else { 2 };
        let galleys: Vec<_> = sizes
            .into_iter()
            .map(|rich| wrapped(ui, rich, width, usize::MAX))
            .collect();
        let fits = galleys
            .iter()
            .position(|g| g.rows.len() <= most)
            .unwrap_or(galleys.len() - 1);
        let title = galleys[fits].clone();
        let brow = eyebrow(ui, deck, width, !phone);
        let by = deck.by().map(|by| {
            let role = if phone { TextRole::Body } else { TextRole::PageTitle };
            let rich = role.rich(&t, by).variation("wght", 400.0).extra_letter_spacing(0.0);
            one_line(
                ui.painter(),
                ui.style(),
                rich.color(t.color("muted-foreground")),
                width - if self.actions.is_some() { 80.0 } else { 0.0 },
            )
        });
        let by_height = by.as_ref().map_or(0.0, |g| g.size().y).max(if self.actions.is_some() {
            t.density("control-sm")
        } else {
            0.0
        });
        let height = brow.as_ref().map_or(0.0, |g| g.size().y) + title.size().y + by_height;
        let mut y = bottom - height;
        let x_of = |w: f32| if center { left + (width - w) / 2.0 } else { left };
        if let Some(brow) = brow {
            let h = brow.size().y;
            ui.painter()
                .galley(pos2(x_of(brow.size().x), y), brow, t.color("primary-ink"));
            y += h;
        }
        let h = title.size().y;
        ui.painter()
            .galley(pos2(x_of(title.size().x), y), title, t.color("foreground"));
        y += h;
        let by_width = by.as_ref().map_or(0.0, |g| g.size().x);
        let actions_width = if self.actions.is_some() { 80.0 } else { 0.0 };
        let line_left = x_of(
            by_width
                + if self.actions.is_some() {
                    8.0 + actions_width
                } else {
                    0.0
                },
        );
        if let Some(by) = by {
            ui.painter().galley(
                pos2(line_left, y + (by_height - by.size().y) / 2.0),
                by,
                t.color("muted-foreground"),
            );
        }
        if let Some(actions) = self.actions.as_mut() {
            let at = Rect::from_min_size(
                pos2(line_left + by_width + 8.0, y),
                vec2(actions_width + 200.0, by_height),
            );
            place(ui, at, Layout::left_to_right(Align::Center), |ui| actions(ui));
        }
        height
    }

    /// After `rows`: what plays across the head with the transport and the volume at its end, the
    /// visualizer (or the lyrics) filling the middle, the waveform along the foot.
    fn rows_view(&mut self, ui: &mut Ui, deck: &mut Deck, rect: Rect, phone: bool, worded: bool) {
        let tablet = !phone && rect.width() < TABLET;
        let px = if phone { 20.0 } else { 40.0 };
        let side = if phone { 72.0 } else { 144.0 };
        let head_top = rect.top() + 64.0;
        let art = Rect::from_min_size(pos2(rect.left() + px, head_top), vec2(side, side));
        cover_pop(ui, deck, art);
        let (_, play) = sides(ui, TransportSize::Lg);
        let steps = deck.steps_width(ui, TransportSize::Lg, 12.0, true);
        let words_right = if phone || tablet {
            rect.right() - px
        } else {
            rect.right() - px - steps - 32.0 - 220.0
        };
        self.screen_words(
            ui,
            deck,
            art.right() + if phone { 16.0 } else { 24.0 },
            words_right,
            art.bottom() - 2.0,
            phone,
            false,
        );
        let mut head_bottom = art.bottom();
        if !phone {
            let y = if tablet {
                art.bottom() + 24.0 + 28.0
            } else {
                art.bottom() - 28.0
            };
            let left = rect.left() + px;
            let row_left = if tablet { left } else { words_right + 32.0 };
            centered_row(ui, row_left + steps / 2.0, y, steps, play, 12.0, |ui| {
                deck.transport(ui, TransportSize::Lg, true)
            });
            end_row(ui, rect.right() - px, y, row_left + steps + 32.0, 56.0, |ui| {
                deck.speed(ui, TransportSize::Md);
                deck.volume(ui, false);
                deck.extras(ui);
            });
            if tablet {
                head_bottom = y + 28.0;
            }
        }
        let foot_height = if phone { 64.0 + 16.0 + play + 24.0 } else { 112.0 + 40.0 };
        let middle = Rect::from_min_max(
            pos2(rect.left() + px, head_bottom + if phone { 20.0 } else { 32.0 }),
            pos2(
                rect.right() - px,
                rect.bottom() - foot_height - if phone { 20.0 } else { 32.0 },
            ),
        );
        self.middle(ui, middle, worded);
        let wave_h = if phone { 64.0 } else { 112.0 };
        let wave = Rect::from_min_size(
            pos2(rect.left() + px, rect.bottom() - foot_height),
            vec2(rect.width() - 2.0 * px, wave_h),
        );
        place(ui, wave, Layout::left_to_right(Align::Center), |ui| {
            deck.scrub(ui, wave_h, !phone, true)
        });
        if phone {
            centered_row(
                ui,
                rect.center().x,
                wave.bottom() + 16.0 + play / 2.0,
                steps,
                play,
                8.0,
                |ui| deck.transport(ui, TransportSize::Lg, true),
            );
        }
    }

    /// After `panel`: the liner at full size in the lower left over the visualizer, which fades into
    /// the ground under the words, with the waveform and the transport along the foot.
    fn panel_view(&mut self, ui: &mut Ui, deck: &mut Deck, rect: Rect, phone: bool, worded: bool) {
        let t = tokens(ui.ctx());
        let px = if phone { 20.0 } else { 40.0 };
        let (_, play) = sides(ui, TransportSize::Lg);
        let wave_h = if phone { 44.0 } else { 64.0 };
        let foot_height = if phone {
            20.0 + wave_h + 20.0 + play + 24.0
        } else {
            28.0 + wave_h + 28.0 + play + 32.0
        };
        let top_room = Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - foot_height));
        let side = if phone { 88.0 } else { 208.0 };
        let art = Rect::from_min_size(pos2(rect.left() + px, top_room.bottom() - side), vec2(side, side));
        if !worded {
            let area = Rect::from_min_max(
                pos2(rect.left() + if phone { 12.0 } else { 32.0 }, rect.top() + 64.0),
                top_room.max,
            );
            self.middle(ui, area, false);
            // the visualizer fades into the ground under the words
            let fade = Rect::from_min_max(pos2(rect.left(), top_room.center().y), top_room.max);
            let ground = t.color("background");
            let mut mesh = Mesh::default();
            mesh.colored_vertex(fade.left_top(), Color32::TRANSPARENT);
            mesh.colored_vertex(fade.right_top(), Color32::TRANSPARENT);
            mesh.colored_vertex(fade.right_bottom(), ground);
            mesh.colored_vertex(fade.left_bottom(), ground);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            ui.painter().add(Shape::mesh(mesh));
        }
        cover_pop(ui, deck, art);
        let words_height = self.screen_words(
            ui,
            deck,
            art.right() + if phone { 16.0 } else { 28.0 },
            rect.right() - px,
            art.bottom() - 4.0,
            phone,
            false,
        );
        if worded {
            let top = art.top().min(art.bottom() - 4.0 - words_height) - 24.0;
            let area = Rect::from_min_max(
                pos2(rect.left() + if phone { 4.0 } else { 16.0 }, rect.top() + 64.0),
                pos2(rect.right() - 16.0, top),
            );
            self.middle(ui, area, true);
        }
        let wave = Rect::from_min_size(
            pos2(rect.left() + px, top_room.bottom() + if phone { 20.0 } else { 28.0 }),
            vec2(rect.width() - 2.0 * px, wave_h),
        );
        place(ui, wave, Layout::left_to_right(Align::Center), |ui| {
            deck.scrub(ui, wave_h, !phone, true)
        });
        let y = wave.bottom() + if phone { 20.0 } else { 28.0 } + play / 2.0;
        let steps = deck.steps_width(ui, TransportSize::Lg, 8.0, true);
        let x = if phone {
            rect.center().x
        } else {
            rect.left() + px + steps / 2.0
        };
        centered_row(ui, x, y, steps, play, 8.0, |ui| {
            deck.transport(ui, TransportSize::Lg, true)
        });
        if !phone {
            end_row(ui, rect.right() - px, y, rect.left() + px + steps + 16.0, play, |ui| {
                deck.speed(ui, TransportSize::Md);
                deck.volume(ui, false);
                deck.extras(ui);
            });
        }
    }

    /// After `stage`: everything on the screen's middle line, the cover standing inside the
    /// visualizer, the words, the waveform, the transport and the volume under it.
    fn stage_view(&mut self, ui: &mut Ui, deck: &mut Deck, rect: Rect, phone: bool, worded: bool) {
        let px = if phone { 20.0 } else { 40.0 };
        let band = if phone {
            (rect.width() * 0.9).min(rect.height() * 0.4)
        } else {
            (rect.height() * 0.46).min(440.0)
        }
        .round();
        let (_, play) = sides(ui, TransportSize::Lg);
        let column = if phone {
            rect.width() - 2.0 * px
        } else {
            (rect.width() - 2.0 * px).min(768.0)
        };
        let wave_h = if phone { 44.0 } else { 56.0 };
        let gap = if phone { 16.0 } else { 20.0 };
        let words_guess = 22.0 + 2.0 * 56.0 + 32.0;
        let under = words_guess + gap + wave_h + gap + play + if phone { 0.0 } else { gap + 36.0 };
        let total = band + if phone { 24.0 } else { 32.0 } + under;
        let top = (rect.top() + if phone { 56.0 } else { 64.0 }).max(rect.center().y - total / 2.0);
        let band_rect = Rect::from_min_size(pos2(rect.left(), top), vec2(rect.width(), band));
        if worded {
            self.middle(ui, Rect::from_center_size(band_rect.center(), vec2(column, band)), true);
        } else {
            self.middle(ui, band_rect, false);
            let side = band * 0.78;
            cover_pop(ui, deck, Rect::from_center_size(band_rect.center(), vec2(side, side)));
        }
        let left = rect.center().x - column / 2.0;
        let words_bottom = band_rect.bottom() + if phone { 24.0 } else { 32.0 } + words_guess;
        self.screen_words(ui, deck, left, left + column, words_bottom, phone, true);
        let wave = Rect::from_min_size(pos2(left, words_bottom + gap), vec2(column, wave_h));
        place(ui, wave, Layout::left_to_right(Align::Center), |ui| {
            deck.scrub(ui, wave_h, !phone, true)
        });
        let steps = deck.steps_width(ui, TransportSize::Lg, 12.0, true);
        let y = wave.bottom() + gap + play / 2.0;
        centered_row(ui, rect.center().x, y, steps, play, 12.0, |ui| {
            deck.transport(ui, TransportSize::Lg, true)
        });
        if !phone {
            let y = y + play / 2.0 + gap + 18.0;
            let width = 32.0 + 8.0 + 128.0 + 8.0 + 60.0;
            end_row(
                ui,
                rect.center().x + width / 2.0,
                y,
                rect.center().x - width / 2.0,
                36.0,
                |ui| {
                    deck.speed(ui, TransportSize::Md);
                    deck.volume(ui, false);
                    deck.extras(ui);
                },
            );
        }
    }
}

/// A screen's cover: the surface's corners and the pop's shadow.
fn cover_pop(ui: &Ui, deck: &Deck, rect: Rect) {
    let t = tokens(ui.ctx());
    let radius = t.radius("surface");
    ui.painter().add(t.shadow("elevation-pop").as_shape(rect, radius));
    let mut ground = ui.painter().clone();
    ground.set_clip_rect(ui.clip_rect());
    cover_on(&ground, deck, rect, radius, t.color("background"));
}

/// The cover with its corners rounded on `ground`.
fn cover_on(painter: &Painter, deck: &Deck, rect: Rect, radius: CornerRadius, ground: Color32) {
    let t = tokens(painter.ctx());
    painter.rect_filled(rect, radius, t.color("muted"));
    match (deck.artwork, &deck.seed) {
        (Some(c), _) => Artwork::of(c).paint(painter, rect),
        (None, Some(seed)) => Artwork::new(seed).paint(painter, rect),
        (None, None) => icons::paint(
            painter,
            rect.center(),
            Icon::Music,
            (rect.width() * 0.4).min(48.0),
            t.color("faint"),
        ),
    }
    round_corners(painter, rect, radius, ground);
}
