//! The clip timeline as cut's editor draws it: the harbor project's tracks and clips at the size of
//! the web shot's Timeline (`parity/cut/web/cut_editor.png`), a clip chosen with a locked track, and
//! the keys and drags an editor answers, each applied as the app would.

use std::sync::Arc;

use egui::{Key, Modifiers, Pos2, Vec2, pos2, vec2};
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::{
    Appearance, ClipChange, ClipDrop, ClipTimeline, ClipTrackKind, DragData, KindColor, Scheme, TimelineClip,
    TimelineTrack, TrackChange, clip_fit_zoom, clip_tick_step, format_timecode, install, place_clip, snap_time,
    zoom_step,
};

/// The timeline's box in the web's editor at 1440 by 900.
const BOX: Vec2 = vec2(1193.0, 277.0);
/// The margin kittest's panel keeps round it.
const MARGIN: f32 = 8.0;
const FPS: f64 = 30.0;

/// The loudest sample in each eighth of a second of the surf on the breakwater, from 0 to 1, as
/// cut's `peaksOf` makes it from its scene (`examples/cut/web/src/media.ts`).
fn surf_peaks() -> Arc<[f32]> {
    let peaks: Vec<f32> = serde_json::from_str(include_str!("fixtures/surf_peaks.json")).unwrap();
    peaks.into()
}

/// cut's `defaultTracks`: a title over the picture, the picture, and two of sound.
fn tracks() -> Vec<TimelineTrack> {
    vec![
        TimelineTrack::new("T1", "Titles", ClipTrackKind::Title),
        TimelineTrack::new("V1", "Picture", ClipTrackKind::Video),
        TimelineTrack::new("A1", "Sound 1", ClipTrackKind::Audio),
        TimelineTrack::new("A2", "Sound 2", ClipTrackKind::Audio),
    ]
}

/// The harbor project's clips (`seed.ts`), as cut's `timelineClip` hands them to the timeline: a
/// title's words, a source's name, color and length, and a sound's peaks.
fn harbor() -> Vec<TimelineClip> {
    let source = |id: &str, track: &str, start: f64, length: f64, in_point: f64, name: &str, full: f64| {
        TimelineClip::new(id, track, start, length, name)
            .in_point(in_point)
            .source_duration(full)
    };
    vec![
        TimelineClip::new("harbor-t1", "T1", 0.0, 4.0, "Harbor morning"),
        TimelineClip::new("harbor-t2", "T1", 20.0, 3.5, "High water at 06:40"),
        source("harbor-v1", "V1", 0.0, 8.0, 2.0, "Harbor at dawn", 24.0).color(KindColor::Blue),
        source("harbor-v2", "V1", 8.0, 6.0, 4.0, "Lighthouse beam", 18.0).color(KindColor::Violet),
        source("harbor-v3", "V1", 14.0, 6.0, 1.0, "Ferry route", 20.0).color(KindColor::Teal),
        source("harbor-v4", "V1", 20.0, 6.0, 5.0, "Tide chart", 12.0).color(KindColor::Blue),
        source("harbor-a1", "A1", 0.0, 26.0, 0.0, "Surf on the breakwater", 30.0)
            .color(KindColor::Teal)
            .waveform(surf_peaks()),
    ]
}

/// An editor's state: the edit, what is chosen, the playhead, and what was last said.
struct Editor {
    tracks: Vec<TimelineTrack>,
    clips: Vec<TimelineClip>,
    selected: Vec<String>,
    playhead: f64,
    said: Option<String>,
    deleted: Vec<String>,
    splits: Vec<(String, f64)>,
    changes: Vec<ClipChange>,
    /// A thing from the bin carried over the timeline.
    carrying: Option<DragData>,
    dropped: Vec<ClipDrop>,
}

impl Editor {
    fn harbor() -> Self {
        Self {
            tracks: tracks(),
            clips: harbor(),
            selected: Vec::new(),
            playhead: 0.0,
            said: None,
            deleted: Vec::new(),
            splits: Vec::new(),
            changes: Vec::new(),
            carrying: None,
            dropped: Vec::new(),
        }
    }
}

