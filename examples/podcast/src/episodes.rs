//! What several pages draw, as the web's `parts.tsx`: an episode as a row with its star, its
//! download and its menu; a list of episodes, dragged onto the rail or reordered; a grid of shows;
//! and what a page with nothing in it says.

use eframe::egui::Ui;
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, DownloadAsked, DownloadButton, DownloadState as Shown, DropdownMenu, Empty,
    Icon, IconButton, MenuItem, StarButton, StarButtonWords, format_time,
};
use sokkel_egui_media::{
    Artwork, DraggableTrackList, MediaCard, MediaGrid, MediaShape, ReorderableTrackList, Track, TrackList,
    TrackListVariant, TrackProgress,
};

use crate::act::Act;
use crate::app::EPISODE_DRAG;
use crate::backend::{DownloadState, size_of};
use crate::catalog::{Episode, episode_by_id, show_by_id};
use crate::parts::{ago, megabytes};
use crate::script::length_of;
use crate::state::{Podcasts, Where};

/// What a page reads: the provider's state, the time, and the summary being read aloud.
#[derive(Clone, Copy)]
pub struct Here<'a> {
    pub state: &'a Podcasts,
    pub now: i64,
    pub speaking: Option<&'a str>,
    /// Whether Check for new is asking.
    pub checking: bool,
}

/// What the star buttons say: the app's word for keeping an episode is starring it.
pub fn star_words() -> StarButtonWords {
    StarButtonWords {
        star: "Star".into(),
        unstar: "Unstar".into(),
    }
}

/// Where the listener stands with an episode: new, played, or how long is left.
pub fn standing(state: &Podcasts, id: &str) -> Option<String> {
    let row = state.episode(id)?;
    if row.played {
        Some("Played".into())
    } else if row.position > 0.0 {
        Some(format!("{} left", format_time(length_of(id) - row.position)))
    } else if row.inbox {
        Some("New".into())
    } else {
        None
    }
}

/// Parts of a meta line, the missing left out, joined by a middle dot.
pub fn meta(parts: &[Option<String>]) -> String {
    parts.iter().flatten().cloned().collect::<Vec<_>>().join(" \u{b7} ")
}

/// An episode as a row: its show's cover, its title, and a line with the show (or its number), when
/// it came out and where the listener stands.
pub fn track_of(here: &Here, episode: &Episode, with_show: bool) -> Track {
    let row = here.state.episode(&episode.id);
    let show = show_by_id(&episode.show);
    let line = meta(&[
        if with_show {
            show.map(|one| one.title.clone())
        } else {
            Some(format!("Episode {}", episode.number))
        },
        row.map(|row| ago(row.out_at, here.now)),
        standing(here.state, &episode.id),
    ]);
    let mut track = Track::new(&episode.id, &episode.title, length_of(&episode.id)).subtitle(line);
    if let Some(show) = show {
        track = track.artwork(show.cover);
    }
    track
}

/// A menu's items, each with what it asks; a rule asks nothing.
#[derive(Default)]
pub struct Menu {
    pub items: Vec<MenuItem>,
    pub acts: Vec<Option<Act>>,
}

impl Menu {
    pub fn item(&mut self, item: MenuItem, act: Act) {
        self.items.push(item);
        self.acts.push(Some(act));
    }

    pub fn rule(&mut self) {
        self.items.push(MenuItem::Separator);
        self.acts.push(None);
    }

    /// What the entry at `index` asks.
    pub fn act(&self, index: Option<usize>) -> Option<Act> {
        index.and_then(|index| self.acts.get(index).cloned().flatten())
    }
}

