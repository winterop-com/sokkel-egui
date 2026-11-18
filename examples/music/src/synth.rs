//! The synthesizer every song is made by, as the web's `synth.ts` makes it: from a song's seed and
//! its style it draws a tempo, a key, two chord progressions, a bass line, an arpeggio, a melody and
//! a drum pattern, and renders them into samples, so the same seed always makes the same song, to
//! the bit. A song is a few phrases of four bars, from about twenty seconds to a minute.
//!
//! The samples are kept as 32-bit floats, each sum made in 64 bits and stored back, as the web's
//! `Float32Array` keeps them. Playing them is the app's: `render` and `wav` only make them.

use serde::{Deserialize, Serialize};

use crate::js;

/// Samples a second.
pub const RATE: u32 = 22_050;
const RATE_F: f64 = RATE as f64;

/// How the music sounds: which voices play and how fast.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    Ambient,
    Folk,
    Pop,
    Electronic,
    Chamber,
    Choir,
    Band,
}

impl Style {
    pub const ALL: [Style; 7] = [
        Style::Ambient,
        Style::Folk,
        Style::Pop,
        Style::Electronic,
        Style::Chamber,
        Style::Choir,
        Style::Band,
    ];

    /// The style as the web names it: `ambient`, `folk`, ...
    pub fn key(self) -> &'static str {
        match self {
            Style::Ambient => "ambient",
            Style::Folk => "folk",
            Style::Pop => "pop",
            Style::Electronic => "electronic",
            Style::Chamber => "chamber",
            Style::Choir => "choir",
            Style::Band => "band",
        }
    }

    /// The style read from its web name.
    pub fn from_key(key: &str) -> Option<Style> {
        Style::ALL.into_iter().find(|style| style.key() == key)
    }

    /// What kind of sound it is, in words (`STYLE_NAMES`).
    pub fn name(self) -> &'static str {
        match self {
            Style::Ambient => "Ambient",
            Style::Folk => "Folk",
            Style::Pop => "Pop",
            Style::Electronic => "Electronic",
            Style::Chamber => "Chamber",
            Style::Choir => "Choral",
            Style::Band => "Band",
        }
    }

    fn tempo(self) -> (f64, f64) {
        match self {
            Style::Ambient => (60.0, 76.0),
            Style::Folk => (84.0, 104.0),
            Style::Pop => (96.0, 118.0),
            Style::Electronic => (118.0, 128.0),
            Style::Chamber => (72.0, 92.0),
            Style::Choir => (58.0, 70.0),
            Style::Band => (104.0, 124.0),
        }
    }
}

/// A small seeded generator, the same run for the same seed (mulberry32), numbers from 0 up to 1.
#[derive(Clone, Debug)]
pub struct Seeded {
    state: u32,
}

impl Seeded {
    /// A generator from a seed taken as `seed >>> 0`.
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    pub fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let state = self.state;
        let mut mixed = (state ^ (state >> 15)).wrapping_mul(1 | state);
        mixed ^= mixed.wrapping_add((mixed ^ (mixed >> 7)).wrapping_mul(61 | mixed));
        f64::from(mixed ^ (mixed >> 14)) / 4_294_967_296.0
    }
}

/// `seeded(seed)`: a generator as a closure.
pub fn seeded(seed: u32) -> impl FnMut() -> f64 {
    let mut generator = Seeded::new(seed);
    move || generator.next_f64()
}

/// Seconds of ring left after the last bar.
const TAIL: f64 = 1.5;

const MAJOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const MINOR: [i32; 7] = [0, 2, 3, 5, 7, 8, 10];
const DORIAN: [i32; 7] = [0, 2, 3, 5, 7, 9, 10];

const MAJOR_WAYS: [[i32; 4]; 6] = [
    [0, 4, 5, 3],
    [0, 5, 3, 4],
    [5, 3, 0, 4],
    [0, 3, 0, 4],
    [3, 0, 4, 5],
    [0, 2, 3, 4],
];
const MINOR_WAYS: [[i32; 4]; 5] = [[0, 5, 2, 6], [0, 3, 4, 0], [0, 6, 5, 6], [0, 5, 3, 4], [3, 4, 0, 0]];

