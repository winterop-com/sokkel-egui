//! A job's page, as the web's `pages/JobPage.tsx`: its name and settings edited in place, its steps
//! as a graph, its schedule, and its recent runs.

use std::cell::RefCell;

use eframe::egui::Ui;
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, Columns, DataTable, DescriptionList, Empty, Gap, Icon, InlineEdit, PageHeader,
    Section, Stack, TableVariant, TableWords, Toast, ToastKind,
};
use sokkel_egui_graph::{GraphCanvas, GraphLink, GraphNode, GraphWords};

use crate::app::{LiveRun, RunsApp};
use crate::model::{
    Cadence, Job, PEOPLE, PROJECTS, Person, Project, Schedule, Status, counted, iso_string, stamp, took,
};
use crate::parts::{run_cells, run_columns, run_menu};
use crate::schedule::{CADENCES, next_fire, say_schedule};
use crate::timeline::is_live;

const MANUAL: &str = "Manual only";

/// A change asked on the page, saved once it is drawn.
enum Edit {
    Name(String),
    Cadence(Option<Cadence>),
    At(String),
    From(String),
    Owner(Person),
    Project(Project),
    Retries(i64),
    Timeout(Option<i64>),
    Description(String),
}

fn at_most_ten(value: &str) -> Option<String> {
    match value.trim().parse::<i64>() {
        Ok(n) if (0..=10).contains(&n) => None,
        _ => Some("Retries are a whole number from 0 to 10.".into()),
    }
}

fn positive(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        return None;
    }
    match value.trim().parse::<i64>() {
        Ok(n) if (1..=1440).contains(&n) => None,
        _ => Some("A timeout is from 1 to 1,440 minutes, or empty for none.".into()),
    }
}

