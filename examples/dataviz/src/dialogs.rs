//! The dialogs several pages open, as the web's `dialogs.tsx` words and fills them: sharing a chart
//! as a link, making a board or changing its details, and placing a saved chart on a board.

use crate::js;
use crate::model::{Board, Dataset, PanelSpan, SavedChart};

/// The share dialog's description.
pub fn share_description(name: &str, dataset: &Dataset) -> String {
    format!(
        "A link to {name}. It carries the {} rows of {} and how the chart draws them, so whoever opens it sees the same chart, saved or not.",
        dataset.rows.len(),
        dataset.name
    )
}

/// What the share dialog's link field shows while the chart is packed.
pub const PACKING: &str = "Packing the chart";

/// The share dialog's foot: how long the link is, in kB.
pub fn link_size(link: &str) -> String {
    format!(
        "The link is {} kB long.",
        js::to_fixed(link.encode_utf16().count() as f64 / 1024.0, 1)
    )
}

/// The widths a chart takes on a board, in the pickers' order.
pub const SPANS: [PanelSpan; 4] = PanelSpan::ALL;

/// The board dialog's title: making one, or changing one's details.
pub fn board_title(editing: bool) -> &'static str {
    if editing { "Board details" } else { "New board" }
}

/// The board dialog's description, naming the chart a new board starts with.
pub fn board_description(adding: Option<&SavedChart>) -> String {
    match adding {
        None => "Saved charts arranged on one page, with filters across all of them.".to_owned(),
        Some(chart) => format!("A board that starts with {}.", chart.name),
    }
}

/// The board dialog's confirm: its label.
pub fn board_confirm(editing: bool) -> &'static str {
    if editing { "Save" } else { "Make" }
}

/// Why the board dialog cannot be confirmed yet.
pub fn board_why(name: &str) -> Option<&'static str> {
    js::trim(name).is_empty().then_some("A board needs a name.")
}

/// A saved chart offered to place on a board.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Offered {
    pub value: String,
    pub label: String,
    pub description: String,
}

/// The saved charts offered for a board: those not on it first, each group by name.
pub fn offered(board: &Board, charts: &[SavedChart], datasets: &[Dataset]) -> Vec<Offered> {
    let placed = |chart: &SavedChart| board.items.iter().any(|item| item.chart_id == chart.id);
    let mut sorted: Vec<&SavedChart> = charts.iter().collect();
    sorted.sort_by(|a, b| {
        placed(a)
            .cmp(&placed(b))
            .then_with(|| js::locale_compare(&a.name, &b.name, false))
    });
    sorted
        .into_iter()
        .map(|chart| Offered {
            value: chart.id.clone(),
            label: chart.name.clone(),
            description: format!(
                "{}{}",
                datasets
                    .iter()
                    .find(|one| one.id == chart.dataset_id)
                    .map_or("A dataset since deleted", |one| one.name.as_str()),
                if placed(chart) { " · on this board already" } else { "" }
            ),
        })
        .collect()
}

/// The chart the dialog adds: the one picked, or the first not on the board yet, or the first.
pub fn chosen(picked: Option<&str>, board: &Board, charts: &[SavedChart], datasets: &[Dataset]) -> String {
    if let Some(picked) = picked {
        return picked.to_owned();
    }
    let offered = offered(board, charts, datasets);
    let placed = |id: &str| board.items.iter().any(|item| item.chart_id == id);
    offered
        .iter()
        .find(|one| !placed(&one.value))
        .or(offered.first())
        .map_or_else(String::new, |one| one.value.clone())
}

/// The add dialog's description.
pub fn add_description(board: &Board) -> String {
    format!("A saved chart to place on {}.", board.name)
}

/// Why a chart cannot be added yet.
pub fn add_why(chosen: &str) -> Option<&'static str> {
    chosen.is_empty().then_some("Save a chart first: there is none to add.")
}