struct Plan {
    tempo: f64,
    bars: usize,
    /// The key's root, as a MIDI note in the bass's octave.
    root: i32,
    scale: &'static [i32; 7],
    verse: [i32; 4],
    chorus: [i32; 4],
    random: Seeded,
}

fn plan_of(seed: u32, style: Style) -> Plan {
    let mut random = Seeded::new(seed);
    let (slow, fast) = style.tempo();
    let tempo = js::round(slow + random.next_f64() * (fast - slow));
    let bar = (4.0 * 60.0) / tempo;
    let target = 22.0 + random.next_f64() * 40.0;
    let bars = (js::round(target / bar / 4.0) * 4.0).max(8.0) as usize;
    let mode = random.next_f64();
    let scale: &'static [i32; 7] = if style == Style::Choir || style == Style::Ambient {
        if mode < 0.5 { &MINOR } else { &DORIAN }
    } else if mode < 0.55 {
        &MAJOR
    } else if mode < 0.85 {
        &MINOR
    } else {
        &DORIAN
    };
    let ways: &[[i32; 4]] = if *scale == MAJOR { &MAJOR_WAYS } else { &MINOR_WAYS };
    let verse_at = (random.next_f64() * ways.len() as f64).floor() as usize;
    let mut chorus_at = (random.next_f64() * ways.len() as f64).floor() as usize;
    if chorus_at == verse_at {
        chorus_at = (verse_at + 1) % ways.len();
    }
    let root = 38 + (random.next_f64() * 10.0).floor() as i32;
    Plan {
        tempo,
        bars,
        root,
        scale,
        verse: ways[verse_at],
        chorus: ways[chorus_at],
        random,
    }
}

/// The song's bars: how long one lasts in seconds and how many there are, for words timed to them.
pub fn bars_of(seed: u32, style: Style) -> (f64, usize) {
    let plan = plan_of(seed, style);
    ((4.0 * 60.0) / plan.tempo, plan.bars)
}

/// How long the song made from `seed` in `style` lasts, in seconds, before a note of it is made.
pub fn length_of(seed: u32, style: Style) -> f64 {
    let plan = plan_of(seed, style);
    ((plan.bars as f64 * ((4.0 * 60.0) / plan.tempo) + TAIL) * RATE_F).ceil() / RATE_F
}

fn hz(note: i32) -> f64 {
    440.0 * 2f64.powf(f64::from(note - 69) / 12.0)
}

/// The MIDI note of `degree` in the scale over `root`, degrees past the scale climbing octaves.
fn note_of(plan: &Plan, degree: i32, octave: i32) -> i32 {
    let steps = plan.scale.len() as i32;
    let wrapped = degree.rem_euclid(steps);
    plan.root + plan.scale[wrapped as usize] + 12 * (octave + degree.div_euclid(steps))
}

const SIZE: usize = 2048;

/// One cycle of a wave made of the overtones given, its peak at 1.
fn table(overtones: &[f64]) -> Vec<f32> {
    let mut sums = vec![0.0_f64; SIZE];
    let mut out = vec![0.0_f32; SIZE];
    let mut peak = 0.0_f64;
    for (index, slot) in sums.iter_mut().enumerate() {
        let mut sum = 0.0;
        for (overtone, weight) in overtones.iter().enumerate() {
            sum += weight * ((2.0 * std::f64::consts::PI * (overtone + 1) as f64 * index as f64) / SIZE as f64).sin();
        }
        *slot = sum;
        out[index] = sum as f32;
        peak = peak.max(sum.abs());
    }
    for value in &mut out {
        *value = (f64::from(*value) / peak) as f32;
    }
    out
}

struct Waves {
    soft: Vec<f32>,
    pad: Vec<f32>,
    pluck: Vec<f32>,
    hollow: Vec<f32>,
    voice: Vec<f32>,
    bass: Vec<f32>,
}

fn waves() -> &'static Waves {
    static WAVES: std::sync::OnceLock<Waves> = std::sync::OnceLock::new();
    WAVES.get_or_init(|| Waves {
        soft: table(&[1.0, 0.12, 0.04]),
        pad: table(&[1.0, 0.5, 0.33, 0.25, 0.2, 0.16]),
        pluck: table(&[1.0, 0.6, 0.35, 0.2, 0.12, 0.06]),
        hollow: table(&[1.0, 0.0, 0.33, 0.0, 0.2, 0.0, 0.14]),
        voice: table(&[1.0, 0.7, 0.9, 0.4, 0.15, 0.3, 0.1]),
        bass: table(&[1.0, 0.45, 0.12]),
    })
}

