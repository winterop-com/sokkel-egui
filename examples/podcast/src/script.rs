//! What is said in an episode, and when, as the web's `script.ts` makes it: its transcript, its
//! chapters and every syllable of every line, timed as the voices in `voice` speak them, so the
//! transcript follows the sound. Made from the episode's chapters and a few banks of sentences, the
//! same every time for an episode.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;

use crate::catalog::{Episode, Host, Show, Talk, episode_by_id, show_by_id};
use crate::chapters::MediaChapter;
use crate::js;
use crate::transcript::TranscriptLine;

/// The mouth's shape for a vowel, read from its letter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Vowel {
    A,
    E,
    I,
    O,
    U,
}

/// How a syllable begins: a hiss, a breath, a stop, a hum, a glide, or straight into the vowel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Onset {
    Hiss,
    Breath,
    Stop,
    Hum,
    Glide,
    None,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Syllable {
    pub at: f64,
    pub length: f64,
    pub vowel: Vowel,
    pub onset: Onset,
    /// The pitch at its start and end, in hertz.
    pub pitch: f64,
    pub pitch_end: f64,
    /// How loud, around 1.
    pub gain: f64,
    /// Whether the voice stops after it, at the end of a word.
    pub last: bool,
    pub voice: Host,
}

/// A tune played at a moment: the jingle at the start and the end, a chime between chapters.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Cue {
    pub at: f64,
    /// Semitones above the show's key.
    pub notes: Vec<i32>,
    /// Seconds between notes.
    pub step: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Script {
    pub duration: f64,
    pub chapters: Vec<MediaChapter>,
    pub lines: Vec<TranscriptLine>,
    pub syllables: Vec<Syllable>,
    pub cues: Vec<Cue>,
}

/// How a host sounds: the pitch they speak at, how long a syllable takes them, and how far their
/// resonances sit above a low voice's.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct VoiceOf {
    pub pitch: f64,
    pub syllable: f64,
    pub formant: f64,
}

/// How each host sounds.
pub fn voice_of(host: Host) -> VoiceOf {
    match host {
        Host::Ada => VoiceOf {
            pitch: 205.0,
            syllable: 0.168,
            formant: 1.16,
        },
        Host::Grace => VoiceOf {
            pitch: 186.0,
            syllable: 0.176,
            formant: 1.12,
        },
        Host::Margaret => VoiceOf {
            pitch: 174.0,
            syllable: 0.186,
            formant: 1.1,
        },
        Host::Linus => VoiceOf {
            pitch: 112.0,
            syllable: 0.18,
            formant: 1.0,
        },
    }
}

/// A number from a text, the same every time: FNV-1a over its UTF-16 units.
pub fn hash(text: &str) -> u32 {
    let mut value: u32 = 2_166_136_261;
    for unit in text.encode_utf16() {
        value ^= u32::from(unit);
        value = value.wrapping_mul(16_777_619);
    }
    value
}

/// Numbers from 0 to 1 drawn from a seed, the same run every time (the web's `seeded`).
#[derive(Clone, Debug)]
pub struct Seeded {
    state: u32,
}

impl Seeded {
    pub fn new(seed: &str) -> Self {
        let hashed = hash(seed);
        Self {
            state: if hashed == 0 { 1 } else { hashed },
        }
    }

    pub fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let state = self.state;
        let mut mixed = (state ^ (state >> 15)).wrapping_mul(1 | state);
        mixed = mixed.wrapping_add((mixed ^ (mixed >> 7)).wrapping_mul(61 | mixed)) ^ mixed;
        f64::from(mixed ^ (mixed >> 14)) / 4_294_967_296.0
    }

    /// One of `list`, as the web's `pick` takes it.
    fn pick<'a, T>(&mut self, list: &'a [T]) -> &'a T {
        let at = (self.next_f64() * list.len() as f64).floor() as usize;
        &list[at.min(list.len() - 1)]
    }
}

const COAST: [&str; 22] = [
    "We got to {place} early, before anyone else was up.",
    "The first thing you notice at {place} is how quiet it is.",
    "Most people walk straight past it without looking.",
    "It changes with every tide, and with every season too.",
    "The old maps show {place} a little further north than it is today.",
    "There is a bench there, and on a clear day you can see the whole coast.",
    "Nobody quite agrees on how long it has been like this.",
    "I asked around, and everyone had a different answer.",
    "It takes about an hour on foot, less if the path is dry.",
    "The wind came round from the west while we stood there.",
    "You can hear the water long before you see it.",
    "That is the part I keep coming back to.",
    "It is easy to miss if you are in a hurry.",
    "In winter the light is gone by four, so you have to be quick.",
    "The people who live here have a name for it that is on no map.",
    "It is smaller than it looks in the pictures, and somehow better.",
    "We brought a flask of coffee and stayed far longer than we meant to.",
    "That is what makes {place} worth the trip.",
    "The gulls were there first, as always, and they let us know it.",
    "If you stand still long enough, everything starts to move again.",
    "Bring good boots. Whatever the forecast says, bring good boots.",
    "There was a fisherman there who told us the whole story in five minutes.",
];

