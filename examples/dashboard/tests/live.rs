//! The live source reads the same every time, keeps its history to its bounds, and moves runs on.

use dashboard::live::{KEEP, Live, RunState, clock_of, on_call, start, upcoming};
use dashboard::model::{Metric, presets};

#[test]
fn it_opens_the_same_way_every_time() {
    let (a, b) = (Live::begin(), Live::begin());
    assert_eq!(a.readings, b.readings);
    assert_eq!(a.runs, b.runs);
    assert_eq!(a.happenings, b.happenings);
    assert_eq!(a.readings.len(), KEEP);
    assert_eq!(a.uptime[0].len(), 90);
}

#[test]
fn ticks_keep_their_bounds_and_move_runs_on() {
    let mut live = Live::begin();
    let running = live.runs.iter().filter(|r| r.state == RunState::Running).count();
    for _ in 0..200 {
        live.tick(3000.0);
    }
    assert_eq!(live.readings.len(), KEEP);
    assert!(live.runs.len() <= 40);
    assert!(live.happenings.len() <= 60);
    assert!(live.logs.iter().all(|l| l.len() <= 200));
    assert!(live.runs.iter().any(|r| r.state != RunState::Running) || running > 0);
    let (now, values) = live.metric(Metric::Success);
    assert!((80.0..=100.0).contains(&now));
    assert_eq!(values.len(), KEEP);
    assert_eq!(clock_of(live.at, false, true), "09:10:00");
}

#[test]
fn the_rotation_and_the_schedules() {
    let (now, next, since, _) = on_call(start());
    assert_ne!(now, next);
    assert_eq!(clock_of(since, false, false), "09:00");
    let fires = upcoming(start(), 6);
    assert_eq!(fires.len(), 6);
    assert!(fires.windows(2).all(|w| w[0].at <= w[1].at));
    assert!(fires.iter().all(|f| f.at >= start()));
    assert_eq!(clock_of(start() + 13.0 * 3_600_000.0, true, false), "10:00 PM");
}

#[test]
fn the_presets() {
    let boards = presets();
    assert_eq!(boards.iter().map(|b| b.widgets.len()).collect::<Vec<_>>(), [13, 10, 7]);
    assert!(boards.iter().all(|b| b.widgets == b.base));
}
