//! How a video is playing, in a few lines over its picture, after sokkel-ui-media's
//! `VideoStatsOverlay` and the core's `media/video-stats` it reads.

use egui::{Align, Layout, Sense, Ui, pos2, vec2};

use sokkel_egui::numbers::{locale_number, to_fixed};
use sokkel_egui::{Icon, IconButton, TextRole, mono_font, mono_size, tokens};

/// How a video is playing, as a player reads it once a second: the picture's size, the rendition's
/// bitrate, the link's bandwidth, the seconds buffered ahead, the frames dropped, and the rendition
/// among those an engine picks from. A fact nobody can tell is `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VideoStats {
    pub resolution: Option<(u32, u32)>,
    pub bitrate: Option<f64>,
    pub bandwidth: Option<f64>,
    pub buffer_ahead: f64,
    pub dropped_frames: Option<u64>,
    pub total_frames: Option<u64>,
    pub level: Option<usize>,
    pub levels: Option<usize>,
}

/// A bitrate as `3.2 Mbit/s` or `850 kbit/s`; `none` when nobody can tell.
pub fn format_bitrate(bits: Option<f64>, none: &str) -> String {
    match bits {
        Some(bits) if bits.is_finite() => {
            if bits >= 1e6 {
                format!("{} Mbit/s", to_fixed(bits / 1e6, 1))
            } else {
                format!("{} kbit/s", (bits / 1e3).round())
            }
        }
        _ => none.to_owned(),
    }
}

/// A picture's size as `1920×1080`; `none` before the first frame.
pub fn format_resolution(resolution: Option<(u32, u32)>, none: &str) -> String {
    resolution.map_or_else(|| none.to_owned(), |(w, h)| format!("{w}\u{d7}{h}"))
}

/// The words the overlay says.
#[derive(Clone, Debug)]
pub struct VideoStatsOverlayWords {
    pub title: String,
    pub resolution: String,
    pub bitrate: String,
    pub bandwidth: String,
    pub buffer: String,
    pub dropped_frames: String,
    pub level: String,
    pub close: String,
    /// Said for a fact nobody can tell yet.
    pub unknown: String,
    /// Joins the frames dropped to the frames shown, and the rendition to how many there are.
    pub of: String,
    /// Said before the first reading.
    pub waiting: String,
}

impl Default for VideoStatsOverlayWords {
    fn default() -> Self {
        Self {
            title: "Stats".into(),
            resolution: "Resolution".into(),
            bitrate: "Bitrate".into(),
            bandwidth: "Bandwidth".into(),
            buffer: "Buffer ahead".into(),
            dropped_frames: "Dropped frames".into(),
            level: "Rendition".into(),
            close: "Close stats".into(),
            unknown: "-".into(),
            of: "of".into(),
            waiting: "Waiting for the player".into(),
        }
    }
}

/// How a headline reads: as plain news, or good, worrying or bad news, in the ink of each.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VideoStatsTone {
    #[default]
    Neutral,
    Good,
    Warning,
    Critical,
}

/// How a video is playing, after sokkel-ui-media's `VideoStatsOverlay`: 256 wide on `terminal/80`
/// with the surface's corners and the pop shadow, in `text.help`: the title (or the app's
/// `headline` in its tone's ink) with a close button, then the facts, each name in
/// `terminal-muted` and its value at the end in the code face: the player's, then the app's own
/// `rows`. Shows it through `VideoPlayer::overlay`, so it stays over the picture in full screen.
#[must_use = "show it with `.show(ui)`"]
pub struct VideoStatsOverlay<'a> {
    stats: Option<VideoStats>,
    headline: Option<(String, VideoStatsTone)>,
    rows: &'a [(String, String)],
    closable: bool,
    words: VideoStatsOverlayWords,
}

impl<'a> VideoStatsOverlay<'a> {
    pub fn new(stats: Option<VideoStats>) -> Self {
        Self {
            stats,
            headline: None,
            rows: &[],
            closable: false,
            words: VideoStatsOverlayWords::default(),
        }
    }

    /// A verdict in one sentence in the title's place, in the tone's ink.
    pub fn headline(mut self, headline: impl Into<String>, tone: VideoStatsTone) -> Self {
        self.headline = Some((headline.into(), tone));
        self
    }

    /// The app's own facts, after the player's.
    pub fn rows(mut self, rows: &'a [(String, String)]) -> Self {
        self.rows = rows;
        self
    }

    /// A close button, as when the app toggles the overlay from a key or a menu.
    pub fn closable(mut self, on: bool) -> Self {
        self.closable = on;
        self
    }

    pub fn words(mut self, words: VideoStatsOverlayWords) -> Self {
        self.words = words;
        self
    }

