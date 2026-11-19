//! The music and video parts that play: the track lists, the player bar, lyrics and the queue.

use eframe::egui::{Id, Painter, Rect, Ui, UiBuilder, pos2, vec2};
use sokkel_egui::{DropdownMenu, Icon, IconButton, MenuItem, StarButton, TransportButton, static_levels, tokens};
use sokkel_egui_media::{
    ArtworkCover, ArtworkHue as Hue, ArtworkMotif as Motif, DraggableTrackList, Lyrics, MediaBar, MediaBarLayout,
    MediaSkip, MediaVisualizerOption, Queue, QueueOptions, ReorderableTrackList, RepeatMode, Track, TrackList,
    TrackListVariant, TrackProgress, parse_lyrics,
};

use super::{State, section};

/// A value the story holds between frames, in egui's memory.
fn held<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, from: impl FnOnce() -> T) -> T {
    ui.data(|d| d.get_temp(Id::unique(("players-story", key))))
        .unwrap_or_else(from)
}

fn hold<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, value: T) {
    ui.data_mut(|d| d.insert_temp(Id::unique(("players-story", key)), value));
}

fn cover(motif: Motif, from: Hue, to: Hue) -> ArtworkCover {
    ArtworkCover { motif, from, to }
}

/// An album's songs by number, some with a second line.
fn album() -> Vec<Track> {
    vec![
        Track::new("1", "Leaving the Sound", 214.0).number(1),
        Track::new("2", "Low Tide", 247.0).number(2).subtitle("with Margaret"),
        Track::new("3", "Breakwater", 198.0)
            .number(3)
            .subtitle("with Grace and Linus"),
        Track::new("4", "Harbor at Dusk", 312.0).number(4),
        Track::new("5", "Salt Air", 189.0).number(5),
    ]
}

/// An album over two discs, each numbered from one.
fn set() -> Vec<Track> {
    [
        (1, 1, "Low Water", 214.0),
        (1, 2, "The Harbor Wall", 247.0),
        (1, 3, "Juniper Point", 198.0),
        (2, 1, "Spring Tide", 189.0),
        (2, 2, "The Long Bay", 312.0),
    ]
    .into_iter()
    .map(|(disc, n, title, length)| Track::new(format!("s{disc}-{n}"), title, length).number(n).disc(disc))
    .collect()
}

/// Songs from several albums, each with its cover, artist and album.
fn mixed() -> Vec<Track> {
    vec![
        Track::new("a", "Harbor at Dusk", 312.0)
            .subtitle("Grace")
            .meta("Open Water")
            .artwork(cover(Motif::Wave, Hue::Blue, Hue::Teal)),
        Track::new("b", "Dry Heat", 224.0)
            .subtitle("Linus")
            .meta("High Desert")
            .artwork(cover(Motif::Peaks, Hue::Amber, Hue::Coral)),
        Track::new("c", "Still Pond", 301.0)
            .subtitle("Margaret")
            .meta("Slow Current")
            .artwork(cover(Motif::Bands, Hue::Jade, Hue::Blue)),
        Track::new("d", "Flat Country", 256.0)
            .subtitle("Ada")
            .meta("Lowlands")
            .artwork(cover(Motif::Bands, Hue::Green, Hue::Jade)),
    ]
}

/// A row's star, held by the story, and its menu.
fn song_actions(ui: &mut Ui, key: &str, track: &Track, starred_from: bool) {
    let star_key = format!("{key}-star-{}", track.id);
    let mut starred = held(ui, &star_key, || starred_from);
    ui.add(StarButton::new(&mut starred));
    hold(ui, &star_key, starred);
    let more = ui.add(IconButton::new(Icon::Ellipsis, "More"));
    let items = [
        MenuItem::action("Play next").icon(Icon::ListStart),
        MenuItem::action("Add to queue").icon(Icon::ListEnd),
        MenuItem::action("Go to album").icon(Icon::Disc3),
    ];
    let _ = DropdownMenu::new(&items).end(true).width(176.0).show(&more);
}

