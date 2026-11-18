//! A panel's own actions, as the web's `Dashboard.tsx` menu and `InspectDialog`: open it in its
//! window, inspect its data as a table with a CSV, duplicate it, and delete it.

use eframe::egui;
use sokkel_egui::{
    Button, ButtonVariant, Cell, Column, ConfirmAnswer, ConfirmDialog, DataTable, Dialog, Icon, MenuItem, PanelAsked,
    PanelWindow, TableState, Toast, ToastKind, format_value,
};

use crate::MetricsApp;
use crate::model::{Panel, PanelKind};
use crate::panel_data::{log_filter, query_of};
use crate::query::{Range, bounds, clock_of, coarse, read_series, span_of, time_label};
use crate::source::{Variables, logs_until, metric_of};

/// What a panel's menu item does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelAct {
    View,
    Inspect,
    Edit,
    Duplicate,
    Delete,
}

/// A panel's menu, as the web's: View (but for a stat), Inspect data, Edit (but while the board is
/// arranged, as the editor is a page of its own), Duplicate, and Delete after a separator; with
/// what each item does, `None` for the separator.
pub fn panel_menu(panel: &Panel, editing: bool) -> (Vec<MenuItem>, Vec<Option<PanelAct>>) {
    let mut items = Vec::new();
    let mut acts = Vec::new();
    if panel.kind != PanelKind::Stat {
        items.push(MenuItem::action("View").icon(Icon::Maximize2));
        acts.push(Some(PanelAct::View));
    }
    items.push(MenuItem::action("Inspect data").icon(Icon::Table2));
    acts.push(Some(PanelAct::Inspect));
    if !editing {
        items.push(MenuItem::action("Edit").icon(Icon::PencilLine));
        acts.push(Some(PanelAct::Edit));
    }
    items.push(MenuItem::action("Duplicate").icon(Icon::Copy));
    acts.push(Some(PanelAct::Duplicate));
    items.push(MenuItem::Separator);
    acts.push(None);
    items.push(MenuItem::action("Delete").icon(Icon::Trash2).destructive());
    acts.push(Some(PanelAct::Delete));
    (items, acts)
}

/// A panel's data as a table, as the web's `InspectDialog`: a row per step and a column per
/// series, newest first, or the log's lines; with what it says over the table and its CSV.
pub struct Inspected {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<Cell>>,
    pub description: String,
    pub csv: String,
}

/// Its times are written `offset` seconds from UTC, the zone the reader chose.
pub fn inspect(panel: &Panel, vars: &Variables, range: &Range, at: f64, offset: f64) -> Inspected {
    let metric = metric_of(panel.metric);
    let decimals = panel.decimals.unwrap_or(metric.decimals);
    let unit = panel.unit.clone().unwrap_or_else(|| metric.unit.to_owned());
    let unit = unit.trim();
    if panel.kind == PanelKind::Logs {
        let (start, end) = bounds(range, at);
        let lines = log_filter(panel, vars).map_or_else(Vec::new, |filter| {
            let mut lines = logs_until(end, start, &filter, 200);
            lines.reverse();
            lines
        });
        let rows: Vec<Vec<Cell>> = lines
            .iter()
            .map(|line| {
                vec![
                    Cell::Mono(clock_of(line.second + offset)),
                    Cell::Mono(line.level.to_owned()),
                    Cell::Mono(line.host.to_owned()),
                    Cell::Text(line.text.clone()),
                ]
            })
            .collect();
        let csv = std::iter::once("time,level,host,line".to_owned())
            .chain(lines.iter().map(|line| {
                format!(
                    "{},{},{},{}",
                    clock_of(line.second + offset),
                    line.level,
                    line.host,
                    json_string(&line.text)
                )
            }))
            .collect::<Vec<_>>()
            .join("\n");
        return Inspected {
            // the web's 6rem, 6rem, 8rem and `minmax(16rem,1fr)`, less the 12 between columns
            columns: vec![
                Column::new("Time", 84.0),
                Column::new("Level", 84.0),
                Column::new("Host", 116.0),
                Column::new("Line", 244.0).fill(),
            ],
            description: format!("The newest {} lines in the range, newest first.", rows.len()),
            rows,
            csv,
        };
    }
    let span = span_of(range, at, 150.0);
    let span = match panel.kind {
        PanelKind::Bar => coarse(&span, 20.0),
        PanelKind::Heatmap => coarse(&span, 30.0),
        _ => span,
    };
    let series = read_series(&query_of(panel, vars), &span);
    let steps: Vec<usize> = (0..span.times.len()).rev().collect();
    let rows = steps
        .iter()
        .map(|&i| {
            std::iter::once(Cell::Mono(time_label(span.times[i] + offset, &span)))
                .chain(series.iter().map(|one| {
                    let value = one.values.get(i).copied().unwrap_or(0.0);
                    Cell::Number {
                        value,
                        text: format_value(value, Some(decimals)),
                    }
                }))
                .collect()
        })
        .collect::<Vec<Vec<Cell>>>();
    // the web's 8rem, then `minmax(7rem,1fr)` a series, less the 12 between columns
    let columns = std::iter::once(Column::new("Time", 116.0))
        .chain(series.iter().map(|one| {
            let header = if unit.is_empty() {
                one.name.clone()
            } else {
                format!("{} ({unit})", one.name)
            };
            Column::new(header, 100.0).numeric().fill()
        }))
        .collect();
    let csv = std::iter::once(
        std::iter::once("time".to_owned())
            .chain(series.iter().map(|one| one.name.clone()))
            .collect::<Vec<_>>()
            .join(","),
    )
    .chain(steps.iter().map(|&i| {
        std::iter::once(iso_time(span.times[i]))
            .chain(
                series
                    .iter()
                    .map(|one| format!("{:.*}", decimals, one.values.get(i).copied().unwrap_or(0.0))),
            )
            .collect::<Vec<_>>()
            .join(",")
    }))
    .collect::<Vec<_>>()
    .join("\n");
    let by = match panel.group_by {
        Some(label) => format!(" by {}", label.key()),
        None => String::new(),
    };
    Inspected {
        columns,
        description: format!(
            "{}, {}{by}: {} steps, newest first.",
            metric.label,
            format!("{:?}", panel.aggregation).to_lowercase(),
            rows.len()
        ),
        rows,
        csv,
    }
}

