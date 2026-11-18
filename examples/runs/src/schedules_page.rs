//! The runs the schedules will start, by day, and the jobs whose schedules are paused, as the
//! web's `pages/Schedules.tsx`.

use eframe::egui::Ui;
use sokkel_egui::{
    Banner, BannerTone, Button, ButtonSize, ButtonVariant, Cell, Column, DataTable, FilterChip, Icon, MenuItem,
    PageHeader, RunTimeline, Section, Segmented, TableVariant, TableWords, TimelineLane, TimelineRun, Toast,
    relative_time_short, tokens,
};

use crate::app::RunsApp;
use crate::model::{STATES, Status, clock, counted, day_of, say_day};
use crate::schedule::{FIRES, fires_between, say_schedule, shift_schedule};

const MINUTE: i64 = 60_000;
const HOUR: i64 = 60 * MINUTE;

/// The spans ahead, in hours, and how the choice names them.
const SPANS: [(i64, &str); 3] = [(24, "24 hours"), (72, "3 days"), (168, "Week")];

/// What the schedules page holds between frames.
#[derive(Default)]
pub struct SchedulesView {
    pub span: usize,
    pub picked: Vec<String>,
    pub chosen: Option<String>,
}

impl RunsApp {
    pub fn schedules_page(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let t = tokens(&ctx);
        let now = self.now();
        let prefs = self.prefs();
        let hours = SPANS[self.schedules_view.span].0;
        let minute = now.div_euclid(MINUTE) * MINUTE;
        let picked = self.schedules_view.picked.clone();
        let wanted = |id: &str| picked.is_empty() || picked.iter().any(|p| p == id);

        // the fires ahead, soonest first
        let mut fires: Vec<(String, i64)> = self
            .backend
            .jobs
            .iter()
            .filter(|job| !job.paused && wanted(&job.id))
            .filter_map(|job| job.schedule.as_ref().map(|s| (job, s)))
            .flat_map(|(job, s)| {
                fires_between(s, minute, minute + hours * HOUR, FIRES)
                    .into_iter()
                    .map(|at| (job.id.clone(), at))
            })
            .collect();
        fires.sort_by_key(|(_, at)| *at);

        let mut span = self.schedules_view.span;
        PageHeader::new("Schedules")
            .count(format!("{} ahead", counted(fires.len() as i64, "run")))
            .description("What the schedules will start, on the UTC clock.")
            .show(ui, |ui| {
                ui.add(Segmented::new(&mut span, SPANS.map(|(_, label)| label)));
            });
        self.schedules_view.span = span;
        if !prefs.scheduler {
            let mut on = false;
            Banner::new(
                BannerTone::Warning,
                "The scheduler is off, so none of these will start until it is on again.",
            )
            .show_with(ui, |ui| {
                let button = Button::new("Turn on")
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Sm);
                on = ui.add(button).clicked();
            });
            if on && let Some(mine) = self.backend.prefs.first_mut() {
                mine.scheduler = true;
            }
        }

        // the timeline looks back half as far as it looks ahead
        let from = minute - hours * HOUR / 2;
        let to = minute + hours * HOUR;
        let lanes: Vec<TimelineLane> = self
            .backend
            .jobs
            .iter()
            .filter(|job| job.schedule.is_some() && wanted(&job.id))
            .map(|job| {
                let mut runs: Vec<TimelineRun> = self
                    .backend
                    .runs
                    .iter()
                    .filter(|run| run.job == job.id && run.ended.unwrap_or(minute) >= from)
                    .map(|run| TimelineRun {
                        id: run.id.clone(),
                        at: run.created,
                        end: run.ended.or((run.state == Status::Running).then_some(minute)),
                        state: Some((
                            run.state.word().into(),
                            t.color(&format!("status-{}", run.state.word())),
                        )),
                    })
                    .collect();
                if let Some(schedule) = job.schedule.as_ref().filter(|_| !job.paused) {
                    runs.extend(
                        fires_between(schedule, minute, to, FIRES)
                            .into_iter()
                            .map(|at| TimelineRun {
                                id: format!("{}-{at}", job.id),
                                at,
                                end: None,
                                state: None,
                            }),
                    );
                }
                TimelineLane {
                    id: job.id.clone(),
                    name: job.id.clone(),
                    detail: Some(if job.paused {
                        "Paused".into()
                    } else {
                        say_schedule(job.schedule.as_ref())
                    }),
                    runs,
                }
            })
            .collect();
        let states: Vec<(String, eframe::egui::Color32)> = STATES
            .iter()
            .map(|s| (s.word().to_owned(), t.color(&format!("status-{}", s.word()))))
            .collect();
        let timeline = RunTimeline::new("schedule-timeline", &lanes, (from, to), minute, &states)
            .selected(self.schedules_view.chosen.as_deref())
            .shift(5 * MINUTE)
            .show(ui);
        if let Some(lane) = timeline.selected {
            self.schedules_view.chosen = if self.schedules_view.chosen.as_ref() == Some(&lane) {
                None
            } else {
                Some(lane)
            };
        }
        if let Some((lane, by)) = timeline.shifted
            && let Some(schedule) = self.backend.job(&lane).and_then(|job| job.schedule.clone())
        {
            let moved = shift_schedule(&schedule, by as f64);
            if moved.at == schedule.at && moved.starts == schedule.starts {
                Toast::new(format!("{lane} stays as it is"))
                    .description(format!(
                        "{}: a shift by whole periods lands on the same times.",
                        say_schedule(Some(&schedule))
                    ))
                    .show(&ctx);
            } else {
                let said = say_schedule(Some(&moved));
                self.backend.save_job(&lane, |job| job.schedule = Some(moved));
                Toast::new(format!("{lane} moved")).description(said).show(&ctx);
            }
        }

