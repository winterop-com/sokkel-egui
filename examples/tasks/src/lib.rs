//! sokkel-ui's tasks example in egui, ported from `examples/tasks/web/src/Tasks.tsx` and
//! `Settings.tsx`: lists in the rail, a quick add, and tasks as rows put in the person's own order
//! by drag, or as a board of their states. Every part is sokkel-egui's; this file holds the
//! example's data, where it is, and what each part's answer does, as the web example does.

use eframe::egui::{self, Color32, Ui};
use sokkel_egui::{
    AccountAction, AccountMenu, Appearance, AppearanceCommandWords, Board, BoardCard, BoardColumn, BoardMove, Cell,
    Column, CommandGroup, CommandPalette, ConfirmAnswer, ConfirmDialog, DataTable, Day, DesktopWindow, Editor, Empty,
    Icon, MenuItem, Page, PageHeader, QuickAdd, Rail, RailEntry, RailGroup, Segmented, SegmentedVariant, Select,
    SettingsDialog, SettingsSection, Shell, ShellPart, StackLayout, StackPlace, StatusLine, Switch, SwitchField,
    TableState, TableVariant, ThemeWords, TopBar, appearance_commands, install, palette_shortcut, place_commands,
    place_of, run_appearance_command, setting_row, theme_settings, tokens, toolbar,
};

mod data;

pub use data::{Order, Prefs, Task, TaskState, View, WeekStart};
use data::{PEOPLE, PROJECTS, STATES, seed};

const KEPT: &str = "tasks/kept";

/// A list in the rail: what a task needs to be in it, and what a task added there starts with.
struct List {
    id: &'static str,
    label: &'static str,
    icon: Icon,
}

const LISTS: [List; 6] = [
    List {
        id: "inbox",
        label: "Inbox",
        icon: Icon::Inbox,
    },
    List {
        id: "today",
        label: "Today",
        icon: Icon::Sun,
    },
    List {
        id: "upcoming",
        label: "Upcoming",
        icon: Icon::CalendarDays,
    },
    List {
        id: "harbor",
        label: "harbor",
        icon: Icon::Folder,
    },
    List {
        id: "juniper",
        label: "juniper",
        icon: Icon::Folder,
    },
    List {
        id: "larkspur",
        label: "larkspur",
        icon: Icon::Folder,
    },
];

fn holds(list: &str, task: &Task, today: Day) -> bool {
    match list {
        "inbox" => true,
        "today" => task.due().is_some_and(|due| due <= today),
        "upcoming" => task.due().is_some_and(|due| due > today),
        project => task.project == project,
    }
}

/// What a task added to `list` starts with: its due day and its project.
fn adds(list: &str, today: Day, project: &str) -> (Option<Day>, String) {
    match list {
        "inbox" => (None, project.into()),
        "today" => (Some(today), project.into()),
        "upcoming" => (Some(today.plus(1)), project.into()),
        own => (None, own.into()),
    }
}

/// The rank that puts a task between the two it lands between.
fn rank_between(before: Option<&Task>, after: Option<&Task>) -> f64 {
    match (before, after) {
        (Some(b), Some(a)) => (b.rank + a.rank) / 2.0,
        (Some(b), None) => b.rank + 1.0,
        (None, Some(a)) => a.rank - 1.0,
        (None, None) => 0.0,
    }
}

/// Open tasks first, by the day they are due, those with none after.
fn by_due(a: &Task, b: &Task) -> std::cmp::Ordering {
    a.done()
        .cmp(&b.done())
        .then(a.due.unwrap_or(i32::MAX).cmp(&b.due.unwrap_or(i32::MAX)))
        .then(a.title.cmp(&b.title))
}

/// The person's own order.
fn by_rank(a: &Task, b: &Task) -> std::cmp::Ordering {
    a.rank.total_cmp(&b.rank)
}

/// The board's columns: their title and the icon and ink of each state.
/// A state's column id, as the web example names its columns.
fn state_id(state: TaskState) -> &'static str {
    match state {
        TaskState::Todo => "todo",
        TaskState::Doing => "doing",
        TaskState::Done => "done",
    }
}

fn column_of(state: TaskState, t: &sokkel_egui::Tokens) -> (&'static str, Icon, Color32) {
    match state {
        TaskState::Todo => ("To do", Icon::Circle, t.color("muted-foreground")),
        TaskState::Doing => ("Doing", Icon::CircleDot, t.color("status-running")),
        TaskState::Done => ("Done", Icon::CircleCheck, t.color("status-succeeded")),
    }
}

