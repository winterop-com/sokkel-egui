//! The one search for a page, after sokkel-ui's `Search`: a `SearchField` whose results open in a
//! popover under it, with a preview of the highlighted one beside them.

use egui::{Align, Id, Layout, Rect, Sense, Stroke, Ui, pos2, vec2};

use super::search_results::{SearchGroup, SearchResult, marked};
use crate::{Icon, SearchField, TextRole, icons, tokens};

/// The words a search says.
pub struct SearchWords {
    pub placeholder: String,
    pub empty: Box<dyn Fn(&str) -> String>,
    /// The way to the results page for the query.
    pub see_all: Box<dyn Fn(&str) -> String>,
}

impl Default for SearchWords {
    fn default() -> Self {
        Self {
            placeholder: "Search".into(),
            empty: Box::new(|query| format!("Nothing matches \u{201c}{query}\u{201d}.")),
            see_all: Box::new(|query| format!("See all results for \u{201c}{query}\u{201d}")),
        }
    }
}

/// What the search was asked this frame.
#[derive(Clone, Debug, Default)]
pub struct SearchResponse {
    /// A result opened: its group's kind and its id. The list closes.
    pub opened: Option<(String, String)>,
    /// The way to the results page was taken, for this query.
    pub see_all: Option<String>,
}

/// What the preview pane draws for the highlighted result.
pub type SearchPreview<'a> = &'a dyn Fn(&SearchResult, &mut Ui);

/// The one search for a page, as sokkel-ui's `Search`: the app answers the query with `groups`
/// each frame, and they open in a popover under the field (512 wide, 736 with a preview, at most
/// 480 tall), grouped under their headings, each result its icon on a 32 muted tile, its title
/// with the query's words marked and a line under it. The list opens on typing, a press on the
/// field or Down, never on focus alone; the arrows move through it, Enter opens the highlighted
/// result, Escape closes it and a second Escape clears the field. With `see_all` the list ends in
/// a way to the full results page, which Enter also takes when nothing is listed.
pub struct Search<'a> {
    id: Id,
    query: &'a mut String,
    groups: &'a [SearchGroup],
    see_all: bool,
    preview: Option<SearchPreview<'a>>,
    width: Option<f32>,
    words: SearchWords,
}

#[derive(Clone, Copy, Default)]
struct State {
    open: bool,
    highlight: usize,
}

impl<'a> Search<'a> {
    pub fn new(id: impl std::hash::Hash + std::fmt::Debug, query: &'a mut String, groups: &'a [SearchGroup]) -> Self {
        Self {
            id: Id::unique(("sokkel-search", id)),
            query,
            groups,
            see_all: false,
            preview: None,
            width: None,
            words: SearchWords::default(),
        }
    }

    /// End the list in a way to the full results page.
    pub fn see_all(mut self, see_all: bool) -> Self {
        self.see_all = see_all;
        self
    }

    pub fn preview(mut self, preview: SearchPreview<'a>) -> Self {
        self.preview = Some(preview);
        self
    }

