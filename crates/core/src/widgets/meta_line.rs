//! Small facts on one line, after sokkel-ui's `MetaLine`.

use egui::{Label, Response, Ui};

use crate::TextRole;

/// Small facts about a thing on one line, joined by a separator: a count and an age, what a
/// service calls on and what calls on it. An empty item is left out, so an optional fact needs no
/// condition around it. Quiet and small by default; `sm` writes it at the text size, `strong` in
/// the foreground and medium weight, and `truncate` keeps it to one line with an ellipsis.
pub struct MetaLine {
    items: Vec<String>,
    separator: String,
    sm: bool,
    strong: bool,
    truncate: bool,
    links: Vec<usize>,
}

/// What a meta line with links was asked this frame.
pub struct MetaLineResponse {
    pub response: Response,
    /// The index of the linked item pressed, among the items kept.
    pub pressed: Option<usize>,
}

impl MetaLine {
    pub fn new<S: Into<String>>(items: impl IntoIterator<Item = S>) -> Self {
        Self {
            items: items
                .into_iter()
                .map(Into::into)
                .filter(|s: &String| !s.is_empty())
                .collect(),
            separator: "·".into(),
            sm: false,
            strong: false,
            truncate: false,
            links: Vec::new(),
        }
    }

    /// The item at `index`, among those kept, is a link, as the web puts a `TextLink` among a meta
    /// line's items: in the link's ink, underlined under the pointer, its press answered by
    /// [`MetaLine::show_links`].
    pub fn link(mut self, index: usize) -> Self {
        self.links.push(index);
        self
    }

    pub fn separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
        self
    }

    /// At the text size, as the web's `size="sm"`.
    pub fn sm(mut self) -> Self {
        self.sm = true;
        self
    }

    /// In the foreground and medium weight, as the web's `tone="strong"`.
    pub fn strong(mut self) -> Self {
        self.strong = true;
        self
    }

    pub fn truncate(mut self) -> Self {
        self.truncate = true;
        self
    }

    /// The facts as one string, for a place that takes words alone, such as a panel's footer.
    pub fn text(&self) -> String {
        self.items.join(&format!(" {} ", self.separator))
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        if !self.links.is_empty() {
            return self.show_links(ui).response;
        }
        let t = crate::tokens(ui.ctx());
        let role = if self.sm { TextRole::Body } else { TextRole::Help };
        let rich = role.rich(&t, self.text());
        let rich = if self.strong {
            rich.variation("wght", t.weight(crate::Weight::Medium))
                .color(t.color("foreground"))
        } else {
            rich.color(t.color("muted-foreground"))
        };
        let label = Label::new(rich);
        ui.add(if self.truncate { label.truncate() } else { label })
    }
}

impl MetaLine {
    /// Draws the line item by item, its links pressable, and answers which was pressed.
    pub fn show_links(self, ui: &mut Ui) -> MetaLineResponse {
        let t = crate::tokens(ui.ctx());
        let role = if self.sm { TextRole::Body } else { TextRole::Help };
        let ink = if self.strong {
            t.color("foreground")
        } else {
            t.color("muted-foreground")
        };
        let rich = |text: &str| {
            let rich = role.rich(&t, text);
            if self.strong {
                rich.variation("wght", t.weight(crate::Weight::Medium))
            } else {
                rich
            }
            .color(ink)
        };
        let mut pressed = None;
        // the line as tall as its words, not a control's height: set before the row is made
        let kept = ui.spacing().interact_size.y;
        ui.spacing_mut().interact_size.y = 0.0;
        let response = ui
            .horizontal(|ui| {
                ui.spacing_mut().interact_size.y = kept;
                ui.spacing_mut().item_spacing.x = 0.0;
                let gap = format!(" {} ", self.separator);
                for (index, item) in self.items.iter().enumerate() {
                    if index > 0 {
                        ui.label(rich(&gap));
                    }
                    if self.links.contains(&index) {
                        if ui.add(crate::TextLink::new(item).role(role)).clicked() {
                            pressed = Some(index);
                        }
                    } else {
                        ui.label(rich(item));
                    }
                }
            })
            .response;
        ui.spacing_mut().interact_size.y = kept;
        MetaLineResponse { response, pressed }
    }
}

#[cfg(test)]
mod tests {
    use super::MetaLine;

    #[test]
    fn an_empty_fact_is_left_out() {
        let line = MetaLine::new(["Calls on queue".to_owned(), String::new(), "called by web".into()]);
        assert_eq!(line.text(), "Calls on queue · called by web");
        assert_eq!(MetaLine::new(["a", "b"]).separator("/").text(), "a / b");
    }
}
