//! The documents open in a work area, a tab each, after sokkel-ui's `DocumentTabs`.

use egui::{CornerRadius, Rect, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{Icon, TextRole, icons, tokens};

/// One open document: a note, a file, a query.
#[derive(Clone, Debug)]
pub struct DocumentTab {
    pub id: String,
    pub label: String,
    pub icon: Option<Icon>,
    /// Holds changes not saved yet: a dot stands in the close button's place until the tab is
    /// pointed at.
    pub dirty: bool,
}

impl DocumentTab {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            dirty: false,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn dirty(mut self, dirty: bool) -> Self {
        self.dirty = dirty;
        self
    }
}

/// The words the tabs say.
#[derive(Clone, Debug)]
pub struct DocumentTabsWords {
    /// What the strip is called, for a screen reader.
    pub label: String,
    /// A tab's close button: `Close {label}`.
    pub close: String,
    /// The dot of a tab with changes not saved yet.
    pub unsaved: String,
}

impl Default for DocumentTabsWords {
    fn default() -> Self {
        Self {
            label: "Open documents".into(),
            close: "Close {label}".into(),
            unsaved: "Not saved yet".into(),
        }
    }
}

/// What the tabs were asked this frame.
#[derive(Clone, Debug, Default)]
pub struct DocumentTabsResponse {
    /// A tab pressed, to show.
    pub chosen: Option<String>,
    /// A tab closed, by its close button or a middle press.
    pub closed: Option<String>,
}

/// The documents open in a work area, after sokkel-ui's `DocumentTabs`: 36 tall on the sidebar's
/// ground over an inset rule in the strong border, each tab its icon, its name cut short past 224
/// and a close button (a dot while it holds unsaved changes, until pointed at), ruled at its end.
/// The shown tab stands on the content's ground under a 2 line in the primary ink, its name at
/// medium weight; a middle press closes a tab; `end` stays at the strip's end.
pub struct DocumentTabs<'a> {
    id: egui::Id,
    tabs: &'a [DocumentTab],
    active: Option<&'a str>,
    closable: bool,
    words: DocumentTabsWords,
}

impl<'a> DocumentTabs<'a> {
    pub fn new(id: impl egui::AsIdSalt, tabs: &'a [DocumentTab], active: Option<&'a str>) -> Self {
        Self {
            id: egui::Id::unique(id),
            tabs,
            active,
            closable: false,
            words: DocumentTabsWords::default(),
        }
    }

    /// Tabs close, from their close button or a middle press.
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    pub fn words(mut self, words: DocumentTabsWords) -> Self {
        self.words = words;
        self
    }

    /// Draws the strip, `end` at its end.
    pub fn show(self, ui: &mut Ui, end: impl FnOnce(&mut Ui)) -> DocumentTabsResponse {
        let t = tokens(ui.ctx());
        let mut out = DocumentTabsResponse::default();
        let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::hover());
        ui.painter().rect_filled(strip, CornerRadius::ZERO, t.color("sidebar"));
        // the controls at the end first, so the tabs take the room they leave
        let mut tail = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(strip.shrink2(vec2(6.0, 0.0)))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        tail.spacing_mut().item_spacing.x = 4.0;
        end(&mut tail);
        let taken = tail.min_rect().width();
        let tabs_end = if taken > 1.0 {
            strip.right() - taken - 12.0
        } else {
            strip.right()
        };
        let painter = ui.painter_at(Rect::from_min_max(strip.min, pos2(tabs_end, strip.bottom())));
        let mut x = strip.left();
        let mut shown_span = None;
        for tab in self.tabs {
            let shown = self.active == Some(tab.id.as_str());
            let role = TextRole::Body;
            let rich = if shown {
                role.rich(&t, &tab.label).variation("wght", 500.0)
            } else {
                role.rich(&t, &tab.label)
            };
            let lead: f32 = if tab.icon.is_some() { 16.0 + 6.0 } else { 0.0 };
            let close = if self.closable { 6.0 + 20.0 } else { 0.0 };
            let pad_end = if self.closable { 4.0 } else { 12.0 };
            let galley = WidgetText::from(rich).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                (224.0 - 12.0 - lead - pad_end - close).max(1.0),
                role.font(&t),
            );
            let width = 12.0 + lead + galley.size().x + pad_end + close;
            let rect = Rect::from_min_size(pos2(x, strip.top()), vec2(width, strip.height()));
            let hit = ui.interact(rect, self.id.with(("tab", &tab.id)), Sense::click());
            hit.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, shown, &tab.label));
            let ink = if shown || hit.hovered() {
                t.color("foreground")
            } else {
                t.color("muted-foreground")
            };
            if shown {
                shown_span = Some(rect.x_range());
                painter.rect_filled(rect, CornerRadius::ZERO, t.color("background"));
                painter.hline(rect.x_range(), rect.top() + 1.0, Stroke::new(2.0, t.color("primary")));
            } else if hit.hovered() {
                painter.rect_filled(rect, CornerRadius::ZERO, t.color("hover"));
            }
            painter.vline(rect.right() - 0.5, rect.y_range(), Stroke::new(1.0, t.color("border")));
            let mut at = rect.left() + 12.0;
            if let Some(icon) = tab.icon {
                icons::paint(&painter, pos2(at + 8.0, rect.center().y), icon, 16.0, ink);
                at += lead;
            }
            painter.galley(pos2(at, rect.center().y - galley.size().y / 2.0), galley, ink);
            if self.closable {
                let slot = Rect::from_center_size(pos2(rect.right() - 6.0 - 10.0, rect.center().y), vec2(20.0, 20.0));
                let said = self.words.close.replace("{label}", &tab.label);
                let button = ui.interact(slot, self.id.with(("close", &tab.id)), Sense::click());
                button.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &said));
                let pointed = hit.hovered() || button.hovered();
                if tab.dirty && !pointed {
                    painter.circle_filled(slot.center(), 4.0, t.color("foreground"));
                } else if shown || pointed {
                    if button.hovered() {
                        painter.rect_filled(slot, t.radius("sm"), t.color("hover"));
                    }
                    let tone = if button.hovered() {
                        "foreground"
                    } else {
                        "muted-foreground"
                    };
                    icons::paint(&painter, slot.center(), Icon::X, 14.0, t.color(tone));
                }
                if button.clicked() {
                    out.closed = Some(tab.id.clone());
                }
            }
            if hit.clicked() && out.closed.is_none() {
                out.chosen = Some(tab.id.clone());
            }
            if self.closable && hit.middle_clicked() {
                out.closed = Some(tab.id.clone());
            }
            x = rect.right();
        }
        // the strip's rule, which the shown tab's ground covers so the tab joins the content
        let rule = strip.bottom() - 0.5;
        let line = Stroke::new(1.0, t.color("border-strong"));
        match shown_span {
            Some(span) => {
                ui.painter().hline(strip.left()..=span.min, rule, line);
                ui.painter().hline(span.max..=strip.right(), rule, line);
            }
            None => {
                ui.painter().hline(strip.x_range(), rule, line);
            }
        }
        out
    }
}