pub fn track_list(_: &mut State, ui: &mut Ui) {
    let tracks = album();
    let width = ui.available_width();
    ui.horizontal_top(|ui| {
        let column = (width - 24.0) / 2.0;
        ui.allocate_ui(vec2(column, 0.0), |ui| {
            ui.vertical(|ui| {
                section(ui, "An album, the fourth song playing");
                let current = held(ui, "album-current", || "4".to_owned());
                let playing = held(ui, "album-playing", || true);
                let r = TrackList::new("story-album", &tracks)
                    .current(Some(&current))
                    .playing(playing)
                    .actions(|ui, track, _| song_actions(ui, "album", track, track.id == "2"))
                    .show(ui);
                if let Some(i) = r.played {
                    if tracks[i].id == current {
                        hold(ui, "album-playing", !playing);
                    } else {
                        hold(ui, "album-current", tracks[i].id.clone());
                        hold(ui, "album-playing", true);
                    }
                }
                section(ui, "Two discs, the time left on the one paused");
                let discs = set();
                TrackList::new("story-discs", &discs)
                    .current(Some("s2-1"))
                    .position(61.0)
                    .progress(TrackProgress::Left)
                    .show(ui);
            });
        });
        ui.add_space(24.0 - ui.spacing().item_spacing.x);
        ui.allocate_ui(vec2(column, 0.0), |ui| {
            ui.vertical(|ui| {
                section(ui, "Covers, how far the playing one has come along its foot");
                let songs = mixed();
                TrackList::new("story-covers", &songs)
                    .current(Some("b"))
                    .playing(true)
                    .position(150.0)
                    .progress(TrackProgress::Bar)
                    .show(ui);
                section(ui, "Plain, on the ground it is placed on");
                TrackList::new("story-plain", &songs[..3])
                    .variant(TrackListVariant::Plain)
                    .current(Some("c"))
                    .show(ui);
            });
        });
    });
}

pub fn reorderable_track_list(_: &mut State, ui: &mut Ui) {
    section(ui, "A playlist in the listener's own order, the second song playing");
    let mut songs: Vec<Track> = held(ui, "playlist", mixed);
    let r = ReorderableTrackList::new(
        TrackList::new("story-playlist", &songs)
            .current(Some("b"))
            .playing(true)
            .actions(|ui, track, _| {
                let key = format!("playlist-star-{}", track.id);
                let mut starred = held(ui, &key, || track.id == "c");
                ui.add(StarButton::new(&mut starred));
                hold(ui, &key, starred);
                if ui.add(IconButton::new(Icon::X, "Remove from the playlist")).clicked() {
                    hold(ui, "playlist-remove", track.id.clone());
                }
            }),
    )
    .show(ui);
    if let Some((from, to)) = r.moved {
        let song = songs.remove(from);
        songs.insert(to, song);
    }
    if let Some(gone) = ui.data_mut(|d| d.remove_temp::<String>(Id::unique(("players-story", "playlist-remove")))) {
        songs.retain(|s| s.id != gone);
    }
    hold(ui, "playlist", songs);
}

pub fn draggable_track_list(_: &mut State, ui: &mut Ui) {
    section(
        ui,
        "An album whose songs drag onto a playlist elsewhere; a press still plays one",
    );
    let tracks = album();
    let r = DraggableTrackList::new(
        TrackList::new("story-draggable", &tracks)
            .current(Some("2"))
            .actions(|ui, track, _| song_actions(ui, "draggable", track, false)),
        "song",
    )
    .show(ui);
    if let Some(i) = r.dragging {
        ui.label(format!("Dragging {}", tracks[i].title));
    }
}

/// A spectrum painted as bars from fixed levels, standing in for a live visualizer.
fn spectrum(painter: &Painter, rect: Rect) {
    let t = tokens(painter.ctx());
    let levels = static_levels(48, 11);
    let gap = 3.0;
    let bar = ((rect.width() - gap * 47.0) / 48.0).max(1.0);
    for (i, level) in levels.iter().enumerate() {
        let h = rect.height() * level;
        let x = rect.left() + i as f32 * (bar + gap);
        let r = Rect::from_min_max(pos2(x, rect.bottom() - h), pos2(x + bar, rect.bottom()));
        painter.rect_filled(r, t.radius("mini"), t.color_at("primary", 0.85));
    }
}

/// A song in the bar as the stories play it.
fn bar<'a>(id: &str, waveform: &'a [f32], layout: MediaBarLayout, size: f32) -> MediaBar<'a> {
    MediaBar::new(id, "Northern Coast")
        .artist("Ada")
        .album("Larkspur")
        .year(2024)
        .track(3, Some(12))
        .artwork(cover(Motif::Sun, Hue::Blue, Hue::Teal))
        .playing(true)
        .position(83.0, 245.0)
        .rate(1.0)
        .waveform(waveform)
        .visualizer(spectrum)
        .volume(0.7, false)
        .steps(true, true)
        .shuffle(false)
        .repeat(RepeatMode::All)
        .layout(layout)
        .size(size)
        .extra(|ui| {
            ui.add(TransportButton::new(Icon::ListMusic, "Show the queue").pressed(false));
        })
}

