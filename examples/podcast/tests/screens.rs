//! The podcasts app's screens at the web's routes, at 1440 by 900 on the parity day
//! (`parity/podcast.json`): `bun scripts/parity.ts board podcast` puts them beside the web's. Then a
//! few things the listener does: an episode starred shows under Starred, one queued plays after
//! the rest, and the episode playing moves along with the clock, the next in the queue following.

use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, SnapshotResults};
use podcast::PodcastApp;
use podcast::act::Act;
use podcast::app::DEMO_START;
use podcast::script::length_of;
use podcast::state::Where;
use sokkel_egui::Appearance;

pub fn render(path: &str) -> Harness<'static, PodcastApp> {
    let mut app = PodcastApp::at(DEMO_START, Appearance::default());
    app.go(path);
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut PodcastApp| {
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
    for (name, path) in [
        ("podcast_home", "home"),
        ("podcast_inbox", "inbox"),
        ("podcast_queue", "queue"),
        ("podcast_discover", "discover"),
        ("podcast_shows", "shows"),
        ("podcast_show", "show/harbor-log"),
        ("podcast_episode", "episode/low-tide-hours-1"),
        ("podcast_stats", "stats"),
        ("podcast_settings", "settings"),
    ] {
        let mut harness = render(path);
        results.add(harness.try_snapshot(name));
    }
}

#[test]
fn an_episode_starred_shows_under_starred() {
    let mut harness = render("inbox");
    let id = "harbor-log-3";
    assert!(!harness.state().state.episode(id).is_some_and(|row| row.starred));
    let title = podcast::catalog::episode_by_id(id)
        .map(|one| one.title.clone())
        .unwrap_or_default();
    // the row's star, pressed as the listener presses it: the fourth row's
    let stars: Vec<_> = harness.get_all_by_label("Star").collect();
    assert!(stars.len() >= 4, "a star on each row");
    stars[3].click();
    harness.run_steps(2);
    assert!(harness.state().state.episode(id).is_some_and(|row| row.starred));
    harness.state_mut().go("starred");
    harness.run_steps(3);
    assert!(harness.query_by_label(&format!("Play {title}")).is_some());
}

#[test]
fn an_episode_queued_plays_after_the_rest() {
    let mut harness = render("queue");
    let ctx = harness.ctx.clone();
    harness
        .state_mut()
        .apply(&ctx, Act::QueueAdd("harbor-log-3".into(), Where::End));
    harness.run_steps(2);
    let queue = harness.state().state.player.queue.clone();
    assert_eq!(queue.last().map(String::as_str), Some("harbor-log-3"));
    assert_eq!(queue.len(), 4);
    // Play next takes the first, and the queue moves up
    harness.get_by_label("Play next").click();
    harness.run_steps(2);
    assert_eq!(harness.state().state.current(), Some(queue[0].as_str()));
    assert_eq!(harness.state().state.player.queue.len(), 3);
}

#[test]
fn the_episode_playing_moves_along_with_the_clock_and_the_next_follows() {
    let mut harness = render("home");
    let first = harness
        .state()
        .state
        .current()
        .map(str::to_owned)
        .expect("an episode in the player");
    let from = harness.state().state.position();
    // the bar's play button
    harness.get_by_label("Play").click();
    harness.run_steps(2);
    assert!(harness.state().state.playing);
    harness.state_mut().wait(10_000);
    harness.run_steps(1);
    let at = harness.state().state.position();
    assert!(
        (from + 9.0..from + 11.0).contains(&at),
        "ten seconds on from {from}, not {at}"
    );
    // past its end the next in the queue plays
    let next = harness.state().state.player.queue[0].clone();
    let left = length_of(&first) - at;
    harness.state_mut().wait((left * 1000.0) as i64 + 2_000);
    harness.run_steps(2);
    assert_eq!(harness.state().state.current(), Some(next.as_str()));
    assert!(harness.state().state.playing);
    assert!(harness.state().state.episode(&first).is_some_and(|row| row.played));
}
