//! The map's controls: full screen takes the map over the whole window, and Exit full screen or
//! Escape brings it back.

use egui::Key;
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_ui_egui::install;
use sokkel_ui_egui_maps::{MapLegend, MapView, Marker};

fn harness() -> Harness<'static, MapView> {
    let mut harness = Harness::builder().with_size([800.0, 600.0]).build_ui_state(
        |ui, map: &mut MapView| {
            install(ui.ctx(), &Default::default());
            ui.add_space(100.0);
            let markers = [Marker::new(59.91, 10.75, "").id("oslo")];
            let response = map.show(ui, egui::vec2(320.0, 200.0), &markers, &[]);
            MapLegend::new("Sites").show(ui, &response);
        },
        MapView::new(59.91, 10.75, 5.0),
    );
    harness.run_steps(3);
    harness
}

#[test]
fn full_screen_covers_the_window_until_it_is_left() {
    let mut harness = harness();
    let small = harness.get_by_label("Zoom in").rect();
    harness.get_by_label("Full screen").click();
    harness.run_steps(3);
    assert!(harness.state().full);
    let large = harness.get_by_label("Zoom in").rect();
    assert!(
        large.right() > small.right() + 300.0,
        "the controls stand at the window's edge: {large:?}"
    );
    harness.get_by_label("Exit full screen").click();
    harness.run_steps(3);
    assert!(!harness.state().full);
    harness.get_by_label("Full screen").click();
    harness.run_steps(3);
    harness.key_press(Key::Escape);
    harness.run_steps(3);
    assert!(!harness.state().full, "Escape leaves it too");
}

#[test]
fn the_basemap_switcher_offers_each_basemap_and_draws_the_one_chosen() {
    let mut harness = harness();
    harness.get_by_label("Basemap").click();
    harness.run_steps(3);
    for name in ["Light", "Dark", "Satellite", "Blank"] {
        assert!(harness.query_by_label(name).is_some(), "{name} is offered");
    }
    harness.snapshot("basemap_switcher");
    harness.get_by_label("Satellite").click();
    harness.run_steps(3);
    assert_eq!(harness.state().basemap, Some(sokkel_ui_egui_maps::Basemap::Satellite));
    assert!(harness.query_by_label("Blank").is_none(), "the menu closes on a choice");
}

#[test]
fn the_dark_basemap_is_dark_in_the_light_scheme() {
    let mut harness = harness();
    harness.state_mut().basemap = Some(sokkel_ui_egui_maps::Basemap::Dark);
    harness.run_steps(3);
    harness.snapshot("basemap_dark_in_light");
}