/// Tidevann's mark, a drop over a wave, as `examples/shared/TidevannMark.tsx` draws it.
pub const TIDEVANN_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="64" height="64" fill="currentColor"><path d="M12 4c-1 1.8-3 3.5-3 5a3 3 0 0 0 6 0c0-1.5-2-3.2-3-5Z"/><path d="M5 13c2-2 4 2 7 0s5-2 7 0v1a7 7 0 0 1-14 0Z"/></svg>"#;

/// What the app keeps between runs, as the web example's simulated backend keeps its rows.
#[derive(serde::Serialize, serde::Deserialize)]
struct Kept {
    appearance: Appearance,
    tasks: Vec<Task>,
    prefs: Prefs,
    next: usize,
}

pub struct TasksApp {
    pub appearance: Appearance,
    /// The frame stands in a desktop window with its title bar hidden: set by the running app, never
    /// in its screenshots, which draw the frame as the browser does.
    pub desktop: bool,
    /// The day the app counts from: the clock's, or a fixed one for its screenshots.
    pub today: Day,
    pub tasks: Vec<Task>,
    pub prefs: Prefs,
    /// The list shown.
    pub list: String,
    /// The settings over the page, with the section shown.
    pub settings: Option<String>,
    /// The question before the demo data is put back, open over the settings.
    pub resetting: bool,
    palette: Option<(String, usize)>,
    quick: String,
    /// The list's table: its active cell and open editor.
    pub table: TableState,
    next: usize,
}

