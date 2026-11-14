//! People: an avatar in the person's kind color, and the account menu in the rail's foot, after
//! sokkel-ui's `PersonAvatar` and `AccountMenu`.

use egui::{Popup, PopupCloseBehavior, Rect, Response, Sense, Stroke, Ui, Widget, pos2, vec2};

use crate::{Icon, Scheme, TextRole, Tokens, icons, tokens};

/// The kind colors, in the order a name's hash picks from, as sokkel-ui's `KIND_COLORS`.
const KINDS: [&str; 6] = ["blue", "violet", "green", "pink", "teal", "jade"];

/// The kind color a name wears, the same as sokkel-ui's `kindOf`: a hash of its letters.
pub fn kind_of(name: &str) -> &'static str {
    let mut hash: u32 = 0;
    for unit in name.encode_utf16() {
        hash = hash.wrapping_mul(31).wrapping_add(unit as u32);
    }
    KINDS[(hash % KINDS.len() as u32) as usize]
}

/// The letters an avatar shows: the first of up to two parts of the name before any `@`.
pub fn initials(name: &str, count: usize) -> String {
    name.split('@')
        .next()
        .unwrap_or_default()
        .split([' ', '.', '_', '-'])
        .filter(|part| !part.is_empty())
        .take(count)
        .filter_map(|part| part.chars().next())
        .collect::<String>()
        .to_uppercase()
}

/// How big an avatar is, as sokkel-ui's sizes: 16, 20, 24, 32 and 36 across.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AvatarSize {
    Xxs,
    Xs,
    #[default]
    Sm,
    Md,
    Lg,
}

impl AvatarSize {
    fn box_and_letters(self) -> (f32, usize, f32) {
        match self {
            AvatarSize::Xxs => (16.0, 1, 10.0),
            AvatarSize::Xs => (20.0, 1, 11.0),
            AvatarSize::Sm => (24.0, 2, 12.0),
            AvatarSize::Md => (32.0, 2, 14.0),
            AvatarSize::Lg => (36.0, 2, 14.0),
        }
    }
}

/// A person's avatar: their initials on their kind color's wash, or on the primary color for the
/// account it is.
#[must_use = "add it with `ui.add(...)`"]
pub struct PersonAvatar {
    name: String,
    size: AvatarSize,
    primary: bool,
}

impl PersonAvatar {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            size: AvatarSize::Sm,
            primary: false,
        }
    }

    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    /// The primary ground and its ink, as the signed-in account's avatar.
    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }
}

pub(crate) fn paint_avatar(ui: &Ui, t: &Tokens, rect: Rect, name: &str, size: AvatarSize, primary: bool) {
    let (_, letters, text) = size.box_and_letters();
    let kind = kind_of(name);
    let (ground, ink) = if primary {
        (t.color("primary"), t.color("primary-foreground"))
    } else {
        (
            t.color_at(&format!("kind-{kind}"), 0.15),
            t.color(&format!("kind-{kind}-ink")),
        )
    };
    let painter = ui.painter();
    painter.circle_filled(rect.center(), rect.width() / 2.0, ground);
    let rich = egui::RichText::new(initials(name, letters))
        .size(text)
        .variation("wght", 600.0);
    let galley = egui::WidgetText::from(rich).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::FontSelection::Default,
    );
    painter.galley(rect.center() - galley.size() / 2.0, galley, ink);
}

impl Widget for PersonAvatar {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = tokens(ui.ctx());
        let (side, _, _) = self.size.box_and_letters();
        let (rect, response) = ui.allocate_exact_size(vec2(side, side), Sense::hover());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, true, &self.name));
        if ui.is_rect_visible(rect) {
            paint_avatar(ui, &t, rect, &self.name, self.size, self.primary);
        }
        response
    }
}

/// What the account menu was asked this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountAction {
    Settings,
    Scheme(Scheme),
}

/// The words the account menu says, English by default.
#[derive(Clone, Debug)]
pub struct AccountWords {
    pub settings: String,
    pub scheme: String,
    pub light: String,
    pub dark: String,
    pub system: String,
    /// The trigger's name for assistive technology.
    pub account: String,
}

impl Default for AccountWords {
    fn default() -> Self {
        Self {
            settings: "Settings".into(),
            scheme: "Scheme".into(),
            light: "Light".into(),
            dark: "Dark".into(),
            system: "System".into(),
            account: "Account".into(),
        }
    }
}

