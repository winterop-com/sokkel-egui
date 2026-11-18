//! Downloads, simulated as the web's `downloads.ts` simulates them: an album comes in one song at a
//! time, each a moment apart, its progress kept, so a new session picks a running download up where
//! it was. A kept album says its size, and is removed with a second press.
//!
//! The web moves them on with an interval; here the app calls `Library::advance` as time passes
//! (each frame will do), and each step is taken at the moment the interval would have taken it.

use crate::catalog::{bytes_of, songs_of_album};
use crate::data::{Download, DownloadState, Library};

/// How long one song takes to come in, in milliseconds.
pub const PER_SONG: i64 = 650;

impl Library {
    /// Starts keeping an album on this device.
    pub fn download(&mut self, album: &str, now: i64) {
        let total = songs_of_album(album).len() as u32;
        self.downloads.retain(|one| one.id != album);
        self.downloads.insert(
            0,
            Download {
                id: album.to_owned(),
                state: DownloadState::Running,
                done: 0,
                total,
                bytes: 0.0,
                at: now,
            },
        );
        self.download_tick.get_or_insert(now + PER_SONG);
    }

    /// Stops a running download, or removes a kept one; either way nothing of it stays.
    pub fn remove_download(&mut self, album: &str) {
        self.downloads.retain(|one| one.id != album);
    }

    pub fn remove_all_downloads(&mut self) {
        self.downloads.clear();
    }

    /// Whether some download is coming in.
    pub fn downloading(&self) -> bool {
        self.downloads.iter().any(|row| row.state == DownloadState::Running)
    }

    /// Moves the downloads on to `now`: one song more for each running one at every step due by
    /// then. A download left running in a saved session starts again from the first call.
    pub fn advance(&mut self, now: i64) {
        if self.download_tick.is_none() && self.downloading() {
            self.download_tick = Some(now + PER_SONG);
        }
        while let Some(at) = self.download_tick.filter(|at| *at <= now) {
            if self.tick_downloads(at) {
                self.download_tick = Some(at + PER_SONG);
            } else {
                self.download_tick = None;
            }
        }
    }

    /// One step of every running download at `now`; false when none was running.
    fn tick_downloads(&mut self, now: i64) -> bool {
        let mut any = false;
        for row in self
            .downloads
            .iter_mut()
            .filter(|row| row.state == DownloadState::Running)
        {
            any = true;
            let done = row.total.min(row.done + 1);
            row.done = done;
            if done >= row.total {
                row.state = DownloadState::Kept;
                row.bytes = songs_of_album(&row.id)
                    .iter()
                    .fold(0.0, |sum, song| sum + bytes_of(song));
                row.at = now;
            }
        }
        any
    }
}
