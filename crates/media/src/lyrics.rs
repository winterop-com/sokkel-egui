//! The lyrics of what plays, after sokkel-ui-media's `Lyrics`.

use egui::{Align, Color32, Id, Mesh, Rect, RichText, ScrollArea, Sense, Shape, Ui, UiBuilder, pos2, vec2};

use sokkel_egui::{TextRole, Tokens, tokens};

use crate::lrc::{LyricLine, ParsedLyrics, lyric_line_at};

/// The words the lyrics say.
#[derive(Clone, Debug)]
pub struct LyricsWords {
    /// The name of the region the lyrics fill.
    pub label: String,
    /// What stands in for lyrics with no lines.
    pub none: String,
}

impl Default for LyricsWords {
    fn default() -> Self {
        Self {
            label: "Lyrics".into(),
            none: "No lyrics".into(),
        }
    }
}

/// What the lyrics were asked this frame.
#[derive(Clone, Debug, Default)]
pub struct LyricsResponse {
    /// A line was pressed: seek to its second.
    pub seek: Option<f64>,
}

/// The width from which the lines are drawn larger, `@3xl`.
const LARGE_FROM: f32 = 768.0;

/// The lyrics of what plays, after sokkel-ui-media's `Lyrics`, filling the room they are given (at
/// least 192 tall) and scrolling within it, as a panel beside the player or the fullscreen view in
/// the visualizer's place.
///
/// Timed lyrics follow `position`: the line being sung is in the foreground and kept in the middle
/// of the box, the others `faint`, and before the first line the first waits there. Each line is in
/// the page size, semibold and tight (the stat size once the box is 768 wide). Pressing a line asks
/// to seek to it (`seekable`); a line under the pointer turns `muted-foreground`. While the reader
/// scrolls the lyrics stay where they put them until the line sung changes. Untimed lyrics read as
/// a page from the top, verse by verse, in `text.base` relaxed (the page size once wide), and follow
/// nothing. Timed lines fade out at the box's top and foot into `background`.
#[must_use = "show it with `.show(ui)`"]
pub struct Lyrics<'a> {
    id: Id,
    lyrics: &'a ParsedLyrics,
    position: f64,
    seekable: bool,
    follow: bool,
    words: LyricsWords,
}

impl<'a> Lyrics<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, lyrics: &'a ParsedLyrics) -> Self {
        Self {
            id: Id::unique(id_salt),
            lyrics,
            position: 0.0,
            seekable: true,
            follow: true,
            words: LyricsWords::default(),
        }
    }

    /// Seconds from the start of what plays.
    pub fn position(mut self, seconds: f64) -> Self {
        self.position = seconds;
        self
    }

    /// Whether a press on a line asks to seek to it (the web's `onSeek`); on by default.
    pub fn seekable(mut self, seekable: bool) -> Self {
        self.seekable = seekable;
        self
    }

    /// Whether timed lyrics keep the line being sung in the middle; on by default.
    pub fn follow(mut self, follow: bool) -> Self {
        self.follow = follow;
        self
    }

    pub fn words(mut self, words: LyricsWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> LyricsResponse {
        let t = tokens(ui.ctx());
        let mut out = LyricsResponse::default();
        let size = vec2(ui.available_width(), ui.available_height().max(192.0));
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Group, true, &self.words.label));
        let mut child = ui.new_child(UiBuilder::new().id_salt(self.id).max_rect(rect));
        let ui = &mut child;
        let large = rect.width() >= LARGE_FROM;
        let lines = &self.lyrics.lines;
        if lines.is_empty() {
            let galley = ui.painter().layout_no_wrap(
                self.words.none.clone(),
                TextRole::Body.font(&t),
                t.color("muted-foreground"),
            );
            ui.painter()
                .galley(rect.center() - galley.size() / 2.0, galley, t.color("muted-foreground"));
            return out;
        }
        if !self.lyrics.timed {
            ScrollArea::vertical()
                .id_salt(self.id.with("page"))
                .auto_shrink([false, false])
                .show(ui, |ui| page(ui, &t, lines, large));
            return out;
        }
        let at = lyric_line_at(lines, self.position);
        let held = at.unwrap_or(0);
        let followed_id = self.id.with("followed");
        let followed: Option<usize> = ui.data(|d| d.get_temp(followed_id));
        let (role, size) = if large {
            (TextRole::Statistic, t.text("stat"))
        } else {
            (TextRole::PageTitle, t.text("page"))
        };
        let width = (rect.width() - 48.0).clamp(0.0, 768.0 - 48.0);
        ScrollArea::vertical()
            .id_salt(self.id.with("timed"))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let left = ui.max_rect().center().x - width / 2.0;
                ui.add_space(rect.height() / 2.0);
                for (index, line) in lines.iter().enumerate() {
                    if line.text.is_empty() {
                        ui.add_space(24.0);
                        continue;
                    }
                    let current = Some(index) == at;
                    let rich = |ink| {
                        role.rich(&t, &line.text)
                            .variation("wght", 600.0)
                            .extra_letter_spacing(-0.025 * size.size)
                            .color(ink)
                    };
                    let galley = egui::WidgetText::from(rich(Color32::PLACEHOLDER)).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Wrap),
                        width,
                        role.font(&t),
                    );
                    let height = galley.size().y + 12.0;
                    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
                    let hit = Rect::from_min_size(pos2(left, row.top()), vec2(width, height));
                    let seeks = self.seekable && line.time.is_some();
                    let sense = if seeks { Sense::click() } else { Sense::hover() };
                    let r = ui.interact(hit, self.id.with(("line", index)), sense);
                    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, seeks, &line.text));
                    if seeks && r.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if r.has_focus() {
                        ui.painter().rect_filled(hit, t.radius("control"), t.color("hover"));
                    }
                    if r.clicked() {
                        out.seek = line.time;
                    }
                    let ink = if current {
                        t.color("foreground")
                    } else if seeks && r.hovered() {
                        t.color("muted-foreground")
                    } else {
                        t.color("faint")
                    };
                    ui.painter().galley(pos2(left, row.top() + 6.0), galley, ink);
                    if index == held && self.follow && (followed != Some(held) || out.seek.is_some()) {
                        ui.scroll_to_rect(row, Some(Align::Center));
                        ui.data_mut(|d| d.insert_temp(followed_id, held));
                    }
                }
                ui.add_space(rect.height() / 2.0);
            });
        fade(ui, &t, rect);
        out
    }
}