        // the jobs not on a schedule: paused, then started by hand only
        let aside: Vec<(String, String, bool)> = self
            .backend
            .jobs
            .iter()
            .filter(|job| job.paused && job.schedule.is_some())
            .chain(self.backend.jobs.iter().filter(|job| job.schedule.is_none()))
            .map(|job| {
                (
                    job.id.clone(),
                    say_schedule(job.schedule.as_ref()),
                    job.schedule.is_none(),
                )
            })
            .collect();
        if !aside.is_empty() {
            let mut pressed = None;
            let columns = vec![
                Column::new("Job", 160.0).fill(),
                Column::new("Schedule", 160.0).fill(),
                Column::new("", 104.0).numeric(),
            ];
            let rows: Vec<Vec<Cell>> = aside
                .iter()
                .map(|(id, said, manual)| {
                    vec![
                        Cell::Code(id.clone()),
                        Cell::Muted(said.clone()),
                        Cell::Button {
                            icon: Icon::Play,
                            label: if *manual { "Run now".into() } else { "Resume".into() },
                        },
                    ]
                })
                .collect();
            Section::new("Not on a schedule")
                .meta(counted(aside.len() as i64, "job"))
                .flush(true)
                .show(ui, |ui| {
                    let state = self.tables.entry("not-scheduled".into()).or_default();
                    let out = DataTable::new("not-scheduled", &columns, &rows, state)
                        .variant(TableVariant::List)
                        .framed(false)
                        .fits(true)
                        .height(f32::INFINITY)
                        .footer(false)
                        .show(ui);
                    pressed = out.pressed.map(|(row, _)| row);
                });
            if let Some((id, _, manual)) = pressed.and_then(|row| aside.get(row).cloned()) {
                if manual {
                    self.run_now(&ctx, &id);
                } else {
                    self.toggle_pause(&ctx, &id);
                }
            }
        }

        // the fires ahead, by day
        let today = day_of(now);
        let columns = vec![
            Column::new("Job", 180.0).fill().sortable(true),
            Column::new("At", 68.0).sortable(true),
            Column::new("Schedule", 148.0).fill(),
            Column::new("Project", 92.0).sortable(true),
            Column::new("Starts", 100.0).numeric().sortable(true),
        ];
        let rows: Vec<Vec<Cell>> = fires
            .iter()
            .filter_map(|(id, at)| self.backend.job(id).map(|job| (job, *at)))
            .map(|(job, at)| {
                vec![
                    Cell::NameThenCode {
                        name: job.name.clone(),
                        code: job.id.clone(),
                    },
                    Cell::Number {
                        value: at as f64,
                        text: clock(at, false),
                    },
                    Cell::Muted(say_schedule(job.schedule.as_ref())),
                    Cell::Text(job.project.word().into()),
                    Cell::Number {
                        value: at as f64,
                        text: relative_time_short((at - now) as f64),
                    },
                ]
            })
            .collect();
        let groups: Vec<String> = fires.iter().map(|(_, at)| say_day(&day_of(*at), &today)).collect();
        let jobs_of: Vec<String> = fires.iter().map(|(id, _)| id.clone()).collect();
        let options: Vec<(String, String)> = self
            .backend
            .jobs
            .iter()
            .filter(|job| job.schedule.is_some())
            .map(|job| (job.id.clone(), job.id.clone()))
            .collect();
        let mut picked = std::mem::take(&mut self.schedules_view.picked);
        let state = self.tables.entry("fires".into()).or_default();
        let out = DataTable::new("fires", &columns, &rows, state)
            .toolbar(|ui| {
                FilterChip::new("fire-jobs", "Job", &options, &mut picked).show(ui);
            })
            .bar(true)
            .groups(groups)
            .actions(|_| {
                vec![
                    MenuItem::action("Open job").icon(Icon::ExternalLink),
                    MenuItem::action("Run now").icon(Icon::Play),
                    MenuItem::action("Pause the schedule").icon(Icon::Pause),
                ]
            })
            .stretch(true)
            .footer(false)
            .words(TableWords {
                rows: "{count} runs".into(),
                row: "{count} run".into(),
                empty: "No schedule fires in this span.".into(),
                ..Default::default()
            })
            .show(ui);
        let opened = state.opened.take();
        self.schedules_view.picked = picked;
        if let Some(id) = opened.and_then(|row| jobs_of.get(row)) {
            self.go(&format!("jobs/{id}"));
        }
        if let Some((row, action)) = out.action
            && let Some(id) = jobs_of.get(row).cloned()
        {
            match action {
                0 => self.go(&format!("jobs/{id}")),
                1 => self.run_now(&ctx, &id),
                _ => self.toggle_pause(&ctx, &id),
            }
        }
    }
}
