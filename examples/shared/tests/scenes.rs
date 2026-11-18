//! The four scenes the web's shots show (`parity/cut/web/cut_projects.png` and `cut_editor.png`):
//! harbor at dawn, the lighthouse beam, the coast line train and snow on the ridge, each 270 by 150
//! as a card's cover draws it. The columns: the moment the projects page shows (a second into the
//! edit), the bin's poster (a third of the way in), late in the clip, and a second in with the
//! title card.

use egui::{Color32, Rect, pos2, vec2};
use egui_kittest::Harness;
use tidevann_shared::clips::{SceneKind, SceneSpec, draw_frame};

const W: f32 = 270.0;
const H: f32 = 150.0;
const GAP: f32 = 8.0;

fn spec(kind: SceneKind, hue: f64, accent: f64, seed: u32, title: &str, duration: f64) -> SceneSpec {
    SceneSpec {
        kind,
        hue,
        accent,
        seed,
        title: title.to_owned(),
        channel: "Tidevann".to_owned(),
        duration,
    }
}

#[test]
fn four_scenes() {
    let rows = [
        (spec(SceneKind::Sea, 212.0, 28.0, 11, "Harbor at dawn", 24.0), 3.0),
        (
            spec(SceneKind::Lighthouse, 225.0, 48.0, 23, "Lighthouse beam", 18.0),
            5.0,
        ),
        (spec(SceneKind::Train, 200.0, 30.0, 37, "Coast line train", 20.0), 1.0),
        (spec(SceneKind::Peaks, 215.0, 190.0, 41, "Snow on the ridge", 16.0), 4.0),
    ];
    let mut harness = Harness::builder()
        .with_size([GAP + 4.0 * (W + GAP), GAP + 4.0 * (H + GAP)])
        .build_ui(move |ui| {
            // The faces the apps draw with: the canvas's sans and monospace are sokkel-egui's.
            sokkel_egui::install_fonts(ui.ctx());
            let painter = ui.painter();
            painter.rect_filled(ui.max_rect(), 0.0, Color32::from_gray(24));
            for (row, (spec, first)) in rows.iter().enumerate() {
                let moments = [
                    (*first, false),
                    (spec.duration * 0.35, false),
                    (spec.duration * 0.8, false),
                    (1.0, true),
                ];
                for (column, (t, card)) in moments.into_iter().enumerate() {
                    let at = pos2(GAP + column as f32 * (W + GAP), GAP + row as f32 * (H + GAP));
                    let rect = Rect::from_min_size(at, vec2(W, H));
                    // Black behind, as a poster's still and the preview have it.
                    painter.rect_filled(rect, 0.0, Color32::BLACK);
                    draw_frame(painter, rect, spec, t, card);
                }
            }
        });
    harness.run();
    harness.snapshot("scenes");
}
