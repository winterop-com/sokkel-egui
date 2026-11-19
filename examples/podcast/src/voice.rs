//! The sound of an episode, as the web's `voice.ts` makes it: each syllable of the script spoken by
//! a small formant voice (a glottal buzz at the host's pitch through three resonances shaped like
//! the vowel, with a hiss, a breath, a stop or a hum before it), and the show's jingle in bells at
//! the start, between chapters and at the end. No words are said, but the voices rise and fall as
//! speech does, line by line, in time with the transcript. The samples are kept as 32-bit floats,
//! as the web's `Float32Array`s keep them, so they are the web's to the bit; `wav_of` writes them
//! as a file and `levels_of` gives the player's waveform. Playing them is the app's.

use std::f64::consts::PI;

use crate::script::{Cue, Onset, Script, Vowel, voice_of};

/// Samples a second.
pub const RATE: u32 = 16_000;
const RATE_F: f64 = RATE as f64;

/// Samples between updates of the voice's controls.
const BLOCK: usize = 32;

/// The first three resonances of each vowel for a low voice, in hertz; a higher voice scales them up.
fn formants(vowel: Vowel) -> [f64; 3] {
    match vowel {
        Vowel::A => [730.0, 1190.0, 2440.0],
        Vowel::E => [530.0, 1840.0, 2480.0],
        Vowel::I => [300.0, 2200.0, 2950.0],
        Vowel::O => [520.0, 900.0, 2380.0],
        Vowel::U => [340.0, 870.0, 2240.0],
    }
}

/// Where the mouth is during a consonant: the resonances it moves from into the vowel.
fn onset_formants(onset: Onset) -> [f64; 3] {
    match onset {
        Onset::Hiss => [400.0, 1700.0, 2600.0],
        Onset::Breath => [500.0, 1500.0, 2500.0],
        Onset::Stop => [350.0, 1600.0, 2500.0],
        Onset::Hum => [260.0, 1100.0, 2400.0],
        Onset::Glide => [320.0, 900.0, 2300.0],
        Onset::None => [500.0, 1400.0, 2500.0],
    }
}

struct Controls {
    /// How loud the voiced part is, from 0.
    voice: Vec<f32>,
    /// How loud the noise is, and how bright (0 dull, 1 a hiss).
    noise: Vec<f32>,
    bright: Vec<f32>,
    pitch: Vec<f32>,
    f1: Vec<f32>,
    f2: Vec<f32>,
    f3: Vec<f32>,
}

/// Writes `write(index, share)` over the blocks from `from` to `to` seconds.
fn set(blocks: usize, from: f64, to: f64, mut write: impl FnMut(usize, f64)) {
    let block = |seconds: f64| {
        crate::js::round((seconds * RATE_F) / BLOCK as f64)
            .max(0.0)
            .min((blocks - 1) as f64) as usize
    };
    let start = block(from);
    let end = (start + 1).max(block(to));
    for index in start..end {
        write(index, (index - start) as f64 / 1.0_f64.max((end - start) as f64 - 1.0));
    }
}

