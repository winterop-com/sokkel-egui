//! The settings, as the web's `pages/Settings.tsx` has them: each section's settings with what they
//! say, what a section's reset puts back, and whether a library root typed can be applied.

use serde::Serialize;

use crate::data::disk::{disk_of, valid_root};
use crate::data::model::{Prefs, PrefsPatch, default_prefs};
use crate::engine::runner::MACHINE_MEMORY;
use crate::js::number_to_string;

/// One setting: its id, title and description.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Setting {
    pub id: &'static str,
    pub title: &'static str,
    pub description: String,
}

/// One section: its id, label and settings.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SettingsSection {
    pub id: &'static str,
    pub label: &'static str,
    pub settings: Vec<Setting>,
}

fn setting(id: &'static str, title: &'static str, description: impl Into<String>) -> Setting {
    Setting {
        id,
        title,
        description: description.into(),
    }
}

/// The app's own sections, under the group titled Models, then the demo data's.
pub fn sections(prefs: &Prefs) -> Vec<SettingsSection> {
    vec![
        SettingsSection {
            id: "library",
            label: "Library",
            settings: vec![
                setting(
                    "root",
                    "Library root",
                    format!(
                        "Where the models are kept, on {}. Moving it moves every model with it.",
                        disk_of(&prefs.root).name
                    ),
                ),
                setting(
                    "confirm-delete",
                    "Ask before deleting a model",
                    "Say what goes with it, and how much room it frees, before its files are removed.",
                ),
            ],
        },
        SettingsSection {
            id: "models",
            label: "Chat",
            settings: vec![setting(
                "default-model",
                "Default model",
                "What a new chat starts with, from the palette, the rail or N.",
            )],
        },
        SettingsSection {
            id: "downloads",
            label: "Downloads",
            settings: vec![
                setting(
                    "parallel",
                    "Pulls at a time",
                    "The rest wait their turn, in the order they were asked for.",
                ),
                setting(
                    "bandwidth",
                    "Bandwidth limit",
                    "Shared by every pull under way, so the network stays usable.",
                ),
            ],
        },
        SettingsSection {
            id: "runner",
            label: "Runner",
            settings: vec![
                setting(
                    "memory",
                    "Memory for models",
                    format!(
                        "How much of this machine's {} GB the runner may fill. It unloads the model used longest ago to make room.",
                        number_to_string(MACHINE_MEMORY / 1e9)
                    ),
                ),
                setting(
                    "keep-alive",
                    "Keep idle models loaded",
                    "A model nobody has asked for this long is unloaded, freeing its memory.",
                ),
                setting(
                    "start-runner",
                    "Start the runner with the app",
                    "Off, it waits for Start in the top bar or on the Runner page.",
                ),
                setting(
                    "auto-load",
                    "Load a model when a chat needs it",
                    "Off, a chat with a model not loaded says so, and it is loaded by hand.",
                ),
            ],
        },
    ]
}

/// The demo data's one setting.
pub fn demo() -> SettingsSection {
    SettingsSection {
        id: "demo",
        label: "Demo data",
        settings: vec![setting(
            "reset",
            "Reset demo data",
            "Puts back the fifteen models, the conversations, the runner and these settings, and forgets every pull.",
        )],
    }
}

/// What Reset to defaults puts back in a section, and leaves the rest; none for a section of
/// another kind.
pub fn reset(section: &str) -> Option<PrefsPatch> {
    let defaults = default_prefs();
    let patch = match section {
        "library" => PrefsPatch {
            root: Some(defaults.root),
            confirm_delete: Some(defaults.confirm_delete),
            ..PrefsPatch::default()
        },
        "models" => PrefsPatch {
            default_model: Some(defaults.default_model),
            ..PrefsPatch::default()
        },
        "downloads" => PrefsPatch {
            parallel: Some(defaults.parallel),
            bandwidth: Some(defaults.bandwidth),
            ..PrefsPatch::default()
        },
        "runner" => PrefsPatch {
            memory: Some(defaults.memory),
            keep_alive: Some(defaults.keep_alive),
            start_runner: Some(defaults.start_runner),
            auto_load: Some(defaults.auto_load),
            ..PrefsPatch::default()
        },
        _ => return None,
    };
    Some(patch)
}

/// Whether a root typed can be applied: a folder path that differs from the root now.
pub fn root_applies(text: &str, root: &str) -> bool {
    valid_root(text) && crate::js::trim(text) != root
}

/// What the app logs and says once the library moves.
pub fn moved(root: &str, models: usize) -> (String, String) {
    (
        format!("library moved to {root} on {}", disk_of(root).name),
        format!("{models} models are now under {root}"),
    )
}
