//! Menus: a dropdown menu under its trigger, and the command palette over the page on Mod+K,
//! after sokkel-ui's `DropdownMenu` and `CommandPalette`.

use egui::{
    Context, Frame, Id, Key, Margin, Modifiers, Popup, PopupCloseBehavior, Response, Sense, Stroke, TextEdit,
    TextWrapMode, Ui, WidgetText, pos2, vec2,
};

use crate::{Icon, TextRole, Tokens, icons, tokens};

/// One entry in a menu.
#[derive(Clone, Debug)]
pub enum MenuItem {
    /// An action: its label, an icon, and the shortcut that does the same, drawn at its end.
    Action {
        label: String,
        icon: Option<Icon>,
        shortcut: Option<String>,
        destructive: bool,
        /// Shown but not chosen, dimmed.
        disabled: bool,
        /// The icon's own color, as a board column's state icon; the muted ink when `None`.
        tint: Option<egui::Color32>,
        /// What the app knows the command by, for a palette built from parts of its own.
        id: Option<String>,
        /// A word at the row's end, muted, such as "in use".
        hint: Option<String>,
        /// A second line under the label in the palette, muted, as the web's `detail`; the row two
        /// lines tall.
        detail: Option<String>,
        /// Other words a palette's search finds it by.
        keywords: Vec<String>,
    },
    /// A label over the items after it.
    Label(String),
    Separator,
}

impl MenuItem {
    pub fn action(label: impl Into<String>) -> Self {
        MenuItem::Action {
            label: label.into(),
            icon: None,
            shortcut: None,
            destructive: false,
            disabled: false,
            tint: None,
            id: None,
            hint: None,
            detail: None,
            keywords: Vec::new(),
        }
    }

    /// The id the app knows the command by.
    pub fn id(mut self, command: impl Into<String>) -> Self {
        if let MenuItem::Action { id, .. } = &mut self {
            *id = Some(command.into());
        }
        self
    }

    /// A muted second line under the label in the palette, which its search also finds it by.
    pub fn detail(mut self, line: impl Into<String>) -> Self {
        if let MenuItem::Action { detail, .. } = &mut self {
            *detail = Some(line.into());
        }
        self
    }

    /// A muted word at the row's end.
    pub fn hint(mut self, word: impl Into<String>) -> Self {
        if let MenuItem::Action { hint, .. } = &mut self {
            *hint = Some(word.into());
        }
        self
    }

    /// Other words the palette's search finds the command by.
    pub fn keywords(mut self, words: impl IntoIterator<Item = impl Into<String>>) -> Self {
        if let MenuItem::Action { keywords, .. } = &mut self {
            keywords.extend(words.into_iter().map(Into::into));
        }
        self
    }

    /// The command's id, if it was given one.
    pub fn command_id(&self) -> Option<&str> {
        match self {
            MenuItem::Action { id, .. } => id.as_deref(),
            _ => None,
        }
    }

    /// An action shown dimmed that cannot be chosen, as the column a card is already in.
    pub fn disabled(mut self, off: bool) -> Self {
        if let MenuItem::Action { disabled, .. } = &mut self {
            *disabled = off;
        }
        self
    }

    /// The icon in a color of its own.
    pub fn tint(mut self, color: egui::Color32) -> Self {
        if let MenuItem::Action { tint, .. } = &mut self {
            *tint = Some(color);
        }
        self
    }

    pub fn icon(mut self, glyph: Icon) -> Self {
        if let MenuItem::Action { icon, .. } = &mut self {
            *icon = Some(glyph);
        }
        self
    }

    pub fn shortcut(mut self, keys: impl Into<String>) -> Self {
        if let MenuItem::Action { shortcut, .. } = &mut self {
            *shortcut = Some(keys.into());
        }
        self
    }

    /// An action that destroys, in the destructive ink.
    pub fn destructive(mut self) -> Self {
        if let MenuItem::Action { destructive, .. } = &mut self {
            *destructive = true;
        }
        self
    }
}

fn popover_frame(t: &Tokens) -> Frame {
    super::popup_frame(t).inner_margin(Margin::same(4))
}

