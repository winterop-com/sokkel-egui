//! The inspector beside the preview, as the web's `editor/ClipInspector.tsx`: the chosen clip's
//! name, text, in and out points, start, duration and volume, each change one step of undo; several
//! chosen, each one's start and length; none, a summary of the edit.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, DescriptionList, Icon, NumberField, PaneHeader, PaneHeaderVariant, SliderField,
    TextField, inspector_section,
};

use crate::editor::{
    any_chosen_under, chosen_list, clip_about, edit_summary, in_point_patch, inspector_title, out_point_patch,
    parse_time, time_bounds, volume_percent, within,
};
use crate::editor_page::EditorView;
use crate::model::{Clip, ClipIds, ClipPatch, FRAME, timecode, track_of};

/// A term's column beside its value: the web's `6rem` and the 12 between them.
const TERM: f32 = 108.0;

/// A time field written and read as a timecode, a frame a step; the time typed when it is within
/// its bounds.
fn time_field(
    ui: &mut Ui,
    id: &str,
    label: &str,
    value: f64,
    (min, max): (Option<f64>, Option<f64>),
    enabled: bool,
) -> Option<f64> {
    let mut typed = value;
    let response = ui.add_enabled(
        enabled,
        NumberField::new(id, label, &mut typed)
            .step(FRAME)
            .mono(true)
            .format(timecode)
            .parse(parse_time),
    );
    (response.changed() && typed != value && within(typed, min, max)).then_some(typed)
}

/// The changes the chosen clip's fields asked for, each with the field it came from.
fn clip_fields(ui: &mut Ui, view: &EditorView, clip: &Clip) -> Vec<(ClipPatch, &'static str)> {
    let mut patches = Vec::new();
    let source = clip.source();
    let locked = track_of(view.edit(), &clip.track).is_some_and(|t| t.is_locked());
    let bounds = time_bounds(clip);
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 24.0;
        inspector_section(ui, "Clip", |ui| {
            let mut name = clip.name.clone().unwrap_or_default();
            let field = TextField::new("Name", &mut name).placeholder(source.map_or("Title", |s| s.name));
            if ui.add_enabled(!locked, field).changed() {
                patches.push((
                    ClipPatch {
                        name: Some(name),
                        ..ClipPatch::default()
                    },
                    "name",
                ));
            }
            if let Some(text) = &clip.text {
                let mut text = text.clone();
                let field = TextField::new("Text", &mut text)
                    .multiline(true)
                    .help("Written over the picture for the title's time.");
                if ui.add_enabled(!locked, field).changed() {
                    patches.push((
                        ClipPatch {
                            text: Some(text),
                            ..ClipPatch::default()
                        },
                        "text",
                    ));
                }
            }
        });
        inspector_section(ui, "Timing", |ui| {
            if let Some(start) = time_field(ui, "clip-start", "Start", clip.start, bounds.start, !locked) {
                patches.push((
                    ClipPatch {
                        start: Some(start),
                        ..ClipPatch::default()
                    },
                    "start",
                ));
            }
            if source.is_some() {
                if let Some(at) = time_field(ui, "clip-in", "In point", clip.in_point, bounds.in_point, !locked) {
                    patches.push((in_point_patch(clip, at), "in"));
                }
                let out = clip.in_point + clip.duration;
                if let Some(at) = time_field(ui, "clip-out", "Out point", out, bounds.out_point, !locked) {
                    patches.push((out_point_patch(clip, at), "out"));
                }
            }
            if let Some(duration) = time_field(ui, "clip-duration", "Duration", clip.duration, bounds.duration, !locked)
            {
                patches.push((
                    ClipPatch {
                        duration: Some(duration),
                        ..ClipPatch::default()
                    },
                    "duration",
                ));
            }
        });
        if source.is_some_and(|s| s.kind == crate::media::SourceKind::Audio) {
            inspector_section(ui, "Sound", |ui| {
                let was = volume_percent(clip);
                let mut volume = was;
                ui.add(SliderField::new("Volume", &mut volume, 100.0).unit("%"));
                if volume != was && !locked {
                    patches.push((
                        ClipPatch {
                            volume: Some(volume / 100.0),
                            ..ClipPatch::default()
                        },
                        "volume",
                    ));
                }
            });
        }
        inspector_section(ui, "About", |ui| {
            let mut list = DescriptionList::new().side(TERM);
            for (term, value) in clip_about(view.edit(), clip) {
                list = list.text(term, value);
            }
            list.show(ui);
        });
    });
    patches
}

pub fn inspector(ui: &mut Ui, view: &mut EditorView, ids: &mut ClipIds, now: i64) {
    let edit = view.edit().clone();
    let chosen: Vec<&Clip> = edit.clips.iter().filter(|c| view.selected.contains(&c.id)).collect();
    let under = any_chosen_under(&edit, &view.selected, view.playback.playhead);
    let (mut split, mut delete) = (false, false);
    PaneHeader::new()
        .title(inspector_title(&edit, &view.selected))
        .variant(PaneHeaderVariant::Panel)
        .show(ui, |ui| {
            if chosen.is_empty() {
                return;
            }
            let button = |label: &str, icon: Icon| {
                Button::new(label)
                    .icon(icon)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Sm)
            };
            delete = ui.add(button("Delete", Icon::Trash2)).clicked();
            split = ui.add_enabled(under, button("Split", Icon::Scissors)).clicked();
        });
    let mut patches = Vec::new();
    egui::ScrollArea::vertical()
        .id_salt("cut-inspector")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::new().inner_margin(egui::Margin::same(12)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                match chosen[..] {
                    [clip] => {
                        let clip = clip.clone();
                        ui.push_id(&clip.id, |ui| {
                            patches = clip_fields(ui, view, &clip)
                                .into_iter()
                                .map(|(patch, field)| (clip.id.clone(), patch, field))
                                .collect();
                        });
                    }
                    [] => inspector_section(ui, "Summary", |ui| {
                        let mut list = DescriptionList::new().side(TERM);
                        for (term, value) in edit_summary(&edit) {
                            list = list.text(term, value);
                        }
                        list.show(ui);
                    }),
                    _ => {
                        let mut list = DescriptionList::new().side(TERM);
                        for (term, value) in chosen_list(&edit, &view.selected) {
                            list = list.text(term, value);
                        }
                        list.show(ui);
                    }
                }
            });
        });
    for (id, patch, field) in patches {
        view.patch(&id, &patch, field, now);
    }
    if split {
        view.split(ids, now);
    }
    if delete {
        view.remove(false, now);
    }
}
