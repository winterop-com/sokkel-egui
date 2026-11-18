//! Every job, as the web's `pages/Jobs.tsx`: its schedule and next run, its last run and how the
//! last two days went; a job opens on its page.

use eframe::egui::Ui;
use sokkel_egui::{
    Button, ButtonSize, Cell, ChipTone, Column, DataTable, FilterChip, Icon, MenuItem, PageHeader, TableWords, tokens,
};

use crate::app::{LiveRun, RunsApp};
use crate::model::{PROJECTS, Status, clock, counted, js_round};
use crate::parts::when;
use crate::schedule::{next_fire, say_schedule};

/// The sparkline's step: twelve of them span the two days of runs the console starts with.
const STEP: i64 = 4 * 3_600_000;

/// What the jobs list holds between frames.
#[derive(Default)]
pub struct JobsView {
    pub query: String,
    pub projects: Vec<String>,
    /// The new job's form while it is open.
    pub making: Option<crate::new_job::NewJobForm>,
}

impl RunsApp {
    pub fn jobs_page(&mut self, ui: &mut Ui, runs: &[LiveRun]) {
        let ctx = ui.ctx().clone();
        let t = tokens(&ctx);
        let now = self.now();
        let prefs = self.prefs();
        let asked = self.jobs_view.query.trim().to_lowercase();
        let jobs: Vec<_> = self
            .backend
            .jobs
            .iter()
            .filter(|job| {
                (self.jobs_view.projects.is_empty() || self.jobs_view.projects.iter().any(|p| p == job.project.word()))
                    && format!("{} {} {}", job.id, job.name, job.owner.word())
                        .to_lowercase()
                        .contains(&asked)
            })
            .cloned()
            .collect();
        let mut make = false;
        PageHeader::new("Jobs")
            .count(counted(self.backend.jobs.len() as i64, "job"))
            .show(ui, |ui| {
                make = ui
                    .add(Button::new("New").icon(Icon::Plus).size(ButtonSize::Sm))
                    .clicked();
            });
        if make {
            self.jobs_view.making = Some(crate::new_job::NewJobForm::new(self.me()));
        }
        // the web's widths less the 12 between columns, which the web's cells hold inside
        let columns = vec![
            Column::new("Job", 196.0).fill_share(1.4).sortable(true),
            Column::new("Project", 92.0).sortable(true),
            Column::new("Owner", 92.0).sortable(true),
            Column::new("Schedule", 148.0).fill().sortable(true),
            Column::new("Next run", 92.0).sortable(true),
            Column::new("Last run", 164.0).sortable(true),
            Column::new("2 days", 84.0),
            Column::new("Succeeded", 84.0).numeric().sortable(true),
        ];
        let rows: Vec<Vec<Cell>> = jobs
            .iter()
            .map(|job| {
                let own: Vec<&LiveRun> = runs.iter().filter(|r| r.run.job == job.id).collect();
                let done: Vec<&&LiveRun> = own
                    .iter()
                    .filter(|r| matches!(r.at.state, Status::Succeeded | Status::Failed))
                    .collect();
                let next = if job.paused {
                    None
                } else {
                    job.schedule.as_ref().and_then(|s| next_fire(s, now))
                };
                let recent: Vec<f64> = (0..12)
                    .map(|index| {
                        let from = now - (12 - index) * STEP;
                        own.iter()
                            .filter(|r| r.run.created > from && r.run.created <= from + STEP)
                            .count() as f64
                    })
                    .collect();
                let success = (!done.is_empty()).then(|| {
                    done.iter().filter(|r| r.at.state == Status::Succeeded).count() as f64 / done.len() as f64
                });
                vec![
                    Cell::NamedCode {
                        name: job.name.clone(),
                        code: job.id.clone(),
                        strong: false,
                    },
                    Cell::Text(job.project.word().into()),
                    Cell::Text(job.owner.word().into()),
                    if job.paused {
                        Cell::Chip("paused".into(), ChipTone::Warning)
                    } else {
                        Cell::Muted(say_schedule(job.schedule.as_ref()))
                    },
                    match next {
                        Some(at) => Cell::Number {
                            value: at as f64,
                            text: clock(at, false),
                        },
                        None => Cell::Muted("-".into()),
                    },
                    match own.first() {
                        Some(last) => Cell::ChipNote {
                            label: last.at.state.word().into(),
                            tone: ChipTone::Status(last.at.state.word()),
                            note: when(last.run.created, now, prefs.times),
                        },
                        None => Cell::Muted("Never".into()),
                    },
                    Cell::Trend(recent, t.color("chart-1")),
                    Cell::Number {
                        value: success.unwrap_or(-1.0),
                        text: success.map_or("-".into(), |s| format!("{}%", js_round(s * 100.0))),
                    },
                ]
            })
            .collect();
        let keys: Vec<String> = jobs.iter().map(|job| job.id.clone()).collect();
        let paused: Vec<(bool, bool)> = jobs.iter().map(|job| (job.paused, job.schedule.is_none())).collect();
        let mut query = std::mem::take(&mut self.jobs_view.query);
        let mut projects = std::mem::take(&mut self.jobs_view.projects);
        let options: Vec<(String, String)> = PROJECTS.iter().map(|p| (p.word().into(), p.word().into())).collect();
        let state = self.tables.entry("jobs".into()).or_default();
        let out = DataTable::new("jobs", &columns, &rows, state)
            .search(&mut query)
            .search_placeholder("Search")
            .toolbar(|ui| {
                FilterChip::new("job-projects", "Project", &options, &mut projects).show(ui);
            })
            .bar(true)
            .auto_rows()
            .keys(keys.iter())
            .actions(|row| {
                let (paused, unscheduled) = paused[row];
                vec![
                    MenuItem::action("Open").icon(Icon::ExternalLink),
                    MenuItem::action("Run now").icon(Icon::Play),
                    MenuItem::action(if paused { "Resume" } else { "Pause" })
                        .icon(if paused { Icon::Play } else { Icon::Pause })
                        .disabled(unscheduled),
                    MenuItem::Separator,
                    MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
                ]
            })
            .stretch(true)
            .footer(false)
            .words(TableWords {
                rows: "{count} jobs".into(),
                row: "{count} job".into(),
                ..Default::default()
            })
            .show(ui);
        let opened = state.opened.take();
        self.jobs_view.query = query;
        self.jobs_view.projects = projects;
        if let Some(id) = opened.and_then(|row| keys.get(row)) {
            self.go(&format!("jobs/{id}"));
        }
        if let Some((row, action)) = out.action
            && let Some(id) = keys.get(row).cloned()
        {
            match action {
                0 => self.go(&format!("jobs/{id}")),
                1 => self.run_now(&ctx, &id),
                2 => self.toggle_pause(&ctx, &id),
                _ => self.deleting = Some(id),
            }
        }
    }
}
