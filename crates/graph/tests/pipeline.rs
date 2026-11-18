//! The pipeline graph as the studio draws it: the nightly-export pipeline at the size of the web
//! shot's graph pane, a step chosen, the block picker open after a step, a loop refused, and a run
//! part way through; and the editor's gestures, each reported.

use egui::{Pos2, Vec2, vec2};
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::{Icon, install};
use sokkel_egui_graph::{
    BlockGroup, BlockPicker, BlockPickerWords, GraphAnchor, GraphControlButton, GraphEditing, GraphEvent, GraphNotice,
    GraphOverlay, GraphToolbar, GraphWords, NoticeTone, PipelineBlock, PipelineGraph, PipelineStep, RunGraph, RunStep,
    cycle_path,
};

/// The graph's pane in the web's studio at 1440 by 900 (`parity/studio/web/studio_pipeline.png`).
const STAGE: Vec2 = vec2(535.0, 427.0);
/// The margin kittest's panel keeps round the stage.
const MARGIN: f32 = 8.0;

/// The kinds of step, each with its glyph, as the studio's `KIND_ICONS`.
const KINDS: [(&str, Icon); 5] = [
    ("fetch", Icon::Download),
    ("transform", Icon::Shuffle),
    ("validate", Icon::ShieldCheck),
    ("upload", Icon::Upload),
    ("notify", Icon::Bell),
];

fn icon(kind: &str) -> Option<Icon> {
    KINDS.iter().find(|(k, _)| *k == kind).map(|(_, i)| *i)
}

/// The nightly-export pipeline's five steps, one after another.
fn nightly_export() -> Vec<PipelineStep> {
    let mut before: Option<&str> = None;
    KINDS
        .iter()
        .map(|(kind, glyph)| {
            let step = PipelineStep::new(*kind)
                .kind(*kind)
                .icon(Some(*glyph))
                .needs(before.into_iter().map(str::to_owned));
            before = Some(kind);
            step
        })
        .collect()
}

/// The blocks a step is made from, as the studio's picker lists them.
fn blocks() -> (Vec<PipelineBlock>, Vec<BlockGroup>) {
    let docs = [
        (
            "fetch",
            "read",
            "Reads rows from an address on example.org, as CSV, JSON or NDJSON.",
        ),
        (
            "transform",
            "shape",
            "Reshapes the rows it is given with a jq or SQL script.",
        ),
        (
            "validate",
            "shape",
            "Checks each row against rules, failing, skipping or warning on the rows it refuses.",
        ),
        ("upload", "write", "Writes the rows to a path in a bucket."),
        (
            "notify",
            "write",
            "Posts to a channel when the run ends in one of the states given.",
        ),
    ];
    let blocks = docs
        .iter()
        .map(|(kind, group, doc)| {
            let mut block = PipelineBlock::new(*kind, *group).description(*doc);
            if let Some(glyph) = icon(kind) {
                block = block.icon(glyph);
            }
            block
        })
        .collect();
    let groups = vec![
        BlockGroup::new("read", "Read"),
        BlockGroup::new("shape", "Shape"),
        BlockGroup::new("write", "Write"),
    ];
    (blocks, groups)
}

/// What the studio's canvas holds over its graph.
#[derive(Clone, Default)]
struct Studio {
    steps: Vec<PipelineStep>,
    selected: Option<String>,
    picker_after: Option<String>,
    notice: Option<(NoticeTone, String)>,
    events: Vec<GraphEvent>,
    picked: Option<String>,
}

