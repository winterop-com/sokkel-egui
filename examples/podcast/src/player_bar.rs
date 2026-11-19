//! The player along the foot of the window, as the web's `Player.tsx`: what plays with its title
//! and show as links, the skip buttons, previous and next stepping through its chapters, the speed,
//! the volume, the sleep timer and the queue's button; the full screen with its transcript in the
//! visualizer's place and a star.

use eframe::egui::Ui;
use sokkel_egui::sound_visualizer::{VOICE_RANGE, band_levels, frequency_bytes};
use sokkel_egui::{Icon, SoundKind, SoundVisualizer, StarButton, TransportButton};
use sokkel_egui_media::{MediaBar, MediaBarWords, SleepTimerMenu, Transcript, chapter_starts};

use crate::RATES;
use crate::act::Act;
use crate::app::Sound;
use crate::catalog::{episode_by_id, hosts_of, show_by_id};
use crate::episodes::{Here, star_words};
use crate::voice::RATE;

/// How many samples the visualizer reads at a time, as the web's analyser's `fftSize`.
const WINDOW: usize = 1024;

/// The window of samples at `at` seconds.
fn samples_at(samples: &[f32], at: f64) -> &[f32] {
    let start = ((at * f64::from(RATE)) as usize).min(samples.len());
    let end = (start + WINDOW).min(samples.len());
    &samples[start..end]
}

/// The player bar, drawn while an episode is playing or paused.
pub fn player_bar(
    ui: &mut Ui,
    here: &Here,
    fullscreen: bool,
    queue_shown: bool,
    sound: Option<&Sound>,
    acts: &mut Vec<Act>,
) {
    let state = here.state;
    let Some(id) = state.current() else { return };
    let (Some(episode), Some(script), Some(row)) = (episode_by_id(id), state.script(), state.episode(id)) else {
        return;
    };
    let Some(show) = show_by_id(&episode.show) else { return };
    let starts = chapter_starts(&script.chapters);
    let position = state.position();
    let mut star = row.starred;
    let wave: Vec<f32> = sound
        .map(|sound| samples_at(&sound.samples, position).to_vec())
        .unwrap_or_default();
    let levels: Vec<f32> = if wave.len() == WINDOW {
        band_levels(&frequency_bytes(&wave), 64, RATE as f32, VOICE_RANGE)
    } else {
        Vec::new()
    };
    let queued = state.player.queue.len();
    let mut sleep_asked = None;
    let mut seek_line = None;
    let mut queue_pressed = false;

    let mut bar = MediaBar::new("player", &episode.title)
        .subtitle(&show.title)
        .artist(hosts_of(show))
        .album(&show.title)
        .artwork(show.cover)
        .linked(true)
        .playing(state.playing)
        .position(position, script.duration)
        .chapters(&starts)
        .skip(state.prefs.skip)
        .rate(state.player.rate)
        .rates(&RATES)
        .volume(state.player.volume as f32, state.player.muted)
        .fullscreen(fullscreen)
        .extra(|ui| {
            let out = SleepTimerMenu::new(state.sleep_timer())
                .remaining(state.sleep_remaining())
                .chapters(script.chapters.len() > 1)
                .show(ui);
            sleep_asked = out.asked;
            let label = format!("Queue, {queued} up next");
            queue_pressed = ui
                .add(TransportButton::new(Icon::ListOrdered, label).pressed(queue_shown))
                .clicked();
        })
        .fullscreen_actions(|ui| {
            ui.add(StarButton::new(&mut star).words(star_words()));
        })
        .lyrics(|ui| {
            if let Some(at) = Transcript::new("player-transcript", &script.lines)
                .position(position)
                .show(ui)
                .seek
            {
                seek_line = Some(at);
            }
        })
        .words(MediaBarWords {
            lyrics: "Transcript".into(),
            ..MediaBarWords::default()
        });
    if let Some(sound) = sound {
        bar = bar.waveform(&sound.waveform);
    }
    if !levels.is_empty() {
        let (levels, wave) = (&levels, &wave);
        bar = bar.visualizer(move |painter, rect| {
            SoundVisualizer::new()
                .kind(SoundKind::Spectrum)
                .levels(levels)
                .samples(wave)
                .label("What is playing")
                .paint(painter, rect);
        });
    }
    let out = bar.show(ui);

    if let Some(on) = out.playing {
        acts.push(Act::SetPlaying(on));
    }
    if let Some(at) = out.seek.or(seek_line) {
        acts.push(Act::Seek(at));
    }
    if let Some(by) = out.skip {
        acts.push(Act::Skip(by));
    }
    if let Some(rate) = out.rate {
        acts.push(Act::Rate(rate));
    }
    if let Some(volume) = out.volume {
        acts.push(Act::Volume(f64::from(volume)));
    }
    if let Some(muted) = out.muted {
        acts.push(Act::Muted(muted));
    }
    if let Some(open) = out.fullscreen {
        acts.push(Act::Fullscreen(open));
    }
    if out.title {
        acts.push(Act::Go(format!("episode/{}", episode.id)));
    }
    if out.subtitle {
        acts.push(Act::Go(format!("show/{}", show.id)));
    }
    if let Some(timer) = sleep_asked {
        acts.push(Act::Sleep(timer));
    }
    if queue_pressed {
        acts.push(Act::Go(if queue_shown { "home" } else { "queue" }.into()));
    }
    if star != row.starred {
        acts.push(Act::Star(episode.id.clone(), star));
    }
}