pub fn audio_bar(_: &mut State, ui: &mut Ui) {
    let waveform = static_levels(160, 3);
    section(
        ui,
        "One line: what plays, the transport over the position, the speed, the volume and the queue",
    );
    let _ = bar("story-bar-line", &waveform, MediaBarLayout::Rows, 64.0).show(ui);
    section(
        ui,
        "A station: the Live mark in place of the position, nothing to seek or skip back to",
    );
    let _ = MediaBar::new("story-bar-live", "Harbor Radio")
        .subtitle("Songs like Northern Coast")
        .placeholder_seed("harbor-radio")
        .playing(true)
        .live(true)
        .volume(0.4, false)
        .steps(false, true)
        .show(ui);
    section(
        ui,
        "A book: buttons that skip back 15 and forward 30 seconds, and the speed at 1.25x",
    );
    let _ = MediaBar::new("story-bar-book", "Chapter 4: The Ferry")
        .subtitle("The Harbor Keeper, read by Margaret")
        .artwork(cover(Motif::Rings, Hue::Teal, Hue::Blue))
        .playing(false)
        .position(1210.0, 2860.0)
        .skip(MediaSkip {
            back: 15.0,
            forward: 30.0,
        })
        .rate(1.25)
        .volume(0.8, false)
        .steps(true, true)
        .show(ui);
    section(ui, "Narrow: two rows, the title beside the transport over the position");
    let width = ui.available_width();
    let rect = ui.available_rect_before_wrap();
    let narrow = Rect::from_min_size(rect.min, vec2(width.min(420.0), 100.0));
    let mut child = ui.new_child(UiBuilder::new().max_rect(narrow));
    let _ = MediaBar::new("story-bar-narrow", "Harbor at Dusk")
        .artist("Grace")
        .album("Juniper")
        .artwork(cover(Motif::Wave, Hue::Violet, Hue::Pink))
        .playing(false)
        .position(12.0, 312.0)
        .rate(1.25)
        .volume(0.7, false)
        .steps(true, false)
        .show(&mut child);
}

pub fn audio_bar_layouts(_: &mut State, ui: &mut Ui) {
    let waveform = static_levels(160, 3);
    for (title, layout, size) in [
        (
            "Rows: the cover across both rows, the waveform under the words and the transport",
            MediaBarLayout::Rows,
            150.0,
        ),
        (
            "Panel: the cover flush and as tall as the bar, the transport along the foot",
            MediaBarLayout::Panel,
            170.0,
        ),
        (
            "Stage: the words, the waveform and the transport centered between the cover and the volume",
            MediaBarLayout::Stage,
            170.0,
        ),
    ] {
        section(ui, title);
        let _ = bar(&format!("story-layout-{title}"), &waveform, layout, size).show(ui);
        ui.add_space(4.0);
    }
}

const TIMED: &str = "[ti:Low Tide]
[ar:Ada]
[al:Larkspur]
[length:03:12]
[offset:+250]
[00:08.00]The water leaves the harbor wall
[00:13.50]and every boat lies down
[00:19.00]the gulls walk out across the bay
[00:24.50]to see what they have found
[00:30.00][01:36.00][02:30.00]Low tide, low tide
[00:35.00][01:41.00][02:35.00]the sea has somewhere else to be
[00:40.50][01:46.50][02:40.50]Low tide, low tide
[00:46.00][01:52.00][02:46.00]it always finds its way to me
[00:52.00]
[01:00.00]The ferry waits beside the pier
[01:05.50]with nothing left to float
[01:11.00]the keeper counts the hours out loud
[01:16.50]and paints another boat
[01:22.00]We read the moon like morning news
[01:28.00]and set the table slow
[01:58.00]
[02:04.00]And when it turns
[02:08.00]it turns for everyone
[02:52.00]Low tide
[02:58.00]";

const UNTIMED: &str = "Morning on the northern coast
the light comes in the long way round
we count the boats along the bay
and none of them make a sound

Hold on, hold on
the fog will lift by noon
hold on, hold on
the water knows the tune";

pub fn lyrics(_: &mut State, ui: &mut Ui) {
    let timed = parse_lyrics(TIMED);
    let untimed = parse_lyrics(UNTIMED);
    let width = ui.available_width();
    let height = ui.available_height() - 40.0;
    ui.horizontal_top(|ui| {
        let column = (width - 24.0) / 2.0;
        for (n, (title, lyrics)) in [("Timed, 1:41 in", &timed), ("Untimed, read as a page", &untimed)]
            .into_iter()
            .enumerate()
        {
            ui.allocate_ui(vec2(column, height), |ui| {
                ui.vertical(|ui| {
                    section(ui, title);
                    let at = held(ui, &format!("lyrics-at-{n}"), || 101.0);
                    let r = Lyrics::new(("story-lyrics", n), lyrics).position(at).show(ui);
                    if let Some(seek) = r.seek {
                        hold(ui, &format!("lyrics-at-{n}"), seek);
                    }
                });
            });
            if n == 0 {
                ui.add_space(24.0 - ui.spacing().item_spacing.x);
            }
        }
    });
}

