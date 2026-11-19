//! The editor, as the web's `editor/EditorPage.tsx`: the media bin, the preview with its transport
//! and the inspector over the timeline. Every change is a step of undo and is saved a moment after
//! it is made, and once more as the editor closes; the keys play, step, cut and zoom.

use eframe::egui::{self, Key, Modifiers, Ui};
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, CLIP_TIMELINE_ZOOM_MAX, CLIP_TIMELINE_ZOOM_MIN, ClipTimeline, ClipTimelineWords,
    ClipTrackKind, Icon, IconButton, PageHeader, PaneHeader, PaneHeaderVariant, SplitPane, TimelineClip, TimelineTrack,
    Toast, ToastKind, zoom_step,
};

use crate::backend::{Backend, Notice, SAVE_AFTER};
use crate::editor::{
    NEW_TITLE, NO_TITLE_TRACK, Place, chosen_under, no_track_for, place_from, present, timeline_clip, timeline_duration,
};
use crate::history::EditHistory;
use crate::media::{KindColor, Source, source_of};
use crate::model::{
    ClipChange, ClipIds, ClipPatch, Edit, FPS, Prefs, Project, TrackChange, TrackKind, add_source, add_title,
    apply_changes, delete_clips, drop_source, length_of, patch_clip, set_track, split_all, split_clip,
};
use crate::playback::Playback;

/// What the editor's header asked of the app this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct EditorAsked {
    pub rename: bool,
    pub duplicate: bool,
    pub delete: bool,
}

/// The export dialog while it is open: the file's name and the resolution chosen.
#[derive(Clone, Debug)]
pub struct ExportView {
    pub file: String,
    /// The size chosen, by its place in `ExportSize::ALL`.
    pub size: usize,
}

/// One project open in the editor.
pub struct EditorView {
    /// The project's id.
    pub project: String,
    pub history: EditHistory,
    /// The edit as last saved, and when it last changed.
    saved: Edit,
    changed_at: i64,
    pub playback: Playback,
    /// The edit the playhead last saw, so a change restarts its sound.
    seen: Edit,
    /// The clips chosen, by id.
    pub selected: Vec<String>,
    pub exporting: Option<ExportView>,
    /// The preview's level from 0 to 1, and whether it is muted.
    pub level: f32,
    pub muted: bool,
    /// How wide a second is drawn; none fits the edit to the timeline, whose fit is `fit`.
    pub zoom: Option<f32>,
    pub fit: f32,
    /// The focus was inside the timeline last frame, which answers its own keys.
    timeline_focused: bool,
    /// The export size the settings name, which the dialog starts on.
    export_size: crate::model::ExportSize,
}

/// The kind color a clip wears, as the library names it.
pub fn kind_color(color: KindColor) -> sokkel_egui::KindColor {
    match color {
        KindColor::Blue => sokkel_egui::KindColor::Blue,
        KindColor::Green => sokkel_egui::KindColor::Green,
        KindColor::Jade => sokkel_egui::KindColor::Jade,
        KindColor::Pink => sokkel_egui::KindColor::Pink,
        KindColor::Teal => sokkel_egui::KindColor::Teal,
        KindColor::Violet => sokkel_egui::KindColor::Violet,
    }
}

fn track_kind(kind: TrackKind) -> ClipTrackKind {
    match kind {
        TrackKind::Video => ClipTrackKind::Video,
        TrackKind::Audio => ClipTrackKind::Audio,
        TrackKind::Title => ClipTrackKind::Title,
    }
}

/// The edit's tracks as the timeline draws them.
fn timeline_tracks(edit: &Edit) -> Vec<TimelineTrack> {
    edit.tracks
        .iter()
        .map(|track| {
            TimelineTrack::new(&track.id, &track.name, track_kind(track.kind))
                .muted(track.is_muted())
                .locked(track.is_locked())
                .hidden(track.is_hidden())
        })
        .collect()
}

/// The edit's clips as the timeline draws them: each its name, its source's color and length, and
/// a sound's peaks.
fn timeline_clips(edit: &Edit) -> Vec<TimelineClip> {
    edit.clips
        .iter()
        .map(|clip| {
            let one = timeline_clip(clip);
            let mut drawn =
                TimelineClip::new(one.id, one.track, one.start, one.duration, one.label).in_point(one.in_point);
            if let Some(length) = one.source_duration {
                drawn = drawn.source_duration(length);
            }
            if let Some(color) = one.color {
                drawn = drawn.color(kind_color(color));
            }
            if let Some(peaks) = one.waveform {
                drawn = drawn.waveform(peaks.iter().map(|p| *p as f32).collect::<Vec<f32>>());
            }
            drawn
        })
        .collect()
}

