//! The server an app talks to, for the top bar's actions, after sokkel-ui's `ServerMenu`.

use std::sync::Arc;

use egui::{
    Color32, Galley, Popup, PopupCloseBehavior, RectAlign, Response, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2,
    vec2,
};

use super::menu::{RowLook, action_row, popover_frame};
use crate::{Icon, MenuItem, TextRole, Tokens, icons, tokens};

/// Whether the server answers: it does, it does not, the app is asking it now, or a state of the
/// app's own, named in [`ServerMenu::states`] (such as `degraded`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerState {
    Up,
    Down,
    Checking,
    Own(&'static str),
}

/// A state of the app's own, with its words and its color token (such as `degraded`, said
/// "Slow to answer", in `warning`). One named `up`, `down` or `checking` changes that state's words
/// or color.
#[derive(Clone, Copy, Debug)]
pub struct ServerOwnState<'a> {
    pub name: &'a str,
    pub words: &'a str,
    /// A color token: a status's (`status-succeeded`) or a tone's (`warning`).
    pub color: &'a str,
}

/// One check behind the state: its name, its state and a detail.
#[derive(Clone, Copy, Debug)]
pub struct ServerCheck<'a> {
    pub name: &'a str,
    pub state: ServerState,
    pub detail: Option<&'a str>,
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
    /// When the server was last asked, `{ago}` the time relative to now: `Checked 2 minutes ago`.
    pub checked_at: String,
}

impl Default for ServerMenuWords {
    fn default() -> Self {
        Self {
            up: "Connected".into(),
            down: "Not answering".into(),
            checking: "Checking".into(),
            check: "Check again".into(),
            label: "Server".into(),
            checked_at: "Checked {ago}".into(),
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
    /// The face was pressed, with [`ServerMenu::on_press`]: the app opens its own pane.
    pub pressed: bool,
    /// The tooltip opened while the server was not being asked: the app may ask it again.
    pub recheck: bool,
}

/// A state as the menu draws it: its words, its dot's color, the menu label's icon, and whether it
/// is well (a quiet face then draws the dot alone).
struct Said {
    words: String,
    color: Color32,
    icon: Icon,
    well: bool,
}

/// The server an app talks to: a dot in the state's color (pulsing while the app asks), the
/// server's name and its version in the code face, opening a menu that says the state in words
/// with the server's facts under it, the checks behind the state, Check again, and the app's own
/// items. Quiet, the face is the dot alone until something is wrong. With checks or a time it was
/// checked, a tooltip on the face lists them.
pub struct ServerMenu<'a> {
    name: &'a str,
    version: Option<&'a str>,
    environment: Option<&'a str>,
    state: ServerState,
    states: &'a [ServerOwnState<'a>],
    quiet: bool,
    checks: &'a [ServerCheck<'a>],
    checked_at: Option<&'a str>,
    press: bool,
    facts: &'a [&'a str],
    check: bool,
    items: &'a [MenuItem],
    default_open: bool,
    words: ServerMenuWords,
}

impl<'a> ServerMenu<'a> {
    pub fn new(name: &'a str, state: ServerState) -> Self {
        Self {
            name,
            version: None,
            environment: None,
            state,
            states: &[],
            quiet: false,
            checks: &[],
            checked_at: None,
            press: false,
            facts: &[],
            check: false,
            items: &[],
            default_open: false,
            words: ServerMenuWords::default(),
        }
    }

    /// The version it answers with, drawn in the code face beside the name.
    pub fn version(mut self, version: Option<&'a str>) -> Self {
        self.version = version;
        self
    }

    /// Which deployment it is, such as `production`, said beside the name in the tooltip and the
    /// menu.
    pub fn environment(mut self, environment: &'a str) -> Self {
        self.environment = Some(environment);
        self
    }

