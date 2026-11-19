//! What plays next, offered over the picture as a video ends, after sokkel-ui-media's `UpNext`.

use egui::{Align, FontSelection, Layout, Painter, Rect, Sense, Ui, pos2, text::LayoutJob, text::TextWrapping, vec2};

use sokkel_egui::{Button, ButtonVariant, Icon, TextRole, tokens};

use crate::artwork::{Artwork, PicturePaint};

/// The words the part says.
#[derive(Clone, Debug)]
pub struct UpNextWords {
    /// Said over the title.
    pub heading: String,
    pub play_now: String,
    pub cancel: String,
    /// How long until it plays by itself: `Playing in {seconds} s`.
    pub playing_in: String,
}

impl Default for UpNextWords {
    fn default() -> Self {
        Self {
            heading: "Up next".into(),
            play_now: "Play now".into(),
            cancel: "Cancel".into(),
            playing_in: "Playing in {seconds} s".into(),
        }
    }
}

/// What was asked this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UpNextResponse {
    /// Play now was pressed.
    pub play: bool,
    /// Cancel was pressed: the viewer stays.
    pub cancel: bool,
}

/// What plays next, after sokkel-ui-media's `UpNext`: drawn light on dark for a `VideoPlayer`'s
/// cover, at most 576 wide: its artwork (two fifths of the width, sixteen by nine, on the surface's
/// corners with the pop shadow, once the part is 448 wide), then the eyebrow, the title in
/// `text.section-title` on up to two lines (`text.page`-sized once 768 wide), the line under it and
/// the countdown in `text.help` and `terminal-muted`, and Play now with Cancel 12 under them. The
/// countdown is the app's: it says how many seconds are left and plays at zero.
#[must_use = "show it with `.show(ui)`"]
pub struct UpNext<'a> {
    title: String,
    subtitle: Option<String>,
    cover: Option<PicturePaint<'a>>,
    seed: Option<String>,
    countdown: Option<f64>,
    cancel: bool,
    words: UpNextWords,
}

impl<'a> UpNext<'a> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            cover: None,
            seed: None,
            countdown: None,
            cancel: false,
            words: UpNextWords::default(),
        }
    }

    /// Which episode it is, or who made it.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Its artwork, painted by the app into a wide frame; without it a cover is drawn from
    /// `placeholder_seed`, the title by default.
    pub fn cover(mut self, paint: impl Fn(&Painter, Rect) + 'a) -> Self {
        self.cover = Some(Box::new(paint));
        self
    }

    pub fn placeholder_seed(mut self, seed: impl Into<String>) -> Self {
        self.seed = Some(seed.into());
        self
    }

    /// Seconds until it plays by itself, as the app counts them; without it the part waits.
    pub fn countdown(mut self, seconds: Option<f64>) -> Self {
        self.countdown = seconds;
        self
    }

    /// Offers Cancel.
    pub fn cancellable(mut self, on: bool) -> Self {
        self.cancel = on;
        self
    }

    pub fn words(mut self, words: UpNextWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> UpNextResponse {
        let t = tokens(ui.ctx());
        let mut out = UpNextResponse::default();
        let room = ui.available_rect_before_wrap();
        let width = room.width().min(576.0);
        let pictured = width >= 448.0;
        let art_width = if pictured { width * 0.4 } else { 0.0 };
        let words_width = width - if pictured { art_width + 16.0 } else { 0.0 };
        let title_role = if width >= 768.0 {
            TextRole::PageTitle
        } else {
            TextRole::SectionTitle
        };
        let mut job = LayoutJob::default();
        title_role
            .rich(&t, &self.title)
            .append_to(&mut job, ui.style(), FontSelection::Default, Align::Min);
        job.wrap = TextWrapping {
            max_width: words_width,
            max_rows: 2,
            break_anywhere: false,
            overflow_character: Some('\u{2026}'),
        };
        let title = ui.fonts_mut(|f| f.layout_job(job));
        let help = |text: &str| {
            egui::WidgetText::from(TextRole::Help.rich(&t, text)).into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                words_width,
                TextRole::Help.font(&t),
            )
        };
        let spec = t.text("eyebrow");
        let eyebrow = egui::WidgetText::from(
            TextRole::Eyebrow
                .rich(&t, self.words.heading.to_uppercase())
                .extra_letter_spacing(spec.size * 0.18),
        )
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            words_width,
            TextRole::Eyebrow.font(&t),
        );
        let subtitle = self.subtitle.as_deref().map(help);
        let counting = self.countdown.filter(|seconds| *seconds > 0.0).map(|seconds| {
            help(
                &self
                    .words
                    .playing_in
                    .replace("{seconds}", &seconds.ceil().max(0.0).to_string()),
            )
        });
        let button = t.density("control");
        let words_height = eyebrow.size().y
            + title.size().y
            + subtitle.as_ref().map_or(0.0, |g| g.size().y)
            + counting.as_ref().map_or(0.0, |g| g.size().y)
            + 12.0
            + button;
        let art_height = art_width * 9.0 / 16.0;
        let height = words_height.max(art_height);
        let area = Rect::from_center_size(room.center(), vec2(width, height));
        let painter = ui.painter().clone();
        let muted = t.color("terminal-muted");
        let ink = t.color("terminal-foreground");
        if pictured {
            let art = Rect::from_min_size(
                pos2(area.left(), area.center().y - art_height / 2.0),
                vec2(art_width, art_height),
            );
            let radius = t.radius("surface");
            painter.add(t.shadow("elevation-pop").as_shape(art, radius));
            let clipped = painter.with_clip_rect(art.intersect(painter.clip_rect()));
            match &self.cover {
                Some(paint) => paint(&clipped, art),
                None => Artwork::new(self.seed.as_deref().unwrap_or(&self.title))
                    .wide(true)
                    .paint(&clipped, art),
            }
        }
        let left = area.left() + if pictured { art_width + 16.0 } else { 0.0 };
        let mut y = area.center().y - words_height / 2.0;
        let title_text = self.title.clone();
        let elided = title.elided;
        for (galley, color) in [
            (Some(eyebrow), muted),
            (Some(title), ink),
            (subtitle, muted),
            (counting, muted),
        ] {
            if let Some(galley) = galley {
                let h = galley.size().y;
                painter.galley(pos2(left, y), galley, color);
                y += h;
            }
        }
        if elided {
            let hit = ui.interact(
                Rect::from_min_size(pos2(left, area.top()), vec2(words_width, y - area.top())),
                ui.scope_id().with("up-next-title"),
                Sense::hover(),
            );
            let _ = hit.on_hover_text(title_text);
        }
        y += 12.0;
        let row = Rect::from_min_size(pos2(left, y), vec2(words_width, button));
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(Layout::left_to_right(Align::Center)),
        );
        child.spacing_mut().item_spacing.x = 8.0;
        out.play = child
            .add(Button::new(&self.words.play_now).icon(Icon::Play).filled_icon(true))
            .clicked();
        if self.cancel {
            out.cancel = child
                .add(
                    Button::new(&self.words.cancel)
                        .variant(ButtonVariant::Ghost)
                        .on_picture(true),
                )
                .clicked();
        }
        ui.allocate_rect(area, Sense::hover());
        out
    }
}