struct Envelope {
    /// Seconds to rise.
    attack: f64,
    /// Seconds over which a plucked note falls by e; none for a held one.
    decay: Option<f64>,
    /// Seconds to fall at the note's end.
    release: f64,
}

/// Adds `value` into a sample, the sum made in 64 bits and kept in 32.
#[inline]
fn add(sample: &mut f32, value: f64) {
    *sample = (f64::from(*sample) + value) as f32;
}

/// Adds a note into `out`: a wave at `frequency` under an envelope, optionally wavering.
#[allow(clippy::too_many_arguments)]
fn note(
    out: &mut [f32],
    start: f64,
    length: f64,
    frequency: f64,
    gain: f64,
    wave: &[f32],
    envelope: &Envelope,
    vibrato: f64,
) {
    let from = (start * RATE_F).floor().max(0.0);
    let end = (out.len() as f64).min(((start + length + envelope.release) * RATE_F).floor());
    let step = (frequency * SIZE as f64) / RATE_F;
    let held = length * RATE_F;
    let attack = (envelope.attack * RATE_F).max(1.0);
    let release = (envelope.release * RATE_F).max(1.0);
    let fall = envelope.decay.map_or(0.0, |decay| 1.0 / (decay * RATE_F));
    let mut phase = 0.0_f64;
    let (from, end) = (from as i64, end as i64);
    let mut index = from;
    while index < end {
        let at = (index - from) as f64;
        let mut level = if at < attack { at / attack } else { 1.0 };
        if fall > 0.0 {
            level *= (-at * fall).exp();
        }
        if at > held {
            level *= (1.0 - (at - held) / release).max(0.0);
        }
        let wobble = if vibrato == 0.0 {
            1.0
        } else {
            1.0 + vibrato * ((2.0 * std::f64::consts::PI * 5.2 * at) / RATE_F).sin()
        };
        phase += step * wobble;
        if phase >= SIZE as f64 {
            phase -= SIZE as f64;
        }
        add(
            &mut out[index as usize],
            gain * level * f64::from(wave[phase as i32 as usize]),
        );
        index += 1;
    }
}

/// A kick: a sine falling from a thump to a boom.
fn kick(out: &mut [f32], start: f64, gain: f64) {
    let from = (start * RATE_F).floor() as usize;
    let length = (0.4 * RATE_F).floor() as usize;
    let mut phase = 0.0_f64;
    let mut at = 0;
    while at < length && from + at < out.len() {
        let time = at as f64 / RATE_F;
        let frequency = 45.0 + 75.0 * (-time / 0.04).exp();
        phase += (2.0 * std::f64::consts::PI * frequency) / RATE_F;
        add(&mut out[from + at], gain * (-time / 0.16).exp() * phase.sin());
        at += 1;
    }
}

/// A snare or a clap: a burst of noise over a short tone.
fn snare(out: &mut [f32], start: f64, gain: f64, noise: &mut Seeded, clap: bool) {
    let from = (start * RATE_F).floor() as usize;
    let length = (0.3 * RATE_F).floor() as usize;
    let mut at = 0;
    while at < length && from + at < out.len() {
        let time = at as f64 / RATE_F;
        let burst = if clap {
            (-((time % 0.012) / 0.004)).exp() * (if time < 0.036 { 1.0 } else { 0.0 }) + (-time / 0.07).exp() * 0.6
        } else {
            (-time / 0.09).exp()
        };
        let tone = if clap {
            0.0
        } else {
            0.5 * (-time / 0.05).exp() * (2.0 * std::f64::consts::PI * 185.0 * time).sin()
        };
        add(
            &mut out[from + at],
            gain * (burst * (noise.next_f64() * 2.0 - 1.0) * 0.7 + tone),
        );
        at += 1;
    }
}

