//! A dashboard's grid and its panels, after sokkel-ui's `DashboardGrid` and `DashboardPanel`.

use egui::{Id, Rect, Sense, Stroke, TextWrapMode, Ui, UiBuilder, WidgetText, pos2, vec2};

use crate::{TextRole, tokens};

/// How much of a dashboard's row a panel takes, of its twelve columns, when the grid is wide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelSpan {
    Sixth,
    Quarter,
    Third,
    Half,
    TwoThirds,
    Full,
}

impl PanelSpan {
    /// The columns this span takes in a grid `width` wide: as named from 896 (`@4xl`); below it a
    /// sixth takes a third (from 672, `@2xl`) or half, a quarter half, the rest the row.
    pub fn columns(self, width: f32) -> usize {
        let wide = width >= 896.0;
        match self {
            Self::Sixth if wide => 2,
            Self::Sixth if width >= 672.0 => 4,
            Self::Sixth | Self::Quarter if !wide => 6,
            Self::Quarter => 3,
            Self::Third if wide => 4,
            Self::Half if wide => 6,
            Self::TwoThirds if wide => 8,
            _ => 12,
        }
    }
}

/// The height a grid's row holds its cells to, read by a panel so it stretches to its row.
#[derive(Clone, Copy)]
struct CellHeight(f32);

/// The height a panel would take unstretched, which the grid sizes its row by.
#[derive(Clone, Copy, Default)]
struct CellNatural(f32);

/// The height a panel drawn in `ui` stretches to: its grid row's, where the grid has measured it.
pub fn cell_height(ui: &Ui) -> Option<f32> {
    ui.data(|d| d.get_temp::<CellHeight>(ui.id())).map(|h| h.0)
}

/// Tells the grid whose cell `cell` is (the id of the `Ui` the panel was given) how tall the panel in it is unstretched, so a row shrinks
/// again when its tallest panel does.
pub fn report_natural_height(ui: &Ui, cell: Id, height: f32) {
    ui.data_mut(|d| d.insert_temp(cell, CellNatural(height)));
}

/// Controls drawn at a panel's head.
type Tools<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

type Cell<'a> = (PanelSpan, Box<dyn FnOnce(&mut Ui) + 'a>);

/// The panels a grid lays out, each with its span.
pub struct GridCells<'a> {
    cells: Vec<Cell<'a>>,
}

impl<'a> GridCells<'a> {
    /// A cell `span` wide; `add` draws in it, a `DashboardPanel` or a chart panel filling it.
    pub fn cell(&mut self, span: PanelSpan, add: impl FnOnce(&mut Ui) + 'a) {
        self.cells.push((span, Box::new(add)));
    }
}

/// The grid a dashboard's panels sit in: twelve columns, a panel's span following the grid's own
/// width, panels in a row stretched to the same height, `section-gap` apart (6 when dense).
pub struct DashboardGrid {
    id: Id,
    dense: bool,
}

impl DashboardGrid {
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id: Id::new(("sokkel-dashboard", id_salt)),
            dense: false,
        }
    }

    pub fn dense(mut self, dense: bool) -> Self {
        self.dense = dense;
        self
    }

    pub fn show<'a>(self, ui: &mut Ui, add: impl FnOnce(&mut GridCells<'a>)) {
        let t = tokens(ui.ctx());
        let gap = if self.dense { 6.0 } else { t.density("section-gap") };
        let mut cells = GridCells { cells: Vec::new() };
        add(&mut cells);
        let width = ui.available_width();
        let column = (width - 11.0 * gap) / 12.0;
        // rows of cells whose columns fill at most twelve
        let mut rows: Vec<Vec<Cell<'a>>> = Vec::new();
        let mut used = 12;
        for cell in cells.cells {
            let columns = cell.0.columns(width);
            if used + columns > 12 {
                rows.push(Vec::new());
                used = 0;
            }
            used += columns;
            rows.last_mut().expect("a row was pushed").push(cell);
        }
        let count = rows.len();
        // the rows are `gap` apart and no more, whatever spacing the page holds its parts at
        let spacing = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 0.0;
        for (r, row) in rows.into_iter().enumerate() {
            let key = self.id.with(("row", r));
            let held = ui.data(|d| d.get_temp::<f32>(key)).unwrap_or(0.0);
            let top = ui.cursor().top();
            let mut x = ui.cursor().left();
            let mut tallest = 0.0_f32;
            for (i, (span, draw)) in row.into_iter().enumerate() {
                let columns = span.columns(width) as f32;
                let w = column * columns + gap * (columns - 1.0);
                let rect = Rect::from_min_size(pos2(x, top), vec2(w, held.max(1.0)));
                let mut child = ui.new_child(
                    UiBuilder::new()
                        .id_salt(("cell", r, i))
                        .max_rect(rect)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                child.data_mut(|d| d.insert_temp(child.id(), CellHeight(held)));
                child.set_width(w);
                child.data_mut(|d| d.remove_temp::<CellNatural>(child.id()));
                draw(&mut child);
                // a panel's own height, unstretched; anything else, the height it took
                let natural = child
                    .data(|d| d.get_temp::<CellNatural>(child.id()))
                    .map_or(child.min_rect().height(), |n| n.0);
                tallest = tallest.max(natural);
                x += w + gap;
            }
            if (tallest - held).abs() > 0.5 {
                ui.data_mut(|d| d.insert_temp(key, tallest));
                ui.ctx().request_discard("a dashboard row changed height");
            }
            if r + 1 == count {
                // the grid as a whole stands its page's spacing from what follows
                ui.spacing_mut().item_spacing.y = spacing;
            }
            ui.allocate_rect(
                Rect::from_min_size(pos2(ui.cursor().left(), top), vec2(width, tallest)),
                Sense::hover(),
            );
            if r + 1 < count {
                ui.add_space(gap);
            }
        }
    }
}

