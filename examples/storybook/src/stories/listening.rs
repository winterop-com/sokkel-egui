//! The parts of a long listen, a podcast or a book: its chapters, its transcript, a sleep timer in
//! the player, and a button that reads a text aloud.

use eframe::egui::{Id, Ui, vec2};
use sokkel_egui::{SpeakButton, SpeakState, TextRole, tokens};
use sokkel_egui_media::{
    ArtworkCover, ArtworkHue as Hue, ArtworkMotif as Motif, ChapterList, MediaBar, MediaChapter, SleepTimer,
    SleepTimerMenu, TrackListVariant, TrackProgress, Transcript, TranscriptLine, chapter_starts,
};

use super::{State, section};

/// A value the story holds between frames, in egui's memory.
fn held<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, from: impl FnOnce() -> T) -> T {
    ui.data(|d| d.get_temp(Id::unique(("listening-story", key))))
        .unwrap_or_else(from)
}

fn hold<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, value: T) {
    ui.data_mut(|d| d.insert_temp(Id::unique(("listening-story", key)), value));
}

const COVER: ArtworkCover = ArtworkCover {
    motif: Motif::Wave,
    from: Hue::Blue,
    to: Hue::Teal,
};

/// A short book in one file: its chapters and where each starts.
fn almanac() -> Vec<MediaChapter> {
    [
        ("Low Water", 0.0),
        ("The Harbor Wall", 2_140.0),
        ("Juniper Point", 4_310.0),
        ("Larkspur Fields", 7_020.0),
        ("Northern Coast", 9_480.0),
        ("The Long Bay", 12_600.0),
    ]
    .into_iter()
    .map(|(title, start)| MediaChapter {
        title: title.into(),
        start,
    })
    .collect()
}

const ALMANAC_LENGTH: f64 = 15_900.0;

/// An episode's chapters, and its length.
fn episode_chapters() -> Vec<MediaChapter> {
    [
        ("The harbor at low water", 0.0),
        ("Rock pools on the north wall", 29.0),
        ("What the boats bring in", 53.0),
    ]
    .into_iter()
    .map(|(title, start)| MediaChapter {
        title: title.into(),
        start,
    })
    .collect()
}

const EPISODE_LENGTH: f64 = 73.0;

/// The episode's words as they are said, by its two hosts.
fn episode_lines() -> Vec<TranscriptLine> {
    [
        (
            2.0,
            "ada",
            "Welcome to Low Tide Hours. I am ada, here with grace, and this time: the harbor at low water.",
        ),
        (
            8.0,
            "ada",
            "We walk out across the empty harbor and count what the tide left behind.",
        ),
        (14.0, "grace", "Nobody quite agrees on how long it has been like this."),
        (
            18.0,
            "ada",
            "The gulls were there first, as always, and they let us know it.",
        ),
        (
            26.0,
            "grace",
            "There was a fisherman there who told us the whole story in five minutes.",
        ),
        (
            31.0,
            "grace",
            "The rock pools on the north wall hold more than you would think.",
        ),
        (38.0, "ada", "Crabs, mostly, and a starfish that would not be counted."),
        (46.0, "linus", "I brought a bucket. Nobody asked me to bring a bucket."),
        (
            55.0,
            "ada",
            "Then the boats came in, and with them the whole morning's catch.",
        ),
        (
            63.0,
            "grace",
            "That is all for this week. The tide comes back on Thursday.",
        ),
    ]
    .into_iter()
    .map(|(start, speaker, text)| TranscriptLine {
        start,
        text: text.into(),
        speaker: Some(speaker.into()),
    })
    .collect()
}