/// Draw one action row; true when it was pressed.
/// How a row of actions is drawn: a dropdown menu's (a control tall, 6 in) or the palette's (34
/// tall, 8 in); both a finger tall on a phone, and lit in the accent, as every list of choices is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RowLook {
    Menu,
    Palette,
}

fn action_row(ui: &mut Ui, t: &Tokens, id: Id, item: &MenuItem, highlighted: bool, look: RowLook) -> (Response, bool) {
    let MenuItem::Action {
        label,
        icon,
        shortcut,
        destructive,
        disabled,
        tint,
        hint,
        detail,
        ..
    } = item
    else {
        unreachable!()
    };
    let (height, pad) = match look {
        RowLook::Menu => (super::choice_row_height(ui.ctx(), t), 6.0),
        RowLook::Palette if super::is_phone(ui.ctx()) => (t.length("spacing-finger"), 8.0),
        RowLook::Palette => (34.0, 8.0),
    };
    // a palette row with a detail is two lines tall: the detail's line more
    let detail = detail.as_ref().filter(|_| look == RowLook::Palette);
    let height = height
        + if detail.is_some() {
            TextRole::Help.line_height(t)
        } else {
            0.0
        };
    let (row, hit) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    let sense = if *disabled { Sense::hover() } else { Sense::click() };
    let hit = ui.interact(row, id, sense).union(hit);
    hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, !*disabled, label));
    let painter = ui.painter();
    let lit = !*disabled && (highlighted || hit.hovered() || hit.has_focus());
    if lit {
        painter.rect_filled(
            row,
            // a menu's rows round as radius-md; the palette's, in a dialog, as radius-lg
            t.radius(if look == RowLook::Menu { "md" } else { "lg" }),
            if *destructive {
                t.color_at("destructive", 0.1)
            } else {
                super::ground(t, "dropdown-menu.base.highlighted.item", "accent")
            },
        );
    }
    // a lit row's icon, label and hint take the accent's ink
    let accent_ink = lit && !*destructive;
    let ink = super::dim(
        if *destructive {
            t.color("destructive")
        } else if accent_ink {
            t.color("accent-foreground")
        } else {
            t.color("popover-foreground")
        },
        !*disabled,
    );
    let mut x = row.left() + pad;
    if let Some(glyph) = icon {
        icons::paint(
            painter,
            pos2(x + 8.0, row.center().y),
            *glyph,
            16.0,
            super::dim(
                if *destructive || accent_ink {
                    ink
                } else {
                    tint.unwrap_or(t.color("muted-foreground"))
                },
                !*disabled,
            ),
        );
        // the icon, then 8 to the label
        x += 16.0 + 8.0;
    }
    // the shortcut (or the hint) first, so the label takes only the room it leaves
    let keys = shortcut
        .as_ref()
        .map(|keys| super::keys_galley(ui, t, TextRole::Help, keys, 1.0))
        .or_else(|| {
            hint.as_ref().map(|hint| {
                WidgetText::from(TextRole::Help.rich(t, hint)).into_galley(
                    ui,
                    Some(TextWrapMode::Extend),
                    f32::INFINITY,
                    TextRole::Help.font(t),
                )
            })
        });
    let room = row.right() - 8.0 - x - keys.as_ref().map_or(0.0, |k| k.size().x + 16.0);
    let galley = WidgetText::from(TextRole::Body.rich(t, label)).into_galley(
        ui,
        Some(TextWrapMode::Truncate),
        room,
        TextRole::Body.font(t),
    );
    match detail {
        None => ui
            .painter()
            .galley(pos2(x, row.center().y - galley.size().y / 2.0), galley, ink),
        Some(detail) => {
            let under = WidgetText::from(TextRole::Help.rich(t, detail)).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                room,
                TextRole::Help.font(t),
            );
            let top = row.center().y - (galley.size().y + under.size().y) / 2.0;
            let label_height = galley.size().y;
            ui.painter().galley(pos2(x, top), galley, ink);
            let quiet = if accent_ink {
                t.color("accent-foreground")
            } else {
                t.color("muted-foreground")
            };
            ui.painter().galley(pos2(x, top + label_height), under, quiet);
        }
    }
    if let Some(keys) = keys {
        let at = pos2(row.right() - 8.0 - keys.size().x, row.center().y - keys.size().y / 2.0);
        let quiet = if accent_ink {
            t.color("accent-foreground")
        } else {
            t.color("muted-foreground")
        };
        ui.painter().galley(at, keys, quiet);
    }
    let pressed = hit.clicked();
    (hit, pressed)
}

