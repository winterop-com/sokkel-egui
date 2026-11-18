//! The export, as the web's `export.ts` and `editor/ExportDialog.tsx` make it: every frame of the
//! edit painted as the preview paints it at the chosen size, counted and reported as it goes, and
//! what the dialog says before, during and after.
//!
//! The web encodes the frames as VP8 and the mixed sound (`sound::mixdown`) as Opus through
//! WebCodecs and writes them into a WebM file. That encoding is not carried here: an `ExportJob`
//! hands out the frames to paint, their times and key frames, and the progress the web reports, so
//! whatever encodes them reports the same; without an encoder every frame is still painted and
//! counted, and no file is made, as the web does in a browser without one.

use crate::js::{number_to_string, round, to_fixed, trim};
use crate::model::{Edit, ExportSize, ExportSpec, FPS, length_of, timecode};

/// A frame of the export to paint and hand to the encoder.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExportFrame {
    pub index: usize,
    /// The second of the edit it shows.
    pub time: f64,
    /// Its timestamp in microseconds, and how long it stands.
    pub timestamp: f64,
    pub duration: f64,
    /// A key frame every two seconds, where a seek lands.
    pub key: bool,
    /// The share done (0 to 1) and the frames painted, reported before this frame every tenth one.
    pub progress: Option<(f64, usize)>,
}

/// An export under way: the edit's frames one by one.
#[derive(Clone, Debug)]
pub struct ExportJob {
    pub size: ExportSpec,
    /// How long the edit runs, in seconds.
    pub length: f64,
    /// How many frames it paints: at least one.
    pub total: usize,
    /// Whether the frames go to an encoder, which leaves the last tenth of the progress for the
    /// sound and the file.
    pub encoding: bool,
    next: usize,
}

impl ExportJob {
    pub fn new(edit: &Edit, size: ExportSize, encoding: bool) -> Self {
        let length = length_of(edit);
        Self {
            size: size.spec(),
            length,
            total: round(length * FPS).max(1.0) as usize,
            encoding,
            next: 0,
        }
    }

    /// The next frame to paint, or none once every frame is painted.
    pub fn next_frame(&mut self) -> Option<ExportFrame> {
        if self.next >= self.total {
            return None;
        }
        let index = self.next;
        self.next += 1;
        let step = 1_000_000.0 / FPS;
        let share = if self.encoding { 0.9 } else { 1.0 };
        Some(ExportFrame {
            index,
            time: index as f64 / FPS,
            timestamp: round(index as f64 * step),
            duration: round(step),
            key: index.is_multiple_of(FPS as usize * 2),
            progress: index
                .is_multiple_of(10)
                .then(|| ((index as f64 / self.total as f64) * share, index)),
        })
    }

    /// The progress reported once every frame is painted: done without an encoder; with one, the
    /// share before the sound is mixed and the file written (`1` once it is).
    pub fn painted(&self) -> (f64, usize) {
        (if self.encoding { 0.93 } else { 1.0 }, self.total)
    }

    /// How long the export runs in the file, in microseconds.
    pub fn file_duration(&self) -> f64 {
        self.length * 1_000_000.0
    }
}

/// A file name from a title, as sokkel-ui's `fileName` makes one: its letters and digits in
/// lowercase, the rest one dash, with `extension` after it; `fallback` when the title has none.
pub fn file_name(title: &str, extension: &str, fallback: &str) -> String {
    let mut slug = String::new();
    let mut dash = false;
    for c in title.to_lowercase().chars() {
        if c.is_alphabetic() || c.is_numeric() {
            slug.push(c);
            dash = false;
        } else if !dash {
            slug.push('-');
            dash = true;
        }
    }
    let slug = slug.strip_prefix('-').unwrap_or(&slug);
    let slug = slug.strip_suffix('-').unwrap_or(slug);
    format!(
        "{}.{}",
        if slug.is_empty() { fallback } else { slug },
        extension.strip_prefix('.').unwrap_or(extension)
    )
}

/// The name the export dialog starts with for a project.
pub fn default_file_name(project: &str) -> String {
    file_name(project, "webm", "edit")
}

/// The file the export writes, `.webm` after the name typed unless it ends so.
pub fn named(file: &str) -> String {
    let trimmed = trim(file);
    if trimmed.ends_with(".webm") {
        trimmed.to_owned()
    } else {
        format!("{trimmed}.webm")
    }
}

/// Why Export cannot start yet, if it cannot.
pub fn why_not(edit: &Edit, file: &str) -> Option<&'static str> {
    if length_of(edit) == 0.0 {
        Some("The edit has nothing in it to export.")
    } else if trim(file).is_empty() {
        Some("The file needs a name.")
    } else {
        None
    }
}

/// The line under the dialog's title.
pub fn description(can_encode: bool) -> &'static str {
    if can_encode {
        "Drawn frame by frame and encoded as WebM in this browser."
    } else {
        "This browser has no video encoder: the frames are drawn and counted, and no file is made."
    }
}

/// What the dialog lists about the export: its length, its frames and its size.
pub fn summary(edit: &Edit, size: ExportSize) -> Vec<(&'static str, String)> {
    let length = length_of(edit);
    let spec = size.spec();
    vec![
        ("Length", timecode(length)),
        (
            "Frames",
            format!(
                "{} at {} a second",
                number_to_string(round(length * FPS)),
                number_to_string(FPS)
            ),
        ),
        ("Size", format!("{} by {}", spec.width, spec.height)),
    ]
}

/// The line under the progress bar.
pub fn progress_text(share: f64, frames: usize, edit: &Edit) -> String {
    let total = round(length_of(edit) * FPS) as usize;
    if share >= 0.9 && frames >= total {
        "Mixing the sound and writing the file".to_owned()
    } else {
        format!("Frame {frames} of {total}")
    }
}

/// A size in bytes as the dialog writes it: `1.2 MB`.
pub fn megabytes(bytes: u64) -> String {
    format!("{} MB", to_fixed(bytes as f64 / 1_000_000.0, 1))
}

/// What the export says when it is done: the title and the line under it. `bytes` is the file's
/// size, none where no file was made; `took` is how long it took, in milliseconds.
pub fn done_notice(file: &str, frames: usize, size: ExportSize, bytes: Option<u64>, took: f64) -> (String, String) {
    let seconds = to_fixed(took / 1000.0, 1);
    match bytes {
        None => (
            format!("Drew {frames} frames in {seconds} s"),
            "This browser has no video encoder, so no file was made.".to_owned(),
        ),
        Some(bytes) => (
            format!("{} exported", named(file)),
            format!(
                "{frames} frames at {}, {}, in {seconds} s.",
                size.spec().label,
                megabytes(bytes)
            ),
        ),
    }
}

/// The title of the note an export stopped by the person leaves.
pub const STOPPED: &str = "Export stopped";

/// The title of the note a failed export leaves, its reason under it.
pub const FAILED: &str = "The export failed";
