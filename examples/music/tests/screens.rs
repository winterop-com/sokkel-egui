//! The music app's screens at the web's routes, at 1440 by 900 on the parity day
//! (`parity/music.json`): `bun scripts/parity.ts board music` puts them beside the web's. Then a
//! few things the listener does: a song starred shows in the favorites, a playlist made holds a
//! song added to it, a song played moves along with the clock.

use egui_kittest::{Harness, SnapshotResults};
use music::MusicApp;
use music::act::Act;
use music::app::DEMO_START;
use music::data::FavoriteKind;
use music::dialogs::Asking;
use sokkel_egui::Appearance;

pub fn render(path: &str) -> Harness<'static, MusicApp> {
    let mut app = MusicApp::at(DEMO_START, Appearance::default());
    app.go(path);
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut MusicApp| {
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
        ("music_albums", "albums"),
        ("music_album", "albums/salt-and-cedar"),
        ("music_artists", "artists"),
        ("music_artist", "artists/eelgrass"),
        ("music_tracks", "tracks"),
        ("music_playlists", "playlists"),
        ("music_radio", "radio"),
        ("music_settings", "settings"),
    ] {
        let mut harness = render(path);
        results.add(harness.try_snapshot(name));
    }
}

#[test]
fn a_song_starred_shows_in_the_favorites() {
    let mut harness = render("albums/salt-and-cedar");
    let song = "salt-and-cedar-1-1";
    assert!(!harness.state().library.is_starred(FavoriteKind::Song, song));
    // the first row's star, as the reader presses it: the header's own star for the album comes first
    use egui_kittest::kittest::Queryable;
    harness
        .get_all_by_label("Add to favorites")
        .nth(1)
        .expect("the first row's star")
        .click();
    harness.run_steps(2);
    assert!(harness.state().library.is_starred(FavoriteKind::Song, song));
    // listed first among the favorite songs
    harness.state_mut().go("favorites");
    harness.run_steps(3);
    let title = music::catalog::library_song(song)
        .map(|s| s.title.clone())
        .unwrap_or_default();
    assert!(harness.query_by_label(&format!("Play {title}")).is_some());
}

#[test]
fn a_playlist_made_holds_the_song_added_to_it() {
    let mut harness = render("playlists");
    harness.state_mut().ask(Asking::New(Vec::new(), None));
    harness.run_steps(2);
    harness.state_mut().dialog.name = "Harbor nights".into();
    harness.run_steps(2);
    use egui_kittest::kittest::Queryable;
    harness.get_by_label("Create").click();
    harness.run_steps(3);
    let app = harness.state();
    assert_eq!(app.path.first().map(String::as_str), Some("playlists"));
    let id = app.path[1].clone();
    assert_eq!(
        app.library.playlist(&id).map(|p| p.name.as_str()),
        Some("Harbor nights")
    );
    assert!(app.library.playlist(&id).is_some_and(|p| p.entries.is_empty()));

    // a song added from its menu, as the act the menu asks for
    let ctx = harness.ctx.clone();
    harness
        .state_mut()
        .apply(&ctx, Act::AddTo(id.clone(), vec!["salt-and-cedar-1-3".into()], None));
    harness.run_steps(2);
    let entries = harness.state().library.playlist_entries(&id);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].song, "salt-and-cedar-1-3");
}

#[test]
fn a_song_played_moves_along_with_the_clock_and_the_next_follows() {
    let mut harness = render("albums");
    use egui_kittest::kittest::Queryable;
    // the bar's play button
    harness.get_by_label("Play").click();
    harness.run_steps(2);
    assert!(harness.state().player.playing);
    harness.state_mut().wait(10_000);
    harness.run_steps(1);
    let at = harness.state().player.position;
    assert!((9.0..11.0).contains(&at), "ten seconds in, not {at}");
    // past its end the next song plays
    let first = harness.state().player.queue.at;
    harness.state_mut().wait(60_000);
    harness.run_steps(1);
    assert_eq!(harness.state().player.queue.at, first + 1);
}