/// A menu under its trigger: actions with icons and shortcuts, labels and separators. Returns the
/// index of the action pressed this frame.
pub struct DropdownMenu<'a> {
    items: &'a [MenuItem],
    width: f32,
    default_open: bool,
    end: bool,
}

impl<'a> DropdownMenu<'a> {
    pub fn new(items: &'a [MenuItem]) -> Self {
        Self {
            items,
            width: 220.0,
            default_open: false,
            end: false,
        }
    }

    /// Line the menu's end up with its trigger's, as `align="end"` does.
    pub fn end(mut self, end: bool) -> Self {
        self.end = end;
        self
    }

    /// At least this wide; by default the menu fits its widest entry.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Open the menu the first time it is shown, as a story shows it.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Show the menu for `trigger`: it opens and closes with a press on the trigger.
    pub fn show(self, trigger: &Response) -> Option<usize> {
        let ctx = trigger.ctx.clone();
        let t = tokens(&ctx);
        let popup_id = trigger.id.with("menu");
        let opened_once = trigger.id.with("menu-opened-once");
        if self.default_open && !ctx.data(|d| d.get_temp::<bool>(opened_once).unwrap_or(false)) {
            ctx.data_mut(|d| d.insert_temp(opened_once, true));
            Popup::open_id(&ctx, popup_id);
        }
        // as wide as its widest entry, and at least its trigger and 128
        let measure = |text: &str, role: TextRole| {
            // the face at its default weight, with room for the role's heavier ones
            let font = role.font(&t);
            ctx.fonts_mut(|fonts| {
                fonts
                    .layout_no_wrap(text.to_owned(), font, egui::Color32::WHITE)
                    .size()
                    .x
            }) * 1.04
        };
        let widest = self
            .items
            .iter()
            .map(|item| match item {
                MenuItem::Action {
                    label, icon, shortcut, ..
                } => {
                    16.0 + measure(label, TextRole::Body)
                        + if icon.is_some() { 24.0 } else { 0.0 }
                        + shortcut
                            .as_deref()
                            // "⌘" is drawn as an icon about as wide as a capital, which the text faces stand in for
                            .map_or(0.0, |keys| 24.0 + measure(&keys.replace('⌘', "W"), TextRole::Help))
                }
                MenuItem::Label(label) => 12.0 + measure(label, TextRole::Badge),
                MenuItem::Separator => 0.0,
            })
            .fold(0.0, f32::max);
        let width = widest.max(trigger.rect.width()).max(128.0).max(self.width);
        let mut pressed = None;
        Popup::menu(trigger)
            .id(popup_id)
            .align(if self.end {
                egui::RectAlign::BOTTOM_END
            } else {
                egui::RectAlign::BOTTOM_START
            })
            .frame(popover_frame(&t))
            .gap(4.0)
            .width(width)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_width(width);
                ui.spacing_mut().item_spacing.y = 0.0;
                for (index, item) in self.items.iter().enumerate() {
                    match item {
                        MenuItem::Separator => {
                            ui.add_space(4.0);
                            let rect = ui.available_rect_before_wrap();
                            ui.painter().hline(
                                (rect.left() - 4.0)..=(rect.right() + 4.0),
                                rect.top(),
                                Stroke::new(1.0, t.color("border")),
                            );
                            ui.add_space(5.0);
                        }
                        MenuItem::Label(label) => {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.add_space(6.0);
                                ui.label(TextRole::Badge.rich(&t, label).color(t.color("muted-foreground")));
                            });
                            ui.add_space(4.0);
                        }
                        action => {
                            let (_, clicked) = action_row(ui, &t, popup_id.with(index), action, false, RowLook::Menu);
                            if clicked {
                                pressed = Some(index);
                                ui.close();
                            }
                        }
                    }
                }
            });
        pressed
    }
}

