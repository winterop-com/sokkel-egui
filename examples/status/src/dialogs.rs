//! A service's own dialogs, as the web's `MaintenanceDialog` and `DeployDialog`: a window it is
//! taken down for work, and a new version put out.

use eframe::egui::{self, Ui};
use sokkel_ui_egui::{Button, ButtonVariant, Columns, DateTimeField, Day, Dialog, Gap, Moment, Stack, TextField};

use crate::data::{self, Deploy, DeployState, Maintenance};

/// What a service's dialog was answered this frame.
pub enum Answer<T> {
    Cancel,
    Done(T),
}

/// Draw a dialog with Cancel and its confirm, the confirm shut with `why` until it can go.
fn dialog(
    ctx: &egui::Context,
    id: &str,
    title: String,
    description: String,
    confirm: &str,
    why: Option<&str>,
    body: impl FnOnce(&mut Ui),
) -> Option<bool> {
    let mut answer = None;
    let close = Dialog::new(id, title)
        .description(description)
        .show(ctx, body, |ui: &mut Ui| {
            let mut go = Button::new(confirm);
            if let Some(why) = why {
                go = go.reason(why);
            }
            if ui.add(go).clicked() && why.is_none() {
                answer = Some(true);
            }
            if ui.add(Button::new("Cancel").variant(ButtonVariant::Ghost)).clicked() {
                answer = Some(false);
            }
        });
    answer.or(close.then_some(false))
}

/// Scheduling a window a service is taken down for work: what the work is, when it starts and ends.
pub struct MaintenanceForm {
    pub service: &'static str,
    pub title: String,
    pub starts: Option<Moment>,
    pub ends: Option<Moment>,
    issues: [bool; 2],
}

impl MaintenanceForm {
    /// A fresh form: tomorrow from two in the night to three, as the web's.
    pub fn new(service: &'static str, today: Day) -> Self {
        Self {
            service,
            title: String::new(),
            starts: Some(Moment::new(today.plus(1), 2, 0)),
            ends: Some(Moment::new(today.plus(1), 3, 0)),
            issues: [false; 2],
        }
    }

    /// Why it cannot be scheduled yet, in the web's words.
    pub fn why(&self) -> Option<&'static str> {
        if self.title.trim().is_empty() {
            Some("Say what the work is.")
        } else if self.starts.is_none() || self.ends.is_none() {
            Some("Pick when it starts and ends.")
        } else if self.issues.contains(&true) {
            Some("Put the times right first.")
        } else if self.ends <= self.starts {
            Some("It has to end after it starts.")
        } else {
            None
        }
    }

    /// The window the form makes, by `me`, its times in milliseconds of the board's clock.
    pub fn make(&self, today: Day, me: &'static str, existing: usize) -> Option<Maintenance> {
        let at = |m: Moment| f64::from(m.day.0 - today.0) * 86_400_000.0 + f64::from(m.minutes) * 60_000.0;
        Some(Maintenance {
            id: format!("maintenance-{}", existing + 1),
            service: self.service,
            title: self.title.trim().into(),
            starts: at(self.starts?),
            ends: at(self.ends?),
            by: me,
            cancelled: false,
        })
    }

    pub fn show(&mut self, ctx: &egui::Context, today: Day, now: Moment) -> Option<Answer<()>> {
        let why = self.why();
        let answer = dialog(
            ctx,
            "maintenance",
            format!("Schedule maintenance of {}", self.service),
            "The status page lists it ahead, and shows it while it runs.".into(),
            "Schedule",
            why,
            |ui| {
                Stack::new(Gap::Lg).show(ui, |ui| {
                    ui.add(TextField::new("What the work is", &mut self.title).placeholder("Moving to larger disks"));
                    let (starts, ends, issues) = (&mut self.starts, &mut self.ends, &mut self.issues);
                    let earliest = *starts;
                    Columns::new(2).min(224.0).gap(16.0).show(ui, 2, |ui, i, _| {
                        if i == 0 {
                            let shown = DateTimeField::new("maintenance-starts", "Starts", starts, today)
                                .min(now)
                                .show(ui);
                            issues[0] = shown.issue.is_some();
                        } else {
                            let shown = DateTimeField::new("maintenance-ends", "Ends", ends, today)
                                .min(earliest.unwrap_or(now))
                                .show(ui);
                            issues[1] = shown.issue.is_some();
                        }
                    });
                });
            },
        )?;
        Some(if answer { Answer::Done(()) } else { Answer::Cancel })
    }
}

/// Putting out a new version of a service: the version, one patch on by default, and what it changes.
pub struct DeployForm {
    pub service: &'static str,
    pub running: String,
    pub version: String,
    pub note: String,
}

impl DeployForm {
    pub fn new(service: &'static str, running: &str) -> Self {
        Self {
            service,
            running: running.to_owned(),
            version: data::next_version(running),
            note: String::new(),
        }
    }

    /// Why it cannot be deployed yet, in the web's words.
    pub fn why(&self) -> Option<String> {
        let version = self.version.trim();
        let well_written = version.strip_prefix('v').is_some_and(|rest| {
            let parts: Vec<&str> = rest.split('.').collect();
            parts.len() == 3
                && parts
                    .iter()
                    .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
        });
        if !well_written {
            Some("Write the version as v1.2.3.".into())
        } else if version == self.running {
            Some(format!("{} is running already.", self.running))
        } else {
            None
        }
    }

    /// The deploy the form starts, at `now` by `me`.
    pub fn make(&self, now: f64, me: &'static str, existing: usize) -> Deploy {
        Deploy {
            id: format!("deploy-{}-new-{existing}", self.service),
            service: self.service,
            version: self.version.trim().into(),
            at: now,
            by: me,
            state: DeployState::Running,
            note: self.note.trim().into(),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<Answer<()>> {
        let why = self.why();
        let answer = dialog(
            ctx,
            "deploy",
            format!("Deploy {}", self.service),
            format!("{} is running now.", self.running),
            "Deploy",
            why.as_deref(),
            |ui| {
                Stack::new(Gap::Lg).show(ui, |ui| {
                    ui.add(TextField::new("Version", &mut self.version));
                    ui.add(
                        TextField::new("What it changes", &mut self.note).placeholder("A fix for a slow validate step"),
                    );
                });
            },
        )?;
        Some(if answer { Answer::Done(()) } else { Answer::Cancel })
    }
}
