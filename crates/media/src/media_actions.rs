//! What the viewer does with a video or an episode, in one row under it, after sokkel-ui-media's
//! `MediaActions`.

use egui::{Align, Layout, Ui, vec2};

use sokkel_egui::numbers::compact_number;
use sokkel_egui::{Button, ButtonSize, ButtonVariant, DropdownMenu, Icon, IconButton, MenuItem, TextRole, tokens};

/// What the viewer thinks of what plays: liked or disliked; neither is `None`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaRating {
    Like,
    Dislike,
}

/// The words the row says.
#[derive(Clone, Debug)]
pub struct MediaActionsWords {
    pub like: String,
    pub dislike: String,
    pub more: String,
}

impl Default for MediaActionsWords {
    fn default() -> Self {
        Self {
            like: "Like".into(),
            dislike: "Dislike".into(),
            more: "More".into(),
        }
    }
}

type Render<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// One action on what plays, such as Share or Save, which moves into More when the row has no room
/// for it.
pub struct MediaAction<'a> {
    pub label: String,
    pub icon: Option<Icon>,
    pub disabled: bool,
    /// On or off, for an action that toggles, such as Saved: pressed in the row, checked in More.
    pub pressed: Option<bool>,
    /// Drawn in the row in place of the plain button, such as a `DownloadButton`; its entry in More
    /// is the plain one.
    pub render: Option<Render<'a>>,
}

impl<'a> MediaAction<'a> {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            disabled: false,
            pressed: None,
            render: None,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = Some(pressed);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn render(mut self, render: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.render = Some(Box::new(render));
        self
    }
}

/// What the row was asked this frame.
#[derive(Clone, Debug, Default)]
pub struct MediaActionsResponse {
    /// Like or dislike was pressed: the new rating, `Some(None)` when the one held was pressed again.
    pub rating: Option<Option<MediaRating>>,
    /// The action pressed, in the row or in More, by its index.
    pub action: Option<usize>,
    /// The entry of `menu` chosen in More, by its index.
    pub menu: Option<usize>,
}

/// The gap between the buttons.
const GAP: f32 = 8.0;

/// What the viewer does with a video or an episode, after sokkel-ui-media's `MediaActions`: like and
/// dislike joined, the like count on the first written short, then the app's actions as small
/// outlined buttons, 8 apart, and More. Those the row has no room for move from the last into More
/// as entries (checked for a toggle), and come back out as it widens; `menu` holds entries that are
/// always in More. Laid out in a row that runs from its end (a right-to-left layout, as beside a
/// `ChannelRow`'s channel), it stands at the end; otherwise it starts at the left.
#[must_use = "show it with `.show(ui)`"]
pub struct MediaActions<'a> {
    rating: Option<MediaRating>,
    likes: Option<f64>,
    rated: bool,
    actions: Vec<MediaAction<'a>>,
    menu: &'a [MenuItem],
    words: MediaActionsWords,
}

impl<'a> MediaActions<'a> {
    pub fn new() -> Self {
        Self {
            rating: None,
            likes: None,
            rated: false,
            actions: Vec::new(),
            menu: &[],
            words: MediaActionsWords::default(),
        }
    }

    /// Shows the like pair, with what the viewer thinks and how many liked it.
    pub fn rating(mut self, rating: Option<MediaRating>, likes: Option<f64>) -> Self {
        self.rated = true;
        self.rating = rating;
        self.likes = likes;
        self
    }

    pub fn action(mut self, action: MediaAction<'a>) -> Self {
        self.actions.push(action);
        self
    }

    /// Entries always in More, after the actions moved there, such as the stats.
    pub fn menu(mut self, items: &'a [MenuItem]) -> Self {
        self.menu = items;
        self
    }

    pub fn words(mut self, words: MediaActionsWords) -> Self {
        self.words = words;
        self
    }

