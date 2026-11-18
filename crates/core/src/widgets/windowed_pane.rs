//! Any pane, with a way to open the same content near full screen, after sokkel-ui's
//! `WindowedPane`.

use egui::{Align, Context, Id, Layout, Margin, Sense, Stroke, Ui, vec2};

use crate::{Icon, IconButton, SplitPane, TextRole, tokens};

/// The words a windowed pane says.
#[derive(Clone, Debug)]
pub struct WindowedPaneWords {
    /// The expand button's label.
    pub expand: String,
    /// The window's close button's label.
    pub close: String,
    /// Said in the pane's place while its content is in the window.
    pub away: String,
}

impl Default for WindowedPaneWords {
    fn default() -> Self {
        Self {
            expand: "Expand".into(),
            close: "Close".into(),
            away: "Shown in a window.".into(),
        }
    }
}

/// Where a windowed pane's content is drawn: in the pane in its place, or in the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowedPaneWhere {
    Pane,
    Window,
}

/// What the aside beside the content in the window draws.
pub type WindowedAside<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// The expand button of a windowed pane, for a pane that sets it in a toolbar of its own, as
/// sokkel-ui's `WindowedPaneButton`: true when pressed.
pub fn windowed_pane_button(ui: &mut Ui, words: &WindowedPaneWords) -> bool {
    ui.add(IconButton::new(Icon::Maximize2, &words.expand)).clicked()
}

/// Any pane, with a way to open the same content near full screen, as sokkel-ui's `WindowedPane`
/// with `button="none"`: the content places the expand button itself (`windowed_pane_button`) and
/// sets `open`. The window stands 24 in from the screen's edges over a shade, its head
/// `panel-head` tall with the title in the section title's type and a close button, its body
/// padded 16; Escape or a press on the shade closes it. `aside` stands beside the content in the
/// window only, the window split 70 to 30. While the window is open the pane says so in its
/// place, a dashed box as tall as it was, unless `keep_pane` keeps the content in both.
pub struct WindowedPane<'a> {
    id: Id,
    title: String,
    keep_pane: bool,
    aside: Option<WindowedAside<'a>>,
    words: WindowedPaneWords,
}

impl<'a> WindowedPane<'a> {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, title: impl Into<String>) -> Self {
        Self {
            id: Id::unique(("sokkel-windowed-pane", id)),
            title: title.into(),
            keep_pane: false,
            aside: None,
            words: WindowedPaneWords::default(),
        }
    }

    /// The pane keeps its content while the window draws a copy of its own.
    pub fn keep_pane(mut self, keep: bool) -> Self {
        self.keep_pane = keep;
        self
    }

    /// A part drawn beside the content in the window only, such as a reference beside an editor.
    pub fn aside(mut self, aside: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.aside = Some(Box::new(aside));
        self
    }

    pub fn words(mut self, words: WindowedPaneWords) -> Self {
        self.words = words;
        self
    }

    /// Draws the pane, and the window while `open`; `content` draws the content where it is told.
    pub fn show(self, ui: &mut Ui, open: &mut bool, mut content: impl FnMut(&mut Ui, WindowedPaneWhere)) {
        let t = tokens(ui.ctx());
        let height_id = self.id.with("height");
        if !*open || self.keep_pane {
            let top = ui.cursor().top();
            content(ui, WindowedPaneWhere::Pane);
            let height = ui.cursor().top() - top;
            ui.data_mut(|d| d.insert_temp(height_id, height));
        } else {
            // the pane's place, as tall as it was, says where its content is
            let height = ui.data(|d| d.get_temp::<f32>(height_id)).unwrap_or(96.0).max(96.0);
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
            dashed_rect(ui, rect, t.radius("surface"), Stroke::new(1.0, t.color("border")));
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                &self.words.away,
                TextRole::Body.font(&t),
                t.color("muted-foreground"),
            );
        }
        if *open && self.window(ui.ctx(), &mut content) {
            *open = false;
        }
    }

    /// The window over the page; true when it asks to close.
    fn window(self, ctx: &Context, content: &mut impl FnMut(&mut Ui, WindowedPaneWhere)) -> bool {
        let t = tokens(ctx);
        let screen = ctx.content_rect();
        let inset = if screen.width() >= 640.0 { 24.0 } else { 12.0 };
        let size = screen.size() - vec2(2.0 * inset, 2.0 * inset);
        let mut close = false;
        let response = egui::Modal::new(self.id.with("window"))
            .backdrop_color(t.color_at("shade", 0.1))
            .frame(super::dialog_frame(&t).inner_margin(Margin::ZERO))
            .show(ctx, |ui| {
                ui.set_min_size(size);
                ui.set_max_size(size);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                // the head: the title, and the close button at its end, over a rule
                let head = t.density("panel-head");
                let pad = t.density("panel-pad");
                let (bar, _) = ui.allocate_exact_size(vec2(size.x, head), Sense::hover());
                ui.painter()
                    .hline(bar.x_range(), bar.bottom() - 0.5, Stroke::new(1.0, t.color("border")));
                let mut row = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(bar.shrink2(vec2(pad, 0.0)))
                        .layout(Layout::left_to_right(Align::Center)),
                );
                row.label(TextRole::SectionTitle.rich(&t, &self.title));
                row.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    close = ui.add(IconButton::new(Icon::X, &self.words.close)).clicked();
                });
                let mut body = |ui: &mut Ui| {
                    egui::ScrollArea::both()
                        .id_salt(self.id.with("body"))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            egui::Frame::new().inner_margin(Margin::same(16)).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                content(ui, WindowedPaneWhere::Window);
                            });
                        });
                };
                match self.aside {
                    None => body(ui),
                    Some(aside) => {
                        SplitPane::new(self.id.with("split"))
                            .default_share(0.7)
                            .narrow_below(720.0)
                            .show(ui, body, |ui| {
                                let rect = ui.max_rect();
                                ui.painter()
                                    .rect_filled(rect, 0.0, t.color_or("panel-ground", t.color("muted")));
                                egui::Frame::new().inner_margin(Margin::same(pad as i8)).show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    aside(ui);
                                });
                            });
                    }
                }
            });
        close || response.should_close()
    }
}

/// A rectangle's edge drawn dashed, 4 on and 4 off.
fn dashed_rect(ui: &Ui, rect: egui::Rect, _radius: egui::CornerRadius, stroke: Stroke) {
    let points = [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
        rect.left_top(),
    ];
    ui.painter().extend(egui::Shape::dashed_line(&points, stroke, 4.0, 4.0));
}
