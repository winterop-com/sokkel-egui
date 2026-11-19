//! The disk the library's root is on, simulated, as the web's `data/disk.ts` has it: its size, what
//! else is on it, and what is left.

use serde::Serialize;

use super::format::GB;
use super::model::{Download, DownloadState, Model, Source};
use crate::js::trim;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Disk {
    /// What the disk is called in a sentence: the volume by its name, or the system disk.
    pub name: String,
    /// Bytes the disk holds in all.
    pub capacity: f64,
    /// Bytes taken by files that are not the library's.
    pub other: f64,
}

/// The disk a root path is on: an external volume under `/Volumes`, the system disk otherwise.
pub fn disk_of(root: &str) -> Disk {
    let volume = root
        .strip_prefix("/Volumes/")
        .map(|rest| rest.split('/').next().unwrap_or(""))
        .filter(|name| !name.is_empty());
    match volume {
        Some(volume) => Disk {
            name: format!("the {volume} volume"),
            capacity: 2000.0 * GB,
            other: 1240.0 * GB,
        },
        None => Disk {
            name: "the system disk".to_owned(),
            capacity: 1000.0 * GB,
            other: 610.0 * GB,
        },
    }
}

/// What the library takes on its disk, what downloads have written so far, and what is free.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Usage {
    pub disk: Disk,
    pub models: f64,
    pub partial: f64,
    pub free: f64,
}

pub fn usage_of(root: &str, library: &[Model], pulling: &[Download]) -> Usage {
    let disk = disk_of(root);
    let models = library.iter().fold(0.0, |sum, model| sum + model.size);
    let partial = pulling
        .iter()
        .filter(|one| one.state != DownloadState::Done && one.state != DownloadState::Cancelled)
        .fold(0.0, |sum, one| sum + one.done);
    let free = (disk.capacity - disk.other - models - partial).max(0.0);
    Usage {
        disk,
        models,
        partial,
        free,
    }
}

/// Where a model's files are kept under the root: `<root>/<source>/<publisher>/<name>-<quant>`.
pub fn path_of(root: &str, source: Source, publisher: &str, name: &str, quant: &str) -> String {
    let root = root.strip_suffix('/').unwrap_or(root);
    format!("{root}/{}/{publisher}/{name}-{quant}", source.key())
}

/// Where a library model's files are kept.
pub fn model_path(root: &str, model: &Model) -> String {
    path_of(root, model.source, &model.publisher, &model.name, &model.quant)
}

/// Whether a root reads as a folder path.
pub fn valid_root(root: &str) -> bool {
    let trimmed = trim(root);
    (trimmed.starts_with('/') || trimmed.starts_with("~/")) && !trimmed.contains('\0') && !root.contains("//")
}
