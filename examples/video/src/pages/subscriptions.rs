//! The latest from the channels the viewer follows, as the web's `pages/Subscriptions.tsx` shows
//! it: newest first, this week's apart from the earlier ones, narrowed to one channel by its chip.

use serde::Serialize;

use crate::catalog::{CHANNELS, VIDEOS, Video, channel_by_id, videos_of};
use crate::data::Data;
use crate::format::plural;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SubscriptionsView {
    pub loading: bool,
    pub count: String,
    /// The chip shown chosen: `all`, or a channel followed.
    pub chosen: String,
    /// The chips: All, then each channel followed, as value and words.
    pub chips: Vec<(String, String)>,
    pub week: Vec<&'static str>,
    pub earlier: Vec<&'static str>,
    /// Whether it says the viewer follows no channel yet.
    pub none_followed: bool,
    /// Whether it says the chosen channel has put up nothing.
    pub no_videos: bool,
}

/// What Subscriptions shows with `chosen` as its chip.
pub fn view(data: &Data, chosen: &str) -> SubscriptionsView {
    let rows = data.follows.seen();
    let following = |id: &str| rows.iter().any(|one| one.id == id);
    let channels: Vec<_> = rows.iter().filter_map(|one| channel_by_id(&one.id)).collect();
    let mut shown: Vec<&Video> = VIDEOS
        .iter()
        .filter(|video| following(video.channel) && (chosen == "all" || video.channel == chosen))
        .collect();
    shown.sort_by(|left, right| left.days.total_cmp(&right.days));
    let ids = |keep: &dyn Fn(&Video) -> bool| shown.iter().filter(|one| keep(one)).map(|one| one.id).collect();
    SubscriptionsView {
        loading: !data.follows.loaded(),
        count: plural(channels.len() as f64, "channel", None),
        chosen: if following(chosen) {
            chosen.to_owned()
        } else {
            "all".to_owned()
        },
        chips: std::iter::once(("all".to_owned(), "All".to_owned()))
            .chain(channels.iter().map(|one| (one.id.to_owned(), one.name.to_owned())))
            .collect(),
        week: ids(&|video| video.days < 7.0),
        earlier: ids(&|video| video.days >= 7.0),
        none_followed: channels.is_empty(),
        no_videos: !channels.is_empty() && shown.is_empty(),
    }
}

/// Manage's rows: every channel, how many videos it has, and whether it is followed.
pub fn manage(data: &Data) -> Vec<(&'static str, String, bool)> {
    CHANNELS
        .iter()
        .map(|channel| {
            (
                channel.id,
                plural(videos_of(channel).len() as f64, "video", None),
                data.follows.seen_get(channel.id).is_some(),
            )
        })
        .collect()
}
