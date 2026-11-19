//! A rail entry that takes drops, a star that toggles, and a download kept and removed in two
//! presses.

use egui::accesskit::Role;
use egui::{Rect, UiBuilder, pos2, vec2};
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::{
    Appearance, DownloadAsked, DownloadButton, DownloadState, DragData, Icon, Rail, RailDrop, RailEntry, RailGroup,
    StarButton, download_share, format_percent, install,
};

#[derive(Default)]
struct Shelf {
    place: String,
    carrying: Option<DragData>,
    dropped: Option<RailDrop>,
}

fn song(kind: &str) -> DragData {
    DragData {
        kind: kind.into(),
        label: "Low Lagoon".into(),
        duration: None,
        value: "salt-and-cedar-1".into(),
    }
}

fn rail() -> Harness<'static, Shelf> {
    let appearance = Appearance::default();
    let groups = vec![
        RailGroup::new(None, [RailEntry::new("albums", "Albums", Icon::DiscAlbum)]),
        RailGroup::new(
            Some("Playlists"),
            [
                RailEntry::new("deep-focus", "Deep focus", Icon::ListMusic).takes("song"),
                RailEntry::new("long-drive", "Long drive", Icon::ListMusic).takes("song"),
            ],
        ),
    ];
    let mut harness = Harness::builder().with_size(vec2(240.0, 400.0)).build_ui_state(
        move |ui, shelf: &mut Shelf| {
            install(ui.ctx(), &appearance);
            if let Some(data) = shelf.carrying.take() {
                egui::DragAndDrop::set_payload(ui.ctx(), data);
            }
            let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(240.0, 400.0));
            let mut dropped = None;
            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                ui.add(Rail::new("Tidevann", &groups, &mut shelf.place).drops(&mut dropped));
            });
            if dropped.is_some() {
                shelf.dropped = dropped;
            }
        },
        Shelf {
            place: "albums".into(),
            ..Default::default()
        },
    );
    harness.run_steps(3);
    harness
}

#[test]
fn a_song_let_go_on_a_playlist_lands_there() {
    let mut harness = rail();
    let at = harness.get_by_label("Long drive").rect().center();
    harness.state_mut().carrying = Some(song("song"));
    harness.hover_at(at);
    harness.run_steps(1);
    // the row the song is over, ringed on the hover ground
    harness.snapshot("rail_drop_over");
    harness.drop_at(at);
    harness.run_steps(2);
    let dropped = harness.state().dropped.clone().expect("a drop");
    assert_eq!(dropped.entry, "long-drive");
    assert_eq!(dropped.data.label, "Low Lagoon");
}

#[test]
fn a_place_takes_only_the_kinds_it_names() {
    let mut harness = rail();
    for (label, kind) in [("Albums", "song"), ("Deep focus", "clip")] {
        let at = harness.get_by_label(label).rect().center();
        harness.state_mut().carrying = Some(song(kind));
        harness.hover_at(at);
        harness.run_steps(1);
        harness.drop_at(at);
        harness.run_steps(2);
    }
    assert!(harness.state().dropped.is_none());
}

#[test]
fn a_star_says_what_a_press_does_and_flips() {
    let appearance = Appearance::default();
    let mut harness = Harness::builder().with_size(vec2(120.0, 60.0)).build_ui_state(
        move |ui, starred: &mut bool| {
            install(ui.ctx(), &appearance);
            ui.add(StarButton::new(starred));
        },
        false,
    );
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::Button, "Add to favorites").click();
    harness.run_steps(2);
    assert!(*harness.state());
    harness
        .get_by_role_and_label(Role::Button, "Remove from favorites")
        .click();
    harness.run_steps(2);
    assert!(!*harness.state());
}

#[test]
fn a_kept_copy_is_removed_in_two_presses() {
    let appearance = Appearance::default();
    let mut harness = Harness::builder().with_size(vec2(240.0, 60.0)).build_ui_state(
        move |ui, asked: &mut Vec<DownloadAsked>| {
            install(ui.ctx(), &appearance);
            let shown = DownloadButton::new("album", DownloadState::Kept)
                .removable()
                .kept_detail("On this device, 48.2 MB")
                .show(ui);
            asked.extend(shown.asked);
        },
        Vec::new(),
    );
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::Button, "Kept on this device. Remove it")
        .click();
    harness.run_steps(2);
    assert!(harness.state().is_empty());
    harness
        .get_by_role_and_label(Role::Button, "Press again to remove this download")
        .click();
    harness.run_steps(2);
    assert_eq!(harness.state().as_slice(), &[DownloadAsked::Remove]);
}

#[test]
fn a_download_says_its_share_as_progress_says_it() {
    assert_eq!(download_share(None, Some(3), Some(12)), 0.25);
    assert_eq!(download_share(Some(0.4), Some(1), Some(2)), 0.4);
    assert_eq!(download_share(Some(1.5), None, None), 1.0);
    assert_eq!(download_share(None, Some(3), None), 0.0);
    assert_eq!(download_share(None, Some(3), Some(0)), 0.0);
    assert_eq!(format_percent(0.62), "62%");
    assert_eq!(format_percent(0.996), ">99%");
    assert_eq!(format_percent(0.125), "13%");
    assert_eq!(format_percent(0.001), "<1%");
}
