//! The funnels people saved, as the web's `Funnels.tsx`: a funnel built from steps with its preview
//! drawn as the steps change, and saving it.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    AsideWidth, Button, ButtonSize, ButtonVariant, Cell, Column, ConfirmAnswer, ConfirmDialog, DashboardGrid,
    DashboardPanel, DataTable, DropdownMenu, Field, Icon, IconButton, Input, MenuItem, PageHeader, PanelSpan, Section,
    Select, Split, StackLayout, TableVariant, Toast, ToastKind,
};
use sokkel_egui_charts::FunnelPanel;

use crate::app::AnalyticsApp;
use crate::days::{DAY, counted, day_of, short_day, thousands};
use crate::metrics::Figures;
use crate::model::{Funnel, Person, STEPS, Step};
use crate::view::range_name;

const WINDOWS: [i64; 6] = [7, 14, 30, 60, 90, 365];

fn window_name(days: i64) -> String {
    if days == 365 {
        "A year".into()
    } else {
        format!("{days} days")
    }
}

/// The demo's funnels, as the web's `funnels` seeds them, saved some days before `now`.
pub fn seed_funnels(now: i64) -> Vec<Funnel> {
    use Step::*;
    let funnel = |id: &str, name: &str, steps: Vec<Step>, window: i64, by: Person, days: i64| Funnel {
        id: id.into(),
        name: name.into(),
        steps,
        window,
        by,
        updated: now - days * DAY,
    };
    vec![
        funnel(
            "signup-to-paid",
            "Sign-up to paid",
            vec![Visited, SignedUp, Project, Invited, Upgraded],
            60,
            Person::Grace,
            6,
        ),
        funnel(
            "team-to-business",
            "Team to Business",
            vec![SignedUp, Upgraded, Seats, Business],
            365,
            Person::Ada,
            20,
        ),
        funnel(
            "activation",
            "Activation",
            vec![SignedUp, Project, Invited],
            14,
            Person::Linus,
            2,
        ),
    ]
}

/// A funnel being built: what it was opened from, and its name, steps and window now.
#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub of: Option<String>,
    pub name: String,
    pub steps: Vec<Step>,
    pub window: i64,
}

/// The funnels page: the saved funnels, the one being built, and a delete asked.
pub struct FunnelsPage {
    pub saved: Vec<Funnel>,
    pub draft: Option<Draft>,
    pub deleting: bool,
    made: u32,
}

impl FunnelsPage {
    pub fn new(now: i64) -> Self {
        Self {
            saved: seed_funnels(now),
            draft: None,
            deleting: false,
            made: 0,
        }
    }
}

/// What was asked on the page this frame.
enum Asked {
    Open(String),
    Save,
    Delete,
}

