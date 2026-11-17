//! A dashboard's own actions, as the web's `Dashboard.tsx` and `DashboardDialog`: its title edited
//! in place, its details, a copy of it, deleting it, and arranging its panels in a draft that Done
//! keeps and Cancel throws away.

use eframe::egui::{self, Ui};
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, ConfirmAnswer, ConfirmDialog, DashboardEditActions, DashboardWidget, Dialog,
    DropdownMenu, Gap, Icon, IconButton, MenuItem, Stack, TextField, Toast, ToastKind,
};

use crate::MetricsApp;
use crate::model::{Dashboard, kind_icon, seed_dashboards, widget_catalog};

/// A question the board is asking over the page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asking {
    /// Delete the dashboard shown.
    DeleteBoard,
}

/// A dashboard's title and description being changed, as the web's `DashboardDialog` with `editing`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetailsForm {
    pub title: String,
    pub description: String,
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
                })
            }
            Some(1) => self.duplicate_board(index),
            Some(3) => self.asking = Some(Asking::DeleteBoard),
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
        let Some(index) = self.board_index() else {
            return;
        };
        if let Some(mut form) = self.details.take() {
            let missing = form.title.trim().is_empty();
            let mut answer = None;
            let close = Dialog::new("dashboard-details", "Dashboard details").show(
                ctx,
                |ui| {
                    Stack::new(Gap::Md).show(ui, |ui| {
                        ui.add(TextField::new("Title", &mut form.title));
                        ui.add(TextField::new("Description", &mut form.description).multiline(true));
                    });
                },
                |ui| {
                    let mut save = Button::new("Save");
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
                Some(true) => {
                    let board = &mut self.dashboards[index];
                    board.title = form.title.trim().to_owned();
                    board.description = form.description.trim().to_owned();
                }
                Some(false) => {}
                None => self.details = Some(form),
            }
        }
        if let Some(Asking::DeleteBoard) = self.asking {
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
                    self.edit.cancel();
                    Toast::new(format!("{} deleted", gone.title))
                        .kind(ToastKind::Success)
                        .show(ctx);
                    self.place = self.dashboards.first().map_or_else(String::new, |b| b.id.clone());
                }
                Some(_) => self.asking = None,
                None => {}
            }
        }
    }
}
