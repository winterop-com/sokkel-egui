//! The export dialog, as the web's `editor/ExportDialog.tsx`: the file's name and resolution, and
//! what the export makes of the edit at that size. The web draws every frame and encodes it as WebM
//! in the browser; no encoder is carried here, so the dialog says what it would make and Export
//! stays shut, saying why.

use eframe::egui;
use sokkel_egui::{DescriptionList, DialogShell, Input, Segmented, field};

use crate::editor_page::EditorView;
use crate::export::{named, summary, why_not};
use crate::model::ExportSize;

/// The `sm` dialog's width.
const SMALL: f32 = 384.0;

/// The terms' column in the summary: the web's `6rem` and the 8 of its small list.
const TERM: f32 = 104.0;

/// What the dialog says under its title, the file it would write named in it.
pub fn export_description(file: &str) -> String {
    format!(
        "Each frame of the edit drawn at the size chosen and written as {file}. This app carries no video encoder, so it makes no file."
    )
}

/// Why Export stays shut once the edit and the name would let it go.
pub const NO_ENCODER: &str = "No video encoder is carried here, so the export cannot be made.";

impl EditorView {
    /// The export dialog while it is open.
    pub fn export_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut state) = self.exporting.take() else {
            return;
        };
        let edit = self.edit().clone();
        let size = ExportSize::ALL[state.size.min(ExportSize::ALL.len() - 1)];
        let why = why_not(&edit, &state.file).unwrap_or(NO_ENCODER);
        let labels = ExportSize::ALL.map(|s| s.spec().label);
        let answer = DialogShell::new("export", "Export", "Export")
            .width(SMALL)
            .description(export_description(&named(&state.file)))
            .why(Some(why))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 24.0;
                field(ui, "File name", None, |ui| {
                    let width = ui.available_width();
                    ui.add(Input::new(&mut state.file).mono(true).name("File name").width(width));
                });
                field(ui, "Resolution", None, |ui| {
                    ui.add(Segmented::new(&mut state.size, labels));
                });
                let mut list = DescriptionList::new().side(TERM);
                for (term, value) in summary(&edit, size) {
                    list = list.text(term, value);
                }
                list.show(ui);
            });
        if answer.is_none() {
            self.exporting = Some(state);
        }
    }
}
