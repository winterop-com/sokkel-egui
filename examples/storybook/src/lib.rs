//! sokkel-ui-egui's storybook: every story in a sidebar grouped as sokkel-ui's Storybook groups
//! them, a toolbar for the theme, the scheme and the density, and the chosen story on a canvas.

use eframe::egui::{self, Align, ComboBox, Frame, Layout, Margin, Sense, Stroke, Ui, vec2};
use sokkel_ui_egui::{Appearance, Scheme, TextRole, densities, install, themes, tokens};

pub mod stories;

use stories::{STORIES, State, Story};

const APPEARANCE_KEY: &str = "sokkel-ui-egui/appearance";
const STORY_KEY: &str = "sokkel-ui-egui/story";

pub struct Storybook {
    pub appearance: Appearance,
    settings: Option<String>,
    /// The storybook's own palette, on Mod+K: its query and highlight while open.
    palette: Option<(String, usize)>,
    story: usize,
    search: String,
    state: State,
}

impl Storybook {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let storage = cc.storage;
        let appearance = storage
            .and_then(|s| eframe::get_value(s, APPEARANCE_KEY))
            .unwrap_or_default();
        let mut book = Self::with(appearance);
        // tiles only in the running app: tests draw the map without the network
        book.state.map = sokkel_ui_egui_maps::MapView::new(60.4, 11.5, 5.0).with_tiles(
            &cc.egui_ctx,
            "sokkel-ui-egui-storybook/0.1 (a component library's example maps)",
            std::env::temp_dir().join("sokkel-ui-egui-tiles").into(),
        );
        if let Some(path) = storage.and_then(|s| eframe::get_value::<String>(s, STORY_KEY)) {
            book.select(&path);
        }
        book
    }

    pub fn with(appearance: Appearance) -> Self {
        Self {
            appearance,
            settings: None,
            palette: None,
            story: 0,
            search: String::new(),
            state: State::default(),
        }
    }

    /// Show the story at `path`, such as `Foundations/Colors`; an unknown path changes nothing.
    pub fn select(&mut self, path: &str) -> bool {
        match STORIES.iter().position(|story| story.path == path) {
            Some(index) => {
                self.story = index;
                true
            }
            None => false,
        }
    }

    pub fn story(&self) -> &'static Story {
        &STORIES[self.story]
    }

    /// The whole storybook; `install` runs first so a change in the toolbar shows on the next frame.
    pub fn show(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        let t = tokens(ui.ctx());
        egui::Panel::left("stories")
            .resizable(true)
            .default_size(240.0)
            .size_range(200.0..=360.0)
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .fill(t.color("sidebar"))
                    .stroke(Stroke::new(1.0, t.color("sidebar-border")))
                    .inner_margin(Margin::symmetric(8, 12)),
            )
            .show(ui, |ui| self.sidebar(ui));
        egui::Panel::top("toolbar")
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .fill(t.color("background"))
                    .stroke(Stroke::new(1.0, t.color("border")))
                    .inner_margin(Margin::symmetric(16, 8)),
            )
            .show(ui, |ui| self.toolbar(ui));
        // the storybook's own palette on Mod+K: every story, by its path
        if sokkel_ui_egui::palette_shortcut(ui.ctx()) {
            self.palette = Some((String::new(), 0));
        }
        if let Some((mut query, mut highlight)) = self.palette.take() {
            let groups = [sokkel_ui_egui::CommandGroup::new(
                "Stories",
                STORIES
                    .iter()
                    .map(|story| sokkel_ui_egui::MenuItem::action(story.path.replace('/', "  /  "))),
            )];
            let outcome = sokkel_ui_egui::CommandPalette::new("storybook-palette", &groups, &mut query, &mut highlight)
                .show(ui.ctx());
            if let Some((_, index)) = outcome.run {
                self.story = index;
            }
            if !outcome.close {
                self.palette = Some((query, highlight));
            }
        }
        // the storybook's own settings, on its real appearance
        if let Some(mut active) = self.settings.take() {
            let sections = stories::settings::sections();
            let outcome = sokkel_ui_egui::SettingsDialog::new("storybook-settings", &sections, &mut active)
                .show(ui.ctx(), |section, ui| {
                    stories::settings::rows(&mut self.state, &mut self.appearance, section, ui)
                });
            if outcome.reset && active == "appearance" {
                self.appearance = Appearance::default();
            }
            if !outcome.close {
                self.settings = Some(active);
            }
        }
        egui::CentralPanel::default()
            .frame(Frame::new().fill(t.color("background")))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    Frame::new()
                        .inner_margin(Margin::same(t.density("page-inset-wide") as i8))
                        .show(ui, |ui| {
                            self.canvas(ui);
                        });
                });
            });
    }

    /// The chosen story's canvas alone, without the sidebar and the toolbar: what a story's snapshot
    /// draws, so adding a story to the sidebar changes no other story's picture.
    pub fn show_story(&mut self, ui: &mut Ui) {
        install(ui.ctx(), &self.appearance);
        let t = tokens(ui.ctx());
        egui::CentralPanel::default()
            .frame(Frame::new().fill(t.color("background")))
            .show(ui, |ui| {
                Frame::new()
                    .inner_margin(Margin::same(t.density("page-inset-wide") as i8))
                    .show(ui, |ui| {
                        self.canvas(ui);
                    });
            });
    }

    fn sidebar(&mut self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            ui.label(
                TextRole::SectionTitle
                    .rich(&t, "sokkel-ui-egui")
                    .color(t.color("sidebar-foreground")),
            );
        });
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text(TextRole::Body.rich(&t, "Search").color(t.color("muted-foreground")))
                .desired_width(f32::INFINITY),
        );
        ui.add_space(8.0);
        let needle = self.search.to_lowercase();
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            let mut shown: Vec<&str> = Vec::new();
            for (index, story) in STORIES.iter().enumerate() {
                if !needle.is_empty() && !story.path.to_lowercase().contains(&needle) {
                    continue;
                }
                let segments: Vec<&str> = story.path.split('/').collect();
                let (groups, name) = segments.split_at(segments.len() - 1);
                // a heading for every group this story enters that the one before it did not
                for (depth, group) in groups.iter().enumerate() {
                    if shown.get(depth) != Some(group) {
                        shown.truncate(depth);
                        shown.push(group);
                        group_heading(ui, group, depth);
                    }
                }
                shown.truncate(groups.len());
                if story_row(ui, name[0], groups.len(), index == self.story).clicked() {
                    self.story = index;
                }
            }
        });
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        ui.horizontal(|ui| {
            ui.label(
                TextRole::Body
                    .rich(&t, self.story().path.replace('/', "  /  "))
                    .color(t.color("muted-foreground")),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let gear = ui.add(
                    sokkel_ui_egui::Button::new("Settings")
                        .icon(sokkel_ui_egui::Icon::Settings)
                        .variant(sokkel_ui_egui::ButtonVariant::Ghost)
                        .size(sokkel_ui_egui::ButtonSize::Sm),
                );
                if gear.clicked() {
                    self.settings = Some("appearance".into());
                }
                ComboBox::from_id_salt("density")
                    .selected_text(self.appearance.density.clone())
                    .show_ui(ui, |ui| {
                        for level in densities() {
                            ui.selectable_value(&mut self.appearance.density, level.to_owned(), level);
                        }
                    });
                for (scheme, label) in [
                    (Scheme::System, "System"),
                    (Scheme::Dark, "Dark"),
                    (Scheme::Light, "Light"),
                ] {
                    ui.selectable_value(&mut self.appearance.scheme, scheme, label);
                }
                let current = themes()
                    .find(|theme| theme.name == self.appearance.theme)
                    .map(|theme| theme.label.clone());
                ComboBox::from_id_salt("theme")
                    .selected_text(current.unwrap_or_default())
                    .show_ui(ui, |ui| {
                        for theme in themes() {
                            ui.selectable_value(&mut self.appearance.theme, theme.name.clone(), &theme.label)
                                .on_hover_text(theme.description.clone().unwrap_or_default());
                        }
                    });
            });
        });
    }

    fn canvas(&mut self, ui: &mut Ui) {
        let t = tokens(ui.ctx());
        let story = self.story();
        let name = story.path.rsplit('/').next().unwrap_or(story.path);
        ui.label(TextRole::PageTitle.rich(&t, name).color(t.color("foreground")));
        ui.add_space(4.0);
        ui.label(TextRole::Body.rich(&t, story.doc).color(t.color("muted-foreground")));
        ui.add_space(t.density("page-gap"));
        (story.show)(&mut self.state, ui);
    }
}