/// A warning in a toast, as the web's `toast.add({ type: 'warning' })`.
fn warn(ctx: &egui::Context, title: impl Into<String>) {
    Toast::new(title).kind(ToastKind::Warning).show(ctx);
}

impl EditorView {
    pub fn new(project: &Project, prefs: &Prefs, now: i64) -> Self {
        let edit = project.edit.clone();
        let length = length_of(&edit);
        Self {
            project: project.id.clone(),
            history: EditHistory::new(edit.clone()),
            saved: edit.clone(),
            changed_at: now,
            playback: Playback::new(length, prefs.looping),
            seen: edit,
            selected: Vec::new(),
            exporting: None,
            level: 1.0,
            muted: false,
            zoom: None,
            fit: 40.0,
            timeline_focused: false,
            export_size: prefs.export_size,
        }
    }

    pub fn edit(&self) -> &Edit {
        self.history.edit()
    }

    /// Whether something waits on the clock: a change not saved yet, or the playhead playing.
    pub fn waiting(&self) -> bool {
        self.edit() != &self.saved || self.playback.playing()
    }

    /// Makes a change, a step of undo unless it joins the last under the same `merge` key.
    pub fn apply(&mut self, next: Option<Edit>, merge: Option<&str>, now: i64) {
        self.history.apply(next, merge, now);
        self.changed_at = now;
    }

    pub fn undo(&mut self, now: i64) {
        self.history.undo();
        self.changed_at = now;
    }

    pub fn redo(&mut self, now: i64) {
        self.history.redo();
        self.changed_at = now;
    }

    /// What the clock moves on each frame: the edit saved a moment after it last changed, the
    /// chosen clips held to the edit, and the playhead moved while it plays. The sound it cues has
    /// nowhere to play here and is let go.
    pub fn tick(&mut self, backend: &mut Backend, now: i64) -> Result<(), Notice> {
        self.selected = present(self.edit(), &self.selected);
        let edit = self.edit().clone();
        let changed = edit != self.seen;
        self.playback
            .update(now as f64, length_of(&edit), backend.prefs.looping, changed);
        self.seen = edit;
        self.playback.frame(now as f64);
        // the cues would start and stop the sound; with no output for it here they are let go
        let _ = self.playback.take_cues();
        self.export_size = backend.prefs.export_size;
        if self.edit() != &self.saved && now - self.changed_at >= SAVE_AFTER {
            return self.save_now(backend, now).map(|()| {
                self.saved = self.edit().clone();
            });
        }
        Ok(())
    }

    /// Keeps the edit on the backend now, when it changed since it was last kept.
    pub fn save_now(&self, backend: &mut Backend, now: i64) -> Result<(), Notice> {
        if self.edit() == &self.saved {
            return Ok(());
        }
        backend.save_edit(&self.project, self.edit(), now)
    }

    /// Places a source from the bin at the playhead or at the end of its track, chosen.
    pub fn place(&mut self, ctx: &egui::Context, source: &Source, place: Place, ids: &mut ClipIds, now: i64) {
        let at = place_from(self.edit(), source, place, self.playback.playhead);
        match add_source(self.edit(), source, at, ids) {
            None => warn(ctx, no_track_for(source.kind)),
            Some((edit, id)) => {
                self.apply(Some(edit), None, now);
                self.selected = vec![id];
            }
        }
    }

    /// Adds a title at the playhead, chosen.
    pub fn title(&mut self, ctx: &egui::Context, prefs: &Prefs, ids: &mut ClipIds, now: i64) {
        match add_title(self.edit(), self.playback.playhead, NEW_TITLE, prefs.title_length, ids) {
            None => warn(ctx, NO_TITLE_TRACK),
            Some((edit, id)) => {
                self.apply(Some(edit), None, now);
                self.selected = vec![id];
            }
        }
    }

    /// Cuts the chosen clips, or every clip when none is chosen, at the playhead.
    pub fn split(&mut self, ids: &mut ClipIds, now: i64) {
        let next = split_all(self.edit(), self.playback.playhead, &self.selected, ids);
        self.apply(next, None, now);
    }

