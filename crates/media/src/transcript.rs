//! A transcript's lines and the one being said, as sokkel-ui-media's `Transcript.tsx` keeps them.
//! Carried here until the media add-on has them.

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
