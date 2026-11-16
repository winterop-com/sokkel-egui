//! A schedule planner for Tidevann's jobs, an app of sokkel-egui's own: each job's runs on a
//! week calendar, the runs before now with how they went and the ones ahead to skip, the jobs shown
//! or hidden beside the week. It uses the library's public parts only.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, Button, ButtonSize, ButtonVariant, CalendarEvent, Checkbox, Chip, ChipTone,
    Day, DescriptionList, DesktopWindow, Empty, Gap, Icon, IconButton, Moment, Page, PageHeader, Rail, RailEntry,
    RailGroup, Section, Shell, ShellPart, SplitPane, Stack, StatusLine, StatusTone, TagChips, TopBar, WeekGrid,
    install,
};

/// Tidevann's mark, as sokkel-ui's examples draw it (`tidevann-shared/mark`).
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// A job and when it runs.
pub struct Job {
    pub id: &'static str,
    pub project: &'static str,
    pub owner: &'static str,
    /// Its calendar's kind color.
    pub color: &'static str,
    pub minutes: u32,
    /// The weekdays it runs on, Monday 0.
    pub weekdays: &'static [u32],
    /// The minutes of the day it starts at.
    pub at: &'static [u32],
}

const EVERY_DAY: &[u32] = &[0, 1, 2, 3, 4, 5, 6];
const WEEKDAYS: &[u32] = &[0, 1, 2, 3, 4];

pub const JOBS: [Job; 5] = [
    Job {
        id: "nightly-export",
        project: "harbor",
        owner: "ada",
        color: "blue",
        minutes: 45,
        weekdays: EVERY_DAY,
        at: &[2 * 60],
    },
    Job {
        id: "sync-inventory",
        project: "juniper",
        owner: "grace",
        color: "green",
        minutes: 20,
        weekdays: EVERY_DAY,
        at: &[30, 6 * 60 + 30, 12 * 60 + 30, 18 * 60 + 30],
    },
    Job {
        id: "rebuild-index",
        project: "larkspur",
        owner: "linus",
        color: "violet",
        minutes: 90,
        weekdays: &[0, 3],
        at: &[3 * 60],
    },
    Job {
        id: "send-digest",
        project: "harbor",
        owner: "margaret",
        color: "pink",
        minutes: 15,
        weekdays: WEEKDAYS,
        at: &[7 * 60],
    },
    Job {
        id: "import-orders",
        project: "juniper",
        owner: "grace",
        color: "teal",
        minutes: 30,
        weekdays: WEEKDAYS,
        at: &[8 * 60, 10 * 60, 12 * 60, 14 * 60, 16 * 60, 18 * 60],
    },
];

/// How a run went, or that it is ahead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    Succeeded,
    Failed,
    Scheduled,
    Skipped,
}

impl RunState {
    pub fn word(self) -> &'static str {
        match self {
            RunState::Succeeded => "succeeded",
            RunState::Failed => "failed",
            RunState::Scheduled => "scheduled",
            RunState::Skipped => "skipped",
        }
    }

    fn tone(self) -> ChipTone {
        match self {
            RunState::Succeeded => ChipTone::Status("succeeded"),
            RunState::Failed => ChipTone::Status("failed"),
            RunState::Scheduled => ChipTone::Info,
            RunState::Skipped => ChipTone::Status("skipped"),
        }
    }
}

/// One run of a job.
#[derive(Clone, Debug)]
pub struct Run {
    pub id: String,
    pub job: usize,
    pub start: Moment,
    pub end: Moment,
    pub state: RunState,
}

/// The planner: the week shown, the jobs hidden, the runs, and the run opened beside the week.
pub struct PlannerApp {
    pub appearance: Appearance,
    pub desktop: bool,
    /// The week's Monday.
    pub monday: Day,
    pub now: Moment,
    pub runs: Vec<Run>,
    pub hidden: Vec<bool>,
    pub opened: Option<String>,
}

