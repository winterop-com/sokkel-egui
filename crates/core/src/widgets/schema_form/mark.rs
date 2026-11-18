//! The issue mark at the end of a control, after sokkel-ui's `IssueMark`: the control's edge in
//! the worst issue's tone, and a mark that opens what is wrong, what to do, and the fix that puts
//! it right. Nothing is inserted between fields, so a form keeps its height.

use egui::{Align2, Area, Id, Order, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use serde_json::Value;

use super::IssueTone;
use crate::{Icon, TextRole, Tokens, icons, tokens};

/// One issue as a field shows it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Shown {
    pub tone: IssueTone,
    pub text: String,
    pub detail: Option<String>,
    pub fix: Option<Value>,
}

/// The worst tone among the issues, or `None` without any.
pub(crate) fn worst(issues: &[Shown]) -> Option<IssueTone> {
    if issues.iter().any(|i| i.tone == IssueTone::Critical) {
        Some(IssueTone::Critical)
    } else {
        issues.first().map(|_| IssueTone::Warning)
    }
}

/// A tone's name in the tokens: `critical` or `warning`.
pub(crate) fn tone_key(tone: IssueTone) -> &'static str {
    match tone {
        IssueTone::Critical => "critical",
        IssueTone::Warning => "warning",
    }
}

fn tone_icon(tone: IssueTone) -> Icon {
    match tone {
        IssueTone::Critical => Icon::CircleAlert,
        IssueTone::Warning => Icon::TriangleAlert,
    }
}

/// What the mark answers this frame.
#[derive(Default)]
pub(crate) struct MarkOutcome {
    /// The mark was pressed: open it, or shut it when it was open.
    pub toggled: bool,
    /// The popover was left: a press outside it, or Escape.
    pub closed: bool,
    /// The fix chosen from the popover.
    pub fix: Option<Value>,
}

/// The words the mark speaks with.
pub(crate) struct MarkWords<'a> {
    pub critical: &'a str,
    pub warning: &'a str,
    pub use_fix: fn(&Value) -> String,
}

/// Every issue spoken, each after "Error: " or "Warning: ", as the mark's name.
pub(crate) fn spoken(issues: &[Shown], words: &MarkWords<'_>) -> String {
    issues
        .iter()
        .map(|issue| {
            let lead = match issue.tone {
                IssueTone::Critical => words.critical,
                IssueTone::Warning => words.warning,
            };
            match &issue.detail {
                Some(detail) => format!("{lead}{}. {detail}", issue.text),
                None => format!("{lead}{}", issue.text),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The edge a control wears for its worst issue: the tone at 80 percent, as `ISSUE_EDGE`.
pub(crate) fn paint_edge(ui: &Ui, t: &Tokens, control: Rect, tone: IssueTone) {
    ui.painter().rect_stroke(
        control,
        t.radius("control"),
        Stroke::new(1.0, t.color_at(tone_key(tone), 0.8)),
        StrokeKind::Inside,
    );
}

/// The mark in `spot`, a `control-sm` square: the worst tone's icon, the `hover` ground under the
/// pointer and while open, and the popover under it while `open`.
pub(crate) fn issue_mark(
    ui: &mut Ui,
    id: Id,
    spot: Rect,
    issues: &[Shown],
    open: bool,
    words: &MarkWords<'_>,
) -> MarkOutcome {
    let mut out = MarkOutcome::default();
    let Some(tone) = worst(issues) else {
        return out;
    };
    let t = tokens(ui.ctx());
    let hit = ui.interact(spot, id.with("mark"), Sense::click());
    let name = spoken(issues, words);
    hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &name));
    if hit.hovered() || open {
        ui.painter().rect_filled(spot, t.radius("md"), t.color("hover"));
    }
    icons::paint(
        ui.painter(),
        spot.center(),
        tone_icon(tone),
        16.0,
        t.color(&format!("{}-ink", tone_key(tone))),
    );
    super::super::focus_ring(ui.painter(), &t, spot, t.radius("md"), &hit);
    out.toggled = hit.clicked();
    if open && !out.toggled {
        let (area, fix) = popover(ui, &t, id, spot, issues, words);
        out.fix = fix;
        let outside = ui.input(|i| i.pointer.any_pressed())
            && ui
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|p| !area.contains(p) && !spot.contains(p));
        out.closed = outside || ui.input(|i| i.key_pressed(egui::Key::Escape)) || out.fix.is_some();
    }
    out
}

/// What is wrong, under the mark and to its end: each issue's icon, message and detail, and its fix.
fn popover(ui: &Ui, t: &Tokens, id: Id, spot: Rect, issues: &[Shown], words: &MarkWords<'_>) -> (Rect, Option<Value>) {
    let width = 320.0_f32.min(ui.ctx().content_rect().width() - 32.0);
    let mut fix = None;
    let response = Area::new(id.with("popover"))
        .order(Order::Foreground)
        .pivot(Align2::RIGHT_TOP)
        .fixed_pos(Pos2::new(spot.right(), spot.bottom() + 6.0))
        .show(ui.ctx(), |ui| {
            super::super::popup_frame(t).inner_margin(12.0).show(ui, |ui| {
                ui.set_width(width - 24.0);
                ui.spacing_mut().item_spacing.y = 12.0;
                for issue in issues {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            let (icon, _) = ui.allocate_exact_size(vec2(16.0, 20.0), Sense::hover());
                            icons::paint(
                                ui.painter(),
                                pos2(icon.center().x, icon.top() + 10.0),
                                tone_icon(issue.tone),
                                16.0,
                                t.color(&format!("{}-ink", tone_key(issue.tone))),
                            );
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                ui.add(
                                    egui::Label::new(
                                        TextRole::Action
                                            .rich(t, &issue.text)
                                            .color(t.color("popover-foreground")),
                                    )
                                    .wrap(),
                                );
                                if let Some(detail) = &issue.detail {
                                    ui.add(
                                        egui::Label::new(
                                            TextRole::Help.rich(t, detail).color(t.color("muted-foreground")),
                                        )
                                        .wrap(),
                                    );
                                }
                            });
                        });
                        if let Some(value) = &issue.fix {
                            ui.horizontal(|ui| {
                                ui.add_space(24.0);
                                let label = (words.use_fix)(value);
                                if fix_link(ui, t, &label).clicked() {
                                    fix = Some(value.clone());
                                }
                            });
                        }
                    });
                }
            });
        });
    (response.response.rect, fix)
}

