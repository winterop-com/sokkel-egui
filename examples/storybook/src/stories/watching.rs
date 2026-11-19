//! The parts of a video site's watch page: the player, what plays next as a video ends, the stats
//! over the picture, the channel under the video with what the viewer does with it, the layout that
//! holds them, a description held to two lines, the comments, and a rail collapsed to its icons.

use eframe::egui::{self, Id, Sense, Ui, vec2};
use sokkel_egui::captions::CAPTION_STYLE;
use sokkel_egui::{
    CommentComposer, CommentThread, Icon, MenuItem, MetaLine, Rail, RailEntry, RailGroup, Reaction, ShowMore, TextRole,
    ThreadComment, VideoPlayer, VideoQuality, tokens,
};
use sokkel_egui_media::{
    Artwork, ChannelRow, MediaAction, MediaActions, MediaRating, UpNext, VideoStats, VideoStatsOverlay, VideoStatsTone,
    WatchLayout,
};

use super::{State, section};

/// A value the story holds between frames, in egui's memory.
fn held<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, from: impl FnOnce() -> T) -> T {
    ui.data(|d| d.get_temp(Id::unique(("watching-story", key))))
        .unwrap_or_else(from)
}

fn hold<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, value: T) {
    ui.data_mut(|d| d.insert_temp(Id::unique(("watching-story", key)), value));
}

/// A picture for the player: a drawn cover, wide.
fn picture(seed: &'static str) -> impl Fn(&egui::Painter, egui::Rect) {
    move |painter, rect| Artwork::new(seed).wide(true).paint(painter, rect)
}

fn qualities() -> Vec<VideoQuality> {
    [
        ("auto", "Auto", "480p"),
        ("720", "720p", "1.6 Mbit/s"),
        ("480", "480p", "850 kbit/s"),
    ]
    .into_iter()
    .map(|(value, label, detail)| VideoQuality {
        value: value.into(),
        label: label.into(),
        detail: Some(detail.into()),
    })
    .collect()
}

pub fn video_player(_: &mut State, ui: &mut Ui) {
    let width = 480.0_f32.min(ui.available_width());
    let tracks = vec!["English".to_owned()];
    let offered = qualities();
    section(ui, "Paused before it starts: the poster, the controls shown");
    ui.allocate_ui(vec2(width, width * 9.0 / 16.0), |ui| {
        ui.set_width(width);
        let _ = VideoPlayer::new("story-player-poster")
            .picture(picture("nightly-export"))
            .title("How a harbor wakes up")
            .position(0.0, 38.0)
            .qualities(&offered, "auto")
            .tracks(&tracks, Some(0))
            .show(ui);
    });
    section(ui, "Playing, with a cue of the captions over the picture");
    ui.allocate_ui(vec2(width, width * 9.0 / 16.0), |ui| {
        ui.set_width(width);
        let _ = VideoPlayer::new("story-player-cue")
            .picture(picture("sync-inventory"))
            .title("Rain on a harbor city at night")
            .position(14.0, 36.0)
            .playing(false)
            .started(true)
            .volume(0.6, false)
            .rate(1.5)
            .qualities(&offered, "720")
            .tracks(&tracks, Some(0))
            .cue(Some("The rain comes in off the sea at dusk.".into()), CAPTION_STYLE)
            .show(ui);
    });
}

pub fn up_next(_: &mut State, ui: &mut Ui) {
    let width = 480.0_f32.min(ui.available_width());
    for (n, (title, countdown)) in [
        ("Counting down: it plays by itself at zero", Some(6.0)),
        ("Autoplay off: it waits for Play now", None),
    ]
    .into_iter()
    .enumerate()
    {
        section(ui, title);
        ui.allocate_ui(vec2(width, width * 9.0 / 16.0), |ui| {
            ui.set_width(width);
            let _ = VideoPlayer::new(("story-up-next", n))
                .picture(picture("harbor"))
                .title("How a harbor wakes up")
                .position(38.0, 38.0)
                .started(true)
                .cover(|ui| {
                    let _ = UpNext::new("Keeping a small boat afloat")
                        .subtitle("Harbor Workshop")
                        .placeholder_seed("boat-afloat")
                        .countdown(countdown)
                        .cancellable(true)
                        .show(ui);
                })
                .show(ui);
        });
    }
}

