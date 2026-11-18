//! Making a job, as the web's `NewJobDialog.tsx`: its name and code, its project and owner, its
//! steps from a starter, and when it runs.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Button, ButtonVariant, Dialog, Field, Gap, Input, Segmented, Select, Stack, TextField, TimeField, Toast, ToastKind,
};

use crate::app::RunsApp;
use crate::backend::{NewJob, STARTERS};
use crate::model::{PEOPLE, PROJECTS, Person, Schedule, day_of};
use crate::schedule::CADENCES;

/// The form while the dialog is open.
#[derive(Clone, Debug)]
pub struct NewJobForm {
    pub name: String,
    /// The code typed, when it is no longer the name's.
    pub code: Option<String>,
    pub description: String,
    pub project: usize,
    pub owner: usize,
    pub starter: usize,
    /// The cadence chosen: 0 for manual only, else one of `CADENCES` after it.
    pub cadence: usize,
    pub at: Option<String>,
    pub tried: bool,
}

impl NewJobForm {
    pub fn new(me: Person) -> Self {
        Self {
            name: String::new(),
            code: None,
            description: String::new(),
            project: 0,
            owner: PEOPLE.iter().position(|p| *p == me).unwrap_or(0),
            starter: 0,
            // daily, after manual only
            cadence: 1 + CADENCES.iter().position(|c| c.value.value() == "daily").unwrap_or(0),
            at: Some("03:00".into()),
            tried: false,
        }
    }
}

/// A job's code from its name: `Weekly cleanup` is `weekly-cleanup`.
fn code_of(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_owned()
}

impl RunsApp {
    /// The dialog while it is open.
    pub fn new_job_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut form) = self.jobs_view.making.take() else {
            return;
        };
        let id = form.code.clone().unwrap_or_else(|| code_of(&form.name));
        let name_error = form.name.trim().is_empty().then_some("A job needs a name.");
        let code_error = if id.is_empty() {
            Some("A job needs a code.")
        } else if !id.starts_with(|c: char| c.is_ascii_lowercase())
            || !id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            Some("A code is lowercase letters, figures and dashes.")
        } else if self.backend.jobs.iter().any(|job| job.id == id) {
            Some("A job has this code already.")
        } else {
            None
        };
        let at_error = (form.cadence != 0 && form.at.is_none()).then_some("A schedule needs a time.");
        let mut submit = false;
        let mut cancel = false;
        let tried = form.tried;
        let close = Dialog::new("new-job", "New job")
            .description("Its steps start from a starter; its schedule and settings change on its page.")
            .show(
                ctx,
                |ui: &mut Ui| {
                    Stack::new(Gap::Lg).show(ui, |ui| {
                        ui.add(TextField::new("Name", &mut form.name).error(name_error.filter(|_| tried)));
                        let mut code = id.clone();
                        Field::new("Code")
                            .hint(Some("in links and the API"))
                            .error(code_error.filter(|_| tried))
                            .show(ui, |ui| {
                                if ui.add(Input::new(&mut code).mono(true).name("Code")).changed() {
                                    form.code = Some(code.clone());
                                }
                            });
                        ui.add(TextField::new("Description", &mut form.description).multiline(true));
                        Field::new("Project").show(ui, |ui| {
                            ui.add(Segmented::new(&mut form.project, PROJECTS.map(|p| p.word())).fill(true));
                        });
                        Field::new("Owner").show(ui, |ui| {
                            ui.add(
                                Select::new(&mut form.owner, PEOPLE.map(|p| p.word()), "Owner")
                                    .width(ui.available_width()),
                            );
                        });
                        Field::new("Steps").show(ui, |ui| {
                            ui.add(
                                Select::new(&mut form.starter, STARTERS.iter().map(|s| s.label), "Steps")
                                    .width(ui.available_width()),
                            );
                        });
                        Field::new("Runs").show(ui, |ui| {
                            let labels = std::iter::once("Manual only").chain(CADENCES.iter().map(|c| c.label));
                            ui.add(Select::new(&mut form.cadence, labels, "Runs").width(ui.available_width()));
                        });
                        if form.cadence != 0 {
                            TimeField::new("job-at", "At", &mut form.at)
                                .step(15)
                                .required(true)
                                .error(at_error.filter(|_| tried))
                                .show(ui);
                        }
                    });
                },
                |ui: &mut Ui| {
                    submit = ui.add(Button::new("Make job")).clicked();
                    cancel = ui.add(Button::new("Cancel").variant(ButtonVariant::Ghost)).clicked();
                },
            );
        if close || cancel {
            return;
        }
        if submit {
            form.tried = true;
            if name_error.is_none() && code_error.is_none() && at_error.is_none() {
                let schedule = (form.cadence != 0).then(|| Schedule {
                    cadence: CADENCES[form.cadence - 1].value,
                    at: form.at.clone().unwrap_or_else(|| "00:00".into()),
                    starts: day_of(self.now()),
                });
                let made = self.backend.create_job(NewJob {
                    id: id.clone(),
                    name: form.name.trim().to_owned(),
                    description: form.description.trim().to_owned(),
                    project: PROJECTS[form.project],
                    owner: PEOPLE[form.owner],
                    starter: STARTERS[form.starter].value,
                    schedule,
                });
                match made {
                    Ok(job) => {
                        Toast::success(format!("{} made", job.name))
                            .description(format!("{}, {} steps", job.id, job.steps.len()))
                            .show(ctx);
                        self.go(&format!("jobs/{}", job.id));
                        return;
                    }
                    Err(failure) => {
                        Toast::new("Could not make the job")
                            .kind(ToastKind::Error)
                            .description(failure)
                            .show(ctx);
                    }
                }
            }
        }
        self.jobs_view.making = Some(form);
    }
}
