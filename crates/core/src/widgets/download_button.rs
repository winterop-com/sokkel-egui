//! A button that downloads something for keeping, after sokkel-ui's `DownloadButton`, and the
//! ring it fills while it runs, after `ui/progress-ring`.

use std::f32::consts::{PI, TAU};

use egui::{Id, Painter, Pos2, Response, Sense, Shape, Stroke, Ui, Widget, pos2, vec2};

use crate::{Button, ButtonSize, ButtonVariant, Icon, IconButton, Tokens, tokens};

/// Where a download stands: not started, under way, held part way, or done and kept on the device.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DownloadState {
    #[default]
    Idle,
    Running,
    Paused,
    Kept,
}

/// The words a download button says, each with an English default.
#[derive(Clone, Debug)]
pub struct DownloadButtonWords {
    pub download: String,
    /// What a running download says, given how many of how many parts are in: `{count}` and `{total}`.
    pub progress: String,
    /// What a running download with no count and no share says.
    pub running: String,
    /// What a paused download says.
    pub paused: String,
    pub pause: String,
    pub resume: String,
    pub cancel: String,
    pub kept: String,
    /// What a kept copy says once pressed to remove, waiting for the second press.
    pub remove: String,
    /// The kept button's name with a removal offered, and once armed.
    pub remove_label: String,
    pub remove_armed_label: String,
}

impl Default for DownloadButtonWords {
    fn default() -> Self {
        Self {
            download: "Download".into(),
            progress: "{count} of {total}".into(),
            running: "Downloading".into(),
            paused: "Paused".into(),
            pause: "Pause the download".into(),
            resume: "Resume the download".into(),
            cancel: "Cancel the download".into(),
            kept: "Kept".into(),
            remove: "Remove?".into(),
            remove_label: "Kept on this device. Remove it".into(),
            remove_armed_label: "Press again to remove this download".into(),
        }
    }
}

/// What a press on a download button asks of the app, which then moves `state` along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DownloadAsked {
    Download,
    Cancel,
    Pause,
    Resume,
    /// Show the kept copy.
    Open,
    /// Remove the kept copy: the second press on an armed button.
    Remove,
}

/// What a download button did this frame.
pub struct DownloadButtonResponse {
    /// The main button's (the status's while it runs).
    pub response: Response,
    pub asked: Option<DownloadAsked>,
}

/// How long a first press to remove stays armed, in seconds.
const ARMED_FOR: f64 = 4.0;

/// How far along a download is, from 0 to 1: the share given, else the count over the total.
pub fn download_share(value: Option<f32>, count: Option<u32>, total: Option<u32>) -> f32 {
    if let Some(value) = value {
        return value.clamp(0.0, 1.0);
    }
    match (count, total) {
        (Some(count), Some(total)) if total > 0 => (count as f32 / total as f32).clamp(0.0, 1.0),
        _ => 0.0,
    }
}

/// A share from 0 to 1 written as a whole percent: `0.42` is `42%`. A share above nothing that
/// rounds to 0 is `<1%`, and one short of everything that rounds to 100 is `>99%`, so neither
/// reads as certain. Values outside 0 to 1 are held to it.
pub fn format_percent(value: f32) -> String {
    let share = if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.0 };
    let whole = (share * 100.0).round() as u32;
    if share > 0.0 && whole == 0 {
        "<1%".into()
    } else if share < 1.0 && whole == 100 {
        ">99%".into()
    } else {
        format!("{whole}%")
    }
}

/// A button that downloads something for keeping, through its states, which the app holds:
/// idle says Download; running fills a [`ProgressRing`] and says how many of how many parts are
/// in (`count` and `total`), or the share done (`value` alone, for bytes), with a button beside it
/// to cancel (`cancellable`) and one to pause (`pausable`); paused holds the ring where it stopped,
/// says Paused, and offers resume (`resumable`) and cancel; kept says Kept with a check, and
/// pressing it asks to open the kept copy (`opens`) or to download again.
///
/// With `removable`, pressing a kept copy removes it in two presses: the first arms it (it says
/// Remove? in the critical ink, for a few seconds or until a press elsewhere), the second asks to
/// remove. `kept_detail` says on hover what is kept, and `error` says on hover why the last try
/// failed, the idle button's icon tinted. `compact` draws the icon alone, always or only on a
/// phone, the words staying for a screen reader.
#[must_use = "show it with `.show(ui)`"]
pub struct DownloadButton {
    id: Id,
    state: DownloadState,
    count: Option<u32>,
    total: Option<u32>,
    value: Option<f32>,
    cancellable: bool,
    pausable: bool,
    resumable: bool,
    opens: bool,
    removable: bool,
    kept_detail: Option<String>,
    error: Option<String>,
    compact: bool,
    compact_on_phone: bool,
    armed: bool,
    variant: ButtonVariant,
    size: ButtonSize,
    words: DownloadButtonWords,
}

