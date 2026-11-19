//! One video playing, as the web's `pages/Watch.tsx` draws it: the player with its frame painted
//! from the clip's scene at the second playing, the stats over it and Up next as it ends; the
//! title, the channel with Follow and what the viewer does with the video; the description; the
//! comments (`pages/watch/Comments.tsx`); and what plays next beside them (`pages/watch/Aside.tsx`).

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, ChipBar, CommentComposer, CommentThread, DownloadAsked, DownloadButton,
    DownloadState, DropdownMenu, Empty, Gap, Icon, MenuItem, MetaLine, ShowMore, Stack, SwitchField, TextRole,
    ThreadComment, TransportButton, TransportSize, VideoPlayer, VideoQuality, tokens,
};
use sokkel_egui_media::{
    ChannelRow, MediaAction, MediaActions, MediaRating, UpNext, VideoStats, VideoStatsOverlay, VideoStatsTone,
    WatchLayout,
};

use crate::act::Act;
use crate::app::{VideoApp, Views};
use crate::captions::caption_settings;
use crate::catalog::{VIEWER, channel_of, video_by_id};
use crate::clips::{FPS, clip_of, cues_of, draw_frame, source::stats_of, spec_of};
use crate::data::{CaptionTrack, PrefsPatch, Quality, RatingKind};
use crate::pages::watch::{Watching, aside, comments};
use crate::parts::CardLayout;
use crate::screens::{outline, poster, video_card};