    /// The field's width; as `SearchField` takes it otherwise.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn words(mut self, words: SearchWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> SearchResponse {
        let t = tokens(ui.ctx());
        let ctx = ui.ctx().clone();
        let mut out = SearchResponse::default();
        let mut state: State = ctx.data(|d| d.get_temp(self.id)).unwrap_or_default();
        let before = self.query.clone();
        let mut field = SearchField::new(self.id.with("field"), self.query).keys("/");
        field = field.placeholder(&self.words.placeholder);
        if let Some(width) = self.width {
            field = field.width(width);
        }
        let response = field.show(ui);
        let typed = *self.query != before;
        if typed {
            state.open = true;
            state.highlight = 0;
        }
        if response.clicked() {
            state.open = true;
        }
        let trimmed = self.query.trim().to_owned();
        let entries: Vec<(&SearchGroup, &SearchResult)> = self
            .groups
            .iter()
            .flat_map(|g| g.results.iter().map(move |r| (g, r)))
            .collect();
        let see_all = self.see_all && !trimmed.is_empty();
        let count = entries.len() + usize::from(see_all);
        if response.has_focus() {
            let (down, up, enter, escape) = ctx.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                    i.key_pressed(egui::Key::Enter),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
                )
            });
            if down {
                if state.open {
                    state.highlight = (state.highlight + 1).min(count.saturating_sub(1));
                } else {
                    state.open = true;
                }
            }
            if up {
                state.highlight = state.highlight.saturating_sub(1);
            }
            if escape {
                if state.open && !trimmed.is_empty() {
                    state.open = false;
                } else {
                    self.query.clear();
                    state.open = false;
                }
            }
            if enter && state.open && !trimmed.is_empty() {
                choose(&entries, state.highlight, see_all, &trimmed, &mut out);
            }
        }
        // Enter takes the focus from egui's field; the list stays bound to what was typed
        if response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) && state.open && !trimmed.is_empty()
        {
            choose(&entries, state.highlight, see_all, &trimmed, &mut out);
        }
        let shown = state.open && !trimmed.is_empty();
        if shown {
            let width: f32 = if self.preview.is_some() { 736.0 } else { 512.0 };
            let screen = ctx.content_rect();
            let width = width.min(screen.width() - 32.0).max(response.rect.width());
            let at = pos2(
                response.rect.left().min(screen.right() - 16.0 - width),
                response.rect.bottom() + 4.0,
            );
            let area = egui::Area::new(self.id.with("results"))
                .order(egui::Order::Foreground)
                .fixed_pos(at)
                .show(&ctx, |ui| {
                    super::popup_frame(&t).show(ui, |ui| {
                        ui.set_width(width - 2.0);
                        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                        let list_height = 480.0 - if see_all { 45.0 } else { 0.0 };
                        let highlighted = entries.get(state.highlight).map(|(_, r)| *r);
                        ui.horizontal_top(|ui| {
                            let list_width = if self.preview.is_some() && highlighted.is_some() {
                                (width - 2.0) * 0.55
                            } else {
                                width - 2.0
                            };
                            ui.allocate_ui_with_layout(
                                vec2(list_width, list_height),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    egui::ScrollArea::vertical()
                                        .id_salt(self.id.with("list"))
                                        .max_height(list_height)
                                        .auto_shrink([false, true])
                                        .show(ui, |ui| {
                                            egui::Frame::new().inner_margin(egui::Margin::same(6)).show(ui, |ui| {
                                                ui.set_width(list_width - 12.0);
                                                if entries.is_empty() {
                                                    ui.add_space(24.0);
                                                    ui.vertical_centered(|ui| {
                                                        ui.label(
                                                            TextRole::Body
                                                                .rich(&t, (self.words.empty)(&trimmed))
                                                                .color(t.color("muted-foreground")),
                                                        );
                                                    });
                                                    ui.add_space(24.0);
                                                }
                                                let mut index = 0;
                                                for group in self.groups.iter().filter(|g| !g.results.is_empty()) {
                                                    ui.horizontal(|ui| {
                                                        ui.add_space(8.0);
                                                        ui.vertical(|ui| {
                                                            ui.add_space(6.0);
                                                            super::group_label(ui, &group.title);
                                                            ui.add_space(4.0);
                                                        });
                                                    });
                                                    for result in &group.results {
                                                        let on = state.highlight == index;
                                                        let row = option_row(ui, result, &trimmed, on);
                                                        if row.hovered()
                                                            && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO)
                                                        {
                                                            state.highlight = index;
                                                        }
                                                        if row.clicked() {
                                                            out.opened = Some((group.kind.clone(), result.id.clone()));
                                                        }
                                                        index += 1;
                                                    }
                                                }
                                            });
                                        });
                                },
                            );
                            if let (Some(preview), Some(result)) = (self.preview, highlighted) {
                                let rect = ui.available_rect_before_wrap();
                                ui.painter().vline(
                                    rect.left() + 0.5,
                                    rect.top()..=rect.top() + list_height,
                                    Stroke::new(1.0, t.color("border")),
                                );
                                ui.allocate_ui_with_layout(
                                    vec2(ui.available_width(), list_height),
                                    Layout::top_down(Align::Min),
                                    |ui| {
                                        egui::ScrollArea::vertical()
                                            .id_salt(self.id.with("preview"))
                                            .max_height(list_height)
                                            .show(ui, |ui| {
                                                egui::Frame::new().inner_margin(egui::Margin::same(12)).show(
                                                    ui,
                                                    |ui| {
                                                        ui.set_width(ui.available_width());
                                                        preview(result, ui);
                                                    },
                                                );
                                            });
                                    },
                                );
                            }
                        });
                        if see_all {
                            let rect = ui.available_rect_before_wrap();
                            ui.painter()
                                .hline(rect.x_range(), rect.top() + 0.5, Stroke::new(1.0, t.color("border")));
                            egui::Frame::new().inner_margin(egui::Margin::same(6)).show(ui, |ui| {
                                let on = state.highlight == entries.len();
                                let (row, hit) =
                                    ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
                                if on || hit.hovered() {
                                    ui.painter().rect_filled(row, t.radius("md"), t.color("accent"));
                                }
                                let ink = if on {
                                    t.color("accent-foreground")
                                } else {
                                    t.color("muted-foreground")
                                };
                                let words = (self.words.see_all)(&trimmed);
                                let galley = egui::WidgetText::from(TextRole::Body.rich(&t, words)).into_galley(
                                    ui,
                                    Some(egui::TextWrapMode::Truncate),
                                    row.width() - 40.0,
                                    TextRole::Body.font(&t),
                                );
                                ui.painter().galley(
                                    pos2(row.left() + 8.0, row.center().y - galley.size().y / 2.0),
                                    galley,
                                    ink,
                                );
                                icons::paint(
                                    ui.painter(),
                                    pos2(row.right() - 8.0 - 8.0, row.center().y),
                                    Icon::ArrowRight,
                                    16.0,
                                    ink,
                                );
                                if hit.clicked() {
                                    out.see_all = Some(trimmed.clone());
                                }
                            });
                        }
                    });
                });
            // a press outside the field and the list closes it
            let outside = ctx.input(|i| i.pointer.any_pressed())
                && ctx
                    .input(|i| i.pointer.interact_pos())
                    .is_some_and(|p| !area.response.rect.contains(p) && !response.rect.contains(p));
            if outside {
                state.open = false;
            }
        }
        if out.opened.is_some() || out.see_all.is_some() {
            state.open = false;
            self.query.clear();
        }
        ctx.data_mut(|d| d.insert_temp(self.id, state));
        out
    }
}