    /// Takes the chosen clips off, closing the gaps they leave with `ripple`.
    pub fn remove(&mut self, ripple: bool, now: i64) {
        if self.selected.is_empty() {
            return;
        }
        let next = delete_clips(self.edit(), &self.selected, ripple);
        self.apply(Some(next), None, now);
        self.selected.clear();
    }

    /// Changes one field of a clip; typing into one field is one step of undo.
    pub fn patch(&mut self, id: &str, patch: &ClipPatch, field: &str, now: i64) {
        let next = patch_clip(self.edit(), id, patch);
        self.apply(Some(next), Some(&format!("{id}:{field}")), now);
    }

    fn zoom_by(&mut self, steps: i32) {
        self.zoom = Some(zoom_step(self.zoom.unwrap_or(self.fit), steps));
    }

    /// The editor's keys, as the web's `useShortcuts` list: play and step, undo, cut and delete,
    /// add a title, and zoom. Keys the timeline answers itself stand aside while it has the focus.
    pub fn shortcuts(&mut self, ctx: &egui::Context, prefs: &Prefs, ids: &mut ClipIds, now: i64) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let ms = now as f64;
        let own = !self.timeline_focused;
        let length = length_of(self.edit());
        let shift = Modifiers::SHIFT;
        let command = Modifiers::COMMAND;
        let keys = ctx.input_mut(|i| {
            let mut pressed = Vec::new();
            let mut take = |i: &mut egui::InputState, modifiers: Modifiers, key: Key, name: &'static str| {
                if i.consume_key(modifiers, key) {
                    pressed.push(name);
                }
            };
            take(i, command | shift, Key::Z, "redo");
            take(i, command, Key::Z, "undo");
            take(i, Modifiers::NONE, Key::Space, "play");
            take(i, Modifiers::NONE, Key::L, "forward");
            take(i, Modifiers::NONE, Key::K, "pause");
            take(i, Modifiers::NONE, Key::J, "backward");
            take(i, Modifiers::NONE, Key::T, "title");
            take(i, Modifiers::NONE, Key::Equals, "zoom-in");
            take(i, Modifiers::NONE, Key::Plus, "zoom-in");
            take(i, Modifiers::NONE, Key::Minus, "zoom-out");
            take(i, Modifiers::NONE, Key::Num0, "zoom-fit");
            take(i, Modifiers::NONE, Key::Escape, "none");
            if own {
                take(i, shift, Key::ArrowLeft, "second-back");
                take(i, shift, Key::ArrowRight, "second-on");
                take(i, Modifiers::NONE, Key::ArrowLeft, "frame-back");
                take(i, Modifiers::NONE, Key::ArrowRight, "frame-on");
                take(i, Modifiers::NONE, Key::Home, "start");
                take(i, Modifiers::NONE, Key::End, "end");
                take(i, Modifiers::NONE, Key::S, "split");
                take(i, shift, Key::Delete, "ripple");
                take(i, Modifiers::NONE, Key::Delete, "delete");
                take(i, Modifiers::NONE, Key::Backspace, "delete");
            }
            pressed
        });
        for key in keys {
            match key {
                "play" => self.playback.toggle(ms),
                "forward" => self.playback.forward(ms),
                "pause" => self.playback.pause(ms),
                "backward" => self.playback.backward(ms),
                "frame-back" => self.playback.step(-1.0, ms),
                "frame-on" => self.playback.step(1.0, ms),
                "second-back" => self.playback.step(-30.0, ms),
                "second-on" => self.playback.step(30.0, ms),
                "start" => self.playback.seek(0.0, ms),
                "end" => self.playback.seek(length, ms),
                "undo" => self.undo(now),
                "redo" => self.redo(now),
                "split" => self.split(ids, now),
                "delete" => self.remove(prefs.ripple_by_default, now),
                "ripple" => self.remove(true, now),
                "title" => self.title(ctx, prefs, ids, now),
                "zoom-in" => self.zoom_by(1),
                "zoom-out" => self.zoom_by(-1),
                "zoom-fit" => self.zoom = None,
                "none" => self.selected.clear(),
                _ => {}
            }
        }
    }

    /// The timeline's pane: its head with the cut, delete and zoom buttons over the timeline.
    fn timeline(&mut self, ui: &mut Ui, prefs: &Prefs, ids: &mut ClipIds, now: i64) {
        let words = ClipTimelineWords::default();
        let pixels = self.zoom.unwrap_or(self.fit);
        let under = chosen_under(self.edit(), &self.selected, self.playback.playhead);
        let none = self.selected.is_empty();
        let (mut split, mut delete, mut ripple, mut out, mut into, mut fit) =
            (false, false, false, false, false, false);
        PaneHeader::new()
            .title("Timeline")
            .variant(PaneHeaderVariant::Panel)
            .show(ui, |ui| {
                fit = ui
                    .add_enabled(self.zoom.is_some(), IconButton::new(Icon::Maximize2, &words.zoom_fit))
                    .clicked();
                into = ui
                    .add_enabled(
                        pixels < CLIP_TIMELINE_ZOOM_MAX,
                        IconButton::new(Icon::ZoomIn, &words.zoom_in),
                    )
                    .clicked();
                out = ui
                    .add_enabled(
                        pixels > CLIP_TIMELINE_ZOOM_MIN,
                        IconButton::new(Icon::ZoomOut, &words.zoom_out),
                    )
                    .clicked();
                ripple = ui
                    .add_enabled(!none, IconButton::new(Icon::ListX, "Ripple delete"))
                    .clicked();
                delete = ui.add_enabled(!none, IconButton::new(Icon::Trash2, "Delete")).clicked();
                split = ui
                    .add_enabled(under, IconButton::new(Icon::Scissors, "Split at the playhead"))
                    .clicked();
            });
        if split {
            self.split(ids, now);
        }
        if delete {
            self.remove(prefs.ripple_by_default, now);
        }
        if ripple {
            self.remove(true, now);
        }
        if out {
            self.zoom_by(-1);
        }
        if into {
            self.zoom_by(1);
        }
        if fit {
            self.zoom = None;
        }

        let edit = self.edit().clone();
        let tracks = timeline_tracks(&edit);
        let clips = timeline_clips(&edit);
        let length = length_of(&edit);
        let selected = self.selected.clone();
        let response = egui::Frame::new()
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ClipTimeline::new(
                    "cut-timeline",
                    &tracks,
                    &clips,
                    timeline_duration(length),
                    self.playback.playhead,
                )
                .selected(&selected)
                .changes(true)
                .splits(true)
                .deletes(true)
                .track_toggles(true)
                .drops(true)
                .accepts(|drag, track| {
                    drag.kind == "media"
                        && source_of(Some(&drag.value)).is_some_and(|s| track_kind(s.kind.track()) == track.kind)
                })
                .zoom(self.zoom)
                .zoom_asks(true)
                .fps(FPS)
                .snap(prefs.snap)
                .show(ui)
            })
            .inner;
        self.timeline_focused = response.focused;
        self.fit = response.fit;
        let ms = now as f64;
        if let Some(at) = response.playhead {
            self.playback.seek(at, ms);
        }
        if let Some(chosen) = response.selected {
            self.selected = chosen;
        }
        if !response.changes.is_empty() {
            let changes: Vec<ClipChange> = response
                .changes
                .into_iter()
                .map(|c| ClipChange {
                    id: c.id,
                    start: c.start,
                    duration: c.duration,
                    in_point: c.in_point,
                    track: c.track,
                })
                .collect();
            let next = apply_changes(self.edit(), &changes);
            self.apply(next, None, now);
        }
        for (id, at) in response.splits {
            let next = split_clip(self.edit(), &id, at, ids).map(|(edit, _)| edit);
            self.apply(next, Some(&format!("split:{at}")), now);
        }
        if let Some((gone, ripple)) = response.deleted {
            let next = delete_clips(self.edit(), &gone, ripple || prefs.ripple_by_default);
            self.apply(Some(next), None, now);
            self.selected.clear();
        }
        for (id, change) in response.track_changes {
            let change = match change {
                sokkel_egui::TrackChange::Muted(on) => TrackChange {
                    muted: Some(on),
                    ..TrackChange::default()
                },
                sokkel_egui::TrackChange::Hidden(on) => TrackChange {
                    hidden: Some(on),
                    ..TrackChange::default()
                },
                sokkel_egui::TrackChange::Locked(on) => TrackChange {
                    locked: Some(on),
                    ..TrackChange::default()
                },
            };
            let next = set_track(self.edit(), &id, &change);
            self.apply(Some(next), None, now);
        }
        if let Some(zoom) = response.zoom {
            self.zoom = Some(zoom);
        }
        if let Some(drop) = response.dropped
            && let Some(source) = source_of(Some(&drop.data.value))
            && let Some((edit, id)) = drop_source(self.edit(), source, &drop.track, drop.at, ids)
        {
            self.apply(Some(edit), None, now);
            self.selected = vec![id];
        }
    }
}

