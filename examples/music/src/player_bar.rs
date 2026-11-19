//! The player bar along the window's foot, as the web's `parts/Player.tsx`: what plays, the
//! transport, shuffle and repeat, the speed, the volume and the queue's button; grown from its top
//! edge into the layout the settings pick, with the song's waveform as its scrubber and the
//! visualizer behind it; and the full screen with its picker of looks, the words and a star.

use eframe::egui::Ui;
use sokkel_egui::sound_visualizer::{MUSIC_RANGE, band_levels, frequency_bytes};
use sokkel_egui::{Icon, SoundKind, SoundVisualizer, StarButton, TransportButton, sound_ramp};
use sokkel_egui_media::{
    Lyrics, MediaBar, MediaBarLayout, MediaVisualizerOption, RepeatMode as BarRepeat, parse_lyrics,
};

use crate::act::Act;
use crate::app::Sound;
use crate::catalog::{album_of, artist_name};
use crate::data::{FavoriteKind, Layout};
use crate::look::{LOOKS, VisualizerKind, VisualizerRamp, look_props};
use crate::lyrics::lyrics_of;
use crate::pages::Here;
use crate::queue::RepeatMode;
use crate::synth::RATE;

/// How many samples the visualizer reads at a time, as the web's analyser's `fftSize`.
const WINDOW: usize = 1024;

fn bar_repeat(mode: RepeatMode) -> BarRepeat {
    match mode {
        RepeatMode::Off => BarRepeat::Off,
        RepeatMode::All => BarRepeat::All,
        RepeatMode::One => BarRepeat::One,
    }
}

fn our_repeat(mode: BarRepeat) -> RepeatMode {
    match mode {
        BarRepeat::Off => RepeatMode::Off,
        BarRepeat::All => RepeatMode::All,
        BarRepeat::One => RepeatMode::One,
    }
}

/// The levels of the sound at `at` seconds, low to high, as the web's analyser reads them.
fn levels_at(samples: &[f32], at: f64, count: usize) -> Vec<f32> {
    let start = ((at * f64::from(RATE)) as usize).min(samples.len());
    let end = (start + WINDOW).min(samples.len());
    if end - start < WINDOW {
        return Vec::new();
    }
    band_levels(&frequency_bytes(&samples[start..end]), count, RATE as f32, MUSIC_RANGE)
}

/// The window of samples at `at` seconds, for a wave.
fn samples_at(samples: &[f32], at: f64) -> &[f32] {
    let start = ((at * f64::from(RATE)) as usize).min(samples.len());
    let end = (start + WINDOW).min(samples.len());
    &samples[start..end]
}