const COAST_ASKED: [(&str, &str); 6] = [
    (
        "How far out does it go at low water?",
        "About as far as you can throw a stone, and then a good bit more.",
    ),
    (
        "And who looks after it now?",
        "A few volunteers, mostly, and the weather does the rest.",
    ),
    (
        "Is it always that windy up there?",
        "Most days. The calm ones are worth the wait.",
    ),
    (
        "Did anyone keep a record of it?",
        "There is a notebook in the harbor office that goes back years.",
    ),
    ("Would you go back?", "Tomorrow, if the tide is right."),
    (
        "What surprised you the most?",
        "How quickly it all changed once the water came back.",
    ),
];

const WEATHER: [&str; 12] = [
    "The pressure has been falling slowly since Monday.",
    "Cold air is heavier than warm air, so on a still night it runs downhill and fills the valley.",
    "Up on the ridge it was clear, and down in the town you could not see the end of the street.",
    "The wind has a long way to come across the sea, and it picks up the damp on the way.",
    "The station on {place} has read the same instruments every morning for sixty years.",
    "When the wind turns to the north, it gets colder and the sky clears.",
    "Rain on the coast, snow on the hills, and sunshine somewhere in between.",
    "Watch the tops of the trees; they turn before the weather does.",
    "The thermometer said two degrees, but the wind made it feel much colder.",
    "By the weekend it should settle, though I would not bet the picnic on it.",
    "A ring around the moon usually means rain within a day or two.",
    "The fog lifts once the sun has warmed the ground, usually by ten.",
];

const WEATHER_ASKED: [(&str, &str); 2] = [
    (
        "You might ask why it always seems worse on a Monday.",
        "It is not, but it does feel that way.",
    ),
    ("Will it last?", "Two more days, and then the wind turns."),
];

const OPS: [&str; 14] = [
    "The {job} job ran every night this week, and failed only once.",
    "It failed at the {step} step, the one we changed on Monday.",
    "Grace moved the {step} step onto the worker, and the runs got faster.",
    "The scheduler started it on time; the trouble came later.",
    "We looked at the logs from the {project} project first.",
    "A retry fixed it, but a retry is not a fix.",
    "The api was slow for about ten minutes, and everything behind it waited.",
    "Margaret wrote a runbook for it, so the next person has an easier night.",
    "Nothing was lost; the run picked up where it stopped.",
    "We added a validate step, so a bad file stops early instead of late.",
    "Most runs finish in under four minutes now.",
    "The queue was empty again by six, before anyone logged on.",
    "It is the kind of failure you see once a month, which makes it hard to catch.",
    "We cancelled the stuck run, and the next one succeeded.",
];

const OPS_ASKED: [(&str, &str); 3] = [
    (
        "How did you find it?",
        "The logs from the {step} step said exactly what was wrong, for once.",
    ),
    ("Could it happen again?", "Not this way. There is a check for it now."),
    (
        "Who was on call?",
        "Grace, and she had it fixed before her coffee was cold.",
    ),
];

/// The sentences a show's talk draws from, and the questions with their answers.
fn bank(talk: Talk) -> (&'static [&'static str], &'static [(&'static str, &'static str)]) {
    match talk {
        Talk::Coast => (&COAST, &COAST_ASKED),
        Talk::Weather => (&WEATHER, &WEATHER_ASKED),
        Talk::Ops => (&OPS, &OPS_ASKED),
    }
}

const INTROS: [&str; 5] = [
    "So: {chapter}.",
    "Let us start with {chapter}.",
    "Now, {chapter}.",
    "Which brings us to {chapter}.",
    "Next, {chapter}.",
];

const JOBS: [&str; 5] = [
    "nightly-export",
    "sync-inventory",
    "rebuild-index",
    "send-digest",
    "import-orders",
];
const STEPS: [&str; 5] = ["fetch", "transform", "validate", "upload", "notify"];
const PROJECTS: [&str; 3] = ["harbor", "juniper", "larkspur"];

