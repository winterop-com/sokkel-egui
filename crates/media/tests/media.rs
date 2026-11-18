//! The media add-on: cut's projects as the web shot lays them out (`parity/cut/web/cut_projects.png`),
//! each card's picture painted by the app; the bin's cards, a sound's wearing its waveform; the
//! drawn covers; and a card answering a press, its menu and a drag.

use egui::{Color32, Mesh, Painter, Pos2, Rect, Stroke, pos2, vec2};
use egui_kittest::{Harness, kittest::Queryable};
use sokkel_egui::{Appearance, DragData, Icon, MenuItem, Scheme, Waveform, install, static_levels, tokens};
use sokkel_egui_media::{
    ARTWORK_MOTIFS, Artwork, ArtworkCover, ArtworkHue, MediaCard, MediaGrid, MediaShape, cover_for, seed_hash,
};

fn light() -> Appearance {
    Appearance {
        scheme: Scheme::Light,
        ..Appearance::default()
    }
}

/// A sky from `top` to `low`, as cut's scenes start.
fn sky(painter: &Painter, rect: Rect, top: Color32, low: Color32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.right_bottom(), low);
    mesh.colored_vertex(rect.left_bottom(), low);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

/// A ridge line filled to the foot.
fn ridge(painter: &Painter, rect: Rect, heights: &[f32], color: Color32) {
    let mut mesh = Mesh::default();
    for (i, h) in heights.iter().enumerate() {
        let x = rect.left() + rect.width() * i as f32 / (heights.len() - 1) as f32;
        mesh.colored_vertex(pos2(x, rect.top() + rect.height() * h), color);
        mesh.colored_vertex(pos2(x, rect.bottom()), color);
        if i > 0 {
            let a = (2 * (i - 1)) as u32;
            mesh.add_triangle(a, a + 1, a + 2);
            mesh.add_triangle(a + 1, a + 3, a + 2);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// The harbor at dawn: a sky warming to the sea's edge, the sun on it, the sea and a boat.
fn harbor(painter: &Painter, rect: Rect) {
    let at = |x: f32, y: f32| rect.min + vec2(rect.width() * x, rect.height() * y);
    sky(
        painter,
        rect,
        Color32::from_rgb(52, 66, 99),
        Color32::from_rgb(233, 150, 82),
    );
    painter.circle_filled(at(0.68, 0.6), rect.height() * 0.09, Color32::from_rgb(255, 214, 140));
    let sea = Rect::from_min_max(at(0.0, 0.6), rect.max);
    sky(
        painter,
        sea,
        Color32::from_rgb(38, 82, 128),
        Color32::from_rgb(20, 38, 66),
    );
    ridge(
        painter,
        sea,
        &[0.5, 0.42, 0.55, 0.4, 0.52, 0.45, 0.5],
        Color32::from_rgb(25, 47, 80),
    );
    let hull = [at(0.14, 0.69), at(0.18, 0.69), at(0.17, 0.71), at(0.15, 0.71)];
    painter.add(egui::Shape::convex_polygon(
        hull.to_vec(),
        Color32::from_rgb(15, 22, 34),
        Stroke::NONE,
    ));
    painter.add(egui::Shape::convex_polygon(
        vec![at(0.16, 0.62), at(0.16, 0.69), at(0.18, 0.68)],
        Color32::from_rgb(15, 22, 34),
        Stroke::NONE,
    ));
}

/// Snow on the ridge: a night sky, its stars, and two ranges of peaks.
fn juniper(painter: &Painter, rect: Rect) {
    sky(
        painter,
        rect,
        Color32::from_rgb(43, 62, 112),
        Color32::from_rgb(92, 118, 170),
    );
    for i in 0..40u32 {
        let x = (seed_hash(&format!("star-{i}")) % 1000) as f32 / 1000.0;
        let y = (seed_hash(&format!("high-{i}")) % 1000) as f32 / 1000.0 * 0.5;
        painter.circle_filled(
            rect.min + vec2(rect.width() * x, rect.height() * y),
            0.8,
            Color32::WHITE,
        );
    }
    ridge(
        painter,
        rect,
        &[0.45, 0.25, 0.42, 0.22, 0.4, 0.3, 0.5],
        Color32::from_rgb(170, 184, 210),
    );
    ridge(
        painter,
        rect,
        &[0.7, 0.55, 0.68, 0.52, 0.66, 0.6, 0.72],
        Color32::from_rgb(54, 66, 92),
    );
}

/// The coast line: a sky over hills, a sun, the poles and their wires over the track.
fn larkspur(painter: &Painter, rect: Rect) {
    let at = |x: f32, y: f32| rect.min + vec2(rect.width() * x, rect.height() * y);
    sky(
        painter,
        rect,
        Color32::from_rgb(78, 140, 168),
        Color32::from_rgb(214, 170, 110),
    );
    painter.circle_filled(at(0.3, 0.38), rect.height() * 0.08, Color32::from_rgb(250, 226, 160));
    ridge(
        painter,
        rect,
        &[0.6, 0.5, 0.58, 0.45, 0.57, 0.5, 0.6],
        Color32::from_rgb(160, 110, 80),
    );
    let ground = Rect::from_min_max(at(0.0, 0.72), rect.max);
    painter.rect_filled(ground, 0.0, Color32::from_rgb(120, 82, 52));
    painter.hline(
        rect.x_range(),
        at(0.0, 0.72).y,
        Stroke::new(1.5, Color32::from_rgb(70, 50, 40)),
    );
    for x in [0.15, 0.45, 0.75] {
        painter.vline(
            at(x, 0.0).x,
            at(0.0, 0.42).y..=rect.bottom(),
            Stroke::new(2.0, Color32::from_rgb(40, 32, 30)),
        );
    }
    painter.hline(
        rect.x_range(),
        at(0.0, 0.46).y,
        Stroke::new(1.0, Color32::from_rgb(40, 32, 30)),
    );
}

type Scene = fn(&Painter, Rect);

/// The projects page's three, as the web shot shows them.
const PROJECTS: [(&str, &str, &str, &str, Scene); 3] = [
    (
        "harbor",
        "Harbor morning",
        "7 clips \u{b7} Edited 2 hours ago",
        "0:26",
        harbor,
    ),
    (
        "juniper",
        "Juniper field notes",
        "5 clips \u{b7} Edited yesterday",
        "0:24",
        juniper,
    ),
    (
        "larkspur",
        "Larkspur line",
        "6 clips \u{b7} Edited 3 days ago",
        "0:22",
        larkspur,
    ),
];

fn menu() -> Vec<MenuItem> {
    vec![
        MenuItem::action("Open").icon(Icon::ExternalLink),
        MenuItem::action("Rename").icon(Icon::PencilLine),
        MenuItem::action("Duplicate").icon(Icon::Copy),
        MenuItem::Separator,
        MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
    ]
}

/// The projects page's grid, as wide as the web page's content (1440 less the rail and its insets).
#[test]
fn the_projects_as_cards() {
    let items = menu();
    let mut harness = Harness::builder()
        .with_size([1125.0 + 16.0, 240.0])
        .build_ui(move |ui| {
            install(ui.ctx(), &light());
            MediaGrid::new(MediaShape::Wide).show(ui, PROJECTS.len(), |ui, index| {
                let (id, title, subtitle, badge, scene) = PROJECTS[index];
                let _ = MediaCard::new(id, title)
                    .shape(MediaShape::Wide)
                    .subtitle(subtitle)
                    .badge(badge)
                    .cover(scene)
                    .menu(&items)
                    .show(ui);
            });
        });
    harness.run_steps(3);
    harness.snapshot("media_card_projects");
}

/// The bin's cards: a picture under its length, a sound wearing its waveform, a card without a
/// picture in the cover drawn from its seed, one playing and part watched, and square albums.
#[test]
fn the_bin_and_the_other_cards() {
    let items = menu();
    let levels = static_levels(48, 4);
    let mut harness = Harness::builder().with_size([820.0, 520.0]).build_ui(move |ui| {
        install(ui.ctx(), &light());
        let t = tokens(ui.ctx());
        let primary = t.color("primary");
        ui.spacing_mut().item_spacing = vec2(16.0, 24.0);
        ui.horizontal_top(|ui| {
            for index in 0..4 {
                ui.allocate_ui(vec2(180.0, 220.0), |ui| {
                    ui.set_width(180.0);
                    let card = match index {
                        0 => MediaCard::new("harbor-dawn", "Harbor at dawn")
                            .subtitle("A harbor town on the northern coast")
                            .title_lines(1)
                            .badge("0:24")
                            .cover(harbor),
                        1 => MediaCard::new("surf", "Surf on the breakwater")
                            .subtitle("A harbor town on the northern coast")
                            .title_lines(1)
                            .badge("0:30")
                            .cover(|painter: &Painter, rect: Rect| {
                                Waveform::new(&levels)
                                    .bar_width(2.0)
                                    .bar_gap(1.0)
                                    .fade_edges(false)
                                    .color(primary)
                                    .paint(painter, rect.shrink2(vec2(8.0, 0.0)));
                            }),
                        2 => MediaCard::new("ridge-snow", "Snow on the ridge")
                            .subtitle("A mountain range in the north")
                            .placeholder_seed("ridge-snow"),
                        _ => MediaCard::new("aurora", "Aurora over the fjord, the whole night through")
                            .subtitle("A fjord in the far north")
                            .playing(true)
                            .progress(0.4)
                            .badge("0:22"),
                    };
                    let _ = card.shape(MediaShape::Wide).menu(&items).show(ui);
                });
            }
        });
        ui.horizontal_top(|ui| {
            for (index, seed) in ["nightly-export", "sync-inventory", "rebuild-index", "send-digest"]
                .iter()
                .enumerate()
            {
                ui.allocate_ui(vec2(152.0, 240.0), |ui| {
                    ui.set_width(152.0);
                    let _ = MediaCard::new(*seed, *seed)
                        .subtitle("Tidevann")
                        .playable(index == 0)
                        .playing(index == 1)
                        .show(ui);
                });
            }
        });
    });
    harness.run_steps(3);
    harness.snapshot("media_card_bin");
}

/// Every motif, square and wide, in hues the seed picks and in two named outright.
#[test]
fn the_drawn_covers() {
    let mut harness = Harness::builder().with_size([820.0, 280.0]).build_ui(|ui| {
        install(ui.ctx(), &light());
        ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
        ui.horizontal(|ui| {
            for (i, motif) in ARTWORK_MOTIFS.iter().enumerate() {
                let cover = ArtworkCover {
                    motif: *motif,
                    from: cover_for(&format!("cover-{i}")).from,
                    to: cover_for(&format!("cover-{i}")).to,
                };
                ui.add(Artwork::of(cover).size(vec2(120.0, 120.0)));
            }
        });
        ui.horizontal(|ui| {
            for (i, motif) in ARTWORK_MOTIFS.iter().enumerate() {
                let (from, to) = if i % 2 == 0 {
                    (ArtworkHue::Blue, ArtworkHue::Amber)
                } else {
                    (ArtworkHue::Violet, ArtworkHue::Coral)
                };
                ui.add(
                    Artwork::of(ArtworkCover {
                        motif: *motif,
                        from,
                        to,
                    })
                    .wide(true)
                    .size(vec2(120.0, 120.0 * 9.0 / 16.0)),
                );
            }
        });
        ui.horizontal(|ui| {
            for seed in ["harbor", "juniper", "larkspur", "ada", "grace", "linus"] {
                ui.add(Artwork::new(seed).wide(true).size(vec2(120.0, 68.0)));
            }
        });
    });
    harness.run_steps(3);
    harness.snapshot("artwork_covers");
}

#[test]
fn the_cover_a_seed_wears_is_sokkel_ui_s() {
    use sokkel_egui_media::ArtworkMotif::{Peaks, Sun};
    // as `seedHash` and `coverFor` answer for the same seeds
    assert_eq!(seed_hash(""), 2_872_998_923);
    assert_eq!(seed_hash("harbor"), 2_548_949_957);
    let cover = |motif, from, to| ArtworkCover { motif, from, to };
    assert_eq!(cover_for(""), cover(Peaks, ArtworkHue::Violet, ArtworkHue::Teal));
    assert_eq!(cover_for("harbor"), cover(Peaks, ArtworkHue::Jade, ArtworkHue::Pink));
    assert_eq!(cover_for("juniper"), cover(Sun, ArtworkHue::Amber, ArtworkHue::Jade));
    assert_eq!(cover_for("larkspur"), cover(Peaks, ArtworkHue::Teal, ArtworkHue::Coral));
}

#[derive(Default)]
struct Pressed {
    selected: usize,
    menu: Option<usize>,
    carried: bool,
}

#[test]
fn a_card_opens_on_a_press_takes_its_menu_and_drags_its_data() {
    let items = menu();
    let mut harness = Harness::builder().with_size([300.0, 260.0]).build_ui_state(
        move |ui, pressed: &mut Pressed| {
            install(ui.ctx(), &light());
            let r = MediaCard::new("harbor", "Harbor morning")
                .shape(MediaShape::Wide)
                .subtitle("7 clips")
                .menu(&items)
                .drag(DragData {
                    kind: "media".into(),
                    label: "Harbor morning".into(),
                    duration: Some(8.0),
                    value: "harbor".into(),
                })
                .width(267.0)
                .show(ui);
            if r.selected {
                pressed.selected += 1;
            }
            if r.menu.is_some() {
                pressed.menu = r.menu;
            }
            if egui::DragAndDrop::has_payload_of_type::<DragData>(ui.ctx()) {
                pressed.carried = true;
            }
        },
        Pressed::default(),
    );
    harness.run_steps(2);
    harness.get_by_label("Harbor morning").click();
    harness.run_steps(2);
    assert_eq!(harness.state().selected, 1);
    harness.get_by_label("More actions: Harbor morning").click();
    harness.run_steps(2);
    harness.get_by_label("Duplicate").click();
    harness.run_steps(2);
    assert_eq!(harness.state().menu, Some(2));
    let from = Pos2::new(100.0, 60.0);
    harness.hover_at(from);
    harness.drag_at(from);
    harness.run_steps(1);
    for step in 1..=5 {
        harness.hover_at(from + vec2(20.0 * step as f32, 0.0));
        harness.run_steps(1);
    }
    assert!(harness.state().carried);
}
