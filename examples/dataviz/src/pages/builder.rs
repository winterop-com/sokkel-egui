//! Building a chart, as the web's `pages/Builder.tsx` does it: its dataset, view and columns beside
//! a live preview that follows every change, the views suggested for the columns picked, and the
//! chart saved, saved as a new one, shared, downloaded or placed on a board.

use serde::Serialize;

use crate::backend::Backend;
use crate::chart_view::is_charted;
use crate::js;
use crate::model::{
    Aggregate, Board, Column, ColumnType, Dataset, Field, PALETTES, Preferences, SavedChart, SortOrder, ViewKind,
    ViewSpec, counted, counted_as,
};
use crate::parse::file_name;
use crate::shape::{Shaped, Suggestion, column_of, shape, starting_spec, suggest};

/// Which columns a slot takes: names, numbers, dates, regions, or any column a chart can run along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Takes {
    Category,
    Number,
    Date,
    Region,
    Along,
}

impl Takes {
    pub fn id(self) -> &'static str {
        match self {
            Takes::Category => "category",
            Takes::Number => "number",
            Takes::Date => "date",
            Takes::Region => "region",
            Takes::Along => "along",
        }
    }

    /// The column types it takes.
    pub fn types(self) -> &'static [ColumnType] {
        match self {
            Takes::Category => &[ColumnType::Text, ColumnType::Region],
            Takes::Number => &[ColumnType::Number],
            Takes::Date => &[ColumnType::Date],
            Takes::Region => &[ColumnType::Region],
            Takes::Along => &[
                ColumnType::Text,
                ColumnType::Region,
                ColumnType::Date,
                ColumnType::Number,
            ],
        }
    }

    pub fn fits(self, column: &Column) -> bool {
        self.types().contains(&column.kind)
    }
}

/// A column a view reads, in the words it reads it by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub field: Field,
    pub label: &'static str,
    pub takes: Takes,
    pub optional: bool,
    pub description: Option<&'static str>,
}

const fn slot(field: Field, label: &'static str, takes: Takes) -> Slot {
    Slot {
        field,
        label,
        takes,
        optional: false,
        description: None,
    }
}

const fn optional(field: Field, label: &'static str, takes: Takes, description: &'static str) -> Slot {
    Slot {
        field,
        label,
        takes,
        optional: true,
        description: Some(description),
    }
}

/// The columns each view reads.
pub fn slots(view: ViewKind) -> &'static [Slot] {
    use Field::*;
    const BAR: [Slot; 3] = [
        slot(X, "X axis", Takes::Along),
        slot(Y, "Y axis", Takes::Number),
        optional(
            Series,
            "Series",
            Takes::Category,
            "A bar of its own for each, in its own color.",
        ),
    ];
    const LINE: [Slot; 3] = [
        slot(X, "X axis", Takes::Along),
        slot(Y, "Y axis", Takes::Number),
        optional(Series, "Series", Takes::Category, "A line of its own for each."),
    ];
    const AREA: [Slot; 3] = [
        slot(X, "X axis", Takes::Along),
        slot(Y, "Y axis", Takes::Number),
        optional(Series, "Series", Takes::Category, "An area of its own for each."),
    ];
    const SCATTER: [Slot; 4] = [
        slot(X, "X axis", Takes::Number),
        slot(Y, "Y axis", Takes::Number),
        optional(Color, "Color", Takes::Category, "Points colored by their name."),
        optional(Size, "Size", Takes::Number, "Points as large as this figure."),
    ];
    const PIE: [Slot; 2] = [slot(X, "Slices", Takes::Category), slot(Y, "Value", Takes::Number)];
    const HEATMAP: [Slot; 3] = [
        slot(X, "Across", Takes::Along),
        slot(Series, "Down", Takes::Category),
        slot(Y, "Color by", Takes::Number),
    ];
    const MAP: [Slot; 2] = [slot(X, "Region", Takes::Region), slot(Y, "Color by", Takes::Number)];
    const TABLE: [Slot; 3] = [
        slot(X, "Rows", Takes::Along),
        optional(Series, "Columns", Takes::Category, "A column of figures for each."),
        slot(Y, "Value", Takes::Number),
    ];
    const STAT: [Slot; 2] = [
        slot(Y, "Value", Takes::Number),
        optional(
            X,
            "Trend over",
            Takes::Date,
            "Draws the figure’s course and its change on the last step.",
        ),
    ];
    match view {
        ViewKind::Bar => &BAR,
        ViewKind::Line => &LINE,
        ViewKind::Area => &AREA,
        ViewKind::Scatter => &SCATTER,
        ViewKind::Pie => &PIE,
        ViewKind::Heatmap => &HEATMAP,
        ViewKind::Map => &MAP,
        ViewKind::Table => &TABLE,
        ViewKind::Stat => &STAT,
    }
}

