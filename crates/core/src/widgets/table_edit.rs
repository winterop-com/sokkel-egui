//! A table cell's editors, after sokkel-ui's `table-editing` and `InlineEditor`: text, a date typed
//! or picked from a calendar, and a choice from a list, each over its cell's whole box with the
//! field's ground and a line under it in the ring's color.

use egui::{
    Align, Area, CornerRadius, Frame, Id, Key, Layout, Margin, Order, Rect, Sense, Shadow, Stroke, TextEdit, Ui,
    UiBuilder, WidgetText, pos2, vec2,
};

use super::calendar::{Calendar, WeekStart, calendar_forget};
use crate::{Day, Icon, TextRole, Tokens, icons, tokens};

/// How a column's cells are edited.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Editor {
    Text,
    /// A `YYYY-MM-DD` day, typed or picked from a calendar whose weeks start on the day given.
    Date(WeekStart),
    /// One of the options, from a list opened over the cell.
    Select(Vec<String>),
}

/// The words a table's editing says, English by default.
#[derive(Clone, Debug)]
pub struct EditWords {
    pub unreadable_date: String,
    pub pick_date: String,
    pub clear: String,
}

impl Default for EditWords {
    fn default() -> Self {
        Self {
            unreadable_date: "Enter a date".into(),
            pick_date: "Choose a date".into(),
            clear: "Clear".into(),
        }
    }
}

/// The cell whose editor is open, with what is typed in it.
#[derive(Clone, Debug)]
pub(crate) struct OpenCell {
    pub row: usize,
    pub column: usize,
    pub draft: String,
    pub error: Option<String>,
    /// The first frame: the editor takes the focus, its text selected, a select's list open.
    pub fresh: bool,
    /// A date editor's calendar is open.
    pub calendar: bool,
    /// The option a select's list marks, moved by the arrows.
    pub highlight: usize,
}

/// What a key, a press or a choice in an open editor asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditMove {
    Stay,
    /// Save what is typed and close.
    Save,
    /// Drop what is typed and close.
    Cancel,
    /// Save and open the next editable cell, or the one before.
    Next(bool),
    /// The focus went elsewhere: save what is typed, and close even when it is refused.
    Leave,
}

/// The line under an editor: the ring's color, or the critical ink while refused.
fn underline(ui: &Ui, t: &Tokens, rect: Rect, refused: bool) {
    let ink = if refused { t.color("critical") } else { t.color("ring") };
    ui.painter().rect_filled(
        Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.0), rect.max),
        CornerRadius::ZERO,
        ink,
    );
}

/// A key the editor takes before its field sees it: Enter saves, Escape drops, Tab moves on.
fn keys(ui: &Ui) -> EditMove {
    ui.input_mut(|i| {
        if i.consume_key(egui::Modifiers::NONE, Key::Enter) {
            EditMove::Save
        } else if i.consume_key(egui::Modifiers::NONE, Key::Escape) {
            EditMove::Cancel
        } else if i.consume_key(egui::Modifiers::SHIFT, Key::Tab) {
            EditMove::Next(false)
        } else if i.consume_key(egui::Modifiers::NONE, Key::Tab) {
            EditMove::Next(true)
        } else {
            EditMove::Stay
        }
    })
}

/// A press this frame outside every rect given: the focus leaving the editor.
fn pressed_outside(ui: &Ui, inside: &[Rect]) -> bool {
    ui.input(|i| {
        i.pointer.any_pressed()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !inside.iter().any(|r| r.contains(p)))
    })
}

/// A text field without a frame in `rect`, its text selected on the editor's first frame.
fn bare_field(ui: &mut Ui, t: &Tokens, rect: Rect, draft: &mut String, id: Id, fresh: bool) -> egui::Response {
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let response = child.add(
        TextEdit::singleline(draft)
            .id(id)
            .font(TextRole::Body.font(t))
            .text_color(t.color("foreground"))
            .frame(Frame::NONE)
            .margin(Margin::ZERO)
            .desired_width(rect.width()),
    );
    if fresh {
        response.request_focus();
        let mut state = egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
        // every character selected, the caret at the start so the field shows the text's start
        let all = egui::text::CCursorRange::two(
            egui::text::CCursor::new(draft.chars().count()),
            egui::text::CCursor::new(0),
        );
        state.cursor.set_char_range(Some(all));
        state.store(ui.ctx(), id);
    }
    response
}