/// The player, `height` tall when it fills the screen.
pub fn player(ui: &mut Ui, app: &VideoApp, height: Option<f32>, acts: &mut Vec<Act>) {
    let Some(watching) = &app.watching else { return };
    let data = &app.data;
    let video = watching.video;
    let view = watching.view(data);
    let state = &app.player;
    let spec = spec_of(video);
    let time = state.time;
    let started = state.started;
    // before it plays, its poster; after, the frame at the second playing, the title card first
    let picture = move |painter: &egui::Painter, rect: egui::Rect| {
        if started {
            draw_frame(painter, rect, &spec, time, true);
        } else {
            draw_frame(painter, rect, &spec, crate::clips::poster_time(video), false);
        }
    };
    let qualities: Vec<VideoQuality> = view
        .qualities
        .iter()
        .map(|(value, label, detail)| VideoQuality {
            value: (*value).to_owned(),
            label: (*label).to_owned(),
            detail: Some(detail.clone()),
        })
        .collect();
    let captions = caption_settings(&data.prefs.captions);
    let tracks = vec!["English".to_owned()];
    let showing = match &captions.track {
        CaptionTrack::On(_) => Some(0),
        _ => None,
    };
    let cue = showing
        .and_then(|_| {
            cues_of(video)
                .into_iter()
                .find(|cue| started && time >= cue.start && time < cue.end)
        })
        .map(|cue| cue.text.to_owned());
    let theater = data.prefs.theater;
    let mut theater_pressed = false;
    let mut stats_closed = false;
    let stats_rows: Vec<(String, String)> = view
        .stats
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .collect();
    let mut up_next = sokkel_egui_media::UpNextResponse::default();
    let left = view.up_next.as_ref().and_then(|one| one.countdown).map(|from| {
        let gone = app
            .player
            .ended_at
            .map_or(0.0, |ended| (app.now() - ended) as f64 / 1000.0);
        from - gone
    });
    let mut player = VideoPlayer::new(("player", video.id))
        .picture(picture)
        .title(video.title)
        .position(time, video.duration)
        .playing(state.playing)
        .started(started)
        .volume(state.volume, state.muted)
        .rate(state.rate)
        .qualities(&qualities, view.quality)
        .tracks(&tracks, showing)
        .cue(cue, captions.style())
        .picture_in_picture(app.picture_in_picture)
        .fullscreen(app.fullscreen)
        .actions(|ui| {
            let label = if theater { "Leave theater mode" } else { "Theater mode" };
            theater_pressed = ui
                .add(
                    TransportButton::new(Icon::RectangleHorizontal, label)
                        .tone(sokkel_egui::MediaTone::Overlay)
                        .size(TransportSize::Md)
                        .pressed(theater),
                )
                .clicked();
        });
    if let Some(height) = height {
        player = player.height(height);
    }
    // the stats over the picture while they are on
    if view.stats.is_some() {
        let rendition = watching.rendition(data);
        let clip = clip_of(video, rendition);
        let source = stats_of(&clip, app.now() as f64 / 1000.0);
        let stats = VideoStats {
            resolution: Some((source.level.width, source.level.height)),
            bitrate: Some(source.level.bitrate),
            bandwidth: Some(source.bandwidth),
            buffer_ahead: video.duration - time,
            dropped_frames: Some(0),
            total_frames: Some((time * FPS) as u64),
            level: Some(source.level.index),
            levels: Some(source.level.count),
        };
        let smooth = stats.buffer_ahead > 1.0;
        let rows = &stats_rows;
        let closed = &mut stats_closed;
        player = player.overlay(move |ui| {
            *closed = VideoStatsOverlay::new(Some(stats))
                .headline(
                    if smooth {
                        "Playing smoothly"
                    } else {
                        "Filling the buffer"
                    },
                    if smooth {
                        VideoStatsTone::Good
                    } else {
                        VideoStatsTone::Warning
                    },
                )
                .rows(rows)
                .closable(true)
                .show(ui);
        });
    }
    // what plays next, over the dimmed picture, once it ended
    if let Some(next) = view.up_next.as_ref().and_then(|one| video_by_id(one.id)) {
        player = player.cover(|ui| {
            up_next = UpNext::new(next.title)
                .subtitle(channel_of(next).name)
                .cover(poster(next))
                .countdown(left)
                .cancellable(true)
                .words(sokkel_egui_media::UpNextWords {
                    cancel: "Stay here".into(),
                    ..Default::default()
                })
                .show(ui);
        });
    }
    let out = player.show(ui);
    if theater_pressed {
        acts.push(Act::Theater);
    }
    if stats_closed {
        acts.push(Act::Stats);
    }
    if up_next.play {
        acts.push(Act::PlayNext);
    }
    if up_next.cancel {
        acts.push(Act::Stay);
    }
    if let Some(on) = out.playing {
        acts.push(Act::SetPlaying(on));
    }
    if let Some(at) = out.seek {
        acts.push(Act::Seek(at));
    }
    if let Some(level) = out.volume {
        acts.push(Act::Volume(level));
    }
    if let Some(muted) = out.muted {
        acts.push(Act::Muted(muted));
    }
    if let Some(rate) = out.rate {
        acts.push(Act::Rate2(rate));
    }
    if let Some(quality) = out.quality.as_deref().and_then(Quality::from_id) {
        acts.push(Act::Quality(quality));
    }
    if let Some(track) = out.track {
        acts.push(Act::Captions(track.is_some()));
    }
    if let Some(on) = out.fullscreen {
        acts.push(Act::Fullscreen(on));
    }
    if let Some(on) = out.picture_in_picture {
        acts.push(Act::PictureInPicture(on));
    }
}