impl PlannerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::on(Appearance::default());
        app.desktop = true;
        app
    }

    /// The planner on Wednesday 18 November 2026 at 09:00, the week of the 16th shown.
    pub fn on(appearance: Appearance) -> Self {
        let monday = Day::parse("2026-11-16").expect("a day");
        let now = Moment::new(monday.plus(2), 9, 0);
        let mut app = Self {
            appearance,
            desktop: false,
            monday,
            now,
            runs: Vec::new(),
            hidden: vec![false; JOBS.len()],
            opened: None,
        };
        app.plan();
        app
    }

    /// The runs of the week shown: every job at each of its times on each of its days, the ones
    /// before now with how they went (one in thirteen failed), the rest scheduled.
    pub fn plan(&mut self) {
        let mut runs = Vec::new();
        for (j, job) in JOBS.iter().enumerate() {
            for d in 0..7u32 {
                if !job.weekdays.contains(&d) {
                    continue;
                }
                let day = self.monday.plus(d as i32);
                for (k, at) in job.at.iter().enumerate() {
                    let start = Moment { day, minutes: *at };
                    let end_minutes = at + job.minutes;
                    let end = Moment {
                        day,
                        minutes: end_minutes.min(24 * 60 - 1),
                    };
                    let seed = (j as u32 * 31 + d * 7 + k as u32) % 13;
                    let state = if start >= self.now {
                        RunState::Scheduled
                    } else if seed == 5 {
                        RunState::Failed
                    } else {
                        RunState::Succeeded
                    };
                    runs.push(Run {
                        id: format!("{}-{day}-{at}", job.id),
                        job: j,
                        start,
                        end,
                        state,
                    });
                }
            }
        }
        runs.sort_by_key(|r| r.start);
        self.runs = runs;
    }

    /// The days of the week shown.
    pub fn days(&self) -> Vec<Day> {
        (0..7).map(|d| self.monday.plus(d)).collect()
    }

    /// The runs of the jobs shown, as the calendar's events.
    pub fn events(&self) -> (Vec<usize>, Vec<CalendarEvent>) {
        self.runs
            .iter()
            .enumerate()
            .filter(|(_, r)| !self.hidden[r.job])
            .map(|(i, r)| {
                let job = &JOBS[r.job];
                (
                    i,
                    CalendarEvent {
                        id: r.id.clone(),
                        title: job.id.into(),
                        start: r.start,
                        end: r.end,
                        color: job.color,
                        detail: Some(r.state.word().into()),
                        muted: r.state == RunState::Skipped,
                    },
                )
            })
            .unzip()
    }

    /// Move the week shown by `weeks`, keeping the jobs hidden.
    pub fn shift(&mut self, weeks: i32) {
        self.monday = self.monday.plus(weeks * 7);
        self.opened = None;
        self.plan();
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        let groups = vec![RailGroup::new(
            None,
            [RailEntry::new("week", "Week", Icon::CalendarDays).description("Every job's runs")],
        )];
        let mut place = "week".to_owned();
        let shell = if self.desktop {
            Shell::new().desktop(DesktopWindow::native())
        } else {
            Shell::new()
        };
        let scheduled = self.runs.iter().filter(|r| r.state == RunState::Scheduled).count();
        let failed = self.runs.iter().filter(|r| r.state == RunState::Failed).count();
        let mut account = None;
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                ui.add(Rail::new("Planner", &groups, &mut place).mark(TIDEVANN_MARK));
            }
            ShellPart::RailFoot => {
                account = AccountMenu::new("ada")
                    .place("tidevann.example.org")
                    .scheme(self.appearance.scheme)
                    .show(ui);
            }
            ShellPart::Top => {
                TopBar::new(&["Planner", "Week"]).show(ui, |_| {});
            }
            ShellPart::Page => Page::show(ui, |ui| self.page(ui)),
            ShellPart::Foot => {
                ui.add(
                    StatusLine::new("Runs planned by each job's schedule")
                        .tone(StatusTone::Quiet)
                        .item("scheduled", scheduled)
                        .item("failed", failed),
                );
            }
        });
        if let Some(AccountAction::Scheme(scheme)) = account {
            self.appearance.scheme = scheme;
        }
    }

    fn page(&mut self, ui: &mut Ui) {
        let t = sokkel_egui::tokens(ui.ctx());
        let sunday = self.monday.plus(6);
        let title = format!("{} to {}", self.monday.short(), sunday.medium());
        let (mut back, mut on, mut today) = (false, false, false);
        let shown = self.events().1.len();
        PageHeader::new(title).count(format!("{shown} runs")).show(ui, |ui| {
            on = ui.add(IconButton::new(Icon::ChevronRight, "The next week")).clicked();
            back = ui.add(IconButton::new(Icon::ChevronLeft, "The week before")).clicked();
            today = ui
                .add(
                    Button::new("This week")
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm),
                )
                .clicked();
        });
        if back {
            self.shift(-1);
        }
        if on {
            self.shift(1);
        }
        if today {
            let offset = (self.now.day.0 - self.monday.0).div_euclid(7);
            self.shift(offset);
        }
        let days = self.days();
        let (indices, events) = self.events();
        let opened_event = self
            .opened
            .as_ref()
            .and_then(|id| events.iter().position(|e| &e.id == id));
        let now = self.now;
        let mut pressed = None;
        let hidden = &mut self.hidden;
        let opened_run = self
            .opened
            .as_ref()
            .and_then(|id| self.runs.iter().position(|r| &r.id == id));
        let runs = &mut self.runs;
        SplitPane::new("planner-split")
            .default_share(0.74)
            .min_share(0.55)
            .max_share(0.85)
            .show(
                ui,
                |ui| {
                    let shown = WeekGrid::new("planner-week", &days, &events)
                        .now(now)
                        .scroll_to_hour(6)
                        .opened(opened_event)
                        .show(ui);
                    pressed = shown.pressed;
                },
                |ui| {
                    egui::ScrollArea::vertical().id_salt("planner-side").show(ui, |ui| {
                        egui::Frame::new().inner_margin(egui::Margin::same(16)).show(ui, |ui| {
                            Stack::new(Gap::Section).show(ui, |ui| {
                                Section::new("Jobs").flush(true).show(ui, |ui| {
                                    egui::Frame::new()
                                        .inner_margin(egui::Margin::symmetric(12, 8))
                                        .show(ui, |ui| {
                                            ui.vertical(|ui| {
                                                ui.spacing_mut().item_spacing.y = 4.0;
                                                for (j, job) in JOBS.iter().enumerate() {
                                                    let mut shown = !hidden[j];
                                                    ui.horizontal(|ui| {
                                                        let (dot, _) = ui.allocate_exact_size(
                                                            egui::vec2(10.0, 10.0),
                                                            egui::Sense::hover(),
                                                        );
                                                        ui.painter().circle_filled(
                                                            dot.center(),
                                                            5.0,
                                                            t.color(&format!("kind-{}", job.color)),
                                                        );
                                                        ui.add_space(4.0);
                                                        if ui.add(Checkbox::new(&mut shown, job.id)).changed() {
                                                            hidden[j] = !shown;
                                                        }
                                                    });
                                                }
                                            });
                                        });
                                });
                                match opened_run {
                                    Some(r) => run_detail(ui, &mut runs[r]),
                                    None => {
                                        Empty::new("Pick a run")
                                            .icon(Icon::CalendarClock)
                                            .description("Its job, its time and how it went stand here.")
                                            .show(ui, |_| {});
                                    }
                                }
                            });
                        });
                    });
                },
            );
        if let Some(e) = pressed {
            self.opened = Some(self.runs[indices[e]].id.clone());
        }
    }
}