impl DownloadButton {
    pub fn new(id: impl egui::AsIdSalt, state: DownloadState) -> Self {
        Self {
            id: Id::unique(("sokkel-download-button", id)),
            state,
            count: None,
            total: None,
            value: None,
            cancellable: false,
            pausable: false,
            resumable: false,
            opens: false,
            removable: false,
            kept_detail: None,
            error: None,
            compact: false,
            compact_on_phone: false,
            armed: false,
            variant: ButtonVariant::Outline,
            size: ButtonSize::Default,
            words: DownloadButtonWords::default(),
        }
    }

    /// How many of how many parts are in, such as an album's songs.
    pub fn parts(mut self, count: u32, total: u32) -> Self {
        self.count = Some(count);
        self.total = Some(total);
        self
    }

    /// The share done, 0 to 1, when there are no parts to count.
    pub fn value(mut self, value: f32) -> Self {
        self.value = Some(value);
        self
    }

    /// Offers a cancel beside a running or paused download.
    pub fn cancellable(mut self) -> Self {
        self.cancellable = true;
        self
    }

    /// Offers a pause beside a running download; the app then holds it as paused.
    pub fn pausable(mut self) -> Self {
        self.pausable = true;
        self
    }

    /// Offers a resume beside a paused download.
    pub fn resumable(mut self) -> Self {
        self.resumable = true;
        self
    }

    /// A press on a kept copy asks to open it, rather than to download again.
    pub fn opens(mut self) -> Self {
        self.opens = true;
        self
    }

    /// A kept copy is removed after a confirming second press.
    pub fn removable(mut self) -> Self {
        self.removable = true;
        self
    }

    /// What the kept copy is, said on hover: its size, where it is kept.
    pub fn kept_detail(mut self, detail: impl Into<String>) -> Self {
        self.kept_detail = Some(detail.into());
        self
    }

    /// Why the last download failed, said on hover of the idle button.
    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// The icon alone, the words said to a screen reader only.
    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    /// The icon alone on a phone only, as the web's `compact="narrow"`.
    pub fn compact_on_phone(mut self) -> Self {
        self.compact_on_phone = true;
        self
    }

    /// Starts armed to remove, waiting for the second press.
    pub fn armed(mut self, armed: bool) -> Self {
        self.armed = armed;
        self
    }

