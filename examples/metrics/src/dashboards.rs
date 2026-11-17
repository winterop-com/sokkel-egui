//! Every dashboard, as the web's `Dashboards.tsx`: open one, make one, put them in the order the
//! rail lists them, or delete one.

use eframe::egui::Ui;
use sokkel_egui::{Button, ButtonSize, Cell, Column, DataTable, Icon, MenuItem, PageHeader, TableVariant};

use crate::MetricsApp;
use crate::board::{Asking, DetailsForm};

impl MetricsApp {
    /// The Dashboards page: its head with New, and the dashboards as a list.
    pub(crate) fn dashboards_page(&mut self, ui: &mut Ui) {
        let count = self.dashboards.len();
        let mut new = false;
        PageHeader::new("Dashboards")
            .count(format!(
                "{count} {}",
                if count == 1 { "dashboard" } else { "dashboards" }
            ))
            .show(ui, |ui| {
                new = ui
                    .add(Button::new("New").icon(Icon::Plus).size(ButtonSize::Sm))
                    .clicked();
            });
        if new {
            self.details = Some(DetailsForm::default());
        }
        // the web's `minmax(12rem,1fr)`, `minmax(16rem,2fr)` and `6rem`, less the 12 between columns
        let columns = [
            Column::new("Title", 180.0).fill(),
            Column::new("Description", 244.0).fill_share(2.0).sortable(false),
            Column::new("Panels", 84.0).numeric(),
        ];
        let rows: Vec<Vec<Cell>> = self
            .dashboards
            .iter()
            .map(|b| {
                vec![
                    Cell::Strong(b.title.clone()),
                    Cell::Muted(b.description.clone()),
                    Cell::Number {
                        value: b.panels.len() as f64,
                        text: b.panels.len().to_string(),
                    },
                ]
            })
            .collect();
        let state = &mut self.list;
        let out = DataTable::new("dashboards", &columns, &rows, state)
            .variant(TableVariant::List)
            .reorder(true)
            .stretch(true)
            .footer(false)
            .actions(|_| {
                vec![
                    MenuItem::action("Open").icon(Icon::ExternalLink),
                    MenuItem::Separator,
                    MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
                ]
            })
            .show(ui);
        if let Some((from, to)) = out.reordered {
            sokkel_egui::move_item(&mut self.dashboards, from, to);
        }
        let open = |app: &mut Self, at: usize| {
            if let Some(board) = app.dashboards.get(at) {
                app.place = board.id.clone();
            }
        };
        match out.action {
            Some((at, 0)) => open(self, at),
            Some((at, 2)) => {
                if let Some(board) = self.dashboards.get(at) {
                    self.asking = Some(Asking::DeleteBoard(board.id.clone()));
                }
            }
            _ => {}
        }
        if let Some(at) = self.list.opened.take() {
            open(self, at);
        }
    }
}
