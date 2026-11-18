//! The network: the notes vault's graph drawn at the web's size, a press on a dot says which, the
//! zoom buttons zoom and fit again, a dragged dot moves, and an empty network says so.

use egui::{Vec2, vec2};
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::install;
use sokkel_egui_graph::{KindColor, NetworkGraph, NetworkLink, NetworkNode, NetworkWords};

/// The graph's box on the web's Graph page, at 1440 by 900.
const STAGE: Vec2 = vec2(1126.0, 556.0);
/// The margin kittest's panel keeps round the stage.
const MARGIN: f32 = 8.0;

/// The notes vault as the web's Graph page builds it: every note, colored by its top folder, and
/// the notes linked to but not made yet, faint.
fn vault() -> (Vec<NetworkNode>, Vec<NetworkLink>) {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/notes_graph.json")).expect("the fixture reads");
    let nodes = fixture["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| {
            NetworkNode::new(n["id"].as_str().unwrap(), n["label"].as_str().unwrap())
                .color(n["color"].as_str().and_then(KindColor::from_name))
                .faint(n["faint"].as_bool().unwrap_or(false))
        })
        .collect();
    let links = fixture["links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| NetworkLink::new(l["source"].as_str().unwrap(), l["target"].as_str().unwrap()))
        .collect();
    (nodes, links)
}

#[derive(Default)]
struct State {
    pressed: Option<String>,
    hovered: Option<String>,
}

fn harness(nodes: Vec<NetworkNode>, links: Vec<NetworkLink>) -> Harness<'static, State> {
    let mut harness = Harness::builder()
        .with_size(STAGE + Vec2::splat(2.0 * MARGIN))
        .build_ui_state(
            move |ui, state: &mut State| {
                install(ui.ctx(), &Default::default());
                let shown = NetworkGraph::new("vault", &nodes, &links)
                    .current(Some("retry"))
                    .size(STAGE)
                    .words(NetworkWords {
                        label: "The notes and their links".into(),
                        empty: "No note to show.".into(),
                        ..Default::default()
                    })
                    .show(ui);
                if let Some(id) = shown.pressed {
                    state.pressed = Some(id);
                }
                state.hovered = shown.hovered;
            },
            State::default(),
        );
    // the first 150 ticks run at once, the rest one a frame until the network settles
    harness.run_steps(120);
    harness
}

#[test]
fn the_vault_settles_as_the_web_draws_it() {
    let (nodes, links) = vault();
    let mut harness = harness(nodes, links);
    harness.snapshot("network_notes_vault");
}

#[test]
fn a_press_on_a_dot_says_which_and_pointing_lights_its_neighbors() {
    let (nodes, links) = vault();
    let mut harness = harness(nodes, links);
    harness.get_by_label("harbor").hover();
    harness.run_steps(2);
    assert_eq!(harness.state().hovered.as_deref(), Some("harbor"));
    harness.snapshot("network_notes_vault_harbor");
    harness.get_by_label("harbor").click();
    harness.run_steps(2);
    assert_eq!(harness.state().pressed.as_deref(), Some("harbor"));
}

#[test]
fn zooming_in_grows_the_dots_and_fit_puts_them_back() {
    let (nodes, links) = vault();
    let mut harness = harness(nodes, links);
    let fitted = harness.get_by_label("harbor").rect();
    harness.get_by_label("Zoom in").click();
    harness.run_steps(2);
    let zoomed = harness.get_by_label("harbor").rect();
    assert!(zoomed.width() > fitted.width() * 1.2, "{fitted:?} then {zoomed:?}");
    harness.get_by_label("Fit the graph").click();
    harness.run_steps(2);
    assert_eq!(harness.get_by_label("harbor").rect(), fitted);
}

#[test]
fn a_dragged_dot_moves_and_pulls_its_neighbors_after_it() {
    let (nodes, links) = vault();
    let mut harness = harness(nodes, links);
    let before = harness.get_by_label("Ideas").rect().center();
    let neighbor = harness.get_by_label("Quarter review").rect().center();
    harness.drag_at(before);
    harness.run_steps(1);
    for step in 1..=10 {
        harness.hover_at(before + vec2(-12.0, 8.0) * step as f32);
        harness.run_steps(1);
    }
    // held, it stands where the pointer has it (the frame after the pointer moved)
    harness.run_steps(1);
    let held = harness.get_by_label("Ideas").rect().center();
    assert!(
        (held - (before + vec2(-120.0, 80.0))).length() < 2.0,
        "{before:?} then {held:?}"
    );
    harness.drop_at(before + vec2(-120.0, 80.0));
    harness.run_steps(10);
    let pulled = harness.get_by_label("Quarter review").rect().center();
    assert!(pulled != neighbor, "its neighbor moved too");
}

#[test]
fn an_empty_network_says_so() {
    let harness = harness(Vec::new(), Vec::new());
    harness.get_by_label("No note to show.");
    assert!(
        harness.query_by_label("Zoom in").is_none(),
        "no zoom buttons on an empty stage"
    );
}