/// A string written as JSON writes it, quoted with its quotes and backslashes escaped.
fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A second as an ISO instant in UTC, as `Date.toISOString` writes it.
fn iso_time(second: f64) -> String {
    let days = (second / 86_400.0).floor() as i64;
    let at = second.rem_euclid(86_400.0) as u64;
    // days since 1970 to a civil date, Howard Hinnant's way
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.000Z",
        at / 3600,
        at / 60 % 60,
        at % 60
    )
}

/// A file name from a title, as the web's download names it.
fn file_name(title: &str) -> String {
    let mut name = String::new();
    for c in title.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            name.push(c);
        } else if !name.ends_with('-') {
            name.push('-');
        }
    }
    format!("{name}.csv")
}

/// What a panel's dialogs hold between frames.
#[derive(Default)]
pub struct PanelDialogs {
    /// The panel open in its window, by its id.
    pub viewing: Option<String>,
    /// The panel whose data is inspected, by its id.
    pub inspecting: Option<String>,
    /// The panel asked to be deleted, by its id.
    pub deleting: Option<String>,
    /// The panel to bring into view on the board, by its id, once the editor saved it.
    pub focus: Option<String>,
    pub(crate) inspect_table: TableState,
}

impl MetricsApp {
    /// Does what a panel's tools asked for.
    pub(crate) fn panel_asked(&mut self, panel: &Panel, asked: PanelAsked) {
        let act = match asked {
            PanelAsked::Expand => Some(PanelAct::View),
            PanelAsked::Item(index) => panel_menu(panel, self.edit.editing()).1.get(index).copied().flatten(),
        };
        match act {
            Some(PanelAct::View) => self.panels.viewing = Some(panel.id.clone()),
            Some(PanelAct::Inspect) => {
                self.panels.inspecting = Some(panel.id.clone());
                self.panels.inspect_table = TableState::default();
            }
            Some(PanelAct::Edit) => self.open_editor(Some(panel)),
            Some(PanelAct::Duplicate) => self.duplicate_panel(panel),
            // while editing, Cancel brings a deleted panel back, so it goes without asking
            Some(PanelAct::Delete) if self.edit.editing() => self.remove_panel(&panel.id),
            Some(PanelAct::Delete) => self.panels.deleting = Some(panel.id.clone()),
            None => {}
        }
    }

    /// The panels shown: the draft while editing, else the board's own.
    fn shown_panels(&mut self) -> Option<&mut Vec<Panel>> {
        let index = self.dashboards.iter().position(|b| b.id == self.place)?;
        Some(self.edit.layout(&mut self.dashboards[index].panels))
    }