/// The studio's pipeline canvas: the steps on a stage that edits them, the toolbar with undo, redo
/// and the problems, the add button in the corner, the picker after a step and a notice.
fn studio(state: Studio) -> Harness<'static, Studio> {
    Harness::builder()
        .with_size(STAGE + Vec2::splat(2.0 * MARGIN))
        .build_ui_state(
            |ui, studio: &mut Studio| {
                install(ui.ctx(), &Default::default());
                let steps = studio.steps.clone();
                let can_connect = |source: &str, target: &str| -> Option<String> {
                    let into = steps.iter().find(|s| s.code == target)?;
                    if into.needs.iter().any(|n| n == source) {
                        return Some(format!("{target} already comes after {source}."));
                    }
                    cycle_path(&steps, source, target).map(|path| {
                        format!(
                            "That makes a loop: {}.",
                            [path, vec![target.to_owned()]].concat().join(" -> ")
                        )
                    })
                };
                let add = [GraphControlButton::new(Icon::Plus, "Add a step")];
                let editing = GraphEditing {
                    append: true,
                    insert: true,
                    moves: true,
                    drop: true,
                    pane_menu: true,
                    ..GraphEditing::new()
                }
                .can_connect(&can_connect);
                let shown = PipelineGraph::new("nightly-export", &studio.steps)
                    .size(STAGE)
                    .selected(studio.selected.as_deref())
                    .editing(Some(editing))
                    .extra_controls(&add)
                    .toolbar(Some(
                        GraphToolbar::new()
                            .history(false, false)
                            .zoom(false)
                            .issues(Some(0), false),
                    ))
                    .empty_action("Add step", Some(Icon::Plus))
                    .words(GraphWords {
                        label: "nightly-export".into(),
                        empty: "No steps yet".into(),
                        ..Default::default()
                    })
                    .show(ui);
                if let Some(code) = shown.pressed {
                    studio.selected = Some(code);
                }
                if shown.pane_pressed {
                    studio.selected = None;
                }
                for event in &shown.events {
                    match event {
                        GraphEvent::Refuse { reason, .. } => {
                            studio.notice = Some((NoticeTone::Critical, reason.clone()));
                        }
                        GraphEvent::Append { node, .. } => studio.picker_after = Some(node.clone()),
                        _ => {}
                    }
                }
                studio.events.extend(shown.events);
                if let Some(after) = studio.picker_after.clone() {
                    let (blocks, groups) = blocks();
                    let overlay = GraphOverlay::new(
                        "picker",
                        GraphAnchor::Node {
                            node: after.clone(),
                            dx: 34.0,
                            dy: 0.0,
                        },
                    )
                    .show(ui, &shown.controls, |ui, room, width| {
                        BlockPicker::new("kinds", &blocks, &groups)
                            .title(Some(format!("After {after}")))
                            .room(room, width)
                            .words(BlockPickerWords {
                                search: "Find a kind of step".into(),
                                list: "Kinds of step".into(),
                                ..Default::default()
                            })
                            .show(ui)
                    });
                    if let Some(picked) = overlay.inner.and_then(|p| p.picked) {
                        studio.picked = Some(blocks[picked].kind.clone());
                        studio.picker_after = None;
                    } else if overlay.closed || overlay.inner.is_some_and(|p| p.closed) {
                        studio.picker_after = None;
                    }
                }
                if let Some((tone, text)) = studio.notice.clone()
                    && GraphNotice::new(text).tone(tone).show(ui, &shown.controls)
                {
                    studio.notice = None;
                }
            },
            state,
        )
}

#[test]
fn the_nightly_export_pipeline_stands_as_the_web_draws_it() {
    let mut harness = studio(Studio {
        steps: nightly_export(),
        ..Default::default()
    });
    harness.run_steps(4);
    harness.snapshot("pipeline_nightly_export");
}

#[test]
fn a_chosen_step_wears_a_ring_and_its_add_button() {
    let mut harness = studio(Studio {
        steps: nightly_export(),
        selected: Some("validate".into()),
        ..Default::default()
    });
    harness.run_steps(4);
    harness.snapshot("pipeline_step_selected");
}

#[test]
fn the_picker_opens_after_a_step() {
    let mut harness = studio(Studio {
        steps: nightly_export(),
        selected: Some("validate".into()),
        picker_after: Some("validate".into()),
        ..Default::default()
    });
    harness.run_steps(4);
    harness.snapshot("pipeline_block_picker");
}

#[test]
fn a_loop_is_refused_with_a_notice() {
    let steps = nightly_export();
    let reason = cycle_path(&steps, "upload", "transform").expect("upload comes after transform");
    let mut harness = studio(Studio {
        steps,
        notice: Some((
            NoticeTone::Critical,
            format!(
                "That makes a loop: {}.",
                [reason, vec!["transform".into()]].concat().join(" -> ")
            ),
        )),
        ..Default::default()
    });
    harness.run_steps(4);
    harness.snapshot("pipeline_cycle_notice");
}

#[test]
fn an_empty_pipeline_offers_its_first_step() {
    let mut harness = studio(Studio::default());
    harness.run_steps(4);
    harness.get_by_label("No steps yet");
    harness.get_by_label("Add step");
}

