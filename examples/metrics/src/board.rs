//! A dashboard's own actions, as the web's `Dashboard.tsx` and `DashboardDialog`: its title edited
//! in place, its details, a copy of it, deleting it, and arranging its panels in a draft that Done
//! keeps and Cancel throws away.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, ConfirmAnswer, ConfirmDialog, DashboardEditActions, DashboardWidget, Dialog,
    DropdownMenu, Field, Gap, Icon, IconButton, MenuItem, Select, Stack, TextField, Toast, ToastKind,
};

use crate::MetricsApp;
use crate::app::DASHBOARDS;
use crate::model::{Dashboard, kind_icon, seed_dashboards, widget_catalog};

/// A question the board is asking over the page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asking {
    /// Delete a dashboard, by its id.
    DeleteBoard(String),
}

/// A dashboard being made, or its title and description being changed, as the web's
/// `DashboardDialog`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DetailsForm {
    pub title: String,
    pub description: String,
    /// The dashboard whose details change, by its id; a new one is made without it.
    pub editing: Option<String>,
    /// What a new one starts from: 0 empty, else a copy of the dashboard at this place less one.
    pub from: usize,
}

impl MetricsApp {
    /// The board shown, by its place.
    fn board_index(&self) -> Option<usize> {
        self.dashboards.iter().position(|b| b.id == self.place)
    }

    /// A panel id no panel has, as the web's `panelId`.
    pub(crate) fn panel_id(&mut self) -> String {
        self.made += 1;
        format!("panel-{}", self.made)
    }

