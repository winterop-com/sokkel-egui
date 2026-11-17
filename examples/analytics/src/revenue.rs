//! The revenue pages, as the web's `Revenue.tsx`: the breakdowns, an arranged board of its own,
//! and the invoices behind them.

use eframe::egui::Ui;
use sokkel_egui::{PageHeader, Segmented};

use crate::app::AnalyticsApp;
use crate::model::BoardId;

impl AnalyticsApp {
    pub fn revenue(&mut self, ui: &mut Ui) {
        let invoices = self.view.path.get(1).is_some_and(|p| p == "invoices");
        let mut tab = usize::from(invoices);
        PageHeader::new("Revenue")
            .description("What the paying teams pay, where it comes from and where it goes.")
            .show(ui, |ui| {
                ui.add(Segmented::new(&mut tab, ["Breakdowns", "Invoices"]));
                if !invoices {
                    self.board_actions(ui, BoardId::Revenue);
                }
            });
        if tab != usize::from(invoices) {
            self.go(if tab == 1 { "revenue/invoices" } else { "revenue" });
        }
        self.view_bar(ui);
        if invoices {
            self.invoices_page(ui);
        } else {
            self.board(ui, BoardId::Revenue);
        }
    }
}