pub fn video_stats(_: &mut State, ui: &mut Ui) {
    let width = 480.0_f32.min(ui.available_width());
    let rows = vec![
        ("Quality".to_owned(), "480p, Auto".to_owned()),
        ("Codecs".to_owned(), "VP8, Opus".to_owned()),
    ];
    section(
        ui,
        "Over the picture: the player's facts, then the app's, with its verdict",
    );
    ui.allocate_ui(vec2(width, width * 9.0 / 16.0), |ui| {
        ui.set_width(width);
        let stats = VideoStats {
            resolution: Some((854, 480)),
            bitrate: Some(850_000.0),
            bandwidth: Some(24_300_000.0),
            buffer_ahead: 22.4,
            dropped_frames: Some(0),
            total_frames: Some(312),
            level: Some(2),
            levels: Some(4),
        };
        let rows = &rows;
        let _ = VideoPlayer::new("story-stats")
            .picture(picture("aurora"))
            .title("Northern lights over a still fjord")
            .position(13.0, 48.0)
            .started(true)
            .overlay(move |ui| {
                VideoStatsOverlay::new(Some(stats))
                    .headline("Playing smoothly", VideoStatsTone::Good)
                    .rows(rows)
                    .closable(true)
                    .show(ui);
            })
            .show(ui);
    });
    section(ui, "Before the first reading");
    let _ = VideoStatsOverlay::new(None).show(ui);
}

/// The channel's actions as the watch page offers them, with the rating held by the story.
fn actions(ui: &mut Ui, key: &str) {
    let mut rating: Option<u8> = held(ui, key, || Some(0));
    let stats = [MenuItem::action("Stats").icon(Icon::Activity)];
    let out = MediaActions::new()
        .rating(
            match rating {
                Some(0) => Some(MediaRating::Like),
                Some(_) => Some(MediaRating::Dislike),
                None => None,
            },
            Some(2_401.0),
        )
        .action(MediaAction::new("Share").icon(Icon::Share2))
        .action(MediaAction::new("Saved").icon(Icon::Bookmark).pressed(true))
        .action(MediaAction::new("Download").icon(Icon::Download))
        .menu(&stats)
        .show(ui);
    if let Some(next) = out.rating {
        rating = next.map(|one| u8::from(one == MediaRating::Dislike));
    }
    hold(ui, key, rating);
}

pub fn channel_row(_: &mut State, ui: &mut Ui) {
    section(
        ui,
        "Under a video: the channel, Follow, and the actions at the line's end",
    );
    let mut following: bool = held(ui, "following", || true);
    let out = ChannelRow::new("Harbor Workshop")
        .color("teal")
        .followers(48_201.0)
        .following(following)
        .linked(true)
        .actions(|ui| actions(ui, "row-rating"))
        .show(ui);
    if let Some(on) = out.following {
        following = on;
    }
    hold(ui, "following", following);
    section(
        ui,
        "Narrow: the actions on a line of their own, those without room in More",
    );
    ui.allocate_ui(vec2(300.0, 120.0), |ui| {
        ui.set_width(300.0);
        let _ = ChannelRow::new("Night Watch")
            .color("green")
            .followers(1.0)
            .following(false)
            .actions(|ui| actions(ui, "narrow-rating"))
            .show(ui);
    });
    section(ui, "Heading the channel's own page");
    let _ = ChannelRow::new("Tidevann Tides")
        .color("blue")
        .large(true)
        .detail("@tidevann, 312K followers, 3 videos")
        .following(false)
        .show(ui);
}

pub fn watch_layout(_: &mut State, ui: &mut Ui) {
    let t = tokens(ui.ctx());
    let block = |ui: &mut Ui, label: &str, height: f32| {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        ui.painter().rect_filled(rect, t.radius("surface"), t.color("muted"));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            TextRole::Help.font(&t),
            t.color("muted-foreground"),
        );
    };
    for (title, theater) in [
        ("Wide: what plays next beside the player and what is under it", false),
        (
            "Theater: the player across both columns, what plays next under it",
            true,
        ),
    ] {
        section(ui, title);
        WatchLayout::new(|ui| {
            let width = ui.available_width();
            block(ui, "Player", width * 9.0 / 16.0);
        })
        .theater(theater, 260.0)
        .body(|ui| block(ui, "Title, channel, description, comments", 96.0))
        .aside(|ui| block(ui, "Up next", 200.0))
        .show(ui);
    }
}