pub fn fullscreen_player(_: &mut State, ui: &mut Ui) {
    let waveform = static_levels(160, 3);
    let timed = parse_lyrics(TIMED);
    let looks = [
        MediaVisualizerOption::new("spectrum", "Bars"),
        MediaVisualizerOption::new("mirror", "Mirror"),
        MediaVisualizerOption::new("wave", "Wave"),
    ];
    let lyrics_shown = held(ui, "fullscreen-lyrics", || true);
    let mut starred = held(ui, "fullscreen-star", || true);
    let r = MediaBar::new("story-fullscreen", "Low Tide")
        .artist("Ada")
        .album("Larkspur")
        .year(2024)
        .track(2, Some(10))
        .artwork(cover(Motif::Wave, Hue::Blue, Hue::Teal))
        .playing(true)
        .position(101.0, 192.0)
        .rate(1.0)
        .waveform(&waveform)
        .visualizer(spectrum)
        .volume(0.7, false)
        .steps(true, true)
        .shuffle(true)
        .repeat(RepeatMode::Off)
        .fullscreen(true)
        .lyrics(|ui| {
            let _ = Lyrics::new("story-fullscreen-lyrics", &timed).position(101.0).show(ui);
        })
        .lyrics_shown(lyrics_shown)
        .fullscreen_actions(|ui| {
            ui.add(StarButton::new(&mut starred));
        })
        .visualizer_options(&looks, None)
        .show(ui);
    if let Some(shown) = r.lyrics_shown {
        hold(ui, "fullscreen-lyrics", shown);
    }
    hold(ui, "fullscreen-star", starred);
}

/// The queue's items in the stories: songs as tracks with their covers.
fn queue_songs() -> Vec<Track> {
    let mut songs = mixed();
    songs.extend([
        Track::new("e", "Pilot Light", 241.0)
            .subtitle("The Tidelines")
            .artwork(cover(Motif::Sun, Hue::Pink, Hue::Coral)),
        Track::new("f", "Lantern Rock", 287.0)
            .subtitle("Northwater Choir")
            .artwork(cover(Motif::Peaks, Hue::Coral, Hue::Amber)),
    ]);
    songs
}

pub fn queue(_: &mut State, ui: &mut Ui) {
    let mut q: Queue<Track> = held(ui, "queue", || {
        Queue::new(
            queue_songs(),
            QueueOptions {
                start: Some(1),
                ..Default::default()
            },
        )
    });
    let Some(now) = q.item().cloned() else {
        return;
    };
    section(
        ui,
        "Up next, in the order it plays; shuffle draws a new order after the song playing",
    );
    let next = q.up_next();
    let first = next.first().map_or(0, |e| e.position);
    let tracks: Vec<Track> = next.iter().map(|e| e.item.clone()).collect();
    let width = ui.available_width().min(560.0);
    let rect = ui.available_rect_before_wrap();
    let mut child = ui.new_child(UiBuilder::new().max_rect(Rect::from_min_size(rect.min, vec2(width, rect.height()))));
    let r = ReorderableTrackList::new(
        TrackList::new("story-queue", &tracks)
            .variant(TrackListVariant::Plain)
            .follow(false)
            .actions(|ui, _, i| {
                if ui.add(IconButton::new(Icon::X, "Remove from the queue")).clicked() {
                    hold(ui, "queue-remove", i);
                }
            }),
    )
    .show(&mut child);
    ui.advance_cursor_after_rect(child.min_rect());
    if let Some((from, to)) = r.moved {
        q = q.moved(first + from, first + to);
    }
    if let Some(i) = r.played {
        q = q.play(next[i].index);
    }
    if let Some(i) = ui.data_mut(|d| d.remove_temp::<usize>(Id::unique(("players-story", "queue-remove")))) {
        q = q.remove(&[next[i].index]);
    }
    ui.add_space(16.0);
    let a = bar_for(ui, &q, &now);
    if let Some(on) = a.shuffle {
        q = q.shuffled(on, Default::default());
    }
    if let Some(mode) = a.repeat {
        q = q.repeating(mode);
    }
    if a.next {
        q = q.next().unwrap_or(q);
    }
    if a.previous {
        q = q.previous().unwrap_or(q);
    }
    hold(ui, "queue", q);
}

fn bar_for(ui: &mut Ui, q: &Queue<Track>, now: &Track) -> sokkel_egui_media::MediaBarResponse {
    let mut b = MediaBar::new("story-queue-bar", &now.title)
        .placeholder_seed(now.id.clone())
        .playing(true)
        .position(40.0, now.duration)
        .volume(0.7, false)
        .steps(q.previous().is_some(), q.next().is_some())
        .shuffle(q.shuffle)
        .repeat(q.repeat)
        .resizable(false)
        .expandable(false);
    if let Some(cover) = now.artwork {
        b = b.artwork(cover);
    }
    if let Some(by) = &now.subtitle {
        b = b.artist(by.clone());
    }
    b.show(ui)
}
