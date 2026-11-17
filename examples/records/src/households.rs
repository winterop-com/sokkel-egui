//! The households the survey follows, as the web's `pages/Households.tsx`, and one household, as
//! `pages/Household.tsx`: who it is, what it told the survey, its place in the program, its visits.

use eframe::egui::Ui;
use sokkel_egui::{
    ApiLink, Banner, BannerTone, Button, ButtonSize, ButtonVariant, Cell, ChipTone, Column, ConfirmAnswer,
    ConfirmDialog, DataTable, DescriptionList, FilterChip, Icon, PageHeader, PageState, Section, TextLink, Toast,
    ToastKind, Unfoldable, tokens,
};

use crate::app::{Foot, RecordsApp};
use crate::backend::{Household, HouseholdStatus};
use crate::forms::{AnswerValue, PROGRAM_WATER, form_of, questions_of};
use crate::geo::place_name;
use crate::ui::{answer_text, count, count_as, on_day, state_chip, when};

/// What the households' list narrows to.
#[derive(Clone, Debug, Default)]
pub struct HouseholdsView {
    pub query: String,
    pub regions: Vec<String>,
    pub sources: Vec<String>,
    pub states: Vec<String>,
    /// A removal asked, by the household's id.
    pub removing: Option<String>,
}

fn status_key(status: HouseholdStatus) -> &'static str {
    match status {
        HouseholdStatus::Active => "active",
        HouseholdStatus::Completed => "completed",
    }
}

fn status_tone(status: HouseholdStatus) -> ChipTone {
    match status {
        HouseholdStatus::Active => ChipTone::Good,
        HouseholdStatus::Completed => ChipTone::Neutral,
    }
}

fn enroll_question(id: &str) -> Option<&'static crate::forms::Question> {
    form_of("FRM-ENROLL").and_then(|f| questions_of(f).find(|q| q.id == id))
}

fn source_text(source: &str) -> String {
    answer_text(enroll_question("source"), Some(&AnswerValue::Text(source.into())))
}

impl RecordsApp {
    fn visits_of(&self, home: &Household) -> usize {
        self.backend
            .events_of(&home.id)
            .iter()
            .filter(|r| r.form_id != "FRM-ENROLL")
            .count()
    }

