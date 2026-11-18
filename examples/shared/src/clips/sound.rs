//! The sound under a clip, made sample by sample as sokkel-ui's `examples/shared/clips/sound.ts`
//! makes it: a soft chord of its own for every clip, and the sound of its scene under it (surf,
//! wind, rain, a train on its rails), the same every time. The arithmetic is the web's, in the same
//! order, each sample kept as a 32-bit float as a `Float32Array` keeps it.

use std::f64::consts::PI;

use super::scenes::{SceneKind, SceneSpec, Seeded};

/// Samples a second, in each channel.
pub const SOUND_RATE: u32 = 48_000;

/// The roots the chords are built on, in hertz, one picked per clip.
const ROOTS: [f64; 6] = [110.0, 123.47, 130.81, 146.83, 164.81, 174.61];

/// The two channels of the clip's sound, each `duration` seconds long.
pub fn synthesize(spec: &SceneSpec) -> (Vec<f32>, Vec<f32>) {
    let rate = f64::from(SOUND_RATE);
    let frames = (spec.duration * rate).ceil().max(0.0) as usize;
    let mut left = vec![0.0f32; frames];
    let mut right = vec![0.0f32; frames];
    let mut random = Seeded::new(spec.seed.wrapping_mul(31).wrapping_add(7));
    let root = ROOTS[(spec.seed as usize) % ROOTS.len()];
    struct Voice {
        frequency: f64,
        rate: f64,
        level: f64,
        pan: f64,
    }
    let voices: Vec<Voice> = [1.0, 1.5, 2.0, 2.52, 3.0]
        .iter()
        .enumerate()
        .map(|(index, ratio)| Voice {
            frequency: root * ratio,
            rate: 0.07 + index as f64 * 0.031,
            level: if index == 0 { 0.05 } else { 0.03 },
            pan: (if index % 2 == 0 { -1.0 } else { 1.0 }) * 0.3,
        })
        .collect();
    let mut brown = 0.0;
    let mut soft = 0.0;
    let mut wind = 0.0;
    let kind = spec.kind;
    let surf = matches!(
        kind,
        SceneKind::Sea | SceneKind::Lighthouse | SceneKind::Islands | SceneKind::Tide
    );
    let breeze = matches!(kind, SceneKind::Peaks | SceneKind::Aurora | SceneKind::Fields);
    let rails = kind == SceneKind::Train;
    let shower = kind == SceneKind::Rain;
    for index in 0..frames {
        let t = index as f64 / rate;
        // Fade in over the first second and out over the last two.
        let envelope = 1.0f64.min(t).min((spec.duration - t) / 2.0);
        let mut l = 0.0;
        let mut r = 0.0;
        for voice in &voices {
            let swell = 0.6 + 0.4 * (PI * 2.0 * voice.rate * t + voice.frequency).sin();
            let sample = (PI * 2.0 * voice.frequency * t).sin() * voice.level * swell;
            l += sample * (1.0 - voice.pan);
            r += sample * (1.0 + voice.pan);
        }
        let white = random.next() * 2.0 - 1.0;
        brown = (brown + 0.02 * white) / 1.02;
        soft += 0.08 * (white - soft);
        if surf {
            let rise = ((PI * 2.0 * t) / 7.3).sin().max(0.0);
            let noise = brown * 3.2 * (0.25 + 0.75 * (rise * rise));
            l += noise;
            r += noise * 0.9;
        }
        if breeze {
            wind += 0.02 * (white - wind);
            let gust = 0.5 + 0.5 * ((PI * 2.0 * t) / 9.0 + 1.0).sin();
            l += wind * 0.9 * gust;
            r += wind * 0.7 * gust;
        }
        if rails {
            let beat = t % 0.62;
            let clack = if beat < 0.03 || (beat > 0.16 && beat < 0.19) {
                white * 0.25 * (1.0 - (beat % 0.16) / 0.03)
            } else {
                0.0
            };
            let rumble = brown * 2.2;
            l += clack + rumble;
            r += clack * 0.8 + rumble;
        }
        if shower {
            l += soft * 0.5;
            r += soft * 0.5;
            let drip = (t * 3.1) % 1.0;
            if drip < 0.04 {
                let plink = (PI * 2.0 * (1800.0 - drip * 9000.0) * t).sin() * 0.05 * (1.0 - drip / 0.04);
                l += plink;
                r += plink * 0.6;
            }
        }
        left[index] = (l * envelope).clamp(-1.0, 1.0) as f32;
        right[index] = (r * envelope).clamp(-1.0, 1.0) as f32;
    }
    (left, right)
}
