//! A sign-in screen, after sokkel-ui's `SignIn`: a brand pane and a form, the window's height.

use egui::{
    Align, Color32, Galley, Id, Key, Layout, Rect, Response, Sense, Stroke, StrokeKind, TextEdit, TextWrapMode, Ui,
    UiBuilder, WidgetInfo, WidgetText, pos2, vec2,
};
use std::sync::Arc;

use super::sign_in_art::{css_ease, still};
use super::{Checkbox, DesktopWindow, Field, IconButton, dim, focus_ring};
use crate::{Icon, TextRole, Tokens, icons, tokens};

/// What a person typed to sign in; the fields the form does not ask for stay empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SignInValues {
    pub server: String,
    pub user: String,
    pub password: String,
    pub token: String,
    /// Whether the person asked to be remembered: the app keeps the session longer, and may keep
    /// the username to give back as the next sign-in's `default_values`.
    pub remember: bool,
}

/// A small fact under the brand: the instance, the version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignInFact {
    pub label: String,
    pub value: String,
}

impl SignInFact {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

/// The words the sign-in screen says, English by default, as sokkel-ui's `SignInWords`.
#[derive(Clone, Debug)]
pub struct SignInWords {
    pub title: String,
    pub user: String,
    /// A placeholder in the username field; none by default, as the label names it.
    pub user_hint: String,
    pub password: String,
    /// A placeholder in the password field; none by default.
    pub password_hint: String,
    pub show: String,
    pub hide: String,
    pub remember: String,
    pub submit: String,
    pub busy: String,
    /// What a screen reader calls the seam between the panes.
    pub seam: String,
}

impl Default for SignInWords {
    fn default() -> Self {
        Self {
            title: "Sign in".into(),
            user: "Username".into(),
            user_hint: String::new(),
            password: "Password".into(),
            password_hint: String::new(),
            show: "Show the password".into(),
            hide: "Hide the password".into(),
            remember: "Remember me".into(),
            submit: "Sign in".into(),
            busy: "Signing in".into(),
            seam: "Brand pane width".into(),
        }
    }
}

/// The narrowest the brand pane is dragged to, and the room the form keeps beside it.
const BRAND_MIN: f32 = 320.0;
const FORM_ROOM: f32 = 480.0;
/// The width the screen goes from a bar over the form to two panes, as the web's `@3xl`.
const WIDE_FROM: f32 = 768.0;
/// The fields' and the submit's height, as the web's `h-11`.
const TALL: f32 = 44.0;
/// The form's widest, as the web's `max-w-md`.
const FORM_MAX: f32 = 448.0;

/// What the form keeps between frames.
#[derive(Clone, Debug, Default)]
struct State {
    values: SignInValues,
    /// The first frame has been drawn: the values are in and the opening field has focus.
    opened: bool,
    /// The password is shown as typed.
    shown: bool,
    /// An error was shown on a free form last frame, so focus went to the field to try again.
    refused: bool,
    /// When the screen opened, for its parts to rise into place.
    started: f64,
    /// How tall the form was last frame, to center it.
    form_height: f32,
}

/// The brand pane's picture, drawn in the room it is given.
type Art<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A sign-in screen in two panes the room's height, after sokkel-ui's `SignIn`. The brand pane
/// holds `art` over its upper part ([`super::FlowLines`] and [`super::LevelBars`] ship with the
/// library, or the app's own picture), the `mark` and `name` large at its foot, and a few `facts`
/// under them, such as the instance and the version. It is dark in every scheme, in the theme's
/// own dark tokens, unless `dark_brand(false)`. The form pane, centered, opens with an `eyebrow` in
/// the primary ink, the title and a `description`, then a username and a password with the
/// password's show button, each led by an icon unless `icons(false)`, and, with `remember`, a
/// "Remember me" box. `error` says why the last try was refused, over the submit, and puts focus in
/// the password, all of it chosen, to try again; `busy` holds the form while a try is under way,
/// its fields keeping focus but taking no typing. The first empty field takes focus as the form
/// opens. `resizable` puts a seam between the panes to drag the brand pane's width, a double press
/// putting it back; `keep_width` keeps the dragged width with the app's stored state. `reveal`
/// lets the form's parts rise into place as the screen opens, held still when motion is off.
///
/// It answers to its own width: narrower than 768, as on a phone, the brand pane becomes a bar
/// over the form with the mark, the name and the first fact's value, and the art is left out.
/// With `desktop`, in a desktop window whose native title bar is hidden, the brand pane (or the
/// bar) drags the window, and the bar leaves room at its start for the window's buttons.
///
/// [`SignIn::show`] fills the room it is given and answers with the values when the form is sent
/// (the submit pressed, or Enter in a field) and every field it asks for is filled.
pub struct SignIn<'a> {
    id: Id,
    name: String,
    mark: Option<&'static str>,
    mark_tile: bool,
    eyebrow: Option<String>,
    description: Option<String>,
    facts: Vec<SignInFact>,
    art: Option<Art<'a>>,
    brand_width: f32,
    dark_brand: bool,
    resizable: bool,
    keep_width: bool,
    reveal: bool,
    icons: bool,
    remember: bool,
    busy: bool,
    error: Option<String>,
    auto_focus: bool,
    default_values: Option<SignInValues>,
    desktop: Option<DesktopWindow>,
    words: SignInWords,
}

