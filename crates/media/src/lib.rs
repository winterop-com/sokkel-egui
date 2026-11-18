//! sokkel-egui's music and video app parts, the counterpart of `@winterop-com/sokkel-ui-media`: a
//! record's cover drawn from a stable seed (`Artwork`), and the card a grid of albums, films or
//! episodes is made of (`MediaCard`, `MediaGrid`).

pub mod artwork;
pub mod media_card;

pub use artwork::{
    ARTWORK_HUES, ARTWORK_MOTIFS, Artwork, ArtworkCover, ArtworkHue, ArtworkMotif, CoverInk, CoverShape, cover_for,
    cover_shapes, seed_hash,
};
pub use media_card::{MediaCard, MediaCardLayout, MediaCardResponse, MediaCardWords, MediaGrid, MediaShape};