fn editor(state: Editor, scheme: Scheme) -> Harness<'static, Editor> {
    let appearance = Appearance {
        scheme,
        ..Appearance::default()
    };
    let mut harness = Harness::builder()
        .with_size(BOX + Vec2::splat(MARGIN * 2.0))
        .build_ui_state(
            move |ui, editor: &mut Editor| {
                install(ui.ctx(), &appearance);
                if let Some(data) = editor.carrying.take() {
                    egui::DragAndDrop::set_payload(ui.ctx(), data);
                }
                // cut's length: the edit's, five seconds on, at least 30
                let length = editor.clips.iter().map(|c| c.start + c.duration).fold(0.0, f64::max);
                let response = ClipTimeline::new(
                    "timeline",
                    &editor.tracks,
                    &editor.clips,
                    (length + 5.0).max(30.0),
                    editor.playhead,
                )
                .selected(&editor.selected)
                .changes(true)
                .splits(true)
                .deletes(true)
                .track_toggles(true)
                .drops(true)
                .accepts(|drag, track| {
                    drag.kind == "media" && (drag.value == "audio") == (track.kind == ClipTrackKind::Audio)
                })
                .fps(FPS)
                .show(ui);
                if let Some(at) = response.playhead {
                    editor.playhead = at;
                }
                if let Some(ids) = response.selected {
                    editor.selected = ids;
                }
                for (id, change) in response.track_changes {
                    if let Some(track) = editor.tracks.iter_mut().find(|t| t.id == id) {
                        match change {
                            TrackChange::Muted(on) => track.muted = on,
                            TrackChange::Hidden(on) => track.hidden = on,
                            TrackChange::Locked(on) => track.locked = on,
                        }
                    }
                }
                for change in &response.changes {
                    if let Some(clip) = editor.clips.iter_mut().find(|c| c.id == change.id) {
                        if let Some(start) = change.start {
                            clip.start = start;
                        }
                        if let Some(duration) = change.duration {
                            clip.duration = duration;
                        }
                        if let Some(in_point) = change.in_point {
                            clip.in_point = in_point;
                        }
                        if let Some(track) = &change.track {
                            clip.track = track.clone();
                        }
                    }
                }
                editor.changes.extend(response.changes);
                editor.splits.extend(response.splits);
                if let Some((ids, _)) = response.deleted {
                    editor.clips.retain(|c| !ids.contains(&c.id));
                    editor.deleted.extend(ids);
                }
                editor.dropped.extend(response.dropped);
                if response.said.is_some() {
                    editor.said = response.said;
                }
            },
            state,
        );
    harness.run_steps(3);
    harness
}

/// Where a time falls across the lanes, at the fit the shot draws.
fn x_at(time: f64) -> f32 {
    let lanes_left = MARGIN + 1.0 + 176.0;
    let room = BOX.x - 2.0 - 176.0;
    lanes_left + time as f32 * clip_fit_zoom(room, 31.0)
}

/// The middle of a track's lane, by its index from the top.
fn lane_y(index: usize) -> f32 {
    MARGIN + 1.0 + 32.0 + index as f32 * 56.0 + 28.0
}

#[test]
fn the_harbor_edit_draws_as_the_shot() {
    let mut harness = editor(Editor::harbor(), Scheme::Light);
    harness.snapshot("clip_timeline_harbor");
}

#[test]
fn a_chosen_clip_and_a_locked_track_in_the_dark() {
    let mut state = Editor::harbor();
    state.selected = vec!["harbor-v2".into()];
    state.playhead = 10.5;
    state.tracks[2].locked = true;
    state.tracks[0].hidden = true;
    let mut harness = editor(state, Scheme::Dark);
    harness.snapshot("clip_timeline_states");
}

#[test]
fn a_press_chooses_a_clip_and_s_splits_it_at_the_playhead() {
    let mut state = Editor::harbor();
    state.playhead = 10.0;
    let mut harness = editor(state, Scheme::Light);
    harness
        .get_by_label("Lighthouse beam, 00:00:08:00 to 00:00:14:00")
        .click();
    harness.run_steps(2);
    assert_eq!(harness.state().selected, vec!["harbor-v2".to_owned()]);
    harness.key_press(Key::S);
    harness.run_steps(2);
    assert_eq!(harness.state().splits, vec![("harbor-v2".to_owned(), 10.0)]);
    assert_eq!(
        harness.state().said.as_deref(),
        Some("Lighthouse beam split at 00:00:10:00")
    );
}

