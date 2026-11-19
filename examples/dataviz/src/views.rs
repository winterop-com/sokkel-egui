//! The views a chart is drawn as, in the builder's order, each with what it suits, as the web's
//! `views.ts` lists them; each view's icon is the UI's.

use crate::model::ViewKind;

/// A view the builder offers: its kind, its label and what it suits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct View {
    pub value: ViewKind,
    pub label: &'static str,
    pub description: &'static str,
}

pub const VIEWS: [View; 9] = [
    View {
        value: ViewKind::Bar,
        label: "Bar",
        description: "Figures by category, side by side or stacked",
    },
    View {
        value: ViewKind::Line,
        label: "Line",
        description: "Figures over time or along an order",
    },
    View {
        value: ViewKind::Area,
        label: "Area",
        description: "Lines with the area under them filled, alone or stacked",
    },
    View {
        value: ViewKind::Scatter,
        label: "Scatter",
        description: "One number against another, a point per row",
    },
    View {
        value: ViewKind::Pie,
        label: "Pie",
        description: "Each category’s share of the whole",
    },
    View {
        value: ViewKind::Heatmap,
        label: "Heatmap",
        description: "A figure for each pair of categories, shaded",
    },
    View {
        value: ViewKind::Map,
        label: "Map",
        description: "Regions shaded by their figure",
    },
    View {
        value: ViewKind::Table,
        label: "Table",
        description: "The figures as rows and columns",
    },
    View {
        value: ViewKind::Stat,
        label: "Figure",
        description: "One figure, with its course over time",
    },
];

/// A view's label.
pub fn view_label(view: ViewKind) -> &'static str {
    VIEWS
        .iter()
        .find(|one| one.value == view)
        .map_or(view.id(), |one| one.label)
}