    /// A copy of a panel after it, as the web's `duplicate`.
    fn duplicate_panel(&mut self, panel: &Panel) {
        let mut made = panel.clone();
        made.id = self.panel_id();
        made.title = format!("{} copy", panel.title);
        let editing = self.edit.editing();
        let Some(panels) = self.shown_panels() else {
            return;
        };
        let at = panels
            .iter()
            .position(|p| p.id == panel.id)
            .map_or(panels.len(), |i| i + 1);
        panels.insert(at, made);
        if !editing {
            self.said.push(Toast::success(format!("{} duplicated", panel.title)));
        }
    }

    /// A panel taken off the board, as the web's `remove`.
    fn remove_panel(&mut self, id: &str) {
        let editing = self.edit.editing();
        let Some(panels) = self.shown_panels() else {
            return;
        };
        let Some(at) = panels.iter().position(|p| p.id == id) else {
            return;
        };
        let gone = panels.remove(at);
        if !editing {
            self.said.push(Toast::success(format!("{} deleted", gone.title)));
        }
    }

    /// A panel by its id among those shown.
    fn shown_panel(&mut self, id: Option<&String>) -> Option<Panel> {
        let id = id?;
        self.shown_panels()?.iter().find(|p| &p.id == id).cloned()
    }

    /// The panel's window, its data inspected, and the question before deleting it.
    pub(crate) fn panel_dialogs(&mut self, ctx: &egui::Context) {
        let (variables, range, at) = (self.variables.clone(), self.range.clone(), self.view_at);
        let view = self.view();
        let viewing = self.panels.viewing.clone();
        match self.shown_panel(viewing.as_ref()) {
            Some(panel) => {
                let table = self.tables.entry(panel.id.clone()).or_default();
                let close = PanelWindow::new(("panel-window", &panel.id), &panel.title).show(ctx, |ui| {
                    crate::app::draw_panel(ui, &panel, &view, table);
                });
                if close {
                    self.panels.viewing = None;
                }
            }
            None => self.panels.viewing = None,
        }
        let inspecting = self.panels.inspecting.clone();
        match self.shown_panel(inspecting.as_ref()) {
            Some(panel) => {
                let inspected = inspect(&panel, &variables, &range, at, view.offset);
                let mut answer = None;
                let state = &mut self.panels.inspect_table;
                let close = Dialog::new("inspect-panel", format!("Inspect {}", panel.title))
                    .description(&inspected.description)
                    .width(896.0)
                    .show(
                        ctx,
                        |ui| {
                            DataTable::new(("inspect", &panel.id), &inspected.columns, &inspected.rows, state)
                                .fits(true)
                                .height(384.0)
                                .footer(false)
                                .show(ui);
                        },
                        |ui| {
                            if ui.add(Button::new("Download CSV")).clicked() {
                                answer = Some(true);
                            }
                            if ui.add(Button::new("Close").variant(ButtonVariant::Ghost)).clicked() {
                                answer = Some(false);
                            }
                        },
                    );
                if answer == Some(true) {
                    let name = file_name(&panel.title);
                    match sokkel_egui::download(&name, &inspected.csv) {
                        Ok(path) => {
                            let saved = path
                                .file_name()
                                .map_or(name.clone(), |f| f.to_string_lossy().into_owned());
                            let folder = path.parent().map(|p| p.display().to_string()).unwrap_or_default();
                            self.said.push(Toast::success(format!("{saved} saved in {folder}")));
                        }
                        Err(error) => self
                            .said
                            .push(Toast::new(format!("{name} could not be saved: {error}")).kind(ToastKind::Error)),
                    }
                }
                if answer.is_some() || close {
                    self.panels.inspecting = None;
                }
            }
            None => self.panels.inspecting = None,
        }
        let deleting = self.panels.deleting.clone();
        if let Some(panel) = self.shown_panel(deleting.as_ref()) {
            let dialog = ConfirmDialog::new("delete-panel", format!("Delete {}?", panel.title), "Delete")
                .detail("The panel leaves this dashboard for good. What it read stays in the source.")
                .destructive(true);
            match dialog.show(ctx) {
                Some(ConfirmAnswer::Confirm) => {
                    self.panels.deleting = None;
                    self.remove_panel(&panel.id);
                }
                Some(_) => self.panels.deleting = None,
                None => {}
            }
        } else {
            self.panels.deleting = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{file_name, iso_time, json_string};

    #[test]
    fn written_as_the_web_writes_them() {
        assert_eq!(iso_time(1_794_992_405.0), "2026-11-18T09:00:05.000Z");
        assert_eq!(file_name("Requests by service"), "requests-by-service.csv");
        assert_eq!(file_name("p95 latency (ms)"), "p95-latency-ms-.csv");
        assert_eq!(json_string("a \"b\""), "\"a \\\"b\\\"\"");
    }
}
