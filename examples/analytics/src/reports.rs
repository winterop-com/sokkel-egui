//! The reports people saved, as the web's `Reports.tsx`: a report composed from panels with its
//! preview drawn live, and its figures as a CSV download.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AsideWidth, Button, ButtonSize, ButtonVariant, Cell, Column, ConfirmAnswer, ConfirmDialog, DashboardArrange,
    DataTable, DropdownMenu, Empty, Field, Icon, IconButton, Input, MenuItem, PageHeader, PanelSpan, Section, Split,
    StackLayout, TableVariant, Toast, ToastKind,
};

use crate::app::AnalyticsApp;
use crate::csv::{ReportContext, file_name, report_csv};
use crate::days::{DAY, counted, day_of, short_day};
use crate::metrics::Figures;
use crate::model::{PANELS, PanelId, Person, Placed, Report, WidgetId};
use crate::view::range_name;

/// The demo's reports, as the web's `reports` seeds them, saved some days before `now`.
pub fn seed_reports(now: i64) -> Vec<Report> {
    use PanelId::*;
    vec![
        Report {
            id: "monthly-board".into(),
            name: "Monthly board report".into(),
            panels: vec![Kpis, Revenue, Movements, Cohorts, Countries],
            by: Person::Margaret,
            updated: now - 3 * DAY,
        },
        Report {
            id: "weekly-growth".into(),
            name: "Weekly growth".into(),
            panels: vec![Kpis, Funnel, Regions, Customers],
            by: Person::Grace,
            updated: now - 9 * DAY,
        },
    ]
}

/// A report's panels as the widgets the preview draws, each as wide as the web's `PANEL_PARTS`.
fn placed(panels: &[PanelId]) -> Vec<Placed> {
    let mut out = Vec::new();
    for panel in panels {
        let at = |id: WidgetId, span: PanelSpan| Placed { id, span };
        match panel {
            PanelId::Kpis => out.extend(
                [
                    WidgetId::Revenue,
                    WidgetId::Mrr,
                    WidgetId::Active,
                    WidgetId::Signups,
                    WidgetId::Conversion,
                    WidgetId::Churn,
                ]
                .map(|id| at(id, PanelSpan::Sixth)),
            ),
            PanelId::Revenue => out.push(at(WidgetId::RevenueChart, PanelSpan::Full)),
            PanelId::Funnel => out.push(at(WidgetId::Funnel, PanelSpan::Third)),
            PanelId::Countries => out.push(at(WidgetId::Countries, PanelSpan::TwoThirds)),
            PanelId::Customers => out.push(at(WidgetId::Customers, PanelSpan::Half)),
            PanelId::Movements => out.push(at(WidgetId::Movements, PanelSpan::Half)),
            PanelId::Regions => out.push(at(WidgetId::Regions, PanelSpan::Half)),
            PanelId::Cohorts => out.push(at(WidgetId::Cohorts, PanelSpan::Full)),
        }
    }
    out
}

/// A report being composed: what it was opened from, and its name and panels now.
#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub of: Option<String>,
    pub name: String,
    pub panels: Vec<PanelId>,
}

/// The reports page: the saved reports, the one being composed, and a delete asked.
pub struct ReportsPage {
    pub saved: Vec<Report>,
    pub draft: Option<Draft>,
    pub deleting: bool,
    made: u32,
}

impl ReportsPage {
    pub fn new(now: i64) -> Self {
        Self {
            saved: seed_reports(now),
            draft: None,
            deleting: false,
            made: 0,
        }
    }
}

enum Asked {
    Open(String),
    Save,
    Export,
    Delete,
}