/// A hat: bright noise, closed and short or open and long.
fn hat(out: &mut [f32], start: f64, gain: f64, noise: &mut Seeded, open: bool) {
    let from = (start * RATE_F).floor() as usize;
    let decay = if open { 0.14 } else { 0.025 };
    let length = (decay * 6.0 * RATE_F).floor() as usize;
    let mut last = 0.0;
    let mut at = 0;
    while at < length && from + at < out.len() {
        let sample = noise.next_f64() * 2.0 - 1.0;
        add(
            &mut out[from + at],
            gain * (-(at as f64) / RATE_F / decay).exp() * (sample - last),
        );
        last = sample;
        at += 1;
    }
}

/// Repeats what is in `bus` after `delay` seconds, each echo `feedback` as loud as the last.
fn echo(bus: &mut [f32], delay: f64, feedback: f64) {
    let lag = (delay * RATE_F).floor() as usize;
    for index in lag..bus.len() {
        let before = f64::from(bus[index - lag]);
        add(&mut bus[index], before * feedback);
    }
}

fn mix_into(out: &mut [f32], bus: &[f32], gain: f64) {
    for (sample, from) in out.iter_mut().zip(bus) {
        add(sample, f64::from(*from) * gain);
    }
}

/// Renders the song made from `seed` in `style` as samples at `RATE`, from -1 to 1.
pub fn render(seed: u32, style: Style) -> Vec<f32> {
    use Style::*;
    let waves = waves();
    let mut plan = plan_of(seed, style);
    let beat = 60.0 / plan.tempo;
    let bar = 4.0 * beat;
    let length = ((plan.bars as f64 * bar + TAIL) * RATE_F).ceil() as usize;
    let mut out = vec![0.0_f32; length];
    let mut fx = vec![0.0_f32; length];
    let mut noise = Seeded::new(seed ^ 0x5bd1_e995);
    let phrases = plan.bars / 4;
    let drums = matches!(style, Folk | Pop | Electronic | Band);
    let melodic = matches!(style, Folk | Pop | Chamber | Band);
    let mut melody_at = 4 + (plan.random.next_f64() * 3.0).floor() as i32;

    for index in 0..plan.bars {
        let phrase = index / 4;
        let chorus = phrases > 2 && phrase % 2 == 1;
        let intro = phrase == 0 && phrases > 2;
        let last = phrase == phrases - 1;
        let degree = (if chorus { plan.chorus } else { plan.verse })[index % 4];
        let start = index as f64 * bar;
        let chord: [i32; 3] = [0, 2, 4].map(|step| note_of(&plan, degree + step, 1));

        // The pad: the chord held across the bar, two voices a hair apart.
        let pad_wave: &[f32] = match style {
            Choir => &waves.voice,
            Electronic | Band => &waves.hollow,
            _ => &waves.pad,
        };
        let pad_gain = match style {
            Ambient | Choir => 0.16,
            Chamber => 0.11,
            _ => 0.07,
        };
        let swell = Envelope {
            attack: if matches!(style, Ambient | Choir) {
                bar * 0.45
            } else {
                0.08
            },
            decay: None,
            release: bar * 0.35,
        };
        for tone in chord {
            let vibrato = if style == Choir { 0.004 } else { 0.0 };
            note(
                &mut out,
                start,
                bar,
                hz(tone) * 1.003,
                pad_gain,
                pad_wave,
                &swell,
                vibrato,
            );
            note(&mut out, start, bar, hz(tone) * 0.997, pad_gain, pad_wave, &swell, 0.0);
        }

        // The bass.
        let low = hz(note_of(&plan, degree, 0));
        if matches!(style, Ambient | Choir) {
            let envelope = Envelope {
                attack: 0.4,
                decay: None,
                release: 0.8,
            };
            note(&mut out, start, bar, low, 0.22, &waves.soft, &envelope, 0.0);
        } else if matches!(style, Chamber | Folk) {
            let envelope = Envelope {
                attack: if style == Chamber { 0.12 } else { 0.005 },
                decay: Some(0.9),
                release: 0.1,
            };
            note(&mut out, start, bar / 2.0, low, 0.26, &waves.bass, &envelope, 0.0);
            let fifth = hz(note_of(&plan, degree + 4, 0));
            note(
                &mut out,
                start + bar / 2.0,
                bar / 2.0,
                fifth,
                0.22,
                &waves.bass,
                &envelope,
                0.0,
            );
        } else {
            let envelope = Envelope {
                attack: 0.004,
                decay: Some(0.25),
                release: 0.04,
            };
            for step in 0..8 {
                if style == Electronic && step % 2 == 0 {
                    continue;
                }
                let octave = if style == Pop && step % 2 == 1 { 2.0 } else { 1.0 };
                let at = start + (f64::from(step) * beat) / 2.0;
                note(
                    &mut out,
                    at,
                    beat / 2.0,
                    low * octave,
                    0.24,
                    &waves.bass,
                    &envelope,
                    0.0,
                );
            }
        }

        // The arpeggio, into the echo's bus.
        if style != Choir && style != Band && !(intro && style != Ambient) {
            let tones = [chord[0] + 12, chord[1] + 12, chord[2] + 12, chord[1] + 24];
            let steps = match style {
                Electronic => 16,
                Ambient => 4,
                _ => 8,
            };
            let wave: &[f32] = match style {
                Electronic => &waves.hollow,
                Ambient => &waves.soft,
                _ => &waves.pluck,
            };
            let envelope = Envelope {
                attack: 0.003,
                decay: Some(if style == Ambient { 1.2 } else { 0.22 }),
                release: 0.05,
            };
            for step in 0..steps {
                if style == Ambient && plan.random.next_f64() < 0.4 {
                    continue;
                }
                let pattern = match style {
                    Folk => [0, 2, 1, 2][step % 4],
                    Chamber => [0, 1, 2, 3, 2, 1, 2, 1][step % 8],
                    _ => step % 4,
                };
                let length = bar / steps as f64;
                let gain = if style == Ambient { 0.1 } else { 0.075 };
                let at = start + step as f64 * length;
                note(&mut fx, at, length, hz(tones[pattern]), gain, wave, &envelope, 0.0);
            }
        }

        // The melody: a walk over the scale, landing on the chord's tones.
        if melodic && !intro && (chorus || plan.random.next_f64() < 0.6) {
            let lead: &[f32] = match style {
                Band => &waves.hollow,
                Chamber => &waves.soft,
                _ => &waves.pluck,
            };
            let mut at = 0.0;
            while at < 4.0 {
                let length = if plan.random.next_f64() < 0.35 {
                    1.0
                } else if plan.random.next_f64() < 0.5 {
                    0.5
                } else {
                    2.0
                };
                let span = f64::min(length, 4.0 - at);
                melody_at += (plan.random.next_f64() * 5.0).floor() as i32 - 2;
                melody_at = melody_at.clamp(0, 9);
                let degree_now = if at == 0.0 {
                    degree + [0, 2, 4][(plan.random.next_f64() * 3.0).floor() as usize]
                } else {
                    melody_at
                };
                if plan.random.next_f64() > 0.15 {
                    let envelope = Envelope {
                        attack: if style == Chamber { 0.06 } else { 0.01 },
                        decay: if style == Chamber { None } else { Some(0.8) },
                        release: 0.08,
                    };
                    let tone = hz(note_of(&plan, degree_now, 2));
                    let gain = if style == Band { 0.09 } else { 0.11 };
                    let vibrato = if style == Chamber { 0.006 } else { 0.002 };
                    note(
                        &mut fx,
                        start + at * beat,
                        span * beat * 0.92,
                        tone,
                        gain,
                        lead,
                        &envelope,
                        vibrato,
                    );
                }
                at += span;
            }
        }

        // The drums, out of the intro, thinning in the last bar.
        if drums && !intro {
            let fill = last && index == plan.bars - 1;
            for step in 0..16 {
                let time = start + (f64::from(step) * beat) / 4.0;
                if fill && step > 8 {
                    continue;
                }
                let on_beat = step % 4 == 0;
                if style == Electronic {
                    if on_beat {
                        kick(&mut out, time, 0.55);
                    }
                    if step == 4 || step == 12 {
                        snare(&mut out, time, 0.32, &mut noise, true);
                    }
                    if step % 4 == 2 {
                        hat(&mut out, time, 0.16, &mut noise, true);
                    } else {
                        hat(&mut out, time, 0.07, &mut noise, false);
                    }
                } else if style == Folk {
                    if step == 0 || step == 8 {
                        kick(&mut out, time, 0.32);
                    }
                    if step == 12 && plan.random.next_f64() < 0.6 {
                        snare(&mut out, time, 0.1, &mut noise, false);
                    }
                    if step % 2 == 0 {
                        hat(
                            &mut out,
                            time,
                            if step % 4 == 0 { 0.05 } else { 0.08 },
                            &mut noise,
                            false,
                        );
                    }
                } else {
                    let loud = if style == Band { 1.25 } else { 1.0 };
                    if step == 0
                        || step == 8
                        || (step == 6 && plan.random.next_f64() < 0.5)
                        || (style == Band && step == 10)
                    {
                        kick(&mut out, time, 0.5 * loud);
                    }
                    if step == 4 || step == 12 {
                        snare(&mut out, time, 0.26 * loud, &mut noise, false);
                    }
                    if step % 2 == 0 {
                        hat(&mut out, time, 0.07 * loud, &mut noise, false);
                    }
                }
            }
            if style == Band && index % 4 == 0 && !intro {
                hat(&mut out, start, 0.22, &mut noise, true);
            }
        }
    }

    echo(&mut fx, beat * 0.75, if style == Ambient { 0.5 } else { 0.32 });
    mix_into(&mut out, &fx, 1.0);

    // A little room: two short echoes of everything, quiet.
    let mut room = out.clone();
    echo(&mut room, 0.043, 0.45);
    echo(&mut room, 0.061, 0.4);
    mix_into(
        &mut out,
        &room,
        if matches!(style, Ambient | Choir) { 0.18 } else { 0.08 },
    );

    // Warmed, leveled, softly clipped, faded in and out.
    let mut warm = 0.0_f64;
    let mut peak = 0.0_f64;
    let smooth = match style {
        Electronic | Band => 0.85,
        Ambient | Choir => 0.6,
        _ => 0.75,
    };
    for sample in &mut out {
        warm += smooth * (f64::from(*sample) - warm);
        *sample = warm as f32;
        peak = peak.max(warm.abs());
    }
    let scale = if peak == 0.0 { 1.0 } else { 1.1 / peak };
    let fade_in = (0.03 * RATE_F).floor() as usize;
    let fade_out = (2.5 * RATE_F).floor() as usize;
    let count = out.len();
    for (index, sample) in out.iter_mut().enumerate() {
        let mut level = (f64::from(*sample) * scale).tanh() * 0.92;
        if index < fade_in {
            level *= index as f64 / fade_in as f64;
        }
        let left = count - index;
        if left < fade_out {
            level *= left as f64 / fade_out as f64;
        }
        *sample = level as f32;
    }
    out
}