/// The watch page.
pub fn watch(ui: &mut Ui, app: &VideoApp, views: &mut Views, acts: &mut Vec<Act>) {
    let Some(watching) = &app.watching else {
        let pressed = Empty::new("No such video")
            .description("There is no video at this address. It may have been taken down.")
            .show(ui, |ui| ui.add(outline("Go home", None)).clicked());
        if pressed {
            acts.push(Act::Go("home".into()));
        }
        return;
    };
    let window = ui.ctx().content_rect().height();
    let mut player_acts = Vec::new();
    let mut body_acts = Vec::new();
    let mut aside_acts = Vec::new();
    let (filter, order) = (&mut views.filter, &mut views.order);
    WatchLayout::new(|ui| {
        if !app.fullscreen && !app.picture_in_picture {
            player(ui, app, None, &mut player_acts);
        } else {
            // the player's place, while it plays elsewhere
            let t = tokens(ui.ctx());
            let width = ui.available_width();
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, width * 9.0 / 16.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, t.radius("surface"), t.color("terminal"));
            let said = if app.picture_in_picture {
                "Playing in picture in picture"
            } else {
                "Playing in full screen"
            };
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                said,
                TextRole::Body.font(&t),
                t.color("terminal-muted"),
            );
        }
    })
    .theater(app.data.prefs.theater, window - 144.0)
    .body(|ui| body(ui, app, watching, order, &mut body_acts))
    .aside(|ui| aside_column(ui, app, watching, filter, &mut aside_acts))
    .show(ui);
    acts.extend(player_acts);
    acts.extend(body_acts);
    acts.extend(aside_acts);
}

/// Under the player: the title, the channel and the actions, the description and the comments.
fn body(ui: &mut Ui, app: &VideoApp, watching: &Watching, order: &mut comments::Order, acts: &mut Vec<Act>) {
    let t = tokens(ui.ctx());
    let data = &app.data;
    let video = watching.video;
    let channel = channel_of(video);
    let view = watching.view(data);
    Stack::new(Gap::Lg).show(ui, |ui| {
        ui.label(TextRole::PageTitle.rich(&t, video.title).color(t.color("foreground")));
        let mut actions_out = sokkel_egui_media::MediaActionsResponse::default();
        let mut download_asked = None;
        let saved = view.saved_label == "Saved";
        let stats_menu = [MenuItem::action("Stats").icon(if data.prefs.stats { Icon::Check } else { Icon::Activity })];
        let rating = match view.rating {
            Some("like") => Some(MediaRating::Like),
            Some("dislike") => Some(MediaRating::Dislike),
            _ => None,
        };
        let download_state = match view.download_state {
            "kept" => DownloadState::Kept,
            "running" => DownloadState::Running,
            _ => DownloadState::Idle,
        };
        let row = ChannelRow::new(channel.name)
            .linked(true)
            .color(channel.color)
            .followers(view.followers as f64)
            .following(view.following)
            .actions(|ui| {
                let download = {
                    let mut button = DownloadButton::new(("download", video.id), download_state)
                        .size(ButtonSize::Sm)
                        .variant(ButtonVariant::Outline)
                        .cancellable()
                        .removable();
                    if let Some(share) = view.download_value {
                        button = button.value(share as f32);
                    }
                    if let Some(detail) = &view.kept_detail {
                        button = button.kept_detail(detail);
                    }
                    button
                };
                let asked = &mut download_asked;
                actions_out = MediaActions::new()
                    .rating(rating, Some(view.likes as f64))
                    .action(MediaAction::new("Share").icon(Icon::Share2))
                    .action(MediaAction::new(view.saved_label).icon(Icon::Bookmark).pressed(saved))
                    .action(
                        MediaAction::new(&view.download_label)
                            .icon(Icon::Download)
                            .render(move |ui| *asked = download.show(ui).asked),
                    )
                    .menu(&stats_menu)
                    .show(ui);
            })
            .show(ui);
        if let Some(on) = row.following {
            acts.push(Act::Follow(channel.id.to_owned(), on));
        }
        if row.opened {
            acts.push(Act::Go(format!("channel/{}", channel.id)));
        }
        if let Some(rating) = actions_out.rating {
            acts.push(Act::Rate(rating.map(|one| match one {
                MediaRating::Like => RatingKind::Like,
                MediaRating::Dislike => RatingKind::Dislike,
            })));
        }
        match actions_out.action {
            Some(0) => acts.push(Act::Share(video.id.to_owned(), app.player_state().map(|one| one.time))),
            Some(1) => acts.push(Act::Save(video.id.to_owned())),
            Some(2) => acts.push(if view.download_state == "kept" {
                Act::Go("downloads".into())
            } else {
                Act::Download
            }),
            _ => {}
        }
        if actions_out.menu == Some(0) {
            acts.push(Act::Stats);
        }
        match download_asked {
            Some(DownloadAsked::Download) => acts.push(Act::Download),
            Some(DownloadAsked::Cancel) => acts.push(Act::CancelDownload),
            Some(DownloadAsked::Remove) => acts.push(Act::RemoveThisDownload),
            Some(DownloadAsked::Open) => acts.push(Act::Go("downloads".into())),
            _ => {}
        }
        ShowMore::new(("description", video.id), video.description)
            .well(true)
            .head(MetaLine::new(view.meta.clone()).sm().strong())
            .show(ui);
        comments_section(ui, app, watching, order, acts);
    });
}