/// The player bar, drawn while there is a song to play.
pub fn player_bar(ui: &mut Ui, here: &Here, fullscreen: bool, sound: Option<&Sound>, acts: &mut Vec<Act>) {
    let library = here.library;
    let player = here.player;
    let Some(song) = player.current_song(library) else {
        return;
    };
    let settings = &library.settings;
    let album = album_of(&song, &library.stations);
    let radio = player.radio.is_some();
    let words = lyrics_of(&song, &library.stations).map(|text| parse_lyrics(&text));
    let starred = library.is_starred(FavoriteKind::Song, &song.id);
    let by = match &song.with {
        None => artist_name(&song.artist),
        Some(with) => format!("{} with {with}", artist_name(&song.artist)),
    };
    let props = look_props(settings.look, settings.ramp, settings.effect);
    let options: Vec<MediaVisualizerOption> = LOOKS
        .iter()
        .map(|(look, label)| MediaVisualizerOption::new(look.key(), *label))
        .collect();
    let at = player.position;
    let levels = sound.map(|sound| levels_at(&sound.samples, at, 64)).unwrap_or_default();
    let wave: Vec<f32> = sound
        .map(|sound| samples_at(&sound.samples, at).to_vec())
        .unwrap_or_default();
    let ramp = match props.ramp {
        VisualizerRamp::Primary => None,
        other => sound_ramp(other.key()),
    };
    let mut queue_open = settings.queue_open;
    let mut star = starred;

    let mut bar = MediaBar::new("player", &song.title)
        .artist(by)
        .album(&album.title)
        .artwork(album.cover)
        .playing(player.playing)
        .position(at, song.duration)
        .live(radio)
        .volume(settings.volume as f32, settings.muted)
        .steps(!radio, player.has_next())
        .shuffle(player.queue.shuffle)
        .repeat(bar_repeat(player.queue.repeat))
        .layout(match settings.layout {
            Layout::Rows => MediaBarLayout::Rows,
            Layout::Panel => MediaBarLayout::Panel,
            Layout::Stage => MediaBarLayout::Stage,
        })
        // held here always; zero is the one line, which the bar keeps as its least height
        .size(settings.bar_size.unwrap_or(0.0) as f32)
        .fullscreen(fullscreen)
        .lyrics_shown(settings.lyrics && words.is_some())
        .visualizer_options(&options, Some(settings.look.key()))
        .extra(|ui| {
            let label = if queue_open { "Hide the queue" } else { "Show the queue" };
            if ui
                .add(TransportButton::new(Icon::ListMusic, label).pressed(queue_open))
                .clicked()
            {
                queue_open = !queue_open;
            }
        })
        .fullscreen_actions(|ui| {
            ui.add(StarButton::new(&mut star));
        });
    if !radio {
        bar = bar
            .year(album.year)
            .track(player.queue.at.max(0) as usize + 1, Some(player.queue.order.len()))
            .rate(settings.rate);
    }
    if let Some(sound) = sound {
        bar = bar.waveform(&sound.waveform);
    }
    if (settings.bar_visualizer || fullscreen) && sound.is_some() {
        let (levels, wave) = (&levels, &wave);
        bar = bar.visualizer(move |painter, rect| {
            let kind = match props.kind {
                VisualizerKind::Spectrum => SoundKind::Spectrum,
                VisualizerKind::Area => SoundKind::Area,
                VisualizerKind::Wave => SoundKind::Wave,
                VisualizerKind::Ring => SoundKind::Ring,
            };
            let mut look = SoundVisualizer::new()
                .kind(kind)
                .mirror(props.mirror)
                .levels(levels)
                .samples(wave)
                .label("What is playing");
            if let Some(stops) = ramp {
                look = look.ramp(stops);
            }
            look.paint(painter, rect);
        });
    }
    let mut seek = None;
    if let Some(words) = &words {
        let seek = &mut seek;
        bar = bar.lyrics(move |ui| {
            if let Some(at) = Lyrics::new("player-lyrics", words).position(at).show(ui).seek {
                *seek = Some(at);
            }
        });
    }
    let out = bar.show(ui);

    if let Some(on) = out.playing {
        acts.push(Act::SetPlaying(on));
    }
    if let Some(at) = out.seek.or(seek) {
        acts.push(Act::Seek(at));
    }
    if let Some(by) = out.skip {
        acts.push(Act::SeekBy(by));
    }
    if out.previous {
        acts.push(Act::Previous);
    }
    if out.next {
        acts.push(Act::Next);
    }
    if let Some(on) = out.shuffle {
        acts.push(Act::Shuffle(on));
    }
    if let Some(mode) = out.repeat {
        acts.push(Act::Repeat(our_repeat(mode)));
    }
    if let Some(open) = out.fullscreen {
        acts.push(Act::Fullscreen(open));
    }
    let mut next = settings.clone();
    if let Some(volume) = out.volume {
        next.volume = f64::from(volume);
        next.muted = false;
    }
    if let Some(muted) = out.muted {
        next.muted = muted;
    }
    if let Some(rate) = out.rate {
        next.rate = rate;
    }
    if let Some(size) = out.size {
        next.bar_size = Some(f64::from(size));
    }
    if let Some(shown) = out.lyrics_shown {
        next.lyrics = shown;
    }
    if let Some(look) = out.visualizer_option.as_deref().and_then(crate::data::Look::from_key) {
        next.look = look;
    }
    next.queue_open = queue_open;
    if next != *settings {
        acts.push(Act::Settings(next));
    }
    if star != starred {
        acts.push(Act::Star(FavoriteKind::Song, song.id.clone(), star));
    }
}