/// A panel of a dashboard: a head with its title and subtitle over a rule, its body padded 16
/// (flush for a table or a graph), and a footer of muted words over a rule, in a card with the
/// strong border and the surface's corners. In a `DashboardGrid` it stretches to its row.
pub struct DashboardPanel<'a> {
    title: String,
    subtitle: Option<String>,
    flush: bool,
    footer: Option<String>,
    actions: Option<Tools<'a>>,
}

impl<'a> DashboardPanel<'a> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            flush: false,
            footer: None,
            actions: None,
        }
    }

    /// Controls at the head's end, such as a small ghost button to the page the panel sums up.
    pub fn actions(mut self, actions: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.actions = Some(Box::new(actions));
        self
    }

    /// Muted words beside the title, such as what the figures are in.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// The body to the card's edges rather than padded.
    pub fn flush(mut self, flush: bool) -> Self {
        self.flush = flush;
        self
    }

    /// A line of muted words under the body, over a rule.
    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    pub fn show<R>(self, ui: &mut Ui, body: impl FnOnce(&mut Ui) -> R) -> R {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        let stretch = cell_height(ui).unwrap_or(0.0);
        let cell = ui.id();
        let radius = t.radius("surface");
        let ink = t.color("muted-foreground");
        let rule = Stroke::new(1.0, t.color("border"));
        egui::Frame::new()
            .fill(t.color("card"))
            .stroke(Stroke::new(1.0, t.color("border-strong")))
            .corner_radius(radius)
            .show(ui, |ui| {
                ui.set_width(width - 2.0);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                let inner = width - 2.0 - 24.0;
                // the head: at least 40 tall, 12 in, the title and the subtitle 8 apart on one line
                let title = WidgetText::from(TextRole::Label.rich(&t, &self.title)).into_galley(
                    ui,
                    Some(TextWrapMode::Truncate),
                    inner.max(1.0),
                    TextRole::Label.font(&t),
                );
                let subtitle = self.subtitle.as_ref().map(|s| {
                    WidgetText::from(TextRole::Help.rich(&t, s)).into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        (inner - title.size().x - 8.0).max(1.0),
                        TextRole::Help.font(&t),
                    )
                });
                let (head, _) = ui.allocate_exact_size(vec2(width - 2.0, 40.0), Sense::hover());
                ui.painter().hline(head.x_range(), head.bottom() - 0.5, rule);
                let middle = head.center().y;
                let x = head.left() + 12.0;
                let title_width = title.size().x;
                // the two sit on one baseline: the title's line centered, the subtitle's foot level with it
                let title_top = middle - title.size().y / 2.0;
                let baseline = title_top + title.rows.first().map_or(0.0, |r| r.max_y());
                ui.painter()
                    .galley(pos2(x, title_top), title, t.color("card-foreground"));
                if let Some(subtitle) = subtitle {
                    let sub_base = subtitle.rows.first().map_or(0.0, |r| r.max_y());
                    ui.painter()
                        .galley(pos2(x + title_width + 8.0, baseline - sub_base), subtitle, ink);
                }
                if let Some(actions) = self.actions {
                    let end = egui::Rect::from_min_max(
                        pos2(head.left() + 12.0, head.top()),
                        pos2(head.right() - 12.0, head.bottom() - 1.0),
                    );
                    let mut tools = ui.new_child(
                        UiBuilder::new()
                            .id_salt("dashboard-panel-actions")
                            .max_rect(end)
                            .layout(egui::Layout::right_to_left(egui::Align::Center)),
                    );
                    tools.spacing_mut().item_spacing.x = 8.0;
                    actions(&mut tools);
                }
                let footer = self.footer.as_ref().map(|f| {
                    WidgetText::from(TextRole::Help.rich(&t, f)).into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        inner.max(1.0),
                        TextRole::Help.font(&t),
                    )
                });
                let footer_height = footer.as_ref().map_or(0.0, |f| f.size().y + 12.0);
                let margin = if self.flush { 0 } else { 16 };
                let out = egui::Frame::new()
                    .inner_margin(egui::Margin::same(margin))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        body(ui)
                    })
                    .inner;
                // the body grows so the footer sits at the foot of a stretched panel
                let used = ui.min_rect().height() + footer_height + 2.0;
                ui.data_mut(|d| d.insert_temp(cell, CellNatural(used)));
                if stretch > used {
                    ui.add_space(stretch - used);
                }
                if let Some(footer) = footer {
                    let (foot, _) = ui.allocate_exact_size(vec2(width - 2.0, footer_height), Sense::hover());
                    ui.painter().hline(foot.x_range(), foot.top() + 0.5, rule);
                    ui.painter()
                        .galley(pos2(foot.left() + 12.0, foot.top() + 6.0), footer, ink);
                }
                out
            })
            .inner
    }
}