/// The signed-in account in the rail's foot: its avatar and name, opening a menu above it with the
/// account, Settings, and the scheme to pick.
pub struct AccountMenu {
    name: String,
    place: Option<String>,
    scheme: Option<Scheme>,
    words: AccountWords,
}

impl AccountMenu {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            place: None,
            scheme: None,
            words: AccountWords::default(),
        }
    }

    /// Where the account is signed in, in mono under its name in the menu.
    pub fn place(mut self, place: impl Into<String>) -> Self {
        self.place = Some(place.into());
        self
    }

    /// Offer the scheme, with the one worn now checked.
    pub fn scheme(mut self, scheme: Scheme) -> Self {
        self.scheme = Some(scheme);
        self
    }

    pub fn words(mut self, words: AccountWords) -> Self {
        self.words = words;
        self
    }

    /// Fill the rail's foot cell with the account; returns what its menu was asked.
    pub fn show(self, ui: &mut Ui) -> Option<AccountAction> {
        let t = tokens(ui.ctx());
        let cell = ui.available_rect_before_wrap();
        ui.painter()
            .rect_filled(cell, egui::CornerRadius::ZERO, t.color("sidebar"));
        ui.painter().hline(
            cell.x_range(),
            cell.top() + 0.5,
            Stroke::new(1.0, t.color("border-strong")),
        );
        ui.painter().vline(
            cell.right() - 0.5,
            cell.y_range(),
            Stroke::new(1.0, t.color("border-strong")),
        );
        let row = Rect::from_min_max(cell.min + vec2(6.0, 4.0), cell.max - vec2(7.0, 4.0));
        // an id of its own wherever it stands, so two frames on one page keep their menus apart
        let trigger = ui.interact(row, ui.auto_id_with(("account", &self.name)), Sense::click());
        // a collapsed rail's cell holds the avatar alone
        let narrow = cell.width() < 120.0;
        trigger.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &self.words.account));
        let popup_id = trigger.id.with("menu");
        if trigger.hovered() || Popup::is_id_open(ui.ctx(), popup_id) {
            ui.painter().rect_filled(row, t.radius("md"), t.color("hover"));
        }
        let avatar_at = if narrow {
            row.center()
        } else {
            pos2(row.left() + 10.0 + 12.0, row.center().y)
        };
        let avatar = Rect::from_center_size(avatar_at, vec2(24.0, 24.0));
        paint_avatar(ui, &t, avatar, &self.name, AvatarSize::Sm, true);
        if !narrow {
            let name = egui::WidgetText::from(TextRole::Action.rich(&t, &self.name)).into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                row.width() - 70.0,
                TextRole::Action.font(&t),
            );
            ui.painter().galley(
                pos2(avatar.right() + 10.0, row.center().y - name.size().y / 2.0),
                name,
                t.color("foreground"),
            );
            icons::paint(
                ui.painter(),
                pos2(row.right() - 14.0, row.center().y),
                Icon::ChevronsUpDown,
                16.0,
                t.color("muted-foreground"),
            );
        }

        let mut action = None;
        let frame = super::popup_frame(&t).inner_margin(egui::Margin::same(4));
        // 4 in from the trigger's start, as the web's `alignOffset`
        Popup::menu(&trigger)
            .anchor(trigger.rect.with_min_x(trigger.rect.left() + 4.0))
            .id(popup_id)
            .frame(frame)
            .align(egui::RectAlign::TOP_START)
            .gap(4.0)
            .width(252.0)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_width(244.0);
                ui.spacing_mut().item_spacing.y = 0.0;
                // the account: its avatar, name and where it is signed in
                let (head, _) = ui.allocate_exact_size(vec2(244.0, 58.0), Sense::hover());
                let big = Rect::from_center_size(pos2(head.left() + 8.0 + 16.0, head.center().y), vec2(32.0, 32.0));
                paint_avatar(ui, &t, big, &self.name, AvatarSize::Md, true);
                let text_x = big.right() + 10.0;
                let name = egui::WidgetText::from(TextRole::Action.rich(&t, &self.name)).into_galley(
                    ui,
                    Some(egui::TextWrapMode::Truncate),
                    180.0,
                    TextRole::Action.font(&t),
                );
                let name_h = name.size().y;
                let top = if self.place.is_some() {
                    head.center().y - name_h
                } else {
                    head.center().y - name_h / 2.0
                };
                ui.painter()
                    .galley(pos2(text_x, top), name, t.color("popover-foreground"));
                if let Some(place) = &self.place {
                    let mono = egui::FontId::monospace(TextRole::Help.font(&t).size * 0.9);
                    ui.painter().text(
                        pos2(text_x, top + name_h + 2.0),
                        egui::Align2::LEFT_TOP,
                        place,
                        mono,
                        t.color("muted-foreground"),
                    );
                }
                separator(ui, &t);
                if menu_row(ui, &t, Some(Icon::Settings), &self.words.settings, false).clicked() {
                    action = Some(AccountAction::Settings);
                    ui.close();
                }
                if let Some(current) = self.scheme {
                    separator(ui, &t);
                    // the group's title: 28 tall, 8 in, as a menu label's `px-2 py-1.5`
                    let (title, _) = ui.allocate_exact_size(vec2(244.0, 28.0), Sense::hover());
                    let words = egui::WidgetText::from(TextRole::Badge.rich(&t, &self.words.scheme)).into_galley(
                        ui,
                        Some(egui::TextWrapMode::Truncate),
                        232.0,
                        TextRole::Badge.font(&t),
                    );
                    ui.painter().galley(
                        pos2(title.left() + 8.0, title.center().y - words.size().y / 2.0),
                        words,
                        t.color("muted-foreground"),
                    );
                    for (scheme, label, icon) in [
                        (Scheme::Light, &self.words.light, Icon::Sun),
                        (Scheme::Dark, &self.words.dark, Icon::Moon),
                        (Scheme::System, &self.words.system, Icon::Monitor),
                    ] {
                        if menu_row(ui, &t, Some(icon), label, scheme == current).clicked() {
                            action = Some(AccountAction::Scheme(scheme));
                            ui.close();
                        }
                    }
                }
            });
        action
    }
}

