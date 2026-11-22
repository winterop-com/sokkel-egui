//! A rail collapsed by its mark, as the web's `collapseWith="mark"`: the mark's tile is the one
//! toggle, showing the collapse icon under the pointer, with no control at the head's end.

use egui::{Rect, UiBuilder, pos2, vec2};
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::{Appearance, Icon, Rail, RailCollapse, RailEntry, RailGroup, install};

const MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

#[derive(Default)]
struct State {
    place: String,
    collapsed: bool,
}

fn rail() -> Harness<'static, State> {
    let appearance = Appearance::default();
    let groups = vec![RailGroup::new(
        None,
        [
            RailEntry::new("runs", "Runs", Icon::Play),
            RailEntry::new("jobs", "Jobs", Icon::Workflow),
        ],
    )];
    let mut harness = Harness::builder().with_size(vec2(240.0, 200.0)).build_ui_state(
        move |ui, state: &mut State| {
            install(ui.ctx(), &appearance);
            let width = Rail::width(state.collapsed);
            let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(width, 200.0));
            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                ui.add(
                    Rail::new("Tidevann", &groups, &mut state.place)
                        .mark(MARK)
                        .collapsible(&mut state.collapsed)
                        .collapse_with(RailCollapse::Mark),
                );
            });
        },
        State {
            place: "runs".into(),
            ..Default::default()
        },
    );
    harness.run_steps(3);
    harness
}

#[test]
fn the_mark_collapses_and_expands_the_rail() {
    let mut harness = rail();
    // expanded: the mark alone collapses it, no control at the head's end
    assert_eq!(harness.query_all_by_label("Collapse the navigation").count(), 1);
    let mark = harness.get_by_label("Collapse the navigation").rect();
    assert!(mark.left() < 20.0, "the mark at the head's start collapses the rail");
    harness.hover_at(mark.center());
    harness.run_steps(2);
    harness.snapshot("rail_collapse_with_mark");
    harness.get_by_label("Collapse the navigation").click();
    harness.run_steps(3);
    assert!(harness.state().collapsed);
    // collapsed: the mark expands it again
    harness.get_by_label("Expand the navigation").click();
    harness.run_steps(3);
    assert!(!harness.state().collapsed);
}
