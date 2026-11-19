//! sokkel-egui's music and video app parts, the counterpart of `@winterop-com/sokkel-ui-media`: a
//! record's cover drawn from a stable seed (`Artwork`), the card a grid of albums, films or
//! episodes is made of (`MediaCard`, `MediaGrid`), the list of songs (`TrackList`, reordered or
//! dragged elsewhere), the player along the foot of the screen (`MediaBar`), timed lyrics (`Lyrics`
//! over `lrc`), and the queue's arithmetic (`queue`).

pub mod artwork;
pub mod bar_growth;
pub mod lrc;
pub mod lyrics;
pub mod media_bar;
pub mod media_card;
pub mod queue;
pub mod track_list;

pub use artwork::{
    ARTWORK_HUES, ARTWORK_MOTIFS, Artwork, ArtworkCover, ArtworkHue, ArtworkMotif, CoverInk, CoverShape, cover_for,
    cover_shapes, seed_hash,
};
pub use lrc::{LyricLine, ParsedLyrics, lyric_line_at, parse_lyrics};
pub use lyrics::{Lyrics, LyricsResponse, LyricsWords};
pub use media_bar::{
    MEDIA_SKIP, MediaBar, MediaBarLayout, MediaBarResponse, MediaBarWords, MediaSkip, MediaVisualizerOption,
    rows_height_for,
};
pub use media_card::{MediaCard, MediaCardLayout, MediaCardResponse, MediaCardWords, MediaGrid, MediaShape};
pub use queue::{
    Queue, QueueEntry, QueueOptions, QueueWhere, REPEAT_MODES, RepeatMode, Seed, ShuffleOptions, seeded_random,
    shuffled_order,
};
pub use track_list::{
    DraggableTrackList, DraggableTrackListWords, ReorderableTrackList, ReorderableTrackListWords, TRACK_WINDOW_FROM,
    Track, TrackList, TrackListResponse, TrackListVariant, TrackListWords, TrackProgress, disc_heads, track_range,
    track_tops,
};