/// A group of commands in the palette, under a title.
#[derive(Clone, Debug)]
pub struct CommandGroup {
    pub title: String,
    pub items: Vec<MenuItem>,
}

impl CommandGroup {
    pub fn new(title: impl Into<String>, items: impl IntoIterator<Item = MenuItem>) -> Self {
        Self {
            title: title.into(),
            items: items.into_iter().collect(),
        }
    }
}

/// What the palette was asked this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaletteOutcome {
    /// The command run, as its group and its index in the group.
    pub run: Option<(usize, usize)>,
    /// Escape, a press on the shade, or a command run.
    pub close: bool,
    /// The running command was let go, by Stop waiting or Escape once it had run `busy-release`:
    /// the app stops it if it can, and the palette closes.
    pub stop_waiting: bool,
}

/// The words the palette says, English by default.
#[derive(Clone, Debug)]
pub struct PaletteWords {
    pub placeholder: String,
    pub no_match: String,
    /// Beside the command that runs, in its keys' place.
    pub running: String,
    /// The row that lets a running command go, once it has run `busy-release`.
    pub stop_waiting: String,
    /// Said where nothing is listed while the app's own search is out.
    pub searching: String,
}

impl Default for PaletteWords {
    fn default() -> Self {
        Self {
            placeholder: "Search or run a command".into(),
            no_match: "No command matches.".into(),
            running: "Running".into(),
            stop_waiting: "Stop waiting".into(),
            searching: "Searching".into(),
        }
    }
}

/// Mod+K, consumed: true on the frame it is pressed. Mod is Ctrl, and on macOS Cmd or Ctrl but not
/// both, as sokkel-ui's shell binds it (`alsoCtrl`); it is heard in a field too.
pub fn palette_shortcut(ctx: &Context) -> bool {
    // an open palette takes the key itself, and closes
    if palette_open(ctx) {
        return false;
    }
    let mac = ctx.os() == egui::os::OperatingSystem::Mac;
    let chord = |m: &Modifiers| {
        // `command` is Cmd on macOS and Ctrl elsewhere; `mac_cmd` is Cmd itself
        let held = if mac {
            (m.mac_cmd || m.command) != m.ctrl
        } else {
            (m.ctrl || m.command) && !m.mac_cmd
        };
        held && !m.alt && !m.shift
    };
    ctx.input_mut(|i| {
        let at = i.events.iter().position(
            |event| matches!(event, egui::Event::Key { key: Key::K, pressed: true, modifiers, .. } if chord(modifiers)),
        );
        at.map(|at| i.events.remove(at)).is_some()
    })
}

fn palette_seen_id() -> Id {
    Id::unique("sokkel-palette-seen")
}

/// A palette was drawn on the frame before this one.
fn palette_open(ctx: &Context) -> bool {
    let pass = ctx.cumulative_pass_nr();
    ctx.data(|d| d.get_temp::<u64>(palette_seen_id()))
        .is_some_and(|seen| seen + 1 >= pass)
}

/// Mod+K pressed, consumed, whether or not a palette is open: what an open palette closes on.
fn mod_k(ctx: &Context) -> bool {
    ctx.data_mut(|d| d.remove::<u64>(palette_seen_id()));
    let pressed = palette_shortcut(ctx);
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(palette_seen_id(), pass));
    pressed
}

/// The command palette: a search over the page's commands in their groups, narrowed as one types,
/// the arrows moving the highlight and Enter running it.
pub struct CommandPalette<'a> {
    id: Id,
    groups: &'a [CommandGroup],
    query: &'a mut String,
    highlight: &'a mut usize,
    words: PaletteWords,
    running: Option<(usize, usize)>,
    searching: bool,
}

/// When the palette's running command, and its search, began, on the context's clock.
#[derive(Clone, Copy, Default)]
struct Since {
    running: Option<f64>,
    searching: Option<f64>,
}

impl<'a> CommandPalette<'a> {
    pub fn new(
        id: impl egui::AsIdSalt,
        groups: &'a [CommandGroup],
        query: &'a mut String,
        highlight: &'a mut usize,
    ) -> Self {
        Self {
            id: Id::unique(id),
            groups,
            query,
            highlight,
            words: PaletteWords::default(),
            running: None,
            searching: false,
        }
    }

