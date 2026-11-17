//! The live stream sends what the web's sends, when it sends it: each event below is what bun reads
//! from the web's own `backend.ts`, its random source seeded at 2026-11-18 09:00 UTC, each wait
//! starting once the backend has kept the event before.

use analytics::AnalyticsApp;
use analytics::app::DEMO_START;
use analytics::model::{ChangeKind, Plan};
use sokkel_egui::Appearance;

#[test]
fn the_first_events_are_the_webs() {
    let mut app = AnalyticsApp::at(DEMO_START, Appearance::default());
    app.run_for(5000);
    let names: Vec<&str> = app.kept.teams.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["Slategate Audio", "Onyxford Florists"]);
    assert_eq!(app.world.live_count, 2);
    app.run_for(2500);
    let seen: Vec<(i64, ChangeKind, &str, &str, i64, Plan)> = app
        .kept
        .changes
        .iter()
        .map(|c| (c.at, c.kind, c.id.as_str(), c.team.as_str(), c.seats, c.plan))
        .collect();
    assert_eq!(
        seen,
        [
            (
                1_794_992_401_804,
                ChangeKind::Signup,
                "c-mwlw4pe4-2",
                "live-mwlw4pe4-1",
                2,
                Plan::Free
            ),
            (
                1_794_992_403_863,
                ChangeKind::Signup,
                "c-mwlw4qzb-4",
                "live-mwlw4qzb-3",
                4,
                Plan::Free
            ),
            (
                1_794_992_405_262,
                ChangeKind::Seats,
                "c-mwlw4s26-5",
                "t20715-9",
                8,
                Plan::Team
            ),
            (
                1_794_992_407_144,
                ChangeKind::Churn,
                "c-mwlw4tig-6",
                "t20532-13",
                0,
                Plan::Team
            ),
        ]
    );
}
