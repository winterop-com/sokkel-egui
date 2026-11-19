//! What the viewer watched, as the web's `pages/History.tsx` shows it: newest first by day, each
//! where they stopped; searched, removed one by one, cleared, or paused.

use serde::Serialize;

use crate::catalog::video_by_id;
use crate::data::{DAY, Data, PrefsPatch, Then, Toast};
use crate::format::plural;
use crate::js::trim;
use crate::site::video_text;

/// Which day a watch falls in, said as a person says it; days begin at midnight UTC.
pub fn day_of(at: i64, now: i64) -> &'static str {
    let midnight = now.div_euclid(DAY) * DAY;
    if at >= midnight {
        "Today"
    } else if at >= midnight - DAY {
        "Yesterday"
    } else if at >= midnight - 6 * DAY {
        "This week"
    } else {
        "Earlier"
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Day {
    pub label: &'static str,
    pub videos: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HistoryView {
    pub loading: bool,
    pub count: String,
    pub clear_disabled: bool,
    pub keep: bool,
    pub days: Vec<Day>,
    /// What it says when it shows nothing: its title and words.
    pub empty: Option<(&'static str, String)>,
}

/// What History shows, searched for `search`.
pub fn view(data: &Data, search: &str) -> HistoryView {
    let rows = data.history.seen();
    let wanted = trim(search).to_lowercase();
    let mut entries: Vec<_> = rows
        .iter()
        .filter_map(|row| video_by_id(&row.id).map(|video| (row, video)))
        .filter(|(_, video)| wanted.is_empty() || video_text(video).to_lowercase().contains(&wanted))
        .collect();
    entries.sort_by_key(|(row, _)| std::cmp::Reverse(row.at));
    let mut days: Vec<Day> = Vec::new();
    for (row, video) in &entries {
        let label = day_of(row.at, data.now());
        match days.iter_mut().find(|one| one.label == label) {
            Some(day) => day.videos.push(video.id),
            None => days.push(Day {
                label,
                videos: vec![video.id],
            }),
        }
    }
    let empty = entries.is_empty().then(|| {
        (
            if search.is_empty() {
                "Nothing watched yet"
            } else {
                "Nothing found"
            },
            if !search.is_empty() {
                format!("Nothing in your history matches \"{search}\".")
            } else if data.prefs.history {
                "Videos you watch land here, with where you stopped.".to_owned()
            } else {
                "History is paused: turn Keep history on to remember what you watch.".to_owned()
            },
        )
    });
    HistoryView {
        loading: !data.history.ready(),
        count: plural(rows.len() as f64, "video", None),
        clear_disabled: rows.is_empty(),
        keep: data.prefs.history,
        days,
        empty,
    }
}

/// A card's Remove from history.
pub fn remove(data: &mut Data, id: &str) {
    data.remove_watched(id, Some(Toast::plain("Removed from history")));
}

/// Clear all's confirm: every row the page shows goes, and once all are gone it says so; opened
/// from the palette (`clearing`), it goes back to the history.
pub fn clear(data: &mut Data, clearing: bool) {
    let ids: Vec<String> = data.history.seen().iter().map(|row| row.id.clone()).collect();
    for id in &ids {
        data.remove_watched(id, None);
    }
    // Once every removal is answered; with none to make, at once.
    let said = Toast::plain("Watch history cleared");
    if ids.is_empty() {
        data.toast(said);
    } else {
        data.then(Then::Toast(said));
    }
    if clearing {
        data.go("/history");
    }
}

/// Keep history's switch.
pub fn keep_history(data: &mut Data, on: bool) {
    data.set_prefs(&PrefsPatch {
        history: Some(on),
        ..PrefsPatch::default()
    });
}