fn separator(ui: &mut Ui, t: &Tokens) {
    ui.add_space(4.0);
    let rect = ui.available_rect_before_wrap();
    ui.painter().hline(
        (rect.left() - 4.0)..=(rect.right() + 4.0),
        rect.top(),
        Stroke::new(1.0, t.color("border")),
    );
    ui.add_space(5.0);
}

/// One row of a menu: an icon, a label and a check when it is the one chosen.
pub(crate) fn menu_row(ui: &mut Ui, t: &Tokens, icon: Option<Icon>, label: &str, checked: bool) -> Response {
    // 32 tall, a finger tall on a phone
    let height = if super::is_phone(ui.ctx()) {
        t.length("spacing-finger")
    } else {
        32.0
    };
    let (row, hit) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    hit.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, checked, label));
    if hit.hovered() || hit.has_focus() {
        ui.painter().rect_filled(row, t.radius("md"), t.color("hover"));
    }
    let mut x = row.left() + 8.0;
    if let Some(glyph) = icon {
        icons::paint(
            ui.painter(),
            pos2(x + 8.0, row.center().y),
            glyph,
            16.0,
            t.color("muted-foreground"),
        );
        // the icon, then 10 to the label
        x += 16.0 + 10.0;
    }
    let galley = egui::WidgetText::from(TextRole::Body.rich(t, label)).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        row.right() - x - 30.0,
        TextRole::Body.font(t),
    );
    ui.painter().galley(
        pos2(x, row.center().y - galley.size().y / 2.0),
        galley,
        t.color("popover-foreground"),
    );
    if checked {
        icons::paint(
            ui.painter(),
            pos2(row.right() - 14.0, row.center().y),
            Icon::Check,
            16.0,
            t.color("popover-foreground"),
        );
    }
    hit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_match_sokkel_ui() {
        // the hash sokkel-ui's kindOf computes: ada 96414 % 6 = 0, grace 98615224 % 6 = 4
        assert_eq!(kind_of("ada"), "blue");
        assert_eq!(kind_of("grace"), "teal");
        assert_eq!(kind_of("linus"), "pink");
        assert_eq!(initials("ada", 1), "A");
        assert_eq!(initials("margaret.hamilton@example.org", 2), "MH");
    }
}