    /// The command that is running, by its group and index, as F23 holds it: the palette stays
    /// open (Escape, Mod+K and the shade do nothing) and other choices are ignored, the command's
    /// row says it runs, and after `busy-release` a Stop waiting row at the list's top, and Escape,
    /// let it go (`PaletteOutcome::stop_waiting`).
    pub fn running(mut self, command: Option<(usize, usize)>) -> Self {
        self.running = command;
        self
    }

    /// The app's own search is out: after `wait-shown` the palette says Searching where nothing is
    /// listed, or dims the earlier rows and passes over them; an answer sooner draws neither.
    pub fn searching(mut self, searching: bool) -> Self {
        self.searching = searching;
        self
    }

    pub fn words(mut self, words: PaletteWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ctx: &Context) -> PaletteOutcome {
        let t = tokens(ctx);
        // how long the command has run and the search been out, on the context's clock
        let now = ctx.input(|i| i.time);
        let since_id = self.id.with("since");
        let mut since: Since = ctx.data(|d| d.get_temp(since_id)).unwrap_or_default();
        since.running = self.running.map(|_| since.running.unwrap_or(now));
        since.searching = self.searching.then(|| since.searching.unwrap_or(now));
        ctx.data_mut(|d| d.insert_temp(since_id, since));
        let after = |start: Option<f64>, key: &str, or: f32| {
            start.map(|start| {
                let wait = f64::from(t.duration(key).unwrap_or(or)) / 1000.0;
                let left = start + wait - now;
                if left > 0.0 {
                    ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
                }
                left <= 0.0
            })
        };
        let busy = self.running.is_some();
        let released = after(since.running, "busy-release", 5000.0).unwrap_or(false);
        let waiting = after(since.searching, "wait-shown", 300.0).unwrap_or(false);
        let needle = self.query.to_lowercase();
        // the commands that match, by group and index, in the order they show
        let shown: Vec<(usize, usize)> = self
            .groups
            .iter()
            .enumerate()
            .flat_map(|(g, group)| {
                let needle = needle.clone();
                group.items.iter().enumerate().filter_map(move |(i, item)| match item {
                    MenuItem::Action {
                        label,
                        keywords,
                        detail,
                        ..
                    } if label.to_lowercase().contains(&needle)
                        || keywords.iter().any(|k| k.to_lowercase().contains(&needle))
                        || detail.as_ref().is_some_and(|d| d.to_lowercase().contains(&needle)) =>
                    {
                        Some((g, i))
                    }
                    _ => None,
                })
            })
            .collect();
        // Mod+K closes an open palette, as it opens a closed one
        let toggled = mod_k(ctx);
        let (down, up, home, end, enter) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::ArrowDown),
                i.consume_key(Modifiers::NONE, Key::ArrowUp),
                i.consume_key(Modifiers::NONE, Key::Home),
                i.consume_key(Modifiers::NONE, Key::End),
                i.consume_key(Modifiers::NONE, Key::Enter),
            )
        });
        if shown.is_empty() {
            *self.highlight = 0;
        } else {
            // the arrows stop at the first and the last row; Home and End reach them
            let last = shown.len() - 1;
            if down {
                *self.highlight = (*self.highlight + 1).min(last);
            }
            if up {
                *self.highlight = self.highlight.saturating_sub(1);
            }
            if home {
                *self.highlight = 0;
            }
            if end {
                *self.highlight = last;
            }
            *self.highlight = (*self.highlight).min(last);
        }
        let mut outcome = PaletteOutcome::default();
        let escape = ctx.input(|i| i.key_pressed(Key::Escape));
        if busy {
            // held while the command runs; once released, Escape lets it go
            if released && escape {
                outcome.stop_waiting = true;
                outcome.close = true;
                return outcome;
            }
        } else if toggled {
            outcome.close = true;
            return outcome;
        }
        // a choice is ignored while a command runs, or while the earlier rows wait on a search
        let choosing = !busy && !(self.searching && waiting);
        if choosing
            && enter
            && let Some(choice) = shown.get(*self.highlight)
        {
            outcome.run = Some(*choice);
        }
        // a dialog a third of the way down the window, as wide as `max-w-sm`
        let window = ctx.content_rect();
        let width = 384.0_f32.min(window.width() - 32.0);
        let inset = t.length("palette-inset");
        let row_x = t.length("palette-row-inset-x");
        let area = egui::Area::new(self.id.with("area"))
            .anchor(egui::Align2::CENTER_TOP, vec2(0.0, (window.height() / 3.0).round()));
        let moved = down || up;
        let response = egui::Modal::new(self.id)
            .area(area)
            .backdrop_color(t.color_at("shade", 0.1))
            .frame(super::dialog_frame(&t).inner_margin(Margin::same(inset as i8)))
            .show(ctx, |ui| {
                let inner = width - 2.0 - 2.0 * inset;
                ui.set_width(inner);
                ui.spacing_mut().item_spacing.y = 0.0;
                // the search: a field on the input's tint, 4 in from the palette's own inset
                ui.add_space(4.0);
                let (field, _) = ui.allocate_exact_size(vec2(inner, t.density("control")), Sense::hover());
                let field = field.shrink2(vec2(4.0, 0.0));
                ui.painter().rect(
                    field,
                    t.radius("control"),
                    t.color_at("input", 0.3),
                    Stroke::new(1.0, t.color_at("input", 0.3)),
                    egui::StrokeKind::Inside,
                );
                icons::paint(
                    ui.painter(),
                    pos2(field.left() + 16.0, field.center().y),
                    Icon::Search,
                    16.0,
                    t.color_at("popover-foreground", 0.5),
                );
                let text = egui::Rect::from_min_max(pos2(field.left() + 31.0, field.top()), field.max - vec2(8.0, 0.0));
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(text)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                // centered in the floor egui 0.37 sets a text edit's height to
                let edit = TextEdit::singleline(self.query)
                    .align(egui::Align2::LEFT_CENTER)
                    .frame(Frame::NONE)
                    .margin(Margin::ZERO)
                    .font(TextRole::Body.font(&t))
                    .text_color(t.color("popover-foreground"))
                    .hint_text(
                        TextRole::Body
                            .rich(&t, &self.words.placeholder)
                            .color(t.color("muted-foreground")),
                    )
                    .desired_width(text.width());
                child.add(edit).accessible_name(&self.words.placeholder).request_focus();
                // released, a row at the list's top lets the running command go
                if busy && released {
                    ui.add_space(4.0);
                    let stop = MenuItem::action(self.words.stop_waiting.clone()).icon(Icon::X);
                    let (_, clicked) = ui
                        .horizontal(|ui| {
                            ui.add_space(4.0);
                            ui.allocate_ui(vec2(inner - 8.0, 34.0), |ui| {
                                action_row(ui, &t, self.id.with("stop-waiting"), &stop, false, RowLook::Palette)
                            })
                            .inner
                        })
                        .inner;
                    if clicked {
                        outcome.stop_waiting = true;
                    }
                }
                if shown.is_empty() {
                    let words = if self.searching && waiting {
                        &self.words.searching
                    } else {
                        &self.words.no_match
                    };
                    ui.add_space(24.0);
                    ui.vertical_centered(|ui| {
                        ui.label(TextRole::Body.rich(&t, words).color(t.color("muted-foreground")));
                    });
                    ui.add_space(24.0);
                    return;
                }
                // the earlier rows, dimmed and passed over, while the search is out past its wait
                if self.searching && waiting {
                    ui.multiply_opacity(0.5);
                }
                // the groups, scrolling past 384 and fading at the foot: each 4 in, its title 32 tall, a
                // rule between them
                let list = egui::ScrollArea::vertical()
                    .id_salt(self.id.with("list"))
                    .max_height(384.0)
                    .min_scrolled_height(384.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let mut last_group = None;
                        for (position, (g, i)) in shown.iter().enumerate() {
                            if last_group != Some(*g) {
                                if last_group.is_some() {
                                    ui.add_space(4.0);
                                    let (rule, _) = ui.allocate_exact_size(vec2(inner, 1.0), Sense::hover());
                                    ui.painter().hline(
                                        (rule.left() - inset)..=(rule.right() + inset),
                                        rule.center().y,
                                        Stroke::new(1.0, t.color("border")),
                                    );
                                }
                                ui.add_space(4.0);
                                let (head, _) = ui.allocate_exact_size(vec2(inner, 32.0), Sense::hover());
                                let title = WidgetText::from(TextRole::Badge.rich(&t, &self.groups[*g].title))
                                    .into_galley(
                                        ui,
                                        Some(TextWrapMode::Truncate),
                                        inner - 2.0 * row_x,
                                        TextRole::Badge.font(&t),
                                    );
                                ui.painter().galley(
                                    pos2(head.left() + 4.0 + row_x, head.center().y - title.size().y / 2.0),
                                    title,
                                    t.color("muted-foreground"),
                                );
                                last_group = Some(*g);
                            }
                            let item = &self.groups[*g].items[*i];
                            let (hit, clicked) = ui
                                .horizontal(|ui| {
                                    ui.add_space(4.0);
                                    ui.allocate_ui(vec2(inner - 8.0, 34.0), |ui| {
                                        action_row(
                                            ui,
                                            &t,
                                            self.id.with((g, i)),
                                            item,
                                            position == *self.highlight,
                                            RowLook::Palette,
                                        )
                                    })
                                    .inner
                                })
                                .inner;
                            if position == *self.highlight && moved {
                                hit.scroll_to_me(None);
                            }
                            if self.running == Some((*g, *i)) {
                                // the running command's spinner and word in its keys' place
                                let words = WidgetText::from(
                                    TextRole::Help
                                        .rich(&t, &self.words.running)
                                        .color(t.color("muted-foreground")),
                                )
                                .into_galley(
                                    ui,
                                    Some(TextWrapMode::Extend),
                                    f32::INFINITY,
                                    TextRole::Help.font(&t),
                                );
                                let right = hit.rect.right() - row_x;
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_max(
                                        pos2(right - words.size().x - 22.0, hit.rect.top() + 2.0),
                                        pos2(right + 2.0, hit.rect.bottom() - 2.0),
                                    ),
                                    0.0,
                                    if position == *self.highlight {
                                        t.color("accent")
                                    } else {
                                        t.color("popover")
                                    },
                                );
                                super::spinner(
                                    ui,
                                    ui.painter(),
                                    pos2(right - words.size().x - 12.0, hit.rect.center().y),
                                    5.0,
                                    t.color("muted-foreground"),
                                );
                                ui.painter().galley(
                                    pos2(right - words.size().x, hit.rect.center().y - words.size().y / 2.0),
                                    words,
                                    t.color("muted-foreground"),
                                );
                                // what a screen reader hears of the running row
                                let said = self.words.running.clone();
                                ui.interact(hit.rect, self.id.with("running"), Sense::hover())
                                    .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &said));
                            }
                            if choosing && hit.hovered() && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
                                *self.highlight = position;
                            }
                            if choosing && clicked {
                                outcome.run = Some((*g, *i));
                            }
                        }
                        ui.add_space(4.0);
                    });
                // the foot fades while more is below, as shadcn's `scroll-fade-b`: over its last
                // twelfth, at most 40, to nothing at the edge
                let below = list.content_size.y - list.inner_rect.height() - list.state.clamped_offset().y;
                if below > 0.5 {
                    let size = (list.inner_rect.height() * 0.12).min(40.0);
                    let fade = egui::Rect::from_min_max(
                        egui::pos2(list.inner_rect.left(), list.inner_rect.bottom() - size),
                        list.inner_rect.max,
                    );
                    ui.painter().add(egui::Shape::gradient_rect(
                        fade,
                        egui::epaint::Direction::TopDown,
                        [egui::Color32::TRANSPARENT, t.color("popover")],
                    ));
                }
            });
        // a running command holds the palette until it is let go
        outcome.close = if busy {
            outcome.stop_waiting
        } else {
            response.should_close() || outcome.run.is_some()
        };
        outcome
    }
}