fn group_heading(ui: &mut Ui, title: &str, depth: usize) {
    let t = tokens(ui.ctx());
    ui.add_space(if depth == 0 { 10.0 } else { 4.0 });
    ui.horizontal(|ui| {
        ui.add_space(4.0 + depth as f32 * 12.0);
        let text = if depth == 0 {
            title.to_uppercase()
        } else {
            title.to_owned()
        };
        let role = if depth == 0 { TextRole::Help } else { TextRole::Label };
        let mut rich = role.rich(&t, text).color(t.color("muted-foreground"));
        if depth == 0 {
            rich = rich.extra_letter_spacing(0.8);
        }
        ui.label(rich);
    });
}

fn story_row(ui: &mut Ui, name: &str, depth: usize, selected: bool) -> egui::Response {
    let t = tokens(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::click());
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, name));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = t.radius("control");
        if selected {
            painter.rect_filled(rect, radius, t.color("sidebar-accent"));
        } else if response.hovered() {
            painter.rect_filled(rect, radius, t.color("hover"));
        }
        let ink = if selected {
            t.color("sidebar-accent-foreground")
        } else {
            t.color("sidebar-foreground")
        };
        let galley = egui::WidgetText::from(TextRole::Body.rich(&t, name)).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            rect.width() - 24.0,
            TextRole::Body.font(&t),
        );
        let x = rect.left() + 12.0 + depth.saturating_sub(1) as f32 * 12.0;
        painter.galley(egui::pos2(x, rect.center().y - galley.size().y / 2.0), galley, ink);
    }
    response
}

impl eframe::App for Storybook {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, APPEARANCE_KEY, &self.appearance);
        eframe::set_value(storage, STORY_KEY, &self.story().path.to_owned());
    }
}
