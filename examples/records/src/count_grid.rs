//! A periodic report's grid of counts, as the web's `CountGrid` in `pages/FormFill.tsx`: every
//! cell kept as it is entered, the questions in rows (a column per cell of the cut, banded, and a
//! total) or in columns (a row per cell under its band, a column per question under its part), a
//! long run searched and narrowed to what is still unfilled.

use eframe::egui::Ui;
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, EntryColumn, EntryGrid, EntryGridWords, EntryRow, Section, Segmented, Toast,
    ToastKind,
};

use crate::app::RecordsApp;
use crate::form_fill::Fill;
use crate::forms::{AnswerValue, Form, cell_key, grid_cells};

/// A run of counts this long or longer is searched, and can show only what is still unfilled.
const LONG_RUN: usize = 8;

/// How the grid is laid and what it narrows to, kept while the app runs.
#[derive(Clone, Debug, Default)]
pub struct GridView {
    /// The questions in columns rather than rows.
    pub across: bool,
    pub query: String,
    pub unfilled: bool,
}

pub fn show(app: &mut RecordsApp, ui: &mut Ui, form: &Form, fill: &mut Fill) {
    let Some(grid) = &form.grid else { return };
    let cells = grid_cells(grid);
    let mut view = std::mem::take(&mut app.grid_view);
    let in_rows = !view.across;
    let cut = match grid.within {
        None => grid.by.to_owned(),
        Some((by, _)) => format!("{} and {by}", grid.by),
    };
    // the lines: a question across every cell, or a cell across every question
    let lines: Vec<EntryRow> = if in_rows {
        grid.rows
            .iter()
            .map(|q| EntryRow {
                id: q.id.into(),
                label: q.label.into(),
                group: Some(q.part.into()),
            })
            .collect()
    } else {
        cells
            .iter()
            .map(|c| EntryRow {
                id: c.value.clone(),
                label: c.label.clone(),
                group: c.band.clone(),
            })
            .collect()
    };
    let columns: Vec<EntryColumn> = if in_rows {
        cells
            .iter()
            .map(|c| EntryColumn {
                id: c.value.clone(),
                label: c.label.clone(),
                band: c.band.clone(),
            })
            .collect()
    } else {
        grid.rows
            .iter()
            .map(|q| EntryColumn {
                id: q.id.into(),
                label: q.label.into(),
                band: Some(q.part.into()),
            })
            .collect()
    };
    let key = |line: &str, column: &str| {
        if in_rows {
            cell_key(line, column)
        } else {
            cell_key(column, line)
        }
    };
    let answers = fill.answers.clone();
    let value = move |line: &str, column: &str| answers.number(&key(line, column));
    let long = lines.len() >= LONG_RUN;
    let asked = view.query.trim().to_lowercase();
    let shown: Vec<EntryRow> = lines
        .into_iter()
        .filter(|line| {
            let words = format!("{} {}", line.group.clone().unwrap_or_default(), line.label).to_lowercase();
            (!long || asked.is_empty() || words.contains(&asked))
                && (!long || !view.unfilled || columns.iter().any(|c| value(&line.id, &c.id).is_none()))
        })
        .collect();
    let mut edits = Vec::new();
    Section::new("Counts")
        .meta(form.kind.label())
        .description(format!(
            "By {cut}. Each count is kept as it is entered; Tab moves to the next, Enter down."
        ))
        .flush(true)
        .show(ui, |ui| {
            let words = EntryGridWords {
                empty: if view.unfilled && asked.is_empty() {
                    "Every count is filled.".into()
                } else {
                    "Nothing matches.".into()
                },
                ..Default::default()
            };
            let mut query = view.query.clone();
            let mut grid_ui = EntryGrid::new(("counts", form.id, in_rows), &shown, &columns, &value)
                .widths(if in_rows { 224.0 } else { 144.0 }, if in_rows { 104.0 } else { 112.0 })
                .total(in_rows)
                .words(words);
            if long {
                grid_ui = grid_ui.search(&mut query);
            }
            edits = grid_ui.show(ui, |ui| {
                if long {
                    let toggle = Button::new("Unfilled only")
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm)
                        .pressed(view.unfilled);
                    if ui.add(toggle).clicked() {
                        view.unfilled = !view.unfilled;
                    }
                }
                let mut at = usize::from(view.across);
                if ui
                    .add(Segmented::new(&mut at, ["Questions in rows", "Questions in columns"]))
                    .changed()
                {
                    view.across = at == 1;
                }
            });
            view.query = query;
        });
    for edit in edits {
        if edit.value.is_some_and(|v| v > 999.0) {
            Toast::new("Not kept")
                .kind(ToastKind::Error)
                .description("A month holds at most 999. Check the count with the station.")
                .show(ui.ctx());
            continue;
        }
        fill.answers
            .set(&key(&edit.row, &edit.column), edit.value.map(AnswerValue::Number));
    }
    app.grid_view = view;
}