impl RunsApp {
    pub fn job_page(&mut self, ui: &mut Ui, runs: &[LiveRun], id: &str) {
        let ctx = ui.ctx().clone();
        let Some(job) = self.backend.job(id).cloned() else {
            let mut all = false;
            Empty::new(format!("No job {id}"))
                .description("It may have been deleted, or the demo data reset.")
                .show(ui, |ui| {
                    all = ui
                        .add(
                            Button::new("All jobs")
                                .icon(Icon::ListChecks)
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Sm),
                        )
                        .clicked();
                });
            if all {
                self.go("jobs");
            }
            return;
        };
        let now = self.now();
        let prefs = self.prefs();
        let own: Vec<&LiveRun> = runs.iter().filter(|r| r.run.job == job.id).collect();
        let recent: Vec<&LiveRun> = own.iter().take(50).copied().collect();
        let next = if job.paused {
            None
        } else {
            job.schedule.as_ref().and_then(|s| next_fire(s, now))
        };
        let schedule = job.schedule.clone();
        let edits: RefCell<Vec<Edit>> = RefCell::new(Vec::new());
        let (mut delete, mut pause, mut run) = (false, false, false);
        let mut table_out = None;
        Stack::new(Gap::Page).show(ui, |ui| {
            let said = if job.paused {
                "paused".to_owned()
            } else {
                say_schedule(schedule.as_ref())
            };
            let header = PageHeader::new(&job.name)
                .edit_title("Name", |name| {
                    name.trim().is_empty().then(|| "A job needs a name.".into())
                })
                .code(&job.id)
                .meta(format!(
                    "{said} · Owned by {} · Project {} · {}",
                    job.owner.word(),
                    job.project.word(),
                    counted(own.len() as i64, "run")
                ))
                .show(ui, |ui| {
                    // the header lays its actions from the end: Run now, Pause, then Delete
                    run = ui
                        .add(Button::new("Run now").icon(Icon::Play).size(ButtonSize::Sm))
                        .clicked();
                    let mut toggle = Button::new(if job.paused { "Resume" } else { "Pause" })
                        .icon(if job.paused { Icon::Play } else { Icon::Pause })
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm);
                    if schedule.is_none() {
                        toggle = toggle.reason("The job has no schedule to pause.");
                    }
                    pause = ui.add(toggle).clicked();
                    delete = ui
                        .add(
                            Button::new("Delete")
                                .icon(Icon::Trash2)
                                .variant(ButtonVariant::Ghost)
                                .size(ButtonSize::Sm),
                        )
                        .clicked();
                });
            if let Some(name) = header.renamed {
                edits.borrow_mut().push(Edit::Name(name.trim().to_owned()));
            }

            let chosen = self.job_step.clone();
            let mut pressed = None;
            Section::new("Steps")
                .meta(counted(job.steps.len() as i64, "step"))
                .flush(true)
                .show(ui, |ui| {
                    let nodes: Vec<GraphNode> = job
                        .steps
                        .iter()
                        .map(|step| GraphNode::new(&step.code).kind(&step.kind))
                        .collect();
                    let links: Vec<GraphLink> = job
                        .steps
                        .iter()
                        .flat_map(|step| step.needs.iter().map(|need| GraphLink::new(need, &step.code)))
                        .collect();
                    pressed = GraphCanvas::new(("job-graph", &job.id), &nodes, &links)
                        .selected(chosen.as_deref())
                        .words(GraphWords {
                            label: format!("{} steps", job.id),
                            ..Default::default()
                        })
                        .show(ui);
                });
            if let Some(code) = pressed {
                self.job_step = if chosen.as_ref() == Some(&code) {
                    None
                } else {
                    Some(code)
                };
            }
            if let Some(step) = job.steps.iter().find(|s| Some(&s.code) == self.job_step.as_ref()) {
                let mut close = false;
                let failed = own
                    .iter()
                    .filter(|r| r.at.state == Status::Failed && r.at.step.as_deref() == Some(step.code.as_str()))
                    .count();
                Section::new(&step.code)
                    .meta(&step.kind)
                    .actions(|ui| {
                        close = ui
                            .add(Button::new("Close").variant(ButtonVariant::Ghost).size(ButtonSize::Xs))
                            .clicked();
                    })
                    .show(ui, |ui| {
                        DescriptionList::new()
                            .side(112.0)
                            .code("Runs on", &step.kind)
                            .code(
                                "Needs",
                                if step.needs.is_empty() {
                                    "nothing".into()
                                } else {
                                    step.needs.join(", ")
                                },
                            )
                            .text("Usual time", took(job.pace * 1000.0))
                            .text("Failed", counted(failed as i64, "time"))
                            .show(ui);
                    });
                if close {
                    self.job_step = None;
                }
            }

            let edits = &edits;
            let job = &job;
            let schedule = &schedule;
            Columns::new(2).min(320.0).gap(16.0).show(ui, 2, |ui, index, _| {
                if index == 0 {
                    Section::new("Schedule")
                        .description("On the UTC clock. Each saves as it is changed.")
                        .show(ui, |ui| schedule_list(ui, job, schedule.as_ref(), next, edits));
                } else {
                    Section::new("Settings")
                        .description("Each saves as it is changed.")
                        .show(ui, |ui| settings_list(ui, job, edits));
                }
            });

            let meta = if own.len() > recent.len() {
                format!("the last {}", recent.len())
            } else {
                counted(own.len() as i64, "run")
            };
            Section::new("Recent runs").meta(meta).flush(true).show(ui, |ui| {
                let columns = run_columns(false);
                let rows: Vec<Vec<sokkel_egui::Cell>> =
                    recent.iter().map(|r| run_cells(r, now, prefs.times, false)).collect();
                let lives: Vec<bool> = recent.iter().map(|r| is_live(r.at.state)).collect();
                let keys: Vec<String> = recent.iter().map(|r| r.run.id.clone()).collect();
                // the run shown in the panel, drawn as opened, as the web's `opened`
                let open_row = self.opened.as_ref().and_then(|id| keys.iter().position(|k| k == id));
                let state = self.tables.entry(format!("job-runs-{}", job.id)).or_default();
                let out = DataTable::new(("job-runs", &job.id), &columns, &rows, state)
                    .opened(open_row)
                    .variant(TableVariant::List)
                    .framed(false)
                    .fits(true)
                    .keys(keys.iter())
                    .actions(|row| run_menu(lives[row], true))
                    .height(f32::INFINITY)
                    .footer(false)
                    .words(TableWords {
                        empty: "No runs yet. Run now starts one.".into(),
                        ..Default::default()
                    })
                    .show(ui);
                let opened = state.opened.take();
                table_out = Some((opened, out.action, keys));
            });
        });

        for edit in edits.into_inner() {
            self.save_edit(&ctx, &job, edit);
        }
        if run {
            self.run_now(&ctx, &job.id);
        }
        if pause {
            self.toggle_pause(&ctx, &job.id);
        }
        if delete {
            self.deleting = Some(job.id.clone());
        }
        if let Some((opened, action, keys)) = table_out {
            if let Some(id) = opened.and_then(|row| keys.get(row)) {
                self.open(id);
            }
            if let Some((row, action)) = action
                && let Some(id) = keys.get(row).cloned()
            {
                match action {
                    0 => self.open(&id),
                    1 => self.go(&format!("runs/{id}")),
                    2 => self.retry(&ctx, &id),
                    3 => {
                        ctx.copy_text(id.clone());
                        Toast::new("Copied").description(id).show(&ctx);
                    }
                    _ => self.cancel(&ctx, &[id]),
                }
            }
        }
    }

    fn save_edit(&mut self, ctx: &eframe::egui::Context, job: &Job, edit: Edit) {
        let today = iso_string(self.now())[..10].to_owned();
        let saved = self.backend.save_job(&job.id, |job| match edit {
            Edit::Name(name) => job.name = name,
            Edit::Cadence(None) => job.schedule = None,
            Edit::Cadence(Some(cadence)) => {
                let (at, starts) = job
                    .schedule
                    .as_ref()
                    .map_or(("03:00".into(), today), |s| (s.at.clone(), s.starts.clone()));
                job.schedule = Some(Schedule { cadence, at, starts });
            }
            Edit::At(at) => {
                if let Some(s) = job.schedule.as_mut() {
                    s.at = at;
                }
            }
            Edit::From(starts) => {
                if let Some(s) = job.schedule.as_mut() {
                    s.starts = starts;
                }
            }
            Edit::Owner(owner) => job.owner = owner,
            Edit::Project(project) => job.project = project,
            Edit::Retries(retries) => job.retries = retries,
            Edit::Timeout(timeout) => job.timeout = timeout,
            Edit::Description(description) => job.description = description,
        });
        if saved.is_none() {
            Toast::new("Could not save the job")
                .kind(ToastKind::Error)
                .description(format!("{} is gone.", job.id))
                .show(ctx);
        }
    }
}