    /// A small outlined button's width: its label in `text.badge`, the icon's 14 and 4, and 8 or 10
    /// at each side.
    fn width_of(ui: &Ui, label: &str, icon: bool) -> f32 {
        let t = tokens(ui.ctx());
        let galley = egui::WidgetText::from(TextRole::Badge.rich(&t, label)).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            TextRole::Badge.font(&t),
        );
        if icon {
            galley.size().x + 14.0 + 4.0 + 16.0
        } else {
            galley.size().x + 20.0
        }
    }

    pub fn show(self, ui: &mut Ui) -> MediaActionsResponse {
        let t = tokens(ui.ctx());
        let mut out = MediaActionsResponse::default();
        let side = t.density("control-sm");
        let count = self.likes.map(compact_number);
        let pair = if self.rated {
            Self::width_of(ui, count.as_deref().unwrap_or(""), true) + side - 1.0
        } else {
            0.0
        };
        let widths: Vec<f32> = self
            .actions
            .iter()
            .map(|one| Self::width_of(ui, &one.label, one.icon.is_some()))
            .collect();
        let always = !self.menu.is_empty();
        let room =
            ui.available_width() - if self.rated { pair + GAP } else { 0.0 } - if always { side + GAP } else { 0.0 };
        // how many fit: all, or as many as leave room for More after them
        let fit = {
            let total: f32 = widths.iter().sum::<f32>() + GAP * widths.len().saturating_sub(1) as f32;
            if total <= room {
                widths.len()
            } else {
                let more = if always { 0.0 } else { side + GAP };
                let mut used = 0.0;
                let mut n = 0;
                for (index, width) in widths.iter().enumerate() {
                    let next = used + width + if index > 0 { GAP } else { 0.0 };
                    if next + more > room {
                        break;
                    }
                    used = next;
                    n += 1;
                }
                n
            }
        };
        let folded = widths.len() - fit;
        let shows_more = folded > 0 || always;
        let mut parts: Vec<f32> = Vec::new();
        if self.rated {
            parts.push(pair);
        }
        parts.extend_from_slice(&widths[..fit]);
        if shows_more {
            parts.push(side);
        }
        let total = parts.iter().sum::<f32>() + GAP * parts.len().saturating_sub(1) as f32;
        let end = ui.layout().prefer_right_to_left();
        let draw = |ui: &mut Ui, out: &mut MediaActionsResponse| {
            ui.spacing_mut().item_spacing.x = GAP;
            let mut actions = self.actions;
            let mut folded_actions: Vec<(usize, MediaAction)> = Vec::new();
            if self.rated {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.x = -1.0;
                    let liked = self.rating == Some(MediaRating::Like);
                    let disliked = self.rating == Some(MediaRating::Dislike);
                    let like = Button::new(count.clone().unwrap_or_default())
                        .icon(Icon::ThumbsUp)
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm)
                        .pressed(liked);
                    let pressed = ui.add(like).on_hover_text(&self.words.like);
                    if pressed.clicked() {
                        out.rating = Some(if liked { None } else { Some(MediaRating::Like) });
                    }
                    let dislike = ui.add(IconButton::new(Icon::ThumbsDown, &self.words.dislike).outline());
                    if dislike.clicked() {
                        out.rating = Some(if disliked { None } else { Some(MediaRating::Dislike) });
                    }
                });
            }
            for (index, action) in actions.drain(..).enumerate() {
                if index >= fit {
                    folded_actions.push((index, action));
                    continue;
                }
                match action.render {
                    Some(render) => render(ui),
                    None => {
                        let mut button = Button::new(&action.label)
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm)
                            .enabled(!action.disabled);
                        if let Some(icon) = action.icon {
                            button = button.icon(icon);
                        }
                        if let Some(on) = action.pressed {
                            button = button.pressed(on);
                        }
                        if ui.add(button).clicked() {
                            out.action = Some(index);
                        }
                    }
                }
            }
            if shows_more {
                let mut items: Vec<MenuItem> = folded_actions
                    .iter()
                    .map(|(_, action)| {
                        let mut item = MenuItem::action(&action.label).disabled(action.disabled);
                        let icon = match action.pressed {
                            Some(true) => Some(Icon::Check),
                            _ => action.icon,
                        };
                        if let Some(icon) = icon {
                            item = item.icon(icon);
                        }
                        item
                    })
                    .collect();
                let first_menu = items.len() + usize::from(!folded_actions.is_empty() && always);
                if !folded_actions.is_empty() && always {
                    items.push(MenuItem::Separator);
                }
                items.extend(self.menu.iter().cloned());
                let more = ui.add(IconButton::new(Icon::Ellipsis, &self.words.more).outline());
                if let Some(index) = DropdownMenu::new(&items).end(true).width(192.0).show(&more) {
                    if index < folded_actions.len() {
                        out.action = Some(folded_actions[index].0);
                    } else if index >= first_menu {
                        out.menu = Some(index - first_menu);
                    }
                }
            }
        };
        if end {
            ui.allocate_ui_with_layout(vec2(total, side), Layout::left_to_right(Align::Center), |ui| {
                draw(ui, &mut out)
            });
        } else {
            ui.horizontal(|ui| draw(ui, &mut out));
        }
        out
    }
}

impl Default for MediaActions<'_> {
    fn default() -> Self {
        Self::new()
    }
}
