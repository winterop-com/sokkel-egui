//! The force layout settles where the web's does: the same bodies and links, run through
//! sokkel-ui-graph's `force.ts` on d3-force 3.0.1 in V8 (as a browser runs it; deno here), give
//! these places, which the port gives too, to the last bit.

use sokkel_egui_graph::force::{FORCE_LINK, FORCE_WARM_TICKS};
use sokkel_egui_graph::{ForceBody, ForceSimulation, dot_size};

/// Each body at the web's place, bit for bit.
fn assert_at(got: &[ForceBody], want: &[[f64; 2]], what: &str) {
    assert_eq!(got.len(), want.len(), "{what}: as many bodies");
    for (body, [x, y]) in got.iter().zip(want) {
        assert!(
            body.x.to_bits() == x.to_bits() && body.y.to_bits() == y.to_bits(),
            "{what}: {} at ({}, {}), the web has ({x}, {y})",
            body.id,
            body.x,
            body.y
        );
    }
}

/// force.test.ts's network: two linked jobs, a third linked to one of them, and a fourth alone.
fn jobs() -> ForceSimulation {
    let bodies = ["nightly-export", "sync-inventory", "rebuild-index", "send-digest"]
        .map(|id| ForceBody::new(id, 30.0))
        .to_vec();
    ForceSimulation::new(
        bodies,
        &[
            ("nightly-export", "sync-inventory"),
            ("sync-inventory", "rebuild-index"),
        ],
    )
}

#[test]
fn bodies_start_on_d3s_spiral() {
    let simulation = jobs();
    assert_at(
        simulation.bodies(),
        &[
            [7.0710678118654755, 0.0],
            [-9.03088751750192, 8.273032735715967],
            [1.3823220809823638, -15.750847141167634],
            [11.382848792909423, 14.846910566099618],
        ],
        "start",
    );
}

#[test]
fn a_small_network_settles_where_the_webs_does_and_follows_a_pinned_body() {
    let mut simulation = jobs();
    simulation.settle(600);
    assert!(simulation.settled());
    assert_at(
        simulation.bodies(),
        &[
            [33.671576987308164, -18.156991116102326],
            [-68.03707299683326, 33.14135145092491],
            [-50.69405315829825, -78.05920536709043],
            [74.78990013039127, 82.41897399548024],
        ],
        "settled",
    );
    let held = simulation.body_mut("nightly-export").unwrap();
    held.fx = Some(400.0);
    held.fy = Some(-300.0);
    simulation.set_alpha(0.6);
    simulation.settle(600);
    assert_at(
        simulation.bodies(),
        &[
            [400.0, -300.0],
            [246.9506043024853, -186.09349420819782],
            [140.81117857370833, -108.3083645153255],
            [-26.85112442856221, 38.01732783048228],
        ],
        "pinned",
    );
    let (a, b) = (&simulation.bodies()[0], &simulation.bodies()[1]);
    assert!(
        (a.x - b.x).hypot(a.y - b.y) < FORCE_LINK * 2.5,
        "its neighbor followed it"
    );
}

#[test]
fn the_notes_vault_settles_where_the_webs_does() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/notes_graph.json")).expect("the fixture reads");
    let ids: Vec<&str> = fixture["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    let links: Vec<(&str, &str)> = fixture["links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| (l["source"].as_str().unwrap(), l["target"].as_str().unwrap()))
        .collect();
    let degree = |id: &str| links.iter().filter(|(s, t)| *s == id || *t == id).count();
    // NetworkGraph's room: half the dot and 38 for the name under it
    let bodies = ids
        .iter()
        .map(|id| ForceBody::new(*id, dot_size(degree(id)) as f64 / 2.0 + 38.0))
        .collect();
    // the places are kept as JavaScript prints them, and read with Rust's own parse, which
    // rounds correctly where serde_json's quick one may be an ulp off
    let number = |v: &serde_json::Value| v.as_str().unwrap().parse::<f64>().unwrap();
    let places = |key: &str| -> Vec<[f64; 2]> {
        fixture[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| [number(&p[0]), number(&p[1])])
            .collect()
    };
    let mut simulation = ForceSimulation::new(bodies, &links);
    simulation.settle(FORCE_WARM_TICKS);
    assert_at(simulation.bodies(), &places("warm"), "warm");
    let mut ticks = FORCE_WARM_TICKS;
    while !simulation.settled() {
        simulation.tick();
        ticks += 1;
    }
    assert_eq!(ticks as u64, fixture["ticks"].as_u64().unwrap());
    assert_at(simulation.bodies(), &places("settled"), "settled");
}

#[test]
fn the_spirals_sines_round_as_v8s_do() {
    // turns of the golden angle where the system's own sine or cosine is an ulp off V8's
    let angle = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    for (i, cos, sin) in [
        (8.0, 0.9393212963241181, 0.343038630874102),
        (12.0, -0.86521120975323, -0.5014075812324265),
        (18.0, 0.7088294143034152, -0.7053799411794167),
        (22.0, -0.8208583369658858, 0.5711318504807802),
    ] {
        assert_eq!(sokkel_egui_graph::force::sin_cos(i * angle), (sin, cos), "turn {i}");
    }
}
