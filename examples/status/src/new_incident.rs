//! Opening an incident by hand, as the web's `NewIncidentDialog`: what is wrong, how bad, what it
//! touches, who takes it and what the status page says first.

use eframe::egui::{self, Ui};
use sokkel_ui_egui::{
    Button, ButtonVariant, Checkbox, Columns, Dialog, Gap, Segmented, Select, Stack, SwitchField, TextField, field,
    tokens,
};

use crate::data::{self, Incident, PEOPLE, SERVICES, SITES, Severity, State, Update};

/// The dialog's form, fresh each time it opens.
pub struct NewIncident {
    pub title: String,
    /// Minor, major or critical.
    pub severity: usize,
    pub services: Vec<bool>,
    /// Everywhere, then each site.
    pub place: usize,
    pub assignee: usize,
    pub message: String,
    pub shown: bool,
    on_call: usize,
}

/// What the dialog was answered this frame.
pub enum Answer {
    Cancel,
    Open(Incident),
}

impl NewIncident {
    /// A fresh form, the service it was opened from ticked, whoever is on call taking it.
    pub fn new(service: Option<&str>, on_call: &str) -> Self {
        let on_call = PEOPLE.iter().position(|p| *p == on_call).unwrap_or(0);
        Self {
            title: String::new(),
            severity: 0,
            services: SERVICES.iter().map(|s| Some(s.id) == service).collect(),
            place: 0,
            assignee: on_call,
            message: "We are looking into it.".into(),
            shown: true,
            on_call,
        }
    }

    /// Why the incident cannot be opened yet, in the web's words.
    pub fn why(&self) -> Option<&'static str> {
        if self.title.trim().is_empty() {
            Some("Say what is wrong first.")
        } else if !self.services.contains(&true) && self.place == 0 {
            Some("Tick a service or pick a site it touches.")
        } else if self.message.trim().is_empty() {
            Some("Write the first update.")
        } else {
            None
        }
    }

    /// The incident the form makes, opened at `now` by `me`, its code the next after `existing`'s.
    pub fn make(&self, existing: &[Incident], now: f64, me: &'static str) -> Incident {
        let next = existing
            .iter()
            .filter_map(|i| i.code.strip_prefix("INC-")?.parse::<u32>().ok())
            .max()
            .unwrap_or(0)
            + 1;
        Incident {
            id: format!("incident-{next}"),
            code: format!("INC-{next}"),
            title: self.title.trim().into(),
            severity: [Severity::Minor, Severity::Major, Severity::Critical][self.severity],
            state: State::Investigating,
            services: SERVICES
                .iter()
                .zip(&self.services)
                .filter(|(_, on)| **on)
                .map(|(s, _)| s.id)
                .collect(),
            sites: if self.place == 0 {
                Vec::new()
            } else {
                vec![SITES[self.place - 1].id]
            },
            assignee: PEOPLE[self.assignee],
            opened: now,
            resolved: None,
            updates: vec![Update {
                at: now,
                state: State::Investigating,
                message: self.message.trim().into(),
                by: me,
            }],
            public: self.shown,
            cause: None,
        }
    }

    /// Draw the dialog; `Some` once it is answered.
    pub fn show(&mut self, ctx: &egui::Context, existing: &[Incident], now: f64, me: &'static str) -> Option<Answer> {
        let t = tokens(ctx);
        let why = self.why();
        let mut answer = None;
        let mut open_it = false;
        let close = Dialog::new("new-incident", "New incident")
            .description("It opens as investigating, with your first update.")
            .width(672.0)
            .show(
                ctx,
                |ui: &mut Ui| {
                    Stack::new(Gap::Lg).show(ui, |ui| {
                        ui.add(
                            TextField::new("What is wrong", &mut self.title).placeholder("Slow exports from storage"),
                        );
                        field(ui, "Severity", None, |ui| {
                            ui.add(
                                Segmented::new(&mut self.severity, ["Minor", "Major", "Critical"]).dots([
                                    Some(t.color("warning")),
                                    Some(t.color("critical")),
                                    Some(t.color("critical")),
                                ]),
                            );
                        });
                        let services = &mut self.services;
                        sokkel_ui_egui::checkbox_list(ui, "Services it touches", |ui| {
                            Columns::new(4)
                                .min(120.0)
                                .gap(0.0)
                                .show(ui, SERVICES.len(), |ui, i, _| {
                                    ui.add(Checkbox::new(&mut services[i], SERVICES[i].id));
                                });
                        });
                        let places: Vec<&str> = std::iter::once("Everywhere")
                            .chain(SITES.iter().map(|s| s.city))
                            .collect();
                        let on_call = self.on_call;
                        let (place, assignee) = (&mut self.place, &mut self.assignee);
                        Columns::new(2).min(160.0).gap(16.0).show(ui, 2, |ui, i, width| {
                            if i == 0 {
                                field(ui, "Where", None, |ui| {
                                    ui.add(Select::new(place, places.clone(), "Where").width(width));
                                });
                            } else {
                                let hint = (*assignee == on_call).then_some("on call");
                                field(ui, "Assignee", hint, |ui| {
                                    ui.add(Select::new(assignee, PEOPLE, "Assignee").width(width));
                                });
                            }
                        });
                        ui.add(TextField::new("First update", &mut self.message).multiline(true));
                        ui.add(SwitchField::new(&mut self.shown, "Show on the status page"));
                    });
                },
                |ui: &mut Ui| {
                    let mut open = Button::new("Open incident");
                    if let Some(why) = why {
                        open = open.reason(why);
                    }
                    open_it = ui.add(open).clicked() && why.is_none();
                    if ui.add(Button::new("Cancel").variant(ButtonVariant::Ghost)).clicked() {
                        answer = Some(Answer::Cancel);
                    }
                },
            );
        if open_it {
            answer = Some(Answer::Open(self.make(existing, now, me)));
        }
        answer.or(close.then_some(Answer::Cancel))
    }
}

/// A severity in the dialog's order, for a reader.
pub fn severity_name(severity: data::Severity) -> &'static str {
    match severity {
        data::Severity::Minor => "Minor",
        data::Severity::Major => "Major",
        data::Severity::Critical => "Critical",
    }
}
