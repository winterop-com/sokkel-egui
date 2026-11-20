//! A row of a list that is not a table, and the list of them, after sokkel-ui's `ListRow` and
//! `ListRows`.

use egui::{Rect, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use super::{AvatarSize, Chip, ChipTone, PersonAvatar};
use crate::{Icon, TextRole, icons, tokens};

/// What stands before a row's two lines.
#[derive(Clone, Debug)]
pub enum RowMark {
    /// An icon in the muted ink, or in a color token's.
    Icon(Icon, Option<&'static str>),
    /// A person's avatar, small.
    Person(String),
    /// A dot in a color token's color, such as a state's.
    Dot(&'static str),
}

/// What stands at a row's end.
#[derive(Clone, Debug)]
pub enum RowAside {
    /// Muted words, such as a time.
    Text(String),
    /// A chip, such as a state.
    Chip(String, ChipTone),
}

/// One row of a list that is not a table, as sokkel-ui's `ListRow`: a title over a muted second
/// line, a mark before them and an aside at the end. A pressable row is a button the full width of
/// the list; `current` marks the one whose detail is open. Both lines end in an ellipsis in the
/// room they have, or wrap with `wrap`. Rows go in `ListRows`.
#[derive(Clone, Debug)]
pub struct ListRow {
    title: String,
    detail: Option<String>,
    mark: Option<RowMark>,
    aside: Option<RowAside>,
    pressable: bool,
    current: bool,
    wrap: bool,
    mono: bool,
}

impl ListRow {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            detail: None,
            mark: None,
            aside: None,
            pressable: false,
            current: false,
            wrap: false,
            mono: false,
        }
    }

    /// The second line under the title, in the muted ink.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn mark(mut self, mark: RowMark) -> Self {
        self.mark = Some(mark);
        self
    }

    pub fn aside(mut self, aside: RowAside) -> Self {
        self.aside = Some(aside);
        self
    }

    /// The whole row is a button that opens what it names.
    pub fn pressable(mut self, pressable: bool) -> Self {
        self.pressable = pressable;
        self
    }

    /// The row whose detail is open, on the accent's wash.
    pub fn current(mut self, current: bool) -> Self {
        self.current = current;
        self
    }

    /// The title and the detail wrap onto more lines rather than end in an ellipsis.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// The title in the code face, for a code value: a job, a host, a key.
    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    /// Draws the row `inset` in from each side, `width` wide.
    fn show(self, ui: &mut Ui, inset: f32) -> egui::Response {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        let mark_width = match &self.mark {
            None => 0.0,
            Some(RowMark::Person(_)) => 24.0,
            Some(_) => 16.0,
        };
        let aside = self.aside.as_ref().map(|aside| match aside {
            RowAside::Text(text) => {
                let galley = WidgetText::from(TextRole::Help.rich(&t, text)).into_galley(
                    ui,
                    Some(TextWrapMode::Extend),
                    f32::INFINITY,
                    TextRole::Help.font(&t),
                );
                (galley.size().x, Some(galley))
            }
            RowAside::Chip(label, _) => {
                let galley = WidgetText::from(TextRole::Badge.rich(&t, label)).into_galley(
                    ui,
                    Some(TextWrapMode::Extend),
                    f32::INFINITY,
                    TextRole::Badge.font(&t),
                );
                (galley.size().x + 12.0, None)
            }
        });
        let gap = 12.0;
        let room = (width
            - 2.0 * inset
            - if mark_width > 0.0 { mark_width + gap } else { 0.0 }
            - aside.as_ref().map_or(0.0, |(w, _)| w + gap))
        .max(1.0);
        let mode = if self.wrap {
            TextWrapMode::Wrap
        } else {
            TextWrapMode::Truncate
        };
        let title = if self.mono {
            WidgetText::from(egui::RichText::new(&self.title).font(crate::mono_font(
                ui.ctx(),
                crate::mono_small_size(&t, TextRole::Help.font(&t).size),
                400.0,
            )))
            .into_galley(ui, Some(mode), room, TextRole::Help.font(&t))
        } else {
            WidgetText::from(TextRole::Body.rich(&t, &self.title)).into_galley(
                ui,
                Some(mode),
                room,
                TextRole::Body.font(&t),
            )
        };
        let detail = self.detail.as_ref().map(|d| {
            WidgetText::from(TextRole::Help.rich(&t, d)).into_galley(ui, Some(mode), room, TextRole::Help.font(&t))
        });
        let lines = title.size().y + detail.as_ref().map_or(0.0, |d| d.size().y);
        let height = lines.max(mark_width) + 16.0;
        let sense = if self.pressable { Sense::click() } else { Sense::hover() };
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), sense);
        response.widget_info(|| {
            let name = match &self.detail {
                Some(detail) => format!("{}, {detail}", self.title),
                None => self.title.clone(),
            };
            egui::WidgetInfo::labeled(
                if self.pressable {
                    egui::Role::Button
                } else {
                    egui::Role::Label
                },
                true,
                name,
            )
        });
        let radius = if inset > 10.0 {
            egui::CornerRadius::ZERO
        } else {
            t.radius("control")
        };
        if self.current {
            ui.painter().rect_filled(rect, radius, t.color("accent"));
        } else if self.pressable && response.hovered() {
            ui.painter().rect_filled(rect, radius, t.color("hover"));
        }
        let ink = if self.current {
            t.color("accent-foreground")
        } else {
            t.color("foreground")
        };
        let muted = t.color("muted-foreground");
        let mut x = rect.left() + inset;
        let middle = rect.center().y;
        match &self.mark {
            Some(RowMark::Icon(icon, tone)) => {
                let color = tone.map_or(muted, |k| t.color(k));
                icons::paint(ui.painter(), pos2(x + 8.0, middle), *icon, 16.0, color);
            }
            Some(RowMark::Dot(tone)) => {
                ui.painter().circle_filled(pos2(x + 8.0, middle), 4.0, t.color(tone));
            }
            Some(RowMark::Person(name)) => {
                let spot = Rect::from_center_size(pos2(x + 12.0, middle), vec2(24.0, 24.0));
                // in a child of its own, so the avatar leaves the list's cursor where the row put it
                ui.new_child(egui::UiBuilder::new().max_rect(spot))
                    .add(PersonAvatar::new(name.clone()).size(AvatarSize::Sm));
            }
            None => {}
        }
        if mark_width > 0.0 {
            x += mark_width + gap;
        }
        let top = middle - lines / 2.0;
        let title_height = title.size().y;
        ui.painter().galley(pos2(x, top), title, ink);
        if let Some(detail) = detail {
            ui.painter().galley(pos2(x, top + title_height), detail, muted);
        }
        if let (Some((w, galley)), Some(aside)) = (aside, &self.aside) {
            let left = rect.right() - inset - w;
            match (aside, galley) {
                (RowAside::Text(_), Some(galley)) => {
                    let y = middle - galley.size().y / 2.0;
                    ui.painter().galley(pos2(left, y), galley, muted);
                }
                (RowAside::Chip(label, tone), _) => {
                    let spot = Rect::from_min_size(pos2(left, middle - 10.0), vec2(w, 20.0));
                    ui.new_child(egui::UiBuilder::new().max_rect(spot))
                        .add(Chip::new(label.clone()).tone(*tone));
                }
                _ => {}
            }
        }
        if self.pressable {
            super::focus_ring_inset(ui.painter(), &t, rect, radius, &response);
        }
        response
    }
}

/// A list of `ListRow`s, one under another, as sokkel-ui's `ListRows`. `flush` sets them edge to
/// edge in a panel, lined up with its padding (`panel-pad`) and parted by hairlines.
pub struct ListRows {
    rows: Vec<ListRow>,
    flush: bool,
}

impl ListRows {
    pub fn new(rows: Vec<ListRow>) -> Self {
        Self { rows, flush: false }
    }

    pub fn flush(mut self, flush: bool) -> Self {
        self.flush = flush;
        self
    }

    /// Draws the rows; returns the index of the one pressed this frame.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let t = tokens(ui.ctx());
        let inset = if self.flush { t.density("panel-pad") } else { 10.0 };
        let spacing = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut pressed = None;
        let count = self.rows.len();
        for (index, row) in self.rows.into_iter().enumerate() {
            let response = row.show(ui, inset);
            if response.clicked() {
                pressed = Some(index);
            }
            if self.flush && index + 1 < count {
                ui.painter().hline(
                    response.rect.x_range(),
                    response.rect.bottom() - 0.5,
                    Stroke::new(1.0, t.color("border")),
                );
            }
        }
        ui.spacing_mut().item_spacing.y = spacing;
        pressed
    }
}