fn lower(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// A line before it is timed: who says what.
struct Said {
    speaker: Host,
    text: String,
}

/// Who says what in an episode, chapter by chapter, before any of it is timed.
fn written(episode: &Episode, show: &Show) -> Vec<(String, Vec<Said>)> {
    let mut random = Seeded::new(&episode.id);
    // Every placeholder's word is drawn, used or not, as the web's `replaceAll` arguments are.
    let fill = |random: &mut Seeded, text: &str| {
        let text = text.replace("{place}", &episode.place);
        let text = text.replace("{job}", random.pick(&JOBS));
        let text = text.replace("{step}", random.pick(&STEPS));
        text.replace("{project}", random.pick(&PROJECTS))
    };
    let (said_bank, asked) = bank(show.talk);
    let mut said: Vec<&str> = said_bank.to_vec();
    // Each sentence once per episode, in an order of its own.
    let next = |random: &mut Seeded, said: &mut Vec<&str>| {
        let at = (random.next_f64() * said.len() as f64).floor() as usize;
        let text = if at < said.len() {
            said.remove(at)
        } else {
            *random.pick(said_bank)
        };
        fill(random, text)
    };
    let host = show.hosts.first().copied().unwrap_or(Host::Ada);
    let other = show.hosts.get(1).copied();
    let mut turn: u32 = 0;
    let speaker = |turn: &mut u32| match other {
        None => host,
        Some(other) => {
            let now = *turn;
            *turn += 1;
            if now.is_multiple_of(2) { host } else { other }
        }
    };
    let count = episode.chapters.len();
    episode
        .chapters
        .iter()
        .enumerate()
        .map(|(index, title)| {
            let mut lines = Vec::new();
            let last = index == count - 1;
            if index == 0 {
                let with = other.map_or(String::new(), |other| format!(", here with {}", other.name()));
                lines.push(Said {
                    speaker: host,
                    text: format!(
                        "Welcome to {}. I am {}{with}, and this time: {}.",
                        show.title,
                        host.name(),
                        lower(&episode.title)
                    ),
                });
                lines.push(Said {
                    speaker: host,
                    text: episode.summary.clone(),
                });
                turn = 1;
            } else {
                let who = speaker(&mut turn);
                lines.push(Said {
                    speaker: who,
                    text: random.pick(&INTROS).replacen("{chapter}", &lower(title), 1),
                });
            }
            let body = if last {
                2
            } else {
                3 + (random.next_f64() * 2.0).floor() as usize
            };
            for _ in 0..body {
                if random.next_f64() < 0.3 {
                    let (question, answer) = *random.pick(asked);
                    let asks = other.unwrap_or(host);
                    let answers = match other {
                        None => host,
                        Some(other) => {
                            if asks == host {
                                other
                            } else {
                                host
                            }
                        }
                    };
                    let question = fill(&mut random, question);
                    lines.push(Said {
                        speaker: asks,
                        text: question,
                    });
                    let answer = fill(&mut random, answer);
                    lines.push(Said {
                        speaker: answers,
                        text: answer,
                    });
                } else {
                    let who = speaker(&mut turn);
                    let text = if random.next_f64() < 0.35 {
                        let first = next(&mut random, &mut said);
                        let second = next(&mut random, &mut said);
                        format!("{first} {second}")
                    } else {
                        next(&mut random, &mut said)
                    };
                    lines.push(Said { speaker: who, text });
                }
            }
            if last {
                lines.push(Said {
                    speaker: host,
                    text: format!("That is all for this episode of {}. Thanks for listening.", show.title),
                });
                if let Some(other) = other {
                    lines.push(Said {
                        speaker: other,
                        text: "See you next time.".to_owned(),
                    });
                }
            }
            (title.clone(), lines)
        })
        .collect()
}

const VOWELS: &[u8] = b"aeiouy";

/// A word's syllables: each vowel group with the consonants before it.
fn syllables_of(word: &str) -> Vec<(Onset, Vowel)> {
    let letters: Vec<u8> = word
        .to_lowercase()
        .bytes()
        .filter(|byte| byte.is_ascii_lowercase())
        .collect();
    let mut found = Vec::new();
    let mut from = 0;
    let mut at = 0;
    while at < letters.len() {
        if VOWELS.contains(&letters[at]) {
            let start = at;
            while at < letters.len() && VOWELS.contains(&letters[at]) {
                at += 1;
            }
            found.push((onset_of(&letters[from..start]), vowel_of(letters[start])));
            from = at;
        } else {
            at += 1;
        }
    }
    // A silent e at the end of a longer word.
    let silent =
        letters.len() >= 2 && letters[letters.len() - 1] == b'e' && !VOWELS.contains(&letters[letters.len() - 2]);
    if found.len() > 1 && silent {
        found.pop();
    }
    if found.is_empty() {
        vec![(Onset::None, Vowel::A)]
    } else {
        found
    }
}

fn onset_of(consonants: &[u8]) -> Onset {
    let first = match consonants.len() {
        0 => return Onset::None,
        1 => consonants[0],
        n => consonants[n - 2],
    };
    if b"szcx".contains(&first) {
        Onset::Hiss
    } else if b"fvh".contains(&first) || consonants.ends_with(b"th") {
        Onset::Breath
    } else if b"pbtdkgq".contains(&first) {
        Onset::Stop
    } else if b"mn".contains(&first) {
        Onset::Hum
    } else {
        Onset::Glide
    }
}

fn vowel_of(first: u8) -> Vowel {
    match first {
        b'e' => Vowel::E,
        b'i' | b'y' => Vowel::I,
        b'o' => Vowel::O,
        b'u' => Vowel::U,
        _ => Vowel::A,
    }
}

/// The sentences of a text as the web's `/[^<ends>]+[<ends>]*/g` finds them, or the whole text
/// when none is found.
fn sentences<'a>(text: &'a str, ends: &[char]) -> Vec<&'a str> {
    let mut found = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        if ends.contains(&c) {
            continue;
        }
        let mut end = start + c.len_utf8();
        while let Some(&(at, c)) = chars.peek() {
            if ends.contains(&c) {
                break;
            }
            end = at + c.len_utf8();
            chars.next();
        }
        while let Some(&(at, c)) = chars.peek() {
            if !ends.contains(&c) {
                break;
            }
            end = at + c.len_utf8();
            chars.next();
        }
        found.push(&text[start..end]);
    }
    if found.is_empty() { vec![text] } else { found }
}

