//! sokkel-ui's dashboard example ported to egui: Tidevann's home of live dashboards, each a grid
//! of widgets to arrange and configure. It uses the library's public parts only.

pub mod dialogs;
pub mod live;
pub mod model;
pub mod widgets;

use eframe::egui::{self, Ui};
use sokkel_ui_egui::{
    AccountAction, AccountMenu, Appearance, Button, ButtonSize, ButtonVariant, Cell, Column, ConfirmAnswer,
    ConfirmDialog, DashboardArrange, DashboardEdit, DashboardEditActions, DashboardWidget, DataTable, DesktopWindow,
    DropdownMenu, Empty, Gap, Icon, IconButton, MenuItem, Page, PageHeader, Rail, RailEntry, RailGroup, Segmented,
    SettingsDialog, SettingsSection, Shell, ShellPart, Stack, StackPlace, StatusLine, StatusTone, Switch, TableState,
    TableVariant, ThemeWords, TopBar, install, move_item, setting_row, theme_settings,
};

use dialogs::{ConfigureForm, NameForm};
use live::{Health, Live, RunState, clock_of, day_of};
use model::{Dashboard, Ids, Pace, Prefs, Todo, Widget, WidgetKind, copy_widgets, palette, presets, seed_todos};
use widgets::{WidgetAsk, WidgetState, draw, kind_info, palette_icon, title_of};

/// Tidevann's mark, as sokkel-ui's examples draw it (`tidevann-shared/mark`).
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// The rail's place for the list of every dashboard.
pub const ALL: &str = "dashboards";

/// A question the app is asking over the page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asking {
    /// Delete this dashboard, by its id.
    Delete(String),
    /// Throw the edits of the dashboard shown away.
    Leave,
    /// Put the demo's data back as it first was.
    ResetDemo,
}

/// Tidevann's home of dashboards: the boards, the place shown and its edit mode, the live source,
/// the to-dos, the person's preferences and the dialogs open over the page.
pub struct DashboardApp {
    pub appearance: Appearance,
    pub desktop: bool,
    pub boards: Vec<Dashboard>,
    /// The board shown by its id, or `ALL` for the list of them.
    pub place: String,
    pub edit: DashboardEdit<Widget>,
    pub configure: Option<ConfigureForm>,
    pub naming: Option<NameForm>,
    pub asking: Option<Asking>,
    /// The settings dialog's section, while it is open.
    pub settings: Option<String>,
    pub live: Live,
    pub todos: Vec<Todo>,
    pub prefs: Prefs,
    pub ids: Ids,
    pub widgets: WidgetState,
    pub list: TableState,
    /// The running app's clock at the last tick, in milliseconds.
    last_tick: Option<f64>,
}

