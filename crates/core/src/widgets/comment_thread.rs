//! The comments under a post or a video, after sokkel-ui's `CommentThread` and `CommentComposer`.

use egui::{Align, Layout, Sense, Stroke, StrokeKind, TextEdit, Ui, vec2};

use crate::reactions::Reaction;
use crate::{
    AvatarSize, Button, ButtonSize, ButtonVariant, DropdownMenu, Icon, IconButton, MenuItem, PersonAvatar, TextRole,
    icons, tokens,
};

/// One comment, and the replies to it, nested to any depth.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThreadComment {
    pub id: String,
    pub author: String,
    /// The kind color the author's picture wears, such as `teal`.
    pub color: Option<String>,
    /// When it was written, as the app says it: `3 weeks ago`.
    pub time: String,
    /// The full date and time, on hover.
    pub time_title: Option<String>,
    pub body: String,
    /// How many liked it, as the app writes it: `1.2K`.
    pub likes: Option<String>,
    pub liked: bool,
    pub disliked: bool,
    /// Pinned to the top by the thread's owner, said over the comment.
    pub pinned: bool,
    /// Written by the owner of what is commented on, marked beside the name.
    pub creator: bool,
    pub reactions: Vec<Reaction>,
    pub replies: Vec<ThreadComment>,
}

/// The words a thread says.
#[derive(Clone, Debug)]
pub struct CommentThreadWords {
    pub like: String,
    pub dislike: String,
    pub react: String,
    pub reply: String,
    pub actions: String,
    pub creator: String,
    /// Over a pinned comment: `Pinned by {by}`.
    pub pinned_by: String,
    pub pinned: String,
    /// `{count} replies`, `{count} reply` for one.
    pub replies: String,
    pub reply_one: String,
    pub hide_replies: String,
}

impl Default for CommentThreadWords {
    fn default() -> Self {
        Self {
            like: "Like".into(),
            dislike: "Dislike".into(),
            react: "Add a reaction".into(),
            reply: "Reply".into(),
            actions: "Comment actions".into(),
            creator: "creator".into(),
            pinned_by: "Pinned by {by}".into(),
            pinned: "Pinned".into(),
            replies: "{count} replies".into(),
            reply_one: "{count} reply".into(),
            hide_replies: "Hide replies".into(),
        }
    }
}

/// The words a composer says.
#[derive(Clone, Debug)]
pub struct CommentComposerWords {
    pub placeholder: String,
    pub submit: String,
    pub cancel: String,
}

impl Default for CommentComposerWords {
    fn default() -> Self {
        Self {
            placeholder: "Add a comment".into(),
            submit: "Comment".into(),
            cancel: "Cancel".into(),
        }
    }
}

/// What a composer was asked this frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommentComposerResponse {
    /// Comment was pressed (or Ctrl+Enter): what was written; the field empties.
    pub submit: Option<String>,
    pub cancel: bool,
}

/// A field to write a comment in, after sokkel-ui's `CommentComposer`: beside the writer's picture
/// (32 across, 24 for a reply), 12 apart. Its Cancel and Comment buttons show under it at the end
/// once something is written, or always for a reply's field (`cancellable`).
#[must_use = "show it with `.show(ui)`"]
pub struct CommentComposer {
    id_salt: egui::Id,
    author: String,
    color: Option<String>,
    small: bool,
    cancellable: bool,
    words: CommentComposerWords,
}

impl CommentComposer {
    pub fn new(id_salt: impl egui::AsIdSalt, author: impl Into<String>) -> Self {
        Self {
            id_salt: egui::Id::unique(id_salt),
            author: author.into(),
            color: None,
            small: false,
            cancellable: false,
            words: CommentComposerWords::default(),
        }
    }

    pub fn color(mut self, kind: impl Into<String>) -> Self {
        self.color = Some(kind.into());
        self
    }

    /// A reply's field, under the comment it answers.
    pub fn small(mut self, small: bool) -> Self {
        self.small = small;
        self
    }

    /// Cancel shows always, as a reply's field opened from a comment.
    pub fn cancellable(mut self, on: bool) -> Self {
        self.cancellable = on;
        self
    }