fn controls_for(script: &Script, blocks: usize) -> Controls {
    let mut c = Controls {
        voice: vec![0.0; blocks],
        noise: vec![0.0; blocks],
        bright: vec![0.0; blocks],
        pitch: vec![150.0; blocks],
        f1: vec![500.0; blocks],
        f2: vec![1400.0; blocks],
        f3: vec![2500.0; blocks],
    };
    for syllable in &script.syllables {
        let scale = voice_of(syllable.voice).formant;
        let target = formants(syllable.vowel);
        let from = onset_formants(syllable.onset);
        let (at, length, gain) = (syllable.at, syllable.length, syllable.gain);
        // The consonant takes the first part of the syllable, the vowel the rest.
        let consonant: f64 = match syllable.onset {
            Onset::None => 0.02,
            Onset::Glide => 0.06,
            Onset::Hum => 0.07,
            _ => 0.06,
        };
        let vowel_start = at + consonant.min(length * 0.4);
        let end = at + length;
        set(blocks, at, end, |index, share| {
            c.pitch[index] = (syllable.pitch + (syllable.pitch_end - syllable.pitch) * share) as f32;
        });
        match syllable.onset {
            Onset::Hiss | Onset::Breath => {
                let hiss = syllable.onset == Onset::Hiss;
                set(blocks, at, vowel_start, |index, _| {
                    c.noise[index] = ((if hiss { 0.32 } else { 0.16 }) * gain) as f32;
                    c.bright[index] = if hiss { 1.0 } else { 0.25 };
                    c.voice[index] = 0.05;
                });
            }
            Onset::Stop => {
                let burst = at + (vowel_start - at) * 0.55;
                set(blocks, at, burst, |index, _| c.voice[index] = 0.0);
                set(blocks, burst, vowel_start, |index, _| {
                    c.noise[index] = (0.3 * gain) as f32;
                    c.bright[index] = 0.7;
                });
            }
            Onset::Hum => set(blocks, at, vowel_start, |index, _| {
                c.voice[index] = (0.45 * gain) as f32
            }),
            Onset::Glide | Onset::None => set(blocks, at, vowel_start, |index, share| {
                c.voice[index] = ((0.3 + 0.6 * share) * gain) as f32;
            }),
        }
        set(blocks, at, vowel_start, |index, _| {
            c.f1[index] = (from[0] * scale) as f32;
            c.f2[index] = (from[1] * scale) as f32;
            c.f3[index] = (from[2] * scale) as f32;
        });
        set(blocks, vowel_start, end, |index, share| {
            // A small dip between syllables of a word, and the voice let go at a word's end.
            let fade = if share > 0.75 {
                1.0 - (share - 0.75) * if syllable.last { 3.2 } else { 1.6 }
            } else {
                1.0
            };
            c.voice[index] = (gain * fade.max(0.0)) as f32;
            c.f1[index] = (target[0] * scale) as f32;
            c.f2[index] = (target[1] * scale) as f32;
            c.f3[index] = (target[2] * scale) as f32;
        });
    }
    c
}

/// A resonance: a two-pole band-pass filter whose center moves with its control.
#[derive(Default)]
struct Resonance {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
    b0: f64,
    a1: f64,
    a2: f64,
}

impl Resonance {
    fn tune(&mut self, frequency: f64, bandwidth: f64) {
        let w = (2.0 * PI * frequency.min(RATE_F * 0.45)) / RATE_F;
        let alpha = w.sin() / (2.0 * (frequency / bandwidth));
        let a0 = 1.0 + alpha;
        self.b0 = alpha / a0;
        self.a1 = (-2.0 * w.cos()) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    fn step(&mut self, input: f64) -> f64 {
        let output = self.b0 * input - self.b0 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }
}

/// A bell for the jingle: a note with two partials, struck and left to ring.
fn ring(samples: &mut [f32], at: f64, semitones: i32, loud: f64) {
    let frequency = 523.25 * 2.0_f64.powf(f64::from(semitones) / 12.0);
    let start = (at * RATE_F).floor() as usize;
    let length = (1.6 * RATE_F).floor() as usize;
    let mut index = 0;
    while index < length && start + index < samples.len() {
        let time = index as f64 / RATE_F;
        let decay = (-time * 3.2).exp() * 1.0_f64.min(time * 400.0);
        let tone = (2.0 * PI * frequency * time).sin()
            + 0.35 * (2.0 * PI * frequency * 2.0 * time).sin() * (-time * 4.0).exp()
            + 0.12 * (2.0 * PI * frequency * 3.01 * time).sin() * (-time * 6.0).exp();
        let sample = &mut samples[start + index];
        *sample = (f64::from(*sample) + tone * decay * loud) as f32;
        index += 1;
    }
}

fn play(samples: &mut [f32], cue: &Cue) {
    let last = cue.notes.len().saturating_sub(1);
    for (index, note) in cue.notes.iter().enumerate() {
        ring(
            samples,
            cue.at + index as f64 * cue.step,
            *note,
            if index == last { 0.2 } else { 0.16 },
        );
    }
}

/// A tiny noise source, the same every time.
struct Noise(u32);

impl Noise {
    fn new(seed: u32) -> Self {
        Self(if seed == 0 { 1 } else { seed })
    }

    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (f64::from(self.0) / 4_294_967_296.0) * 2.0 - 1.0
    }
}

