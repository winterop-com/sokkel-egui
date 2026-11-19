//! The chapters of a book, a podcast or a long work, after sokkel-ui-media's `ChapterList`.

use egui::{Id, Ui};

use sokkel_egui::format_time;

use crate::chapters::{MediaChapter, chapter_index, chapter_span, chapter_starts};
use crate::track_list::{Track, TrackList, TrackListResponse, TrackListVariant, TrackListWords, TrackProgress};

/// What the chapter list was asked this frame.
#[derive(Clone, Debug)]
pub struct ChapterListResponse {
    pub list: TrackListResponse,
    /// A chapter was pressed: seek to its start, in seconds from the start of the whole (the web's
    /// `onSeek`).
    pub seek: Option<f64>,
}

/// The chapters of a book, a podcast or a long work, in order, after sokkel-ui-media's
/// `ChapterList`: each one's number, its title over where it starts in the code face, and its
/// length at the end, the last one's running to `duration`. It is a [`TrackList`]: the chapter
/// playing at `position` is marked as the playing row is, and kept in view (`follow`); pressing a
/// chapter asks to seek to its start (`seekable`). `progress` shows how far the chapter playing has
/// played, as the time left or a line along its foot.
///
/// The chapters and the position count on the whole, the same as [`MediaBar`](crate::MediaBar)'s
/// `chapters` (which [`chapter_starts`] makes).
#[must_use = "show it with `.show(ui)`"]
pub struct ChapterList<'a> {
    id: Id,
    chapters: &'a [MediaChapter],
    duration: f64,
    position: Option<f64>,
    playing: bool,
    seekable: bool,
    progress: Option<TrackProgress>,
    follow: bool,
    variant: TrackListVariant,
    words: TrackListWords,
}

impl<'a> ChapterList<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, chapters: &'a [MediaChapter], duration: f64) -> Self {
        Self {
            id: Id::unique(id_salt),
            chapters,
            duration,
            position: None,
            playing: false,
            seekable: true,
            progress: None,
            follow: true,
            variant: TrackListVariant::Boxed,
            words: TrackListWords::default(),
        }
    }

    /// Seconds from the start of the whole; the chapter it falls in is marked.
    pub fn position(mut self, seconds: f64) -> Self {
        self.position = Some(seconds);
        self
    }

    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// Whether a press on a chapter asks to seek to it; on by default.
    pub fn seekable(mut self, seekable: bool) -> Self {
        self.seekable = seekable;
        self
    }

    pub fn progress(mut self, progress: TrackProgress) -> Self {
        self.progress = Some(progress);
        self
    }

    /// Whether the chapter playing is kept in view as it changes; on by default.
    pub fn follow(mut self, follow: bool) -> Self {
        self.follow = follow;
        self
    }

    pub fn variant(mut self, variant: TrackListVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn words(mut self, words: TrackListWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ChapterListResponse {
        let starts = chapter_starts(self.chapters);
        let index = self.position.map_or(-1, |at| chapter_index(&starts, at));
        let tracks: Vec<Track> = self
            .chapters
            .iter()
            .enumerate()
            .map(|(at, chapter)| {
                let (start, end) = chapter_span(&starts, at, self.duration);
                Track::new(at.to_string(), &chapter.title, end - start)
                    .number(at + 1)
                    .subtitle(format_time(chapter.start))
                    .mono_subtitle(true)
            })
            .collect();
        let current = (index >= 0).then(|| index.to_string());
        let mut list = TrackList::new(self.id, &tracks)
            .current(current.as_deref())
            .playing(self.playing)
            .follow(self.follow)
            .playable(self.seekable)
            .variant(self.variant)
            .words(self.words);
        if let (Some(at), true) = (self.position, index >= 0) {
            list = list.position(at - starts[index as usize]);
        }
        if let Some(progress) = self.progress {
            list = list.progress(progress);
        }
        let out = list.show(ui);
        let seek = out.played.and_then(|i| starts.get(i).copied());
        ChapterListResponse { list: out, seek }
    }
}
