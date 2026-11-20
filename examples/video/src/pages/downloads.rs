//! The videos kept on this device, as the web's `pages/Downloads.tsx` lists them: played, saved as
//! a file, or removed.

use serde::Serialize;
use sokkel_egui::format_time;

use crate::catalog::{channel_of, video_by_id};
use crate::clips::rendition_of;
use crate::data::{Data, Toast};
use crate::format::{bytes_of, plural};
use crate::parts::{path_of, watch_href};

/// One download as the table draws it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Kept {
    pub id: &'static str,
    pub title: &'static str,
    pub channel: &'static str,
    pub length: String,
    pub quality: &'static str,
    pub size: String,
    /// When it was downloaded, in milliseconds.
    pub at: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DownloadsView {
    pub loading: bool,
    pub count: String,
    /// The rows as the backend lists them; the table sorts them newest first.
    pub rows: Vec<Kept>,
}

pub fn view(data: &Data) -> DownloadsView {
    let rows: Vec<Kept> = data
        .downloads
        .seen()
        .iter()
        .filter_map(|download| {
            let video = video_by_id(&download.id)?;
            Some(Kept {
                id: video.id,
                title: video.title,
                channel: channel_of(video).name,
                length: format_time(video.duration),
                quality: rendition_of(download.rendition.id()).label,
                size: bytes_of(download.bytes),
                at: download.at,
            })
        })
        .collect();
    let bytes: f64 = data
        .downloads
        .seen()
        .iter()
        .filter(|one| video_by_id(&one.id).is_some())
        .map(|one| one.bytes)
        .sum();
    DownloadsView {
        loading: !data.downloads.loaded(),
        count: format!("{}, {}", plural(rows.len() as f64, "video", None), bytes_of(bytes)),
        rows,
    }
}

/// A row's Play.
pub fn play(data: &mut Data, id: &str) {
    if let Some(video) = video_by_id(id) {
        data.go(path_of(&watch_href(video, None)));
    }
}

/// A row's Remove the download, with a note once it is gone.
pub fn remove(data: &mut Data, id: &str) {
    data.remove_download(id, Some(Toast::plain("Download removed")), false);
}

/// A downloaded file's name, by its video's title.
pub fn file_name(id: &str) -> Option<String> {
    video_by_id(id).map(|video| format!("{}.webm", video.title))
}
