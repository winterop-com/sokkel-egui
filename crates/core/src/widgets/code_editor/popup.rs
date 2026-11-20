//! What the editor shows over its text: the words offered while typing, the hover tooltip, and
//! the find and replace bar, each from the tokens as the spec's THEME draws CodeMirror's.

use egui::{
    Align2, Area, Color32, CornerRadius, FontId, Frame, Id, Margin, Order, Pos2, Rect, Response, Sense, Stroke,
    StrokeKind, TextEdit, Ui, Vec2, WidgetInfo, pos2, vec2,
};

use super::{CodeEditorWords, MarkTone};
use crate::{Icon, Tokens, icons};

fn tone_color(t: &Tokens, tone: MarkTone) -> Color32 {
    t.color(match tone {
        MarkTone::Error => "critical",
        MarkTone::Warning => "warning",
        MarkTone::Info => "info",
    })
}

/// The popover's frame: its ground and ink, a 1px border edge, radius-control and elevation-pop.
fn surface(t: &Tokens) -> Frame {
    Frame::new()
        .fill(t.color("popover"))
        .stroke(Stroke::new(1.0, t.color("border")))
        .corner_radius(t.radius("control"))
        .shadow(t.shadow("elevation-pop"))
}

fn xs(t: &Tokens) -> FontId {
    FontId::proportional(t.text("xs").size)
}

/// One part of a hover tooltip.
pub(crate) enum Tip {
    /// A problem's message, after a 3 wide rule in its tone.
    Problem(MarkTone, String),
    /// A schema key's type: muted, in the mono face.
    Detail(String),
    /// Words: what a word is, a key's description.
    Text(String),
}

/// The hover tooltip over `anchor`, above it as CodeMirror's `above: true` places it.
pub(crate) fn tooltip(ui: &Ui, id: Id, anchor: Rect, t: &Tokens, mono: &FontId, parts: &[Tip]) {
    if parts.is_empty() {
        return;
    }
    let ink = t.color("popover-foreground");
    let line = t.text("xs").line_height;
    // above the word, as CodeMirror places it, below where the window has no room above
    let above = anchor.top() - ui.ctx().content_rect().top() > 72.0;
    Area::new(id)
        .order(Order::Tooltip)
        .pivot(if above { Align2::LEFT_BOTTOM } else { Align2::LEFT_TOP })
        .fixed_pos(if above {
            anchor.left_top() - vec2(0.0, 4.0)
        } else {
            anchor.left_bottom() + vec2(0.0, 4.0)
        })
        .constrain(true)
        .interactable(false)
        .show(ui.ctx(), |ui| {
            surface(t).show(ui, |ui| {
                ui.set_max_width(448.0);
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                for part in parts {
                    match part {
                        Tip::Problem(tone, message) => {
                            Frame::new().inner_margin(Margin::symmetric(10, 6)).show(ui, |ui| {
                                let text = egui::RichText::new(message)
                                    .font(xs(t))
                                    .color(ink)
                                    .line_height(Some(line));
                                let r = ui.label(text);
                                let rule = Rect::from_min_max(
                                    pos2(ui.min_rect().left() - 10.0, r.rect.top() - 6.0),
                                    pos2(ui.min_rect().left() - 7.0, r.rect.bottom() + 6.0),
                                );
                                ui.painter().rect_filled(rule, 0.0, tone_color(t, *tone));
                            });
                        }
                        Tip::Detail(words) => {
                            Frame::new()
                                .inner_margin(Margin {
                                    left: 10,
                                    right: 10,
                                    top: 6,
                                    bottom: 0,
                                })
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(words)
                                            .font(crate::mono_font(ui.ctx(), mono.size.min(t.text("xs").size), 400.0))
                                            .color(t.color("muted-foreground")),
                                    );
                                });
                        }
                        Tip::Text(words) => {
                            Frame::new().inner_margin(Margin::symmetric(10, 6)).show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(words)
                                        .font(xs(t))
                                        .color(ink)
                                        .line_height(Some(line)),
                                );
                            });
                        }
                    }
                }
            });
        });
}

/// One word offered.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Offer {
    pub label: String,
    pub detail: Option<String>,
    pub info: Option<String>,
    pub kind: Option<String>,
    pub deprecated: bool,
}

