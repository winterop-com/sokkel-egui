//! Choosing a block for a step, after sokkel-ui-graph's `BlockPicker`, a searchable list held over
//! a canvas, and `BlockPalette`, the blocks listed beside it to drag on or click.

use egui::{
    Align, Id, Key, Layout, Modifiers, Order, Rect, ScrollArea, Sense, Stroke, StrokeKind, TextEdit, Ui, UiBuilder,
    Vec2, pos2, vec2,
};
use sokkel_egui::{Chip, Icon, SearchField, TextRole, Tokens, group_label, mono_font, tokens};

use crate::blocks::{BlockGroup, GraphDragItem, PipelineBlock, group_blocks};
use crate::toolbar::galley;

/// The picker's widths: 16, 20 and 24 rem.
const WIDTH_MD: f32 = 256.0;
const WIDTH_LG: f32 = 320.0;
const WIDTH_XL: f32 = 384.0;
/// The tallest a picker is drawn, as `max-h-[20rem]`.
const MOST_HEIGHT: f32 = 320.0;
/// A row's glyph box, the glyph in it, and the gap after it.
const GLYPH_BOX: f32 = 28.0;
const GLYPH: f32 = 14.0;
const ROW_GAP: f32 = 10.0;
/// A row's padding.
const ROW_PAD: Vec2 = vec2(6.0, 4.0);
/// The search's glyph, the search line's padding and the list's.
const SEARCH_GLYPH: f32 = 16.0;
const SEARCH_PAD: Vec2 = vec2(12.0, 8.0);
const LIST_PAD: f32 = 4.0;
/// A title's line over the search and the gap above it.
const TITLE_TOP: f32 = 10.0;

/// How wide a block picker is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PickerWidth {
    /// 16rem, the default.
    #[default]
    Md,
    Lg,
    Xl,
}

/// The words a block picker says.
#[derive(Clone, Debug)]
pub struct BlockPickerWords {
    /// The search field's name.
    pub search: String,
    /// What the empty search field shows; the search field's name when left out.
    pub placeholder: Option<String>,
    /// The list's name.
    pub list: String,
    /// What it says when no block matches the search.
    pub none: String,
    /// What it says when no block fits where the step would go.
    pub nothing: String,
    /// What it says while the blocks are still being read.
    pub loading: String,
}

impl Default for BlockPickerWords {
    fn default() -> Self {
        Self {
            search: "Find a block".into(),
            placeholder: None,
            list: "Blocks".into(),
            none: "No block matches".into(),
            nothing: "No block fits here".into(),
            loading: "Reading the blocks".into(),
        }
    }
}

/// What the picker answers with: the block picked, by its index in the blocks given, and whether
/// Escape on an empty search asked it to close.
#[derive(Clone, Copy, Debug, Default)]
pub struct BlockPickerResponse {
    pub picked: Option<usize>,
    pub closed: bool,
}

/// The search and the active row, kept while the picker stays open.
#[derive(Clone, Debug, Default)]
struct PickerState {
    shown_at: u64,
    query: String,
    active: usize,
    scroll: bool,
}

/// A searchable list of blocks to choose one from, for a step about to be added: after a step,
/// between two, or at the end of a connection. The search takes the focus; the arrow keys move
/// through the list, Enter picks, and Escape clears the search, or closes once it is empty. Hand it
/// only the blocks that fit where the step would go, and hold it over a canvas with `GraphOverlay`.
pub struct BlockPicker<'a> {
    id: Id,
    blocks: &'a [PipelineBlock],
    groups: &'a [BlockGroup],
    title: Option<String>,
    loading: bool,
    show_description: bool,
    width: PickerWidth,
    room: Option<f32>,
    most_width: Option<f32>,
    words: BlockPickerWords,
}