impl<'a> SignIn<'a> {
    /// A sign-in screen for the app called `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Id::unique("sokkel-sign-in"),
            name: name.into(),
            mark: None,
            mark_tile: true,
            eyebrow: None,
            description: None,
            facts: Vec::new(),
            art: None,
            brand_width: 0.5,
            dark_brand: true,
            resizable: false,
            keep_width: false,
            reveal: true,
            icons: true,
            remember: false,
            busy: false,
            error: None,
            auto_focus: true,
            default_values: None,
            desktop: None,
            words: SignInWords::default(),
        }
    }

    /// Where the form keeps its values between frames, for two sign-in screens in one app.
    pub fn id(mut self, id: impl egui::AsIdSalt) -> Self {
        self.id = Id::unique(id);
        self
    }

    /// The app's mark, as SVG drawn in `currentColor`, the way the web takes a mark.
    pub fn mark(mut self, svg: &'static str) -> Self {
        self.mark = Some(svg);
        self
    }

    /// Sets the mark on a tile of the primary ground; turn it off for a mark in its own colors.
    pub fn mark_tile(mut self, tile: bool) -> Self {
        self.mark_tile = tile;
        self
    }

    /// A short line over the title in the primary ink, such as "Welcome to" and the app's name.
    pub fn eyebrow(mut self, eyebrow: impl Into<String>) -> Self {
        self.eyebrow = Some(eyebrow.into());
        self
    }

    /// A line under the title.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn facts(mut self, facts: impl IntoIterator<Item = SignInFact>) -> Self {
        self.facts = facts.into_iter().collect();
        self
    }

    /// The picture over the brand pane's upper part, such as `FlowLines::new()`; decorative, left
    /// out on a phone. It is given the room over the brand.
    pub fn art(mut self, art: impl egui::Widget + 'a) -> Self {
        self.art = Some(Box::new(move |ui: &mut Ui| {
            ui.add(art);
        }));
        self
    }

    /// The brand pane's width in a wide room, as a share of it; half by default.
    pub fn brand_width(mut self, share: f32) -> Self {
        self.brand_width = share.clamp(0.0, 1.0);
        self
    }

    /// The brand pane in the theme's dark tokens in either scheme (the default), or in the page's.
    pub fn dark_brand(mut self, dark: bool) -> Self {
        self.dark_brand = dark;
        self
    }

    /// A seam between the panes that drags the brand pane's width; a double press puts it back.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Keeps the dragged width with the app's stored state, as the web's `widthKey`, for the next
    /// time the app opens.
    pub fn keep_width(mut self, keep: bool) -> Self {
        self.keep_width = keep;
        self
    }

    /// The form's parts rise into place as the screen opens (the default).
    pub fn reveal(mut self, reveal: bool) -> Self {
        self.reveal = reveal;
        self
    }

    /// An icon at the start of each field: a person, a lock (the default).
    pub fn icons(mut self, icons: bool) -> Self {
        self.icons = icons;
        self
    }

    /// Offers a "Remember me" box; its answer reaches the app as `values.remember`.
    pub fn remember(mut self, remember: bool) -> Self {
        self.remember = remember;
        self
    }

    /// Holds the form while a try is under way: the fields keep focus but take no typing, and the
    /// submit waits, turning.
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    /// Why the last try was refused, over the submit; focus goes to the password as it comes.
    pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
        self.error = error.map(Into::into).filter(|e: &String| !e.is_empty());
        self
    }

    /// Puts focus in the first empty field as the form opens (the default).
    pub fn auto_focus(mut self, focus: bool) -> Self {
        self.auto_focus = focus;
        self
    }

    /// The values the form starts with, such as a username kept from the last sign-in with
    /// `remember` on.
    pub fn default_values(mut self, values: SignInValues) -> Self {
        self.default_values = Some(values);
        self
    }

    /// The desktop window the screen stands in, when its native title bar is hidden.
    pub fn desktop(mut self, desktop: DesktopWindow) -> Self {
        self.desktop = Some(desktop);
        self
    }

    pub fn words(mut self, words: SignInWords) -> Self {
        self.words = words;
        self
    }

    /// Whether the fields the form asks for are filled, so it can be sent.
    pub fn ready(values: &SignInValues) -> bool {
        !values.user.trim().is_empty() && !values.password.is_empty()
    }

    /// Fill the whole window with the page, to its edges, whatever margin the `Ui` it is given
    /// has, as `Shell::show_window` fills it: an app's door.
    pub fn show_window(self, ui: &mut Ui) -> Option<SignInValues> {
        let window = ui.ctx().content_rect();
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(window)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(window);
        self.show(&mut child)
    }

    /// Draws the screen over all the room it is given, and answers with the values when the form
    /// is sent.
    pub fn show(mut self, ui: &mut Ui) -> Option<SignInValues> {
        let ctx = ui.ctx().clone();
        let t = tokens(&ctx);
        let id = self.id;
        let now = ctx.input(|i| i.time);
        let mut state: State = ctx.data(|d| d.get_temp(id)).unwrap_or_default();
        if !state.opened {
            state.values = self.default_values.take().unwrap_or_default();
            state.started = now;
        }
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(rect, 0.0, t.color("background"));
        let wide = rect.width() >= WIDE_FROM;

        let main = if wide {
            let width_id = id.with("brand-width");
            let dragged: Option<f32> = if self.keep_width {
                ctx.data_mut(|d| d.get_persisted(width_id))
            } else {
                ctx.data(|d| d.get_temp(width_id))
            };
            let max = (rect.width() - FORM_ROOM).max(BRAND_MIN);
            // the web's `min-w-80` wins over its `max-w`, as CSS has it
            let width = dragged
                .unwrap_or(rect.width() * self.brand_width)
                .min(rect.width() - FORM_ROOM)
                .max(BRAND_MIN);
            let brand = Rect::from_min_size(rect.min, vec2(width, rect.height()));
            self.drag_window(ui, brand, id);
            self.brand_ground(ui, brand, |this, ui| this.brand_pane(ui, brand));
            if self.resizable {
                self.seam(ui, brand, width_id, max);
            }
            Rect::from_min_max(pos2(brand.right(), rect.top()), rect.max)
        } else {
            let bar = self.bar_height(&t);
            let bar = Rect::from_min_size(rect.min, vec2(rect.width(), bar));
            self.drag_window(ui, bar, id);
            self.brand_ground(ui, bar, |this, ui| this.bar(ui, bar));
            Rect::from_min_max(pos2(rect.left(), bar.bottom()), rect.max)
        };

        let sent = self.main(ui, main, wide, id, &mut state);
        state.opened = true;
        ctx.data_mut(|d| d.insert_temp(id, state));
        ui.allocate_rect(rect, Sense::hover());
        sent
    }

    /// In a desktop window, the brand's ground drags the window and a double press zooms it; sensed
    /// before the brand's own parts, so the seam drawn after takes its own presses.
    fn drag_window(&self, ui: &mut Ui, rect: Rect, id: Id) {
        if self.desktop.is_none() {
            return;
        }
        let title = ui.interact(rect, id.with("window-drag"), Sense::click_and_drag());
        if title.double_clicked() {
            let zoomed = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!zoomed));
        } else if title.drag_started() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
    }

    /// The brand's ground over `rect`, in the theme's dark tokens unless the page's are asked for,
    /// and `add` drawn on it in those tokens.
    fn brand_ground(&mut self, ui: &mut Ui, rect: Rect, add: impl FnOnce(&mut Self, &mut Ui)) {
        let theme = tokens(ui.ctx()).theme.clone();
        let dark = self.dark_brand;
        let draw = |ui: &mut Ui| {
            let t = tokens(ui.ctx());
            ui.painter().rect_filled(rect, 0.0, t.color("brand-ground"));
            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| add(self, ui));
        };
        if dark {
            crate::scope(ui, &theme, Some(true), draw);
        } else {
            draw(ui);
        }
    }

    /// The mark on its tile, `side` square with the mark `glyph` square in it, centered in `tile`.
    fn paint_mark(&self, ui: &Ui, t: &Tokens, tile: Rect, glyph: f32) {
        let Some(svg) = self.mark else {
            return;
        };
        let ink = if self.mark_tile {
            ui.painter().rect_filled(tile, t.radius("surface"), t.color("primary"));
            t.color("primary-foreground")
        } else {
            t.color("foreground")
        };
        // the mark's ink is `currentColor`: drawn white, then tinted with the tile's ink
        let white = svg.replace("currentColor", "#ffffff");
        let uri = format!("bytes://sign-in-mark-{}.svg", Id::unique(svg).value());
        egui::Image::from_bytes(uri, white.into_bytes())
            .tint(ink)
            .paint_at(ui, Rect::from_center_size(tile.center(), vec2(glyph, glyph)));
    }

    /// The wide brand pane: the art over its upper part, the mark and name large at its foot, and
    /// the facts under them, 48 in from its edges.
    fn brand_pane(&mut self, ui: &mut Ui, rect: Rect) {
        let t = tokens(ui.ctx());
        // `p-12`, `gap-10` between the art and the brand, `gap-8` between the brand and the facts
        let inner = rect.shrink(48.0);
        let tile = if self.mark.is_some() { 72.0 } else { 0.0 };
        let name_room = (inner.width() - if tile > 0.0 { tile + 20.0 } else { 0.0 }).max(1.0);
        let name = galley(ui, TextRole::BrandName.rich(&t, &self.name), name_room, true);
        let row = tile.max(name.size().y);
        let help = TextRole::Help;
        let mono = small_mono(&t);
        let labels: Vec<Arc<Galley>> = self
            .facts
            .iter()
            .map(|f| galley(ui, help.rich(&t, &f.label), f32::INFINITY, false))
            .collect();
        let label_width = labels.iter().map(|g| g.size().x).fold(0.0, f32::max);
        // a row of facts is 20 tall (`leading-5`), 4 between rows (`gap-y-1`)
        let line = help.line_height(&t);
        let facts_height = if self.facts.is_empty() {
            0.0
        } else {
            self.facts.len() as f32 * line + (self.facts.len() - 1) as f32 * 4.0
        };
        let block = row
            + if self.facts.is_empty() {
                0.0
            } else {
                32.0 + facts_height
            };
        let top = inner.bottom() - block;
        if let Some(art) = self.art.take() {
            let room = Rect::from_min_max(inner.min, pos2(inner.right(), top - 40.0));
            if room.height() > 0.0 {
                ui.scope_builder(UiBuilder::new().max_rect(room), |ui| {
                    ui.set_clip_rect(room.intersect(ui.clip_rect()));
                    art(ui);
                });
            }
        }
        let painter = ui.painter();
        if tile > 0.0 {
            let mark = Rect::from_min_size(pos2(inner.left(), top + (row - tile) / 2.0), vec2(tile, tile));
            self.paint_mark(ui, &t, mark, 40.0);
        }
        let name_x = inner.left() + if tile > 0.0 { tile + 20.0 } else { 0.0 };
        painter.galley(
            pos2(name_x, top + (row - name.size().y) / 2.0),
            name,
            t.color("foreground"),
        );
        // the facts as a grid: the labels as wide as the widest, 40 before the values
        let mut y = top + row + 32.0;
        let value_x = inner.left() + label_width + 40.0;
        for (fact, label) in self.facts.iter().zip(labels) {
            painter.galley(
                pos2(inner.left(), y + (line - label.size().y) / 2.0),
                label,
                t.color("muted-foreground"),
            );
            let value = truncated(
                ui,
                egui::RichText::new(&fact.value).font(mono.clone()),
                (inner.right() - value_x).max(1.0),
            );
            painter.galley(
                pos2(value_x, y + (line - value.size().y) / 2.0),
                value,
                t.color("foreground"),
            );
            y += line + 4.0;
        }
    }

    /// The narrow bar's height: 16 over and under the small tile, or a line of the name.
    fn bar_height(&self, t: &Tokens) -> f32 {
        let content = if self.mark.is_some() {
            40.0_f32.max(TextRole::DetailTitle.line_height(t))
        } else {
            TextRole::DetailTitle.line_height(t)
        };
        // `py-4` and the rule under it
        content + 32.0 + 1.0
    }

    /// The narrow bar over the form: the mark, the name and the first fact's value, as on a phone.
    fn bar(&mut self, ui: &mut Ui, rect: Rect) {
        let t = tokens(ui.ctx());
        let painter = ui.painter();
        painter.hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, t.color("border")));
        let lights = self
            .desktop
            .and_then(|d| d.traffic_lights)
            .map(|room| room.unwrap_or_else(|| t.length("spacing-traffic-lights")));
        let mut x = rect.left() + lights.unwrap_or(20.0);
        let right = rect.right() - 20.0;
        let middle = rect.top() + (rect.height() - 1.0) / 2.0;
        if self.mark.is_some() {
            let tile = Rect::from_min_size(pos2(x, middle - 20.0), vec2(40.0, 40.0));
            self.paint_mark(ui, &t, tile, 24.0);
            x = tile.right() + 12.0;
        }
        // the fact takes only the room the name leaves, so the name is cut short last
        let name = galley(ui, TextRole::DetailTitle.rich(&t, &self.name), f32::INFINITY, false);
        let room = (right - x).max(1.0);
        let fact_room = (room - name.size().x - 12.0).max(0.0);
        let name = if name.size().x > room {
            truncated(ui, TextRole::DetailTitle.rich(&t, &self.name), room)
        } else {
            name
        };
        let name_width = name.size().x;
        painter.galley(pos2(x, middle - name.size().y / 2.0), name, t.color("foreground"));
        if let Some(fact) = self.facts.first()
            && fact_room > 1.0
        {
            let mono = small_mono(&t);
            let value = truncated(ui, egui::RichText::new(&fact.value).font(mono), fact_room);
            let at = pos2(right - value.size().x, middle - value.size().y / 2.0);
            if at.x > x + name_width {
                painter.galley(at, value, t.color("muted-foreground"));
            }
        }
    }

    /// The seam between the panes: 8 wide over the brand pane's edge, a 2px primary line on it
    /// under the pointer, while dragged and while focused; the arrows step it 16, Home and End take
    /// it to its ends, Enter and a double press put it back.
    fn seam(&self, ui: &mut Ui, brand: Rect, width_id: Id, max: f32) {
        let t = tokens(ui.ctx());
        let rect = Rect::from_center_size(pos2(brand.right(), brand.center().y), vec2(8.0, brand.height()));
        let response = ui.interact(rect, width_id.with("seam"), Sense::click_and_drag());
        let width = brand.width();
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(egui::Role::Slider, true, &self.words.seam);
            info.value = Some(f64::from(width));
            info
        });
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
        }
        let mut next: Option<Option<f32>> = None;
        if response.dragged()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            next = Some(Some((pointer.x - brand.left()).clamp(BRAND_MIN, max).round()));
        }
        if response.double_clicked() {
            next = Some(None);
        }
        if response.has_focus() {
            ui.input(|i| {
                if i.key_pressed(Key::ArrowRight) {
                    next = Some(Some((width + 16.0).clamp(BRAND_MIN, max)));
                } else if i.key_pressed(Key::ArrowLeft) {
                    next = Some(Some((width - 16.0).clamp(BRAND_MIN, max)));
                } else if i.key_pressed(Key::Home) {
                    next = Some(Some(BRAND_MIN));
                } else if i.key_pressed(Key::End) {
                    next = Some(Some(max));
                } else if i.key_pressed(Key::Enter) {
                    next = Some(None);
                }
            });
        }
        if let Some(next) = next {
            let keep = self.keep_width;
            ui.ctx().data_mut(|d| match next {
                Some(width) if keep => d.insert_persisted(width_id, width),
                Some(width) => {
                    d.insert_temp(width_id, width);
                }
                None => d.remove::<f32>(width_id),
            });
        }
        if response.hovered() || response.dragged() || response.has_focus() {
            ui.painter().rect_filled(
                Rect::from_center_size(rect.center(), vec2(2.0, rect.height())),
                0.0,
                t.color("primary"),
            );
        }
    }

    /// The form pane: the form centered in a wide room, at the top in a narrow one.
    fn main(&mut self, ui: &mut Ui, main: Rect, wide: bool, id: Id, state: &mut State) -> Option<SignInValues> {
        let pad = if wide { 48.0 } else { 20.0 };
        let width = (main.width() - pad * 2.0).clamp(1.0, FORM_MAX);
        let mut sent = None;
        let mut child = ui.new_child(UiBuilder::new().max_rect(main).layout(Layout::top_down(Align::Min)));
        egui::ScrollArea::vertical()
            .id_salt(id.with("scroll"))
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let top = if wide {
                    ((main.height() - state.form_height) / 2.0).max(pad)
                } else {
                    32.0
                };
                ui.add_space(top);
                let left = if wide {
                    main.center().x - width / 2.0
                } else {
                    main.left() + pad
                };
                let room = Rect::from_min_size(pos2(left, ui.cursor().top()), vec2(width, main.height().max(1.0)));
                let drawn = ui.scope_builder(
                    UiBuilder::new().max_rect(room).layout(Layout::top_down(Align::Min)),
                    |ui| {
                        ui.set_width(width);
                        self.form(ui, id, state)
                    },
                );
                sent = drawn.inner;
                let height = drawn.response.rect.height();
                if (height - state.form_height).abs() > 0.5 {
                    state.form_height = height;
                    if wide {
                        ui.ctx().request_discard("the sign-in form measured");
                    }
                }
                ui.add_space(if wide { pad } else { 32.0 });
            });
        sent
    }

    /// How far a part of the form has risen into place, 0 to 1, the `step`th a beat after the one
    /// before, as the web's `rise-in` over 420ms with 60ms between parts, eased out.
    fn risen(&self, ui: &Ui, state: &State, step: usize) -> f32 {
        if !self.reveal || still(ui) {
            return 1.0;
        }
        let since = ui.input(|i| i.time) - state.started - step as f64 * 0.06;
        let along = (since / 0.42).clamp(0.0, 1.0) as f32;
        if along < 1.0 {
            ui.ctx().request_repaint();
        }
        css_ease(0.0, 0.0, 0.58, 1.0, along)
    }

    /// A part of the form rising into place, `risen` of the way: faded in and moved up from 8 below.
    fn rise<R>(ui: &mut Ui, risen: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
        let drop = 8.0 * (1.0 - risen);
        ui.add_space(drop);
        let inner = ui
            .scope(|ui| {
                ui.multiply_opacity(risen);
                add(ui)
            })
            .inner;
        ui.add_space(-drop);
        inner
    }

    fn form(&mut self, ui: &mut Ui, id: Id, state: &mut State) -> Option<SignInValues> {
        let t = tokens(ui.ctx());
        let words = self.words.clone();
        let busy = self.busy;
        let ready = Self::ready(&state.values) && !busy;
        let user_id = id.with("user");
        let password_id = id.with("password");
        // the first empty field takes focus as the form opens
        if !state.opened && self.auto_focus {
            let opening = if state.values.user.is_empty() || !state.values.password.is_empty() {
                user_id
            } else {
                password_id
            };
            ui.memory_mut(|m| m.request_focus(opening));
        }
        // focus goes to the password, all of it chosen, as an error comes and the form is free
        let refused = self.error.is_some() && !busy;
        if refused && !state.refused {
            ui.memory_mut(|m| m.request_focus(password_id));
            let mut edit = TextEdit::load_state(ui.ctx(), password_id).unwrap_or_default();
            edit.cursor
                .set_char_range(Some(egui::text::CCursorRange::select_all_str(&state.values.password)));
            edit.store(ui.ctx(), password_id);
        }
        state.refused = refused;
        let mut enter = false;

        let risen = self.risen(ui, state, 0);
        Self::rise(ui, risen, |ui| {
            if let Some(eyebrow) = &self.eyebrow {
                ui.add(
                    egui::Label::new(
                        TextRole::Eyebrow
                            .rich(&t, eyebrow.to_uppercase())
                            .color(t.color("primary-ink")),
                    )
                    .wrap(),
                );
                ui.add_space(6.0);
            }
            ui.add(
                egui::Label::new(
                    TextRole::WelcomeTitle
                        .rich(&t, &words.title)
                        .color(t.color("foreground")),
                )
                .wrap(),
            );
            if let Some(description) = &self.description {
                ui.add_space(6.0);
                ui.add(
                    egui::Label::new(TextRole::Body.rich(&t, description).color(t.color("muted-foreground"))).wrap(),
                );
            }
        });
        ui.add_space(20.0);
        let icons = self.icons;
        let risen = self.risen(ui, state, 1);
        let remember = self.remember;
        Self::rise(ui, risen, |ui| {
            let values = &mut state.values;
            let user = Field::new(&words.user).show(ui, |ui| {
                icon_input(
                    ui,
                    user_id,
                    &mut values.user,
                    icons.then_some(Icon::User),
                    None,
                    (&words.user, &words.user_hint),
                    busy,
                )
            });
            enter |= submitted(ui, &user);
            ui.add_space(16.0);
            let shown = &mut state.shown;
            let password = Field::new(&words.password).show(ui, |ui| {
                icon_input(
                    ui,
                    password_id,
                    &mut values.password,
                    icons.then_some(Icon::Lock),
                    Some((shown, &words)),
                    (&words.password, &words.password_hint),
                    busy,
                )
            });
            enter |= submitted(ui, &password);
            if remember {
                ui.add_space(16.0);
                ui.add(
                    Checkbox::new(&mut values.remember, &words.remember)
                        .field()
                        .enabled(!busy),
                );
            }
        });
        ui.add_space(20.0);
        let error = self.error.clone();
        let risen = self.risen(ui, state, 2);
        let pressed = Self::rise(ui, risen, |ui| {
            if let Some(error) = &error {
                alert(ui, &t, error);
                ui.add_space(12.0);
            }
            submit(ui, &t, &words, ready, busy)
        });
        ((pressed || enter) && ready).then(|| state.values.clone())
    }
}

