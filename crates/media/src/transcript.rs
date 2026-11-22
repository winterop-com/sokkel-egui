//! The words of what plays as they are said, after sokkel-ui-media's `Transcript`: its lines, the
//! one being said, and the box that reads them.

use serde::Serialize;

/// One line of a transcript: what was said, from the second it starts, and who said it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TranscriptLine {
    /// Seconds from the start of what plays.
    pub start: f64,
    pub text: String,
    /// Who speaks, written over the line where it changes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
}

/// The line being said at `position`: the last one started by then, or -1 before the first.
pub fn transcript_line_at(lines: &[TranscriptLine], position: f64) -> i64 {
    let mut found = -1;
    for (index, line) in lines.iter().enumerate() {
        if line.start > position {
            break;
        }
        found = index as i64;
    }
    found
}

/// The words the transcript says.
#[derive(Clone, Debug)]
pub struct TranscriptWords {
    /// The name of the region the transcript fills.
    pub label: String,
    /// What stands in for a transcript with no lines.
    pub none: String,
    /// `Play from {time}: {text}`: a line's name.
    pub seek: String,
}

impl Default for TranscriptWords {
    fn default() -> Self {
        Self {
            label: "Transcript".into(),
            none: "No transcript".into(),
            seek: "Play from {time}: {text}".into(),
        }
    }
}

/// What the transcript was asked this frame.
#[derive(Clone, Debug, Default)]
pub struct TranscriptResponse {
    /// A line was pressed: seek to its start.
    pub seek: Option<f64>,
}

/// The widest a transcript's column reads, as `max-w-prose` (65 of the face's zero).
const PROSE: f32 = 65.0;

/// The words of a podcast, an interview or a talk as they are said, after sokkel-ui-media's
/// `Transcript`, filling the room it is given (at least 192 tall) and scrolling within it: each line
/// under its time in the code face, the speaker written in `foreground` where the voice changes,
/// read in `text.base` relaxed in a column at most a prose measure wide, 24 in from its sides and
/// 32 from its top and foot. The line said at `position` is in `foreground` and the rest
/// `muted-foreground`; it is kept in the middle of the box as it changes (`follow`). Pressing a
/// line asks to seek to its start (`seekable`).
///
/// Where [`Lyrics`](crate::Lyrics) sets a song's lines large to sing along, a transcript is read:
/// beside the player, on an episode's page, or in the fullscreen view in the visualizer's place
/// ([`MediaBar`](crate::MediaBar)'s `lyrics`).
#[must_use = "show it with `.show(ui)`"]
pub struct Transcript<'a> {
    id: egui::Id,
    lines: &'a [TranscriptLine],
    position: f64,
    seekable: bool,
    follow: bool,
    words: TranscriptWords,
}

