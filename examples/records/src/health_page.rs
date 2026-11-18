//! The metadata's health, as the web's `Health.tsx`: what the metadata holds that a record cannot
//! carry cleanly, by how bad it is and the kind of object, and how far the forms are translated.

use eframe::egui::Ui;
use sokkel_egui::{
    ApiLink, BarList, BarListItem, Button, ButtonSize, ButtonVariant, Cell, Column, DataTable, Icon, PageHeader,
    SearchField, Section, Stat, StatTiles, TableVariant, TextRole, Toast, Unfoldable, tokens,
};

use crate::app::{Foot, RecordsApp};
use crate::health::{Finding, Severity, check_metadata, coverage};
use crate::ui::{count, grouped};

/// What the health page holds between frames: the findings of the last check and the search.
#[derive(Default)]
pub struct HealthView {
    pub findings: Option<Vec<Finding>>,
    pub query: String,
}

impl RecordsApp {
    pub fn health_page(&mut self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        if self.health.findings.is_none() {
            self.health.findings = Some(check_metadata(&self.backend.households));
        }
        let findings = self.health.findings.clone().unwrap_or_default();
        let asked = self.health.query.trim().to_lowercase();
        let shown: Vec<&Finding> = findings
            .iter()
            .filter(|f| asked.is_empty() || format!("{} {}", f.object, f.object_id).to_lowercase().contains(&asked))
            .collect();
        self.foot = Foot {
            note: format!("Showing {} of {}", shown.len(), count(findings.len(), "finding")),
            items: Vec::new(),
        };

        let mut again = false;
        PageHeader::new("Metadata health")
            .description("What the server's metadata holds that a record cannot carry cleanly, and how far the forms are translated. Each finding opens the object it is about.")
            .show(ui, |ui| {
                again = ui
                    .add(
                        Button::new("Check again")
                            .icon(Icon::RefreshCw)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm),
                    )
                    .clicked();
                ui.add(ApiLink::new("/health"));
            });
        if again {
            let found = check_metadata(&self.backend.households);
            Toast::new("Checked again")
                .description(count(found.len(), "finding"))
                .show(ui.ctx());
            self.health.findings = Some(found);
        }

        let tiles: Vec<Stat> = Severity::ORDER
            .iter()
            .map(|severity| {
                let of = findings.iter().filter(|f| f.severity == *severity).count();
                Stat::new(severity.label(), of)
                    .dot(t.color(severity.dot()))
                    .note(match severity {
                        Severity::Error => "refused by the register",
                        Severity::Warning => "filed, but degraded",
                        Severity::Note => "worth a look",
                    })
                    .emphasis(*severity == Severity::Error && of > 0)
            })
            .collect();
        StatTiles::new("health-tiles", 3, 160.0, 12.0).show(ui, tiles);

        let languages: Vec<BarListItem> = coverage()
            .iter()
            .map(|language| BarListItem {
                label: format!("{} ({})", language.name, language.language),
                value: language.translated as f64 / language.total as f64,
                note: Some(format!(
                    "{} of {} strings",
                    grouped(language.translated as f64),
                    grouped(language.total as f64)
                )),
                ..Default::default()
            })
            .collect();
        Section::new("Translations")
            .description("Each language's share of the strings the forms say: titles, questions and their help.")
            .show(ui, |ui| BarList::new(&languages).show(ui));

        // the web's widths: a list's cells have no padding of their own
        let columns = vec![
            Column::new("Object", 160.0).fill(),
            Column::new("Field", 112.0),
            Column::new("Problem", 192.0).fill_share(1.4),
            Column::new("What it costs", 192.0).fill_share(1.4),
        ];
        let mut go = None;
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 12.0;
            SearchField::new("health-search", &mut self.health.query)
                .placeholder("Object name or id")
                .keys("/")
                .width(448.0)
                .show(ui);
            if shown.is_empty() {
                let said = if asked.is_empty() {
                    "Every name and code is one a record can carry as it stands.".to_owned()
                } else {
                    format!("No object matches. {} in all.", count(findings.len(), "finding"))
                };
                ui.label(TextRole::Help.rich(&t, said).color(t.color("muted-foreground")));
            }
            for severity in Severity::ORDER {
                let these: Vec<&Finding> = shown.iter().copied().filter(|f| f.severity == severity).collect();
                if these.is_empty() {
                    continue;
                }
                let rows: Vec<Vec<Cell>> = these
                    .iter()
                    .map(|f| {
                        vec![
                            Cell::NamedCode {
                                name: f.object.clone(),
                                code: f.object_id.clone(),
                                strong: true,
                            },
                            Cell::Mono(f.field.into()),
                            Cell::Text(f.problem.clone()),
                            Cell::Text(f.cost.into()),
                        ]
                    })
                    .collect();
                let key = format!("health-{}", severity.label());
                let aside = count(these.len(), "finding");
                let tables = &mut self.tables;
                Unfoldable::new(&key, severity.label())
                    .dot(t.color(severity.dot()))
                    .default_open(matches!(severity, Severity::Error | Severity::Warning))
                    .forced(!asked.is_empty())
                    .aside(move |ui| {
                        let t = tokens(ui.ctx());
                        ui.label(TextRole::Help.rich(&t, aside).color(t.color("muted-foreground")));
                    })
                    .show(ui, |ui| {
                        let state = tables.entry(key.clone()).or_default();
                        DataTable::new(&key, &columns, &rows, state)
                            .variant(TableVariant::List)
                            .groups(these.iter().map(|f| f.kind))
                            .auto_rows()
                            .chevron(true)
                            .height(f32::INFINITY)
                            .footer(false)
                            .show(ui);
                        if let Some(index) = state_opened(tables, &key) {
                            go = these[index].href.clone();
                        }
                    });
            }
        });
        if let Some(to) = go {
            self.go(&to);
        }
    }
}

/// The row a table's press opened, taken.
fn state_opened(tables: &mut std::collections::HashMap<String, sokkel_egui::TableState>, key: &str) -> Option<usize> {
    tables.get_mut(key).and_then(|s| s.opened.take())
}