impl TasksApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::on(Day::today(), Appearance::default());
        if let Some(kept) = cc.storage.and_then(|s| eframe::get_value::<Kept>(s, KEPT)) {
            app.appearance = kept.appearance;
            app.tasks = kept.tasks;
            app.prefs = kept.prefs;
            app.next = kept.next;
            app.list = app.prefs.start_list.clone();
        }
        app.desktop = true;
        app
    }

    /// A fresh app on `today`, with the seed.
    pub fn on(today: Day, appearance: Appearance) -> Self {
        let prefs = Prefs::default();
        Self {
            desktop: false,
            appearance,
            today,
            tasks: seed(today),
            list: prefs.start_list.clone(),
            prefs,
            settings: None,
            resetting: false,
            palette: None,
            quick: String::new(),
            table: TableState::default(),
            next: 15,
        }
    }

    /// Go to a list, or open the settings over the list shown.
    pub fn go(&mut self, place: &str) {
        if place == "settings" {
            self.settings = Some("lists".into());
        } else if LISTS.iter().any(|list| list.id == place) {
            self.list = place.into();
        }
    }

    fn list(&self) -> &'static List {
        LISTS.iter().find(|list| list.id == self.list).unwrap_or(&LISTS[0])
    }

    fn open_in(&self, list: &str) -> usize {
        self.tasks
            .iter()
            .filter(|task| !task.done() && holds(list, task, self.today))
            .count()
    }

    /// The tasks the list shows, in the order chosen.
    fn shown(&self) -> Vec<Task> {
        let mut shown: Vec<Task> = self
            .tasks
            .iter()
            .filter(|task| holds(&self.list, task, self.today) && (self.prefs.show_done || !task.done()))
            .cloned()
            .collect();
        shown.sort_by(if self.prefs.order == Order::Mine {
            by_rank
        } else {
            by_due
        });
        shown
    }

    fn change(&mut self, id: &str, change: impl FnOnce(&mut Task)) {
        if let Some(task) = self.tasks.iter_mut().find(|task| task.id == id) {
            change(task);
        }
    }

    fn add(&mut self, title: String) {
        let (due, project) = adds(&self.list, self.today, &self.prefs.project);
        let rank = self.tasks.iter().map(|task| task.rank).fold(0.0, f64::min) - 1.0;
        self.tasks.push(Task {
            id: format!("task-{}", self.next),
            title,
            due: due.map(|day| day.0),
            assignee: self.prefs.assignee.clone(),
            project,
            state: TaskState::Todo,
            rank,
        });
        self.next += 1;
    }

    fn groups(&self) -> Vec<RailGroup> {
        let entry = |list: &List| RailEntry::new(list.id, list.label, list.icon).count(self.open_in(list.id) as u32);
        vec![
            RailGroup::new(None, LISTS[..3].iter().map(entry)),
            RailGroup::new(Some("Projects"), LISTS[3..].iter().map(entry)),
            RailGroup::new(Some("System"), [RailEntry::new("settings", "Settings", Icon::Settings)]),
        ]
    }

    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        if palette_shortcut(ui.ctx()) {
            self.palette = Some((String::new(), 0));
        }
        let groups = self.groups();
        let mut place = if self.settings.is_some() {
            "settings".to_owned()
        } else {
            self.list.clone()
        };
        let label = self.list().label;
        let mut searched = false;
        let mut account = None;
        let all_open = self.tasks.iter().filter(|task| !task.done()).count();
        let shell = if self.desktop {
            Shell::new().desktop(DesktopWindow::native())
        } else {
            Shell::new()
        };
        shell.show_window(ui, |part, ui| match part {
            ShellPart::Rail => {
                ui.add(Rail::new("Tasks", &groups, &mut place).mark(TIDEVANN_MARK));
            }
            ShellPart::RailFoot => {
                account = AccountMenu::new("ada")
                    .place("tidevann.example.org")
                    .appearance(true)
                    .show(ui);
            }
            ShellPart::Top => {
                searched = TopBar::new(&["Tasks", label])
                    .search("Search", "⌘K")
                    .show(ui, |_| {})
                    .search;
            }
            ShellPart::Page => self.page(ui),
            ShellPart::Foot => {
                ui.add(StatusLine::new("All changes saved").item("open", all_open));
            }
        });
        let was = if self.settings.is_some() {
            "settings"
        } else {
            self.list.as_str()
        };
        if place != was {
            let place = place.clone();
            self.go(&place);
        }
        match account {
            Some(AccountAction::Settings) => self.go("settings"),
            Some(AccountAction::Appearance) => self.settings = Some("appearance".into()),
            None => {}
        }
        if searched {
            self.palette = Some((String::new(), 0));
        }
        self.settings_dialog(ui);
        self.palette(ui);
    }

    fn page(&mut self, ui: &mut Ui) {
        let label = self.list().label;
        let open = self
            .tasks
            .iter()
            .filter(|task| !task.done() && holds(&self.list, task, self.today))
            .count();
        Page::show(ui, |ui| {
            PageHeader::new(label).count(format!("{open} open")).show(ui, |_| {});
            let mut added = None;
            let mut view = if self.prefs.view == View::List { 0 } else { 1 };
            let mut show_done = self.prefs.show_done;
            let list_view = self.prefs.view == View::List;
            toolbar(
                ui,
                |ui| {
                    added = QuickAdd::new("quick-add", &mut self.quick)
                        .placeholder("Add a task")
                        .show(ui)
                },
                |ui| {
                    ui.add(Segmented::new(&mut view, ["List", "Board"]).variant(SegmentedVariant::Track));
                    if list_view {
                        ui.add(SwitchField::new(&mut show_done, "Show done"));
                    }
                },
            );
            self.prefs.view = if view == 0 { View::List } else { View::Board };
            self.prefs.show_done = show_done;
            if let Some(title) = added {
                self.add(title);
            }
            match self.prefs.view {
                View::List => self.rows(ui),
                View::Board => self.board(ui),
            }
        });
    }

    fn rows(&mut self, ui: &mut Ui) {
        let shown = self.shown();
        let now = self.today;
        // what an empty list says: how many done tasks it hides, and a way to show them
        let list = self.list();
        let in_list = self.tasks.iter().filter(|task| holds(&self.list, task, now)).count();
        let hidden_done = in_list - self.open_in(&self.list);
        let mut show_done = false;
        let week_start = match self.prefs.week_start {
            WeekStart::Monday => sokkel_egui::WeekStart::Monday,
            WeekStart::Sunday => sokkel_egui::WeekStart::Sunday,
        };
        let columns = [
            Column::new("Done", 48.0).sortable(false).stack(StackPlace::Lead),
            Column::new("Title", 224.0)
                .fill()
                .sortable(false)
                .editable(Editor::Text)
                .validate(|title| title.trim().is_empty().then(|| "A task needs a title.".to_owned())),
            Column::new("Due", 152.0)
                .sortable(false)
                .editable(Editor::Date(week_start)),
            Column::new("Assignee", 136.0)
                .sortable(false)
                .editable(Editor::Select(PEOPLE.map(String::from).to_vec())),
            Column::new("Project", 112.0)
                .sortable(false)
                .editable(Editor::Select(PROJECTS.map(String::from).to_vec())),
        ];
        let rows: Vec<Vec<Cell>> = shown
            .iter()
            .map(|task| {
                vec![
                    Cell::Check(task.done()),
                    if task.done() {
                        Cell::Struck(task.title.clone())
                    } else {
                        Cell::Text(task.title.clone())
                    },
                    Cell::Due {
                        date: task.due(),
                        now,
                        done: task.done(),
                    },
                    Cell::Person(task.assignee.clone()),
                    Cell::Text(task.project.clone()),
                ]
            })
            .collect();
        let done: Vec<bool> = shown.iter().map(Task::done).collect();
        let output = DataTable::new(("tasks", &self.list), &columns, &rows, &mut self.table)
            .variant(TableVariant::List)
            .reorder(self.prefs.order == Order::Mine)
            .actions(|row| {
                vec![
                    MenuItem::action(if done[row] { "Reopen" } else { "Mark done" }).icon(if done[row] {
                        Icon::Undo2
                    } else {
                        Icon::Check
                    }),
                    MenuItem::Separator,
                    MenuItem::action("Delete").icon(Icon::Trash2).destructive(),
                ]
            })
            .empty(|ui| {
                let description = match hidden_done {
                    0 => "Add one above, and it lands here.".to_owned(),
                    1 => "1 done task is hidden. Add one above, or show the done ones.".to_owned(),
                    n => format!("{n} done tasks are hidden. Add one above, or show the done ones."),
                };
                Empty::new(format!("Nothing in {}", list.label))
                    .icon(list.icon)
                    .description(description)
                    .show(ui, |ui| {
                        if hidden_done > 0 {
                            let button = sokkel_egui::Button::new("Show done")
                                .variant(sokkel_egui::ButtonVariant::Outline)
                                .size(sokkel_egui::ButtonSize::Sm);
                            show_done = ui.add(button).clicked();
                        }
                    });
            })
            .keys(shown.iter().map(|task| &task.id))
            .stack_layout(StackLayout::Compact)
            .footer(false)
            .stretch(true)
            .today(now)
            .show(ui);
        if show_done {
            self.prefs.show_done = true;
        }
        if let Some(edit) = output.edited {
            let id = shown[edit.row].id.clone();
            let value = edit.value;
            self.change(&id, |task| match edit.column {
                1 => task.title = value.trim().to_owned(),
                2 => task.due = Day::parse(&value).map(|day| day.0),
                3 => task.assignee = value,
                _ => task.project = value,
            });
        }
        if let Some(row) = output.toggled {
            let id = shown[row].id.clone();
            self.change(&id, |task| {
                task.state = if task.done() { TaskState::Todo } else { TaskState::Done }
            });
        }
        match output.action {
            Some((row, 0)) => {
                let id = shown[row].id.clone();
                self.change(&id, |task| {
                    task.state = if task.done() { TaskState::Todo } else { TaskState::Done }
                });
            }
            Some((row, 2)) => {
                let id = shown[row].id.clone();
                self.tasks.retain(|task| task.id != id);
            }
            _ => {}
        }
        if let Some((from, to)) = output.reordered {
            let mut moved = shown.clone();
            let task = moved.remove(from);
            moved.insert(to, task);
            let rank = rank_between(to.checked_sub(1).and_then(|i| moved.get(i)), moved.get(to + 1));
            let id = moved[to].id.clone();
            self.change(&id, |task| task.rank = rank);
        }
    }

    fn board(&mut self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let height = (ui.available_height() - 40.0).max(200.0);
        let now = self.today;
        let in_list: Vec<Task> = self
            .tasks
            .iter()
            .filter(|task| holds(&self.list, task, self.today))
            .cloned()
            .collect();
        let mut chosen: Option<(String, usize)> = None;
        let moved = Board::new(("board", &self.list)).show(ui, |ui| {
            for state in STATES {
                let mut held: Vec<&Task> = in_list.iter().filter(|task| task.state == state).collect();
                held.sort_by(|a, b| by_rank(a, b));
                let (title, icon, ink) = column_of(state, &t);
                BoardColumn::new(title, held.len())
                    .id(state_id(state))
                    .icon(icon, ink)
                    .show(ui, height, |ui| {
                        for task in &held {
                            // the card's menu: the columns it can move to, then delete
                            let mut menu = vec![MenuItem::Label("Move to".into())];
                            for other in STATES {
                                let (title, icon, ink) = column_of(other, &t);
                                menu.push(
                                    MenuItem::action(title)
                                        .icon(icon)
                                        .tint(ink)
                                        .disabled(other == task.state),
                                );
                            }
                            menu.push(MenuItem::Separator);
                            menu.push(MenuItem::action("Delete").icon(Icon::Trash2).destructive());
                            let card = BoardCard::new(&task.title)
                                .id(&task.id)
                                .assignee(&task.assignee)
                                .due(task.due(), now)
                                .done(task.done())
                                .menu(menu)
                                .show(ui);
                            if let Some(index) = card.chosen {
                                chosen = Some((task.id.clone(), index));
                            }
                        }
                    });
            }
        });
        // a card dropped in a column: its state, and a rank between the cards it lands beside
        if let Some(BoardMove { card, to, index }) = moved
            && let Some(state) = STATES.into_iter().find(|s| state_id(*s) == to)
        {
            let mut others: Vec<Task> = in_list
                .iter()
                .filter(|task| task.state == state && task.id != card)
                .cloned()
                .collect();
            others.sort_by(by_rank);
            let rank = rank_between(index.checked_sub(1).and_then(|i| others.get(i)), others.get(index));
            self.change(&card, |task| {
                task.state = state;
                task.rank = rank;
            });
        }
        match chosen {
            // Move to: the end of that column
            Some((id, index @ 1..=3)) => {
                let state = STATES[index - 1];
                let mut others: Vec<Task> = in_list.iter().filter(|task| task.state == state).cloned().collect();
                others.sort_by(by_rank);
                let rank = rank_between(others.last(), None);
                self.change(&id, |task| {
                    task.state = state;
                    task.rank = rank;
                });
            }
            Some((id, 5)) => self.tasks.retain(|task| task.id != id),
            _ => {}
        }
    }

    fn settings_dialog(&mut self, ui: &mut Ui) {
        let Some(mut active) = self.settings.take() else { return };
        let sections = [
            SettingsSection::new("lists", "Lists", Icon::ListTodo).group("Preferences"),
            SettingsSection::new("adding", "New tasks", Icon::Plus).group("Preferences"),
            SettingsSection::new("appearance", "Appearance", Icon::Palette).group("Preferences"),
            SettingsSection::new("demo", "Demo data", Icon::Database)
                .group("This demo")
                .reset(false),
        ];
        let (prefs, appearance, today) = (&mut self.prefs, &mut self.appearance, self.today);
        let mut reset_data = false;
        let outcome = SettingsDialog::new("settings", &sections, &mut active).show(ui.ctx(), |section, ui| match section {
            "lists" => {
                setting_row(ui, "Open on", "The list shown when the app opens, and when the mark is pressed.", |ui| {
                    let mut chosen = LISTS.iter().position(|list| list.id == prefs.start_list).unwrap_or(0);
                    if ui.add(Select::new(&mut chosen, LISTS.iter().map(|list| list.label), "Open on").width(110.0)).changed() {
                        prefs.start_list = LISTS[chosen].id.into();
                    }
                });
                setting_row(ui, "Show lists as", "Rows, or a board with a column for each state whose cards are dragged between them.", |ui| {
                    let mut chosen = usize::from(prefs.view == View::Board);
                    if ui.add(Select::new(&mut chosen, ["List", "Board"], "Show lists as").width(82.0)).changed() {
                        prefs.view = if chosen == 0 { View::List } else { View::Board };
                    }
                });
                setting_row(ui, "Order", "Your order is set by dragging a row by its grip; due date puts the soonest first.", |ui| {
                    let mut chosen = usize::from(prefs.order == Order::Due);
                    if ui.add(Select::new(&mut chosen, ["Your order", "Due date"], "Order").width(112.0)).changed() {
                        prefs.order = if chosen == 0 { Order::Mine } else { Order::Due };
                    }
                });
                setting_row(ui, "Weeks start on", "The first day of each week in the calendar a due date is picked from.", |ui| {
                    let mut chosen = usize::from(prefs.week_start == WeekStart::Sunday);
                    if ui.add(Select::new(&mut chosen, ["Monday", "Sunday"], "Weeks start on").width(95.0)).changed() {
                        prefs.week_start = if chosen == 0 { WeekStart::Monday } else { WeekStart::Sunday };
                    }
                });
                setting_row(
                    ui,
                    "Show done tasks",
                    "Lists keep the tasks already done, struck through; ordered by due date, they come after the open ones.",
                    |ui| {
                        ui.add(Switch::new(&mut prefs.show_done, "Show done tasks"));
                    },
                );
            }
            "adding" => {
                setting_row(ui, "Assign to", "Who a task added from the quick add is given to.", |ui| {
                    let mut chosen = PEOPLE.iter().position(|p| *p == prefs.assignee).unwrap_or(0);
                    if ui.add(Select::new(&mut chosen, PEOPLE, "Assign to").width(110.0)).changed() {
                        prefs.assignee = PEOPLE[chosen].into();
                    }
                });
                setting_row(ui, "Project", "The project a task added outside a project's list goes into.", |ui| {
                    let mut chosen = PROJECTS.iter().position(|p| *p == prefs.project).unwrap_or(0);
                    if ui.add(Select::new(&mut chosen, PROJECTS, "Project").width(110.0)).changed() {
                        prefs.project = PROJECTS[chosen].into();
                    }
                });
            }
            "appearance" => theme_settings(ui, appearance, true, &ThemeWords::default()),
            _ => {
                setting_row(ui, "Reset demo data", "Puts back the tasks and these settings, as the demo started.", |ui| {
                    let reset = sokkel_egui::Button::new("Reset")
                        .size(sokkel_egui::ButtonSize::Sm)
                        .variant(sokkel_egui::ButtonVariant::Outline);
                    if ui.add(reset).clicked() {
                        reset_data = true;
                    }
                });
            }
        });
        if reset_data {
            self.resetting = true;
        }
        if self.resetting {
            let question = ConfirmDialog::new("reset-demo", "Reset the demo data?", "Reset")
                .detail("Tasks added, changed or deleted since are put back, and the page loads again.")
                .destructive(true);
            match question.show(ui.ctx()) {
                Some(ConfirmAnswer::Confirm) => {
                    self.resetting = false;
                    self.tasks = seed(today);
                    self.prefs = Prefs::default();
                    self.appearance = Appearance::default();
                    self.next = 15;
                }
                // the reset is never busy, so letting it go is leaving it
                Some(ConfirmAnswer::Cancel | ConfirmAnswer::StopWaiting) => self.resetting = false,
                None => {}
            }
        }
        if outcome.reset {
            match active.as_str() {
                "lists" => {
                    let defaults = Prefs::default();
                    self.prefs = Prefs {
                        assignee: self.prefs.assignee.clone(),
                        project: self.prefs.project.clone(),
                        ..defaults
                    };
                }
                "adding" => {
                    let defaults = Prefs::default();
                    self.prefs.assignee = defaults.assignee;
                    self.prefs.project = defaults.project;
                }
                "appearance" => self.appearance = Appearance::default(),
                _ => {}
            }
        }
        if !outcome.close {
            self.settings = Some(active);
        }
    }

    fn palette(&mut self, ui: &mut Ui) {
        let Some((mut query, mut highlight)) = self.palette.take() else {
            return;
        };
        // as the web builds it: its own group, the rail's places (the lists, then the rest), the look,
        // and putting the demo data back
        let rail = self.groups();
        let mut groups = vec![CommandGroup::new(
            "Tasks",
            [MenuItem::action("New task").icon(Icon::Plus).id("task-new")],
        )];
        groups.extend(place_commands(&rail[..2], "Lists"));
        groups.extend(place_commands(&rail[2..], "Go to"));
        let themes: Vec<_> = sokkel_egui::themes().filter(|theme| theme.name == "tidevann").collect();
        groups.extend(appearance_commands(
            &self.appearance,
            &themes,
            &AppearanceCommandWords::default(),
        ));
        groups.push(CommandGroup::new(
            "Demo",
            [MenuItem::action("Reset demo data")
                .icon(Icon::RotateCcw)
                .id("demo-reset")],
        ));
        let outcome = CommandPalette::new("palette", &groups, &mut query, &mut highlight).show(ui.ctx());
        let command = outcome
            .run
            .and_then(|(g, i)| groups[g].items[i].command_id().map(str::to_owned));
        match command.as_deref() {
            Some("task-new") => ui
                .ctx()
                .memory_mut(|m| m.request_focus(QuickAdd::field_id("quick-add"))),
            Some("demo-reset") => {
                self.settings = Some("demo".into());
                self.resetting = true;
            }
            Some(command) => {
                if let Some(place) = place_of(command) {
                    self.go(place);
                } else {
                    run_appearance_command(&mut self.appearance, command);
                }
            }
            None => {}
        }
        if !outcome.close {
            self.palette = Some((query, highlight));
        }
    }
}

impl eframe::App for TasksApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.show(ui));
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let kept = Kept {
            appearance: self.appearance.clone(),
            tasks: self.tasks.clone(),
            prefs: self.prefs.clone(),
            next: self.next,
        };
        eframe::set_value(storage, KEPT, &kept);
    }
}