/// A run's details beside the week, and Skip this run for one ahead.
fn run_detail(ui: &mut Ui, run: &mut Run) {
    let job = &JOBS[run.job];
    let when = format!(
        "{}, {:02}:{:02} to {:02}:{:02}",
        run.start.day.with_weekday(),
        run.start.minutes / 60,
        run.start.minutes % 60,
        run.end.minutes / 60,
        run.end.minutes % 60
    );
    let project = vec![job.project.to_owned()];
    let state = run.state;
    let mut skip = false;
    Section::new(job.id)
        .actions(|ui| {
            ui.add(Chip::new(state.word()).tone(state.tone()));
        })
        .show(ui, |ui| {
            DescriptionList::new()
                .text("When", when)
                .item("Project", |ui| {
                    ui.add(TagChips::new(&project));
                })
                .text("Owner", job.owner)
                .text("Every", schedule_words(job))
                .show(ui);
            if state == RunState::Scheduled {
                ui.add_space(12.0);
                skip = ui
                    .add(
                        Button::new("Skip this run")
                            .icon(Icon::SkipForward)
                            .variant(ButtonVariant::Outline),
                    )
                    .clicked();
            }
        });
    if skip {
        run.state = RunState::Skipped;
    }
}

/// When a job runs, in words.
fn schedule_words(job: &Job) -> String {
    let days = match job.weekdays.len() {
        7 => "Every day".to_owned(),
        5 if job.weekdays == WEEKDAYS => "Weekdays".to_owned(),
        _ => job
            .weekdays
            .iter()
            .map(|d| ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][*d as usize])
            .collect::<Vec<_>>()
            .join(" and "),
    };
    let times: Vec<String> = job.at.iter().map(|m| format!("{:02}:{:02}", m / 60, m % 60)).collect();
    format!("{days} at {}", times.join(", "))
}

impl eframe::App for PlannerApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }
}
