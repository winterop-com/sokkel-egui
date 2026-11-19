//! The channel a video comes from, under the video, after sokkel-ui-media's `ChannelRow` and the
//! core's `ProfileRow` it is made of.

use egui::{Align, Layout, Sense, Stroke, Ui, UiBuilder, pos2, vec2};

use sokkel_egui::numbers::compact_number;
use sokkel_egui::{AvatarSize, Button, ButtonSize, ButtonVariant, Icon, PersonAvatar, TextRole, tokens};

/// The words the row says.
#[derive(Clone, Debug)]
pub struct ChannelRowWords {
    pub follow: String,
    pub following: String,
    /// The line under the name: `{count} followers`, `{count} follower` for one.
    pub followers: String,
    pub follower: String,
}

impl Default for ChannelRowWords {
    fn default() -> Self {
        Self {
            follow: "Follow".into(),
            following: "Following".into(),
            followers: "{count} followers".into(),
            follower: "{count} follower".into(),
        }
    }
}

/// What the row was asked this frame.
#[derive(Clone, Debug, Default)]
pub struct ChannelRowResponse {
    /// The picture or the name was pressed, to open the channel's page (with `linked`).
    pub opened: bool,
    /// Follow or Following was pressed: following or not from now on.
    pub following: Option<bool>,
}

/// How wide the row must be for the actions to stand at its end beside the channel, as the web's
/// `@2xl/channel`.
const BESIDE: f32 = 672.0;

type Part<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// The channel a video comes from, after sokkel-ui-media's `ChannelRow`: its picture (36 across)
/// and its name in `text.label` over how many follow it (written short, in `text.help`, muted), 12
/// apart; then Follow, a small primary button that turns to Following with a check on the secondary
/// ground once pressed, 16 after the name. The viewer's actions on the video (`MediaActions`) stand
/// at the line's end, 24 after, while the row is 672 wide or more, and on a line of their own 12
/// under it on a narrower one. `large` heads the channel's own page: a picture 64 across, the name
/// as the page's title and a button of the regular size, 16 apart.
#[must_use = "show it with `.show(ui)`"]
pub struct ChannelRow<'a> {
    name: String,
    color: Option<String>,
    detail: Option<String>,
    followers: Option<f64>,
    following: bool,
    follow: bool,
    linked: bool,
    large: bool,
    actions: Option<Part<'a>>,
    words: ChannelRowWords,
}