/// The views that sort and cut their categories.
pub const SORTED: [ViewKind; 6] = [
    ViewKind::Bar,
    ViewKind::Line,
    ViewKind::Area,
    ViewKind::Pie,
    ViewKind::Heatmap,
    ViewKind::Table,
];

/// The view switched, keeping the columns that still fit it and picking ones that do where they do
/// not.
pub fn fit_view(dataset: &Dataset, spec: &ViewSpec, view: ViewKind) -> ViewSpec {
    let mut next = ViewSpec { view, ..spec.clone() };
    for slot in slots(view) {
        let held = column_of(dataset, spec.field(slot.field));
        if held.is_some_and(|column| slot.takes.fits(column)) {
            continue;
        }
        let used: Vec<String> = slots(view)
            .iter()
            .filter_map(|one| next.field(one.field).map(str::to_owned))
            .collect();
        let pick = dataset
            .columns
            .iter()
            .find(|column| slot.takes.fits(column) && !used.contains(&column.id))
            .or_else(|| dataset.columns.iter().find(|column| slot.takes.fits(column)));
        next.set_field(
            slot.field,
            if slot.optional {
                None
            } else {
                pick.map(|column| column.id.clone())
            },
        );
    }
    if view == ViewKind::Scatter {
        let numbers: Vec<&Column> = dataset
            .columns
            .iter()
            .filter(|column| column.kind == ColumnType::Number)
            .collect();
        if next.x == next.y && numbers.len() > 1 {
            next.y = numbers
                .iter()
                .find(|column| Some(&column.id) != next.x.as_ref())
                .map(|column| column.id.clone());
        }
    }
    if view == ViewKind::Pie && next.limit == 0.0 {
        next.limit = 10.0;
    }
    next
}

/// The draft the builder edits: the chart's name, its dataset and its view.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub name: String,
    pub dataset_id: String,
    pub spec: ViewSpec,
}

/// A column offered in a slot: its id (empty for none), its name and its type.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ColumnOption {
    pub value: String,
    pub label: String,
    pub detail: Option<&'static str>,
}

/// A slot as the builder shows it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SlotField {
    pub field: &'static str,
    pub label: &'static str,
    pub hint: Option<&'static str>,
    pub description: Option<&'static str>,
    pub value: String,
    pub options: Vec<ColumnOption>,
    pub placeholder: Option<String>,
    pub disabled: bool,
}

/// The builder: the chart it edits, if any, and the draft.
#[derive(Clone, Debug, PartialEq)]
pub struct Builder {
    pub chart: Option<SavedChart>,
    pub original: Option<Draft>,
    pub draft: Option<Draft>,
    pub saving: bool,
}

impl Builder {
    /// The builder for a saved chart, or for a new one from a dataset (the first where none is
    /// named or the one named is gone).
    pub fn new(datasets: &[Dataset], prefs: &Preferences, chart: Option<&SavedChart>, dataset: Option<&str>) -> Self {
        let wanted = chart.map(|one| one.dataset_id.as_str()).or(dataset);
        let start = datasets
            .iter()
            .find(|one| Some(one.id.as_str()) == wanted)
            .or(datasets.first());
        let original = chart.map(|chart| Draft {
            name: chart.name.clone(),
            dataset_id: chart.dataset_id.clone(),
            spec: chart.spec.clone(),
        });
        let draft = original.clone().or_else(|| {
            start.map(|one| Draft {
                name: String::new(),
                dataset_id: one.id.clone(),
                spec: starting_spec(one, prefs),
            })
        });
        Builder {
            chart: chart.cloned(),
            original,
            draft,
            saving: false,
        }
    }

