//! A board, as the web's `pages/Board.tsx` keeps it: its saved charts in a grid, filtered together
//! by the columns their datasets share, edited in a draft until Done keeps it, a chart taken off
//! with an Undo, copied, and presented full screen.

use serde::Serialize;
use sokkel_egui::Filter;

use crate::backend::Backend;
use crate::model::{Board, BoardItem, ColumnType, Dataset, PanelSpan, SavedChart, counted};
use crate::parse::file_name;
use crate::shape::{FieldSpec, filters_for, naturally, shape, typed_rows};
use sokkel_egui_charts::data::Cell;

/// The filters a board offers: every column of its charts' datasets, once by name, with the values
/// a name holds across all of them.
pub fn board_fields(datasets: &[&Dataset]) -> Vec<FieldSpec> {
    let mut fields: Vec<(String, FieldSpec, Vec<String>)> = Vec::new();
    for dataset in datasets {
        let rows = typed_rows(dataset);
        for column in &dataset.columns {
            let key = column.name.to_lowercase();
            let at = match fields.iter().position(|(one, _, _)| *one == key) {
                Some(at) => at,
                None => {
                    let kind = match column.kind {
                        ColumnType::Number => "number",
                        ColumnType::Date => "date",
                        _ => "set",
                    };
                    fields.push((
                        key,
                        FieldSpec {
                            id: column.name.clone(),
                            label: column.name.clone(),
                            kind,
                            options: None,
                        },
                        Vec::new(),
                    ));
                    fields.len() - 1
                }
            };
            let (_, field, values) = &mut fields[at];
            if field.kind == "set" {
                for row in &rows {
                    if let Cell::Text(value) = row.get(&column.id)
                        && !values.contains(value)
                    {
                        values.push(value.clone());
                    }
                }
            }
        }
    }
    fields
        .into_iter()
        .map(|(_, mut field, mut values)| {
            if field.kind == "set" {
                if values.len() > 60 {
                    field.kind = "text";
                } else {
                    values.sort_by(|a, b| naturally(a, b));
                    field.options = Some(values);
                }
            }
            field
        })
        .collect()
}

/// A board's item with its chart and its chart's dataset, either gone.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed<'a> {
    pub item: BoardItem,
    pub chart: Option<&'a SavedChart>,
    pub dataset: Option<&'a Dataset>,
}

/// How a board's item is drawn.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Panel {
    /// The chart, its subtitle saying its dataset and how the filters reach it.
    Chart {
        title: String,
        subtitle: String,
        #[serde(serialize_with = "crate::model::filters_json::serialize")]
        filters: Vec<Filter>,
        file: String,
    },
    /// A chart or its dataset since deleted.
    Missing { title: String, text: String },
}

/// The board page's own state: the filters as last set, Edit's draft, and the items last saved
/// outside Edit until the backend answers.
#[derive(Clone, Debug, PartialEq)]
pub struct BoardPage {
    pub board: String,
    pub filters: Vec<Filter>,
    pub draft: Option<Vec<BoardItem>>,
    pub opened: bool,
}

impl BoardPage {
    /// The page as it opens on a board: its filters as kept, and Edit open where it has no charts
    /// and there are saved charts to drag onto it.
    pub fn new(board: Option<&Board>, charts: &[SavedChart]) -> Self {
        let draft = board
            .filter(|board| board.items.is_empty() && !charts.is_empty())
            .map(|board| board.items.clone());
        BoardPage {
            board: board.map(|one| one.id.clone()).unwrap_or_default(),
            filters: board.map(|one| one.filters.clone()).unwrap_or_default(),
            draft,
            opened: true,
        }
    }

    pub fn arranging(&self) -> bool {
        self.draft.is_some()
    }

    /// The items shown: Edit's draft, or the board's.
    pub fn items(&self, board: &Board) -> Vec<BoardItem> {
        self.draft.clone().unwrap_or_else(|| board.items.clone())
    }

    pub fn placed<'a>(&self, board: &Board, charts: &'a [SavedChart], datasets: &'a [Dataset]) -> Vec<Placed<'a>> {
        self.items(board)
            .into_iter()
            .map(|item| {
                let chart = charts.iter().find(|one| one.id == item.chart_id);
                let dataset = chart.and_then(|chart| datasets.iter().find(|one| one.id == chart.dataset_id));
                Placed { item, chart, dataset }
            })
            .collect()
    }