/// The open text editor over `cell`.
pub(crate) fn text(ui: &mut Ui, cell: Rect, open: &mut OpenCell, id: Id) -> EditMove {
    let t = tokens(ui.ctx());
    let fresh = std::mem::take(&mut open.fresh);
    ui.painter().rect_filled(cell, CornerRadius::ZERO, t.color("field"));
    let moved = keys(ui);
    let inner = Rect::from_min_max(
        pos2(cell.left() + 12.0, cell.top()),
        pos2(cell.right() - 12.0, cell.bottom()),
    );
    if bare_field(ui, &t, inner, &mut open.draft, id, fresh).changed() {
        open.error = None;
    }
    underline(ui, &t, cell, open.error.is_some());
    alert(ui, &t, cell, open.error.as_deref());
    if moved != EditMove::Stay {
        moved
    } else if pressed_outside(ui, &[cell]) {
        EditMove::Leave
    } else {
        EditMove::Stay
    }
}

/// The refusal's mark at the cell's end, saying why on hover.
fn alert(ui: &mut Ui, t: &Tokens, cell: Rect, error: Option<&str>) {
    if let Some(error) = error {
        let place = Rect::from_center_size(pos2(cell.right() - 20.0, cell.center().y), vec2(20.0, 20.0));
        let hit = ui.interact(
            place,
            ui.id().with(("refused", cell.min.x as i32, cell.min.y as i32)),
            Sense::hover(),
        );
        icons::paint(
            ui.painter(),
            place.center(),
            Icon::CircleAlert,
            16.0,
            t.color("critical-ink"),
        );
        super::tooltip_shown(hit, error.to_owned());
    }
}

/// The open date editor over `cell`: the day typed as `YYYY-MM-DD`, a button that clears it and one
/// that opens a calendar under the cell.
pub(crate) fn date(
    ui: &mut Ui,
    cell: Rect,
    open: &mut OpenCell,
    id: Id,
    week_start: WeekStart,
    today: Day,
    words: &EditWords,
) -> EditMove {
    let t = tokens(ui.ctx());
    let fresh = std::mem::take(&mut open.fresh);
    ui.painter().rect_filled(cell, CornerRadius::ZERO, t.color("field"));
    let moved = if open.calendar {
        // the calendar's own Escape closes it and leaves the field open
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            open.calendar = false;
        }
        EditMove::Stay
    } else {
        keys(ui)
    };
    // the buttons at the end: clear, then the calendar, 24 square, 12 from the cell's end
    let pick = Rect::from_min_size(pos2(cell.right() - 36.0, cell.center().y - 12.0), vec2(24.0, 24.0));
    let clear = pick.translate(vec2(-28.0, 0.0));
    let inner = Rect::from_min_max(
        pos2(cell.left() + 12.0, cell.top()),
        pos2(clear.left() - 4.0, cell.bottom()),
    );
    if bare_field(ui, &t, inner, &mut open.draft, id.with("field"), fresh).changed() {
        open.error = None;
    }
    for (place, icon, label, which) in [
        (clear, Icon::X, &words.clear, 0),
        (pick, Icon::Calendar, &words.pick_date, 1),
    ] {
        let hit = ui.interact(place, id.with(("button", which)), Sense::click());
        hit.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
        if hit.hovered() || (which == 1 && open.calendar) {
            ui.painter().rect_filled(place, t.radius("control"), t.color("hover"));
        }
        icons::paint(
            ui.painter(),
            place.center(),
            icon,
            if which == 0 { 14.0 } else { 16.0 },
            t.color("muted-foreground"),
        );
        if hit.clicked() {
            if which == 0 {
                open.draft.clear();
                open.error = None;
            } else {
                open.calendar = !open.calendar;
                if open.calendar {
                    calendar_forget(ui.ctx(), id.with("calendar"));
                }
            }
        }
    }
    underline(ui, &t, cell, open.error.is_some());

    // the calendar under the cell, 3 below it and 1 out to its edge
    let mut inside = vec![cell];
    if open.calendar {
        let mut chosen = Day::parse(open.draft.trim());
        let frame = popover_frame(&t);
        let area = Area::new(id.with("calendar-area"))
            .order(Order::Foreground)
            .fixed_pos(pos2(cell.left() - 1.0, cell.bottom() + 3.0))
            .show(ui.ctx(), |ui| {
                frame
                    .show(ui, |ui| {
                        ui.add(Calendar::new(id.with("calendar"), &mut chosen, today).week_start(week_start))
                    })
                    .inner
            });
        inside.push(area.response.rect);
        if area.inner.changed()
            && let Some(day) = chosen
        {
            open.draft = day.to_string();
            open.error = None;
            open.calendar = false;
        }
    }
    if moved != EditMove::Stay {
        moved
    } else if pressed_outside(ui, &inside) {
        EditMove::Leave
    } else {
        EditMove::Stay
    }
}

/// The popover's frame: its ground, a faint edge, the surface's corners and a soft shadow.
fn popover_frame(t: &Tokens) -> Frame {
    Frame::new()
        .fill(t.color("popover"))
        .stroke(Stroke::new(1.0, t.color_at("foreground", 0.1)))
        .corner_radius(t.radius("surface"))
        .shadow(Shadow {
            offset: [0, 4],
            blur: 12,
            spread: 0,
            color: egui::Color32::from_black_alpha(if t.dark { 90 } else { 24 }),
        })
}