impl<'a> BlockPicker<'a> {
    pub fn new(
        id: impl std::hash::Hash + std::fmt::Debug,
        blocks: &'a [PipelineBlock],
        groups: &'a [BlockGroup],
    ) -> Self {
        Self {
            id: Id::unique(("sokkel-block-picker", id)),
            blocks,
            groups,
            title: None,
            loading: false,
            show_description: false,
            width: PickerWidth::Md,
            room: None,
            most_width: None,
            words: BlockPickerWords::default(),
        }
    }

    /// Where the step would go, said over the search: "After validate".
    pub fn title(mut self, title: Option<String>) -> Self {
        self.title = title;
        self
    }

    /// The blocks are still being read: it says so, and Enter picks nothing.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Draws each block's description as a line of its own under its title.
    pub fn show_description(mut self, show: bool) -> Self {
        self.show_description = show;
        self
    }

    pub fn width(mut self, width: PickerWidth) -> Self {
        self.width = width;
        self
    }

    /// The most height and width it has, as an overlay tells it.
    pub fn room(mut self, height: f32, width: f32) -> Self {
        self.room = Some(height);
        self.most_width = Some(width);
        self
    }

    pub fn words(mut self, words: BlockPickerWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> BlockPickerResponse {
        let t = tokens(ui.ctx());
        let frame_nr = ui.ctx().cumulative_frame_nr();
        let state_id = self.id.with("state");
        let mut state: PickerState = ui.data(|d| d.get_temp(state_id)).unwrap_or_default();
        // a picker not shown last frame opens afresh
        let fresh = state.shown_at + 1 < frame_nr || state.shown_at == 0;
        if fresh {
            state = PickerState::default();
        }
        state.shown_at = frame_nr;
        let mut answer = BlockPickerResponse::default();

        let listed = if self.loading {
            Vec::new()
        } else {
            group_blocks(self.blocks, self.groups, &state.query)
        };
        let flat: Vec<usize> = listed.iter().flat_map(|(_, b)| b.iter().copied()).collect();
        let at = state.active.min(flat.len().saturating_sub(1));

        // the keys the search answers before its field sees them
        let field_id = self.id.with("search");
        let focused = ui.memory(|m| m.has_focus(field_id)) || fresh;
        if focused {
            let (down, up, enter, escape) = ui.input_mut(|i| {
                (
                    i.consume_key(Modifiers::NONE, Key::ArrowDown),
                    i.consume_key(Modifiers::NONE, Key::ArrowUp),
                    i.consume_key(Modifiers::NONE, Key::Enter),
                    i.consume_key(Modifiers::NONE, Key::Escape),
                )
            });
            if !flat.is_empty() && (down || up) {
                state.active = if down {
                    (at + 1) % flat.len()
                } else {
                    (at + flat.len() - 1) % flat.len()
                };
                state.scroll = true;
            }
            if enter && let Some(&block) = flat.get(at) {
                answer.picked = Some(block);
            }
            if escape {
                if state.query.is_empty() {
                    answer.closed = true;
                } else {
                    state.query.clear();
                    state.active = 0;
                }
            }
        }
        let at = state.active.min(flat.len().saturating_sub(1));

        let width = match self.width {
            PickerWidth::Md => WIDTH_MD,
            PickerWidth::Lg => WIDTH_LG,
            PickerWidth::Xl => WIDTH_XL,
        }
        .min(self.most_width.unwrap_or(f32::INFINITY));
        let most_height = MOST_HEIGHT.min(self.room.unwrap_or(MOST_HEIGHT));
        let corners = t.radius("control");
        let frame = egui::Frame::new()
            .fill(t.color("popover"))
            .stroke(Stroke::new(1.0, t.color("border")))
            .corner_radius(corners)
            .shadow(t.shadow("elevation-pop"));
        frame.show(ui, |ui| {
            ui.set_width(width - 2.0);
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let top = ui.cursor().top();
            if let Some(title) = &self.title {
                ui.add_space(TITLE_TOP);
                let words = galley(ui, TextRole::Badge.rich(&t, title));
                let (row, _) = ui.allocate_exact_size(vec2(width - 2.0, words.size().y), Sense::hover());
                ui.painter()
                    .with_clip_rect(row.shrink2(vec2(SEARCH_PAD.x, 0.0)))
                    .galley(
                        pos2(row.left() + SEARCH_PAD.x, row.top()),
                        words,
                        t.color("muted-foreground"),
                    );
            }

            // the search
            let line = TextRole::Body.line_height(&t);
            let (search, _) = ui.allocate_exact_size(vec2(width - 2.0, line + 2.0 * SEARCH_PAD.y), Sense::hover());
            ui.painter().hline(
                search.x_range(),
                search.bottom() - 0.5,
                Stroke::new(1.0, t.color("border")),
            );
            sokkel_egui::icons::paint(
                ui.painter(),
                pos2(search.left() + SEARCH_PAD.x + SEARCH_GLYPH / 2.0, search.center().y),
                Icon::Search,
                SEARCH_GLYPH,
                t.color("faint"),
            );
            let field_rect = Rect::from_min_max(
                pos2(
                    search.left() + SEARCH_PAD.x + SEARCH_GLYPH + 8.0,
                    search.top() + SEARCH_PAD.y,
                ),
                pos2(search.right() - SEARCH_PAD.x, search.bottom() - SEARCH_PAD.y),
            );
            let placeholder = self
                .words
                .placeholder
                .clone()
                .unwrap_or_else(|| self.words.search.clone());
            let before = state.query.clone();
            let mut field = ui.scope_builder(UiBuilder::new().max_rect(field_rect), |ui| {
                ui.add(
                    TextEdit::singleline(&mut state.query)
                        .id(field_id)
                        .frame(egui::Frame::NONE)
                        .font(TextRole::Body.font(&t))
                        .text_color(t.color("popover-foreground"))
                        .hint_text(TextRole::Body.rich(&t, placeholder).color(t.color("faint")))
                        .desired_width(field_rect.width())
                        .margin(Vec2::ZERO),
                )
            });
            field.inner = field.inner.on_hover_cursor(egui::CursorIcon::Text);
            field.inner.widget_info(|| {
                egui::WidgetInfo::text_edit(true, before.clone(), state.query.clone(), self.words.search.clone())
            });
            if fresh {
                field.inner.request_focus();
            }
            if state.query != before {
                state.active = 0;
            }

            // the list
            let used = ui.cursor().top() - top;
            let list_height = (most_height - used - 2.0).max(GLYPH_BOX + 2.0 * LIST_PAD);
            ScrollArea::vertical()
                .id_salt(self.id.with("list"))
                .max_height(list_height)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    ui.add_space(LIST_PAD);
                    if flat.is_empty() {
                        let said = if self.loading {
                            &self.words.loading
                        } else if self.blocks.is_empty() {
                            &self.words.nothing
                        } else {
                            &self.words.none
                        };
                        let words = galley(ui, TextRole::Body.rich(&t, said));
                        let (row, _) = ui.allocate_exact_size(vec2(width - 2.0, words.size().y + 40.0), Sense::hover());
                        ui.painter().galley(
                            pos2(
                                row.center().x - words.size().x / 2.0,
                                row.center().y - words.size().y / 2.0,
                            ),
                            words,
                            t.color("muted-foreground"),
                        );
                    }
                    let mut index = 0usize;
                    for (group, inside) in &listed {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.add_space(LIST_PAD + 8.0);
                            group_label(ui, &group.label);
                        });
                        ui.add_space(2.0);
                        for &k in inside {
                            let mine = index;
                            index += 1;
                            let block = &self.blocks[k];
                            let active = mine == at;
                            let size = vec2(
                                width - 2.0 - 2.0 * LIST_PAD,
                                row_height(&t, block, self.show_description),
                            );
                            let (spot, _) = ui.allocate_exact_size(vec2(width - 2.0, size.y), Sense::hover());
                            let row = Rect::from_min_size(pos2(spot.left() + LIST_PAD, spot.top()), size);
                            let response = ui.interact(row, self.id.with(("block", k)), Sense::click());
                            response.widget_info(|| {
                                egui::WidgetInfo::selected(egui::Role::Button, true, active, block_title(block))
                            });
                            if response.hovered() && ui.input(|i| i.pointer.delta() != Vec2::ZERO) {
                                state.active = mine;
                            }
                            if active {
                                ui.painter().rect_filled(row, t.radius("chip"), t.color("muted"));
                                if state.scroll {
                                    ui.scroll_to_rect(row, None);
                                    state.scroll = false;
                                }
                            }
                            paint_row(ui, &t, row.shrink2(ROW_PAD), block, active, self.show_description);
                            if let Some(description) = block.description.as_deref().filter(|_| !self.show_description) {
                                let response = response.on_hover_text(description);
                                if response.clicked() {
                                    answer.picked = Some(k);
                                }
                            } else if response.clicked() {
                                answer.picked = Some(k);
                            }
                        }
                    }
                    ui.add_space(LIST_PAD);
                });
        });
        ui.data_mut(|d| d.insert_temp(state_id, state));
        answer
    }
}