fn kind_icon(kind: Option<&str>) -> Icon {
    match kind.unwrap_or("text") {
        "function" | "method" => Icon::SquareFunction,
        "keyword" => Icon::Key,
        "variable" => Icon::Variable,
        "property" => Icon::Box,
        "constant" => Icon::Pi,
        "type" | "class" => Icon::Type,
        "enum" => Icon::List,
        "link" => Icon::Link,
        _ => Icon::LetterText,
    }
}

/// The letters of a label matched by what is typed, in order, as byte ranges: the matched
/// letters are drawn at 600.
pub(crate) fn matched(label: &str, typed: &str) -> Option<Vec<(usize, usize)>> {
    if typed.is_empty() {
        return Some(Vec::new());
    }
    let lower = label.to_lowercase();
    let want = typed.to_lowercase();
    if lower.len() == label.len()
        && let Some(at) = lower.find(&want)
    {
        return Some(vec![(at, at + want.len())]);
    }
    let mut out = Vec::new();
    let mut wanted = want.chars().peekable();
    for (i, c) in label.char_indices() {
        if let Some(w) = wanted.peek()
            && c.to_lowercase().eq(w.to_lowercase())
        {
            out.push((i, i + c.len_utf8()));
            wanted.next();
        }
    }
    wanted.peek().is_none().then_some(out)
}

/// The list of words offered below the caret; the row pressed, when one is.
#[allow(clippy::too_many_arguments)]
pub(crate) fn completion(
    ui: &Ui,
    id: Id,
    at: Pos2,
    t: &Tokens,
    mono: &FontId,
    bold: &FontId,
    offers: &[Offer],
    typed: &str,
    selected: usize,
) -> Option<usize> {
    let mut pressed = None;
    let muted = t.color("muted-foreground");
    let row_height = ui.fonts_mut(|f| f.row_height(mono)) + 4.0;
    // each row's galleys, to size the list to its widest
    let rows: Vec<_> = offers
        .iter()
        .enumerate()
        .map(|(i, offer)| {
            let chosen = i == selected;
            let ink = if offer.deprecated {
                muted
            } else if chosen {
                t.color("accent-foreground")
            } else {
                t.color("popover-foreground")
            };
            let mut job = egui::text::LayoutJob::default();
            let marks = matched(&offer.label, typed).unwrap_or_default();
            let mut last = 0;
            let format = |font: &FontId| egui::TextFormat {
                font_id: font.clone(),
                color: ink,
                strikethrough: if offer.deprecated {
                    Stroke::new(1.0, muted)
                } else {
                    Stroke::NONE
                },
                ..Default::default()
            };
            for (a, b) in marks {
                job.append(&offer.label[last..a], 0.0, format(mono));
                job.append(&offer.label[a..b], 0.0, format(bold));
                last = b;
            }
            job.append(&offer.label[last..], 0.0, format(mono));
            let label = ui.fonts_mut(|f| f.layout_job(job));
            let detail = offer.detail.as_ref().map(|d| {
                ui.fonts_mut(|f| {
                    f.layout_no_wrap(
                        d.clone(),
                        mono.clone(),
                        if chosen { t.color("accent-foreground") } else { muted },
                    )
                })
            });
            (label, detail)
        })
        .collect();
    let icon = mono.size;
    let width = rows
        .iter()
        .map(|(l, d)| 8.0 + icon + 6.0 + l.size().x + d.as_ref().map_or(0.0, |d| 12.0 + d.size().x) + 8.0)
        .fold(160.0_f32, f32::max)
        .min(700.0);
    let list = Area::new(id)
        .order(Order::Foreground)
        .fixed_pos(at)
        .constrain(true)
        .show(ui.ctx(), |ui| {
            surface(t).show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                egui::ScrollArea::vertical()
                    .max_height(256.0)
                    .auto_shrink([true, true])
                    .show(ui, |ui| {
                        for (i, (label, detail)) in rows.iter().enumerate() {
                            let (rect, response) = ui.allocate_exact_size(vec2(width, row_height), Sense::click());
                            let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                            response.widget_info(|| {
                                WidgetInfo::selected(egui::Role::ListBoxOption, true, i == selected, &offers[i].label)
                            });
                            if i == selected {
                                ui.painter().rect_filled(rect, 0.0, t.color("accent"));
                                ui.scroll_to_rect(rect, None);
                            }
                            let ink = if i == selected {
                                t.color("accent-foreground")
                            } else {
                                muted
                            };
                            icons::paint(
                                ui.painter(),
                                pos2(rect.left() + 8.0 + icon / 2.0, rect.center().y),
                                kind_icon(offers[i].kind.as_deref()),
                                icon * 0.9,
                                ink,
                            );
                            let x = rect.left() + 8.0 + icon + 6.0;
                            ui.painter()
                                .galley(pos2(x, rect.center().y - label.size().y / 2.0), label.clone(), ink);
                            if let Some(d) = detail {
                                ui.painter().galley(
                                    pos2(x + label.size().x + 12.0, rect.center().y - d.size().y / 2.0),
                                    d.clone(),
                                    muted,
                                );
                            }
                            if response.clicked() {
                                pressed = Some(i);
                            }
                        }
                    });
            });
        });
    // the chosen word's docs in a box beside the list
    if let Some(info) = offers.get(selected).and_then(|o| o.info.as_ref()) {
        let beside = list.response.rect;
        Area::new(id.with("info"))
            .order(Order::Foreground)
            .fixed_pos(beside.right_top() + vec2(4.0, 0.0))
            .constrain(true)
            .interactable(false)
            .show(ui.ctx(), |ui| {
                surface(t).inner_margin(Margin::symmetric(10, 6)).show(ui, |ui| {
                    ui.set_max_width(384.0);
                    ui.label(
                        egui::RichText::new(info)
                            .font(xs(t))
                            .color(t.color("popover-foreground"))
                            .line_height(Some(t.text("xs").line_height)),
                    );
                });
            });
    }
    pressed
}