    /// Draws it; true when its close button was pressed.
    pub fn show(self, ui: &mut Ui) -> bool {
        let t = tokens(ui.ctx());
        let words = &self.words;
        let none = words.unknown.as_str();
        let mut rows: Vec<(String, String)> = match self.stats {
            None => Vec::new(),
            Some(stats) => {
                let mut rows = vec![
                    (words.resolution.clone(), format_resolution(stats.resolution, none)),
                    (words.bitrate.clone(), format_bitrate(stats.bitrate, none)),
                    (words.bandwidth.clone(), format_bitrate(stats.bandwidth, none)),
                    (words.buffer.clone(), format!("{} s", to_fixed(stats.buffer_ahead, 1))),
                    (
                        words.dropped_frames.clone(),
                        match stats.dropped_frames {
                            None => none.to_owned(),
                            Some(dropped) => format!(
                                "{} {} {}",
                                locale_number(dropped as f64),
                                words.of,
                                locale_number(stats.total_frames.unwrap_or(0) as f64)
                            ),
                        },
                    ),
                ];
                if let (Some(level), Some(levels)) = (stats.level, stats.levels)
                    && levels > 1
                {
                    rows.insert(2, (words.level.clone(), format!("{} {} {levels}", level + 1, words.of)));
                }
                rows
            }
        };
        rows.extend(self.rows.iter().cloned());
        let mut closed = false;
        let width = 256.0_f32.min(ui.available_width());
        egui::Frame::new()
            .fill(t.color_at("terminal", 0.8))
            .corner_radius(t.radius("surface"))
            .shadow(t.shadow("elevation-pop"))
            .inner_margin(egui::Margin {
                left: 12,
                right: 6,
                top: 6,
                bottom: 6,
            })
            .show(ui, |ui| {
                ui.set_width(width - 18.0);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                let inner = ui.available_width();
                // the head: the headline or the title, and the close button, 28 tall
                ui.allocate_ui_with_layout(vec2(inner, 28.0), Layout::left_to_right(Align::Center), |ui| {
                    let (text, ink) = match &self.headline {
                        Some((headline, tone)) => (
                            headline.clone(),
                            match tone {
                                VideoStatsTone::Neutral => t.color("terminal-foreground"),
                                VideoStatsTone::Good => t.color("status-succeeded"),
                                VideoStatsTone::Warning => t.color("terminal-warning"),
                                VideoStatsTone::Critical => t.color("terminal-critical"),
                            },
                        ),
                        None => (words.title.clone(), t.color("terminal-foreground")),
                    };
                    let close = if self.closable { 28.0 + 8.0 } else { 0.0 };
                    let galley = egui::WidgetText::from(TextRole::Badge.rich(&t, text)).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Truncate),
                        inner - close,
                        TextRole::Badge.font(&t),
                    );
                    let (r, _) = ui.allocate_exact_size(vec2(inner - close, 28.0), Sense::hover());
                    ui.painter()
                        .galley(pos2(r.left(), r.center().y - galley.size().y / 2.0), galley, ink);
                    if self.closable {
                        ui.add_space(8.0);
                        closed = ui
                            .add(IconButton::new(Icon::X, &words.close).on_picture(true))
                            .clicked();
                    }
                });
                if self.stats.is_none() {
                    let galley = egui::WidgetText::from(TextRole::Help.rich(&t, &words.waiting)).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Truncate),
                        inner,
                        TextRole::Help.font(&t),
                    );
                    let (r, _) = ui.allocate_exact_size(vec2(inner, galley.size().y + 4.0), Sense::hover());
                    ui.painter().galley(r.min, galley, t.color("terminal-muted"));
                }
                let spec = t.text("help");
                let mono = mono_font(ui.ctx(), mono_size(&t, spec.size), 400.0);
                for (name, value) in &rows {
                    let (r, _) = ui.allocate_exact_size(vec2(inner - 6.0, spec.line_height), Sense::hover());
                    let name = egui::WidgetText::from(TextRole::Help.rich(&t, name)).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Truncate),
                        r.width() * 0.5,
                        TextRole::Help.font(&t),
                    );
                    ui.painter().galley(
                        pos2(r.left(), r.center().y - name.size().y / 2.0),
                        name.clone(),
                        t.color("terminal-muted"),
                    );
                    let value =
                        ui.painter()
                            .layout_no_wrap(value.clone(), mono.clone(), t.color("terminal-foreground"));
                    ui.painter().galley(
                        pos2(r.right() - value.size().x, r.center().y - value.size().y / 2.0),
                        value,
                        t.color("terminal-foreground"),
                    );
                }
                ui.add_space(4.0);
            });
        closed
    }
}
