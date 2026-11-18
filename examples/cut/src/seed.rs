//! The projects the demo starts with, as the web's `seed.ts` makes them: three edits cut from the
//! bin, one for each of Tidevann's projects.

use crate::model::{Clip, Edit, Project, default_tracks};

const HOUR: i64 = 3_600_000;
const DAY: i64 = 24 * HOUR;

/// A clip at `start` for `duration` on `track`, from the start of its source unless `in_point`
/// says otherwise.
fn clip(id: &str, track: &str, start: f64, duration: f64) -> Clip {
    Clip {
        id: id.to_owned(),
        track: track.to_owned(),
        start,
        duration,
        in_point: 0.0,
        source: None,
        name: None,
        text: None,
        volume: None,
    }
}

fn title(id: &str, start: f64, duration: f64, text: &str) -> Clip {
    Clip {
        text: Some(text.to_owned()),
        ..clip(id, "T1", start, duration)
    }
}

fn picture(id: &str, start: f64, duration: f64, source: &str, in_point: f64) -> Clip {
    Clip {
        source: Some(source.to_owned()),
        in_point,
        ..clip(id, "V1", start, duration)
    }
}

#[allow(clippy::too_many_arguments)]
fn sound(id: &str, track: &str, start: f64, duration: f64, source: &str, in_point: f64, volume: f64) -> Clip {
    Clip {
        source: Some(source.to_owned()),
        in_point,
        volume: Some(volume),
        ..clip(id, track, start, duration)
    }
}

fn project(id: &str, name: &str, owner: &str, (created, updated): (i64, i64), clips: Vec<Clip>) -> Project {
    Project {
        id: id.to_owned(),
        name: name.to_owned(),
        owner: owner.to_owned(),
        created,
        updated,
        edit: Edit {
            tracks: default_tracks(),
            clips,
        },
    }
}

/// The three projects, as they stand at `now`.
pub fn seed_projects(now: i64) -> Vec<Project> {
    vec![
        project(
            "harbor",
            "Harbor morning",
            "ada",
            (now - 9 * DAY, now - 2 * HOUR),
            vec![
                title("harbor-t1", 0.0, 4.0, "Harbor morning"),
                title("harbor-t2", 20.0, 3.5, "High water at 06:40"),
                picture("harbor-v1", 0.0, 8.0, "harbor-dawn", 2.0),
                picture("harbor-v2", 8.0, 6.0, "lighthouse", 4.0),
                picture("harbor-v3", 14.0, 6.0, "ferry-route", 1.0),
                picture("harbor-v4", 20.0, 6.0, "tide-chart", 5.0),
                sound("harbor-a1", "A1", 0.0, 26.0, "surf", 0.0, 0.8),
            ],
        ),
        project(
            "juniper",
            "Juniper field notes",
            "grace",
            (now - 6 * DAY, now - DAY),
            vec![
                title("juniper-t1", 0.5, 3.5, "Field notes: the ridge"),
                picture("juniper-v1", 0.0, 7.0, "ridge-snow", 3.0),
                picture("juniper-v2", 7.0, 9.0, "wind-farm", 2.0),
                picture("juniper-v3", 16.0, 8.0, "aurora", 6.0),
                sound("juniper-a1", "A1", 0.0, 24.0, "high-wind", 1.0, 0.7),
            ],
        ),
        project(
            "larkspur",
            "Larkspur line",
            "linus",
            (now - 4 * DAY, now - 3 * DAY),
            vec![
                title("larkspur-t1", 0.0, 3.0, "The Larkspur line"),
                title("larkspur-t2", 13.0, 3.0, "Into the city"),
                picture("larkspur-v1", 0.0, 13.0, "coast-line", 0.0),
                picture("larkspur-v2", 13.0, 9.0, "city-rain", 2.0),
                sound("larkspur-a1", "A1", 0.0, 13.0, "rails", 4.0, 0.9),
                sound("larkspur-a2", "A2", 12.0, 10.0, "roof-rain", 0.0, 0.6),
            ],
        ),
    ]
}
