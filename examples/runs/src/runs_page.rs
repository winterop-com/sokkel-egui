//! Every run, newest first, as the web's `pages/Runs.tsx`: searched, filtered, sorted and kept as
//! saved views; a run opens beside the list or on its page.

use eframe::egui::Ui;
use sokkel_egui::{
    Button, ButtonSize, DataTable, DropdownMenu, Filter, FilterBar, FilterField, FilterType, FilterValue, Icon,
    ListViewState, MenuItem, PageHeader, SavedView, SavedViews, SavedViewsAsked, TableWords, Toast,
};

use crate::app::{LiveRun, RunsApp};
use crate::model::{PEOPLE, PROJECTS, STATES, counted};
use crate::parts::{COLUMN_IDS, run_cells, run_columns, run_menu};
use crate::timeline::is_live;

/// What the runs list holds between frames.
#[derive(Default)]
pub struct RunsView {
    pub query: String,
    pub filters: Vec<Filter>,
    pub chosen: Option<String>,
}

impl RunsApp {
    /// The listing as it stands, for the saved views.
    fn runs_setup(&mut self) -> ListViewState {
        let state = self.tables.entry("runs".into()).or_default();
        ListViewState {
            filters: self.runs_view.filters.clone(),
            sort: state
                .sort
                .map(|(column, ascending)| (COLUMN_IDS[column].to_owned(), ascending)),
            hidden: state.hidden.iter().map(|c| COLUMN_IDS[*c].to_owned()).collect(),
        }
    }

    fn apply_runs_view(&mut self, state: &ListViewState) {
        self.runs_view.filters = state.filters.clone();
        let table = self.tables.entry("runs".into()).or_default();
        table.sort = state
            .sort
            .as_ref()
            .and_then(|(column, ascending)| COLUMN_IDS.iter().position(|c| c == column).map(|at| (at, *ascending)));
        table.hidden = state
            .hidden
            .iter()
            .filter_map(|c| COLUMN_IDS.iter().position(|id| id == c))
            .collect();
    }