pub fn chapter_list(_: &mut State, ui: &mut Ui) {
    let chapters = almanac();
    section(
        ui,
        "A short book 12:34 into the third chapter, playing: each start under its title, the time left in the one playing",
    );
    let at = held(ui, "chapters-at", || 4_310.0 + 754.0);
    let out = ChapterList::new("story-chapters", &chapters, ALMANAC_LENGTH)
        .position(at)
        .playing(true)
        .progress(TrackProgress::Left)
        .follow(false)
        .show(ui);
    if let Some(seek) = out.seek {
        hold(ui, "chapters-at", seek);
    }
    section(
        ui,
        "Plain, inside a section's box: the first chapter paused at its start, a line along its foot",
    );
    let episode = episode_chapters();
    let _ = ChapterList::new("story-chapters-plain", &episode, EPISODE_LENGTH)
        .position(12.0)
        .progress(TrackProgress::Bar)
        .variant(TrackListVariant::Plain)
        .follow(false)
        .show(ui);
}

pub fn transcript(_: &mut State, ui: &mut Ui) {
    let lines = episode_lines();
    let chapters = episode_chapters();
    let starts = chapter_starts(&chapters);
    let at = held(ui, "transcript-at", || 20.0);
    let width = ui.available_width();
    let height = ui.available_height() - 120.0;
    let mut seek = None;
    ui.horizontal_top(|ui| {
        let column = (width - 24.0) / 2.0;
        ui.allocate_ui(vec2(column, height), |ui| {
            ui.vertical(|ui| {
                section(ui, "Chapters beside it, 0:20 in");
                let out = ChapterList::new("story-transcript-chapters", &chapters, EPISODE_LENGTH)
                    .position(at)
                    .progress(TrackProgress::Left)
                    .follow(false)
                    .show(ui);
                seek = seek.or(out.seek);
            });
        });
        ui.add_space(24.0 - ui.spacing().item_spacing.x);
        ui.allocate_ui(vec2(column, height), |ui| {
            ui.vertical(|ui| {
                section(ui, "The line said inked, the speaker over each change of voice");
                let out = Transcript::new("story-transcript", &lines).position(at).show(ui);
                seek = seek.or(out.seek);
            });
        });
    });
    let out = MediaBar::new("story-transcript-bar", "The harbor at low water")
        .artist("ada and grace")
        .album("Low Tide Hours")
        .artwork(COVER)
        .position(at, EPISODE_LENGTH)
        .chapters(&starts)
        .skip(15.0)
        .rate(1.0)
        .volume(0.7, false)
        .show(ui);
    if let Some(to) = seek.or(out.seek) {
        hold(ui, "transcript-at", to);
    }
}

pub fn sleep_timer(_: &mut State, ui: &mut Ui) {
    let starts = chapter_starts(&episode_chapters());
    for (n, (title, timer, remaining)) in [
        ("No timer: the moon quiet at the bar's end", None, None),
        (
            "Fifteen minutes with 12:34 left: the moon in the accent, the time beside it",
            Some(SleepTimer::Minutes(15.0)),
            Some(754.0),
        ),
        (
            "To the end of the chapter: the moon in the accent alone",
            Some(SleepTimer::Chapter),
            None,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        section(ui, title);
        let key = format!("sleep-{n}");
        let mut shown: Option<SleepTimer> = held(ui, &key, || timer);
        let _ = MediaBar::new(("story-sleep", n), "The harbor at low water")
            .artist("ada and grace")
            .album("Low Tide Hours")
            .artwork(COVER)
            .position(36.0, EPISODE_LENGTH)
            .chapters(&starts)
            .skip(15.0)
            .rate(1.0)
            .volume(0.7, false)
            .extra(|ui| {
                let out = SleepTimerMenu::new(shown).remaining(remaining).chapters(true).show(ui);
                if let Some(asked) = out.asked {
                    shown = asked;
                }
            })
            .show(ui);
        hold(ui, &key, shown);
    }
}

pub fn speak_button(_: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    for (title, labeled) in [
        ("As an icon, for a hover bar", false),
        ("With its words, for a row of actions", true),
    ] {
        section(ui, title);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            for (state, name) in [
                (SpeakState::Idle, "idle"),
                (SpeakState::Loading, "loading"),
                (SpeakState::Playing, "playing"),
                (SpeakState::Error, "error"),
            ] {
                ui.vertical(|ui| {
                    ui.label(TextRole::Help.rich(&t, name).color(t.color("muted-foreground")));
                    ui.add(SpeakButton::new(state).labeled(labeled));
                });
            }
        });
    }
}
