//! The overview, as the web's `Overview.tsx` and `arrange.tsx`: the key figures and panels as the
//! person arranged them, under the bar of the range and segment every panel reads.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    ArrangeItem, Button, ButtonSize, DashboardArrange, DashboardEditActions, DashboardWidget, Empty, Icon, PageHeader,
    PanelSpan, TimeRangePicker, variable_select, variable_toggle,
};

use crate::app::AnalyticsApp;
use crate::days::day_of;
use crate::model::{BoardId, PLANS, Placed, REGIONS, WIDGETS, WidgetId, default_layout};
use crate::panels::{kpi_name, widget_span};
use crate::view::{PRESETS, Range, range_name};

impl WidgetId {
    /// As the backend keeps it.
    pub fn key(self) -> &'static str {
        match self {
            WidgetId::Revenue => "revenue",
            WidgetId::Mrr => "mrr",
            WidgetId::Arr => "arr",
            WidgetId::Active => "active",
            WidgetId::Paying => "paying",
            WidgetId::Arpa => "arpa",
            WidgetId::Signups => "signups",
            WidgetId::Conversion => "conversion",
            WidgetId::Churn => "churn",
            WidgetId::Refunded => "refunded",
            WidgetId::RevenueChart => "revenue-chart",
            WidgetId::Plans => "plans",
            WidgetId::Funnel => "funnel",
            WidgetId::Countries => "countries",
            WidgetId::Feed => "feed",
            WidgetId::Customers => "customers",
            WidgetId::Movements => "movements",
            WidgetId::Regions => "regions",
            WidgetId::Cohorts => "cohorts",
        }
    }

    /// Its name in the palette and on the grid while arranging.
    pub fn label(self) -> &'static str {
        match self {
            WidgetId::RevenueChart => "Revenue over time",
            WidgetId::Plans => "Revenue by plan",
            WidgetId::Funnel => "Sign-up funnel",
            WidgetId::Countries => "Revenue by country",
            WidgetId::Feed => "Live feed",
            WidgetId::Customers => "Top customers",
            WidgetId::Movements => "MRR movements",
            WidgetId::Regions => "Revenue by region",
            WidgetId::Cohorts => "Retention by cohort",
            kpi => kpi_name(kpi),
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            WidgetId::Revenue => Icon::Banknote,
            WidgetId::Mrr => Icon::Repeat,
            WidgetId::Arr => Icon::CalendarClock,
            WidgetId::Active => Icon::Users,
            WidgetId::Paying => Icon::CreditCard,
            WidgetId::Arpa => Icon::UserRound,
            WidgetId::Signups => Icon::UserPlus,
            WidgetId::Conversion => Icon::Percent,
            WidgetId::Churn => Icon::UserMinus,
            WidgetId::Refunded => Icon::Undo2,
            WidgetId::RevenueChart => Icon::ChartArea,
            WidgetId::Plans => Icon::ChartPie,
            WidgetId::Funnel => Icon::ListFilter,
            WidgetId::Countries => Icon::Earth,
            WidgetId::Feed => Icon::Activity,
            WidgetId::Customers => Icon::Trophy,
            WidgetId::Movements => Icon::ChartColumn,
            WidgetId::Regions => Icon::ChartBar,
            WidgetId::Cohorts => Icon::Grid3x3,
        }
    }
}

impl ArrangeItem for Placed {
    fn id(&self) -> &str {
        self.id.key()
    }

    fn span(&self) -> PanelSpan {
        self.span
    }

    fn set_span(&mut self, span: PanelSpan) {
        self.span = span;
    }
}

impl AnalyticsApp {
    pub fn overview(&mut self, ui: &mut Ui) {
        let description = format!(
            "{}'s sales, {}{}.",
            self.prefs.workspace,
            range_name(&self.view.range).to_lowercase(),
            if self.view.compare {
                ", against the period before"
            } else {
                ""
            }
        );
        PageHeader::new("Overview")
            .description(description)
            .show(ui, |ui| self.board_actions(ui, BoardId::Overview));
        self.view_bar(ui);
        self.board(ui, BoardId::Overview);
    }

    /// The page header's actions: Edit, or Reset, Cancel and Done while editing.
    pub fn board_actions(&mut self, ui: &mut Ui, board: BoardId) {
        let default = default_layout(board);
        self.layout_of(board);
        let kept = self.layouts.get_mut(&board).expect("the layout was just made");
        DashboardEditActions::new(&mut self.edit, kept)
            .default_layout(&default)
            .show(ui);
    }