/// The find bar's state, kept between frames.
#[derive(Clone, Debug, Default)]
pub(crate) struct SearchBar {
    pub query: super::edit::Query,
    pub replace: String,
    /// Put the focus in the find field on the next frame.
    pub focus: bool,
    /// The match gone to last, washed at 48 percent: egui keeps no selection in a text it has
    /// not focused, so the bar keeps its own.
    pub current: Option<(usize, usize)>,
}

/// What the find bar asks for.
#[derive(Default)]
pub(crate) struct SearchAsked {
    pub next: bool,
    pub previous: bool,
    pub replace: bool,
    pub replace_all: bool,
    pub close: bool,
}

/// A small button of the bar: background, a border edge, accent under the pointer.
fn bar_button(ui: &mut Ui, t: &Tokens, words: &str) -> Response {
    let font = xs(t);
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(words.to_owned(), font, t.color("foreground")));
    let size = galley.size() + vec2(16.0, 4.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, words));
    let ground = if response.hovered() {
        t.color("accent")
    } else {
        t.color("background")
    };
    ui.painter().rect(
        rect,
        t.radius("control"),
        ground,
        Stroke::new(1.0, t.color("border")),
        StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, t.color("foreground"));
    response
}

fn bar_check(ui: &mut Ui, t: &Tokens, on: &mut bool, words: &str) {
    let font = xs(t);
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(words.to_owned(), font, t.color("foreground")));
    let size = vec2(12.0 + 4.0 + galley.size().x, galley.size().y.max(12.0));
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
    }
    let state = *on;
    response.widget_info(|| WidgetInfo::selected(egui::Role::CheckBox, true, state, words));
    let boxed = Rect::from_center_size(pos2(rect.left() + 6.0, rect.center().y), vec2(12.0, 12.0));
    if state {
        ui.painter().rect_filled(boxed, t.radius("mini"), t.color("primary"));
        icons::paint(
            ui.painter(),
            boxed.center(),
            Icon::Check,
            10.0,
            t.color("primary-foreground"),
        );
    } else {
        ui.painter().rect(
            boxed,
            t.radius("mini"),
            t.color("field"),
            Stroke::new(1.0, t.color("input")),
            StrokeKind::Inside,
        );
    }
    ui.painter().galley(
        pos2(boxed.right() + 4.0, rect.center().y - galley.size().y / 2.0),
        galley,
        t.color("foreground"),
    );
}

