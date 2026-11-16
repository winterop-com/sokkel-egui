//! The dashboard's dialogs, as the web's `Configure.tsx` and `DashboardDialog.tsx`: a widget's
//! options, and naming a new dashboard or renaming one.

use eframe::egui::{self, Ui};
use sokkel_egui::{Button, ButtonVariant, Dialog, Gap, Segmented, Select, Stack, SwitchField, TextField, field};

use crate::model::{Group, Metric, SERVICES, Style, Widget, WidgetKind, presets};
use crate::widgets::{kind_info, title_of};

/// Draw a dialog with Cancel and its confirm, the confirm shut with `why` until it can go: true
/// for the confirm, false for Cancel, Escape or the shade.
fn dialog(
    ctx: &egui::Context,
    id: &str,
    title: String,
    confirm: &str,
    why: Option<&str>,
    body: impl FnOnce(&mut Ui),
) -> Option<bool> {
    let mut answer = None;
    let close = Dialog::new(id, title).show(ctx, body, |ui: &mut Ui| {
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

const ROWS: [usize; 4] = [4, 6, 8, 10];

/// A widget's options, as the web's `ConfigureDialog`: its title (empty for the one its options
/// give it) and what its kind reads, applied to the dashboard being edited.
pub struct ConfigureForm {
    pub widget: Widget,
    pub title: String,
    metric: usize,
    /// 0 for nothing beside it, else 1 + the metric's place among the others.
    compare: usize,
    style: usize,
    group: usize,
    rows: usize,
    service: usize,
    pub text: String,
    failed_only: bool,
    hide_done: bool,
    show_next: bool,
}

impl ConfigureForm {
    pub fn new(widget: &Widget) -> Self {
        let o = &widget.options;
        let metric = o.metric.unwrap_or(match widget.kind {
            WidgetKind::Gauge => Metric::Cpu,
            _ => Metric::Runs,
        });
        let others: Vec<Metric> = Metric::ALL.into_iter().filter(|m| *m != metric).collect();
        Self {
            widget: widget.clone(),
            title: widget.title.clone().unwrap_or_default(),
            metric: metric.index(),
            compare: o
                .compare
                .flatten()
                .and_then(|c| others.iter().position(|m| *m == c))
                .map_or(0, |i| i + 1),
            style: usize::from(o.style == Some(Style::Area)),
            group: match o.group.unwrap_or(Group::Job) {
                Group::Job => 0,
                Group::Project => 1,
                Group::Person => 2,
            },
            rows: ROWS.iter().position(|r| *r == o.rows.unwrap_or(6)).unwrap_or(1),
            service: SERVICES.iter().position(|s| Some(*s) == o.service).unwrap_or(0),
            text: o.text.clone().unwrap_or_default(),
            failed_only: o.failed_only == Some(true),
            hide_done: o.hide_done == Some(true),
            show_next: o.show_next != Some(false),
        }
    }

    /// The widget with the form's options, its own title only where it differs from its kind's.
    pub fn applied(&self) -> Widget {
        let mut widget = self.widget.clone();
        let o = &mut widget.options;
        let kind = widget.kind;
        let metric = Metric::ALL[self.metric];
        if matches!(kind, WidgetKind::Stat | WidgetKind::Gauge | WidgetKind::Trend) {
            o.metric = Some(metric);
        }
        if kind == WidgetKind::Trend {
            let others: Vec<Metric> = Metric::ALL.into_iter().filter(|m| *m != metric).collect();
            o.compare = Some(self.compare.checked_sub(1).and_then(|i| others.get(i).copied()));
            o.style = Some(if self.style == 1 { Style::Area } else { Style::Line });
        }
        if kind == WidgetKind::Bars {
            o.group = Some([Group::Job, Group::Project, Group::Person][self.group]);
        }
        if matches!(kind, WidgetKind::Runs | WidgetKind::Activity | WidgetKind::Upcoming) {
            o.rows = Some(ROWS[self.rows]);
        }
        if kind == WidgetKind::Runs {
            o.failed_only = Some(self.failed_only);
        }
        if kind == WidgetKind::Log {
            o.service = Some(SERVICES[self.service]);
        }
        if kind == WidgetKind::Note {
            o.text = Some(self.text.clone());
        }
        if kind == WidgetKind::Todo {
            o.hide_done = Some(self.hide_done);
        }
        if kind == WidgetKind::Oncall {
            o.show_next = Some(self.show_next);
        }
        widget.title = None;
        let fallback = title_of(&widget);
        let given = self.title.trim();
        if !given.is_empty() && given != fallback {
            widget.title = Some(given.to_owned());
        }
        widget
    }

    /// Draws the dialog: the applied widget on Apply, `Some(None)` on Cancel.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<Option<Widget>> {
        let kind = self.widget.kind;
        let fallback = {
            let mut plain = self.applied();
            plain.title = None;
            title_of(&plain)
        };
        let title = format!("Configure {}", kind_info(kind).0.to_lowercase());
        let labels: Vec<&str> = Metric::ALL.iter().map(|m| m.info().label).collect();
        let answer = dialog(ctx, "configure", title, "Apply", None, |ui| {
            Stack::new(Gap::Lg).show(ui, |ui| {
                ui.add(
                    TextField::new("Title", &mut self.title)
                        .placeholder(fallback.clone())
                        .help(format!("Empty, it is called {fallback}")),
                );
                let width = ui.available_width();
                if matches!(kind, WidgetKind::Stat | WidgetKind::Gauge | WidgetKind::Trend) {
                    let label = if kind == WidgetKind::Trend { "Figure" } else { "Reads" };
                    field(ui, label, None, |ui| {
                        ui.add(Select::new(&mut self.metric, labels.clone(), label).width(width));
                    });
                }
                if kind == WidgetKind::Trend {
                    let beside: Vec<&str> = std::iter::once("Nothing")
                        .chain(
                            Metric::ALL
                                .iter()
                                .filter(|m| m.index() != self.metric)
                                .map(|m| m.info().label),
                        )
                        .collect();
                    field(ui, "Beside it", None, |ui| {
                        ui.add(Select::new(&mut self.compare, beside, "Beside it").width(width));
                    });
                    field(ui, "Drawn as", None, |ui| {
                        ui.add(Segmented::new(&mut self.style, ["Line", "Area"]));
                    });
                }
                if kind == WidgetKind::Bars {
                    field(ui, "Runs by", None, |ui| {
                        ui.add(Segmented::new(&mut self.group, ["Job", "Project", "Person"]));
                    });
                }
                if matches!(kind, WidgetKind::Runs | WidgetKind::Activity | WidgetKind::Upcoming) {
                    field(ui, "Rows", None, |ui| {
                        ui.add(Segmented::new(&mut self.rows, ROWS.map(|r| r.to_string())));
                    });
                }
                if kind == WidgetKind::Runs {
                    ui.add(SwitchField::new(&mut self.failed_only, "Only failed runs"));
                }
                if kind == WidgetKind::Log {
                    field(ui, "Service", None, |ui| {
                        ui.add(Segmented::new(&mut self.service, SERVICES));
                    });
                }
                if kind == WidgetKind::Note {
                    ui.add(TextField::new("Text", &mut self.text).multiline(true).help("Markdown"));
                }
                if kind == WidgetKind::Todo {
                    ui.add(SwitchField::new(&mut self.hide_done, "Hide what is done"));
                }
                if kind == WidgetKind::Oncall {
                    ui.add(SwitchField::new(&mut self.show_next, "Show who is next"));
                }
            });
        })?;
        Some(answer.then(|| self.applied()))
    }
}

/// Naming a new dashboard and what it starts with, or renaming one, as the web's `DashboardDialog`.
pub struct NameForm {
    /// The dashboard being renamed, by its id; a new one is made without it.
    pub renaming: Option<String>,
    pub name: String,
    /// What a new one starts as: 0 empty, else 1 + a preset's place.
    pub from: usize,
    /// The names in use, which a new name may not repeat, and the one being renamed.
    taken: Vec<String>,
    was: Option<String>,
}

impl NameForm {
    pub fn new(taken: Vec<String>) -> Self {
        Self {
            renaming: None,
            name: String::new(),
            from: 0,
            taken,
            was: None,
        }
    }

    pub fn rename(id: &str, name: &str, taken: Vec<String>) -> Self {
        Self {
            renaming: Some(id.to_owned()),
            name: name.to_owned(),
            from: 0,
            taken,
            was: Some(name.to_owned()),
        }
    }

    /// Whether the name is another dashboard's.
    fn clash(&self) -> bool {
        let wanted = self.name.trim().to_lowercase();
        self.taken
            .iter()
            .any(|one| one.to_lowercase() == wanted && Some(one) != self.was.as_ref())
    }

    /// Why it cannot be made yet, in the web's words.
    pub fn why(&self) -> Option<&'static str> {
        if self.name.trim().is_empty() {
            Some("A dashboard needs a name.")
        } else if self.clash() {
            Some("Another dashboard has this name.")
        } else {
            None
        }
    }

    /// Draws the dialog: true on Create or Rename, false on Cancel.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<bool> {
        let new = self.renaming.is_none();
        let title = if new { "New dashboard" } else { "Rename dashboard" };
        let confirm = if new { "Create" } else { "Rename" };
        let why = self.why();
        let clash = self.clash();
        let starts: Vec<String> = std::iter::once("Empty".to_owned())
            .chain(presets().iter().map(|p| format!("A copy of {} as it comes", p.name)))
            .collect();
        let entered = ctx.input(|i| i.key_pressed(egui::Key::Enter));
        let answer = dialog(ctx, "dashboard-name", title.into(), confirm, why, |ui| {
            Stack::new(Gap::Lg).show(ui, |ui| {
                ui.add(
                    TextField::new("Name", &mut self.name).error(clash.then_some("Another dashboard has this name.")),
                );
                if new {
                    let width = ui.available_width();
                    field(ui, "Starts as", None, |ui| {
                        ui.add(Select::new(&mut self.from, starts, "Starts as").width(width));
                    });
                }
            });
        });
        match answer {
            Some(answer) => Some(answer),
            None if entered && why.is_none() => Some(true),
            None => None,
        }
    }
}