    /// The head's actions, drawn right to left: Edit with the More menu before it, or while
    /// editing Reset, Cancel and Done.
    pub(crate) fn board_actions(&mut self, ui: &mut Ui, board: &Dashboard) {
        let Some(index) = self.board_index() else {
            return;
        };
        // the seeded panels in their places and widths, each keeping what it was edited to read
        let seeded: Vec<_> = seed_dashboards()
            .into_iter()
            .find(|b| b.id == board.id)
            .map(|b| {
                b.panels
                    .into_iter()
                    .map(|p| {
                        let mut kept = board
                            .panels
                            .iter()
                            .find(|one| one.id == p.id)
                            .cloned()
                            .unwrap_or(p.clone());
                        kept.span = p.span;
                        kept
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut more = None;
        let mut add = false;
        let (edit, kept) = (&mut self.edit, &mut self.dashboards[index].panels);
        let mut actions = DashboardEditActions::new(edit, kept).primary(|ui| {
            add = ui
                .add(Button::new("Add panel").icon(Icon::Plus).size(ButtonSize::Sm))
                .clicked();
        });
        if !seeded.is_empty() {
            actions = actions.default_layout(&seeded);
        }
        let outcome = actions.show_with(ui, |ui| {
            let items = [
                MenuItem::action("Edit details").icon(Icon::PencilLine),
                MenuItem::action("Duplicate dashboard").icon(Icon::CopyPlus),
                MenuItem::Separator,
                MenuItem::action("Delete dashboard").icon(Icon::Trash2).destructive(),
            ];
            let trigger = ui.add(IconButton::new(Icon::Ellipsis, "More").outline());
            more = DropdownMenu::new(&items).end(true).width(208.0).show(&trigger);
        });
        let _ = outcome;
        if add {
            self.open_editor(None);
        }
        match more {
            Some(0) => {
                self.details = Some(DetailsForm {
                    title: board.title.clone(),
                    description: board.description.clone(),
                    editing: Some(board.id.clone()),
                    from: 0,
                })
            }
            Some(1) => self.duplicate_board(index),
            Some(3) => self.asking = Some(Asking::DeleteBoard(board.id.clone())),
            _ => {}
        }
    }

    /// A copy of a dashboard with new panel ids, shown at once, as the web's `duplicateBoard`.
    fn duplicate_board(&mut self, index: usize) {
        let mut made = self.dashboards[index].clone();
        made.title = format!("{} copy", made.title);
        made.id = self.board_id(&made.title);
        for panel in &mut made.panels {
            panel.id = self.panel_id();
        }
        self.said.push(Toast::success(format!("{} made", made.title)));
        self.place = made.id.clone();
        self.dashboards.push(made);
    }

    /// An id from a title no dashboard has yet.
    fn board_id(&self, title: &str) -> String {
        let base: String = title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let base = base.trim_matches('-').to_owned();
        let mut id = base.clone();
        let mut n = 1;
        while self.dashboards.iter().any(|b| b.id == id) {
            n += 1;
            id = format!("{base}-{n}");
        }
        id
    }

    /// The palette of panels to add while editing: the catalog's, less those the board has by title.
    pub(crate) fn palette(&self, board: &Dashboard) -> Vec<DashboardWidget> {
        widget_catalog()
            .into_iter()
            .filter(|p| !board.panels.iter().any(|one| one.title == p.title))
            .map(|p| {
                DashboardWidget::new(p.id.clone(), p.title.clone())
                    .icon(kind_icon(p.kind))
                    .span(p.span)
            })
            .collect()
    }

    /// The board's dialogs over the page, and what it has said as toasts.
    pub(crate) fn board_dialogs(&mut self, ctx: &egui::Context) {
        for toast in self.said.drain(..) {
            toast.show(ctx);
        }
        if let Some(mut form) = self.details.take() {
            let missing = form.title.trim().is_empty();
            let new = form.editing.is_none();
            let mut answer = None;
            let starts: Vec<String> = std::iter::once("Empty".to_owned())
                .chain(self.dashboards.iter().map(|b| format!("A copy of {}", b.title)))
                .collect();
            let close = Dialog::new(
                "dashboard-details",
                if new { "New dashboard" } else { "Dashboard details" },
            )
            .show(
                ctx,
                |ui| {
                    Stack::new(Gap::Md).show(ui, |ui| {
                        ui.add(TextField::new("Title", &mut form.title));
                        ui.add(TextField::new("Description", &mut form.description).multiline(true));
                        if new {
                            Field::new("Start from")
                                .description("An empty dashboard, or a copy of one's panels.")
                                .show(ui, |ui| {
                                    let width = ui.available_width();
                                    ui.add(
                                        Select::new(&mut form.from, starts.iter().map(String::as_str), "Start from")
                                            .width(width),
                                    );
                                });
                        }
                    });
                },
                |ui| {
                    let mut save = Button::new(if new { "Create" } else { "Save" });
                    if missing {
                        save = save.reason("A dashboard needs a title.");
                    }
                    if ui.add(save).clicked() && !missing {
                        answer = Some(true);
                    }
                    if ui.add(Button::new("Cancel").variant(ButtonVariant::Ghost)).clicked() {
                        answer = Some(false);
                    }
                },
            );
            match answer.or(close.then_some(false)) {
                Some(true) => self.keep_details(form),
                Some(false) => {}
                None => self.details = Some(form),
            }
        }
        if let Some(Asking::DeleteBoard(id)) = self.asking.clone() {
            let Some(index) = self.dashboards.iter().position(|b| b.id == id) else {
                self.asking = None;
                return;
            };
            let board = &self.dashboards[index];
            let count = board.panels.len();
            let dialog = ConfirmDialog::new("delete-dashboard", format!("Delete {}?", board.title), "Delete")
                .detail(format!(
                    "Its {count} {} with it.",
                    if count == 1 { "panel goes" } else { "panels go" }
                ))
                .destructive(true);
            match dialog.show(ctx) {
                Some(ConfirmAnswer::Confirm) => {
                    self.asking = None;
                    let gone = self.dashboards.remove(index);
                    if self.place == gone.id {
                        self.edit.cancel();
                        self.place = DASHBOARDS.into();
                    }
                    Toast::new(format!("{} deleted", gone.title))
                        .kind(ToastKind::Success)
                        .show(ctx);
                }
                Some(_) => self.asking = None,
                None => {}
            }
        }
    }
}

impl MetricsApp {
    /// Keeps the details asked for: a dashboard's title and description changed, or a new one
    /// made, empty or with a copy of another's panels, and shown at once.
    fn keep_details(&mut self, form: DetailsForm) {
        let (title, description) = (form.title.trim().to_owned(), form.description.trim().to_owned());
        if let Some(id) = &form.editing {
            if let Some(board) = self.dashboards.iter_mut().find(|b| &b.id == id) {
                board.title = title;
                board.description = description;
            }
            return;
        }
        let copied = form
            .from
            .checked_sub(1)
            .and_then(|at| self.dashboards.get(at))
            .map(|b| b.panels.clone())
            .unwrap_or_default();
        let mut panels = Vec::new();
        for mut panel in copied {
            panel.id = self.panel_id();
            panels.push(panel);
        }
        let made = Dashboard {
            id: self.board_id(&title),
            title,
            description,
            panels,
        };
        self.said.push(Toast::success(format!("{} made", made.title)));
        self.place = made.id.clone();
        self.dashboards.push(made);
    }
}