/// A block's title: its name, or else its kind.
fn block_title(block: &PipelineBlock) -> &str {
    match block.name.as_deref() {
        Some(name) if !name.is_empty() => name,
        _ => &block.kind,
    }
}

/// How tall a block's row is, its padding included.
fn row_height(t: &Tokens, block: &PipelineBlock, show_description: bool) -> f32 {
    let named = block.name.as_deref().is_some_and(|n| !n.is_empty());
    let described = show_description && block.description.as_deref().is_some_and(|d| !d.is_empty());
    let title = 20.0;
    let under = if described {
        2.0 * t.text("help").line_height.min(16.0)
    } else if named {
        16.0
    } else {
        0.0
    };
    2.0 * ROW_PAD.y + GLYPH_BOX.max(title + under)
}

/// One block as a row: its glyph in a box, the title (the name, or else the kind in the code face),
/// the kind under a name, or the description when asked, and its badge at the end.
fn paint_row(ui: &mut Ui, t: &Tokens, row: Rect, block: &PipelineBlock, active: bool, show_description: bool) {
    let painter = ui.painter().with_clip_rect(row.expand(1.0));
    let glyph_box = Rect::from_min_size(
        pos2(row.left(), row.center().y - GLYPH_BOX / 2.0),
        Vec2::splat(GLYPH_BOX),
    );
    let (edge, ink) = if active {
        (t.color("border-strong"), t.color("foreground"))
    } else {
        (t.color("border"), t.color("muted-foreground"))
    };
    painter.rect(
        glyph_box,
        t.radius("control"),
        t.color("background"),
        Stroke::new(1.0, edge),
        StrokeKind::Inside,
    );
    sokkel_egui::icons::paint(
        &painter,
        glyph_box.center(),
        block.icon.unwrap_or(Icon::Box),
        GLYPH,
        ink,
    );
    let named = block.name.as_deref().is_some_and(|n| !n.is_empty());
    let mut right = row.right();
    if let Some(badge) = block.shown_badge() {
        let chip = Chip::new(&badge.label).tone(badge.tone);
        let chip_width = galley(ui, TextRole::Badge.rich(t, &badge.label)).size().x + 16.0;
        let spot = Rect::from_min_size(pos2(right - chip_width, row.top()), vec2(chip_width, 20.0));
        let response = ui.put(spot, chip);
        if let Some(title) = &badge.title {
            response.on_hover_text(title);
        }
        right = spot.left() - 8.0;
    }
    let left = glyph_box.right() + ROW_GAP;
    let room = (right - left).max(0.0);
    let words = Rect::from_min_max(pos2(left, row.top()), pos2(right, row.bottom()));
    let painter = painter.with_clip_rect(words);
    let title = if named {
        galley(ui, TextRole::Body.rich(t, block_title(block)).line_height(Some(20.0)))
    } else {
        let size = t.text("body").size;
        painter.layout_no_wrap(
            block.kind.clone(),
            mono_font(ui.ctx(), size, 400.0),
            egui::Color32::PLACEHOLDER,
        )
    };
    let described = show_description
        .then_some(block.description.as_deref())
        .flatten()
        .filter(|d| !d.is_empty());
    let second = if let Some(description) = described {
        let mut job = egui::text::LayoutJob::default();
        TextRole::Help.rich(t, description).line_height(Some(16.0)).append_to(
            &mut job,
            ui.style(),
            egui::FontSelection::Default,
            Align::Min,
        );
        job.wrap.max_width = room;
        job.wrap.max_rows = 2;
        Some(painter.layout_job(job))
    } else if named {
        Some(painter.layout_no_wrap(
            block.kind.clone(),
            mono_font(ui.ctx(), t.text("help").size, 400.0),
            egui::Color32::PLACEHOLDER,
        ))
    } else {
        None
    };
    let block_height = 20.0 + second.as_ref().map_or(0.0, |g| g.size().y);
    let top = row.center().y - block_height / 2.0;
    let title_top = top + (20.0 - title.size().y) / 2.0;
    painter.galley(pos2(left, title_top), title, t.color("popover-foreground"));
    if let Some(second) = second {
        painter.galley(pos2(left, top + 20.0), second, t.color("muted-foreground"));
    }
}