impl DashboardApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::on(Appearance::default());
        app.desktop = true;
        app
    }

    /// The app as it opens, on the first dashboard.
    pub fn on(appearance: Appearance) -> Self {
        let boards = presets();
        let order = boards.iter().map(|b| b.id.clone()).collect();
        Self {
            appearance,
            desktop: false,
            place: boards[0].id.clone(),
            boards,
            edit: DashboardEdit::default(),
            configure: None,
            naming: None,
            asking: None,
            settings: None,
            live: Live::begin(),
            todos: seed_todos(),
            prefs: Prefs::new(order),
            ids: Ids::default(),
            widgets: WidgetState::default(),
            list: TableState::default(),
            last_tick: None,
        }
    }

    /// The boards in the order the person put them; any not in it follow, oldest first.
    pub fn in_order(&self) -> Vec<&Dashboard> {
        let mut ranked: Vec<&Dashboard> = self
            .prefs
            .order
            .iter()
            .filter_map(|id| self.boards.iter().find(|b| &b.id == id))
            .collect();
        let mut rest: Vec<&Dashboard> = self
            .boards
            .iter()
            .filter(|b| !self.prefs.order.contains(&b.id))
            .collect();
        rest.sort_by(|a, b| a.created.total_cmp(&b.created));
        ranked.extend(rest);
        ranked
    }

    /// Go to a board by its id, or to `ALL`, leaving edit mode.
    pub fn open(&mut self, place: &str) {
        if self.place != place {
            self.place = place.to_owned();
            self.edit.cancel();
            self.configure = None;
        }
    }

    /// One tick of the live source.
    pub fn tick(&mut self) {
        self.live.tick(self.prefs.pace.every() as f64);
    }

    /// Makes a dashboard, empty or a copy of a starting one, listed last, and opens it.
    pub fn create(&mut self, name: &str, from: usize) {
        let widgets = match from.checked_sub(1).and_then(|i| presets().into_iter().nth(i)) {
            Some(preset) => copy_widgets(&preset.widgets, &mut self.ids).0,
            None => Vec::new(),
        };
        let id = format!("board-{}", self.boards.len() + 1);
        self.boards.push(Dashboard {
            id: id.clone(),
            name: name.to_owned(),
            base: widgets.clone(),
            widgets,
            created: self.live.at,
        });
        self.prefs.order.push(id.clone());
        self.open(&id);
    }

    /// A copy of a dashboard, its to-do lists with it, listed after it, and opens it.
    pub fn duplicate(&mut self, id: &str) {
        let Some(board) = self.boards.iter().find(|b| b.id == id).cloned() else {
            return;
        };
        let (widgets, taken) = copy_widgets(&board.widgets, &mut self.ids);
        let new_id = |old: &str| taken.iter().find(|(o, _)| o == old).map(|(_, n)| n.clone());
        let base: Vec<Widget> = board
            .base
            .iter()
            .map(|w| Widget {
                id: new_id(&w.id).unwrap_or_else(|| self.ids.next(w.kind)),
                ..w.clone()
            })
            .collect();
        let copied: Vec<Todo> = self
            .todos
            .iter()
            .filter_map(|t| {
                new_id(&t.widget).map(|widget| Todo {
                    id: format!("{}-{}", widget, t.id),
                    widget,
                    ..t.clone()
                })
            })
            .collect();
        self.todos.extend(copied);
        let made = format!("board-{}", self.boards.len() + 1);
        self.boards.push(Dashboard {
            id: made.clone(),
            name: format!("{} copy", board.name),
            widgets,
            base,
            created: self.live.at,
        });
        let at = self
            .prefs
            .order
            .iter()
            .position(|o| o == id)
            .map_or(self.prefs.order.len(), |i| i + 1);
        self.prefs.order.insert(at, made.clone());
        self.open(&made);
    }

    /// Deletes a dashboard with its to-do lists, and opens the one listed first.
    pub fn delete(&mut self, id: &str) {
        let Some(index) = self.boards.iter().position(|b| b.id == id) else {
            return;
        };
        let board = self.boards.remove(index);
        self.todos.retain(|t| !board.widgets.iter().any(|w| w.id == t.widget));
        self.prefs.order.retain(|o| o != id);
        let first = self.in_order().first().map(|b| b.id.clone());
        self.place = String::new();
        self.open(&first.unwrap_or_else(|| ALL.to_owned()));
    }

    /// Everything back as the demo first was, the look kept.
    pub fn reset_demo(&mut self) {
        let appearance = self.appearance.clone();
        let desktop = self.desktop;
        *self = Self::on(appearance);
        self.desktop = desktop;
    }

    /// Opens the name dialog: for a new dashboard, or renaming the one with this id.
    pub fn ask_name(&mut self, renaming: Option<&str>) {
        let taken: Vec<String> = self.boards.iter().map(|b| b.name.clone()).collect();
        self.naming = Some(match renaming.and_then(|id| self.boards.iter().find(|b| b.id == id)) {
            Some(board) => NameForm::rename(&board.id, &board.name, taken),
            None => NameForm::new(taken),
        });
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        // the live source reads at the person's pace while it is followed
        if self.prefs.live {
            let now = ui.input(|i| i.time) * 1000.0;
            let every = self.prefs.pace.every() as f64;
            match self.last_tick {
                Some(last) if now - last >= every => {
                    self.tick();
                    self.last_tick = Some(now);
                }
                None => self.last_tick = Some(now),
                _ => {}
            }
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(self.prefs.pace.every()));
        }
        let boards: Vec<(String, String)> = self.in_order().iter().map(|b| (b.id.clone(), b.name.clone())).collect();
        let groups = vec![
            RailGroup::new(
                None,
                [RailEntry::new(ALL, "All dashboards", Icon::LayoutGrid).count(boards.len() as u32)],
            ),
            RailGroup::new(
                Some("Dashboards"),
                boards
                    .iter()
                    .map(|(id, name)| RailEntry::new(id.clone(), name.clone(), Icon::LayoutDashboard))
                    .collect::<Vec<_>>(),
            ),
            RailGroup::new(
                Some("System"),
                [RailEntry::new("settings", "Settings", Icon::Settings2)],
            ),
        ];
        let mut place = self.place.clone();
        let shell = if self.desktop {
            Shell::new().desktop(DesktopWindow::native())
        } else {
            Shell::new()
        };
        let here = if self.place == ALL {
            "Dashboards".to_owned()
        } else {
            boards
                .iter()
                .find(|(id, _)| *id == self.place)
                .map(|(_, n)| n.clone())
                .unwrap_or_default()
        };
        let trail = ["Tidevann", here.as_str()];
        let running = self.live.runs.iter().filter(|r| r.state == RunState::Running).count();
        let troubled = self
            .live
            .sites
            .iter()
            .filter(|s| s.state != Health::Operational)
            .count();
        let at = clock_of(self.live.at, self.prefs.twelve_hour, true);
        let note = if self.prefs.live {
            format!("Live, read {}, last at {at}", self.prefs.pace.label().to_lowercase())
        } else {
            format!("Paused at {at}")
        };
        let live = self.prefs.live;
        let mut account = None;
        let (mut toggle_live, mut new_board) = (false, false);
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                ui.add(Rail::new("Tidevann", &groups, &mut place).mark(TIDEVANN_MARK));
            }
            ShellPart::RailFoot => {
                account = AccountMenu::new("ada")
                    .place("tidevann.example.org")
                    .scheme(self.appearance.scheme)
                    .show(ui);
            }
            ShellPart::Top => {
                TopBar::new(&trail).show(ui, |ui| {
                    let (icon, label) = if live {
                        (Icon::Pause, "Pause live data")
                    } else {
                        (Icon::Play, "Follow live data")
                    };
                    toggle_live = ui.add(IconButton::new(icon, label)).clicked();
                    new_board = ui.add(IconButton::new(Icon::Plus, "New dashboard")).clicked();
                });
            }
            ShellPart::Page => Page::show(ui, |ui| {
                if self.place == ALL {
                    self.list_page(ui);
                } else {
                    self.board_page(ui);
                }
            }),
            ShellPart::Foot => {
                let tone = if troubled > 0 {
                    StatusTone::Warn
                } else if live {
                    StatusTone::Live
                } else {
                    StatusTone::Quiet
                };
                ui.add(
                    StatusLine::new(note.clone())
                        .tone(tone)
                        .item("running", running)
                        .item("sites in trouble", troubled),
                );
            }
        });
        if place == "settings" {
            self.settings = Some("live".into());
        } else if place != self.place {
            self.open(&place);
        }
        if toggle_live {
            self.prefs.live = !self.prefs.live;
            self.last_tick = None;
        }
        if new_board {
            self.ask_name(None);
        }
        match account {
            Some(AccountAction::Scheme(scheme)) => self.appearance.scheme = scheme,
            Some(AccountAction::Settings) => self.settings = Some("live".into()),
            _ => {}
        }
        self.overlays(ui.ctx());
    }

    fn board_page(&mut self, ui: &mut Ui) {
        let Some(index) = self.boards.iter().position(|b| b.id == self.place) else {
            Empty::new("Nothing at this address")
                .description("There is no such dashboard.")
                .show(ui, |_| {});
            return;
        };
        let editing = self.edit.editing();
        let count = self.boards[index].widgets.len();
        let description = if editing {
            "Drag a widget by its grip to move it, set its width, configure or remove it, and drag one in from the palette to add it.".to_owned()
        } else {
            format!(
                "{count} {}, {}.",
                if count == 1 { "widget" } else { "widgets" },
                if self.prefs.live { "live" } else { "paused" }
            )
        };
        let entries = palette();
        let palette_widgets: Vec<DashboardWidget> = entries
            .iter()
            .map(|e| {
                DashboardWidget::new(e.id, e.label)
                    .icon(palette_icon(e.id, e.kind))
                    .span(e.span)
            })
            .collect();
        let mut more = None;
        let mut cancel_asked = false;
        let mut saved = false;
        Stack::new(Gap::Page).show(ui, |ui| {
            let board = &mut self.boards[index];
            let name = board.name.clone();
            let base = board.base.clone();
            let guard = self.prefs.confirm_cancel;
            let (edit, kept) = (&mut self.edit, &mut board.widgets);
            PageHeader::new(name).description(description).show(ui, |ui| {
                // right to left: More last in reading order, after Edit
                if !edit.editing() {
                    let items = [
                        MenuItem::action("Rename").icon(Icon::PencilLine),
                        MenuItem::action("Duplicate").icon(Icon::Copy),
                        MenuItem::Separator,
                        MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
                    ];
                    let trigger = ui.add(IconButton::new(Icon::Ellipsis, "More"));
                    more = DropdownMenu::new(&items).end(true).width(176.0).show(&trigger);
                }
                let outcome = DashboardEditActions::new(edit, kept)
                    .default_layout(&base)
                    .guard_cancel(guard)
                    .show(ui);
                cancel_asked = outcome.cancel_asked;
                saved = outcome.saved;
            });
            let editing = self.edit.editing();
            if board.widgets.is_empty() && !editing {
                let mut start = false;
                Empty::new("Nothing on this dashboard")
                    .icon(Icon::LayoutDashboard)
                    .description("Edit it and drag in the widgets to show.")
                    .show(ui, |ui| {
                        start = ui
                            .add(Button::new("Edit").icon(Icon::Pencil).size(ButtonSize::Sm))
                            .clicked();
                    });
                if start {
                    let kept = board.widgets.clone();
                    self.edit.edit(&kept);
                }
                return;
            }
            let layout = self.edit.layout(&mut board.widgets);
            let label = |w: &Widget| title_of(w);
            let live = &self.live;
            let twelve = self.prefs.twelve_hour;
            let (todos, state) = (&mut self.todos, &mut self.widgets);
            let mut asked = None;
            let configurable = |_: &Widget| true;
            let arranged = DashboardArrange::new(("board", &board.id), layout, &label)
                .arranging(editing)
                .palette(&palette_widgets)
                .configurable(&configurable)
                .show(ui, |ui, widget| {
                    if draw(ui, widget, live, twelve, editing, todos, state) == Some(WidgetAsk::Configure) {
                        asked = Some(widget.id.clone());
                    }
                });
            if let Some((entry, at)) = arranged.added
                && let Some(entry) = entries.iter().find(|e| e.id == entry)
            {
                let made = Widget {
                    id: self.ids.next(entry.kind),
                    kind: entry.kind,
                    span: entry.span,
                    title: None,
                    options: entry.options.clone(),
                };
                // a note opens its options as it is added, to be written
                if made.kind == WidgetKind::Note {
                    self.configure = Some(ConfigureForm::new(&made));
                }
                let layout = self.edit.layout(&mut board.widgets);
                let at = at.min(layout.len());
                layout.insert(at, made);
            }
            if let Some(id) = arranged.configure.or(asked) {
                let layout = self.edit.layout(&mut board.widgets);
                if let Some(widget) = layout.iter().find(|w| w.id == id) {
                    self.configure = Some(ConfigureForm::new(widget));
                }
            }
        });
        if saved {
            // the to-dos of a to-do list taken off go with it
            let boards = &self.boards;
            self.todos
                .retain(|t| boards.iter().any(|b| b.widgets.iter().any(|w| w.id == t.widget)));
        }
        if cancel_asked {
            self.asking = Some(Asking::Leave);
        }
        let id = self.boards[index].id.clone();
        match more {
            Some(0) => self.ask_name(Some(&id)),
            Some(1) => self.duplicate(&id),
            Some(3) => self.asking = Some(Asking::Delete(id)),
            _ => {}
        }
    }

    /// Every dashboard in the rail's order: dragged into another order, each opened, renamed,
    /// duplicated or deleted, as the web's `DashboardsPage`.
    fn list_page(&mut self, ui: &mut Ui) {
        let boards: Vec<Dashboard> = self.in_order().into_iter().cloned().collect();
        let count = boards.len();
        let mut new = false;
        PageHeader::new("Dashboards")
            .count(format!(
                "{count} {}",
                if count == 1 { "dashboard" } else { "dashboards" }
            ))
            .description("Drag a dashboard by its grip to change the order the rail lists them in.")
            .show(ui, |ui| {
                new = ui
                    .add(Button::new("New").icon(Icon::Plus).size(ButtonSize::Sm))
                    .clicked();
            });
        let columns = [
            Column::new("Name", 160.0).fill().stack(StackPlace::Primary),
            Column::new("Holds", 192.0).fill(),
            Column::new("Widgets", 96.0).numeric().stack(StackPlace::Secondary),
            Column::new("Made", 128.0).numeric(),
        ];
        let rows: Vec<Vec<Cell>> = boards
            .iter()
            .map(|b| {
                let mut kinds: Vec<&str> = Vec::new();
                for w in &b.widgets {
                    let label = kind_info(w.kind).0;
                    if !kinds.contains(&label) {
                        kinds.push(label);
                    }
                }
                let holds = if kinds.is_empty() {
                    "nothing yet".to_owned()
                } else {
                    kinds.iter().map(|k| k.to_lowercase()).collect::<Vec<_>>().join(", ")
                };
                vec![
                    Cell::Text(b.name.clone()),
                    Cell::Text(holds),
                    Cell::Text(b.widgets.len().to_string()),
                    Cell::Text(day_of(b.created).medium()),
                ]
            })
            .collect();
        let actions = |_: usize| {
            vec![
                MenuItem::action("Open").icon(Icon::ExternalLink),
                MenuItem::action("Rename").icon(Icon::PencilLine),
                MenuItem::action("Duplicate").icon(Icon::Copy),
                MenuItem::Separator,
                MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
            ]
        };
        ui.add_space(sokkel_ui_egui::tokens(ui.ctx()).density("page-gap"));
        let output = DataTable::new("dashboards", &columns, &rows, &mut self.list)
            .variant(TableVariant::List)
            .keys(boards.iter().map(|b| b.id.clone()))
            .reorder(true)
            .actions(actions)
            .footer(false)
            .show(ui);
        if new {
            self.ask_name(None);
        }
        if let Some((from, to)) = output.reordered {
            let mut order: Vec<String> = boards.iter().map(|b| b.id.clone()).collect();
            move_item(&mut order, from, to);
            self.prefs.order = order;
        }
        if let Some(row) = self.list.opened.take() {
            let id = boards[row].id.clone();
            self.open(&id);
        }
        if let Some((row, action)) = output.action {
            let id = boards[row].id.clone();
            match action {
                0 => self.open(&id),
                1 => self.ask_name(Some(&id)),
                2 => self.duplicate(&id),
                4 => self.asking = Some(Asking::Delete(id)),
                _ => {}
            }
        }
    }

    /// The dialogs over the page: a widget's options, a dashboard's name, the questions, settings.
    fn overlays(&mut self, ctx: &egui::Context) {
        if let Some(mut form) = self.configure.take() {
            match form.show(ctx) {
                Some(Some(changed)) => {
                    if let Some(board) = self.boards.iter_mut().find(|b| b.id == self.place) {
                        let layout = self.edit.layout(&mut board.widgets);
                        if let Some(widget) = layout.iter_mut().find(|w| w.id == changed.id) {
                            *widget = changed;
                        }
                    }
                }
                Some(None) => {}
                None => self.configure = Some(form),
            }
        }
        if let Some(mut form) = self.naming.take() {
            match form.show(ctx) {
                Some(true) => {
                    let name = form.name.trim().to_owned();
                    match &form.renaming {
                        Some(id) => {
                            if let Some(board) = self.boards.iter_mut().find(|b| &b.id == id) {
                                board.name = name;
                            }
                        }
                        None => self.create(&name, form.from),
                    }
                }
                Some(false) => {}
                None => self.naming = Some(form),
            }
        }
        if let Some(asking) = self.asking.clone() {
            let dialog = match &asking {
                Asking::Delete(id) => {
                    let board = self.boards.iter().find(|b| &b.id == id);
                    ConfirmDialog::new(
                        "delete-dashboard",
                        format!("Delete {}?", board.map_or("the dashboard", |b| b.name.as_str())),
                        "Delete",
                    )
                    .detail(format!(
                        "Its {} widgets and their to-dos go with it. This cannot be undone.",
                        board.map_or(0, |b| b.widgets.len())
                    ))
                    .destructive(true)
                }
                Asking::Leave => {
                    let name = self
                        .boards
                        .iter()
                        .find(|b| b.id == self.place)
                        .map_or(String::new(), |b| b.name.clone());
                    ConfirmDialog::new("leave-edits", "Throw the edits away?", "Throw away")
                        .detail(format!("{name} goes back to how it was before Edit."))
                        .destructive(true)
                }
                Asking::ResetDemo => ConfirmDialog::new("reset-demo", "Reset the demo data?", "Reset")
                    .detail(
                        "Every dashboard, widget and to-do goes back to how the demo started. This cannot be undone.",
                    )
                    .destructive(true),
            };
            match dialog.show(ctx) {
                Some(ConfirmAnswer::Confirm) => {
                    self.asking = None;
                    match asking {
                        Asking::Delete(id) => self.delete(&id),
                        Asking::Leave => self.edit.cancel(),
                        Asking::ResetDemo => {
                            self.reset_demo();
                            return;
                        }
                    }
                }
                Some(_) => self.asking = None,
                None => {}
            }
        }
        if let Some(mut active) = self.settings.take() {
            let sections = [
                SettingsSection::new("live", "Live data", Icon::Radio).group("Preferences"),
                SettingsSection::new("editing", "Editing", Icon::LayoutDashboard).group("Preferences"),
                SettingsSection::new("appearance", "Appearance", Icon::Palette).group("Preferences"),
                SettingsSection::new("data", "Data", Icon::Database)
                    .group("This demo")
                    .reset(false),
            ];
            let (appearance, prefs) = (&mut self.appearance, &mut self.prefs);
            let mut reset_demo = false;
            let outcome = SettingsDialog::new("settings", &sections, &mut active).show(ctx, |section, ui| match section {
                "live" => {
                    setting_row(
                        ui,
                        "Follow live",
                        "Read the source on every tick. Off, every widget holds still where it is.",
                        |ui| {
                            ui.add(Switch::new(&mut prefs.live, "Follow live"));
                        },
                    );
                    setting_row(ui, "Reading", "How often the source is read while following live.", |ui| {
                        let mut pace = Pace::ALL.iter().position(|p| *p == prefs.pace).unwrap_or(1);
                        if ui
                            .add(Segmented::new(&mut pace, Pace::ALL.map(|p| p.label())))
                            .changed()
                        {
                            prefs.pace = Pace::ALL[pace];
                        }
                    });
                    setting_row(
                        ui,
                        "Clock",
                        "How the times on the charts, the runs and the schedules are written.",
                        |ui| {
                            let mut clock = usize::from(prefs.twelve_hour);
                            if ui.add(Segmented::new(&mut clock, ["24-hour", "12-hour"])).changed() {
                                prefs.twelve_hour = clock == 1;
                            }
                        },
                    );
                }
                "editing" => {
                    setting_row(
                        ui,
                        "Ask before throwing edits away",
                        "Cancel asks first when a dashboard has edits that are not kept yet.",
                        |ui| {
                            ui.add(Switch::new(&mut prefs.confirm_cancel, "Ask before throwing edits away"));
                        },
                    );
                }
                "appearance" => theme_settings(ui, appearance, true, &ThemeWords::default()),
                _ => {
                    setting_row(
                        ui,
                        "Reset demo data",
                        "Puts the dashboards, their widgets, the to-do lists and these preferences back as they first were.",
                        |ui| {
                            reset_demo = ui
                                .add(Button::new("Reset").variant(ButtonVariant::Outline).size(ButtonSize::Sm))
                                .clicked();
                        },
                    );
                }
            });
            if outcome.reset {
                let fresh = Prefs::new(self.prefs.order.clone());
                match active.as_str() {
                    "live" => {
                        self.prefs.live = fresh.live;
                        self.prefs.pace = fresh.pace;
                        self.prefs.twelve_hour = fresh.twelve_hour;
                    }
                    "editing" => self.prefs.confirm_cancel = fresh.confirm_cancel,
                    "appearance" => self.appearance = Appearance::default(),
                    _ => {}
                }
            }
            if reset_demo {
                self.asking = Some(Asking::ResetDemo);
            }
            if !outcome.close {
                self.settings = Some(active);
            }
        }
    }
}

impl eframe::App for DashboardApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }
}