    pub fn runs_page(&mut self, ui: &mut Ui, runs: &[LiveRun]) {
        let ctx = ui.ctx().clone();
        let now = self.now();
        let prefs = self.prefs();
        let current = self.runs_setup();
        let mut jobs: Vec<String> = self
            .backend
            .jobs
            .iter()
            .map(|job| job.id.clone())
            .chain(runs.iter().map(|r| r.run.job.clone()))
            .collect();
        jobs.sort();
        jobs.dedup();
        let fields: Vec<FilterField<'_, LiveRun>> = vec![
            FilterField::new("job", "Job", FilterType::Set, |r: &LiveRun| {
                FilterValue::Text(r.run.job.clone())
            })
            .options(jobs.iter().map(|j| (j.as_str(), j.as_str()))),
            FilterField::new("state", "State", FilterType::Set, |r: &LiveRun| {
                FilterValue::Text(r.at.state.word().into())
            })
            .options(STATES.iter().map(|s| (s.word(), s.word()))),
            FilterField::new("project", "Project", FilterType::Set, |r: &LiveRun| {
                FilterValue::Text(r.run.project.word().into())
            })
            .options(PROJECTS.iter().map(|p| (p.word(), p.word()))),
            FilterField::new("by", "Started by", FilterType::Set, |r: &LiveRun| {
                FilterValue::Text(r.run.by.word().into())
            })
            .options(
                PEOPLE
                    .iter()
                    .map(|p| (p.word(), p.word()))
                    .chain([("schedule", "schedule")]),
            ),
            FilterField::new("step", "Step", FilterType::Text, |r: &LiveRun| {
                FilterValue::Text(r.at.step.clone().unwrap_or_default())
            }),
            FilterField::new("started", "Started", FilterType::Date, |r: &LiveRun| {
                FilterValue::Day(crate::model::day_of(r.run.created))
            }),
            FilterField::new("duration", "Duration", FilterType::Number, |r: &LiveRun| {
                FilterValue::Number(crate::model::js_round(r.at.duration as f64 / 1000.0))
            }),
        ];
        let kept = sokkel_egui::filter_rows(runs.iter(), &self.runs_view.filters, &fields);
        let wanted = self.runs_view.query.trim().to_lowercase();
        let shown: Vec<&LiveRun> = kept
            .into_iter()
            .filter(|r| {
                wanted.is_empty() || format!("{} {} {}", r.run.id, r.run.job, r.run.project.word()).contains(&wanted)
            })
            .collect();
        let views: Vec<SavedView> = self
            .backend
            .views
            .iter()
            .map(|v| SavedView {
                id: v.id.clone(),
                name: v.name.clone(),
                state: v.state.clone(),
                locked: v.locked,
            })
            .collect();
        let quick: Vec<String> = self
            .backend
            .jobs
            .iter()
            .filter(|job| !job.paused)
            .map(|job| job.id.clone())
            .collect();
        let chosen = self.runs_view.chosen.clone().unwrap_or_else(|| "all".into());
        let mut asked = None;
        let mut run_job = None;
        PageHeader::new("Runs")
            .count(counted(runs.len() as i64, "run"))
            .show(ui, |ui| {
                let items: Vec<MenuItem> = quick.iter().map(|id| MenuItem::action(id).code(true)).collect();
                let trigger = ui.add_enabled(
                    !quick.is_empty(),
                    Button::new("Run a job")
                        .icon(Icon::Play)
                        .trailing(Icon::ChevronDown)
                        .size(ButtonSize::Sm),
                );
                if let Some(at) = DropdownMenu::new(&items).end(true).show(&trigger) {
                    run_job = quick.get(at).cloned();
                }
                asked = SavedViews::new("run-views", &views, Some(&chosen), &current).show(ui);
            });
        match asked {
            Some(SavedViewsAsked::Picked(id)) => {
                if let Some(view) = views.iter().find(|v| v.id == id) {
                    self.apply_runs_view(&view.state.clone());
                }
                self.runs_view.chosen = Some(id);
            }
            Some(SavedViewsAsked::Save) => {
                if let Some(view) = self.backend.views.iter_mut().find(|v| v.id == chosen) {
                    view.state = current.clone();
                }
            }
            Some(SavedViewsAsked::SaveNew(name)) => {
                let id = format!("view-{}", self.backend.views.len() + 1);
                self.backend.views.push(crate::model::RunView {
                    id: id.clone(),
                    name,
                    state: current.clone(),
                    locked: false,
                });
                self.runs_view.chosen = Some(id);
            }
            Some(SavedViewsAsked::Rename(name)) => {
                if let Some(view) = self.backend.views.iter_mut().find(|v| v.id == chosen) {
                    view.name = name;
                }
            }
            Some(SavedViewsAsked::Delete) => {
                self.backend.views.retain(|v| v.id != chosen);
                self.runs_view.chosen = Some("all".into());
                self.apply_runs_view(&ListViewState::default());
            }
            None => {}
        }
        if let Some(job) = run_job {
            self.run_now(&ctx, &job);
        }

        let columns = run_columns(true);
        let rows: Vec<Vec<sokkel_egui::Cell>> = shown.iter().map(|r| run_cells(r, now, prefs.times, true)).collect();
        let keys: Vec<String> = shown.iter().map(|r| r.run.id.clone()).collect();
        let lives: Vec<bool> = shown.iter().map(|r| is_live(r.at.state)).collect();
        let mut filters = std::mem::take(&mut self.runs_view.filters);
        let mut query = std::mem::take(&mut self.runs_view.query);
        let state = self.tables.entry("runs".into()).or_default();
        let out = DataTable::new("runs", &columns, &rows, state)
            .search(&mut query)
            .search_placeholder("Search")
            .toolbar(|ui| {
                FilterBar::new("run-filters", &fields, &mut filters).show(ui);
            })
            .bar(true)
            .selectable(true)
            .keys(keys.iter())
            .actions(|row| run_menu(lives[row], true))
            .stretch(true)
            .footer(false)
            .words(TableWords {
                rows: "{count} runs".into(),
                row: "{count} run".into(),
                ..Default::default()
            })
            .show(ui);
        let opened = state.opened.take();
        drop(fields);
        self.runs_view.filters = filters;
        self.runs_view.query = query;
        if let Some(row) = opened
            && let Some(id) = keys.get(row)
        {
            self.open(id);
        }
        if let Some((row, action)) = out.action
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
