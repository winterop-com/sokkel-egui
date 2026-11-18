//! A source beside what it makes, after sokkel-ui's `EditPreview` in its `pane` variant.

use egui::Ui;

use crate::{SplitPane, tokens};

/// Which panes stand: the editor beside its preview, or one of them alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EditPreviewView {
    Edit,
    #[default]
    Both,
    Preview,
}

/// A source and what it makes, as an editor beside its live preview, filling a pane of the app's
/// as the web's `variant="pane"`: the two edge to edge with no titles, the rule between them
/// dragged to share the room (kept under its id), stacked narrower than 640, the editor over the
/// preview. The editor fills its pane; the preview is padded by `panel-pad` and scrolls. Its
/// height is what is left, never under 320.
pub struct EditPreview {
    id: egui::Id,
    view: EditPreviewView,
    flush_preview: bool,
}

impl EditPreview {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id: egui::Id::unique(("sokkel-edit-preview", id)),
            view: EditPreviewView::Both,
            flush_preview: false,
        }
    }

    /// The editor or the preview alone, at every width, for a reader who turns the other off.
    pub fn view(mut self, view: EditPreviewView) -> Self {
        self.view = view;
        self
    }

    /// The preview edge to edge too, as a frame or a canvas is.
    pub fn flush_preview(mut self, flush: bool) -> Self {
        self.flush_preview = flush;
        self
    }

    pub fn show(self, ui: &mut Ui, edit: impl FnOnce(&mut Ui), preview: impl FnOnce(&mut Ui)) {
        let t = tokens(ui.ctx());
        let pad = if self.flush_preview {
            0
        } else {
            t.density("panel-pad") as i8
        };
        let height = ui.available_height().max(320.0);
        let width = ui.available_width();
        let id = self.id;
        let only = match self.view {
            EditPreviewView::Both => None,
            EditPreviewView::Edit => Some(0),
            EditPreviewView::Preview => Some(1),
        };
        ui.allocate_ui(egui::vec2(width, height), |ui| {
            ui.set_min_size(egui::vec2(width, height));
            SplitPane::new(id).only(only).show(ui, edit, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(id.with("preview"))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Frame::new().inner_margin(egui::Margin::same(pad)).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            preview(ui);
                        });
                    });
            });
        });
    }
}
