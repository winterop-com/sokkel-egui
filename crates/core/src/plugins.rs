//! How a crate outside the core adds to it, after sokkel-ui's `lib/plugins`: a plugin names what it
//! contributes, and an app installs it with [`register_plugins`] before it draws, after which the
//! core's parts know those contributions by name. The only kind of contribution so far is a
//! [`CodeEditor`](crate::CodeEditor) language, drawn as `CodeLanguage::Plugin(name)`.

use std::sync::Arc;

use egui::{Context, Id};

use crate::CodeLanguage;
pub use crate::widgets::{CodeTag, CodeToken};

/// How a plugin's language reads a document: its tokens as byte ranges, in order and never
/// overlapping, each named by the tag the editor colors it with.
pub type Highlight = Arc<dyn Fn(&str) -> Vec<CodeToken> + Send + Sync>;

/// A language a plugin gives the code editor, by its name and the media types it is written in.
#[derive(Clone)]
pub struct PluginLanguage {
    /// What `CodeLanguage::Plugin` calls it.
    pub name: &'static str,
    pub highlight: Highlight,
    /// The media types [`language_for_media_type`] reads as this language, lower case.
    pub media_types: Vec<&'static str>,
}

impl PluginLanguage {
    pub fn new(name: &'static str, highlight: impl Fn(&str) -> Vec<CodeToken> + Send + Sync + 'static) -> Self {
        Self {
            name,
            highlight: Arc::new(highlight),
            media_types: Vec::new(),
        }
    }

    pub fn media_types(mut self, types: impl IntoIterator<Item = &'static str>) -> Self {
        self.media_types = types.into_iter().collect();
        self
    }
}

impl std::fmt::Debug for PluginLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginLanguage")
            .field("name", &self.name)
            .field("media_types", &self.media_types)
            .finish_non_exhaustive()
    }
}

/// What a crate adds to the core, under its own name.
#[derive(Clone, Debug)]
pub struct SokkelPlugin {
    pub name: &'static str,
    /// Languages the code editor knows by name once the plugin is installed.
    pub languages: Vec<PluginLanguage>,
}

/// The languages installed, each with the plugin it came from, and how many times plugins were
/// installed: an editor reads its document again when that count moves.
#[derive(Clone, Default)]
struct Installed {
    languages: Vec<(&'static str, PluginLanguage)>,
    generation: u64,
}

fn installed_id() -> Id {
    Id::unique("sokkel-plugins")
}

fn installed(ctx: &Context) -> Installed {
    ctx.data(|d| d.get_temp::<Installed>(installed_id()))
        .unwrap_or_default()
}

/// Installs plugins. A plugin installed again, or a language named again, replaces the one before,
/// and every editor showing it takes the new one; a language a plugin installed again no longer
/// gives is gone. An app installs a plugin again when what it was built from changes, such as the
/// names a server of another version knows.
pub fn register_plugins(ctx: &Context, plugins: impl IntoIterator<Item = SokkelPlugin>) {
    let mut now = installed(ctx);
    for plugin in plugins {
        now.languages.retain(|(owner, _)| *owner != plugin.name);
        for language in plugin.languages {
            now.languages.retain(|(_, one)| one.name != language.name);
            now.languages.push((plugin.name, language));
        }
    }
    now.generation += 1;
    ctx.data_mut(|d| d.insert_temp(installed_id(), now));
    ctx.request_repaint();
}

/// The installed language of that name.
pub fn plugin_language(ctx: &Context, name: &str) -> Option<PluginLanguage> {
    installed(ctx)
        .languages
        .into_iter()
        .find(|(_, one)| one.name == name)
        .map(|(_, one)| one)
}

/// The installed language a media type (without parameters) is written in.
pub fn language_for_media_type(ctx: &Context, media_type: &str) -> Option<CodeLanguage> {
    let asked = media_type.to_lowercase();
    installed(ctx)
        .languages
        .into_iter()
        .find(|(_, one)| one.media_types.iter().any(|given| given.to_lowercase() == asked))
        .map(|(_, one)| CodeLanguage::Plugin(one.name))
}

/// How many times plugins were installed, for a part that keeps what it drew from them.
pub(crate) fn generation(ctx: &Context) -> u64 {
    installed(ctx).generation
}