impl<'a> ChannelRow<'a> {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            color: None,
            detail: None,
            followers: None,
            following: false,
            follow: false,
            linked: false,
            large: false,
            actions: None,
            words: ChannelRowWords::default(),
        }
    }

    /// The kind color its picture wears, such as `teal`.
    pub fn color(mut self, kind: impl Into<String>) -> Self {
        self.color = Some(kind.into());
        self
    }

    /// How many follow it, written short under the name.
    pub fn followers(mut self, count: f64) -> Self {
        self.followers = Some(count);
        self
    }

    /// The line under the name in the count's place, such as the count and how many videos.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Shows Follow, pressed or not.
    pub fn following(mut self, following: bool) -> Self {
        self.follow = true;
        self.following = following;
        self
    }

    /// The picture and the name open the channel's page, the name underlined under the pointer.
    pub fn linked(mut self, linked: bool) -> Self {
        self.linked = linked;
        self
    }

    /// The head of the channel's own page.
    pub fn large(mut self, large: bool) -> Self {
        self.large = large;
        self
    }

    /// What the viewer does with what plays, such as `MediaActions`.
    pub fn actions(mut self, actions: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.actions = Some(Box::new(actions));
        self
    }

    pub fn words(mut self, words: ChannelRowWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ChannelRowResponse {
        let t = tokens(ui.ctx());
        let mut out = ChannelRowResponse::default();
        let width = ui.available_width();
        let beside = width >= BESIDE;
        let detail = self.detail.clone().or_else(|| {
            self.followers.map(|count| {
                let word = if count == 1.0 {
                    &self.words.follower
                } else {
                    &self.words.followers
                };
                word.replace("{count}", &compact_number(count))
            })
        });
        let gap = if self.large { 16.0 } else { 12.0 };
        let (name_role, detail_role) = if self.large {
            (TextRole::PageTitle, TextRole::Help)
        } else {
            (TextRole::Label, TextRole::Help)
        };
        let name = egui::WidgetText::from(name_role.rich(&t, &self.name)).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            width,
            name_role.font(&t),
        );
        let detail = detail.map(|detail| {
            egui::WidgetText::from(detail_role.rich(&t, detail)).into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                width,
                detail_role.font(&t),
            )
        });
        // a small row's detail stands on a 16 line, `text-xs leading-4`
        let detail_line = if self.large {
            detail.as_ref().map_or(0.0, |d| d.size().y)
        } else {
            detail.as_ref().map_or(0.0, |_| 16.0)
        };
        let words_height = name.size().y + detail_line;
        let words_width = name.size().x.max(detail.as_ref().map_or(0.0, |d| d.size().x));
        let avatar = if self.large { AvatarSize::Xl } else { AvatarSize::Lg };
        let side: f32 = if self.large { 64.0 } else { 36.0 };
        let height = side.max(words_height);

        let row = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            // the channel: its picture and its words
            let (who, hit) = ui.allocate_exact_size(
                vec2(side + gap + words_width, height),
                if self.linked { Sense::click() } else { Sense::hover() },
            );
            hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, self.linked, &self.name));
            let mut child = ui.new_child(
                UiBuilder::new()
                    .max_rect(egui::Rect::from_min_size(
                        pos2(who.left(), who.center().y - side / 2.0),
                        vec2(side, side),
                    ))
                    .layout(Layout::left_to_right(Align::Center)),
            );
            let mut picture = PersonAvatar::new(&self.name).size(avatar);
            if let Some(kind) = &self.color {
                picture = picture.color(kind);
            }
            child.add(picture);
            let left = who.left() + side + gap;
            let top = who.center().y - words_height / 2.0;
            let name_rect = egui::Rect::from_min_size(pos2(left, top), name.size());
            ui.painter().galley(name_rect.min, name.clone(), t.color("foreground"));
            if self.linked && hit.hovered() {
                ui.painter().hline(
                    name_rect.x_range(),
                    name_rect.bottom() - 2.0,
                    Stroke::new(1.0, t.color("foreground")),
                );
            }
            if let Some(detail) = &detail {
                let y = top + name.size().y + (detail_line - detail.size().y) / 2.0;
                ui.painter()
                    .galley(pos2(left, y), detail.clone(), t.color("muted-foreground"));
            }
            out.opened = self.linked && hit.clicked();
            // Follow, 4 past the row's gap after the name
            if self.follow {
                ui.add_space(gap + 4.0);
                let label = if self.following {
                    &self.words.following
                } else {
                    &self.words.follow
                };
                let mut button = Button::new(label)
                    .size(if self.large {
                        ButtonSize::Default
                    } else {
                        ButtonSize::Sm
                    })
                    .variant(if self.following {
                        ButtonVariant::Secondary
                    } else {
                        ButtonVariant::Default
                    })
                    .pressed(self.following);
                if self.following {
                    button = button.icon(Icon::Check);
                }
                if ui.add(button).clicked() {
                    out.following = Some(!self.following);
                }
            }
            if beside && let Some(actions) = self.actions {
                ui.add_space(24.0);
                let room = ui.available_width();
                // at the line's end: a part laid out from the end, such as `MediaActions`, stands there
                ui.allocate_ui_with_layout(vec2(room, height), Layout::right_to_left(Align::Center), |ui| {
                    ui.set_min_width(room);
                    actions(ui);
                });
                None
            } else {
                self.actions
            }
        });
        if let Some(actions) = row.inner {
            ui.add_space(12.0);
            ui.horizontal(|ui| actions(ui));
        }
        out
    }
}