/// The words a palette says.
#[derive(Clone, Debug)]
pub struct BlockPaletteWords {
    /// The palette's accessible name.
    pub label: String,
    pub search: String,
    /// What it says when no block matches the search.
    pub none: String,
    /// The line under the list that says how a block is used.
    pub hint: String,
}

impl Default for BlockPaletteWords {
    fn default() -> Self {
        Self {
            label: "Blocks".into(),
            search: "Search blocks".into(),
            none: "No block matches".into(),
            hint: "Drag a block onto the canvas, or click it to add it after the chosen step.".into(),
        }
    }
}

/// What a palette answers with: the block clicked, by its index in the blocks given.
#[derive(Clone, Copy, Debug, Default)]
pub struct BlockPaletteResponse {
    pub picked: Option<usize>,
}

/// The blocks a pipeline is built from, listed under their groups with a search over them. A block
/// drags onto a canvas whose editing takes dropped items, carrying its kind as a `GraphDragItem`;
/// a click picks it. The palette fills the room it is given and its list scrolls.
pub struct BlockPalette<'a> {
    id: Id,
    blocks: &'a [PipelineBlock],
    groups: &'a [BlockGroup],
    query: &'a mut String,
    words: BlockPaletteWords,
}

impl<'a> BlockPalette<'a> {
    pub fn new(
        id: impl std::hash::Hash + std::fmt::Debug,
        blocks: &'a [PipelineBlock],
        groups: &'a [BlockGroup],
        query: &'a mut String,
    ) -> Self {
        Self {
            id: Id::unique(("sokkel-block-palette", id)),
            blocks,
            groups,
            query,
            words: BlockPaletteWords::default(),
        }
    }

