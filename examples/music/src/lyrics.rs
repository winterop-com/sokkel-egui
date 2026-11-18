//! The words of a song, written from its seed as the web's `lyrics.ts` writes them: verses and a
//! chorus from a few banks of lines about the coast. Timed songs come as LRC, a line every two bars
//! from the end of the intro, so the fullscreen view follows them as the song plays (`lrc`); the
//! rest as plain verses to read.

use crate::catalog::{Song, Words, album_of, artist_name};
use crate::data::MadeStation;
use crate::js;
use crate::synth::{Seeded, bars_of};

const VERSES: [&str; 20] = [
    "The boats came in before the light",
    "We left the harbor wall behind",
    "Salt on the window, rain on the quay",
    "You said the tide would wait for us",
    "A lantern swinging on the pier",
    "The ferry calls across the sound",
    "We counted gulls along the shore",
    "The wind came round from the west again",
    "I kept your letters in a tin",
    "The water stands so still tonight",
    "We walked the causeway, shoes in hand",
    "The keeper climbs the winding stair",
    "Your coat still smells of woodsmoke",
    "The nets are drying on the rail",
    "A single light out on the reef",
    "We named the islands one by one",
    "The road runs out where the dunes begin",
    "The radio plays the shipping news",
    "I heard the bell from the far side",
    "The moon is pulling at the bay",
];

const CHORUSES: [[&str; 4]; 5] = [
    [
        "And the tide comes in",
        "And the tide goes out",
        "And I wait for you",
        "By the water",
    ],
    [
        "Hold on, hold on",
        "Till the morning breaks",
        "Hold on, hold on",
        "For the long way home",
    ],
    [
        "Low water, high water",
        "Every turn of the sea",
        "Low water, high water",
        "Brings you back to me",
    ],
    [
        "Over the breakwater",
        "Under the open sky",
        "We were never lost",
        "Only drifting by",
    ],
    [
        "Carry me home",
        "On the evening swell",
        "Carry me home",
        "Where the lanterns dwell",
    ],
];

/// Seconds written as LRC's stamp: `[01:02.50]`.
fn stamp(seconds: f64) -> String {
    let minutes = (seconds / 60.0).floor();
    let rest = seconds - minutes * 60.0;
    format!("[{:0>2}:{:0>5}]", js::number_to_string(minutes), js::to_fixed(rest, 2))
}

/// The verses and choruses of a song, as lines.
fn lines_of(song: &Song, count: usize) -> Vec<&'static str> {
    let mut random = Seeded::new(song.seed ^ 0x2545_f491);
    let chorus = CHORUSES[(random.next_f64() * CHORUSES.len() as f64).floor() as usize];
    let mut lines = Vec::new();
    let mut verse = 0;
    while lines.len() < count {
        if verse % 2 == 0 {
            for _ in 0..4 {
                lines.push(VERSES[(random.next_f64() * VERSES.len() as f64).floor() as usize]);
            }
        } else {
            lines.extend(chorus);
        }
        verse += 1;
    }
    lines.truncate(count);
    lines
}

/// A song's words as LRC or plain text, or none when it has none. `made` are the listener's
/// stations, for a radio song's album.
pub fn lyrics_of(song: &Song, made: &[MadeStation]) -> Option<String> {
    if song.words == Words::None {
        return None;
    }
    let (bar, bars) = bars_of(song.seed, song.style);
    let start = if bars > 8 { 4 } else { 2 };
    let slots = ((bars as i64 - start as i64 - 2) as f64 / 2.0).floor().max(4.0) as usize;
    let lines = lines_of(song, slots.min(16));
    if song.words == Words::Plain {
        let written: Vec<String> = lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                if index > 0 && index % 4 == 0 {
                    format!("\n{line}")
                } else {
                    (*line).to_owned()
                }
            })
            .collect();
        return Some(written.join("\n"));
    }
    let mut out = vec![
        format!("[ti:{}]", song.title),
        format!("[ar:{}]", artist_name(&song.artist)),
        format!("[al:{}]", album_of(song, made).title),
    ];
    for (index, line) in lines.iter().enumerate() {
        out.push(format!("{}{line}", stamp(((start + index * 2) as f64) * bar)));
    }
    out.push(stamp(((start + lines.len() * 2) as f64) * bar));
    Some(out.join("\n"))
}