/// The menu at the end of an episode's row: the queue, played, archive, the episode and its show,
/// and with `all` its star and its download too, as the web's `EpisodeMenu`.
pub fn episode_menu(state: &Podcasts, id: &str, all: bool) -> Menu {
    let mut menu = Menu::default();
    let (Some(row), Some(episode)) = (state.episode(id), episode_by_id(id)) else {
        return menu;
    };
    menu.item(
        MenuItem::action("Play next").icon(Icon::ListStart),
        Act::QueueAdd(id.to_owned(), Where::Next),
    );
    if state.player.queue.iter().any(|one| one == id) {
        menu.item(
            MenuItem::action("Remove from queue").icon(Icon::ListX),
            Act::QueueRemove(id.to_owned()),
        );
    } else {
        menu.item(
            MenuItem::action("Add to queue").icon(Icon::ListEnd),
            Act::QueueAdd(id.to_owned(), Where::End),
        );
    }
    menu.rule();
    menu.item(
        MenuItem::action(if row.played { "Mark unplayed" } else { "Mark played" }).icon(if row.played {
            Icon::RotateCcw
        } else {
            Icon::Check
        }),
        Act::MarkPlayed(id.to_owned(), !row.played),
    );
    if row.inbox {
        menu.item(
            MenuItem::action("Archive").icon(Icon::Archive),
            Act::Archive(id.to_owned()),
        );
    }
    if all {
        menu.item(
            MenuItem::action(if row.starred { "Unstar" } else { "Star" }).icon(if row.starred {
                Icon::StarOff
            } else {
                Icon::Star
            }),
            Act::Star(id.to_owned(), !row.starred),
        );
        match row.download {
            DownloadState::Idle => menu.item(
                MenuItem::action("Download").icon(Icon::Download),
                Act::Download(id.to_owned()),
            ),
            DownloadState::Running => menu.item(
                MenuItem::action("Cancel the download").icon(Icon::Trash2),
                Act::RemoveDownload(id.to_owned()),
            ),
            DownloadState::Paused | DownloadState::Kept => menu.item(
                MenuItem::action("Remove the download").icon(Icon::Trash2),
                Act::RemoveDownload(id.to_owned()),
            ),
        }
    }
    menu.rule();
    menu.item(
        MenuItem::action("Episode details").icon(Icon::Info),
        Act::Go(format!("episode/{id}")),
    );
    menu.item(
        MenuItem::action("Go to show").icon(Icon::Podcast),
        Act::Go(format!("show/{}", episode.show)),
    );
    menu
}

/// The menu's button, `{label}: {title}`, and what was chosen in it.
pub fn episode_menu_button(ui: &mut Ui, state: &Podcasts, id: &str, label: &str, all: bool, acts: &mut Vec<Act>) {
    let title = episode_by_id(id).map_or(id, |episode| episode.title.as_str());
    let more = ui.add(IconButton::new(Icon::Ellipsis, format!("{label}: {title}")));
    let menu = episode_menu(state, id, all);
    let chosen = DropdownMenu::new(&menu.items).end(true).width(208.0).show(&more);
    acts.extend(menu.act(chosen));
}

/// An episode's download, kept or under way, as an icon, as the web's `EpisodeDownload`.
pub fn episode_download(ui: &mut Ui, state: &Podcasts, id: &str, acts: &mut Vec<Act>) {
    let Some(row) = state.episode(id) else { return };
    let shown = match row.download {
        DownloadState::Idle => Shown::Idle,
        DownloadState::Running => Shown::Running,
        DownloadState::Paused => Shown::Paused,
        DownloadState::Kept => Shown::Kept,
    };
    let out = DownloadButton::new(("episode-download", id), shown)
        .compact(true)
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::Sm)
        .value(row.progress as f32)
        .cancellable()
        .removable()
        .kept_detail(format!("{} on this device", megabytes(size_of(id))))
        .show(ui);
    match out.asked {
        Some(DownloadAsked::Download) => acts.push(Act::Download(id.to_owned())),
        Some(DownloadAsked::Cancel | DownloadAsked::Remove) => acts.push(Act::RemoveDownload(id.to_owned())),
        _ => {}
    }
}

/// A control before a row's own, such as taking it out of the queue: its icon, its name, and what
/// it asks.
pub type Extra = fn(&Episode) -> Option<(Icon, String, Act)>;

/// The controls at the end of an episode's row: its star, its download and its menu, as the web's
/// `EpisodeActions`.
pub fn episode_actions(ui: &mut Ui, state: &Podcasts, id: &str, extra: Option<Extra>, acts: &mut Vec<Act>) {
    let Some(row) = state.episode(id) else { return };
    if let Some((icon, label, act)) = extra.and_then(|extra| episode_by_id(id).and_then(extra))
        && ui.add(IconButton::new(icon, label)).clicked()
    {
        acts.push(act);
    }
    let mut star = row.starred;
    ui.add(StarButton::new(&mut star).words(star_words()));
    if star != row.starred {
        acts.push(Act::Star(id.to_owned(), star));
    }
    episode_download(ui, state, id, acts);
    episode_menu_button(ui, state, id, "More", false, acts);
}

/// How a list of episodes is drawn.
#[derive(Clone, Copy, Default)]
pub struct ListLook {
    /// The show on each row's line, or its number.
    pub with_show: bool,
    /// A press on a row opens its page, its cover playing it.
    pub linked: bool,
    /// Inside a section's box, rather than in a box of its own.
    pub plain: bool,
    /// Each row ends in a grip that reorders the queue.
    pub reorder: bool,
    pub extra: Option<Extra>,
}

