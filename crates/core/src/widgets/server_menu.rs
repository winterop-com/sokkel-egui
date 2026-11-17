//! The server an app talks to, for the top bar's actions, after sokkel-ui's `ServerMenu`.

use egui::{Popup, PopupCloseBehavior, Response, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2};

use super::menu::{RowLook, action_row, popover_frame};
use crate::{Icon, MenuItem, TextRole, icons, tokens};

/// Whether the server answers: it does, it does not, or the app is asking it now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerState {
    Up,
    Down,
    Checking,
}

/// The words a server menu says.
#[derive(Clone, Debug)]
pub struct ServerMenuWords {
    pub up: String,
    pub down: String,
    pub checking: String,
    /// The item that asks the server again.
    pub check: String,
    /// The trigger's name for a screen reader, before the state.
    pub label: String,
}

impl Default for ServerMenuWords {
    fn default() -> Self {
        Self {
            up: "Connected".into(),
            down: "Not answering".into(),
            checking: "Checking".into(),
            check: "Check again".into(),
            label: "Server".into(),
        }
    }
}

/// What a server menu was asked this frame.
#[derive(Default)]
pub struct ServerMenuResponse {
    /// Check again was pressed.
    pub check: bool,
    /// The index of the app's own item pressed.
    pub item: Option<usize>,
}

/// The server an app talks to: a dot in the state's color (pulsing while the app asks), the
/// server's name and its version in the code face, opening a menu that says the state in words
/// with the server's facts under it, Check again, and the app's own items.
pub struct ServerMenu<'a> {
    name: &'a str,
    version: Option<&'a str>,
    state: ServerState,
    facts: &'a [&'a str],
    check: bool,
    items: &'a [MenuItem],
    words: ServerMenuWords,
}

impl<'a> ServerMenu<'a> {
    pub fn new(name: &'a str, state: ServerState) -> Self {
        Self {
            name,
            version: None,
            state,
            facts: &[],
            check: false,
            items: &[],
            words: ServerMenuWords::default(),
        }
    }

    /// The version it answers with, drawn in the code face beside the name.
    pub fn version(mut self, version: Option<&'a str>) -> Self {
        self.version = version;
        self
    }