/// What people say under the video.
fn comments_section(
    ui: &mut Ui,
    app: &VideoApp,
    watching: &Watching,
    order: &mut comments::Order,
    acts: &mut Vec<Act>,
) {
    let t = tokens(ui.ctx());
    let video = watching.video;
    let channel = channel_of(video);
    let shown = comments::view(&app.data, video, channel, *order);
    Stack::new(Gap::Lg).show(ui, |ui| {
        title_row(
            ui,
            |ui| {
                ui.label(
                    TextRole::SectionTitle
                        .rich(&t, &shown.title)
                        .color(t.color("foreground")),
                );
            },
            |ui| {
                let label = comments::ORDERS
                    .iter()
                    .find(|(one, _)| one == order)
                    .map_or("", |(_, label)| *label);
                let trigger = ui.add(
                    Button::new(label)
                        .icon(Icon::SlidersHorizontal)
                        .trailing(Icon::ChevronDown)
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::Sm),
                );
                let items: Vec<MenuItem> = comments::ORDERS
                    .iter()
                    .map(|(one, label)| {
                        let item = MenuItem::action(*label);
                        if one == order { item.icon(Icon::Check) } else { item }
                    })
                    .collect();
                if let Some(index) = DropdownMenu::new(&items).end(true).width(176.0).show(&trigger) {
                    *order = comments::ORDERS[index].0;
                }
            },
        );
        let composer = CommentComposer::new(("composer", video.id), VIEWER.name())
            .color(VIEWER.color())
            .show(ui);
        if let Some(text) = composer.submit {
            acts.push(Act::Comment(text, None));
        }
        let threads: Vec<ThreadComment> = shown.threads.iter().map(|one| thread_of(one, app.now())).collect();
        let menus: std::collections::HashMap<String, &'static str> = all_menus(&shown.threads);
        let out = CommentThread::new(("thread", video.id), &threads)
            .viewer(VIEWER.name(), Some(VIEWER.color().to_owned()))
            .pinned_by(channel.owner.name())
            .reactions(true)
            .menu(|comment| match menus.get(&comment.id).copied() {
                Some("Delete") => vec![MenuItem::action("Delete").icon(Icon::Trash2).destructive()],
                _ => vec![MenuItem::action("Report").icon(Icon::Flag)],
            })
            .show(ui);
        if let Some(id) = out.like {
            acts.push(Act::RateComment(id, true));
        }
        if let Some(id) = out.dislike {
            acts.push(Act::RateComment(id, false));
        }
        if let Some((id, text)) = out.reply {
            acts.push(Act::Comment(text, Some(id)));
        }
        if let Some((id, emoji, name)) = out.reaction {
            acts.push(Act::React(id, emoji, name));
        }
        if let Some((id, _)) = out.menu {
            acts.push(match menus.get(&id).copied() {
                Some("Delete") => Act::DeleteComment(id),
                _ => Act::ReportComment(id),
            });
        }
    });
}

