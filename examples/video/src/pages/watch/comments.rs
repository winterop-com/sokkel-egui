//! What people say under a video, as the web's `pages/watch/Comments.tsx` keeps it: added, replied
//! to, liked and disliked, reacted to, sorted, and deleted when one's own or reported when not.

use serde::Serialize;

use crate::catalog::{Channel, Person, VIEWER, Video};
use crate::data::{Comment, Data, Toast};
use crate::format::{plural, short};
use crate::js::locale_date_time;
use crate::reactions::toggle_reaction;

/// How the comments are sorted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Order {
    #[default]
    Top,
    New,
}

pub const ORDERS: [(Order, &str); 2] = [(Order::Top, "Top comments"), (Order::New, "Newest first")];

/// A comment as the thread draws it, with the replies to it under it, oldest first.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Thread {
    pub id: String,
    pub author: Person,
    pub color: &'static str,
    /// When it was said, in milliseconds, written relative to now by the thread.
    pub at: i64,
    /// When it was said, written out, as the time's title.
    pub time_title: String,
    pub body: String,
    pub likes: Option<String>,
    pub liked: bool,
    pub disliked: bool,
    pub pinned: bool,
    /// Said by the channel's owner.
    pub creator: bool,
    pub reactions: Vec<(String, String, i64, bool)>,
    pub replies: Vec<Thread>,
    /// The one entry of its menu: Delete for the viewer's own, Report for another's.
    pub menu: &'static str,
}

/// The comments shown under a video: those not reported.
pub fn here<'a>(data: &'a Data, video: &Video) -> Vec<&'a Comment> {
    data.comments
        .seen()
        .iter()
        .filter(|one| one.video == video.id && !one.reported)
        .collect()
}

pub fn thread_of(comment: &Comment, all: &[&Comment], owner: Person) -> Thread {
    let mut replies: Vec<&&Comment> = all
        .iter()
        .filter(|one| one.parent.as_deref() == Some(comment.id.as_str()))
        .collect();
    replies.sort_by_key(|one| one.at);
    Thread {
        id: comment.id.clone(),
        author: comment.author,
        color: comment.author.color(),
        at: comment.at,
        time_title: locale_date_time(comment.at),
        body: comment.text.clone(),
        likes: (comment.likes > 0).then(|| short(comment.likes as f64)),
        liked: comment.liked,
        disliked: comment.disliked,
        pinned: comment.pinned,
        creator: comment.author == owner,
        reactions: comment
            .reactions
            .iter()
            .flatten()
            .map(|one| (one.id.clone(), one.label.clone(), one.count, one.mine))
            .collect(),
        replies: replies.iter().map(|reply| thread_of(reply, all, owner)).collect(),
        menu: if comment.author == VIEWER { "Delete" } else { "Report" },
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CommentsView {
    pub title: String,
    pub threads: Vec<Thread>,
}

pub fn view(data: &Data, video: &Video, channel: &Channel, order: Order) -> CommentsView {
    let here = here(data, video);
    let mut top: Vec<&Comment> = here.iter().copied().filter(|one| one.parent.is_none()).collect();
    match order {
        Order::Top => top.sort_by(|left, right| {
            right
                .pinned
                .cmp(&left.pinned)
                .then(right.likes.cmp(&left.likes))
                .then(right.at.cmp(&left.at))
        }),
        Order::New => top.sort_by_key(|one| std::cmp::Reverse(one.at)),
    }
    CommentsView {
        title: plural(here.len() as f64, "comment", None),
        threads: top.iter().map(|one| thread_of(one, &here, channel.owner)).collect(),
    }
}

/// Every comment under `id` the page shows, itself included, for taking a comment away with its
/// replies.
pub fn branch(id: &str, all: &[&Comment]) -> Vec<String> {
    let mut out = vec![id.to_owned()];
    for one in all.iter().filter(|one| one.parent.as_deref() == Some(id)) {
        out.extend(branch(&one.id, all));
    }
    out
}

/// The composer's send, or a reply's.
pub fn add(data: &mut Data, video: &Video, text: &str, parent: Option<&str>) {
    data.add_comment(video.id, text, parent);
}

/// Like (`liked`) or Dislike on a comment the page shows: on, or off again; one turns the other off.
pub fn rate(data: &mut Data, video: &Video, id: &str, liked_field: bool) {
    let Some(comment) = here(data, video).into_iter().find(|one| one.id == id).cloned() else {
        return;
    };
    let on = !if liked_field { comment.liked } else { comment.disliked };
    let liked = if liked_field {
        on
    } else if on {
        false
    } else {
        comment.liked
    };
    let disliked = if liked_field {
        if on { false } else { comment.disliked }
    } else {
        on
    };
    let likes = (comment.likes + i64::from(liked) - i64::from(comment.liked)).max(0);
    data.update_comment(id, |row| {
        row.liked = liked;
        row.disliked = disliked;
        row.likes = likes;
    });
}

/// A reaction given or taken back.
pub fn react(data: &mut Data, video: &Video, id: &str, emoji: &str, name: &str) {
    let Some(comment) = here(data, video).into_iter().find(|one| one.id == id).cloned() else {
        return;
    };
    let next = toggle_reaction(comment.reactions.as_deref().unwrap_or_default(), emoji, name);
    data.update_comment(id, |row| row.reactions = Some(next));
}

/// Delete: the comment and every reply under it, and a note at once.
pub fn remove(data: &mut Data, video: &Video, id: &str) {
    let all = branch(id, &here(data, video));
    for one in all {
        data.remove_comment(&one);
    }
    data.toast(Toast::plain("Comment deleted"));
}

/// Report: hidden for the viewer, with Undo.
pub fn report(data: &mut Data, id: &str) {
    data.update_comment(id, |row| row.reported = true);
    data.toast(Toast {
        action: Some("Undo".to_owned()),
        ..Toast::plain("Reported, and hidden for you")
    });
}

/// Report's Undo.
pub fn unreport(data: &mut Data, id: &str) {
    data.update_comment(id, |row| row.reported = false);
}
