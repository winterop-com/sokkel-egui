//! A chart opened from a shared link, as the web's `pages/Shared.tsx` shows it: rebuilt from the
//! data and view the link carries, then kept here if wanted.

use crate::model::{Dataset, counted};
use crate::share::SharedChart;

/// The header's description.
pub fn description(dataset: &Dataset) -> String {
    format!(
        "Opened from a link, which carries the chart and the {} of {} it draws. Nothing here is saved until you keep it.",
        counted(dataset.rows.len(), "row"),
        dataset.name
    )
}

/// The page's title: the chart's name.
pub fn title(shared: &SharedChart) -> &str {
    &shared.name
}
