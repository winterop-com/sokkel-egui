//! One variant's pull, as the web's `pages/pull.tsx` finds it: the pull of it now, the variant a
//! row offers at once, and what its download button says and does in each state.

use crate::data::catalog::{CatalogEntry, Variant, model_id};
use crate::data::format::bytes;
use crate::data::model::{Download, DownloadState, Model};

/// The pull of an entry's variant now: under way, waiting or paused, else the last one that
/// failed, as the web's `pullOf` finds it.
pub fn pull_of<'a>(downloads: &'a [Download], entry: &str, quant: &str) -> Option<&'a Download> {
    let mine: Vec<&Download> = downloads
        .iter()
        .filter(|one| one.entry == entry && one.quant == quant)
        .collect();
    mine.iter()
        .find(|one| one.state.going())
        .or_else(|| mine.iter().find(|one| one.state == DownloadState::Failed))
        .copied()
}

/// The variant a row offers to pull at once: four-bit if there is one, else the first.
pub fn usual_variant(entry: &CatalogEntry) -> &Variant {
    entry
        .variants
        .iter()
        .find(|variant| variant.quant == "Q4_K_M" || variant.quant == "4bit")
        .unwrap_or(&entry.variants[0])
}

/// What a variant's download button shows: kept in the library, ready to pull (or try again), or
/// its pull's share with pause, resume and cancel.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum PullButton {
    Kept {
        detail: String,
        open: String,
        said: String,
    },
    Idle {
        said: String,
        error: Option<String>,
        retry: Option<String>,
    },
    Running {
        value: f64,
        paused: bool,
        queued: bool,
        id: String,
    },
}

/// The button for `variant` of `entry`, given the library and the pulls.
pub fn pull_button(entry: &CatalogEntry, variant: &Variant, models: &[Model], downloads: &[Download]) -> PullButton {
    let id = model_id(&entry.name, &variant.quant);
    if let Some(kept) = models.iter().find(|model| model.id == id) {
        return PullButton::Kept {
            detail: format!("In the library, {}", bytes(kept.size)),
            open: format!("/library/{}", kept.id),
            said: "In library".to_owned(),
        };
    }
    match pull_of(downloads, &entry.id, &variant.quant) {
        None => PullButton::Idle {
            said: format!("Pull {}", variant.quant),
            error: None,
            retry: None,
        },
        Some(now) if now.state == DownloadState::Failed => PullButton::Idle {
            said: "Try again".to_owned(),
            error: now.error.clone(),
            retry: Some(now.id.clone()),
        },
        Some(now) => PullButton::Running {
            value: now.done / now.size,
            paused: now.state == DownloadState::Paused,
            queued: now.state == DownloadState::Queued,
            id: now.id.clone(),
        },
    }
}