    pub fn words(mut self, words: BlockPaletteWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> BlockPaletteResponse {
        let t = tokens(ui.ctx());
        let mut answer = BlockPaletteResponse::default();
        let size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Unknown, true, &self.words.label));
        ui.painter().rect(
            rect,
            t.radius("surface"),
            t.color("card"),
            Stroke::new(1.0, t.color("border")),
            StrokeKind::Inside,
        );
        let mut inner = ui.new_child(
            UiBuilder::new()
                .max_rect(rect.shrink(1.0))
                .layout(Layout::top_down(Align::Min)),
        );
        let ui = &mut inner;
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let inner_width = rect.width() - 2.0;

        // the search, then the hint at the foot, then the list between them
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            SearchField::new(self.id.with("search"), self.query)
                .placeholder(&self.words.search)
                .width(inner_width - 16.0)
                .show(ui);
        });
        ui.add_space(8.0);
        let line = ui.cursor().top();
        ui.painter()
            .hline(rect.x_range(), line - 0.5, Stroke::new(1.0, t.color("border")));
        let mut hint_job = egui::text::LayoutJob::default();
        TextRole::Help.rich(&t, &self.words.hint).append_to(
            &mut hint_job,
            ui.style(),
            egui::FontSelection::Default,
            Align::Min,
        );
        hint_job.wrap.max_width = inner_width - 24.0;
        let hint = ui.painter().layout_job(hint_job);
        let hint_height = hint.size().y + 16.0;
        let foot = rect.bottom() - 1.0 - hint_height;
        ui.painter()
            .hline(rect.x_range(), foot + 0.5, Stroke::new(1.0, t.color("border")));
        ui.painter()
            .galley(pos2(rect.left() + 13.0, foot + 8.0), hint, t.color("muted-foreground"));

        let listed = group_blocks(self.blocks, self.groups, self.query);
        let list_height = (foot - ui.cursor().top()).max(0.0);
        ScrollArea::vertical()
            .id_salt(self.id.with("list"))
            .max_height(list_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.add_space(6.0);
                if listed.is_empty() {
                    let words = galley(ui, TextRole::Body.rich(&t, &self.words.none));
                    let (row, _) = ui.allocate_exact_size(vec2(inner_width, words.size().y + 48.0), Sense::hover());
                    ui.painter().galley(
                        pos2(
                            row.center().x - words.size().x / 2.0,
                            row.center().y - words.size().y / 2.0,
                        ),
                        words,
                        t.color("muted-foreground"),
                    );
                }
                for (group, inside) in &listed {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.add_space(14.0);
                        group_label(ui, &group.label);
                    });
                    ui.add_space(4.0);
                    for &k in inside {
                        let block = &self.blocks[k];
                        let height = row_height(&t, block, false);
                        let (spot, _) = ui.allocate_exact_size(vec2(inner_width, height), Sense::hover());
                        let row = spot.shrink2(vec2(6.0, 0.0));
                        let id = self.id.with(("block", k));
                        let response = ui.interact(row, id, Sense::click_and_drag());
                        response
                            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, block_title(block)));
                        if response.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                            ui.painter().rect_filled(row, t.radius("control"), t.color("hover"));
                        }
                        if response.drag_started() {
                            egui::DragAndDrop::set_payload(ui.ctx(), GraphDragItem(block.kind.clone()));
                        }
                        if response.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                            if let Some(p) = ui.ctx().pointer_interact_pos() {
                                // the block follows the pointer while it is carried
                                let layer = egui::LayerId::new(Order::Tooltip, id.with("carried"));
                                let ghost = Rect::from_min_size(p + vec2(8.0, 8.0), row.size());
                                let painter = ui.ctx().layer_painter(layer);
                                painter.rect(
                                    ghost,
                                    t.radius("control"),
                                    t.color("card"),
                                    Stroke::new(1.0, t.color("border")),
                                    StrokeKind::Inside,
                                );
                                let mut child = ui.new_child(UiBuilder::new().layer_id(layer).max_rect(ghost));
                                paint_row(&mut child, &t, ghost.shrink2(ROW_PAD), block, false, false);
                            }
                        }
                        paint_row(ui, &t, row.shrink2(ROW_PAD), block, false, false);
                        let response = match block.description.as_deref() {
                            Some(description) => response.on_hover_text(description),
                            None => response,
                        };
                        if response.clicked() {
                            answer.picked = Some(k);
                        }
                    }
                    ui.add_space(6.0);
                }
            });
        answer
    }
}
