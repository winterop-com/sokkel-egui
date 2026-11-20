//! The pages, as the web's `pages/*.tsx` draw them over the views in `pages`: Home, a channel,
//! Subscriptions, the Library, History, Watch later and a playlist, Liked videos, Playlists,
//! Downloads, the search, and a page for an address that leads nowhere; and a video's card and its
//! menu, as the web's `parts.tsx`. The watch page is `watch_page`.

use eframe::egui::{self, Painter, Rect, Ui};
use sokkel_egui::{
    Button, ButtonSize, ButtonVariant, Cell, ChipBar, Column, DataTable, DescriptionList, Empty, Gap, Icon, MenuItem,
    Page, PageHeader, QuickAdd, SearchField, Section, Segmented, Stack, SwitchField, Tabs, TabsVariant, TextRole,
    relative_time, tokens, toolbar,
};
use sokkel_egui_media::{Artwork, ChannelRow, MediaCard, MediaCardLayout, MediaGrid, MediaShape};

use crate::act::{Act, Asking};
use crate::app::{VideoApp, Views};
use crate::catalog::{Channel, Video, channel_by_id, video_by_id};
use crate::clips::{draw_frame, poster_time, spec_of};
use crate::data::{PrefsPatch, WATCH_LATER};
use crate::format::plural;
use crate::pages;
use crate::parts::{CardLayout, path_of, video_menu};

/// The page the address names, in the page's scroller; the watch page is drawn by `watch_page`.
pub fn page(ui: &mut Ui, app: &VideoApp, page: &str, views: &mut Views, acts: &mut Vec<Act>) {
    let (_, second, third, _) = app.place();
    if page == "downloads" {
        Page::show(ui, |ui| {
            Stack::new(Gap::Lg).show(ui, |ui| downloads(ui, app, views, acts))
        });
        return;
    }
    // the web's pages stand in a `Stack gap="lg"`, the library's in `xl`
    let gap = if page == "library" { Gap::Xl } else { Gap::Lg };
    Page::scroll(ui, |ui| {
        Stack::new(gap).show(ui, |ui| match page {
            "home" => home(ui, app, views, acts),
            "watch" => crate::watch_page::watch(ui, app, views, acts),
            "channel" => match second.as_deref().and_then(channel_by_id) {
                Some(channel) => channel_page(ui, app, channel, third.as_deref() == Some("about"), views, acts),
                None => gone(ui, app, "channel", acts),
            },
            "subscriptions" => subscriptions(ui, app, views, acts),
            "library" => library(ui, app, acts),
            "history" => history(ui, app, views, acts),
            "later" => playlist(ui, app, WATCH_LATER, acts),
            "liked" => liked(ui, app, acts),
            "playlists" => playlists(ui, app, views, acts),
            "playlist" => playlist(ui, app, second.as_deref().unwrap_or_default(), acts),
            "search" => search(ui, app, second.as_deref().unwrap_or_default(), views, acts),
            _ => gone(ui, app, "page", acts),
        })
    });
}

/// The video's still: a frame of its own clip a third of the way in, painted from its scene.
pub fn poster(video: &'static Video) -> impl Fn(&Painter, Rect) {
    let spec = spec_of(video);
    let at = poster_time(video);
    move |painter, rect| draw_frame(painter, rect, &spec, at, false)
}

/// A small outline button, with an icon when given.
pub fn outline(label: &str, icon: Option<Icon>) -> Button {
    let button = Button::new(label).variant(ButtonVariant::Outline).size(ButtonSize::Sm);
    match icon {
        Some(icon) => button.icon(icon),
        None => button,
    }
}

/// An entry of a card's menu, and what it does.
pub type Entry = (MenuItem, Act);