    pub fn households_page(&mut self, ui: &mut Ui) {
        let homes = self.backend.households.clone();
        let view = self.households.clone();
        let asked = view.query.trim().to_lowercase();
        let mut shown: Vec<&Household> = homes
            .iter()
            .filter(|h| {
                h.identifier.to_lowercase().contains(&asked)
                    && (view.regions.is_empty() || view.regions.contains(&h.region))
                    && (view.sources.is_empty() || view.sources.contains(&h.source))
                    && (view.states.is_empty() || view.states.iter().any(|s| s == status_key(h.status)))
            })
            .collect();
        shown.sort_by_key(|h| std::cmp::Reverse(h.registered_at));
        let mut items = Vec::new();
        if !asked.is_empty() {
            items.push(("identifier".to_owned(), view.query.trim().to_owned()));
        }
        self.foot = Foot {
            note: format!("Showing {} of {}", shown.len(), count(homes.len(), "household")),
            items,
        };
        let mut register = false;
        PageHeader::new("Households")
            .description("The households the water survey follows. One row is one registered household, named by the identifier the server gave it.")
            .show(ui, |ui| {
                register = ui
                    .add(Button::new("Register").icon(Icon::UserPlus).size(ButtonSize::Sm))
                    .clicked();
                ui.add(ApiLink::new(if asked.is_empty() {
                    "/households".to_owned()
                } else {
                    format!("/households?identifier={}", view.query.trim())
                }));
            });
        // the web's widths, less the 12 between columns
        let columns = vec![
            Column::new("Identifier", 148.0),
            Column::new("Region", 116.0),
            Column::new("Site", 116.0).fill(),
            Column::new("People", 68.0).numeric(),
            Column::new("Water source", 148.0),
            Column::new("Visits", 68.0).numeric(),
            Column::new("Registered", 116.0),
            Column::new("State", 100.0),
        ];
        let rows: Vec<Vec<Cell>> = shown
            .iter()
            .map(|h| {
                let visits = self.visits_of(h);
                vec![
                    Cell::Mono(h.identifier.clone()),
                    Cell::Text(place_name(&h.region)),
                    Cell::Text(place_name(&h.site)),
                    Cell::Number {
                        value: h.size as f64,
                        text: h.size.to_string(),
                    },
                    Cell::Text(source_text(&h.source)),
                    Cell::Number {
                        value: visits as f64,
                        text: visits.to_string(),
                    },
                    Cell::Number {
                        value: h.registered_at as f64,
                        text: on_day(&crate::backend::iso_utc_day(h.registered_at)),
                    },
                    Cell::Chip(status_key(h.status).into(), status_tone(h.status)),
                ]
            })
            .collect();
        let mut regions: Vec<String> = homes.iter().map(|h| h.region.clone()).collect();
        regions.sort();
        regions.dedup();
        let region_options: Vec<(String, String)> = regions.iter().map(|r| (r.clone(), place_name(r))).collect();
        let source_options: Vec<(String, String)> = enroll_question("source")
            .map(|q| {
                q.options
                    .iter()
                    .map(|o| (o.value.to_owned(), o.label.to_owned()))
                    .collect()
            })
            .unwrap_or_default();
        let state_options: Vec<(String, String)> = ["active", "completed"]
            .iter()
            .map(|s| ((*s).to_owned(), (*s).to_owned()))
            .collect();
        let filtered =
            !asked.is_empty() || !(view.regions.is_empty() && view.sources.is_empty() && view.states.is_empty());
        let mut view = std::mem::take(&mut self.households);
        let ids: Vec<String> = shown.iter().map(|h| h.id.clone()).collect();
        let state = self.tables.entry("households".into()).or_default();
        let (mut chosen_regions, mut chosen_sources, mut chosen_states) =
            (view.regions.clone(), view.sources.clone(), view.states.clone());
        let mut table = DataTable::new("households", &columns, &rows, state)
            .search(&mut view.query)
            .search_placeholder("Identifier")
            .toolbar(|ui| {
                FilterChip::new("households-region", "Region", &region_options, &mut chosen_regions).show(ui);
                FilterChip::new(
                    "households-source",
                    "Water source",
                    &source_options,
                    &mut chosen_sources,
                )
                .show(ui);
                FilterChip::new("households-state", "State", &state_options, &mut chosen_states).show(ui);
            })
            .chevron(true)
            .stretch(true)
            .footer(false);
        if filtered {
            table = table.words(sokkel_egui::TableWords {
                empty: "No household matches the search and the filters.".into(),
                ..Default::default()
            });
        } else if homes.is_empty() {
            table = table.words(sokkel_egui::TableWords {
                empty: "No household is registered yet. Register one with the household registration form.".into(),
                ..Default::default()
            });
        }
        table.show(ui);
        view.regions = chosen_regions;
        view.sources = chosen_sources;
        view.states = chosen_states;
        self.households = view;
        let opened = self.tables.get_mut("households").and_then(|s| s.opened.take());
        if let Some(index) = opened
            && let Some(id) = ids.get(index)
        {
            self.go(&format!("people/{id}"));
        }
        if register {
            self.go("forms/FRM-ENROLL");
        }
    }