/// The job's schedule, each part saved as it is changed.
fn schedule_list(ui: &mut Ui, job: &Job, schedule: Option<&Schedule>, next: Option<i64>, edits: &RefCell<Vec<Edit>>) {
    let none = schedule.is_none();
    let cadences: Vec<&str> = std::iter::once(MANUAL)
        .chain(CADENCES.iter().map(|c| c.label))
        .collect();
    let cadence_now = schedule
        .and_then(|s| CADENCES.iter().find(|c| c.value == s.cadence).map(|c| c.label))
        .unwrap_or(MANUAL);
    let at = schedule.map(|s| s.at.clone()).unwrap_or_default();
    let starts = schedule.map(|s| s.starts.clone()).unwrap_or_default();
    let next_said = if job.paused {
        "Paused".to_owned()
    } else {
        next.map_or("Only when started".into(), stamp)
    };
    DescriptionList::new()
        .side(96.0)
        .editable("Runs", |ui| {
            if let Some(label) = InlineEdit::new(("job-runs", &job.id), "Runs", cadence_now)
                .select(cadences)
                .show(ui)
            {
                let cadence = CADENCES.iter().find(|c| c.label == label).map(|c| c.value);
                edits.borrow_mut().push(Edit::Cadence(cadence));
            }
        })
        .editable("At", |ui| {
            let edit = InlineEdit::new(("job-at", &job.id), "At", at)
                .disabled(none)
                .placeholder("No schedule")
                .display(|at| format!("{at} UTC"))
                .validate(|at| {
                    sokkel_egui::read_time(at)
                        .is_none()
                        .then(|| "A schedule needs a time.".into())
                });
            if let Some(at) = edit.show(ui).and_then(|at| sokkel_egui::read_time(&at)) {
                edits.borrow_mut().push(Edit::At(at));
            }
        })
        .editable("From", |ui| {
            let edit = InlineEdit::new(("job-from", &job.id), "From", starts)
                .disabled(none)
                .placeholder("No schedule")
                .validate(|day| {
                    sokkel_egui::Day::parse(day)
                        .is_none()
                        .then(|| "A schedule needs a first day.".into())
                });
            if let Some(day) = edit.show(ui) {
                edits.borrow_mut().push(Edit::From(day));
            }
        })
        .text("Next run", next_said)
        .show(ui);
}

/// The job's settings, each saved as it is changed.
fn settings_list(ui: &mut Ui, job: &Job, edits: &RefCell<Vec<Edit>>) {
    DescriptionList::new()
        .side(96.0)
        .editable("Owner", |ui| {
            if let Some(owner) = InlineEdit::new(("job-owner", &job.id), "Owner", job.owner.word())
                .select(PEOPLE.map(|p| p.word()))
                .show(ui)
                && let Some(person) = PEOPLE.iter().find(|p| p.word() == owner)
            {
                edits.borrow_mut().push(Edit::Owner(*person));
            }
        })
        .editable("Project", |ui| {
            if let Some(project) = InlineEdit::new(("job-project", &job.id), "Project", job.project.word())
                .select(PROJECTS.map(|p| p.word()))
                .show(ui)
                && let Some(project) = PROJECTS.iter().find(|p| p.word() == project)
            {
                edits.borrow_mut().push(Edit::Project(*project));
            }
        })
        .editable("Retries", |ui| {
            if let Some(retries) = InlineEdit::new(("job-retries", &job.id), "Retries", job.retries.to_string())
                .validate(at_most_ten)
                .show(ui)
            {
                edits
                    .borrow_mut()
                    .push(Edit::Retries(retries.trim().parse().unwrap_or(0)));
            }
        })
        .editable("Timeout", |ui| {
            let value = job.timeout.map(|m| m.to_string()).unwrap_or_default();
            if let Some(timeout) = InlineEdit::new(("job-timeout", &job.id), "Timeout", value)
                .placeholder("None")
                .display(|m| format!("{m} min"))
                .validate(positive)
                .show(ui)
            {
                edits.borrow_mut().push(Edit::Timeout(timeout.trim().parse().ok()));
            }
        })
        .editable("Description", |ui| {
            if let Some(description) = InlineEdit::new(("job-description", &job.id), "Description", &job.description)
                .placeholder("None")
                .multiline(true)
                .show(ui)
            {
                edits.borrow_mut().push(Edit::Description(description));
            }
        })
        .show(ui);
}