/// The result or the way to the results page at `index`, as Enter takes it.
fn choose(
    entries: &[(&SearchGroup, &SearchResult)],
    index: usize,
    see_all: bool,
    query: &str,
    out: &mut SearchResponse,
) {
    match entries.get(index) {
        Some((group, result)) => out.opened = Some((group.kind.clone(), result.id.clone())),
        None if see_all => out.see_all = Some(query.to_owned()),
        None => {}
    }
}

/// One result in the list: 8 across and 6 over and under, the accent ground while highlighted;
/// its icon on a 32 muted tile, 10 before its title over its line, the query's words marked.
fn option_row(ui: &mut Ui, result: &SearchResult, query: &str, on: bool) -> egui::Response {
    let t = tokens(ui.ctx());
    let tile = 32.0;
    let (rect, hit) = ui.allocate_exact_size(vec2(ui.available_width(), tile + 12.0), Sense::click());
    hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &result.title));
    if on {
        ui.painter().rect_filled(rect, t.radius("md"), t.color("accent"));
    }
    let mut x = rect.left() + 8.0;
    if let Some(icon) = result.icon {
        let square = Rect::from_min_size(pos2(x, rect.center().y - tile / 2.0), vec2(tile, tile));
        ui.painter().rect_filled(square, t.radius("md"), t.color("muted"));
        icons::paint(ui.painter(), square.center(), icon, 16.0, t.color("muted-foreground"));
        x += tile + 10.0;
    }
    let (ink, quiet) = if on {
        (t.color("accent-foreground"), t.color("accent-foreground"))
    } else {
        (t.color("foreground"), t.color("muted-foreground"))
    };
    let mut end = rect.right() - 8.0;
    if let Some(meta) = &result.meta {
        let galley = egui::WidgetText::from(TextRole::Help.rich(&t, meta)).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            TextRole::Help.font(&t),
        );
        end -= galley.size().x;
        ui.painter()
            .galley(pos2(end, rect.center().y - galley.size().y / 2.0), galley, quiet);
        end -= 10.0;
    }
    let room = (end - x).max(1.0);
    let title = ui
        .painter()
        .layout_job(marked(ui, &result.title, query, TextRole::Body, 400.0, ink, room));
    let detail = result.detail.as_ref().map(|d| {
        ui.painter()
            .layout_job(marked(ui, d, query, TextRole::Help, 400.0, quiet, room))
    });
    let height = title.size().y + detail.as_ref().map_or(0.0, |d| d.size().y);
    let mut y = rect.center().y - height / 2.0;
    let title_height = title.size().y;
    ui.painter().galley(pos2(x, y), title, ink);
    y += title_height;
    if let Some(detail) = detail {
        ui.painter().galley(pos2(x, y), detail, quiet);
    }
    hit
}
