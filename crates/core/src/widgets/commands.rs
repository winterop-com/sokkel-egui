//! Palette groups built from what an app already has, after sokkel-ui's `lib/commands`: its places
//! from the rail's groups, and the look's parts (scheme, density, theme) with the one in use hinted.

use crate::{Appearance, CommandGroup, Icon, MenuItem, RailGroup, Scheme, ThemeInfo};

/// The places in `groups` as one palette group titled `title` ("Go to" in sokkel-ui), each command
/// `place-{id}` with the entry's label and icon; `None` when the groups hold no entries.
pub fn place_commands(groups: &[RailGroup], title: impl Into<String>) -> Option<CommandGroup> {
    let items: Vec<MenuItem> = groups
        .iter()
        .flat_map(|group| &group.entries)
        .map(|entry| {
            MenuItem::action(&entry.label)
                .icon(entry.icon)
                .id(format!("place-{}", entry.id))
        })
        .collect();
    (!items.is_empty()).then(|| CommandGroup::new(title, items))
}

/// The place a `place-{id}` command goes to.
pub fn place_of(command: &str) -> Option<&str> {
    command.strip_prefix("place-")
}

/// The words the look's commands say, English by default.
#[derive(Clone, Debug)]
pub struct AppearanceCommandWords {
    pub scheme: String,
    pub light: String,
    pub dark: String,
    pub system: String,
    pub density: String,
    pub regular: String,
    pub compact: String,
    pub theme: String,
    /// The hint on the choice in use.
    pub current: String,
}

impl Default for AppearanceCommandWords {
    fn default() -> Self {
        Self {
            scheme: "Scheme".into(),
            light: "Light".into(),
            dark: "Dark".into(),
            system: "System".into(),
            density: "Density".into(),
            regular: "Regular".into(),
            compact: "Compact".into(),
            theme: "Theme".into(),
            current: "in use".into(),
        }
    }
}

/// The look's palette groups: Scheme (light, dark, the system's), Density (regular, compact) and
/// Theme (the `themes` given), the choice in use hinted. Run one with [`run_appearance_command`].
pub fn appearance_commands(
    appearance: &Appearance,
    themes: &[&ThemeInfo],
    words: &AppearanceCommandWords,
) -> Vec<CommandGroup> {
    let hint = |item: MenuItem, in_use: bool| if in_use { item.hint(&words.current) } else { item };
    let schemes = [
        (Scheme::Light, "light", &words.light, Icon::Sun),
        (Scheme::Dark, "dark", &words.dark, Icon::Moon),
        (Scheme::System, "system", &words.system, Icon::Monitor),
    ];
    let densities = [("regular", &words.regular), ("compact", &words.compact)];
    vec![
        CommandGroup::new(
            &words.scheme,
            schemes.map(|(scheme, id, label, icon)| {
                let item = MenuItem::action(label)
                    .icon(icon)
                    .id(format!("scheme-{id}"))
                    .keywords([&words.scheme, &words.theme]);
                hint(item, appearance.scheme == scheme)
            }),
        ),
        CommandGroup::new(
            &words.density,
            densities.map(|(id, label)| {
                let item = MenuItem::action(label)
                    .icon(Icon::Rows3)
                    .id(format!("density-{id}"))
                    .keywords([&words.density]);
                hint(item, appearance.density == id)
            }),
        ),
        CommandGroup::new(
            &words.theme,
            themes.iter().map(|theme| {
                let item = MenuItem::action(&theme.label)
                    .icon(Icon::Palette)
                    .id(format!("theme-{}", theme.name))
                    .keywords([&words.theme]);
                hint(item, appearance.theme == theme.name)
            }),
        ),
    ]
}

/// Run a command [`appearance_commands`] made on `appearance`; false for a command that is not one.
pub fn run_appearance_command(appearance: &mut Appearance, command: &str) -> bool {
    if let Some(scheme) = command.strip_prefix("scheme-") {
        appearance.scheme = match scheme {
            "light" => Scheme::Light,
            "dark" => Scheme::Dark,
            _ => Scheme::System,
        };
    } else if let Some(density) = command.strip_prefix("density-") {
        appearance.density = density.to_owned();
    } else if let Some(theme) = command.strip_prefix("theme-") {
        appearance.theme = theme.to_owned();
    } else {
        return false;
    }
    true
}