/// Enter pressed in a one-line field: egui lets go of the field's focus, which the web keeps.
fn submitted(ui: &Ui, response: &Response) -> bool {
    if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
        response.request_focus();
        true
    } else {
        false
    }
}

/// Text laid out in the painter's ink (each caller gives its own), wrapped in `width` or kept on
/// one line.
fn galley(ui: &Ui, text: egui::RichText, width: f32, wrap: bool) -> Arc<Galley> {
    let mode = if wrap { TextWrapMode::Wrap } else { TextWrapMode::Extend };
    WidgetText::from(text).into_galley(ui, Some(mode), width, egui::FontSelection::Default)
}

/// The code face in small text (`font-mono text-xs`), as tall in x-height as the web sets mono
/// inside small text: `mono-x-height-small` of the size rather than `mono-x-height`.
fn small_mono(t: &Tokens) -> egui::FontId {
    let size = crate::mono_size(t, TextRole::Help.font(t).size);
    let ratio = match (t.number("mono-x-height-small"), t.number("mono-x-height")) {
        (Some(small), Some(body)) if body > 0.0 => small / body,
        _ => 1.0,
    };
    egui::FontId::monospace(size * ratio)
}

/// Text on one line, cut short with an ellipsis to fit `width`.
fn truncated(ui: &Ui, text: egui::RichText, width: f32) -> Arc<Galley> {
    WidgetText::from(text).into_galley(ui, Some(TextWrapMode::Truncate), width, egui::FontSelection::Default)
}