    /// The datasets the board's charts read, each once.
    pub fn used<'a>(placed: &[Placed<'a>]) -> Vec<&'a Dataset> {
        let mut out: Vec<&Dataset> = Vec::new();
        for entry in placed {
            if let Some(dataset) = entry.dataset
                && !out.iter().any(|one| one.id == dataset.id)
            {
                out.push(dataset);
            }
        }
        out
    }

    /// How an item is drawn under the board's filters.
    pub fn panel(&self, entry: &Placed) -> Panel {
        match (entry.chart, entry.dataset) {
            (Some(chart), Some(dataset)) => {
                let own = filters_for(dataset, &self.filters);
                let left = self.filters.len() - own.len();
                let reaching = if left == 0 {
                    String::new()
                } else {
                    format!(
                        " · {left} {} not apply",
                        if left == 1 { "filter does" } else { "filters do" }
                    )
                };
                Panel::Chart {
                    title: chart.name.clone(),
                    subtitle: format!(
                        "{}{}{reaching}",
                        dataset.name,
                        if own.is_empty() { "" } else { " · filtered" }
                    ),
                    filters: own,
                    file: file_name(&chart.name),
                }
            }
            (chart, _) => Panel::Missing {
                title: chart.map_or_else(|| "A deleted chart".to_owned(), |chart| chart.name.clone()),
                text: if chart.is_none() {
                    "This chart was deleted. Remove it from the board."
                } else {
                    "The dataset this chart reads was deleted."
                }
                .to_owned(),
            },
        }
    }

    /// The saved charts not on the board yet, by name, offered to drag onto it.
    pub fn palette<'a>(&self, board: &Board, charts: &'a [SavedChart]) -> Vec<&'a SavedChart> {
        let items = self.items(board);
        let mut out: Vec<&SavedChart> = charts
            .iter()
            .filter(|chart| !items.iter().any(|item| item.chart_id == chart.id))
            .collect();
        out.sort_by(|a, b| crate::js::locale_compare(&a.name, &b.name, false));
        out
    }

    /// The words beside the filter bar.
    pub fn filter_note(&self) -> String {
        if self.filters.is_empty() {
            "Filters here reach every chart whose dataset has the column.".to_owned()
        } else {
            format!("{}, kept with the board", counted(self.filters.len(), "filter"))
        }
    }

    /// The header's count.
    pub fn count(&self, board: &Board) -> String {
        counted(self.items(board).len(), "chart")
    }

    /// The header's description: how to arrange while editing, else the board's own.
    pub fn description(&self, board: &Board) -> Option<String> {
        if self.arranging() {
            Some("Drag a chart onto the board, move a panel by its grip, set its width or take it off.".to_owned())
        } else if board.description.is_empty() {
            None
        } else {
            Some(board.description.clone())
        }
    }

    /// The filters set, kept with the board.
    pub fn set_filters(&mut self, backend: &mut Backend, filters: Vec<Filter>) {
        backend.set_board_filters(&self.board, &filters);
        self.filters = filters;
    }

    /// Edit opened, its draft the items shown.
    pub fn edit(&mut self, board: &Board) {
        self.draft = Some(board.items.clone());
    }

    /// A change in Edit's draft.
    pub fn change(&mut self, items: Vec<BoardItem>) {
        self.draft = Some(items);
    }

    /// A saved chart dragged in at `index`.
    pub fn add(&mut self, backend: &mut Backend, board: &Board, chart: &str, span: PanelSpan, index: usize) {
        let mut items = self.items(board);
        let item = BoardItem {
            id: backend.fresh_id("it"),
            chart_id: chart.to_owned(),
            span,
        };
        items.insert(index.min(items.len()), item);
        self.draft = Some(items);
    }

    /// Edit's draft kept, where it differs from the board's items.
    pub fn done(&mut self, backend: &mut Backend, board: &Board) {
        if let Some(draft) = self.draft.take() {
            let same = draft.len() == board.items.len()
                && draft
                    .iter()
                    .zip(&board.items)
                    .all(|(a, b)| a.id == b.id && a.span == b.span);
            if !same {
                backend.save_board_items(&board.id, &draft);
            }
        }
    }

    pub fn cancel(&mut self) {
        self.draft = None;
    }

    /// A chart taken off: from the draft while editing, else kept at once with an Undo.
    pub fn take_off(&mut self, backend: &mut Backend, board: &Board, entry: &Placed) {
        let items = self.items(board);
        if self.arranging() {
            self.draft = Some(items.into_iter().filter(|item| item.id != entry.item.id).collect());
            return;
        }
        backend.take_off(
            board,
            &items,
            &entry.item.id,
            entry.chart.map(|chart| chart.name.as_str()),
        );
    }

    /// A chart's figures under the board's filters, handed out as CSV.
    pub fn download_csv(&self, backend: &mut Backend, entry: &Placed) {
        let (Some(chart), Some(dataset)) = (entry.chart, entry.dataset) else {
            return;
        };
        let shaped = shape(dataset, &chart.spec, &filters_for(dataset, &self.filters));
        backend.download_csv(&file_name(&chart.name), &shaped.table.names, &shaped.table.rows);
    }

    /// A copy of the board with the items and filters shown.
    pub fn duplicate(&self, backend: &mut Backend, board: &Board) {
        backend.duplicate_board(board, &self.items(board), &self.filters);
    }
}

/// The question and its detail before a board is deleted.
pub fn delete_question(board: Option<&Board>) -> (String, &'static str) {
    (
        format!("Delete {}?", board.map_or("the board", |one| one.name.as_str())),
        "The board goes; the charts on it stay saved.",
    )
}

/// The page's message where the board is gone.
pub const NO_BOARD: &str = "There is no such board here. It may have been deleted.";