/// The editor's page: its header with undo, redo, the project's own actions and Export, over the
/// bin, the preview and the inspector, over the timeline.
pub fn editor_page(
    ui: &mut Ui,
    view: &mut EditorView,
    name: &str,
    prefs: &Prefs,
    ids: &mut ClipIds,
    now: i64,
) -> EditorAsked {
    let ctx = ui.ctx().clone();
    let mut asked = EditorAsked::default();
    let (can_undo, can_redo) = (view.history.can_undo(), view.history.can_redo());
    let length = length_of(view.edit());
    let (mut undo, mut redo, mut export) = (false, false, false);
    PageHeader::new(name).show(ui, |ui| {
        let item = |label: &str, icon: Icon| {
            Button::new(label)
                .icon(icon)
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Sm)
        };
        let mut primary = Button::new("Export").icon(Icon::Download).size(ButtonSize::Sm);
        if length == 0.0 {
            primary = primary.reason("The edit has nothing in it to export.");
        }
        export = ui.add(primary).clicked();
        asked.delete = ui.add(item("Delete", Icon::Trash2)).clicked();
        asked.duplicate = ui.add(item("Duplicate", Icon::Copy)).clicked();
        asked.rename = ui.add(item("Rename", Icon::PencilLine)).clicked();
        let mut redo_button = item("Redo", Icon::Redo2);
        if !can_redo {
            redo_button = redo_button.reason("Nothing to redo.");
        }
        redo = ui.add(redo_button).clicked();
        let mut undo_button = item("Undo", Icon::Undo2);
        if !can_undo {
            undo_button = undo_button.reason("Nothing to undo.");
        }
        undo = ui.add(undo_button).clicked();
    });
    if undo {
        view.undo(now);
    }
    if redo {
        view.redo(now);
    }
    if export {
        view.exporting = Some(ExportView {
            file: crate::export::default_file_name(name),
            size: crate::model::ExportSize::ALL
                .iter()
                .position(|s| *s == view.export_size)
                .unwrap_or(0),
        });
    }
    let bin_asked = std::cell::Cell::new(crate::bin_pane::BinAsked::default());
    // the panes are drawn by closures each of which may change the edit
    let view = std::cell::RefCell::new(view);
    let ids = std::cell::RefCell::new(ids);
    SplitPane::new("cut-rows")
        .down(true)
        .default_share(0.52)
        .min_share(0.35)
        .max_share(0.8)
        .show(
            ui,
            |ui| {
                SplitPane::new("cut-bin")
                    .default_share(0.24)
                    .min_share(0.15)
                    .max_share(0.4)
                    .narrow_below(720.0)
                    .show(
                        ui,
                        |ui| bin_asked.set(crate::bin_pane::bin_pane(ui)),
                        |ui| {
                            SplitPane::new("cut-inspector")
                                .default_share(0.66)
                                .min_share(0.45)
                                .max_share(0.8)
                                .narrow_below(720.0)
                                .show(
                                    ui,
                                    |ui| crate::preview::preview(ui, &mut view.borrow_mut(), now),
                                    |ui| {
                                        crate::inspector::inspector(
                                            ui,
                                            &mut view.borrow_mut(),
                                            &mut ids.borrow_mut(),
                                            now,
                                        )
                                    },
                                );
                        },
                    );
            },
            |ui| view.borrow_mut().timeline(ui, prefs, &mut ids.borrow_mut(), now),
        );
    let view = view.into_inner();
    let ids = ids.into_inner();
    let bin = bin_asked.take();
    if let Some(source) = bin.add {
        view.place(&ctx, source, Place::Playhead, ids, now);
    }
    if let Some(source) = bin.append {
        view.place(&ctx, source, Place::End, ids, now);
    }
    if bin.title {
        view.title(&ctx, prefs, ids, now);
    }
    asked
}
