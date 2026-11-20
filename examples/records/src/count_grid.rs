//! A periodic report's grid of counts, as the web's `CountGrid` in `pages/FormFill.tsx`: every
//! cell kept as it is entered, the questions in rows (a column per cell of the cut, banded, and a
//! total) or in columns (a row per cell under its band, a column per question under its part), a
//! long run searched and narrowed to what is still unfilled.

use eframe::egui::Ui;
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, Cell, Column, DataTable, Editor, Section, Segmented, StackLayout, TableState,
    TableVariant, TableWords, sum_aggregate,
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
    /// The table's active cell, open editor and folded groups, started again when the questions
    /// move, as the web keys its table by where they go.
    pub table: TableState,
}

/// One line of the grid: a question across every cell, or a cell across every question.
struct Line {
    id: String,
    label: String,
    group: Option<String>,
}

/// A count's draft refused before it is kept: not a whole number of 0 or more, or more than a
/// month holds.
fn count_refused(draft: &str) -> Option<String> {
    let plain: String = draft.trim().chars().filter(|c| *c != ',').collect();
    let count: f64 = plain.parse().ok()?;
    if count.fract() != 0.0 || count < 0.0 {
        Some("Counts are whole numbers, 0 or more.".into())
    } else if count > 999.0 {
        Some("A month holds at most 999. Check the count with the station.".into())
    } else {
        None
    }
}

/// A count as the web's `toLocaleString('en-US')` writes it.
fn written(count: f64) -> String {
    sokkel_egui::numbers::locale_number(count)
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
    // what names a cell: its category, or in bands the larger category, as the band names the smaller
    let cells_by = match grid.within {
        None => grid.by.to_owned(),
        Some((within_by, within)) if cells.iter().any(|c| c.band.is_some()) => {
            if grid.categories.len() > within.len() {
                grid.by.to_owned()
            } else {
                within_by.to_owned()
            }
        }
        Some(_) => cut.clone(),
    };
    let lines: Vec<Line> = if in_rows {
        grid.rows
            .iter()
            .map(|q| Line {
                id: q.id.into(),
                label: q.label.into(),
                group: Some(q.part.into()),
            })
            .collect()
    } else {
        cells
            .iter()
            .map(|c| Line {
                id: c.value.clone(),
                label: c.label.clone(),
                group: c.band.clone(),
            })
            .collect()
    };
    // each count column: its id, its head and its band
    let across: Vec<(String, String, Option<String>)> = if in_rows {
        cells
            .iter()
            .map(|c| (c.value.clone(), c.label.clone(), c.band.clone()))
            .collect()
    } else {
        grid.rows
            .iter()
            .map(|q| (q.id.to_owned(), q.label.to_owned(), Some(q.part.to_owned())))
            .collect()
    };
    let key = |line: &str, column: &str| {
        if in_rows {
            cell_key(line, column)
        } else {
            cell_key(column, line)
        }
    };
    let value = |line: &str, column: &str| fill.answers.number(&key(line, column));
    let long = lines.len() >= LONG_RUN;
    let asked = view.query.trim().to_lowercase();
    let shown: Vec<Line> = lines
        .into_iter()
        .filter(|line| {
            let words = format!("{} {}", line.group.clone().unwrap_or_default(), line.label).to_lowercase();
            (!long || asked.is_empty() || words.contains(&asked))
                && (!long || !view.unfilled || across.iter().any(|(id, ..)| value(&line.id, id).is_none()))
        })
        .collect();
    let mut columns = vec![
        Column::new(
            if in_rows {
                "Question".to_owned()
            } else {
                capitalized(&cells_by)
            },
            if in_rows { 224.0 } else { 144.0 },
        )
        .sortable(false),
    ];
    if in_rows {
        columns[0] = columns[0].clone().fill();
    }
    for (_, label, band) in &across {
        let mut column = Column::new(label.clone(), if in_rows { 104.0 } else { 112.0 })
            .numeric()
            .sortable(false)
            .aggregate(sum_aggregate)
            .editable(Editor::Number)
            .validate(count_refused);
        if !in_rows {
            column = column.fill();
        }
        if let Some(band) = band {
            column = column.band(band.clone());
        }
        columns.push(column);
    }
    if in_rows {
        columns.push(
            Column::new("Total", 104.0)
                .numeric()
                .sortable(false)
                .aggregate(sum_aggregate),
        );
    }
    let rows: Vec<Vec<Cell>> = shown
        .iter()
        .map(|line| {
            let counts: Vec<Option<f64>> = across.iter().map(|(id, ..)| value(&line.id, id)).collect();
            let mut row = vec![Cell::Text(line.label.clone())];
            row.extend(
                counts
                    .iter()
                    .map(|count| Cell::Text(count.map(written).unwrap_or_default())),
            );
            if in_rows {
                row.push(Cell::Strong(written(counts.iter().flatten().sum())));
            }
            row
        })
        .collect();
    let grouped = shown.iter().any(|line| line.group.is_some());
    let mut edited = None;
    Section::new("Counts")
        .meta(form.kind.label())
        .description(format!(
            "By {cut}. Each count is kept as it is entered; Tab moves to the next, Enter down."
        ))
        .flush(true)
        .show(ui, |ui| {
            let words = TableWords {
                empty: if view.unfilled && asked.is_empty() {
                    "Every count is filled.".into()
                } else {
                    "Nothing matches.".into()
                },
                ..Default::default()
            };
            let mut query = view.query.clone();
            let mut unfilled = view.unfilled;
            let mut at = usize::from(view.across);
            let mut table = DataTable::new(("counts", form.id), &columns, &rows, &mut view.table)
                .variant(TableVariant::List)
                .keys(shown.iter().map(|line| line.id.clone()))
                .stack_layout(StackLayout::Labeled)
                .stack_below(720.0)
                .fits(true)
                .framed(false)
                .footer(false)
                .height(f32::INFINITY)
                .words(words)
                .toolbar(|ui| {
                    if long {
                        let toggle = Button::new("Unfilled only")
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm)
                            .pressed(unfilled);
                        if ui.add(toggle).clicked() {
                            unfilled = !unfilled;
                        }
                    }
                    ui.add(Segmented::new(&mut at, ["Questions in rows", "Questions in columns"]));
                });
            if grouped {
                table = table.groups(shown.iter().map(|line| line.group.clone().unwrap_or_default()));
            }
            if long {
                table = table.search(&mut query);
            }
            edited = table.show(ui).edited;
            view.query = query;
            view.unfilled = unfilled;
            if (at == 1) != view.across {
                view.across = at == 1;
                view.table = TableState::default();
            }
        });
    if let Some(edit) = edited
        && let (Some(line), Some((column, ..))) = (
            shown.get(edit.row),
            edit.column.checked_sub(1).and_then(|c| across.get(c)),
        )
    {
        let count = edit.value.parse::<f64>().ok();
        fill.answers.set(&key(&line.id, column), count.map(AnswerValue::Number));
    }
    app.grid_view = view;
}

/// The word with its first letter capitalized.
fn capitalized(word: &str) -> String {
    let mut letters = word.chars();
    letters
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + letters.as_str())
        .unwrap_or_default()
}