/// The samples of a script, at 16 kHz.
pub fn render(script: &Script) -> Vec<f32> {
    let total = (script.duration * RATE_F).ceil() as usize;
    let blocks = total.div_ceil(BLOCK) + 1;
    let controls = controls_for(script, blocks);
    let mut samples = vec![0.0_f32; total];
    let mut resonances = [Resonance::default(), Resonance::default(), Resonance::default()];
    let mut noise = Noise::new((script.syllables.len() as u32).wrapping_mul(7919));
    let mut phase = 0.0_f64;
    // Smoothed controls, so the mouth moves rather than jumps.
    let (mut voice, mut hiss, mut bright, mut pitch) = (0.0_f64, 0.0_f64, 0.0_f64, 150.0_f64);
    let (mut f1, mut f2, mut f3) = (500.0_f64, 1400.0_f64, 2500.0_f64);
    let mut previous_noise = 0.0_f64;
    let mut low = 0.0_f64;
    for block in 0..blocks - 1 {
        f1 += (f64::from(controls.f1[block]) - f1) * 0.35;
        f2 += (f64::from(controls.f2[block]) - f2) * 0.35;
        f3 += (f64::from(controls.f3[block]) - f3) * 0.35;
        resonances[0].tune(f1, 90.0);
        resonances[1].tune(f2, 110.0);
        resonances[2].tune(f3, 170.0);
        let end = total.min((block + 1) * BLOCK);
        for sample in samples.iter_mut().take(end).skip(block * BLOCK) {
            voice += (f64::from(controls.voice[block]) - voice) * 0.012;
            hiss += (f64::from(controls.noise[block]) - hiss) * 0.02;
            bright += (f64::from(controls.bright[block]) - bright) * 0.02;
            pitch += (f64::from(controls.pitch[block]) - pitch) * 0.004;
            // A falling saw at the pitch, with its corner smoothed so it does not alias.
            let increment = pitch / RATE_F;
            phase += increment;
            if phase >= 1.0 {
                phase -= 1.0;
            }
            let mut saw = 1.0 - 2.0 * phase;
            if phase < increment {
                let t = phase / increment;
                saw += t + t - t * t - 1.0;
            } else if phase > 1.0 - increment {
                let t = (phase - 1.0) / increment;
                saw += t * t + t + t + 1.0;
            }
            let breath = noise.next();
            let source = saw * voice + breath * 0.04 * voice;
            let voiced =
                resonances[0].step(source) + 0.6 * resonances[1].step(source) + 0.28 * resonances[2].step(source);
            let raw = noise.next();
            let sharp = raw - previous_noise;
            previous_noise = raw;
            let consonant = (sharp * bright + raw * (1.0 - bright) * 0.6) * hiss;
            low += (voiced * 1.6 + consonant - low) * 0.7;
            *sample = low as f32;
        }
    }
    for cue in &script.cues {
        play(&mut samples, cue);
    }
    let peak = samples
        .iter()
        .fold(0.0_f64, |peak, sample| peak.max(f64::from(*sample).abs()));
    let gain = if peak > 0.0 { 0.88 / peak } else { 1.0 };
    for sample in &mut samples {
        *sample = (f64::from(*sample) * gain) as f32;
    }
    samples
}

/// The loudness across the samples, `count` levels from 0 to 1, for the player's waveform (the
/// web's default is 180).
pub fn levels_of(samples: &[f32], count: usize) -> Vec<f64> {
    let size = (samples.len() / count).max(1);
    let levels: Vec<f64> = (0..count)
        .map(|bucket| {
            let from = bucket * size;
            let mut sum = 0.0_f64;
            let mut index = from;
            while index < from + size && index < samples.len() {
                let sample = f64::from(samples[index]);
                sum += sample * sample;
                index += 1;
            }
            (sum / size as f64).sqrt()
        })
        .collect();
    let top = levels.iter().fold(1e-6_f64, |top, level| top.max(*level));
    levels.iter().map(|level| 1.0_f64.min(level / top)).collect()
}

/// The waveform's level count the player draws.
pub const LEVELS: usize = 180;

/// The samples as a 16-bit mono WAV file.
pub fn wav_of(samples: &[f32]) -> Vec<u8> {
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
        let sample = f64::from(*sample).clamp(-1.0, 1.0);
        let scaled = if sample < 0.0 {
            sample * 32_768.0
        } else {
            sample * 32_767.0
        };
        out.extend_from_slice(&(scaled.trunc() as i16).to_le_bytes());
    }
    out
}