/// The timed lines fade out over the box's top and foot 15%, as `mask-y-from-85%` does, into the
/// page's `background`.
fn fade(ui: &Ui, t: &Tokens, rect: Rect) {
    let ground = t.color("background");
    let band = rect.height() * 0.15;
    for (edge, inward) in [(rect.top(), 1.0), (rect.bottom(), -1.0)] {
        let inner = edge + inward * band;
        let mut mesh = Mesh::default();
        mesh.colored_vertex(pos2(rect.left(), edge), ground);
        mesh.colored_vertex(pos2(rect.right(), edge), ground);
        mesh.colored_vertex(pos2(rect.right(), inner), Color32::TRANSPARENT);
        mesh.colored_vertex(pos2(rect.left(), inner), Color32::TRANSPARENT);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        ui.painter().add(Shape::mesh(mesh));
    }
}

/// Untimed lyrics as a page, verse by verse.
fn page(ui: &mut Ui, t: &Tokens, lines: &[LyricLine], large: bool) {
    let width = (ui.available_width() - 48.0).clamp(0.0, 640.0);
    let left = ui.max_rect().center().x - width / 2.0;
    ui.add_space(32.0);
    for (n, verse) in verses(lines).iter().enumerate() {
        if n > 0 {
            ui.add_space(24.0);
        }
        let text = verse.join("\n");
        let rich = if large {
            TextRole::PageTitle
                .rich(t, text)
                .variation("wght", 400.0)
                .extra_letter_spacing(0.0)
        } else {
            let base = t.text("base");
            RichText::new(text)
                .font(egui::FontId::proportional(base.size))
                .line_height(Some(base.size * 1.625))
        };
        let galley = egui::WidgetText::from(rich.color(t.color("foreground"))).into_galley(
            ui,
            Some(egui::TextWrapMode::Wrap),
            width,
            TextRole::Body.font(t),
        );
        let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), galley.size().y), Sense::hover());
        ui.painter()
            .galley(pos2(left, row.top()), galley, t.color("foreground"));
    }
    ui.add_space(32.0);
}

/// Untimed lines in their verses, split at the blank lines between them.
fn verses(lines: &[LyricLine]) -> Vec<Vec<&str>> {
    let mut found: Vec<Vec<&str>> = vec![Vec::new()];
    for line in lines {
        if line.text.is_empty() {
            found.push(Vec::new());
        } else if let Some(last) = found.last_mut() {
            last.push(&line.text);
        }
    }
    found.retain(|v| !v.is_empty());
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lrc::parse_lyrics;

    #[test]
    fn splits_untimed_lines_into_verses_at_the_blank_lines() {
        let lyrics = parse_lyrics("One\nTwo\n\nThree");
        assert_eq!(verses(&lyrics.lines), [vec!["One", "Two"], vec!["Three"]]);
    }
}