#[test]
fn the_arrows_step_the_playhead_and_delete_takes_the_chosen_clip() {
    let mut harness = editor(Editor::harbor(), Scheme::Light);
    harness.get_by_label("Ferry route, 00:00:14:00 to 00:00:20:00").click();
    harness.run_steps(2);
    harness.key_press(Key::ArrowRight);
    harness.run_steps(2);
    assert!((harness.state().playhead - 1.0 / FPS).abs() < 1e-9);
    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
    harness.run_steps(2);
    assert!((harness.state().playhead - (1.0 + 1.0 / FPS)).abs() < 1e-9);
    assert_eq!(harness.state().said.as_deref(), Some("Playhead at 00:00:01:01"));
    harness.key_press(Key::End);
    harness.run_steps(2);
    assert!((harness.state().playhead - 31.0).abs() < 1e-9);
    harness.key_press(Key::Delete);
    harness.run_steps(2);
    assert_eq!(harness.state().deleted, vec!["harbor-v3".to_owned()]);
    assert_eq!(harness.state().said.as_deref(), Some("Clip deleted"));
}

#[test]
fn a_press_on_the_ruler_moves_the_playhead() {
    let mut harness = editor(Editor::harbor(), Scheme::Light);
    let at = pos2(x_at(12.0), MARGIN + 1.0 + 16.0);
    harness.hover_at(at);
    harness.drag_at(at);
    harness.run_steps(2);
    harness.drop_at(at);
    harness.run_steps(2);
    assert!((harness.state().playhead - 12.0).abs() < 0.05);
}

/// Press at `from`, move to `to` in steps, let go.
fn drag(harness: &mut Harness<'static, Editor>, from: Pos2, to: Pos2) {
    harness.hover_at(from);
    harness.run_steps(1);
    harness.drag_at(from);
    harness.run_steps(1);
    for step in 1..=6 {
        harness.hover_at(from.lerp(to, step as f32 / 6.0));
        harness.run_steps(1);
    }
    harness.drop_at(to);
    harness.run_steps(2);
}

#[test]
fn a_dragged_clip_lands_clear_of_the_others() {
    let mut harness = editor(Editor::harbor(), Scheme::Light);
    // the second title, carried three seconds later along its track
    let from = pos2(x_at(21.0), lane_y(0));
    drag(&mut harness, from, pos2(x_at(24.0), lane_y(0)));
    let moved = harness
        .state()
        .clips
        .iter()
        .find(|c| c.id == "harbor-t2")
        .unwrap()
        .start;
    assert!((moved - 23.0).abs() < 0.1, "moved to {moved}");
    assert!(
        harness
            .state()
            .said
            .as_deref()
            .unwrap()
            .starts_with("High water at 06:40 moved to")
    );
    // a picture carried onto the next picture goes beside it
    let from = pos2(x_at(9.0), lane_y(1));
    drag(&mut harness, from, pos2(x_at(10.0), lane_y(1)));
    let clips = &harness.state().clips;
    let v2 = clips.iter().find(|c| c.id == "harbor-v2").unwrap();
    assert!(
        (v2.start - 8.0).abs() < 1e-6,
        "a move onto a neighbor lands back, at {}",
        v2.start
    );
}

#[test]
fn a_trimmed_end_stops_at_its_neighbor() {
    let mut harness = editor(Editor::harbor(), Scheme::Light);
    // the first title's end, pulled two seconds on
    let from = pos2(x_at(4.0) - 3.0, lane_y(0));
    drag(&mut harness, from, pos2(x_at(6.0) - 3.0, lane_y(0)));
    let t1 = harness.state().clips.iter().find(|c| c.id == "harbor-t1").unwrap();
    assert!((t1.duration - 6.0).abs() < 0.1, "trimmed to {}", t1.duration);
    // the last picture's end, pulled past its source's end, stops there
    let from = pos2(x_at(26.0) - 3.0, lane_y(1));
    drag(&mut harness, from, pos2(x_at(30.0), lane_y(1)));
    let v4 = harness.state().clips.iter().find(|c| c.id == "harbor-v4").unwrap();
    assert!(
        (v4.duration - 7.0).abs() < 1e-6,
        "the source's end holds it at {}",
        v4.duration
    );
}

