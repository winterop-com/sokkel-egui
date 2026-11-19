//! Every saved chart, as the web's `pages/Charts.tsx` lists them: searched, each with the boards it
//! is on, copied, shared, deleted.

use crate::model::{Board, Dataset, SavedChart, counted};

/// The header's count.
pub fn count(charts: &[SavedChart]) -> String {
    counted(charts.len(), "chart")
}

/// The charts a search finds, by their name, their dataset's and their view.
pub fn search<'a>(charts: &'a [SavedChart], datasets: &[Dataset], query: &str) -> Vec<&'a SavedChart> {
    let wanted = crate::js::trim(query).to_lowercase();
    charts
        .iter()
        .filter(|chart| {
            wanted.is_empty() || {
                let dataset = datasets
                    .iter()
                    .find(|one| one.id == chart.dataset_id)
                    .map_or("", |one| one.name.as_str());
                format!("{} {dataset} {}", chart.name, chart.spec.view.id())
                    .to_lowercase()
                    .contains(&wanted)
            }
        })
        .collect()
}

/// The boards a chart is on.
pub fn on_boards<'a>(chart: &SavedChart, boards: &'a [Board]) -> Vec<&'a Board> {
    boards
        .iter()
        .filter(|board| board.items.iter().any(|item| item.chart_id == chart.id))
        .collect()
}

/// The Boards column's words for a chart.
pub fn boards_cell(chart: &SavedChart, boards: &[Board]) -> String {
    let names: Vec<&str> = on_boards(chart, boards)
        .iter()
        .map(|board| board.name.as_str())
        .collect();
    if names.is_empty() {
        "-".to_owned()
    } else {
        names.join(", ")
    }
}

/// The question and its detail before a chart is deleted.
pub fn delete_question(chart: &SavedChart, boards: &[Board]) -> (String, String) {
    let on = on_boards(chart, boards);
    (
        format!("Delete {}?", chart.name),
        if on.is_empty() {
            "Its dataset stays.".to_owned()
        } else {
            format!(
                "It leaves {} too. Its dataset stays.",
                on.iter()
                    .map(|board| board.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
    )
}

/// A chart card's description: its dataset, or that it was deleted.
pub fn card_description(chart: &SavedChart, datasets: &[Dataset]) -> String {
    datasets
        .iter()
        .find(|one| one.id == chart.dataset_id)
        .map_or_else(|| "Its dataset was deleted".to_owned(), |one| one.name.clone())
}