/// A sentence's words, as `trim().split(/\s+/).filter(Boolean)` makes them.
fn words(sentence: &str) -> Vec<&str> {
    js::trim(sentence)
        .split(js::is_space)
        .filter(|word| !word.is_empty())
        .collect()
}

/// How long the jingle at the start and the end takes, and a chime between chapters.
const JINGLE: f64 = 2.6;
const CHIME: f64 = 1.2;

fn make_script(episode: &Episode, show: &Show) -> Script {
    let mut random = Seeded::new(&format!("{}:voice", episode.id));
    let mut chapters = Vec::new();
    let mut lines = Vec::new();
    let mut syllables = Vec::new();
    let mut cues = vec![Cue {
        at: 0.0,
        notes: show.jingle.clone(),
        step: 0.26,
    }];
    let mut at = JINGLE;
    let mut previous: Option<Host> = None;
    for (index, (title, said)) in written(episode, show).into_iter().enumerate() {
        if index > 0 {
            cues.push(Cue {
                at: at + 0.2,
                notes: vec![
                    show.jingle.first().copied().unwrap_or(0),
                    show.jingle.get(2).copied().unwrap_or(7),
                ],
                step: 0.18,
            });
            at += CHIME;
        }
        chapters.push(MediaChapter {
            title,
            start: if index == 0 {
                0.0
            } else {
                js::round((at - CHIME) * 10.0) / 10.0
            },
        });
        for line in said {
            if previous.is_some_and(|previous| previous != line.speaker) {
                at += 0.25;
            }
            previous = Some(line.speaker);
            let voice = voice_of(line.speaker);
            lines.push(TranscriptLine {
                start: js::round(at * 10.0) / 10.0,
                speaker: Some(line.speaker.name().to_owned()),
                text: line.text.clone(),
            });
            for sentence in sentences(&line.text, &['.', '?', '!', ';', ':']) {
                let words = words(sentence);
                let asking = js::trim(sentence).ends_with('?');
                let parts: Vec<Vec<(Onset, Vowel)>> = words.iter().map(|word| syllables_of(word)).collect();
                let total = parts.iter().map(Vec::len).sum::<usize>() as f64;
                let mut counted = 0.0_f64;
                for (place, word) in words.iter().enumerate() {
                    let part = &parts[place];
                    for (within, (onset, vowel)) in part.iter().enumerate() {
                        let share = counted / (total - 1.0).max(1.0);
                        // The voice falls through a sentence, and a question rises at its end.
                        let fall = 1.12 - 0.26 * share;
                        let rise = if asking && counted >= total - 2.0 {
                            1.18 + 0.1 * (counted - total + 2.0)
                        } else {
                            1.0
                        };
                        let stress = if within == 0 && part.len() > 1 { 1.08 } else { 1.0 };
                        let wobble = 1.0 + (random.next_f64() - 0.5) * 0.08;
                        let length =
                            voice.syllable * (0.78 + random.next_f64() * 0.44) * if stress > 1.0 { 1.15 } else { 1.0 };
                        let gain = (0.82 + random.next_f64() * 0.3) * if stress > 1.0 { 1.12 } else { 1.0 };
                        syllables.push(Syllable {
                            at,
                            length,
                            vowel: *vowel,
                            onset: *onset,
                            pitch: voice.pitch * fall * stress * wobble * rise,
                            pitch_end: voice.pitch * (fall - 0.03) * wobble * rise,
                            gain,
                            last: within == part.len() - 1,
                            voice: line.speaker,
                        });
                        at += length;
                        counted += 1.0;
                    }
                    at += 0.05 + if word.ends_with(',') { 0.18 } else { 0.0 };
                }
                at += if asking { 0.34 } else { 0.28 };
            }
            at += 0.3;
        }
    }
    at += 0.4;
    cues.push(Cue {
        at,
        notes: show.jingle.iter().rev().copied().collect(),
        step: 0.3,
    });
    Script {
        duration: ((at + JINGLE) * 10.0).ceil() / 10.0,
        chapters,
        lines,
        syllables,
        cues,
    }
}