    pub fn household_page(&mut self, ui: &mut Ui, id: &str) {
        let Some(home) = self.backend.households.iter().find(|h| h.id == id).cloned() else {
            PageHeader::new("Household").show(ui, |_| {});
            PageState::missing(format!("No household is registered under {id}.")).show(ui);
            return;
        };
        let events: Vec<crate::backend::Receipt> = self.backend.events_of(&home.id).into_iter().cloned().collect();
        let registration = events.iter().find(|r| r.form_id == "FRM-ENROLL").cloned();
        let visits: Vec<&crate::backend::Receipt> = events.iter().filter(|r| r.form_id != "FRM-ENROLL").collect();
        self.foot = Foot {
            note: format!(
                "{} · registered {}",
                count(visits.len(), "visit"),
                on_day(&crate::backend::iso_utc_day(home.registered_at))
            ),
            items: Vec::new(),
        };
        let closed = home.status == HouseholdStatus::Completed;
        let twins: Vec<String> = self
            .backend
            .households
            .iter()
            .filter(|h| h.identifier == home.identifier && h.id != home.id)
            .map(|h| h.id.clone())
            .collect();
        let (mut remove, mut exit, mut visit) = (false, false, false);
        let mut go: Option<String> = None;
        PageHeader::new(&home.identifier)
            .code(&home.id)
            .chip(status_key(home.status), status_tone(home.status))
            .meta(count_as(home.size as usize, "person", "people"))
            .meta(place_name(&home.site))
            .show(ui, |ui| {
                let mut record = Button::new("Record a visit")
                    .icon(Icon::ClipboardPlus)
                    .size(ButtonSize::Sm);
                if closed {
                    record = record.reason("The household has left the survey, so it takes no more visits.");
                }
                visit = ui.add(record).clicked();
                let mut leave = Button::new("Exit interview")
                    .icon(Icon::DoorOpen)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Sm);
                if closed {
                    leave = leave.reason("The household has left the survey.");
                }
                exit = ui.add(leave).clicked();
                remove = ui
                    .add(
                        Button::new("Remove")
                            .icon(Icon::Trash2)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm),
                    )
                    .clicked();
                ui.add(ApiLink::new(format!("/households?identifier={}", home.identifier)));
            });
        let t = tokens(ui.ctx());
        let link = |ui: &mut Ui, words: String, to: String, go: &mut Option<String>| {
            if ui.add(TextLink::new(words)).clicked() {
                *go = Some(to);
            }
        };
        if !twins.is_empty() {
            let said = if twins.len() == 1 {
                "Another household carries".to_owned()
            } else {
                format!("{} other households carry", twins.len())
            };
            Banner::new(
                BannerTone::Critical,
                format!(
                    "{said} the same identifier: {}. The register refuses the second registration; remove the one registered by mistake.",
                    twins.join(", ")
                ),
            )
            .show(ui);
        }
        let cell = std::cell::RefCell::new(None::<String>);
        let to = &cell;
        Section::new("Identifier")
            .description("What names the household. The server gave it when the registration was filed.")
            .show(ui, |ui| {
                let region = home.region.clone();
                let site = home.site.clone();
                let registration = registration.clone();
                DescriptionList::new()
                    .columns(2)
                    .code("Identifier", &home.identifier)
                    .text("Registered", when(home.registered_at))
                    .item("Region", move |ui| {
                        let mut go = None;
                        link(ui, place_name(&region), format!("places/{region}"), &mut go);
                        if go.is_some() {
                            *to.borrow_mut() = go;
                        }
                    })
                    .item("Site", move |ui| {
                        let mut go = None;
                        link(ui, place_name(&site), format!("places/{site}"), &mut go);
                        if go.is_some() {
                            *to.borrow_mut() = go;
                        }
                    })
                    .item("Registration", move |ui| match &registration {
                        None => {
                            ui.label("Not on this server");
                        }
                        Some(r) => {
                            let mut go = None;
                            link(
                                ui,
                                r.receipt[..12].to_owned(),
                                format!("responses/{}", r.receipt),
                                &mut go,
                            );
                            if go.is_some() {
                                *to.borrow_mut() = go;
                            }
                        }
                    })
                    .show(ui);
            });
        let label = |id: &str, fallback: &str| enroll_question(id).map_or(fallback.to_owned(), |q| q.label.to_owned());
        Section::new("What the household told the survey")
            .description("The answers the registration holds, as the server stores them.")
            .show(ui, |ui| {
                DescriptionList::new()
                    .columns(2)
                    .text(label("born", "Born"), home.born.to_string())
                    .text(label("size", "People"), home.size.to_string())
                    .text(label("source", "Source"), source_text(&home.source))
                    .text(
                        label("treats", "Treats water"),
                        match home.treats {
                            None => "Not answered",
                            Some(true) => "Yes",
                            Some(false) => "No",
                        },
                    )
                    .show(ui);
            });
        Section::new("Program")
            .description("The program the household is registered in.")
            .show(ui, |ui| {
                let status = home.status;
                DescriptionList::new()
                    .columns(2)
                    .text("Program", PROGRAM_WATER.1)
                    .item("State", move |ui| {
                        ui.add(sokkel_egui::Chip::new(status_key(status)).tone(status_tone(status)));
                    })
                    .text("Visits", visits.len().to_string())
                    .text(
                        "New visits",
                        if closed {
                            "Not taken: the household has left the survey."
                        } else {
                            "Taken: each visit is its own record."
                        },
                    )
                    .show(ui);
            });
        Section::new("Visits")
            .flat(true)
            .meta(visits.len().to_string())
            .description("Every visit filed for the household, newest first. Each opens to its answers.")
            .show(ui, |ui| {
                if visits.is_empty() {
                    ui.label(
                        sokkel_egui::TextRole::Help
                            .rich(
                                &t,
                                "No visit has been filed for this household yet. Record one with Record a visit.",
                            )
                            .color(t.color("muted-foreground")),
                    );
                    return;
                }
                ui.spacing_mut().item_spacing.y = 8.0;
                for (index, visit) in visits.iter().enumerate() {
                    let form = form_of(&visit.form_id);
                    let day = visit
                        .visit_date
                        .as_deref()
                        .map_or_else(|| when(visit.received_at), on_day);
                    let heading = format!("{}, {day}", form.map_or(visit.form_id.as_str(), |f| f.title));
                    let state = visit.state;
                    Unfoldable::new(("visit", &visit.id), heading)
                        .default_open(index == 0)
                        .aside(move |ui| {
                            ui.add(state_chip(state));
                        })
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 8.0;
                            let mut list = DescriptionList::new().side(256.0);
                            if let Some(form) = form {
                                for q in questions_of(form) {
                                    let text = answer_text(Some(q), visit.answers.get(q.id));
                                    if !text.is_empty() {
                                        list = list.text(q.label, text);
                                    }
                                }
                            }
                            list.show(ui);
                            let open = Button::new("Open the receipt")
                                .trailing(Icon::ArrowUpRight)
                                .variant(ButtonVariant::Ghost)
                                .size(ButtonSize::Sm);
                            if ui.add(open).clicked() {
                                *to.borrow_mut() = Some(format!("responses/{}", visit.receipt));
                            }
                        });
                }
            });
        if remove {
            self.households.removing = Some(home.id.clone());
        }
        if self.households.removing.as_deref() == Some(home.id.as_str()) {
            let answer = ConfirmDialog::new("remove-household", format!("Remove {}?", home.identifier), "Remove")
                .detail("The household leaves the register and takes no more visits. The records filed for it stay on the server.")
                .destructive(true)
                .show(ui.ctx());
            match answer {
                Some(ConfirmAnswer::Confirm) => {
                    self.households.removing = None;
                    match self.backend.remove_household(&home.id) {
                        Ok(()) => {
                            Toast::success("Household removed")
                                .description(format!(
                                    "{} is no longer in the register; its records stay on the server.",
                                    home.identifier
                                ))
                                .show(ui.ctx());
                            go = Some("people".into());
                        }
                        Err(failure) => {
                            Toast::new("Not removed")
                                .kind(ToastKind::Error)
                                .description(failure.to_string())
                                .show(ui.ctx());
                        }
                    }
                }
                Some(_) => self.households.removing = None,
                None => {}
            }
        }
        if visit {
            go = Some(format!("forms/FRM-HOME/for/{}", home.id));
        }
        if exit {
            go = Some(format!("forms/FRM-EXIT/for/{}", home.id));
        }
        if let Some(path) = go.or(cell.into_inner()) {
            self.go(&path);
        }
    }
}