/// A video's card, as the web's `VideoCard`: its still, its length and how far the viewer got,
/// its title, channel, views and age, and its menu (Watch later, Save to, Share, its channel, then
/// the page's own `extra`).
#[allow(clippy::too_many_arguments)]
pub fn video_card(
    ui: &mut Ui,
    app: &VideoApp,
    context: &str,
    video: &'static Video,
    layout: CardLayout,
    bare: bool,
    list: Option<&str>,
    playing: bool,
    extra: Vec<Entry>,
    acts: &mut Vec<Act>,
) {
    let card = crate::parts::video_card(&app.data, video, layout, bare, list, playing);
    let subtitle = match card.channel {
        Some(channel) => format!("{channel}\n{}", card.meta.join(" \u{b7} ")),
        None => card.meta.join(" \u{b7} "),
    };
    let words = video_menu(&app.data, video);
    let icons = [Icon::Clock, Icon::ListPlus, Icon::Share2, Icon::User];
    let own = [
        Act::WatchLater(video.id.to_owned()),
        Act::Save(video.id.to_owned()),
        Act::Share(video.id.to_owned(), None),
        Act::Go(format!("channel/{}", video.channel)),
    ];
    let mut items: Vec<MenuItem> = words
        .iter()
        .zip(icons)
        .map(|(label, icon)| MenuItem::action(label).icon(icon))
        .collect();
    let mut does: Vec<Option<Act>> = own.into_iter().map(Some).collect();
    if !extra.is_empty() {
        items.push(MenuItem::Separator);
        does.push(None);
        for (item, act) in extra {
            items.push(item);
            does.push(Some(act));
        }
    }
    let mut media = MediaCard::new(("video-card", context, video.id), card.title)
        .shape(MediaShape::Wide)
        .layout(if layout == CardLayout::Row {
            MediaCardLayout::Row
        } else {
            MediaCardLayout::Stack
        })
        .subtitle(subtitle)
        .cover(poster(video))
        .badge(card.badge)
        .playing(card.playing)
        .menu(&items);
    if let Some(progress) = card.progress {
        media = media.progress(progress as f32);
    }
    let out = media.show(ui);
    if out.selected {
        acts.push(Act::Go(path_of(&card.href)));
    }
    if let Some(index) = out.menu
        && let Some(Some(act)) = does.get(index)
    {
        acts.push(act.clone());
    }
}