/// How tall a select's option is.
const OPTION: f32 = 30.0;

/// The open select over `cell`: the cell as the select's trigger, and its list over it with the
/// chosen option where the cell's value was. A choice is the draft; the move is `Save`.
pub(crate) fn select(ui: &mut Ui, cell: Rect, open: &mut OpenCell, id: Id, options: &[String]) -> EditMove {
    let t = tokens(ui.ctx());
    let chosen = options.iter().position(|o| *o == open.draft);
    if std::mem::take(&mut open.fresh) {
        open.highlight = chosen.unwrap_or(0);
    }
    ui.painter().rect_filled(cell, CornerRadius::ZERO, t.color("field"));
    underline(ui, &t, cell, open.error.is_some());
    let mut moved = ui.input_mut(|i| {
        if i.consume_key(egui::Modifiers::NONE, Key::Escape) {
            EditMove::Cancel
        } else if i.consume_key(egui::Modifiers::NONE, Key::Enter) {
            EditMove::Save
        } else if i.consume_key(egui::Modifiers::NONE, Key::Tab) {
            EditMove::Next(true)
        } else {
            EditMove::Stay
        }
    });
    ui.input_mut(|i| {
        if i.consume_key(egui::Modifiers::NONE, Key::ArrowDown) {
            open.highlight = (open.highlight + 1).min(options.len().saturating_sub(1));
        }
        if i.consume_key(egui::Modifiers::NONE, Key::ArrowUp) {
            open.highlight = open.highlight.saturating_sub(1);
        }
    });
    if moved == EditMove::Save
        && let Some(option) = options.get(open.highlight)
    {
        open.draft = option.clone();
    }
    // the list: 5 in from the cell, 10 wider, the chosen option over the cell's middle
    let width = cell.width() + 10.0;
    let top = cell.center().y - (chosen.unwrap_or(0) as f32 + 0.5) * OPTION - 1.0;
    let mut picked = None;
    let area = Area::new(id.with("list"))
        .order(Order::Foreground)
        .fixed_pos(pos2(cell.left() + 5.0, top))
        .show(ui.ctx(), |ui| {
            popover_frame(&t).show(ui, |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                for (index, option) in options.iter().enumerate() {
                    let (row, hit) = ui.allocate_exact_size(vec2(width - 2.0, OPTION), Sense::click());
                    let marked = Some(index) == chosen;
                    hit.widget_info(|| {
                        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, marked, option)
                    });
                    if hit.hovered() {
                        open.highlight = index;
                    }
                    let lit = index == open.highlight;
                    if lit {
                        ui.painter().rect_filled(row, CornerRadius::ZERO, t.color("accent"));
                    }
                    let ink = if lit {
                        t.color("accent-foreground")
                    } else {
                        t.color("popover-foreground")
                    };
                    let galley = WidgetText::from(TextRole::Body.rich(&t, option)).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Truncate),
                        row.width() - 36.0,
                        TextRole::Body.font(&t),
                    );
                    ui.painter().galley(
                        pos2(row.left() + 6.0, row.center().y - galley.size().y / 2.0),
                        galley,
                        ink,
                    );
                    if marked {
                        icons::paint(
                            ui.painter(),
                            pos2(row.right() - 16.0, row.center().y),
                            Icon::Check,
                            16.0,
                            ink,
                        );
                    }
                    if hit.clicked() {
                        picked = Some(index);
                    }
                }
            });
        });
    if let Some(index) = picked {
        open.draft = options[index].clone();
        moved = EditMove::Save;
    }
    if moved == EditMove::Stay && pressed_outside(ui, &[area.response.rect]) {
        // a press outside the list closes it and changes nothing
        moved = EditMove::Cancel;
    }
    moved
}

/// A refused draft at rest in its cell: the draft as typed, the cell washed in the critical tone
/// with its line under it, and the mark saying why.
pub(crate) fn refused(ui: &mut Ui, cell: Rect, draft: &str, error: &str) {
    let t = tokens(ui.ctx());
    ui.painter()
        .rect_filled(cell, CornerRadius::ZERO, t.color_at("critical", 0.08));
    underline(ui, &t, cell, true);
    let galley = WidgetText::from(TextRole::Body.rich(&t, draft)).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        cell.width() - 36.0,
        TextRole::Body.font(&t),
    );
    ui.painter().galley(
        pos2(cell.left(), cell.center().y - galley.size().y / 2.0),
        galley,
        t.color("foreground"),
    );
    alert(ui, &t, cell, Some(error));
}