    /// The app's own states with their words and colors.
    pub fn states(mut self, states: &'a [ServerOwnState<'a>]) -> Self {
        self.states = states;
        self
    }

    /// A face that is the dot alone while the server is well or being asked, and the dot with the
    /// state in words when it is not.
    pub fn quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        self
    }

    /// Every check behind the state, listed in a tooltip on the face and in the menu.
    pub fn checks(mut self, checks: &'a [ServerCheck<'a>]) -> Self {
        self.checks = checks;
        self
    }

    /// When the server was last asked, relative to now (`2 minutes ago`), said in the tooltip as
    /// `words.checked_at`.
    pub fn checked_at(mut self, ago: &'a str) -> Self {
        self.checked_at = Some(ago);
        self
    }

    /// A press on the face answers [`ServerMenuResponse::pressed`] in place of opening the menu,
    /// for an app with its own pane about the server: no menu and no chevron.
    pub fn on_press(mut self, press: bool) -> Self {
        self.press = press;
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

    /// Start with the menu open.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    pub fn words(mut self, words: ServerMenuWords) -> Self {
        self.words = words;
        self
    }

    /// A state's words, color, icon and whether it is well.
    fn said(&self, t: &Tokens, state: ServerState) -> Said {
        let name = match state {
            ServerState::Up => "up",
            ServerState::Down => "down",
            ServerState::Checking => "checking",
            ServerState::Own(name) => name,
        };
        let own = self.states.iter().find(|s| s.name == name);
        let (words, color, icon) = match state {
            ServerState::Up => (&self.words.up, "status-succeeded", Icon::PlugZap),
            ServerState::Down => (&self.words.down, "status-failed", Icon::ServerCrash),
            ServerState::Checking => (&self.words.checking, "status-skipped", Icon::RefreshCw),
            ServerState::Own(name) => (&name.to_owned(), "status-skipped", Icon::CircleAlert),
        };
        let words = own.map_or_else(|| words.clone(), |s| s.words.to_owned());
        let color = own.map_or(color, |s| s.color);
        let well = matches!(color, "status-succeeded" | "good" | "succeeded");
        Said {
            words,
            color: t.color(color),
            icon,
            well,
        }
    }

    pub fn show(self, ui: &mut Ui) -> ServerMenuResponse {
        let t = tokens(ui.ctx());
        let mut out = ServerMenuResponse::default();
        let said = self.said(&t, self.state);
        let checking = self.state == ServerState::Checking;
        let phone = super::is_phone(ui.ctx());
        // quiet and well (or being asked): the dot alone; quiet and wrong: the dot and the state
        let quiet_face = self.quiet;
        let quiet_words = quiet_face && !(said.well || checking);
        let role = TextRole::Badge;
        let lay = |ui: &Ui, text: &str, width: f32| {
            WidgetText::from(role.rich(&t, text)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                width.max(1.0),
                role.font(&t),
            )
        };
        let mono = crate::mono_font(
            ui.ctx(),
            crate::mono_size(&t, t.text("badge").size),
            t.weight(crate::Weight::Medium),
        );
        let version = (!quiet_face).then_some(self.version).flatten().map(|v| {
            WidgetText::from(egui::RichText::new(v).font(mono.clone())).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                mono.clone(),
            )
        });
        let gap = 8.0;
        let chevron = !self.press && !quiet_face;
        // the face: a ghost button at sm, 8 across, its pieces 8 apart
        let fixed = 8.0
            + 8.0
            + version.as_ref().map_or(0.0, |v| gap + v.size().x + 12.0)
            + if chevron { gap + 16.0 } else { 0.0 }
            + 8.0;
        // the name, cut to its room, hidden below md; quiet, the state's words when wrong
        let words: Option<Arc<Galley>> = if quiet_face {
            quiet_words.then(|| lay(ui, &said.words, f32::INFINITY))
        } else if phone {
            None
        } else {
            Some(lay(ui, self.name, ui.available_width() - fixed - gap))
        };
        let width = fixed + words.as_ref().map_or(0.0, |w| gap + w.size().x);
        let height = super::finger_tall(ui.ctx(), &t, t.density("control-sm"));
        let (rect, face) = ui.allocate_exact_size(vec2(width, height), Sense::click());
        let label = format!(
            "{}: {}{}, {}",
            self.words.label,
            self.name,
            self.version.map(|v| format!(" {v}")).unwrap_or_default(),
            said.words
        );
        face.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
        let popup = face.id.with("menu");
        let open_before = Popup::is_id_open(ui.ctx(), popup);
        // open on the first frame when asked; ArrowDown opens as Enter and Space do
        let first_id = face.id.with("drawn");
        if !self.press && self.default_open && !ui.data(|d| d.get_temp::<bool>(first_id).unwrap_or(false)) {
            Popup::open_id(ui.ctx(), popup);
        }
        ui.data_mut(|d| d.insert_temp(first_id, true));
        if !self.press && face.has_focus() && ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
            Popup::open_id(ui.ctx(), popup);
        }
        let painter = ui.painter().clone();
        if face.hovered() || open_before {
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
        painter.circle_filled(pos2(x + 4.0, rect.center().y), 4.0, said.color.gamma_multiply(alpha));
        x += 8.0;
        if let Some(words) = words {
            x += gap;
            let w = words.size().x;
            painter.galley(
                pos2(x, rect.center().y - words.size().y / 2.0),
                words,
                t.color("foreground"),
            );
            x += w;
        }
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
        if chevron {
            icons::paint(
                &painter,
                pos2(x + gap + 8.0, rect.center().y),
                Icon::ChevronDown,
                16.0,
                t.color("muted-foreground"),
            );
        }
        super::focus_ring(&painter, &t, rect, t.radius("control"), &face);
        if self.press {
            out.pressed = face.clicked();
        } else {
            self.menu(ui, &face, &said, &mut out);
        }
        // Escape shuts the menu, the focus back on the face
        if open_before && !Popup::is_id_open(ui.ctx(), popup) && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            face.request_focus();
        }
        // the tooltip, with checks or a time checked, while the menu is shut
        if (!self.checks.is_empty() || self.checked_at.is_some()) && !Popup::is_id_open(ui.ctx(), popup) {
            let shown_id = face.id.with("tip-shown");
            let was = ui.data(|d| d.get_temp::<bool>(shown_id)).unwrap_or(false);
            let shown = super::overlay::tooltip_with(&face, false, RectAlign::BOTTOM_END, |ui| {
                ui.set_max_width(320.0);
                self.tooltip(ui, &t);
            });
            if shown && !was && !checking {
                out.recheck = true;
            }
            ui.data_mut(|d| d.insert_temp(shown_id, shown));
        }
        out
    }

    /// The tooltip: the name at 500, the environment and the version, when it was checked, the
    /// checks.
    fn tooltip(&self, ui: &mut Ui, t: &Tokens) {
        let ink = t.color("background");
        ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
        ui.spacing_mut().interact_size.y = 0.0;
        ui.horizontal(|ui| {
            ui.label(
                TextRole::Help
                    .rich(t, self.name)
                    .variation("wght", t.weight(crate::Weight::Medium))
                    .color(ink),
            );
            if let Some(environment) = self.environment {
                ui.label(TextRole::Help.rich(t, environment).color(ink.gamma_multiply(0.8)));
            }
            if let Some(version) = self.version {
                let mono = crate::mono_font(ui.ctx(), crate::mono_size(t, t.text("help").size), 400.0);
                ui.label(egui::RichText::new(version).font(mono).color(ink.gamma_multiply(0.8)));
            }
        });
        if let Some(ago) = self.checked_at {
            let words = self.words.checked_at.replace("{ago}", ago);
            ui.label(TextRole::Help.rich(t, words).color(ink.gamma_multiply(0.8)));
        }
        if !self.checks.is_empty() {
            self.check_lines(ui, t, ink, ink.gamma_multiply(0.8));
        }
    }

    /// Every check, one a line, 4 apart: its dot, its name at 500, its state's words and its
    /// detail, quieter.
    fn check_lines(&self, ui: &mut Ui, t: &Tokens, ink: Color32, quiet: Color32) {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
            ui.spacing_mut().interact_size.y = 0.0;
            for check in self.checks {
                let said = self.said(t, check.state);
                ui.horizontal_wrapped(|ui| {
                    let (dot, _) = ui.allocate_exact_size(vec2(8.0, t.text("help").line_height), Sense::hover());
                    ui.painter().circle_filled(dot.center(), 4.0, said.color);
                    ui.label(
                        TextRole::Help
                            .rich(t, check.name)
                            .variation("wght", t.weight(crate::Weight::Medium))
                            .color(ink),
                    );
                    ui.label(TextRole::Help.rich(t, &said.words).color(quiet));
                    if let Some(detail) = check.detail {
                        ui.label(TextRole::Help.rich(t, detail).color(quiet));
                    }
                });
            }
        });
    }

    fn menu(&self, ui: &Ui, face: &Response, said: &Said, out: &mut ServerMenuResponse) {
        let t = tokens(ui.ctx());
        let width = 288.0;
        let popup = face.id.with("menu");
        let checking = self.state == ServerState::Checking;
        Popup::menu(face)
            .id(popup)
            .align(RectAlign::BOTTOM_END)
            .frame(popover_frame(&t))
            .gap(4.0)
            .width(width)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_width(width);
                ui.spacing_mut().item_spacing.y = 0.0;
                // the state in words, as the menu's label: its icon and the words, 6 in
                let (head, _) = ui.allocate_exact_size(vec2(width, 32.0), Sense::hover());
                icons::paint(
                    ui.painter(),
                    pos2(head.left() + 6.0 + 8.0, head.center().y),
                    said.icon,
                    16.0,
                    t.color("popover-foreground"),
                );
                ui.painter().text(
                    pos2(head.left() + 6.0 + 16.0 + 8.0, head.center().y),
                    egui::Align2::LEFT_CENTER,
                    &said.words,
                    TextRole::Action.font(&t),
                    t.color("popover-foreground"),
                );
                // the name, the environment, the version while quiet, and the facts under them
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: 8,
                        right: 8,
                        top: 0,
                        bottom: 6,
                    })
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.spacing_mut().interact_size.y = 0.0;
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            ui.label(
                                TextRole::Help
                                    .rich(&t, self.name)
                                    .variation("wght", t.weight(crate::Weight::Medium))
                                    .color(t.color("foreground")),
                            );
                            if let Some(environment) = self.environment {
                                ui.label(TextRole::Help.rich(&t, environment).color(t.color("muted-foreground")));
                            }
                            if self.quiet
                                && let Some(version) = self.version
                            {
                                let mono = crate::mono_font(ui.ctx(), crate::mono_size(&t, t.text("help").size), 400.0);
                                ui.label(
                                    egui::RichText::new(version)
                                        .font(mono)
                                        .color(t.color("muted-foreground")),
                                );
                            }
                        });
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
                if !self.checks.is_empty() {
                    separator(ui);
                    egui::Frame::new()
                        .inner_margin(egui::Margin::symmetric(8, 6))
                        .show(ui, |ui| {
                            self.check_lines(ui, &t, t.color("popover-foreground"), t.color("muted-foreground"));
                        });
                }
                if self.check || !self.items.is_empty() {
                    separator(ui);
                }
                if self.check {
                    let words = if checking {
                        &self.words.checking
                    } else {
                        &self.words.check
                    };
                    let mut item = MenuItem::action(words.clone()).icon(Icon::RefreshCw).disabled(checking);
                    if checking {
                        // the icon turns while the server is asked: drawn here, turning
                        item = item.tint(Color32::TRANSPARENT);
                    }
                    let (row, clicked) = action_row(ui, &t, popup.with("check"), &item, false, RowLook::Menu);
                    if checking {
                        let time = ui.input(|i| i.time) as f32;
                        let center = pos2(row.rect.left() + 6.0 + 8.0, row.rect.center().y);
                        turning_icon(
                            ui,
                            center,
                            Icon::RefreshCw,
                            16.0,
                            t.color("muted-foreground").gamma_multiply(super::DISABLED),
                            time * std::f32::consts::TAU,
                        );
                        ui.ctx().request_repaint();
                    }
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
    }
}

/// An icon turned by `angle` around `center`, as a spinning glyph.
fn turning_icon(ui: &Ui, center: egui::Pos2, icon: Icon, size: f32, color: Color32, angle: f32) {
    if !icons::ready(ui.ctx()) {
        return;
    }
    let galley = ui
        .painter()
        .layout_no_wrap(char::from(icon).to_string(), icons::font(size), color);
    // a glyph turns around its own center: place its top-left so the center lands on `center`
    let half = galley.size() / 2.0;
    let rot = egui::emath::Rot2::from_angle(angle);
    let pos = center - rot * half;
    ui.painter()
        .add(egui::epaint::TextShape::new(pos, galley, color).with_angle(angle));
}