    /// The bar of the range, the comparison and the segment every panel of a page reads.
    pub fn view_bar(&mut self, ui: &mut Ui) {
        let plans: Vec<String> = std::iter::once("All plans".to_owned())
            .chain(PLANS.iter().map(|p| p.name().to_owned()))
            .collect();
        let regions: Vec<String> = std::iter::once("Everywhere".to_owned())
            .chain(REGIONS.iter().map(|r| r.name().to_owned()))
            .collect();
        let today = day_of(self.world.now);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let mut preset = match self.view.range {
                    Range::Preset(id) => id.to_owned(),
                    Range::Days { .. } => String::new(),
                };
                let mut span = match self.view.range {
                    Range::Days { from, to } => Some((sokkel_egui::Day(from as i32), sokkel_egui::Day(to as i32))),
                    Range::Preset(_) => None,
                };
                let presets: Vec<(&str, &str)> = PRESETS.iter().map(|p| (p.id, p.label)).collect();
                if TimeRangePicker::new(&mut preset, &presets)
                    .custom(&mut span, sokkel_egui::Day(today as i32))
                    .show(ui)
                    .changed()
                {
                    self.view.range = match span {
                        Some((from, to)) if preset.is_empty() => Range::Days {
                            from: i64::from(from.0),
                            to: i64::from(to.0),
                        },
                        _ => Range::preset(&preset),
                    };
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let mut plan = self
                        .view
                        .plan
                        .map_or(0, |p| PLANS.iter().position(|one| *one == p).unwrap_or(0) + 1);
                    if variable_select(ui, "Plan", &mut plan, &plans).changed() {
                        self.view.plan = plan.checked_sub(1).map(|i| PLANS[i]);
                    }
                    let mut region = self
                        .view
                        .region
                        .map_or(0, |r| REGIONS.iter().position(|one| *one == r).unwrap_or(0) + 1);
                    if variable_select(ui, "Region", &mut region, &regions).changed() {
                        self.view.region = region.checked_sub(1).map(|i| REGIONS[i]);
                    }
                    variable_toggle(ui, "Compare", &mut self.view.compare);
                });
            });
        });
    }

    /// A dashboard the person edits: its panels as they keep them, and while editing each dragged
    /// to its place, resized or removed, and the widgets not on it offered in a palette.
    pub fn board(&mut self, ui: &mut Ui, board: BoardId) {
        let editing = self.edit.editing();
        self.layout_of(board);
        let mut kept = self.layouts.remove(&board).unwrap_or_default();
        let layout = self.edit.layout(&mut kept).clone();
        if layout.is_empty() && !editing {
            let edit = Empty::new("Nothing on this dashboard")
                .icon(Icon::LayoutDashboard)
                .description("Edit it and drag in the figures and panels to show.")
                .show(ui, |ui| {
                    ui.add(Button::new("Edit").icon(Icon::Pencil).size(ButtonSize::Sm))
                        .clicked()
                });
            if edit {
                self.edit.edit(&kept);
            }
            self.layouts.insert(board, kept);
            return;
        }
        let placed: Vec<WidgetId> = layout.iter().map(|p| p.id).collect();
        let palette: Vec<DashboardWidget> = WIDGETS
            .iter()
            .filter(|id| !placed.contains(id))
            .map(|id| {
                DashboardWidget::new(id.key(), id.label())
                    .icon(id.icon())
                    .span(widget_span(*id))
            })
            .collect();
        let mut drawn = layout.clone();
        let label = |p: &Placed| p.id.label().to_owned();
        let arranged = DashboardArrange::new(("board", board as u8), &mut drawn, &label)
            .arranging(editing)
            .palette(&palette)
            .show(ui, |ui, placed| self.widget(ui, placed.id));
        if drawn != layout {
            *self.edit.layout(&mut kept) = drawn;
        }
        if let Some((key, at)) = arranged.added
            && let Some(id) = WIDGETS.iter().find(|w| w.key() == key)
        {
            let list = self.edit.layout(&mut kept);
            let at = at.min(list.len());
            list.insert(
                at,
                Placed {
                    id: *id,
                    span: widget_span(*id),
                },
            );
        }
        self.layouts.insert(board, kept);
    }
}
