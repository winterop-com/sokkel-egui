//! The downloads page, as the web's `pages/Downloads.tsx` has it: the pulls under way with their
//! progress, speed and time left, paused or queued, then those that ended, and what each row's
//! control and menu offer.

use serde::Serialize;

use super::disk::{Stat, stat};
use crate::data::format::{bytes, counted, remaining, speed};
use crate::data::model::{Download, DownloadState, Prefs};
use crate::js::round;

/// What a pull's state chip says: its status and its label.
pub fn state_chip(state: DownloadState) -> (&'static str, &'static str) {
    match state {
        DownloadState::Running => ("running", "pulling"),
        DownloadState::Queued => ("queued", "queued"),
        DownloadState::Paused => ("paused", "paused"),
        DownloadState::Done => ("succeeded", "in library"),
        DownloadState::Failed => ("failed", "failed"),
        DownloadState::Cancelled => ("cancelled", "cancelled"),
    }
}

/// A pull's row: its id, its group, and what each column says.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PullRow {
    pub id: String,
    pub group: &'static str,
    pub name: String,
    pub from: &'static str,
    pub state: String,
    /// The error when it failed, else the bytes in so far and its share in tenths of a percent.
    pub progress: String,
    pub value: Option<f64>,
    pub speed: String,
    pub left: String,
    pub actions: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DownloadsPage {
    pub count: String,
    pub description: String,
    pub can_clear: bool,
    pub stats: Vec<Stat>,
    pub rows: Vec<PullRow>,
}

/// What a row's menu offers, by id.
pub fn actions_of(pull: &Download) -> Vec<&'static str> {
    let mut list = Vec::new();
    if matches!(pull.state, DownloadState::Running | DownloadState::Queued) {
        list.push("pause");
    }
    if pull.state == DownloadState::Paused {
        list.push("resume");
    }
    if pull.state.going() {
        list.push("cancel");
    }
    if matches!(pull.state, DownloadState::Failed | DownloadState::Cancelled) {
        list.push("retry");
    }
    if pull.state == DownloadState::Done {
        list.push("open");
    }
    if !pull.state.going() {
        list.push("remove");
    }
    list
}

pub fn downloads_page(downloads: &[Download], prefs: &Prefs) -> DownloadsPage {
    let active: Vec<&Download> = downloads.iter().filter(|one| one.state.going()).collect();
    let running: Vec<&Download> = downloads
        .iter()
        .filter(|one| one.state == DownloadState::Running)
        .collect();
    let rate = running.iter().fold(0.0, |sum, one| sum + one.speed);
    let left = active.iter().fold(0.0, |sum, one| sum + (one.size - one.done));
    let ended = downloads.len() - active.len();
    let mut rows: Vec<&Download> = downloads.iter().collect();
    rows.sort_by_key(|one| std::cmp::Reverse(one.started));
    let mut grouped: Vec<&Download> = rows.iter().copied().filter(|one| one.state.going()).collect();
    grouped.extend(rows.iter().copied().filter(|one| !one.state.going()));
    DownloadsPage {
        count: format!("{} under way", counted(active.len() as f64, "pull")),
        description: format!(
            "{} at a time, {}. Change it in Settings.",
            crate::js::number_to_string(prefs.parallel),
            if prefs.bandwidth == 0.0 {
                "with no bandwidth limit".to_owned()
            } else {
                format!("at most {} MB/s together", crate::js::number_to_string(prefs.bandwidth))
            }
        ),
        can_clear: ended > 0,
        stats: vec![
            stat(
                "Speed",
                if running.is_empty() {
                    "-".to_owned()
                } else {
                    speed(rate)
                },
                Some(format!("{} pulling", counted(running.len() as f64, "pull"))),
                None,
            ),
            stat(
                "Left to pull",
                bytes(left),
                Some(counted(active.len() as f64, "model")),
                None,
            ),
            stat(
                "Time left",
                if rate > 0.0 {
                    remaining(left / rate)
                } else {
                    "-".to_owned()
                },
                Some("at the speed now".to_owned()),
                None,
            ),
        ],
        rows: grouped
            .into_iter()
            .map(|pull| {
                let (status, label) = state_chip(pull.state);
                let failed = pull.state == DownloadState::Failed;
                PullRow {
                    id: pull.id.clone(),
                    group: if pull.state.going() { "going" } else { "ended" },
                    name: format!("{} {}", pull.name, pull.quant),
                    from: pull.source.label(),
                    state: format!("{status}|{label}"),
                    progress: if failed {
                        pull.error.clone().unwrap_or_default()
                    } else {
                        format!("{} of {}", bytes(pull.done), bytes(pull.size))
                    },
                    value: (!failed).then(|| round((pull.done / pull.size) * 1000.0) / 10.0),
                    speed: if pull.state == DownloadState::Running {
                        speed(pull.speed)
                    } else {
                        "-".to_owned()
                    },
                    left: if pull.state == DownloadState::Running && pull.speed > 0.0 {
                        remaining((pull.size - pull.done) / pull.speed)
                    } else {
                        "-".to_owned()
                    },
                    actions: actions_of(pull),
                }
            })
            .collect(),
    }
}