impl AnalyticsApp {
    pub fn reports_page(&mut self, ui: &mut Ui) {
        let id = self.view.path.get(1).cloned();
        let chosen: Option<Report> = match id.as_deref() {
            Some("new") => None,
            Some(id) => self.reports.saved.iter().find(|r| r.id == id).cloned(),
            None => self.reports.saved.first().cloned(),
        };
        let missing = id.as_deref().is_some_and(|i| i != "new") && chosen.is_none();
        let of = chosen.as_ref().map(|r| r.id.clone());
        if self.reports.draft.as_ref().is_none_or(|d| d.of != of) {
            self.reports.draft = Some(match &chosen {
                Some(r) => Draft {
                    of: of.clone(),
                    name: r.name.clone(),
                    panels: r.panels.clone(),
                },
                None => Draft {
                    of: None,
                    name: String::new(),
                    panels: vec![PanelId::Kpis, PanelId::Revenue],
                },
            });
        }
        let mut asked = None;
        PageHeader::new("Reports")
            .description("Panels put together, read over any range and segment, and kept as a file.")
            .show(ui, |ui| {
                if ui.add(Button::new("New").icon(Icon::Plus)).clicked() {
                    asked = Some(Asked::Open("new".into()));
                }
            });
        self.view_bar(ui);
        let range = range_name(&self.view.range).to_lowercase();
        let saved = self.reports.saved.clone();
        let mut draft = self.reports.draft.take().expect("a draft was just made");
        let list_state = self.tables.entry("reports-saved".into()).or_default();
        list_state.opened = None;
        let was_opened = chosen.as_ref().and_then(|c| saved.iter().position(|r| r.id == c.id));
        let mut panels_state = std::mem::take(self.tables.entry("report-panels".into()).or_default());
        let list_state = self.tables.get_mut("reports-saved").expect("made above");
        Split::new(AsideWidth::Sm).start(true).show(
            ui,
            |ui| {
                if missing {
                    Empty::new("No such report")
                        .description(format!(
                            "No saved report has the id {}. It may have been deleted.",
                            id.clone().unwrap_or_default()
                        ))
                        .show(ui, |_| {});
                    return;
                }
                let changed = chosen
                    .as_ref()
                    .is_none_or(|c| c.name != draft.name.trim() || c.panels != draft.panels);
                let why = if draft.name.trim().is_empty() {
                    Some("A report needs a name.")
                } else if draft.panels.is_empty() {
                    Some("A report needs a panel.")
                } else if !changed {
                    Some("Nothing has changed since it was saved.")
                } else {
                    None
                };
                let (title, description) = match &chosen {
                    Some(c) => (
                        c.name.clone(),
                        format!("Saved by {}, {}.", c.by.name(), short_day(day_of(c.updated))),
                    ),
                    None => ("New report".to_owned(), "Not saved yet.".to_owned()),
                };
                let no_panels = draft.panels.is_empty();
                Section::new(title)
                    .description(description)
                    .actions(|ui| {
                        let mut save = Button::new("Save").icon(Icon::Save).size(ButtonSize::Sm);
                        if let Some(why) = why {
                            save = save.reason(why);
                        }
                        if ui.add(save).clicked() {
                            asked = Some(Asked::Save);
                        }
                        let mut export = Button::new("Export CSV")
                            .icon(Icon::Download)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm);
                        if no_panels {
                            export = export.reason("Add a panel first.");
                        }
                        if ui.add(export).clicked() {
                            asked = Some(Asked::Export);
                        }
                        if chosen.is_some() && ui.add(IconButton::new(Icon::Trash2, "Delete report")).clicked() {
                            asked = Some(Asked::Delete);
                        }
                    })
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 16.0;
                        Field::new("Name").show(ui, |ui| {
                            ui.add(Input::new(&mut draft.name).placeholder("Monthly board report"))
                        });
                        Field::new("Panels")
                            .description(format!("Read over {range}, as the bar above says."))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 8.0;
                                let columns = vec![
                                    Column::new("#", 28.0),
                                    Column::new("Panel", 144.0).fill(),
                                    Column::new("", 28.0).numeric(),
                                ];
                                let rows: Vec<Vec<Cell>> = draft
                                    .panels
                                    .iter()
                                    .enumerate()
                                    .map(|(i, panel)| {
                                        vec![
                                            Cell::Muted((i + 1).to_string()),
                                            Cell::Text(panel.name().into()),
                                            Cell::Button {
                                                icon: Icon::X,
                                                label: format!("Take {} out", panel.name()),
                                            },
                                        ]
                                    })
                                    .collect();
                                let out = DataTable::new("report-panels", &columns, &rows, &mut panels_state)
                                    .variant(TableVariant::List)
                                    .reorder(true)
                                    .bar(false)
                                    .footer(false)
                                    .keys(draft.panels.iter().map(|p| format!("{p:?}")))
                                    .words(sokkel_egui::TableWords {
                                        empty: "No panels yet. Add the first below.".into(),
                                        ..Default::default()
                                    })
                                    .show(ui);
                                if let Some((from, to)) = out.reordered {
                                    let panel = draft.panels.remove(from);
                                    draft.panels.insert(to.min(draft.panels.len()), panel);
                                }
                                if let Some((row, _)) = out.pressed {
                                    draft.panels.remove(row);
                                }
                                let left: Vec<PanelId> =
                                    PANELS.iter().filter(|p| !draft.panels.contains(p)).copied().collect();
                                let add = ui.add(
                                    Button::new("Add panel")
                                        .icon(Icon::Plus)
                                        .variant(ButtonVariant::Outline)
                                        .size(ButtonSize::Sm)
                                        .enabled(!left.is_empty()),
                                );
                                let items: Vec<MenuItem> = left.iter().map(|p| MenuItem::action(p.name())).collect();
                                if let Some(picked) = DropdownMenu::new(&items).show(&add) {
                                    draft.panels.push(left[picked]);
                                }
                            });
                    });
            },
            |ui| {
                Section::new("Saved")
                    .meta(saved.len().to_string())
                    .flush(true)
                    .show(ui, |ui| {
                        let columns = vec![
                            Column::new("Report", 116.0).fill(),
                            Column::new("Panels", 76.0).numeric(),
                        ];
                        let rows: Vec<Vec<Cell>> = saved
                            .iter()
                            .map(|r| {
                                vec![
                                    Cell::Text(r.name.clone()),
                                    Cell::Muted(counted(r.panels.len(), "panel")),
                                ]
                            })
                            .collect();
                        DataTable::new("reports-saved", &columns, &rows, list_state)
                            .opened(was_opened)
                            .variant(TableVariant::List)
                            .framed(false)
                            .bar(false)
                            .footer(false)
                            .stack_layout(StackLayout::Compact)
                            .words(sokkel_egui::TableWords {
                                empty: "No report saved yet.".into(),
                                ..Default::default()
                            })
                            .show(ui);
                    });
            },
        );
        *self.tables.entry("report-panels".into()).or_default() = panels_state;
        // the preview, across the page under the composer, the panels as the dashboards draw them
        if !missing {
            if draft.panels.is_empty() {
                Empty::new("Nothing to preview")
                    .description("Add a panel and its figures show here as the report will read.")
                    .show(ui, |_| {});
            } else {
                let mut layout = placed(&draft.panels);
                let label = |p: &Placed| p.id.label().to_owned();
                DashboardArrange::new(("report", of.clone()), &mut layout, &label)
                    .arranging(false)
                    .show(ui, |ui, placed| self.widget(ui, placed.id));
            }
        }
        let opened = self.tables.get_mut("reports-saved").and_then(|s| s.opened.take());
        if let Some(r) = opened.filter(|i| Some(*i) != was_opened).and_then(|i| saved.get(i)) {
            asked = Some(Asked::Open(r.id.clone()));
        }
        match asked {
            Some(Asked::Open(id)) => {
                // the draft stays while the question is asked; a page taken starts its own
                self.reports.draft = Some(draft);
                self.go(&format!("reports/{id}"));
            }
            Some(Asked::Save) => {
                let name = draft.name.trim().to_owned();
                let id = match &chosen {
                    Some(c) => c.id.clone(),
                    None => {
                        self.reports.made += 1;
                        format!("report-{}", self.reports.made)
                    }
                };
                let report = Report {
                    id: id.clone(),
                    name: name.clone(),
                    panels: draft.panels.clone(),
                    by: Person::Ada,
                    updated: self.now(),
                };
                match self.reports.saved.iter_mut().find(|r| r.id == id) {
                    Some(known) => *known = report,
                    None => self.reports.saved.push(report),
                }
                Toast::success(format!("{name} saved"))
                    .description(counted(draft.panels.len(), "panel"))
                    .show(ui.ctx());
                draft.of = Some(id.clone());
                self.reports.draft = Some(draft);
                self.go(&format!("reports/{id}"));
            }
            Some(Asked::Export) => {
                self.export_report(ui.ctx(), &draft);
                self.reports.draft = Some(draft);
            }
            Some(Asked::Delete) => {
                self.reports.deleting = true;
                self.reports.draft = Some(draft);
            }
            None => self.reports.draft = Some(draft),
        }
        self.report_delete(ui.ctx(), chosen.as_ref());
    }

    /// The report's figures as a CSV file in the Downloads folder, as the web hands one over.
    fn export_report(&self, ctx: &egui::Context, draft: &Draft) {
        let title = if draft.name.trim().is_empty() {
            "Report".to_owned()
        } else {
            draft.name.trim().to_owned()
        };
        let (span, previous) = self.spans();
        let figures = Figures::new(&self.world);
        let context = ReportContext {
            figures: &figures,
            segment: self.segment(),
            span: &span,
            previous: &previous,
            compare: self.view.compare,
            range: self.view.range,
            currency: self.prefs.currency,
        };
        let text = report_csv(&title, &draft.panels, &context);
        let file = file_name(&title, self.world.now);
        match sokkel_egui::download(&file, &text) {
            Ok(path) => {
                let saved = path
                    .file_name()
                    .map_or(file.clone(), |f| f.to_string_lossy().into_owned());
                Toast::new("CSV downloaded")
                    .description(format!("{saved}, {}", counted(draft.panels.len(), "panel")))
                    .show(ctx);
            }
            Err(error) => {
                Toast::new(format!("{file} could not be saved"))
                    .kind(ToastKind::Error)
                    .description(error.to_string())
                    .show(ctx);
            }
        }
    }

    fn report_delete(&mut self, ctx: &egui::Context, chosen: Option<&Report>) {
        if !self.reports.deleting {
            return;
        }
        let Some(chosen) = chosen else {
            self.reports.deleting = false;
            return;
        };
        let answer = ConfirmDialog::new("delete-report", format!("Delete {}?", chosen.name), "Delete")
            .detail("The report goes for everyone; the figures it reads stay as they are.")
            .destructive(true)
            .show(ctx);
        match answer {
            Some(ConfirmAnswer::Confirm) => {
                self.reports.deleting = false;
                self.reports.saved.retain(|r| r.id != chosen.id);
                self.reports.draft = None;
                Toast::new(format!("{} deleted", chosen.name)).show(ctx);
                self.go("reports");
            }
            Some(_) => self.reports.deleting = false,
            None => {}
        }
    }
}