/// The samples as a WAV file: 16-bit, one channel, at `RATE`.
pub fn wav(samples: &[f32]) -> Vec<u8> {
    let data = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for sample in samples {
        let value = js::round(f64::from(*sample).clamp(-1.0, 1.0) * 32_767.0) as i16;
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

/// The song's loudness in `bins` steps from start to end, each from 0 to 1, for the player's
/// waveform: stretched between the quietest step and the loudest, so the verses, choruses and
/// fades show. The web's default is 180 steps.
pub fn waveform_of(samples: &[f32], bins: usize) -> Vec<f64> {
    let size = (samples.len() / bins).max(1);
    let levels: Vec<f64> = (0..bins)
        .map(|bin| {
            let mut sum = 0.0_f64;
            let mut index = bin * size;
            while index < (bin + 1) * size && index < samples.len() {
                let sample = f64::from(samples[index]);
                sum += sample * sample;
                index += 1;
            }
            (sum / size as f64).sqrt()
        })
        .collect();
    let loudest = levels.iter().fold(1e-6_f64, |most, level| most.max(*level));
    let from = (bins as f64 * 0.05).floor() as usize;
    let to = (bins as f64 * 0.9).floor() as usize;
    let quietest = levels[from.min(to)..to]
        .iter()
        .fold(f64::INFINITY, |least, level| least.min(*level));
    let floor = quietest.min(loudest) * 0.85;
    levels
        .iter()
        .map(|level| (0.2 + (0.8 * (level - floor)) / (loudest - floor).max(1e-6)).clamp(0.04, 1.0))
        .collect()
}

/// The waveform's step count the player draws.
pub const WAVEFORM_BINS: usize = 180;