/// Videos in a grid of wide cards.
fn grid(ui: &mut Ui, app: &VideoApp, context: &str, videos: &[&'static Video], bare: bool, acts: &mut Vec<Act>) {
    MediaGrid::new(MediaShape::Wide).show(ui, videos.len(), |ui, index| {
        video_card(
            ui,
            app,
            context,
            videos[index],
            CardLayout::Stack,
            bare,
            None,
            false,
            Vec::new(),
            acts,
        );
    });
}

/// Videos as rows, 12 apart, each with the page's own entries.
fn rows(
    ui: &mut Ui,
    app: &VideoApp,
    context: &str,
    videos: &[&'static Video],
    list: Option<&str>,
    extra: impl Fn(usize, &'static Video) -> Vec<Entry>,
    acts: &mut Vec<Act>,
) {
    Stack::new(Gap::Md).show(ui, |ui| {
        for (index, video) in videos.iter().enumerate() {
            video_card(
                ui,
                app,
                context,
                video,
                CardLayout::Row,
                false,
                list,
                false,
                extra(index, video),
                acts,
            );
        }
    });
}

/// A page for an address that leads nowhere: a page, a channel or a playlist that is not there.
fn gone(ui: &mut Ui, app: &VideoApp, what: &str, acts: &mut Vec<Act>) {
    let address = app.path.join("/");
    let pressed = Empty::new(format!("No such {what}"))
        .description(format!("There is no {what} at #/{address}. It may have been removed."))
        .show(ui, |ui| ui.add(outline("Go home", None)).clicked());
    if pressed {
        acts.push(Act::Go("home".into()));
    }
}

/// Nothing to show, with a button under the words.
fn nothing(ui: &mut Ui, title: &str, words: &str, button: Option<(&str, bool)>) -> bool {
    Empty::new(title).description(words).show(ui, |ui| match button {
        Some((label, primary)) => {
            let button = if primary {
                Button::new(label).size(ButtonSize::Sm)
            } else {
                outline(label, None)
            };
            ui.add(button).clicked()
        }
        None => false,
    })
}

/// Home: the chips, and the videos they hold.
fn home(ui: &mut Ui, app: &VideoApp, views: &mut Views, acts: &mut Vec<Act>) {
    use pages::home::{Chip, chips, view};
    if views.chip.is_empty() {
        views.chip = "all".into();
    }
    let chosen = Chip::from_value(&views.chip).unwrap_or(Chip::All);
    let shown = view(&app.data, chosen);
    let all = chips();
    let pairs: Vec<(&str, &str)> = all.iter().map(|(chip, label)| (chip.value(), *label)).collect();
    ChipBar::new("topics", &pairs).show(ui, &mut views.chip);
    if shown.loading {
        return;
    }
    if let Some(says) = shown.empty {
        if nothing(ui, "Nothing to show", says, Some(("Show all", false))) {
            views.chip = "all".into();
        }
        return;
    }
    let videos: Vec<&'static Video> = shown.videos.iter().filter_map(|id| video_by_id(id)).collect();
    grid(ui, app, "home", &videos, false, acts);
}

/// A channel's own page: its banner and name with Follow, then its videos, or what it is about.
fn channel_page(
    ui: &mut Ui,
    app: &VideoApp,
    channel: &'static Channel,
    about: bool,
    views: &mut Views,
    acts: &mut Vec<Act>,
) {
    let shown = pages::channel::view(&app.data, channel, views.sort);
    // the banner, six times as wide as it is tall
    let width = ui.available_width();
    let (banner, _) = ui.allocate_exact_size(egui::vec2(width, width / 6.0), egui::Sense::hover());
    let painter = ui.painter_at(banner);
    let t = tokens(ui.ctx());
    Artwork::new(&format!("{}-banner", channel.id))
        .wide(true)
        .paint(&painter, banner);
    // its corners rounded on the page's ground
    painter.rect_stroke(
        banner.expand(6.0),
        egui::CornerRadius::same(t.radius("surface").nw.saturating_add(6)),
        egui::Stroke::new(12.0, t.color("background")),
        egui::StrokeKind::Middle,
    );
    let out = ChannelRow::new(channel.name)
        .large(true)
        .color(channel.color)
        .detail(&shown.detail)
        .following(shown.following)
        .show(ui);
    if let Some(on) = out.following {
        acts.push(Act::Follow(channel.id.to_owned(), on));
    }
    let mut tab = usize::from(about);
    if ui
        .add(Tabs::new(&mut tab, ["Videos", "About"]).variant(TabsVariant::Line))
        .changed()
    {
        acts.push(Act::Go(if tab == 1 {
            format!("channel/{}/about", channel.id)
        } else {
            format!("channel/{}", channel.id)
        }));
    }
    if about {
        ui.label(TextRole::Body.rich(&t, channel.about).color(t.color("foreground")));
        let mut list = DescriptionList::new().stacked();
        for (term, value) in &shown.about {
            list = list.text(*term, value.clone());
        }
        list.show(ui);
        return;
    }
    let mut play = false;
    toolbar(
        ui,
        |ui| {
            let mut at = pages::channel::SORTS
                .iter()
                .position(|(sort, _)| *sort == views.sort)
                .unwrap_or(0);
            if ui
                .add(Segmented::new(
                    &mut at,
                    pages::channel::SORTS.iter().map(|(_, label)| *label),
                ))
                .changed()
            {
                views.sort = pages::channel::SORTS[at].0;
            }
        },
        |ui| {
            if !shown.videos.is_empty() {
                play = ui.add(outline("Play", Some(Icon::Play))).clicked();
            }
        },
    );
    if play && let Some(first) = shown.videos.first() {
        acts.push(Act::Go(format!("watch/{first}")));
    }
    let videos: Vec<&'static Video> = shown.videos.iter().filter_map(|id| video_by_id(id)).collect();
    grid(ui, app, "channel", &videos, true, acts);
}

/// The latest from the channels followed, this week's apart from the earlier ones.
fn subscriptions(ui: &mut Ui, app: &VideoApp, views: &mut Views, acts: &mut Vec<Act>) {
    if views.channel_chip.is_empty() {
        views.channel_chip = "all".into();
    }
    let shown = pages::subscriptions::view(&app.data, &views.channel_chip);
    let mut manage = false;
    PageHeader::new("Subscriptions").count(&shown.count).show(ui, |ui| {
        manage = ui.add(outline("Manage", None)).clicked();
    });
    if manage {
        views.managing = true;
    }
    if !shown.loading {
        if shown.none_followed {
            if nothing(
                ui,
                "You follow no channels yet",
                "Follow a channel and its newest videos land here.",
                Some(("Find channels", true)),
            ) {
                views.managing = true;
            }
        } else {
            let chips: Vec<(&str, &str)> = shown.chips.iter().map(|(v, l)| (v.as_str(), l.as_str())).collect();
            let mut chosen = shown.chosen.clone();
            if ChipBar::new("channels", &chips).show(ui, &mut chosen) {
                views.channel_chip = chosen;
            }
            for (title, ids) in [("This week", &shown.week), ("Earlier", &shown.earlier)] {
                if ids.is_empty() {
                    continue;
                }
                let videos: Vec<&'static Video> = ids.iter().filter_map(|id| video_by_id(id)).collect();
                Section::new(title).show(ui, |ui| grid(ui, app, title, &videos, false, acts));
            }
            if shown.no_videos
                && nothing(
                    ui,
                    "No videos yet",
                    "This channel has put up nothing yet.",
                    Some(("Open the channel", false)),
                )
            {
                acts.push(Act::Go(format!("channel/{}", views.channel_chip)));
            }
        }
    }
    if views.managing {
        manage_dialog(ui.ctx(), app, views, acts);
    }
}

/// Every channel, with Follow beside each.
fn manage_dialog(ctx: &egui::Context, app: &VideoApp, views: &mut Views, acts: &mut Vec<Act>) {
    let rows = pages::subscriptions::manage(&app.data);
    let answer = sokkel_egui::DialogShell::new("manage", "Channels", "Done")
        .description("Follow a channel and its videos land in Subscriptions.")
        .width(448.0)
        .show(ctx, |ui| {
            Stack::new(Gap::Md).show(ui, |ui| {
                for (id, detail, following) in &rows {
                    let Some(channel) = channel_by_id(id) else { continue };
                    let out = ChannelRow::new(channel.name)
                        .color(channel.color)
                        .detail(detail)
                        .following(*following)
                        .linked(true)
                        .show(ui);
                    if let Some(on) = out.following {
                        acts.push(Act::Follow(channel.id.to_owned(), on));
                    }
                    if out.opened {
                        acts.push(Act::Go(format!("channel/{}", channel.id)));
                    }
                }
            });
        });
    if answer.is_some() {
        views.managing = false;
    }
}

/// A playlist as a card: its first video's still, its name, how many and when it changed.
fn playlist_card(ui: &mut Ui, card: &pages::playlists::PlaylistCard, acts: &mut Vec<Act>) {
    let mut items = vec![
        MenuItem::action("Play all")
            .icon(Icon::Play)
            .disabled(card.cover.is_none()),
    ];
    if card.deletable {
        items.push(MenuItem::action("Delete").icon(Icon::Trash2).destructive());
    }
    let first = card.cover.and_then(video_by_id);
    let mut media = MediaCard::new(("playlist-card", &card.id), &card.title)
        .shape(MediaShape::Wide)
        .subtitle(&card.subtitle)
        .badge(&card.badge)
        .playable(first.is_some())
        .menu(&items);
    media = match first {
        Some(video) => media.cover(poster(video)),
        None => media.placeholder_seed(&card.id),
    };
    let out = media.show(ui);
    if out.selected {
        acts.push(Act::Go(path_of(&card.href)));
    }
    if out.play || out.menu == Some(0) {
        acts.push(Act::PlayAll(card.id.clone()));
    }
    if out.menu == Some(1) {
        acts.push(Act::DeletePlaylist(card.id.clone()));
    }
}

/// Everything the viewer keeps, each part with See all.
fn library(ui: &mut Ui, app: &VideoApp, acts: &mut Vec<Act>) {
    let shown = pages::library::view(&app.data);
    PageHeader::new("Library").show(ui, |_| {});
    if shown.loading {
        return;
    }
    for part in &shown.parts {
        let mut all = false;
        Section::new(part.title)
            .meta(&part.meta)
            .actions(|ui| {
                all = ui
                    .add(
                        Button::new("See all")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Sm),
                    )
                    .clicked();
            })
            .show(ui, |ui| {
                if let Some(empty) = part.empty {
                    let t = tokens(ui.ctx());
                    ui.label(TextRole::Help.rich(&t, empty).color(t.color("muted-foreground")));
                } else if part.title == "Playlists" {
                    MediaGrid::new(MediaShape::Wide).show(ui, part.lists.len(), |ui, index| {
                        playlist_card(ui, &part.lists[index], acts);
                    });
                } else {
                    let videos: Vec<&'static Video> = part.videos.iter().filter_map(|id| video_by_id(id)).collect();
                    grid(ui, app, part.title, &videos, false, acts);
                }
            });
        if all {
            acts.push(Act::Go(path_of(part.all)));
        }
    }
}

/// What the viewer watched, by day.
fn history(ui: &mut Ui, app: &VideoApp, views: &mut Views, acts: &mut Vec<Act>) {
    let shown = pages::history::view(&app.data, &views.history_search);
    let mut clear = false;
    PageHeader::new("History").count(&shown.count).show(ui, |ui| {
        clear = ui
            .add(outline("Clear all", Some(Icon::Trash2)).enabled(!shown.clear_disabled))
            .clicked();
    });
    if clear {
        acts.push(Act::Ask(Asking::ClearHistory { clearing: false }));
    }
    let mut keep = shown.keep;
    toolbar(
        ui,
        |ui| {
            SearchField::new("history-search", &mut views.history_search)
                .keys("/")
                .show(ui);
        },
        |ui| {
            ui.add(
                SwitchField::new(&mut keep, "Keep history").explain("Remembers what you watch and where you stopped"),
            );
        },
    );
    if keep != shown.keep {
        acts.push(Act::Prefs(PrefsPatch {
            history: Some(keep),
            ..PrefsPatch::default()
        }));
    }
    if shown.loading {
        return;
    }
    if let Some((title, words)) = &shown.empty {
        nothing(ui, title, words, None);
        return;
    }
    for day in &shown.days {
        let videos: Vec<&'static Video> = day.videos.iter().filter_map(|id| video_by_id(id)).collect();
        Section::new(day.label).show(ui, |ui| {
            rows(
                ui,
                app,
                day.label,
                &videos,
                None,
                |_, video| {
                    vec![(
                        MenuItem::action("Remove from history").icon(Icon::Trash2).destructive(),
                        Act::RemoveFromHistory(video.id.to_owned()),
                    )]
                },
                acts,
            );
        });
    }
}

/// One playlist, or Watch later.
fn playlist(ui: &mut Ui, app: &VideoApp, id: &str, acts: &mut Vec<Act>) {
    if !app.data.playlists.loaded() {
        return;
    }
    let Some(shown) = pages::playlist::view(&app.data, id) else {
        gone(ui, app, "playlist", acts);
        return;
    };
    let mut asked: Option<Act> = None;
    let mut header = PageHeader::new(&shown.name)
        .count(&shown.count)
        .description(&shown.description);
    if !shown.later {
        header = header.edit_title("Playlist name", |name| {
            pages::playlist::validate_name(name).map(str::to_owned)
        });
    }
    let out = header.show(ui, |ui| {
        if ui
            .add(
                Button::new("Shuffle")
                    .icon(Icon::Shuffle)
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Sm)
                    .enabled(!shown.shuffle_disabled),
            )
            .clicked()
        {
            asked = Some(Act::Shuffle(id.to_owned()));
        }
        if shown.deletable && ui.add(outline("Delete", Some(Icon::Trash2))).clicked() {
            asked = Some(Act::Ask(Asking::DeletePlaylist(id.to_owned())));
        }
        if ui
            .add(
                Button::new("Play all")
                    .icon(Icon::Play)
                    .size(ButtonSize::Sm)
                    .enabled(!shown.play_disabled),
            )
            .clicked()
        {
            asked = Some(Act::PlayAll(id.to_owned()));
        }
    });
    if let Some(name) = out.renamed {
        acts.push(Act::RenamePlaylist(id.to_owned(), name));
    }
    acts.extend(asked);
    if let Some(empty) = &shown.empty {
        if nothing(ui, "Nothing here yet", empty, Some(("Find videos", false))) {
            acts.push(Act::Go("home".into()));
        }
        return;
    }
    let videos: Vec<&'static Video> = shown.videos.iter().filter_map(|one| video_by_id(one.id)).collect();
    let entries = &shown.videos;
    rows(
        ui,
        app,
        id,
        &videos,
        Some(id),
        |index, video| {
            let entry = &entries[index];
            vec![
                (
                    MenuItem::action("Move up")
                        .icon(Icon::ArrowUp)
                        .disabled(entry.up_disabled),
                    Act::MoveInPlaylist(id.to_owned(), index, -1),
                ),
                (
                    MenuItem::action("Move down")
                        .icon(Icon::ArrowDown)
                        .disabled(entry.down_disabled),
                    Act::MoveInPlaylist(id.to_owned(), index, 1),
                ),
                (
                    MenuItem::action(&entry.remove).icon(Icon::Trash2).destructive(),
                    Act::RemoveFromPlaylist(id.to_owned(), video.id.to_owned()),
                ),
            ]
        },
        acts,
    );
}

/// The videos liked, the latest first.
fn liked(ui: &mut Ui, app: &VideoApp, acts: &mut Vec<Act>) {
    let videos = pages::liked::videos(&app.data);
    let mut play = false;
    PageHeader::new("Liked videos")
        .count(plural(videos.len() as f64, "video", None))
        .show(ui, |ui| {
            play = ui
                .add(
                    Button::new("Play the latest")
                        .icon(Icon::Play)
                        .size(ButtonSize::Sm)
                        .enabled(!videos.is_empty()),
                )
                .clicked();
        });
    if play && let Some(first) = videos.first() {
        acts.push(Act::Go(format!("watch/{}", first.id)));
    }
    if !app.data.ratings.loaded() {
        return;
    }
    if videos.is_empty() {
        nothing(
            ui,
            "No liked videos",
            "Videos you like land here, so they are easy to find again.",
            None,
        );
        return;
    }
    rows(
        ui,
        app,
        "liked",
        &videos,
        None,
        |_, video| {
            vec![(
                MenuItem::action("Remove from Liked videos")
                    .icon(Icon::Trash2)
                    .destructive(),
                Act::Unlike(video.id.to_owned()),
            )]
        },
        acts,
    );
}

/// The playlists, Watch later first, and a field to make a new one.
fn playlists(ui: &mut Ui, app: &VideoApp, views: &mut Views, acts: &mut Vec<Act>) {
    let shown = pages::playlists::view(&app.data);
    PageHeader::new("Playlists").count(&shown.count).show(ui, |_| {});
    let data = &app.data;
    let mut made = None;
    toolbar(
        ui,
        |ui| {
            made = QuickAdd::new("new-playlist", &mut views.new_playlist)
                .placeholder("New playlist")
                .validate(|name| crate::dialogs::validate_name(data, name).map(str::to_owned))
                .show(ui);
        },
        |_| {},
    );
    if let Some(name) = made {
        acts.push(Act::NewPlaylist(name));
    }
    if shown.loading {
        return;
    }
    MediaGrid::new(MediaShape::Wide).show(ui, shown.cards.len(), |ui, index| {
        playlist_card(ui, &shown.cards[index], acts);
    });
}

/// The videos kept on this device, as a table.
fn downloads(ui: &mut Ui, app: &VideoApp, views: &mut Views, acts: &mut Vec<Act>) {
    let shown = pages::downloads::view(&app.data);
    PageHeader::new("Downloads").count(&shown.count).show(ui, |_| {});
    if shown.rows.is_empty() {
        if !shown.loading {
            nothing(
                ui,
                "No downloads",
                "Download a video from its page, and it is kept on this device to watch without the site.",
                None,
            );
        }
        return;
    }
    let mut kept = shown.rows.clone();
    kept.sort_by_key(|one| std::cmp::Reverse(one.at));
    let now = app.now();
    let columns = vec![
        Column::new("Video", 256.0).fill(),
        Column::new("Length", 96.0),
        Column::new("Quality", 96.0),
        Column::new("Size", 96.0),
        Column::new("Downloaded", 144.0),
    ];
    let cells: Vec<Vec<Cell>> = kept
        .iter()
        .map(|one| {
            vec![
                Cell::Lines {
                    title: one.title.to_owned(),
                    detail: one.channel.to_owned(),
                    over: false,
                },
                Cell::Text(one.length.clone()),
                Cell::Text(one.quality.to_owned()),
                Cell::Text(one.size.clone()),
                Cell::Muted(relative_time((one.at - now) as f64)),
            ]
        })
        .collect();
    let out = DataTable::new("downloads", &columns, &cells, &mut views.table)
        .actions(|_| {
            vec![
                MenuItem::action("Play").icon(Icon::Play),
                MenuItem::Separator,
                MenuItem::action("Remove the download").icon(Icon::Trash2).destructive(),
            ]
        })
        .row_height(48.0)
        .stretch(true)
        .show(ui);
    if let Some((row, action)) = out.action {
        match action {
            0 => acts.push(Act::Go(format!("watch/{}", kept[row].id))),
            _ => acts.push(Act::RemoveDownload(kept[row].id.to_owned())),
        }
    }
}

/// What a search finds: channels with Follow, and videos.
fn search(ui: &mut Ui, app: &VideoApp, query: &str, views: &mut Views, acts: &mut Vec<Act>) {
    let query = crate::js::decode_uri_component(query);
    let shown = pages::search::view(&app.data, &query, views.kind, views.search_sort);
    PageHeader::new(&shown.title).count(&shown.count).show(ui, |_| {});
    let mut submitted = false;
    toolbar(
        ui,
        |ui| {
            let field = SearchField::new("search-page", &mut views.search_text)
                .placeholder("Search videos and channels")
                .show(ui);
            submitted = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        },
        |ui| {
            let mut at = pages::search::SORTS
                .iter()
                .position(|(sort, _)| *sort == views.search_sort)
                .unwrap_or(0);
            if ui
                .add(Segmented::new(
                    &mut at,
                    pages::search::SORTS.iter().map(|(_, label)| *label),
                ))
                .changed()
            {
                views.search_sort = pages::search::SORTS[at].0;
            }
        },
    );
    if submitted {
        let text = crate::js::trim(&views.search_text);
        if !text.is_empty() {
            acts.push(Act::Go(format!("search/{}", crate::js::encode_uri_component(text))));
        }
    }
    use pages::search::Kind;
    let kinds = [Kind::All, Kind::Videos, Kind::Channels];
    let chips: Vec<(&str, &str)> = kinds
        .iter()
        .zip(&shown.chips)
        .map(|(kind, label)| (kind.value(), label.as_str()))
        .collect();
    let mut chosen = views.kind.value().to_owned();
    if ChipBar::new("search-kinds", &chips).show(ui, &mut chosen)
        && let Some(kind) = kinds.iter().find(|kind| kind.value() == chosen)
    {
        views.kind = *kind;
    }
    if let Some((title, words)) = &shown.empty {
        nothing(ui, title, words, None);
        return;
    }
    if !shown.channels.is_empty() {
        Section::new("Channels").show(ui, |ui| {
            Stack::new(Gap::Md).show(ui, |ui| {
                for found in &shown.channels {
                    let Some(channel) = channel_by_id(found.id) else {
                        continue;
                    };
                    let out = ChannelRow::new(channel.name)
                        .color(channel.color)
                        .detail(&found.detail)
                        .following(found.following)
                        .linked(true)
                        .show(ui);
                    if let Some(on) = out.following {
                        acts.push(Act::Follow(channel.id.to_owned(), on));
                    }
                    if out.opened {
                        acts.push(Act::Go(format!("channel/{}", channel.id)));
                    }
                }
            });
        });
    }
    if !shown.videos.is_empty() {
        let videos: Vec<&'static Video> = shown.videos.iter().filter_map(|id| video_by_id(id)).collect();
        Section::new("Videos").show(ui, |ui| rows(ui, app, "search", &videos, None, |_, _| Vec::new(), acts));
    }
}