/// Episodes as a list, as the web's `EpisodeList`: a press plays one, or pauses the one playing, or
/// with `linked` opens its page; each row ends in its actions, and drags onto the queue, Starred or
/// Downloads in the rail, or with `reorder` moves within the queue.
pub fn episode_list(ui: &mut Ui, here: &Here, id: &str, episodes: &[&Episode], look: ListLook, acts: &mut Vec<Act>) {
    let state = here.state;
    let tracks: Vec<Track> = episodes
        .iter()
        .map(|episode| track_of(here, episode, look.with_show))
        .collect();
    let current = state
        .current()
        .filter(|current| episodes.iter().any(|episode| episode.id == *current));
    let mut row_acts = Vec::new();
    let mut list = TrackList::new(("episodes", id), &tracks)
        .current(current)
        .playing(state.playing)
        .position(state.position())
        .progress(TrackProgress::Bar)
        .linked(look.linked)
        .follow(false)
        .variant(if look.plain {
            TrackListVariant::Plain
        } else {
            TrackListVariant::Boxed
        })
        .actions(|ui, track, _| episode_actions(ui, state, &track.id, look.extra, &mut row_acts));
    if current.is_none() {
        list = list.position(0.0);
    }
    let out = if look.reorder {
        ReorderableTrackList::new(list).show(ui)
    } else {
        DraggableTrackList::new(list, EPISODE_DRAG).show(ui)
    };
    acts.extend(row_acts);
    if let Some(index) = out.played {
        acts.push(Act::Play(tracks[index].id.clone(), None));
    }
    if let Some(index) = out.opened {
        acts.push(Act::Go(format!("episode/{}", tracks[index].id)));
    }
    if let Some((from, to)) = out.moved {
        acts.push(Act::QueueMove(from, to));
    }
}

/// Shows as a grid of covers, each opening the show's page, with a play button for its latest
/// episode and a menu to follow it, as the web's `ShowGrid`.
pub fn show_grid(ui: &mut Ui, here: &Here, shows: &[String], acts: &mut Vec<Act>) {
    let state = here.state;
    MediaGrid::new(MediaShape::Square).show(ui, shows.len(), |ui, index| {
        let id = &shows[index];
        let Some(show) = show_by_id(id) else { return };
        let followed = state.show(id).is_some_and(|row| row.subscribed);
        let of_show = |row: &&crate::backend::EpisodeState| episode_by_id(&row.id).is_some_and(|one| one.show == *id);
        let fresh = state.episodes.iter().filter(|row| row.inbox).filter(of_show).count();
        let latest = state
            .episodes
            .iter()
            .filter(|row| row.out)
            .filter(of_show)
            .max_by_key(|row| row.out_at)
            .map(|row| row.id.clone());
        let mut menu = Menu::default();
        menu.item(
            MenuItem::action(if followed { "Unfollow" } else { "Follow" }),
            if followed {
                Act::Unsubscribe(id.clone())
            } else {
                Act::Subscribe(id.clone())
            },
        );
        menu.item(MenuItem::action("Open show"), Act::Go(format!("show/{id}")));
        let hosts: Vec<&str> = show.hosts.iter().map(|host| host.name()).collect();
        let cover = show.cover;
        let playing = state.playing
            && state
                .current()
                .and_then(episode_by_id)
                .is_some_and(|episode| episode.show == *id);
        let mut card = MediaCard::new(("show", id), &show.title)
            .subtitle(hosts.join(", "))
            .cover(move |painter, rect| Artwork::of(cover).paint(painter, rect))
            .playable(latest.is_some())
            .playing(playing)
            .menu(&menu.items);
        if fresh > 0 {
            card = card.badge(format!("{fresh} new"));
        }
        let out = card.show(ui);
        if out.selected {
            acts.push(Act::Go(format!("show/{id}")));
        }
        if out.play
            && let Some(latest) = &latest
        {
            acts.push(Act::Play(latest.clone(), None));
        }
        acts.extend(menu.act(out.menu));
    });
}

/// What a list with nothing in it says, and the way to fill it: true when that was pressed.
pub fn nothing(ui: &mut Ui, icon: Icon, title: &str, description: &str, action: Option<(&str, Option<Icon>)>) -> bool {
    Empty::new(title).icon(icon).description(description).show(ui, |ui| {
        action.is_some_and(|(label, icon)| {
            let mut button = Button::new(label).variant(ButtonVariant::Outline).size(ButtonSize::Sm);
            if let Some(icon) = icon {
                button = button.icon(icon);
            }
            ui.add(button).clicked()
        })
    })
}
