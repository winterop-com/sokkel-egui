//! One frame of an edit, painted straight from the scenes as the web's `render.ts` draws it: the
//! picture of the clip under second `t` on the topmost shown picture track, at its own second
//! (`in_point + t - start`), and the titles under `t` written over it. Nothing is decoded, so the
//! preview and the export paint the same frame.

use eframe::egui::{Color32, Painter, Rect};
use tidevann_shared::clips::scenes::{Canvas, font, rgba};

use crate::js::{is_space, round, trim};
use crate::media::draw_source;
use crate::model::{Clip, Edit, TrackKind, plays_at};

/// The clip whose picture shows at second `t`: on the topmost shown picture track that has one there.
pub fn picture_at(edit: &Edit, t: f64) -> Option<&Clip> {
    edit.tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Video && !track.is_hidden())
        .find_map(|track| {
            edit.clips
                .iter()
                .find(|clip| clip.track == track.id && plays_at(clip, t))
        })
}

/// The titles written over the picture at second `t`, on every shown title track.
pub fn titles_at(edit: &Edit, t: f64) -> Vec<&Clip> {
    let shown: Vec<&str> = edit
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Title && !track.is_hidden())
        .map(|track| track.id.as_str())
        .collect();
    edit.clips
        .iter()
        .filter(|clip| shown.contains(&clip.track.as_str()) && plays_at(clip, t))
        .collect()
}

/// A title's words over the lower part of the picture, on a dark band, three lines at most.
fn draw_title(canvas: &Canvas, clip: &Clip) {
    let Some(text) = clip.text.as_deref().map(trim).filter(|text| !text.is_empty()) else {
        return;
    };
    let (w, h) = (canvas.w, canvas.h);
    let face = font(round(h * 0.07), false);
    let lines: Vec<String> = canvas
        .wrap(text.split(is_space).filter(|word| !word.is_empty()), &face, w * 0.8)
        .into_iter()
        .take(3)
        .collect();
    let line_height = h * 0.09;
    let block = lines.len() as f64 * line_height;
    let top = h * 0.78 - block;
    canvas.rect(0.0, top - h * 0.03, w, block + h * 0.06, rgba(0.0, 0.0, 0.0, 0.45));
    for (index, line) in lines.iter().enumerate() {
        canvas.text(
            w * 0.08,
            top + (index as f64 + 1.0) * line_height - h * 0.02,
            line,
            &face,
            rgba(255.0, 255.0, 255.0, 1.0),
        );
    }
}

/// Paints the edit's frame at second `t` on `rect`: black where no picture plays.
pub fn draw_edit_frame(painter: &Painter, rect: Rect, edit: &Edit, t: f64) {
    let painter = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
    painter.rect_filled(rect, 0.0, Color32::BLACK);
    if let Some(picture) = picture_at(edit, t)
        && let Some(source) = picture.source()
    {
        draw_source(&painter, rect, source, picture.in_point + t - picture.start);
    }
    let canvas = Canvas::new(&painter, rect);
    for title in titles_at(edit, t) {
        draw_title(&canvas, title);
    }
}
