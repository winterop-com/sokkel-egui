//! sokkel-egui's music and video app parts, the counterpart of `@winterop-com/sokkel-ui-media`: a
//! record's cover drawn from a stable seed (`Artwork`), the card a grid of albums, films or
//! episodes is made of (`MediaCard`, `MediaGrid`), the list of songs (`TrackList`, reordered or
//! dragged elsewhere), the player along the foot of the screen (`MediaBar`), timed lyrics (`Lyrics`
//! over `lrc`), and the queue's arithmetic (`queue`). For a long listen, a podcast or a book: the
//! chapters (`chapters`, `ChapterList`), a transcript that follows what is said (`Transcript`) and
//! a sleep timer (`SleepTimerMenu`). For a video site's watch page: the layout (`WatchLayout`),
//! the channel under the video (`ChannelRow`), what the viewer does with it (`MediaActions`), what
//! plays next as it ends (`UpNext`), and how it is playing (`VideoStatsOverlay`).

pub mod artwork;
pub mod bar_growth;
pub mod channel_row;
pub mod chapter_list;
pub mod chapters;
pub mod lrc;
pub mod lyrics;
pub mod media_actions;
pub mod media_bar;
pub mod media_card;
pub mod queue;
pub mod sleep_timer;
pub mod track_list;
pub mod transcript;
pub mod up_next;
pub mod video_stats_overlay;
pub mod watch_layout;

pub use artwork::{
    ARTWORK_HUES, ARTWORK_MOTIFS, Artwork, ArtworkCover, ArtworkHue, ArtworkMotif, CoverInk, CoverShape, cover_for,
    cover_shapes, seed_hash,
};
pub use channel_row::{ChannelRow, ChannelRowResponse, ChannelRowWords};
pub use chapter_list::{ChapterList, ChapterListResponse};
pub use chapters::{
    MediaChapter, RESTART_AFTER, chapter_index, chapter_span, chapter_starts, next_chapter_start,
    previous_chapter_start, seek_point, waveform_span,
};
pub use lrc::{LyricLine, ParsedLyrics, lyric_line_at, parse_lyrics};
pub use lyrics::{Lyrics, LyricsResponse, LyricsWords};
pub use media_actions::{MediaAction, MediaActions, MediaActionsResponse, MediaActionsWords, MediaRating};
pub use media_bar::{
    MEDIA_SKIP, MediaBar, MediaBarLayout, MediaBarResponse, MediaBarWords, MediaSkip, MediaVisualizerOption,
    rows_height_for,
};
pub use media_card::{MediaCard, MediaCardLayout, MediaCardResponse, MediaCardWords, MediaGrid, MediaShape};
pub use queue::{
    Queue, QueueEntry, QueueOptions, QueueWhere, REPEAT_MODES, RepeatMode, Seed, ShuffleOptions, seeded_random,
    shuffled_order,
};
pub use sleep_timer::{
    SLEEP_MINUTES, SleepTimer, SleepTimerMenu, SleepTimerResponse, SleepTimerWords, chapter_timer_step, played_on,
};
pub use track_list::{
    DraggableTrackList, DraggableTrackListWords, ReorderableTrackList, ReorderableTrackListWords, TRACK_WINDOW_FROM,
    Track, TrackList, TrackListResponse, TrackListVariant, TrackListWords, TrackProgress, disc_heads, track_range,
    track_tops,
};
pub use transcript::{Transcript, TranscriptLine, TranscriptResponse, TranscriptWords, transcript_line_at};
pub use up_next::{UpNext, UpNextResponse, UpNextWords};
pub use video_stats_overlay::{
    VideoStats, VideoStatsOverlay, VideoStatsOverlayWords, VideoStatsTone, format_bitrate, format_resolution,
};
pub use watch_layout::WatchLayout;
