//! The videos the viewer liked, as the web's `pages/Liked.tsx` lists them: the latest liked first,
//! each one a press away from being unliked.

use crate::catalog::{Video, video_by_id};
use crate::data::{Data, RatingKind, Toast};
use crate::parts::{path_of, watch_href};

pub fn videos(data: &Data) -> Vec<&'static Video> {
    data.ratings
        .seen()
        .iter()
        .filter(|one| one.rating == RatingKind::Like)
        .filter_map(|one| video_by_id(&one.id))
        .collect()
}

/// Play the latest.
pub fn play_latest(data: &mut Data) {
    if let Some(first) = videos(data).first() {
        data.go(path_of(&watch_href(first, None)));
    }
}

/// A card's Remove from Liked videos, with a note once it is gone.
pub fn remove(data: &mut Data, id: &str) {
    data.remove_rating(id, Some(Toast::plain("Removed from Liked videos")));
}