#[test]
fn a_sound_from_the_bin_lands_on_a_sound_track_clear_of_its_clips() {
    let mut harness = editor(Editor::harbor(), Scheme::Light);
    let sound = DragData {
        kind: "media".into(),
        label: "Rain on the roof".into(),
        duration: Some(16.0),
        value: "audio".into(),
    };
    // over the second sound track at ten seconds, then let go there
    let at = pos2(x_at(10.0), lane_y(3));
    harness.state_mut().carrying = Some(sound.clone());
    harness.hover_at(at);
    harness.run_steps(1);
    harness.drop_at(at);
    harness.run_steps(2);
    let dropped = &harness.state().dropped;
    assert_eq!(dropped.len(), 1);
    assert_eq!(dropped[0].track, "A2");
    assert!((dropped[0].at - 10.0).abs() < 0.05, "at {}", dropped[0].at);
    assert!(
        harness
            .state()
            .said
            .as_deref()
            .unwrap()
            .starts_with("Rain on the roof added to Sound 2 at")
    );
    // a picture track takes no sound
    harness.state_mut().carrying = Some(sound);
    let at = pos2(x_at(10.0), lane_y(1));
    harness.hover_at(at);
    harness.run_steps(1);
    harness.drop_at(at);
    harness.run_steps(2);
    assert_eq!(harness.state().dropped.len(), 1);
}

#[test]
fn a_header_toggle_locks_its_track() {
    let mut harness = editor(Editor::harbor(), Scheme::Light);
    harness.get_all_by_label("Lock").nth(1).unwrap().click();
    harness.run_steps(2);
    assert!(harness.state().tracks[1].locked);
}

#[test]
fn the_helpers_answer_as_sokkel_ui_does() {
    assert_eq!(format_timecode(4.0, 30.0), "00:00:04:00");
    assert_eq!(format_timecode(9.5, 30.0), "00:00:09:15");
    assert_eq!(format_timecode(3725.0 + 1.0 / 30.0, 30.0), "01:02:05:01");
    assert_eq!(format_timecode(1.0, 120.0), "00:00:01:000");
    assert_eq!(place_clip(5.0, 2.0, &[(0.0, 4.0), (8.0, 2.0)]), Some(5.0));
    assert_eq!(place_clip(3.0, 2.0, &[(0.0, 4.0), (8.0, 2.0)]), Some(4.0));
    assert_eq!(place_clip(1.0, 2.0, &[(0.0, 10.0)]), None);
    assert_eq!(snap_time(1.02, &[1.0], 0.1, 30.0), 1.0);
    assert!((snap_time(1.52, &[1.0], 0.1, 30.0) - 46.0 / 30.0).abs() < 1e-9);
    assert_eq!(clip_tick_step(32.7, 30.0), 5.0);
    assert!((clip_tick_step(600.0, 30.0) - 5.0 / 30.0).abs() < 1e-9);
    assert_eq!(clip_fit_zoom(640.0, 31.0), 640.0 / 31.0);
    assert_eq!(clip_fit_zoom(320.0, 31.0), 16.0);
    assert_eq!(clip_fit_zoom(320.0, 600.0), 960.0 / 600.0);
    assert!((zoom_step(16.0, 1) - 20.0).abs() < 1e-6);
    assert_eq!(zoom_step(1000.0, 1), 600.0);
}

#[test]
fn a_clip_in_hand_shows_where_it_lands() {
    let mut harness = editor(Editor::harbor(), Scheme::Light);
    // the second title carried toward the first, its landing blocked over it
    let from = pos2(x_at(21.0), lane_y(0));
    let to = pos2(x_at(3.0), lane_y(0));
    harness.hover_at(from);
    harness.run_steps(1);
    harness.drag_at(from);
    harness.run_steps(1);
    for step in 1..=6 {
        harness.hover_at(from.lerp(to, step as f32 / 6.0));
        harness.run_steps(1);
    }
    harness.snapshot("clip_timeline_dragging");
}