pub fn show_more(_: &mut State, ui: &mut Ui) {
    let width = 560.0_f32.min(ui.available_width());
    section(ui, "A description under a video, in a well, held to two lines");
    ui.allocate_ui(vec2(width, 160.0), |ui| {
        ui.set_width(width);
        ShowMore::new("story-show-more", "From the first engine to the last boat out: an hour on the quay of a small harbor town, shortened to under a minute. The boats leave in the same order every morning, and nobody writes it down. The gulls know the order better than the harbor master does, and wait at the end of the wall for the first boat back.")
            .well(true)
            .head(MetaLine::new(["31.2K views", "9 days ago", "Harbors"]).sm().strong())
            .show(ui);
    });
    section(ui, "Short enough: no Show more");
    ui.allocate_ui(vec2(width, 80.0), |ui| {
        ui.set_width(width);
        ShowMore::new("story-show-short", "The tide table for the harbor wall, read aloud.").show(ui);
    });
}

pub fn comment_thread(_: &mut State, ui: &mut Ui) {
    let width = 480.0_f32.min(ui.available_width());
    let comments = vec![
        ThreadComment {
            id: "owner".into(),
            author: "grace".into(),
            color: Some("violet".into()),
            time: "9 days ago".into(),
            body: "Thank you for watching. The next one is already in the works.".into(),
            likes: Some("157".into()),
            pinned: true,
            creator: true,
            reactions: vec![Reaction {
                id: "+1".into(),
                label: "plus one".into(),
                count: 8,
                mine: false,
            }],
            ..Default::default()
        },
        ThreadComment {
            id: "c0".into(),
            author: "linus".into(),
            color: Some("green".into()),
            time: "8 days ago".into(),
            body: "The second boat always leaves a minute late. Every single morning.".into(),
            likes: Some("81".into()),
            liked: true,
            replies: vec![ThreadComment {
                id: "c0-r".into(),
                author: "grace".into(),
                color: Some("violet".into()),
                time: "8 days ago".into(),
                body: "It waits for the baker.".into(),
                likes: Some("12".into()),
                creator: true,
                ..Default::default()
            }],
            ..Default::default()
        },
    ];
    section(
        ui,
        "The composer, then the comments: pinned, by the creator, liked, with a reply folded",
    );
    ui.allocate_ui(vec2(width, 420.0), |ui| {
        ui.set_width(width);
        let _ = CommentComposer::new("story-composer", "ada").color("teal").show(ui);
        ui.add_space(16.0);
        let _ = CommentThread::new("story-thread", &comments)
            .viewer("ada", Some("teal".into()))
            .pinned_by("grace")
            .reactions(true)
            .menu(|_| vec![MenuItem::action("Report").icon(Icon::Flag)])
            .show(ui);
    });
}

pub fn rail_collapsed(_: &mut State, ui: &mut Ui) {
    let groups = vec![
        RailGroup::new(
            None,
            [
                RailEntry::new("home", "Home", Icon::House),
                RailEntry::new("subscriptions", "Subscriptions", Icon::Users),
            ],
        ),
        RailGroup::new(
            Some("You"),
            [
                RailEntry::new("later", "Watch later", Icon::Clock).count(3),
                RailEntry::new("liked", "Liked videos", Icon::ThumbsUp).count(12),
                RailEntry::new("downloads", "Downloads", Icon::Download),
            ],
        ),
        RailGroup::new(
            Some("Following"),
            [
                RailEntry::new("harbor", "Harbor Workshop", Icon::User).dot("kind-teal"),
                RailEntry::new("night", "Night Watch", Icon::User).dot("kind-green"),
            ],
        ),
    ];
    let mut place: String = held(ui, "rail-place", || "home".to_owned());
    section(
        ui,
        "Collapsed: the icons, the counts at their corners, the channels' dots; and open",
    );
    ui.horizontal_top(|ui| {
        for collapsed in [true, false] {
            let (rect, _) = ui.allocate_exact_size(vec2(Rail::width(collapsed), 420.0), Sense::hover());
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                ui.add(Rail::new("Video", &groups, &mut place).collapsed(collapsed));
            });
            ui.add_space(24.0);
        }
    });
    hold(ui, "rail-place", place);
}