    /// Lines under the name in the menu: its address, what it serves, how it stores.
    pub fn facts(mut self, facts: &'a [&'a str]) -> Self {
        self.facts = facts;
        self
    }

    /// Adds Check again to the menu, its icon turning while the server is asked.
    pub fn check(mut self, check: bool) -> Self {
        self.check = check;
        self
    }

    /// The app's own items after Check again.
    pub fn items(mut self, items: &'a [MenuItem]) -> Self {
        self.items = items;
        self
    }

    pub fn words(mut self, words: ServerMenuWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> ServerMenuResponse {
        let t = tokens(ui.ctx());
        let (said, dot) = match self.state {
            ServerState::Up => (&self.words.up, t.color("status-succeeded")),
            ServerState::Down => (&self.words.down, t.color("status-failed")),
            ServerState::Checking => (&self.words.checking, t.color("status-skipped")),
        };
        let checking = self.state == ServerState::Checking;
        // the face: a ghost button, 8 across, the dot, the name, the version and the chevron 8 apart
        // in the small button's type, as the web's `size="sm"` sets it
        let name = WidgetText::from(TextRole::Badge.rich(&t, self.name)).into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            TextRole::Badge.font(&t),
        );
        let mono = egui::FontId::monospace(TextRole::Badge.font(&t).size);
        let version = self.version.map(|v| {
            WidgetText::from(egui::RichText::new(v).font(mono.clone())).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                mono.clone(),
            )
        });
        let gap = 8.0;
        let width = 8.0
            + 8.0
            + gap
            + name.size().x
            + version.as_ref().map_or(0.0, |v| gap + v.size().x + 12.0)
            + gap
            + 16.0
            + 8.0;
        let height = t.density("control-sm");
        let (rect, face) = ui.allocate_exact_size(vec2(width, height), Sense::click());
        let label = format!(
            "{}: {}{}, {said}",
            self.words.label,
            self.name,
            self.version.map(|v| format!(" {v}")).unwrap_or_default()
        );
        face.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
        let painter = ui.painter();
        if face.hovered() || Popup::is_id_open(ui.ctx(), face.id.with("menu")) {
            painter.rect_filled(rect, t.radius("control"), t.color("hover"));
        }
        let mut x = rect.left() + 8.0;
        let alpha = if checking {
            // a pulse while the app asks, as `animate-pulse`
            ui.ctx().request_repaint();
            let time = ui.input(|i| i.time) as f32;
            0.5 + 0.5 * (time * std::f32::consts::PI).cos().abs()
        } else {
            1.0
        };
        painter.circle_filled(pos2(x + 4.0, rect.center().y), 4.0, dot.gamma_multiply(alpha));
        x += 8.0 + gap;
        let name_width = name.size().x;
        painter.galley(
            pos2(x, rect.center().y - name.size().y / 2.0),
            name,
            t.color("foreground"),
        );
        x += name_width;
        if let Some(version) = version {
            x += gap;
            let badge = egui::Rect::from_min_size(pos2(x, rect.center().y - 10.0), vec2(version.size().x + 12.0, 20.0));
            painter.rect_filled(badge, t.radius("chip"), t.color("secondary"));
            painter.galley(
                pos2(badge.left() + 6.0, badge.center().y - version.size().y / 2.0),
                version,
                t.color("secondary-foreground"),
            );
            x = badge.right();
        }
        icons::paint(
            painter,
            pos2(x + gap + 8.0, rect.center().y),
            Icon::ChevronDown,
            16.0,
            t.color("muted-foreground"),
        );
        super::focus_ring(painter, &t, rect, t.radius("control"), &face);
        self.menu(ui, &face, said)
    }

    fn menu(&self, ui: &Ui, face: &Response, said: &str) -> ServerMenuResponse {
        let t = tokens(ui.ctx());
        let mut out = ServerMenuResponse::default();
        let width = 288.0;
        let popup = face.id.with("menu");
        Popup::menu(face)
            .id(popup)
            .align(egui::RectAlign::BOTTOM_END)
            .frame(popover_frame(&t))
            .gap(4.0)
            .width(width)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_width(width);
                ui.spacing_mut().item_spacing.y = 0.0;
                let checking = self.state == ServerState::Checking;
                let icon = match self.state {
                    ServerState::Checking => Icon::RefreshCw,
                    ServerState::Down => Icon::ServerCrash,
                    ServerState::Up => Icon::PlugZap,
                };
                // the state in words, as the menu's label: an icon and the words, 6 in
                let (head, _) = ui.allocate_exact_size(vec2(width, 32.0), Sense::hover());
                icons::paint(
                    ui.painter(),
                    pos2(head.left() + 6.0 + 8.0, head.center().y),
                    icon,
                    16.0,
                    t.color("popover-foreground"),
                );
                ui.painter().text(
                    pos2(head.left() + 6.0 + 16.0 + 8.0, head.center().y),
                    egui::Align2::LEFT_CENTER,
                    said,
                    TextRole::Action.font(&t),
                    t.color("popover-foreground"),
                );
                // the name and the facts under it, in the help role
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: 8,
                        right: 8,
                        top: 0,
                        bottom: 6,
                    })
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.label(
                            TextRole::Help
                                .rich(&t, self.name)
                                .variation("wght", 500.0)
                                .color(t.color("foreground")),
                        );
                        for fact in self.facts {
                            ui.add(
                                egui::Label::new(TextRole::Help.rich(&t, *fact).color(t.color("muted-foreground")))
                                    .wrap(),
                            );
                        }
                    });
                let separator = |ui: &mut Ui| {
                    ui.add_space(4.0);
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().hline(
                        (rect.left() - 4.0)..=(rect.right() + 4.0),
                        rect.top(),
                        Stroke::new(1.0, t.color("border")),
                    );
                    ui.add_space(5.0);
                };
                if self.check || !self.items.is_empty() {
                    separator(ui);
                }
                if self.check {
                    let words = if checking {
                        &self.words.checking
                    } else {
                        &self.words.check
                    };
                    let item = MenuItem::action(words.clone()).icon(Icon::RefreshCw).disabled(checking);
                    let (_, clicked) = action_row(ui, &t, popup.with("check"), &item, false, RowLook::Menu);
                    // the menu stays open, so the state it says changes in front of the reader
                    out.check = clicked;
                }
                for (index, item) in self.items.iter().enumerate() {
                    let (_, clicked) = action_row(ui, &t, popup.with(index), item, false, RowLook::Menu);
                    if clicked {
                        out.item = Some(index);
                        ui.close();
                    }
                }
            });
        out
    }
}
