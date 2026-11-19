//! The video site's screens at the web's routes, at 1440 by 900 on the parity day
//! (`parity/video.json`), each a second after the page opened as the data's first answers come
//! (the watch page fifteen, as the web's steps wait): `bun scripts/parity.ts board video` puts them
//! beside the web's. Then a few things the viewer does: a video liked shows under Liked videos, one
//! saved for later under Watch later, one watched in the history, and a channel followed in the
//! rail.

use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, SnapshotResults};
use sokkel_egui::Appearance;
use video::VideoApp;
use video::act::Act;
use video::app::DEMO_START;

pub fn render(path: &str, wait: i64) -> Harness<'static, VideoApp> {
    let mut app = VideoApp::at(DEMO_START, Appearance::default());
    app.go(path);
    app.wait(wait);
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut VideoApp| {
            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(4);
    harness
}

#[test]
fn every_screen() {
    let mut results = SnapshotResults::new();
    for (name, path, wait) in [
        ("video_home", "home", 1_000),
        ("video_watch", "watch/harbor-wakes", 15_000),
        ("video_channel", "channel/harbor", 1_000),
        ("video_subscriptions", "subscriptions", 1_000),
        ("video_library", "library", 1_000),
        ("video_history", "history", 1_000),
        ("video_playlists", "playlists", 1_000),
        ("video_search", "search/harbor", 1_000),
        ("video_settings", "settings", 1_000),
    ] {
        let mut harness = render(path, wait);
        results.add(harness.try_snapshot(name));
    }
}

#[test]
fn a_video_liked_shows_under_liked_videos() {
    let mut harness = render("watch/harbor-wakes", 1_000);
    assert!(
        !video::pages::liked::videos(&harness.state().data)
            .iter()
            .any(|one| one.id == "harbor-wakes")
    );
    // the like button says how many liked it
    harness.get_by_label("2.4K").click();
    harness.run_steps(2);
    harness.state_mut().wait(500);
    harness.run_steps(2);
    assert!(
        video::pages::liked::videos(&harness.state().data)
            .iter()
            .any(|one| one.id == "harbor-wakes")
    );
    harness.state_mut().go("liked");
    harness.run_steps(3);
    assert!(harness.query_by_label("How a harbor wakes up").is_some());
}

#[test]
fn a_video_saved_for_later_shows_under_watch_later() {
    let mut harness = render("home", 1_000);
    let ctx = harness.ctx.clone();
    let later = |app: &VideoApp| {
        app.data
            .playlists
            .seen_get("later")
            .is_some_and(|list| list.videos.iter().any(|one| one == "harbor-wakes"))
    };
    assert!(!later(harness.state()));
    harness.state_mut().apply(&ctx, Act::WatchLater("harbor-wakes".into()));
    harness.state_mut().wait(500);
    harness.run_steps(3);
    assert!(later(harness.state()));
    harness.state_mut().go("later");
    harness.run_steps(3);
    assert!(harness.query_by_label("How a harbor wakes up").is_some());
}

#[test]
fn a_video_watched_is_kept_in_the_history_where_it_stopped() {
    let mut harness = render("watch/storm-watch", 1_000);
    assert!(harness.state().data.history.seen_get("storm-watch").is_none());
    assert!(harness.state().player.attached);
    harness.get_by_label("Play").click();
    harness.run_steps(2);
    assert!(harness.state().player.playing);
    harness.state_mut().wait(10_000);
    harness.run_steps(2);
    let at = harness.state().player.time;
    assert!((9.0..11.0).contains(&at), "ten seconds in, not {at}");
    let ctx = harness.ctx.clone();
    harness.state_mut().apply(&ctx, Act::SetPlaying(false));
    harness.state_mut().wait(1_000);
    harness.run_steps(2);
    let row = harness.state().data.history.seen_get("storm-watch").cloned();
    let row = row.expect("the video in the history");
    assert!(
        (9.0..11.0).contains(&row.position),
        "kept where it stopped, not {}",
        row.position
    );
    harness.state_mut().go("history");
    harness.run_steps(3);
    assert!(
        harness
            .query_by_label("Watching a storm come in from the sea")
            .is_some()
    );
}

#[test]
fn a_channel_followed_shows_in_the_rail() {
    let mut harness = render("channel/atlas", 1_000);
    assert!(harness.query_all_by_label("Coastline Atlas").next().is_some());
    let rail = |harness: &Harness<'static, VideoApp>| {
        harness
            .state()
            .rail_groups()
            .0
            .iter()
            .flat_map(|group| group.entries.clone())
            .any(|entry| entry.id == "channel-atlas")
    };
    assert!(!rail(&harness));
    harness.get_by_label("Follow").click();
    harness.run_steps(2);
    harness.state_mut().wait(500);
    harness.run_steps(2);
    assert!(rail(&harness));
}