/// A fix's action: its words underlined in the `border-strong` ink, the foreground ink under the
/// pointer.
fn fix_link(ui: &mut Ui, t: &Tokens, label: &str) -> egui::Response {
    let galley = egui::WidgetText::from(TextRole::Help.rich(t, label)).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        TextRole::Help.font(t),
    );
    let (rect, hit) = ui.allocate_exact_size(galley.size() + vec2(0.0, 3.0), Sense::click());
    hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    let line = if hit.hovered() {
        t.color("foreground")
    } else {
        t.color("border-strong")
    };
    let text_rect = Rect::from_min_size(rect.min, galley.size());
    ui.painter().galley(text_rect.min, galley, t.color("foreground"));
    ui.painter()
        .hline(text_rect.x_range(), text_rect.bottom() + 1.5, Stroke::new(1.0, line));
    super::super::focus_ring(ui.painter(), t, rect, t.radius("sm"), &hit);
    hit
}

/// The mark's square at the end of a control, `control-sm` wide, 4 in from its end: centered on
/// one line, at its top in a text area.
pub(crate) fn mark_spot(t: &Tokens, control: Rect, top: bool) -> Rect {
    let side = t.density("control-sm");
    let y = if top {
        control.top() + 4.0 + side / 2.0
    } else {
        control.center().y
    };
    Rect::from_center_size(pos2(control.right() - 4.0 - side / 2.0, y), vec2(side, side))
}

/// A fieldset's edge: a 1px `border` box with `radius-surface` corners, open on its top edge
/// between `gap_from` and `gap_to` where the legend sits.
pub(crate) fn fieldset_edge(ui: &Ui, t: &Tokens, rect: Rect, gap_from: f32, gap_to: f32) {
    let r = f32::from(t.radius("surface").nw)
        .min(rect.height() / 2.0)
        .min(rect.width() / 2.0);
    let arc = |center: Pos2, from: f32, to: f32, points: &mut Vec<Pos2>| {
        for step in 0..=8 {
            let a = from + (to - from) * step as f32 / 8.0;
            points.push(center + r * vec2(a.cos(), a.sin()));
        }
    };
    use std::f32::consts::{FRAC_PI_2, PI};
    let rect = rect.shrink(0.5);
    let mut points = vec![pos2(gap_to, rect.top())];
    arc(pos2(rect.right() - r, rect.top() + r), -FRAC_PI_2, 0.0, &mut points);
    arc(pos2(rect.right() - r, rect.bottom() - r), 0.0, FRAC_PI_2, &mut points);
    arc(pos2(rect.left() + r, rect.bottom() - r), FRAC_PI_2, PI, &mut points);
    arc(pos2(rect.left() + r, rect.top() + r), PI, PI + FRAC_PI_2, &mut points);
    points.push(pos2(gap_from, rect.top()));
    ui.painter()
        .add(egui::Shape::line(points, Stroke::new(1.0, t.color("border"))));
}
