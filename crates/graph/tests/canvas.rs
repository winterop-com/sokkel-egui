//! The canvas: a press on a node says which, and the zoom buttons zoom and fit again.

use egui_kittest::{Harness, kittest::Queryable};
use sokkel_ui_egui::install;
use sokkel_ui_egui_graph::{GraphCanvas, GraphLink, GraphNode};

struct State {
    pressed: Option<String>,
}

fn harness() -> Harness<'static, State> {
    Harness::builder().with_size([480.0, 320.0]).build_ui_state(
        |ui, state: &mut State| {
            install(ui.ctx(), &Default::default());
            let nodes = [
                GraphNode::new("nightly-export").kind("job"),
                GraphNode::new("upload").kind("step"),
            ];
            let links = [GraphLink::new("nightly-export", "upload").arrow(true)];
            if let Some(id) = GraphCanvas::new("jobs", &nodes, &links)
                .selected(Some("upload"))
                .show(ui)
            {
                state.pressed = Some(id);
            }
        },
        State { pressed: None },
    )
}

#[test]
fn a_press_on_a_node_says_which() {
    let mut harness = harness();
    harness.run();
    harness.get_by_label("nightly-export").click();
    harness.run();
    assert_eq!(harness.state().pressed.as_deref(), Some("nightly-export"));
}

#[test]
fn zooming_in_grows_the_boxes_and_fit_puts_them_back() {
    let mut harness = harness();
    harness.run();
    let fitted = harness.get_by_label("upload").rect();
    harness.get_by_label("Zoom in").click();
    harness.run();
    let zoomed = harness.get_by_label("upload").rect();
    assert!(zoomed.width() > fitted.width() * 1.2, "{fitted:?} then {zoomed:?}");
    harness.get_by_label("Fit the graph").click();
    harness.run();
    assert_eq!(harness.get_by_label("upload").rect(), fitted);
}