fn bar_field(ui: &mut Ui, t: &Tokens, id: Id, text: &mut String, name: &str, focus: bool) -> Response {
    let font = xs(t);
    let output = TextEdit::singleline(text)
        .id(id)
        .font(font)
        .text_color(t.color("foreground"))
        .hint_text(egui::RichText::new(name).color(t.color("muted-foreground")).font(xs(t)))
        .desired_width(160.0)
        .return_key(None)
        .frame(Frame::NONE.inner_margin(Margin::symmetric(8, 2)))
        .show(ui);
    let response = output.response.response.clone().accessible_name(name);
    let edge = if response.has_focus() {
        t.color("ring")
    } else {
        t.color("input")
    };
    let rect = response.rect;
    // the field's ground behind its text, its edge over it
    ui.painter()
        .rect_stroke(rect, t.radius("control"), Stroke::new(1.0, edge), StrokeKind::Inside);
    if focus {
        response.request_focus();
    }
    response
}

/// The find and replace bar over the text, on muted, its fields and buttons in text.xs; the
/// height it takes and what it asks for.
pub(crate) fn search_bar(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    t: &Tokens,
    bar: &mut SearchBar,
    words: &CodeEditorWords,
    read_only: bool,
) -> (f32, SearchAsked) {
    let mut asked = SearchAsked::default();
    let ground = ui.painter().add(egui::Shape::Noop);
    let inner = Rect::from_min_max(rect.min + vec2(8.0, 6.0), pos2(rect.right() - 32.0, rect.bottom()));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.spacing_mut().item_spacing = vec2(8.0, 4.0);
    // fields and buttons as tall as their text and padding, not a control
    child.spacing_mut().interact_size.y = 0.0;
    let field_ground = |ui: &mut Ui| ui.painter().add(egui::Shape::Noop);
    let mut grounds = Vec::new();
    child.horizontal_wrapped(|ui| {
        let g = field_ground(ui);
        let find = bar_field(
            ui,
            t,
            id.with("find"),
            &mut bar.query.search,
            &words.find,
            std::mem::take(&mut bar.focus),
        );
        grounds.push((g, find.rect));
        if find.has_focus() {
            let (enter, shift_enter, escape) = ui.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                    i.consume_key(egui::Modifiers::SHIFT, egui::Key::Enter),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
                )
            });
            asked.next |= enter && !shift_enter;
            asked.previous |= shift_enter;
            asked.close |= escape;
        }
        asked.next |= bar_button(ui, t, &words.next).clicked();
        asked.previous |= bar_button(ui, t, &words.previous).clicked();
        bar_check(ui, t, &mut bar.query.case, &words.match_case);
        bar_check(ui, t, &mut bar.query.regexp, &words.regexp);
        bar_check(ui, t, &mut bar.query.word, &words.by_word);
        if !read_only {
            ui.end_row();
            let g = field_ground(ui);
            let replace = bar_field(ui, t, id.with("replace"), &mut bar.replace, &words.replace, false);
            grounds.push((g, replace.rect));
            if replace.has_focus() && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                asked.close = true;
            }
            asked.replace |= bar_button(ui, t, &words.replace_one).clicked();
            asked.replace_all |= bar_button(ui, t, &words.replace_all).clicked();
        }
    });
    for (index, rect) in grounds {
        child.painter().set(
            index,
            egui::Shape::rect_filled(rect, t.radius("control"), t.color("field")),
        );
    }
    let height = child.min_rect().bottom() - rect.top() + 6.0;
    let bar_rect = Rect::from_min_size(rect.min, vec2(rect.width(), height));
    // the close, in the 32 at the bar's end
    let close_rect = Rect::from_center_size(
        pos2(bar_rect.right() - 16.0, bar_rect.top() + 6.0 + 9.0),
        vec2(18.0, 18.0),
    );
    let close = ui.interact(close_rect, id.with("close"), Sense::click());
    close.widget_info(|| WidgetInfo::labeled(egui::Role::Button, true, &words.close));
    let close_ink = if close.hovered() {
        t.color("foreground")
    } else {
        t.color("muted-foreground")
    };
    icons::paint(ui.painter(), close_rect.center(), Icon::X, t.text("sm").size, close_ink);
    asked.close |= close.clicked();
    ui.painter().set(
        ground,
        egui::Shape::Vec(vec![
            egui::Shape::rect_filled(bar_rect, CornerRadius::ZERO, t.color("muted")),
            egui::Shape::line_segment(
                [
                    bar_rect.left_bottom() - vec2(0.0, 0.5),
                    bar_rect.right_bottom() - vec2(0.0, 0.5),
                ],
                Stroke::new(1.0, t.color("border")),
            ),
        ]),
    );
    (height, asked)
}