    pub fn words(mut self, words: CommentComposerWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> CommentComposerResponse {
        let t = tokens(ui.ctx());
        let id = ui.make_persistent_id(self.id_salt);
        let mut draft: String = ui.data(|d| d.get_temp(id)).unwrap_or_default();
        let mut out = CommentComposerResponse::default();
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = vec2(12.0, 8.0);
            let mut avatar =
                PersonAvatar::new(&self.author).size(if self.small { AvatarSize::Sm } else { AvatarSize::Md });
            if let Some(kind) = &self.color {
                avatar = avatar.color(kind);
            }
            ui.add(avatar);
            ui.vertical(|ui| {
                let width = ui.available_width();
                let edit = TextEdit::multiline(&mut draft)
                    .id(id.with("field"))
                    .hint_text(
                        TextRole::Body
                            .rich(&t, &self.words.placeholder)
                            .color(t.color("muted-foreground")),
                    )
                    .font(TextRole::Body.font(&t))
                    .desired_rows(1)
                    .desired_width(width - 16.0)
                    .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 5)));
                let field = ui.add(edit);
                let ring = if field.has_focus() {
                    t.color("ring")
                } else {
                    t.color("input")
                };
                ui.painter().rect_stroke(
                    field.rect,
                    t.radius("control"),
                    Stroke::new(1.0, ring),
                    StrokeKind::Inside,
                );
                let send_key = field.has_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter) && (i.modifiers.command || i.modifiers.ctrl));
                if !draft.is_empty() || self.cancellable {
                    ui.allocate_ui_with_layout(
                        vec2(width, t.density("control-sm")),
                        Layout::right_to_left(Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            let ready = !draft.trim().is_empty();
                            if ui
                                .add(Button::new(&self.words.submit).size(ButtonSize::Sm).enabled(ready))
                                .clicked()
                            {
                                out.submit = Some(draft.clone());
                            }
                            if ui
                                .add(
                                    Button::new(&self.words.cancel)
                                        .size(ButtonSize::Sm)
                                        .variant(ButtonVariant::Ghost),
                                )
                                .clicked()
                            {
                                out.cancel = true;
                            }
                        },
                    );
                }
                if send_key && !draft.trim().is_empty() {
                    out.submit = Some(draft.trim_end_matches('\n').to_owned());
                }
            });
        });
        if out.submit.is_some() || out.cancel {
            draft.clear();
        }
        ui.data_mut(|d| d.insert_temp(id, draft));
        out
    }
}

/// What a thread was asked this frame, each by the comment's id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommentThreadResponse {
    pub like: Option<String>,
    pub dislike: Option<String>,
    /// A reply written: the comment it answers and the words.
    pub reply: Option<(String, String)>,
    /// A reaction given or taken back: the comment, the emoji and its name.
    pub reaction: Option<(String, String, String)>,
    /// An entry of a comment's menu chosen, by its index.
    pub menu: Option<(String, usize)>,
}

/// The reactions a react button offers at once.
pub const QUICK_REACTIONS: [(&str, &str); 6] = [
    ("\u{1F44D}", "thumbs up"),
    ("\u{2764}\u{FE0F}", "red heart"),
    ("\u{1F602}", "face with tears of joy"),
    ("\u{1F389}", "party popper"),
    ("\u{1F62E}", "face with open mouth"),
    ("\u{1F64F}", "folded hands"),
];

type Menu<'a> = Box<dyn Fn(&ThreadComment) -> Vec<MenuItem> + 'a>;

/// The comments under a post or a video, after sokkel-ui's `CommentThread`, 24 apart: each with the
/// author's picture (32 across, 24 for a reply), a pinned comment saying so over the name, the name
/// in `text.help` at 500 with a creator mark and the time beside it, the words in `text.body` at the
/// relaxed leading, its reactions, then like (with the count), dislike, react and Reply as extra
/// small ghost buttons, and its menu at the end. Replies fold under their comment behind a count, 16
/// apart. Liking and replying are the app's: the thread says what was pressed.
#[must_use = "show it with `.show(ui)`"]
pub struct CommentThread<'a> {
    id_salt: egui::Id,
    comments: &'a [ThreadComment],
    viewer: Option<(String, Option<String>)>,
    pinned_by: Option<String>,
    reactions: bool,
    menu: Option<Menu<'a>>,
    words: CommentThreadWords,
}