/// An episode's script, timed: made once and kept. None for an id the catalog lacks.
pub fn script_of(id: &str) -> Option<Arc<Script>> {
    static SCRIPTS: OnceLock<Mutex<HashMap<String, Arc<Script>>>> = OnceLock::new();
    let kept = SCRIPTS.get_or_init(Default::default);
    if let Some(found) = kept.lock().ok().and_then(|map| map.get(id).cloned()) {
        return Some(found);
    }
    let episode = episode_by_id(id)?;
    let show = show_by_id(&episode.show)?;
    let made = Arc::new(make_script(episode, show));
    if let Ok(mut map) = kept.lock() {
        map.insert(id.to_owned(), made.clone());
    }
    Some(made)
}

/// The length of an episode in seconds; 0 for an id the catalog lacks.
pub fn length_of(id: &str) -> f64 {
    script_of(id).map_or(0.0, |script| script.duration)
}

/// A line said alone, timed as an episode's are, for an episode's summary read aloud.
pub fn spoken_script(text: &str, speaker: Host, seed: &str) -> Script {
    let voice = voice_of(speaker);
    let mut random = Seeded::new(seed);
    let mut syllables = Vec::new();
    let mut at = 0.3;
    for sentence in sentences(text, &['.', '?', '!']) {
        let words = words(sentence);
        let parts: Vec<Vec<(Onset, Vowel)>> = words.iter().map(|word| syllables_of(word)).collect();
        let total = parts.iter().map(Vec::len).sum::<usize>() as f64;
        let mut counted = 0.0_f64;
        for part in &parts {
            for (within, (onset, vowel)) in part.iter().enumerate() {
                let fall = 1.1 - 0.24 * (counted / (total - 1.0).max(1.0));
                let length = voice.syllable * (0.8 + random.next_f64() * 0.4);
                let gain = 0.85 + random.next_f64() * 0.25;
                syllables.push(Syllable {
                    at,
                    length,
                    vowel: *vowel,
                    onset: *onset,
                    pitch: voice.pitch * fall,
                    pitch_end: voice.pitch * (fall - 0.03),
                    gain,
                    last: within == part.len() - 1,
                    voice: speaker,
                });
                at += length;
                counted += 1.0;
            }
            at += 0.05;
        }
        at += 0.3;
    }
    Script {
        duration: at + 0.3,
        chapters: Vec::new(),
        lines: vec![TranscriptLine {
            start: 0.3,
            speaker: Some(speaker.name().to_owned()),
            text: text.to_owned(),
        }],
        syllables,
        cues: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syllables_as_the_web_reads_them() {
        assert_eq!(syllables_of("tide"), vec![(Onset::Stop, Vowel::I)]);
        assert_eq!(
            syllables_of("harbor"),
            vec![(Onset::Breath, Vowel::A), (Onset::Glide, Vowel::O)]
        );
        assert_eq!(syllables_of("..."), vec![(Onset::None, Vowel::A)]);
        assert_eq!(sentences("A. B? c", &['.', '?']), vec!["A.", " B?", " c"]);
        assert_eq!(sentences("...", &['.']), vec!["..."]);
    }
}