/// A field 44 tall with an icon at its start, when given, and, for a secret (`show` given), a
/// button at its end that shows it, after the web's `InputGroup`: the `field` ground, the `input`
/// edge, the `ring` edge and a 1px ring while focused. `held` keeps focus but takes no typing, the
/// field at half strength. Its name is what a screen reader calls it, as its label says.
fn icon_input(
    ui: &mut Ui,
    id: Id,
    value: &mut String,
    icon: Option<Icon>,
    show: Option<(&mut bool, &SignInWords)>,
    (name, placeholder): (&str, &str),
    held: bool,
) -> Response {
    let t = tokens(ui.ctx());
    let width = ui.available_width();
    ui.scope(|ui| {
        if held {
            ui.multiply_opacity(super::DISABLED);
        }
        let (rect, ground) = ui.allocate_exact_size(vec2(width, TALL), Sense::click());
        ui.painter().rect_filled(rect, t.radius("control"), t.color("field"));
        let body = TextRole::Body;
        let row = ui.fonts_mut(|f| f.row_height(&body.font(&t)));
        // the icon `pl-3.5` in from the edge, the text `pl-2` after it, or `pl-3.5` in alone
        let start = rect.left() + 1.0 + 14.0;
        let text_left = if icon.is_some() { start + 16.0 + 8.0 } else { start };
        // the show button `icon-xs` square, its addon's `pr-2` less its `-mr-[0.3rem]` in from the
        // edge, the text `pr-1.5` before it; without it the text stops `pr-2.5` in
        let button = show.as_ref().map(|_| {
            let side = t.density("control-xs");
            Rect::from_center_size(
                pos2(rect.right() - 1.0 - 8.0 + 4.8 - side / 2.0, rect.center().y),
                vec2(side, side),
            )
        });
        let text_right = button.map_or(rect.right() - 1.0 - 10.0, |b| b.left() - 6.0);
        let text_rect = Rect::from_min_max(
            pos2(text_left, rect.center().y - row / 2.0),
            pos2(text_right.max(text_left + 1.0), rect.center().y + row / 2.0),
        );
        let secret = show.as_ref().is_some_and(|(shown, _)| !**shown);
        let mut typed = value.clone();
        let mut edit = TextEdit::singleline(&mut typed)
            .id(id)
            .font(body.font(&t))
            .text_color(t.color("foreground"))
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .password(secret)
            .desired_width(text_rect.width());
        if !placeholder.is_empty() {
            edit = edit.hint_text(body.rich(&t, placeholder).color(t.color("muted-foreground")));
        }
        let response = ui
            .scope_builder(UiBuilder::new().max_rect(text_rect), |ui| {
                ui.spacing_mut().interact_size.y = 0.0;
                ui.add(edit)
            })
            .inner
            .accessible_name(name);
        // held, what was typed is let go: the field keeps its value
        if !held && typed != *value {
            *value = typed;
        }
        // a press on the field's ground or its icon puts focus in its text
        if ground.clicked() {
            response.request_focus();
        }
        if ground.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        }
        let focused = response.has_focus();
        let state = if focused { "focus" } else { "rest" };
        let edge = super::edge(
            &t,
            &format!("text-field.base.{state}.control"),
            if focused { "ring" } else { "input" },
        );
        let painter = ui.painter();
        painter.rect_stroke(rect, t.radius("control"), Stroke::new(1.0, edge), StrokeKind::Inside);
        if focused {
            painter.rect_stroke(
                rect,
                t.radius("control"),
                Stroke::new(1.0, t.color("ring")),
                StrokeKind::Outside,
            );
        }
        if let Some(icon) = icon {
            icons::paint(
                painter,
                pos2(start + 8.0, rect.center().y),
                icon,
                16.0,
                t.color("muted-foreground"),
            );
        }
        if let (Some((shown, words)), Some(button)) = (show, button) {
            let (glyph, label) = if *shown {
                (Icon::EyeOff, &words.hide)
            } else {
                (Icon::Eye, &words.show)
            };
            let pressed = ui
                .scope_builder(UiBuilder::new().max_rect(button), |ui| {
                    ui.add(IconButton::new(glyph, label.clone()).xs())
                })
                .inner;
            if pressed.clicked() {
                *shown = !*shown;
            }
        }
        response
    })
    .inner
}