/// A toolbar with a title, as the web's `Toolbar` with a `ToolbarTitle` and an `end`: the title at
/// the start and the end's parts from the row's end, a control tall.
fn title_row(ui: &mut Ui, start: impl FnOnce(&mut Ui), end: impl FnOnce(&mut Ui)) {
    let height = tokens(ui.ctx()).density("control");
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            start(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), end);
        },
    );
}

/// Each comment's one menu entry, by its id, replies included.
fn all_menus(threads: &[comments::Thread]) -> std::collections::HashMap<String, &'static str> {
    let mut out = std::collections::HashMap::new();
    for one in threads {
        out.insert(one.id.clone(), one.menu);
        out.extend(all_menus(&one.replies));
    }
    out
}

/// A comment as the thread draws it, its time said relative to now.
fn thread_of(one: &comments::Thread, now: i64) -> ThreadComment {
    ThreadComment {
        id: one.id.clone(),
        author: one.author.name().to_owned(),
        color: Some(one.color.to_owned()),
        time: sokkel_egui::relative_time((one.at - now) as f64),
        time_title: Some(one.time_title.clone()),
        body: one.body.clone(),
        likes: one.likes.clone(),
        liked: one.liked,
        disliked: one.disliked,
        pinned: one.pinned,
        creator: one.creator,
        reactions: one
            .reactions
            .iter()
            .map(|(id, label, count, mine)| sokkel_egui::Reaction {
                id: id.clone(),
                label: label.clone(),
                count: *count,
                mine: *mine,
            })
            .collect(),
        replies: one.replies.iter().map(|reply| thread_of(reply, now)).collect(),
    }
}

/// Beside the player: what plays next, with Autoplay and its chips, or the playlist playing.
fn aside_column(ui: &mut Ui, app: &VideoApp, watching: &Watching, filter: &mut aside::Filter, acts: &mut Vec<Act>) {
    let t = tokens(ui.ctx());
    let data = &app.data;
    let video = watching.video;
    let list = watching.playlist(data);
    let shown = aside::view(data, video, list, *filter);
    let mut autoplay = shown.autoplay;
    Stack::new(Gap::Md).show(ui, |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            title_row(
                ui,
                |ui| {
                    ui.label(
                        TextRole::DetailTitle
                            .rich(&t, &shown.title)
                            .color(t.color("foreground")),
                    );
                },
                |ui| {
                    ui.add(
                        SwitchField::new(&mut autoplay, "Autoplay").explain("Plays what is next when this video ends"),
                    );
                },
            );
            if let Some(meta) = &shown.meta {
                MetaLine::new(meta.clone()).show(ui);
            }
        });
        if list.is_none() {
            let values = [
                aside::Filter::All,
                aside::Filter::Channel,
                aside::Filter::Topic,
                aside::Filter::Unwatched,
            ];
            let chips: Vec<(&str, &str)> = values
                .iter()
                .zip(&shown.chips)
                .map(|(one, label)| (one.value(), label.as_str()))
                .collect();
            let mut chosen = filter.value().to_owned();
            if ChipBar::new("up-next", &chips).show(ui, &mut chosen)
                && let Some(one) = values.iter().find(|one| one.value() == chosen)
            {
                *filter = *one;
            }
        }
        Stack::new(Gap::Md).show(ui, |ui| {
            for entry in &shown.videos {
                let Some(one) = video_by_id(entry.id) else { continue };
                video_card(
                    ui,
                    app,
                    "aside",
                    one,
                    CardLayout::Row,
                    false,
                    entry.list.as_deref(),
                    entry.playing,
                    Vec::new(),
                    acts,
                );
            }
        });
    });
    if autoplay != shown.autoplay {
        acts.push(Act::Prefs(PrefsPatch {
            autoplay: Some(autoplay),
            ..PrefsPatch::default()
        }));
    }
}