#[test]
fn a_run_part_way_through_shows_each_step_in_its_state() {
    let steps = vec![
        RunStep::new("fetch", "succeeded")
            .kind("fetch")
            .icon(Some(Icon::Download))
            .duration_ms(Some(1240.0)),
        RunStep::new("transform", "succeeded")
            .kind("transform")
            .icon(Some(Icon::Shuffle))
            .needs(["fetch"])
            .duration_ms(Some(3800.0)),
        RunStep::new("validate", "running")
            .kind("validate")
            .icon(Some(Icon::ShieldCheck))
            .needs(["transform"])
            .duration_ms(Some(2100.0))
            .progress(Some(0.4)),
        RunStep::new("upload", "queued")
            .kind("upload")
            .icon(Some(Icon::Upload))
            .needs(["validate"]),
        RunStep::new("notify", "pending")
            .kind("notify")
            .icon(Some(Icon::Bell))
            .needs(["upload"]),
    ];
    let mut harness = Harness::builder()
        .with_size(STAGE + Vec2::splat(2.0 * MARGIN))
        .build_ui(move |ui| {
            install(ui.ctx(), &Default::default());
            // still, so the flowing edge holds one place for the picture
            ui.style_mut().animation_time = 0.0;
            RunGraph::new("run-42", &steps).size(STAGE).show(ui);
        });
    harness.run_steps(4);
    harness.snapshot("pipeline_run_midway");
}

// The core's sans face carries Latin alone, so "→" (U+2192) in the studio's words is written "->"
// here until the core falls back to a face that has it.

/// The middle of a node's box on the screen, by its label.
fn middle(harness: &Harness<'_, Studio>, label: &str) -> Pos2 {
    harness.get_by_label(label).rect().center()
}

#[test]
fn a_connection_dragged_back_up_the_chain_is_refused_for_its_loop() {
    let mut harness = studio(Studio {
        steps: nightly_export(),
        ..Default::default()
    });
    harness.run_steps(4);
    // upload's output port sits at the foot of its box
    let upload = harness.get_by_label("upload").rect();
    let from = Pos2::new(upload.center().x, upload.bottom());
    let to = middle(&harness, "transform");
    harness.drag_at(from);
    harness.step();
    harness.hover_at(from + vec2(0.0, 20.0));
    harness.step();
    harness.hover_at(to);
    harness.step();
    harness.drop_at(to);
    harness.run_steps(2);
    let refused = harness
        .state()
        .events
        .iter()
        .find(|e| matches!(e, GraphEvent::Refuse { .. }))
        .cloned();
    assert_eq!(
        refused,
        Some(GraphEvent::Refuse {
            source: "upload".into(),
            target: "transform".into(),
            reason: "That makes a loop: transform -> validate -> upload -> transform.".into(),
        })
    );
    harness.get_by_label("That makes a loop: transform -> validate -> upload -> transform.");
}

#[test]
fn delete_removes_the_chosen_step() {
    let mut harness = studio(Studio {
        steps: nightly_export(),
        ..Default::default()
    });
    harness.run_steps(4);
    harness.get_by_label("validate").click();
    harness.run_steps(2);
    assert_eq!(harness.state().selected.as_deref(), Some("validate"));
    harness.key_press(egui::Key::Delete);
    harness.run_steps(2);
    assert!(harness.state().events.contains(&GraphEvent::Remove("validate".into())));
}

#[test]
fn the_add_button_at_a_chosen_step_opens_the_picker_and_a_pick_is_reported() {
    let mut harness = studio(Studio {
        steps: nightly_export(),
        selected: Some("validate".into()),
        ..Default::default()
    });
    harness.run_steps(4);
    // every step's output carries an add button; validate's is the one at its foot
    let validate = harness.get_by_label("validate").rect();
    harness
        .get_all_by_label("Add a step after this one")
        .find(|port| port.rect().contains(validate.center_bottom()))
        .expect("validate's add button")
        .click();
    harness.run_steps(4);
    assert_eq!(harness.state().picker_after.as_deref(), Some("validate"));
    // the picker's search has the keys: down twice and Enter picks the third kind listed
    harness.key_press(egui::Key::ArrowDown);
    harness.key_press(egui::Key::ArrowDown);
    harness.key_press(egui::Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().picked.as_deref(), Some("validate"));
    assert_eq!(harness.state().picker_after, None);
}