    /// `Default`, `Outline` (the default), `Secondary` or `Ghost`.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// `Sm`, `Default` or `Lg`.
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn words(mut self, words: DownloadButtonWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> DownloadButtonResponse {
        let compact = self.compact || (self.compact_on_phone && super::is_phone(ui.ctx()));
        let armed_id = self.id.with("armed");
        let now = ui.input(|i| i.time);
        // when the arming was pressed, kept while the copy is kept and for a few seconds
        let mut armed_at: Option<f64> = ui.data(|d| d.get_temp(armed_id));
        if self.armed && armed_at.is_none() && !ui.data(|d| d.get_temp::<bool>(self.id.with("seen")).unwrap_or(false)) {
            armed_at = Some(f64::INFINITY);
        }
        ui.data_mut(|d| d.insert_temp(self.id.with("seen"), true));
        if self.state != DownloadState::Kept || armed_at.is_some_and(|at| at.is_finite() && now - at > ARMED_FOR) {
            armed_at = None;
        }
        let mut asked = None;
        let response = match self.state {
            DownloadState::Running | DownloadState::Paused => {
                let said = if self.state == DownloadState::Paused {
                    self.words.paused.clone()
                } else if let (Some(count), Some(total)) = (self.count, self.total) {
                    self.words
                        .progress
                        .replace("{count}", &count.to_string())
                        .replace("{total}", &total.to_string())
                } else if let Some(value) = self.value {
                    format_percent(value)
                } else {
                    self.words.running.clone()
                };
                let share = download_share(self.value, self.count, self.total);
                let inner = ui.horizontal(|ui| {
                    // `gap-0.5`
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let mut status = Button::new(said).variant(self.variant).size(self.size);
                    status.ring = Some(share);
                    status.still = true;
                    let response = ui.add(status);
                    let icon = |icon: Icon, label: &str| {
                        let button = IconButton::new(icon, label);
                        if self.size == ButtonSize::Sm {
                            button
                        } else {
                            button.md()
                        }
                    };
                    if self.state == DownloadState::Running
                        && self.pausable
                        && ui.add(icon(Icon::Pause, &self.words.pause)).clicked()
                    {
                        asked = Some(DownloadAsked::Pause);
                    }
                    if self.state == DownloadState::Paused
                        && self.resumable
                        && ui.add(icon(Icon::Play, &self.words.resume)).clicked()
                    {
                        asked = Some(DownloadAsked::Resume);
                    }
                    if self.cancellable && ui.add(icon(Icon::X, &self.words.cancel)).clicked() {
                        asked = Some(DownloadAsked::Cancel);
                    }
                    response
                });
                inner.inner
            }
            DownloadState::Kept => {
                let t = tokens(ui.ctx());
                let armed = self.removable && armed_at.is_some();
                let label = match (self.removable, armed) {
                    (true, true) => self.words.remove_armed_label.clone(),
                    (true, false) => self.words.remove_label.clone(),
                    _ => self.words.kept.clone(),
                };
                let mut button = Button::new(if armed { &self.words.remove } else { &self.words.kept })
                    .variant(ButtonVariant::Ghost)
                    .size(self.size)
                    .icon(Icon::Check);
                button.unseen_label = compact;
                if armed {
                    button.ink = Some(t.color("critical-ink"));
                } else {
                    button.icon_ink = Some(t.color("good-ink"));
                }
                let response = ui.add(button);
                // the name a screen reader hears: what a press does
                response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
                let hover = if armed {
                    Some(self.words.remove_armed_label.clone())
                } else {
                    self.kept_detail.clone()
                };
                let response = match hover {
                    Some(text) => super::tooltip(response, text),
                    None => response,
                };
                if response.clicked() {
                    if self.removable {
                        if armed {
                            armed_at = None;
                            asked = Some(DownloadAsked::Remove);
                        } else {
                            armed_at = Some(now);
                        }
                    } else {
                        asked = Some(if self.opens {
                            DownloadAsked::Open
                        } else {
                            DownloadAsked::Download
                        });
                    }
                } else if armed && ui.input(|i| i.pointer.any_pressed()) && !response.hovered() {
                    // a press elsewhere disarms it, as focus leaving does
                    armed_at = None;
                }
                if let Some(at) = armed_at.filter(|at| at.is_finite()) {
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_secs_f64((ARMED_FOR - (now - at)).max(0.0)));
                }
                response
            }
            DownloadState::Idle => {
                let t = tokens(ui.ctx());
                let mut button = Button::new(&self.words.download)
                    .variant(self.variant)
                    .size(self.size)
                    .icon(Icon::Download);
                button.unseen_label = compact;
                if self.error.is_some() {
                    button.icon_ink = Some(t.color("critical-ink"));
                }
                let response = ui.add(button);
                let response = match &self.error {
                    Some(error) => super::tooltip(response, error.clone()),
                    None => response,
                };
                if response.clicked() {
                    asked = Some(DownloadAsked::Download);
                }
                response
            }
        };
        ui.data_mut(|d| match armed_at {
            Some(at) => {
                d.insert_temp(armed_id, at);
            }
            None => d.remove::<f64>(armed_id),
        });
        DownloadButtonResponse { response, asked }
    }
}

/// How far a piece of work has come, drawn as a ring that fills clockwise from the top, after
/// sokkel-ui's `ProgressRing`: an upload, a download. `value` runs from 0 to 1. 16 across: a 2
/// wide ring of radius 6 in `muted`, the share drawn over it in `primary` with round ends.
#[must_use = "add it with `ui.add(...)`"]
pub struct ProgressRing {
    value: f32,
    size: f32,
}

impl ProgressRing {
    pub fn new(value: f32) -> Self {
        Self { value, size: 16.0 }
    }

    /// How wide it is drawn, 16 by default.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
}

impl Widget for ProgressRing {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(vec2(self.size, self.size), Sense::hover());
        let t = tokens(ui.ctx());
        paint_progress_ring(ui.painter(), &t, rect.center(), self.size, self.value);
        response
    }
}

/// The progress ring `size` across around `center`, at `value` from 0 to 1.
pub(crate) fn paint_progress_ring(painter: &Painter, t: &Tokens, center: Pos2, size: f32, value: f32) {
    let share = if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.0 };
    let scale = size / 16.0;
    let radius = 6.0 * scale;
    let width = 2.0 * scale;
    painter.circle_stroke(center, radius, Stroke::new(width, t.color("muted")));
    if share <= 0.0 {
        return;
    }
    let ink = t.color("primary");
    let sweep = TAU * share;
    let steps = ((sweep / TAU) * 48.0).ceil().max(2.0) as usize;
    let points: Vec<Pos2> = (0..=steps)
        .map(|step| {
            let angle = -PI / 2.0 + sweep * step as f32 / steps as f32;
            pos2(center.x + radius * angle.cos(), center.y + radius * angle.sin())
        })
        .collect();
    // round ends, as `stroke-linecap="round"`
    let ends = [points[0], points[points.len() - 1]];
    painter.add(Shape::line(points, Stroke::new(width, ink)));
    for end in ends {
        painter.circle_filled(end, width / 2.0, ink);
    }
}
