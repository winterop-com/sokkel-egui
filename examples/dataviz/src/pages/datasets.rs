//! Every dataset, as the web's `pages/Datasets.tsx` lists them: searched, each with its size and
//! source, downloaded as CSV, deleted with the charts drawn from it.

use crate::model::{Dataset, DatasetSource, SavedChart, counted};
use crate::parse::to_csv;

/// The header's count.
pub fn count(datasets: &[Dataset]) -> String {
    counted(datasets.len(), "dataset")
}

/// Where a dataset came from, in words.
pub fn source_label(source: DatasetSource) -> &'static str {
    match source {
        DatasetSource::Seed => "Demo data",
        DatasetSource::File => "File",
        DatasetSource::Paste => "Pasted",
        DatasetSource::Link => "From a link",
    }
}

/// The datasets a search finds, by their name, description and columns' names.
pub fn search<'a>(datasets: &'a [Dataset], query: &str) -> Vec<&'a Dataset> {
    let wanted = crate::js::trim(query).to_lowercase();
    datasets
        .iter()
        .filter(|one| {
            wanted.is_empty() || {
                let columns: Vec<&str> = one.columns.iter().map(|column| column.name.as_str()).collect();
                format!("{} {} {}", one.name, one.description, columns.join(" "))
                    .to_lowercase()
                    .contains(&wanted)
            }
        })
        .collect()
}

/// A dataset's rows as CSV, each column under its name.
pub fn dataset_csv(dataset: &Dataset) -> String {
    let names: Vec<String> = dataset.columns.iter().map(|column| column.name.clone()).collect();
    let rows: Vec<Vec<crate::chart_data::Cell>> = dataset
        .rows
        .iter()
        .map(|row| {
            dataset
                .columns
                .iter()
                .map(|column| crate::chart_data::Cell::text(row.text(&column.id)))
                .collect()
        })
        .collect();
    to_csv(&names, &rows)
}

/// A dataset handed out as CSV, under a file name made from its name.
pub fn download(backend: &mut crate::backend::Backend, dataset: &Dataset) {
    backend.download(
        format!("{}.csv", crate::parse::file_name(&dataset.name)),
        dataset_csv(dataset),
    );
}

/// How many saved charts are drawn from a dataset.
pub fn charts_of(dataset: &Dataset, charts: &[SavedChart]) -> usize {
    charts.iter().filter(|chart| chart.dataset_id == dataset.id).count()
}

/// The question and its detail before a dataset is deleted, naming the charts that go with it.
pub fn delete_question(dataset: &Dataset, charts: &[SavedChart]) -> (String, String) {
    let drawn: Vec<&str> = charts
        .iter()
        .filter(|chart| chart.dataset_id == dataset.id)
        .map(|chart| chart.name.as_str())
        .collect();
    (
        format!("Delete {}?", dataset.name),
        if drawn.is_empty() {
            "Its rows go for good. No saved chart is drawn from it.".to_owned()
        } else {
            format!(
                "Its rows go for good, and so {} drawn from it: {}.",
                if drawn.len() == 1 {
                    "does the chart".to_owned()
                } else {
                    format!("do the {} charts", drawn.len())
                },
                drawn.join(", ")
            )
        },
    )
}