    pub fn dataset<'a>(&self, datasets: &'a [Dataset]) -> Option<&'a Dataset> {
        let draft = self.draft.as_ref()?;
        datasets.iter().find(|one| one.id == draft.dataset_id)
    }

    pub fn dirty(&self) -> bool {
        self.draft.is_some() && (self.original.is_none() || self.draft != self.original)
    }

    /// The chart's title: its name, or `Untitled chart`.
    pub fn title(&self) -> String {
        match &self.draft {
            Some(draft) if !js::trim(&draft.name).is_empty() => js::trim(&draft.name).to_owned(),
            _ => "Untitled chart".to_owned(),
        }
    }

    /// The preview's figures.
    pub fn shaped(&self, datasets: &[Dataset]) -> Shaped {
        let empty = Dataset {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            source: crate::model::DatasetSource::Seed,
            columns: Vec::new(),
            rows: Vec::new(),
            updated_at: String::new(),
        };
        let spec = self.draft.as_ref().map_or_else(
            || ViewSpec {
                view: ViewKind::Table,
                aggregate: Aggregate::Count,
                ..ViewSpec::base()
            },
            |draft| draft.spec.clone(),
        );
        shape(self.dataset(datasets).unwrap_or(&empty), &spec, &[])
    }

    /// The views suggested, each fitted to its view; none where the settings turn them off.
    pub fn suggestions(&self, datasets: &[Dataset], prefs: &Preferences) -> Vec<Suggestion> {
        let (Some(dataset), Some(draft)) = (self.dataset(datasets), &self.draft) else {
            return Vec::new();
        };
        if !prefs.suggestions {
            return Vec::new();
        }
        suggest(dataset, &draft.spec)
            .into_iter()
            .map(|one| Suggestion {
                spec: fit_view(dataset, &one.spec, one.spec.view),
                ..one
            })
            .collect()
    }

    /// Why the chart cannot be saved now.
    pub fn save_reason(&self) -> Option<&'static str> {
        let draft = self.draft.as_ref()?;
        if js::trim(&draft.name).is_empty() {
            Some("A chart needs a name.")
        } else if !self.dirty() {
            Some("Nothing has changed yet.")
        } else if self.saving {
            Some("Saving")
        } else {
            None
        }
    }

    /// Why Save is held back, where there is no dataset to chart too.
    pub fn save_held(&self) -> Option<&'static str> {
        if self.draft.is_none() {
            return Some("There is no dataset to chart.");
        }
        self.save_reason()
    }

    /// Why Save as new is held back.
    pub fn save_as_new_held(&self) -> Option<&'static str> {
        let draft = self.draft.as_ref()?;
        if js::trim(&draft.name).is_empty() {
            Some("A chart needs a name.")
        } else if self.saving {
            Some("Saving")
        } else {
            None
        }
    }

    /// The form bar's note.
    pub fn note(&self) -> &'static str {
        if self.chart.is_none() {
            "A new chart, not saved yet"
        } else if self.dirty() {
            "Unsaved changes"
        } else {
            "Saved"
        }
    }

    /// The page's title.
    pub fn header(&self) -> String {
        self.chart
            .as_ref()
            .map_or_else(|| "New chart".to_owned(), |chart| chart.name.clone())
    }

    /// The chart edited as the backend now tells it, so the draft is held against it as saved.
    pub fn refresh(&mut self, charts: &[SavedChart]) {
        let Some(id) = self.chart.as_ref().map(|chart| chart.id.clone()) else {
            return;
        };
        if let Some(chart) = charts.iter().find(|one| one.id == id) {
            self.original = Some(Draft {
                name: chart.name.clone(),
                dataset_id: chart.dataset_id.clone(),
                spec: chart.spec.clone(),
            });
            self.chart = Some(chart.clone());
        }
    }

    pub fn set_name(&mut self, name: &str) {
        if let Some(draft) = &mut self.draft {
            draft.name = name.to_owned();
        }
    }

    /// Changes the draft's spec.
    pub fn set_spec(&mut self, change: impl FnOnce(&mut ViewSpec)) {
        if let Some(draft) = &mut self.draft {
            change(&mut draft.spec);
        }
    }

    /// The view switched, its columns fitted.
    pub fn set_view(&mut self, datasets: &[Dataset], view: ViewKind) {
        let Some(dataset) = self.dataset(datasets).cloned() else {
            return;
        };
        if let Some(draft) = &mut self.draft {
            draft.spec = fit_view(&dataset, &draft.spec, view);
        }
    }

    /// Another dataset, the view started over from its columns.
    pub fn set_dataset(&mut self, datasets: &[Dataset], prefs: &Preferences, id: &str) {
        let Some(next) = datasets.iter().find(|one| one.id == id) else {
            return;
        };
        if let Some(draft) = &mut self.draft {
            draft.dataset_id = id.to_owned();
            draft.spec = starting_spec(next, prefs);
        }
    }

    /// A slot's column picked, or none.
    pub fn set_slot(&mut self, field: Field, value: &str) {
        self.set_spec(|spec| spec.set_field(field, (!value.is_empty()).then(|| value.to_owned())));
    }

    /// The draft put back as saved.
    pub fn discard(&mut self) {
        if let Some(original) = &self.original {
            self.draft = Some(original.clone());
        }
    }

    /// The columns a slot offers.
    pub fn column_options(dataset: &Dataset, slot: &Slot) -> Vec<ColumnOption> {
        let mut out = Vec::new();
        if slot.optional {
            out.push(ColumnOption {
                value: String::new(),
                label: "None".to_owned(),
                detail: None,
            });
        }
        out.extend(
            dataset
                .columns
                .iter()
                .filter(|column| slot.takes.fits(column))
                .map(|column| ColumnOption {
                    value: column.id.clone(),
                    label: column.name.clone(),
                    detail: Some(column.kind.id()),
                }),
        );
        out
    }

    /// The slots of the view drawn, as the builder shows them.
    pub fn slot_fields(&self, datasets: &[Dataset]) -> Vec<SlotField> {
        let (Some(dataset), Some(draft)) = (self.dataset(datasets), &self.draft) else {
            return Vec::new();
        };
        let spec = &draft.spec;
        let counting = spec.aggregate == Aggregate::Count;
        slots(spec.view)
            .iter()
            .map(|slot| {
                let options = Self::column_options(dataset, slot);
                let value = spec.field(slot.field).unwrap_or("").to_owned();
                let by_count = slot.field == Field::Y && spec.view != ViewKind::Scatter && counting;
                SlotField {
                    field: slot.field.id(),
                    label: slot.label,
                    hint: slot.optional.then_some("Optional"),
                    description: if by_count {
                        Some("Count counts the rows, so no column is read.")
                    } else {
                        slot.description
                    },
                    value: if options.iter().any(|one| one.value == value) {
                        value
                    } else {
                        String::new()
                    },
                    placeholder: if options.is_empty() {
                        Some(format!(
                            "No {}columns",
                            if slot.takes == Takes::Along {
                                String::new()
                            } else {
                                format!("{} ", slot.takes.id())
                            }
                        ))
                    } else if slot.optional {
                        Some("None".to_owned())
                    } else {
                        None
                    },
                    disabled: options.is_empty() || by_count,
                    options,
                }
            })
            .collect()
    }

    /// The Combine control: its value, and whether the dataset has numbers to combine.
    pub fn combine(&self, datasets: &[Dataset]) -> Option<(Aggregate, bool)> {
        let (dataset, draft) = (self.dataset(datasets)?, self.draft.as_ref()?);
        if draft.spec.view == ViewKind::Scatter {
            return None;
        }
        let numbers = dataset.columns.iter().any(|column| column.kind == ColumnType::Number);
        Some((
            if numbers {
                draft.spec.aggregate
            } else {
                Aggregate::Count
            },
            numbers,
        ))
    }

    /// Whether the dates' grouping shows: a date along the X axis, and not a scatter.
    pub fn shows_bin(&self, datasets: &[Dataset]) -> bool {
        let (Some(dataset), Some(draft)) = (self.dataset(datasets), &self.draft) else {
            return false;
        };
        column_of(dataset, draft.spec.x.as_deref()).is_some_and(|x| x.kind == ColumnType::Date)
            && draft.spec.view != ViewKind::Scatter
    }

    /// The sort's labels where the view sorts: `In time` for a date's own order.
    pub fn sort_labels(&self, datasets: &[Dataset]) -> Option<Vec<&'static str>> {
        let (dataset, draft) = (self.dataset(datasets)?, self.draft.as_ref()?);
        if !SORTED.contains(&draft.spec.view) {
            return None;
        }
        let dated = column_of(dataset, draft.spec.x.as_deref()).is_some_and(|x| x.kind == ColumnType::Date);
        Some(
            SortOrder::ALL
                .iter()
                .map(|one| {
                    if *one == SortOrder::Natural && dated {
                        "In time"
                    } else {
                        one.label()
                    }
                })
                .collect(),
        )
    }

    /// The Colors control's label for the settings' own palette.
    pub fn default_palette_label(prefs: &Preferences) -> String {
        format!(
            "As in settings ({})",
            PALETTES
                .iter()
                .find(|one| one.id == prefs.palette)
                .map_or("Theme", |one| one.label)
        )
    }

    /// The stack switch where it shows: checked, and held where there are no series.
    pub fn stack_switch(&self) -> Option<(bool, bool)> {
        let spec = &self.draft.as_ref()?.spec;
        matches!(spec.view, ViewKind::Bar | ViewKind::Area).then_some((spec.stack, spec.series.is_none()))
    }

    /// The bars-lying-down switch where it shows.
    pub fn horizontal_switch(&self) -> Option<bool> {
        let spec = &self.draft.as_ref()?.spec;
        (spec.view == ViewKind::Bar).then_some(spec.horizontal)
    }

    /// The note under the preview.
    pub fn preview_note(&self, datasets: &[Dataset]) -> Option<String> {
        let (dataset, draft) = (self.dataset(datasets)?, self.draft.as_ref()?);
        let shaped = self.shaped(datasets);
        let shown = match &shaped.order {
            None => String::new(),
            Some(order) => format!(
                ", {} shown",
                if draft.spec.view == ViewKind::Map {
                    counted(order.len(), "region")
                } else {
                    counted_as(order.len(), "category", "categories")
                }
            ),
        };
        Some(format!(
            "Reads {} of {}{shown}. The preview follows every change.",
            counted(shaped.read, "row"),
            dataset.name
        ))
    }

    /// Whether the view drawn exports as a picture.
    pub fn charted(&self) -> bool {
        self.draft.as_ref().is_some_and(|draft| is_charted(draft.spec.view))
    }

    /// The boards Add to board offers: those the chart is not on.
    pub fn other_boards<'a>(&self, boards: &'a [Board]) -> Vec<&'a Board> {
        boards
            .iter()
            .filter(|board| match &self.chart {
                None => true,
                Some(chart) => !board.items.iter().any(|item| item.chart_id == chart.id),
            })
            .collect()
    }

    /// Saves the draft: the chart edited, or a new one.
    pub fn save(&mut self, backend: &mut Backend, as_new: bool) {
        let Some(draft) = &self.draft else { return };
        if js::trim(&draft.name).is_empty() {
            return;
        }
        backend.save_chart(self.chart.as_ref(), &draft.name, &draft.dataset_id, &draft.spec, as_new);
    }

    /// The preview's figures handed out as CSV.
    pub fn download_csv(&self, backend: &mut Backend, datasets: &[Dataset]) {
        let shaped = self.shaped(datasets);
        backend.download_csv(&file_name(&self.title()), &shaped.table.names, &shaped.table.rows);
    }
}