impl<'a> CommentThread<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, comments: &'a [ThreadComment]) -> Self {
        Self {
            id_salt: egui::Id::unique(id_salt),
            comments,
            viewer: None,
            pinned_by: None,
            reactions: false,
            menu: None,
            words: CommentThreadWords::default(),
        }
    }

    /// Who replies, for the field Reply opens, with the kind color their picture wears; offers Reply.
    pub fn viewer(mut self, name: impl Into<String>, color: Option<String>) -> Self {
        self.viewer = Some((name.into(), color));
        self
    }

    /// Who pinned the pinned comment, as said over it.
    pub fn pinned_by(mut self, by: impl Into<String>) -> Self {
        self.pinned_by = Some(by.into());
        self
    }

    /// Offers reactions: a react button, and each reaction a press gives or takes back.
    pub fn reactions(mut self, on: bool) -> Self {
        self.reactions = on;
        self
    }

    /// A comment's menu items, none for no menu.
    pub fn menu(mut self, menu: impl Fn(&ThreadComment) -> Vec<MenuItem> + 'a) -> Self {
        self.menu = Some(Box::new(menu));
        self
    }

    pub fn words(mut self, words: CommentThreadWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> CommentThreadResponse {
        let mut out = CommentThreadResponse::default();
        let id = ui.make_persistent_id(self.id_salt);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 24.0;
            for comment in self.comments {
                self.item(ui, id, comment, 0, &mut out);
            }
        });
        out
    }

    fn item(&self, ui: &mut Ui, id: egui::Id, comment: &ThreadComment, depth: usize, out: &mut CommentThreadResponse) {
        let t = tokens(ui.ctx());
        let here = id.with(&comment.id);
        let words = &self.words;
        let items = self.menu.as_ref().map(|menu| menu(comment)).unwrap_or_default();
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = vec2(12.0, 2.0);
            let mut avatar =
                PersonAvatar::new(&comment.author).size(if depth > 0 { AvatarSize::Sm } else { AvatarSize::Md });
            if let Some(kind) = &comment.color {
                avatar = avatar.color(kind);
            }
            ui.add(avatar);
            let menu_room = if items.is_empty() {
                0.0
            } else {
                t.density("control-sm") + 12.0
            };
            let width = (ui.available_width() - menu_room).max(0.0);
            ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.set_width(width);
                ui.spacing_mut().item_spacing.y = 2.0;
                // each line as tall as its words, not a control's height
                ui.spacing_mut().interact_size.y = 0.0;
                let help = |text: &str| TextRole::Help.rich(&t, text).color(t.color("muted-foreground"));
                if comment.pinned {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let (r, _) = ui.allocate_exact_size(vec2(12.0, 16.0), Sense::hover());
                        icons::paint(ui.painter(), r.center(), Icon::Pin, 12.0, t.color("muted-foreground"));
                        let said = match &self.pinned_by {
                            Some(by) => words.pinned_by.replace("{by}", by),
                            None => words.pinned.clone(),
                        };
                        ui.label(help(&said));
                    });
                }
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(
                        TextRole::Help
                            .rich(&t, &comment.author)
                            .variation("wght", 500.0)
                            .color(t.color("foreground")),
                    );
                    if comment.creator {
                        egui::Frame::new()
                            .fill(t.color("muted"))
                            .corner_radius(t.radius("mini"))
                            .inner_margin(egui::Margin::symmetric(4, 0))
                            .show(ui, |ui| ui.label(help(&words.creator)));
                    }
                    let time = ui.label(help(&comment.time));
                    if let Some(title) = &comment.time_title {
                        let _ = time.on_hover_text(title);
                    }
                });
                let spec = t.text("body");
                ui.label(
                    TextRole::Body
                        .rich(&t, &comment.body)
                        .line_height(Some((spec.size * 1.625).round()))
                        .color(t.color("foreground")),
                );
                if self.reactions && !comment.reactions.is_empty() {
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                        for reaction in &comment.reactions {
                            let label = format!("{} {}", reaction.id, reaction.count);
                            let chip = Button::new(label)
                                .size(ButtonSize::Xs)
                                .variant(ButtonVariant::Outline)
                                .pressed(reaction.mine);
                            if ui.add(chip).on_hover_text(&reaction.label).clicked() {
                                out.reaction = Some((comment.id.clone(), reaction.id.clone(), reaction.label.clone()));
                            }
                        }
                    });
                }
                // like, dislike, react and Reply, 8 back so their words line up with the text
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.add_space(-8.0);
                    let mut like = Button::new(comment.likes.clone().unwrap_or_default())
                        .icon(Icon::ThumbsUp)
                        .size(ButtonSize::Xs)
                        .variant(ButtonVariant::Ghost)
                        .muted(!comment.liked)
                        .pressed(comment.liked);
                    if comment.likes.is_none() {
                        like.unseen_label = true;
                    }
                    if ui.add(like).on_hover_text(&words.like).clicked() {
                        out.like = Some(comment.id.clone());
                    }
                    if ui.add(IconButton::new(Icon::ThumbsDown, &words.dislike).xs()).clicked() {
                        out.dislike = Some(comment.id.clone());
                    }
                    if self.reactions {
                        let react = ui.add(IconButton::new(Icon::SmilePlus, &words.react).xs());
                        let quick: Vec<MenuItem> = QUICK_REACTIONS
                            .iter()
                            .map(|(emoji, name)| MenuItem::action(format!("{emoji}  {name}")))
                            .collect();
                        if let Some(index) = DropdownMenu::new(&quick).width(200.0).show(&react) {
                            let (emoji, name) = QUICK_REACTIONS[index];
                            out.reaction = Some((comment.id.clone(), emoji.to_owned(), name.to_owned()));
                        }
                    }
                    if self.viewer.is_some()
                        && ui
                            .add(
                                Button::new(&words.reply)
                                    .size(ButtonSize::Xs)
                                    .variant(ButtonVariant::Ghost)
                                    .muted(true),
                            )
                            .clicked()
                    {
                        ui.data_mut(|d| d.insert_temp(here.with("replying"), true));
                    }
                });
                let replying = ui.data(|d| d.get_temp::<bool>(here.with("replying"))).unwrap_or(false);
                if replying && let Some((name, color)) = &self.viewer {
                    let mut composer = CommentComposer::new(here.with("reply"), name)
                        .small(true)
                        .cancellable(true)
                        .words(CommentComposerWords {
                            placeholder: words.reply.clone(),
                            submit: words.reply.clone(),
                            ..CommentComposerWords::default()
                        });
                    if let Some(kind) = color {
                        composer = composer.color(kind);
                    }
                    let asked = composer.show(ui);
                    if let Some(text) = asked.submit {
                        out.reply = Some((comment.id.clone(), text));
                        ui.data_mut(|d| {
                            d.insert_temp(here.with("replying"), false);
                            d.insert_temp(here.with("open"), true);
                        });
                    }
                    if asked.cancel {
                        ui.data_mut(|d| d.insert_temp(here.with("replying"), false));
                    }
                }
                if !comment.replies.is_empty() {
                    let open = ui.data(|d| d.get_temp::<bool>(here.with("open"))).unwrap_or(false);
                    let count = comment.replies.len();
                    let said = if open {
                        words.hide_replies.clone()
                    } else if count == 1 {
                        words.reply_one.replace("{count}", "1")
                    } else {
                        words.replies.replace("{count}", &count.to_string())
                    };
                    let toggle = ui
                        .horizontal(|ui| {
                            ui.add_space(-8.0);
                            let mut button = Button::new(said).size(ButtonSize::Xs).variant(ButtonVariant::Ghost);
                            button.ink = Some(t.color("primary-ink"));
                            ui.add(button)
                        })
                        .inner;
                    if toggle.clicked() {
                        ui.data_mut(|d| d.insert_temp(here.with("open"), !open));
                    }
                    if open {
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 16.0;
                            for reply in &comment.replies {
                                self.item(ui, id, reply, depth + 1, out);
                            }
                        });
                    }
                }
            });
            if !items.is_empty() {
                let trigger = ui.add(IconButton::new(Icon::Ellipsis, &words.actions));
                if let Some(index) = DropdownMenu::new(&items).end(true).width(160.0).show(&trigger) {
                    out.menu = Some((comment.id.clone(), index));
                }
            }
        });
    }
}