/// The error over the submit, after the web's destructive `Alert` with a description alone: the
/// card ground and a 1px edge, the circled mark and the words in the destructive ink.
fn alert(ui: &mut Ui, t: &Tokens, error: &str) {
    let ink = t.color("destructive");
    egui::Frame::new()
        .fill(t.color("card"))
        .stroke(Stroke::new(1.0, t.color("border")))
        .corner_radius(t.radius("surface"))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let line = TextRole::Body.line_height(t);
                let (mark, _) = ui.allocate_exact_size(vec2(16.0, line), Sense::hover());
                // the mark sits 2 below the line's middle, as `translate-y-0.5`
                icons::paint(
                    ui.painter(),
                    pos2(mark.center().x, mark.top() + 8.0 + 2.0),
                    Icon::CircleAlert,
                    16.0,
                    ink,
                );
                ui.add(egui::Label::new(TextRole::Body.rich(t, error).color(ink)).wrap());
            });
        })
        .response
        .widget_info(|| WidgetInfo::labeled(egui::Role::Alert, true, error));
}

/// The submit, the room's width and 44 tall, after the web's large default `Button`: its words and
/// an arrow after them, or a spinner and the busy words while a try is under way. Shut while the
/// form is not ready, it stays focusable, so focus is kept through a try. True when it was pressed.
fn submit(ui: &mut Ui, t: &Tokens, words: &SignInWords, ready: bool, busy: bool) -> bool {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, TALL), Sense::click());
    let label = if busy { &words.busy } else { &words.submit };
    response.widget_info(|| WidgetInfo::labeled(egui::Role::Button, ready, label));
    let open = ready && !busy;
    let hot = response.hovered() && open;
    if response.hovered() && busy {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Progress);
    } else if response.hovered() && !open {
        ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
    }
    let rect = if hot && response.is_pointer_button_down_on() {
        rect.translate(vec2(0.0, 1.0))
    } else {
        rect
    };
    let key = format!("button.default.{}.container", if hot { "hover" } else { "rest" });
    let ground = super::ground(t, &key, "primary");
    let ground = if hot && t.paint(&key).is_none() {
        t.color_at("primary", 0.8)
    } else {
        ground
    };
    let ink = super::ink(t, &key, "primary-foreground");
    let enabled = ready;
    let radius = t.radius("control");
    let painter = ui.painter();
    painter.rect(rect, radius, dim(ground, enabled), Stroke::NONE, StrokeKind::Inside);
    focus_ring(painter, t, rect, radius, &response);
    let ink: Color32 = dim(ink, enabled);
    let text = galley(ui, TextRole::Action.rich(t, label), f32::INFINITY, false);
    // the glyph and the 6 between it and the words (`gap-1.5`)
    let lead = 16.0 + 6.0;
    let total = text.size().x + lead;
    let mut x = rect.center().x - total / 2.0;
    if busy {
        super::spinner(ui, painter, pos2(x + 8.0, rect.center().y), 16.0 * 0.4, ink);
        x += lead;
        painter.galley(pos2(x, rect.center().y - text.size().y / 2.0), text, ink);
    } else {
        let end = x + text.size().x;
        painter.galley(pos2(x, rect.center().y - text.size().y / 2.0), text, ink);
        icons::paint(
            painter,
            pos2(end + 6.0 + 8.0, rect.center().y),
            Icon::ArrowRight,
            16.0,
            ink,
        );
    }
    response.clicked() && open
}
