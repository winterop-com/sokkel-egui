//! A dashboard's variables, after sokkel-ui's `VariablesBar`: each a chip of its label and its
//! choice, in a row.

use egui::{Response, Sense, Stroke, StrokeKind, Ui, WidgetText, pos2, vec2};

use crate::{Select, TextRole, tokens};

/// One variable as a chip, as sokkel-ui's `VariablesBar` draws a select: `control-sm` tall in the
/// field's ground and the border, its label on the secondary ground with a rule after it, then the
/// choice at least 96 wide. The select's response says when the choice changed.
pub fn variable_select(ui: &mut Ui, label: &str, chosen: &mut usize, options: &[String]) -> Response {
    let t = tokens(ui.ctx());
    let height = t.density("control-sm");
    let font = TextRole::Help.font(&t);
    let name = WidgetText::from(TextRole::Help.rich(&t, label).variation("wght", 500.0)).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        font.clone(),
    );
    let widest = options
        .iter()
        .map(|o| {
            ui.painter()
                .layout_no_wrap(o.clone(), font.clone(), t.color("foreground"))
                .size()
                .x
        })
        .fold(0.0, f32::max);
    // the choice's words, 8 before and the chevron after, never under 96
    let choice = (8.0 + widest + 6.0 + 14.0 + 6.0).max(96.0);
    let label_width = 8.0 + name.size().x + 8.0;
    let (rect, _) = ui.allocate_exact_size(vec2(label_width + 1.0 + choice, height), Sense::hover());
    let radius = t.radius("control");
    let painter = ui.painter().clone();
    painter.rect_filled(rect, radius, t.color("field"));
    let label_rect = egui::Rect::from_min_size(rect.min, vec2(label_width, height));
    painter.rect_filled(
        label_rect,
        egui::CornerRadius { ne: 0, se: 0, ..radius },
        t.color("secondary"),
    );
    painter.vline(
        label_rect.right() + 0.5,
        rect.y_range(),
        Stroke::new(1.0, t.color("border")),
    );
    painter.galley(
        pos2(label_rect.left() + 8.0, rect.center().y - name.size().y / 2.0),
        name,
        t.color("muted-foreground"),
    );
    let place = egui::Rect::from_min_max(pos2(label_rect.right() + 1.0, rect.top()), rect.max);
    let response = ui.put(
        place,
        Select::new(chosen, options.iter().cloned(), label)
            .small(true)
            .bare(true)
            .width(choice),
    );
    painter.rect_stroke(rect, radius, Stroke::new(1.0, t.color("border")), StrokeKind::Inside);
    response
}
