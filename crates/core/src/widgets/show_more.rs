//! A block of text held to a few lines, after sokkel-ui's `ShowMore`.

use egui::{Align, FontSelection, Sense, Stroke, Ui, text::LayoutJob, text::TextWrapping, vec2};

use crate::{MetaLine, TextRole, tokens};

/// The words the block says.
#[derive(Clone, Debug)]
pub struct ShowMoreWords {
    pub more: String,
    pub less: String,
}

impl Default for ShowMoreWords {
    fn default() -> Self {
        Self {
            more: "Show more".into(),
            less: "Show less".into(),
        }
    }
}

/// A block of text held to a few lines, with Show more under it to read the rest and Show less to
/// fold it again: a description, a prompt, a long note. The control shows only when the text runs
/// past its lines. `head` is a line over the text, such as a count of views and an age; `well`
/// draws it on the `muted` ground in a box with the surface's corners, 12 in, as a description
/// under a video sits. The text is `text.body` at the relaxed leading (1.625), 4 under the head.
pub struct ShowMore {
    id_salt: egui::Id,
    text: String,
    lines: usize,
    head: Option<MetaLine>,
    well: bool,
    words: ShowMoreWords,
}

impl ShowMore {
    pub fn new(id_salt: impl egui::AsIdSalt, text: impl Into<String>) -> Self {
        Self {
            id_salt: egui::Id::unique(id_salt),
            text: text.into(),
            lines: 2,
            head: None,
            well: false,
            words: ShowMoreWords::default(),
        }
    }

    /// How many lines show while it is folded; two by default.
    pub fn lines(mut self, lines: usize) -> Self {
        self.lines = lines.max(1);
        self
    }

    pub fn head(mut self, head: MetaLine) -> Self {
        self.head = Some(head);
        self
    }

    /// On the muted ground in a rounded box, as the web's `variant="well"`.
    pub fn well(mut self, well: bool) -> Self {
        self.well = well;
        self
    }

    pub fn words(mut self, words: ShowMoreWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> egui::Response {
        let t = tokens(ui.ctx());
        let id = ui.make_persistent_id(self.id_salt);
        let mut open = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
        let mut frame = egui::Frame::new();
        if self.well {
            frame = frame
                .fill(t.color("muted"))
                .corner_radius(t.radius("surface"))
                .inner_margin(egui::Margin::same(12));
        }
        let response = frame
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing = vec2(0.0, 4.0);
                if let Some(head) = self.head {
                    head.show(ui);
                }
                let spec = t.text("body");
                let width = ui.available_width();
                let layout = |rows: usize| {
                    let mut job = LayoutJob::default();
                    TextRole::Body
                        .rich(&t, &self.text)
                        .line_height(Some((spec.size * 1.625).round()))
                        .color(t.color("foreground"))
                        .append_to(&mut job, ui.style(), FontSelection::Default, Align::Min);
                    job.wrap = TextWrapping {
                        max_width: width,
                        max_rows: rows,
                        break_anywhere: false,
                        overflow_character: Some('\u{2026}'),
                    };
                    ui.fonts_mut(|f| f.layout_job(job))
                };
                let folded = layout(self.lines);
                let overflows = folded.elided;
                let galley = if open { layout(usize::MAX) } else { folded };
                let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
                ui.painter().galley(rect.min, galley, t.color("foreground"));
                if open || overflows {
                    let word = if open { &self.words.less } else { &self.words.more };
                    let rich = TextRole::Action.rich(&t, word).color(t.color("foreground"));
                    let control = ui.add(egui::Label::new(rich).sense(Sense::click()));
                    control.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, word));
                    if control.hovered() {
                        let y = control.rect.bottom() - 2.0;
                        ui.painter()
                            .hline(control.rect.x_range(), y, Stroke::new(1.0, t.color("foreground")));
                    }
                    if control.clicked() {
                        open = !open;
                    }
                }
            })
            .response;
        ui.data_mut(|d| d.insert_temp(id, open));
        response
    }
}