impl AnalyticsApp {
    pub fn funnels_page(&mut self, ui: &mut Ui) {
        let id = self.view.path.get(1).cloned();
        let chosen: Option<Funnel> = match id.as_deref() {
            Some("new") => None,
            Some(id) => self.funnels.saved.iter().find(|f| f.id == id).cloned(),
            None => self.funnels.saved.first().cloned(),
        };
        let missing = id.as_deref().is_some_and(|i| i != "new") && chosen.is_none();
        // a funnel opened starts its draft from what was saved
        let of = chosen.as_ref().map(|f| f.id.clone());
        if self.funnels.draft.as_ref().is_none_or(|d| d.of != of) {
            self.funnels.draft = Some(match &chosen {
                Some(f) => Draft {
                    of: of.clone(),
                    name: f.name.clone(),
                    steps: f.steps.clone(),
                    window: f.window,
                },
                None => Draft {
                    of: None,
                    name: String::new(),
                    steps: vec![Step::SignedUp, Step::Project],
                    window: 30,
                },
            });
        }
        let mut asked = None;
        PageHeader::new("Funnels")
            .description("How far teams get, step by step, from the day they sign up.")
            .show(ui, |ui| {
                if ui.add(Button::new("New").icon(Icon::Plus)).clicked() {
                    asked = Some(Asked::Open("new".into()));
                }
            });
        self.view_bar(ui);
        let segment = self.segment();
        let (span, previous) = self.spans();
        let compare = self.view.compare;
        let range = range_name(&self.view.range).to_lowercase();
        let saved = self.funnels.saved.clone();
        let mut draft = self.funnels.draft.take().expect("a draft was just made");
        let figures = Figures::new(&self.world);
        let stages = figures.funnel(segment, span.from, span.to, &draft.steps, draft.window);
        let before = compare.then(|| figures.funnel(segment, previous.from, previous.to, &draft.steps, draft.window));
        let list_state = self.tables.entry("funnels-saved".into()).or_default();
        list_state.opened = None;
        let was_opened = chosen.as_ref().and_then(|c| saved.iter().position(|f| f.id == c.id));
        let mut steps_state = std::mem::take(self.tables.entry("funnel-steps".into()).or_default());
        let mut stage_state = std::mem::take(self.tables.entry("funnel-stages".into()).or_default());
        let list_state = self.tables.get_mut("funnels-saved").expect("made above");
        let mut refused = None;
        Split::new(AsideWidth::Sm).start(true).show(
            ui,
            |ui| {
                if missing {
                    Section::new("No such funnel").show(ui, |ui| {
                        ui.label(format!(
                            "No saved funnel has the id {}. It may have been deleted.",
                            id.clone().unwrap_or_default()
                        ));
                    });
                    return;
                }
                let changed = chosen
                    .as_ref()
                    .is_none_or(|c| c.name != draft.name.trim() || c.window != draft.window || c.steps != draft.steps);
                let why = if draft.name.trim().is_empty() {
                    Some("A funnel needs a name.")
                } else if draft.steps.len() < 2 {
                    Some("A funnel needs two steps or more.")
                } else if !changed {
                    Some("Nothing has changed since it was saved.")
                } else {
                    None
                };
                let (title, description) = match &chosen {
                    Some(c) => (
                        c.name.clone(),
                        format!("Saved by {}, {}.", c.by.name(), short_day(day_of(c.updated))),
                    ),
                    None => ("New funnel".to_owned(), "Not saved yet.".to_owned()),
                };
                Section::new(title)
                    .description(description)
                    .actions(|ui| {
                        let mut save = Button::new("Save").icon(Icon::Save).size(ButtonSize::Sm);
                        if let Some(why) = why {
                            save = save.reason(why);
                        }
                        if ui.add(save).clicked() {
                            asked = Some(Asked::Save);
                        }
                        if chosen.is_some() && ui.add(IconButton::new(Icon::Trash2, "Delete funnel")).clicked() {
                            asked = Some(Asked::Delete);
                        }
                    })
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 16.0;
                        Field::new("Name").show(ui, |ui| {
                            ui.add(Input::new(&mut draft.name).placeholder("Sign-up to paid"))
                        });
                        Field::new("Steps")
                            .description("Each step counts the teams that took it after the step before.")
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 8.0;
                                let columns = vec![
                                    Column::new("#", 28.0),
                                    Column::new("Step", 144.0).fill(),
                                    Column::new("Reached", 100.0).numeric(),
                                    Column::new("", 28.0).numeric(),
                                ];
                                let rows: Vec<Vec<Cell>> = draft
                                    .steps
                                    .iter()
                                    .enumerate()
                                    .map(|(i, step)| {
                                        vec![
                                            Cell::Muted((i + 1).to_string()),
                                            Cell::Text(step.name().into()),
                                            Cell::Text(thousands(stages.get(i).map_or(0, |s| s.count))),
                                            Cell::Button {
                                                icon: Icon::X,
                                                label: format!("Take {} out", step.name()),
                                            },
                                        ]
                                    })
                                    .collect();
                                let out = DataTable::new("funnel-steps", &columns, &rows, &mut steps_state)
                                    .variant(TableVariant::List)
                                    .reorder(true)
                                    .bar(false)
                                    .footer(false)
                                    .keys(draft.steps.iter().map(|s| format!("{s:?}")))
                                    .show(ui);
                                if let Some((from, to)) = out.reordered {
                                    let mut next = draft.steps.clone();
                                    let step = next.remove(from);
                                    next.insert(to.min(next.len()), step);
                                    if next.contains(&Step::Visited) && next[0] != Step::Visited {
                                        refused = Some(step);
                                    } else {
                                        draft.steps = next;
                                    }
                                }
                                if let Some((row, _)) = out.pressed {
                                    draft.steps.remove(row);
                                }
                                let left: Vec<Step> =
                                    STEPS.iter().filter(|s| !draft.steps.contains(s)).copied().collect();
                                let add = ui.add(
                                    Button::new("Add step")
                                        .icon(Icon::Plus)
                                        .variant(ButtonVariant::Outline)
                                        .size(ButtonSize::Sm)
                                        .enabled(!left.is_empty()),
                                );
                                let items: Vec<MenuItem> = left.iter().map(|s| MenuItem::action(s.name())).collect();
                                if let Some(picked) = DropdownMenu::new(&items).show(&add) {
                                    let step = left[picked];
                                    if step == Step::Visited {
                                        draft.steps.insert(0, step);
                                    } else {
                                        draft.steps.push(step);
                                    }
                                }
                            });
                        Field::new("Within")
                            .description("How long after signing up a team has to take every step.")
                            .show(ui, |ui| {
                                let mut at = WINDOWS.iter().position(|w| *w == draft.window).unwrap_or(0);
                                let names: Vec<String> = WINDOWS.iter().map(|w| window_name(*w)).collect();
                                if ui.add(Select::new(&mut at, names, "Within").width(96.0)).changed() {
                                    draft.window = WINDOWS[at];
                                }
                            });
                    });
                let last = stages.last().map(|s| s.of_first);
                let within = draft.window;
                DashboardGrid::new("funnel-preview").show(ui, |grid| {
                    let rows: Vec<(String, f64)> = stages.iter().map(|s| (s.name.to_owned(), s.count as f64)).collect();
                    let empty = draft.steps.is_empty();
                    let range = range.clone();
                    grid.cell(PanelSpan::Full, move |ui| {
                        FunnelPanel::new("Preview", rows)
                            .subtitle(format!("teams that signed up, {range}"))
                            .empty(empty)
                            .show(ui);
                    });
                    let stages = &stages;
                    let before = &before;
                    let stage_state = &mut stage_state;
                    grid.cell(PanelSpan::Full, move |ui| {
                        let mut panel = DashboardPanel::new("Steps").flush(true);
                        if let Some(last) = last {
                            panel = panel.subtitle(format!(
                                "{last:.1}% made it through, within {}",
                                window_name(within).to_lowercase()
                            ));
                        }
                        panel.show(ui, |ui| {
                            let mut columns = vec![
                                Column::new("Step", 148.0).fill(),
                                Column::new("Reached", 84.0).numeric(),
                                Column::new("Of the step before", 132.0).numeric(),
                                Column::new("Of the first", 100.0).numeric(),
                            ];
                            if before.is_some() {
                                columns.push(Column::new("Period before", 116.0).numeric());
                            }
                            let rows: Vec<Vec<Cell>> = stages
                                .iter()
                                .map(|stage| {
                                    let mut cells = vec![
                                        Cell::Text(stage.name.into()),
                                        Cell::Text(thousands(stage.count)),
                                        Cell::Text(format!("{:.1}%", stage.of_previous)),
                                        Cell::Text(format!("{:.1}%", stage.of_first)),
                                    ];
                                    if let Some(before) = before {
                                        let then = before.iter().find(|b| b.step == stage.step);
                                        cells.push(Cell::Muted(
                                            then.map_or("-".to_owned(), |b| format!("{:.1}%", b.of_first)),
                                        ));
                                    }
                                    cells
                                })
                                .collect();
                            DataTable::new("funnel-stages", &columns, &rows, stage_state)
                                .variant(TableVariant::List)
                                .framed(false)
                                .bar(false)
                                .footer(false)
                                .show(ui);
                        });
                    });
                });
            },
            |ui| {
                Section::new("Saved")
                    .meta(saved.len().to_string())
                    .flush(true)
                    .show(ui, |ui| {
                        let columns = vec![
                            Column::new("Funnel", 116.0).fill(),
                            Column::new("Steps", 68.0).numeric(),
                        ];
                        let rows: Vec<Vec<Cell>> = saved
                            .iter()
                            .map(|f| vec![Cell::Text(f.name.clone()), Cell::Muted(counted(f.steps.len(), "step"))])
                            .collect();
                        DataTable::new("funnels-saved", &columns, &rows, list_state)
                            .opened(was_opened)
                            .variant(TableVariant::List)
                            .framed(false)
                            .bar(false)
                            .footer(false)
                            .stack_layout(StackLayout::Compact)
                            .words(sokkel_egui::TableWords {
                                empty: "No funnel saved yet.".into(),
                                ..Default::default()
                            })
                            .show(ui);
                    });
            },
        );
        let opened = self.tables.get_mut("funnels-saved").and_then(|s| s.opened.take());
        if let Some(f) = opened.filter(|i| Some(*i) != was_opened).and_then(|i| saved.get(i)) {
            asked = Some(Asked::Open(f.id.clone()));
        }
        *self.tables.entry("funnel-steps".into()).or_default() = steps_state;
        *self.tables.entry("funnel-stages".into()).or_default() = stage_state;
        if let Some(step) = refused {
            Toast::new(format!("{} stays where it was", step.name()))
                .kind(ToastKind::Warning)
                .description("Visiting the site comes first.")
                .show(ui.ctx());
        }
        match asked {
            Some(Asked::Open(id)) => {
                self.funnels.draft = None;
                self.go(&format!("funnels/{id}"));
            }
            Some(Asked::Save) => {
                let now = self.now();
                let name = draft.name.trim().to_owned();
                let id = match &chosen {
                    Some(c) => c.id.clone(),
                    None => {
                        self.funnels.made += 1;
                        format!("funnel-{}", self.funnels.made)
                    }
                };
                let funnel = Funnel {
                    id: id.clone(),
                    name: name.clone(),
                    steps: draft.steps.clone(),
                    window: draft.window,
                    by: Person::Ada,
                    updated: now,
                };
                match self.funnels.saved.iter_mut().find(|f| f.id == id) {
                    Some(known) => *known = funnel,
                    None => self.funnels.saved.push(funnel),
                }
                Toast::success(format!("{name} saved"))
                    .description(counted(draft.steps.len(), "step"))
                    .show(ui.ctx());
                draft.of = Some(id.clone());
                self.funnels.draft = Some(draft);
                self.go(&format!("funnels/{id}"));
            }
            Some(Asked::Delete) => {
                self.funnels.deleting = true;
                self.funnels.draft = Some(draft);
            }
            None => self.funnels.draft = Some(draft),
        }
        self.funnel_delete(ui.ctx(), chosen.as_ref());
    }

    fn funnel_delete(&mut self, ctx: &egui::Context, chosen: Option<&Funnel>) {
        if !self.funnels.deleting {
            return;
        }
        let Some(chosen) = chosen else {
            self.funnels.deleting = false;
            return;
        };
        let answer = ConfirmDialog::new("delete-funnel", format!("Delete {}?", chosen.name), "Delete")
            .detail("The funnel goes for everyone; the teams it counts stay as they are.")
            .destructive(true)
            .show(ctx);
        match answer {
            Some(ConfirmAnswer::Confirm) => {
                self.funnels.deleting = false;
                self.funnels.saved.retain(|f| f.id != chosen.id);
                self.funnels.draft = None;
                Toast::new(format!("{} deleted", chosen.name)).show(ctx);
                self.go("funnels");
            }
            Some(_) => self.funnels.deleting = false,
            None => {}
        }
    }
}