impl<'a> Transcript<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, lines: &'a [TranscriptLine]) -> Self {
        Self {
            id: egui::Id::unique(id_salt),
            lines,
            position: 0.0,
            seekable: true,
            follow: true,
            words: TranscriptWords::default(),
        }
    }

    /// Seconds from the start of what plays; the line said then is inked.
    pub fn position(mut self, seconds: f64) -> Self {
        self.position = seconds;
        self
    }

    /// Whether a press on a line asks to seek to it (the web's `onSeek`); on by default.
    pub fn seekable(mut self, seekable: bool) -> Self {
        self.seekable = seekable;
        self
    }

    /// Whether the line being said is kept in the middle of the box; on by default.
    pub fn follow(mut self, follow: bool) -> Self {
        self.follow = follow;
        self
    }

    pub fn words(mut self, words: TranscriptWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> TranscriptResponse {
        use egui::{Align, RichText, ScrollArea, Sense, TextWrapMode, WidgetText, pos2, vec2};
        use sokkel_egui::{TextRole, format_time, mono_font, mono_size, tokens};

        let t = tokens(ui.ctx());
        let mut out = TranscriptResponse::default();
        let size = vec2(ui.available_width(), ui.available_height().max(192.0));
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Group, true, &self.words.label));
        let mut child = ui.new_child(egui::UiBuilder::new().id_salt(self.id).max_rect(rect));
        let ui = &mut child;
        ui.set_clip_rect(rect.intersect(ui.clip_rect()));
        if self.lines.is_empty() {
            let galley = ui.painter().layout_no_wrap(
                self.words.none.clone(),
                TextRole::Body.font(&t),
                t.color("muted-foreground"),
            );
            ui.painter()
                .galley(rect.center() - galley.size() / 2.0, galley, t.color("muted-foreground"));
            return out;
        }
        let at = transcript_line_at(self.lines, self.position);
        let base = t.text("base");
        let mono = mono_font(ui.ctx(), mono_size(&t, t.text("xs").size), 400.0);
        let zero = ui
            .painter()
            .layout_no_wrap("0".into(), egui::FontId::proportional(base.size), egui::Color32::WHITE)
            .size()
            .x;
        let width = (rect.width() - 48.0).clamp(0.0, PROSE * zero);
        let followed_id = self.id.with("followed");
        let followed: Option<i64> = ui.data(|d| d.get_temp(followed_id));
        ScrollArea::vertical()
            .id_salt(self.id.with("scroll"))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let left = ui.max_rect().center().x - width / 2.0;
                ui.add_space(32.0);
                for (index, line) in self.lines.iter().enumerate() {
                    let current = index as i64 == at;
                    let speaker = line
                        .speaker
                        .as_ref()
                        .filter(|who| index == 0 || self.lines[index - 1].speaker.as_ref() != Some(*who));
                    let above = if speaker.is_some() && index > 0 { 16.0 } else { 0.0 };
                    let head = TextRole::Help.line_height(&t);
                    let body = WidgetText::from(
                        RichText::new(&line.text)
                            .font(egui::FontId::proportional(base.size))
                            .line_height(Some(base.size * 1.625))
                            .color(t.color(if current { "foreground" } else { "muted-foreground" })),
                    )
                    .into_galley(
                        ui,
                        Some(TextWrapMode::Wrap),
                        width,
                        egui::FontId::proportional(base.size),
                    );
                    let height = above + 6.0 + head + body.size().y + 6.0;
                    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
                    let hit = egui::Rect::from_min_max(pos2(left, row.top() + above), pos2(left + width, row.bottom()));
                    let time = format_time(line.start);
                    let sense = if self.seekable { Sense::click() } else { Sense::hover() };
                    let r = ui.interact(hit, self.id.with(("line", index)), sense);
                    let name = self.words.seek.replace("{time}", &time).replace("{text}", &line.text);
                    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, self.seekable, &name));
                    if self.seekable && r.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if r.has_focus() {
                        ui.painter().rect_filled(hit, t.radius("control"), t.color("hover"));
                    }
                    if r.clicked() {
                        out.seek = Some(line.start);
                    }
                    // the head: who speaks where it changes, and when, 8 apart
                    let mut x = left;
                    let head_top = hit.top() + 6.0;
                    if let Some(who) = speaker {
                        let galley = WidgetText::from(
                            TextRole::Help
                                .rich(&t, who)
                                .variation("wght", t.weight(sokkel_egui::Weight::Medium))
                                .color(t.color("foreground")),
                        )
                        .into_galley(
                            ui,
                            Some(TextWrapMode::Extend),
                            f32::INFINITY,
                            TextRole::Help.font(&t),
                        );
                        let w = galley.size().x;
                        ui.painter().galley(
                            pos2(x, head_top + (head - galley.size().y) / 2.0),
                            galley,
                            t.color("foreground"),
                        );
                        x += w + 8.0;
                    }
                    let stamp = ui
                        .painter()
                        .layout_no_wrap(time, mono.clone(), t.color("muted-foreground"));
                    ui.painter().galley(
                        pos2(x, head_top + (head - stamp.size().y) / 2.0),
                        stamp,
                        t.color("muted-foreground"),
                    );
                    ui.painter()
                        .galley(pos2(left, head_top + head), body, t.color("muted-foreground"));
                    let held = at.max(0) as usize;
                    if index == held && self.follow && at >= 0 && (followed != Some(at) || out.seek.is_some()) {
                        ui.scroll_to_rect(row, Some(Align::Center));
                        ui.data_mut(|d| d.insert_temp(followed_id, at));
                    }
                }
                ui.add_space(32.0);
            });
        out
    }
}
