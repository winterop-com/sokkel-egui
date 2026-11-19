//! The pages, as the web's `pages/*.tsx`: Listen now, New, the queue, Discover, the shows followed,
//! a show, an episode, Starred, Downloads and Stats, and a page for an address that leads nowhere.

use eframe::egui::{self, Ui, vec2};
use sokkel_egui::{
    BarList, BarListItem, Button, ButtonSize, ButtonVariant, ChipBar, Columns, Gap, Icon, Markdown, MetaLine, Page,
    PageHeader, SearchField, Section, Segmented, SpeakButton, SpeakState, Stack, StarButton, Stat, TextRole,
    format_time, tokens, toolbar,
};
use sokkel_egui_media::{
    Artwork, ChapterList, MediaCard, MediaGrid, MediaShape, TrackListVariant, TrackProgress, Transcript,
};

use crate::act::{Act, Asking};
use crate::app::{Views, encode};
use crate::backend::{DownloadState, size_of};
use crate::catalog::{
    CATEGORIES, Category, Episode, Show, category_label, episode_by_id, episodes, episodes_of, hosts_of, show_by_id,
    shows,
};
use crate::discover::find;
use crate::episodes::{
    Here, ListLook, episode_download, episode_list, episode_menu_button, nothing, show_grid, standing, star_words,
};
use crate::parts::{ago, megabytes, spoken};
use crate::script::{Script, length_of, script_of};
use crate::stats::Stats;

/// The page the address names, in the page's scroller.
pub fn page(
    ui: &mut Ui,
    here: &Here,
    page: &str,
    second: Option<&str>,
    third: Option<&str>,
    views: &mut Views,
    acts: &mut Vec<Act>,
) {
    let _ = third;
    Page::scroll(ui, |ui| {
        Stack::new(Gap::Section).show(ui, |ui| match page {
            "home" => home(ui, here, acts),
            "inbox" => inbox(ui, here, acts),
            "queue" => queue(ui, here, acts),
            "discover" => discover(ui, here, second.unwrap_or("all"), views, acts),
            "shows" => followed(ui, here, acts),
            "show" => match second.and_then(show_by_id) {
                Some(show) => show_page(ui, here, show, views, acts),
                None => gone(ui, "show", acts),
            },
            "episode" => match second.and_then(episode_by_id) {
                Some(episode) => episode_page(ui, here, episode, acts),
                None => gone(ui, "episode", acts),
            },
            "starred" => starred(ui, here, acts),
            "downloads" => downloads(ui, here, acts),
            "stats" => stats(ui, here),
            _ => gone(ui, "page", acts),
        });
    });
}

/// A small outline button with an icon.
fn outline(label: &str, icon: Option<Icon>) -> Button {
    let button = Button::new(label).variant(ButtonVariant::Outline).size(ButtonSize::Sm);
    match icon {
        Some(icon) => button.icon(icon),
        None => button,
    }
}

/// A small filled button with the play or pause glyph filled.
fn filled(label: &str, icon: Icon) -> Button {
    Button::new(label).icon(icon).filled_icon(true).size(ButtonSize::Sm)
}

/// The episodes by when they came out, newest first.
fn newest<'a>(here: &Here, mut list: Vec<&'a Episode>) -> Vec<&'a Episode> {
    let out_at = |id: &str| here.state.episode(id).map_or(0, |row| row.out_at);
    list.sort_by_key(|episode| std::cmp::Reverse(out_at(&episode.id)));
    list
}

/// A page for an address that leads nowhere: a show or an episode not in the catalog, or a
/// mistyped one.
fn gone(ui: &mut Ui, what: &str, acts: &mut Vec<Act>) {
    let pressed = sokkel_egui::Empty::new(format!("No {what} here"))
        .description("The address is wrong, or it is no longer listed.")
        .show(ui, |ui| {
            ui.add(
                Button::new("Listen now")
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Sm),
            )
            .clicked()
        });
    if pressed {
        acts.push(Act::Go("home".into()));
    }
}

/// Listen now: what was left halfway, what plays next, and the latest from the shows followed.
fn home(ui: &mut Ui, here: &Here, acts: &mut Vec<Act>) {
    let state = here.state;
    PageHeader::new("Listen now")
        .description("Pick up where you left off, or start something new.")
        .show(ui, |_| {});
    let mut started: Vec<&crate::backend::EpisodeState> = state
        .episodes
        .iter()
        .filter(|row| row.position > 0.0 && !row.played)
        .collect();
    started.sort_by_key(|row| std::cmp::Reverse(row.heard.unwrap_or(0)));
    started.truncate(6);
    let next: Vec<&Episode> = state
        .player
        .queue
        .iter()
        .filter_map(|id| episode_by_id(id))
        .take(4)
        .collect();
    let followed: Vec<&str> = state
        .shows
        .iter()
        .filter(|show| show.subscribed)
        .map(|show| show.id.as_str())
        .collect();
    let latest: Vec<&Episode> = episodes()
        .iter()
        .filter(|episode| {
            followed.contains(&episode.show.as_str())
                && state.episode(&episode.id).is_some_and(|row| row.out && !row.played)
        })
        .collect();
    let mut latest = newest(here, latest);
    latest.truncate(8);

    if !started.is_empty() {
        Section::new("Continue listening").show(ui, |ui| {
            MediaGrid::new(MediaShape::Square).show(ui, started.len(), |ui, index| {
                let row = started[index];
                let (Some(episode), Some(show)) = (
                    episode_by_id(&row.id),
                    episode_by_id(&row.id).and_then(|one| show_by_id(&one.show)),
                ) else {
                    return;
                };
                let length = length_of(&row.id);
                let cover = show.cover;
                let out = MediaCard::new(("continue", &row.id), &episode.title)
                    .subtitle(format!(
                        "{} \u{b7} {} left",
                        show.title,
                        format_time(length - row.position)
                    ))
                    .cover(move |painter, rect| Artwork::of(cover).paint(painter, rect))
                    .playable(true)
                    .playing(state.playing && state.current() == Some(row.id.as_str()))
                    .progress((row.position / length) as f32)
                    .show(ui);
                if out.selected {
                    acts.push(Act::Go(format!("episode/{}", row.id)));
                }
                if out.play {
                    acts.push(Act::Play(row.id.clone(), None));
                }
            });
        });
    }
    if !next.is_empty() {
        let mut open_queue = false;
        Section::new("Up next")
            .flush(true)
            .actions(|ui| {
                open_queue = ui.add(outline("Queue", None)).clicked();
            })
            .show(ui, |ui| {
                let look = ListLook {
                    with_show: true,
                    plain: true,
                    ..ListLook::default()
                };
                episode_list(ui, here, "home-next", &next, look, acts);
            });
        if open_queue {
            acts.push(Act::Go("queue".into()));
        }
    }
    Section::new("Latest from your shows").flush(true).show(ui, |ui| {
        if latest.is_empty() {
            if nothing(
                ui,
                Icon::Compass,
                "Nothing new to play",
                "Follow a few shows and their latest episodes are listed here.",
                Some(("Discover", None)),
            ) {
                acts.push(Act::Go("discover".into()));
            }
        } else {
            let look = ListLook {
                with_show: true,
                plain: true,
                ..ListLook::default()
            };
            episode_list(ui, here, "home-latest", &latest, look, acts);
        }
    });
}

/// `1 episode`, `3 episodes`.
fn episodes_word(count: usize) -> String {
    format!("{count} {}", if count == 1 { "episode" } else { "episodes" })
}

/// New: the episodes of followed shows not yet heard or archived, newest first.
fn inbox(ui: &mut Ui, here: &Here, acts: &mut Vec<Act>) {
    let state = here.state;
    let waiting = newest(
        here,
        episodes()
            .iter()
            .filter(|episode| state.episode(&episode.id).is_some_and(|row| row.inbox))
            .collect(),
    );
    let checking = here.checking;
    let check = |ui: &mut Ui| {
        ui.add(
            outline(
                if checking { "Checking" } else { "Check for new" },
                Some(Icon::RefreshCw),
            )
            .busy(checking),
        )
        .clicked()
    };
    PageHeader::new("New")
        .count(episodes_word(waiting.len()))
        .show(ui, |ui| {
            if ui
                .add(outline("Archive all", Some(Icon::Archive)).enabled(!waiting.is_empty()))
                .clicked()
            {
                acts.push(Act::ArchiveAll);
            }
            if check(ui) {
                acts.push(Act::CheckForNew);
            }
        });
    if waiting.is_empty() {
        let pressed = sokkel_egui::Empty::new("All caught up")
            .icon(Icon::Inbox)
            .description("New episodes of the shows you follow land here until you play or archive them.")
            .show(ui, check);
        if pressed {
            acts.push(Act::CheckForNew);
        }
        return;
    }
    let look = ListLook {
        with_show: true,
        extra: Some(|episode| {
            Some((
                Icon::Archive,
                format!("Archive {}", episode.title),
                Act::Archive(episode.id.clone()),
            ))
        }),
        ..ListLook::default()
    };
    episode_list(ui, here, "inbox", &waiting, look, acts);
}

/// The queue: what plays now, and what plays after it, in an order the listener sets.
fn queue(ui: &mut Ui, here: &Here, acts: &mut Vec<Act>) {
    let state = here.state;
    let queue: Vec<&Episode> = state.player.queue.iter().filter_map(|id| episode_by_id(id)).collect();
    let now = state.current().and_then(episode_by_id);
    let left: f64 = queue
        .iter()
        .map(|episode| {
            let row = state.episode(&episode.id);
            length_of(&episode.id)
                - if row.is_some_and(|row| row.played) {
                    0.0
                } else {
                    row.map_or(0.0, |row| row.position)
                }
        })
        .sum();
    PageHeader::new("Queue")
        .count(if queue.is_empty() {
            "Empty".to_owned()
        } else {
            format!("{} up next, {}", queue.len(), spoken(left))
        })
        .show(ui, |ui| {
            if ui
                .add(outline("Clear", Some(Icon::ListX)).enabled(!queue.is_empty()))
                .clicked()
            {
                acts.push(Act::Ask(Asking::ClearQueue));
            }
            if ui
                .add(outline("Play next", Some(Icon::SkipForward)).enabled(!queue.is_empty()))
                .clicked()
            {
                acts.push(Act::PlayNext);
            }
        });
    if let Some(now) = now {
        Section::new(if state.playing { "Now playing" } else { "Paused" })
            .flush(true)
            .show(ui, |ui| {
                let look = ListLook {
                    with_show: true,
                    plain: true,
                    ..ListLook::default()
                };
                episode_list(ui, here, "queue-now", &[now], look, acts);
            });
    }
    Section::new("Up next").flush(true).show(ui, |ui| {
        if queue.is_empty() {
            if nothing(
                ui,
                Icon::ListOrdered,
                "Nothing up next",
                "Add episodes from their menu, with Play next or Add to queue, and they play here in the order you drag them into.",
                Some(("New episodes", None)),
            ) {
                acts.push(Act::Go("inbox".into()));
            }
            return;
        }
        let look = ListLook {
            with_show: true,
            plain: true,
            reorder: true,
            extra: Some(|episode| {
                Some((
                    Icon::ListX,
                    format!("Remove {} from the queue", episode.title),
                    Act::QueueRemove(episode.id.clone()),
                ))
            }),
            ..ListLook::default()
        };
        episode_list(ui, here, "queue-next", &queue, look, acts);
    });
}

/// Discover: every show by category, and a search across the shows and their episodes; the address
/// holds both.
fn discover(ui: &mut Ui, here: &Here, category: &str, views: &mut Views, acts: &mut Vec<Act>) {
    let state = here.state;
    let chosen = Category::from_id(category);
    let chosen_id = chosen.map_or("all", Category::id);
    PageHeader::new("Discover")
        .description("Shows about the sea, the land and the work in between.")
        .show(ui, |_| {});
    let before = views.discover.clone();
    let mut value = chosen_id.to_owned();
    let chips: Vec<(&str, &str)> = std::iter::once(("all", "All"))
        .chain(CATEGORIES.iter().map(|(one, label)| (one.id(), *label)))
        .collect();
    Stack::new(Gap::Md).show(ui, |ui| {
        SearchField::new("discover-search", &mut views.discover)
            .placeholder("Search shows and episodes")
            .keys("/")
            .width(ui.available_width())
            .show(ui);
        if ChipBar::new("categories", &chips).show(ui, &mut value) {
            acts.push(Act::Go(path_of(&value, &views.discover)));
        }
    });
    // typing keeps the address current
    if views.discover != before {
        acts.push(Act::Go(path_of(chosen_id, &views.discover)));
    }
    let typed = views.discover.trim().to_owned();
    let found = (!typed.is_empty()).then(|| find(&typed, |id| state.episode(id).is_some_and(|row| row.out)));
    let in_category: Vec<String> = match &found {
        Some((shows, _)) => shows.to_vec(),
        None => shows().iter().collect(),
    }
    .into_iter()
    .filter(|show| chosen.is_none_or(|category| show.category == category))
    .map(|show| show.id.clone())
    .collect();
    let found_episodes: Vec<&Episode> = found
        .as_ref()
        .map(|(_, episodes)| episodes.clone())
        .unwrap_or_default()
        .into_iter()
        .filter(|episode| {
            chosen.is_none_or(|category| show_by_id(&episode.show).is_some_and(|show| show.category == category))
        })
        .collect();
    if found.is_some() && in_category.is_empty() && found_episodes.is_empty() {
        let within = chosen.map_or(String::new(), |category| format!("in {} ", category_label(category)));
        if nothing(
            ui,
            Icon::SearchX,
            "Nothing found",
            &format!("No show or episode {within}mentions \u{201c}{typed}\u{201d}."),
            Some(("Clear the search", None)),
        ) {
            views.discover.clear();
            acts.push(Act::Go("discover/all".into()));
        }
        return;
    }
    if !in_category.is_empty() {
        let title = match (&found, chosen) {
            (Some(_), _) => "Shows".to_owned(),
            (None, None) => "All shows".to_owned(),
            (None, Some(category)) => category_label(category).to_owned(),
        };
        Section::new(title).show(ui, |ui| show_grid(ui, here, &in_category, acts));
    }
    if !found_episodes.is_empty() {
        let shown: Vec<&Episode> = found_episodes.iter().take(30).copied().collect();
        Section::new("Episodes")
            .flush(true)
            .meta(found_episodes.len().to_string())
            .show(ui, |ui| {
                let look = ListLook {
                    with_show: true,
                    plain: true,
                    ..ListLook::default()
                };
                episode_list(ui, here, "discover-episodes", &shown, look, acts);
            });
    }
}

/// The address of Discover in a category with words typed.
fn path_of(category: &str, words: &str) -> String {
    let words = words.trim();
    if words.is_empty() {
        format!("discover/{category}")
    } else {
        format!("discover/{category}/{}", encode(words))
    }
}

/// The shows the listener follows, by title.
fn followed(ui: &mut Ui, here: &Here, acts: &mut Vec<Act>) {
    let mut followed: Vec<String> = here
        .state
        .shows
        .iter()
        .filter(|show| show.subscribed)
        .map(|show| show.id.clone())
        .collect();
    let title = |id: &str| show_by_id(id).map_or(id.to_owned(), |show| show.title.to_lowercase());
    followed.sort_by_key(|id| title(id));
    PageHeader::new("Shows")
        .count(format!("{} followed", followed.len()))
        .show(ui, |ui| {
            if ui.add(outline("Discover", Some(Icon::Compass))).clicked() {
                acts.push(Act::Go("discover".into()));
            }
        });
    if followed.is_empty() {
        if nothing(
            ui,
            Icon::Podcast,
            "No shows yet",
            "Follow a show from Discover, and its new episodes come to New.",
            Some(("Discover", Some(Icon::Compass))),
        ) {
            acts.push(Act::Go("discover".into()));
        }
        return;
    }
    show_grid(ui, here, &followed, acts);
}

/// The words of a show's filter.
const SHOWN: [&str; 4] = ["All", "Unplayed", "Downloaded", "Starred"];

/// A show's page: what it is, following it, and its episodes, filtered.
fn show_page(ui: &mut Ui, here: &Here, show: &Show, views: &mut Views, acts: &mut Vec<Act>) {
    let state = here.state;
    let id = &show.id;
    let followed = state.show(id).is_some_and(|row| row.subscribed);
    let out: Vec<&Episode> = episodes_of(id)
        .into_iter()
        .filter(|episode| state.episode(&episode.id).is_some_and(|row| row.out))
        .collect();
    let listed: Vec<&Episode> = out
        .iter()
        .copied()
        .filter(|episode| {
            let Some(row) = state.episode(&episode.id) else {
                return false;
            };
            match views.shown {
                1 => !row.played,
                2 => row.download == DownloadState::Kept,
                3 => row.starred,
                _ => true,
            }
        })
        .collect();
    let unplayed: Vec<String> = out
        .iter()
        .filter(|episode| state.episode(&episode.id).is_some_and(|row| !row.played))
        .map(|episode| episode.id.clone())
        .collect();
    // an episode started and not finished, else the newest not played
    let next = out
        .iter()
        .find(|episode| {
            state
                .episode(&episode.id)
                .is_some_and(|row| row.position > 0.0 && !row.played)
        })
        .map(|episode| episode.id.clone())
        .or_else(|| unplayed.first().cloned());
    let playing_here = state.playing && next.is_some() && state.current() == next.as_deref();
    PageHeader::new(&show.title)
        .meta_line(
            MetaLine::new([
                hosts_of(show),
                category_label(show.category).to_owned(),
                format!("{} episodes", out.len()),
            ])
            .sm(),
        )
        .show(ui, |ui| {
            let following = if followed {
                outline("Following", Some(Icon::Check))
            } else {
                outline("Follow", Some(Icon::Plus))
            };
            if ui.add(following).clicked() {
                acts.push(if followed {
                    Act::Ask(Asking::Unfollow(id.clone()))
                } else {
                    Act::Subscribe(id.clone())
                });
            }
            if let Some(next) = &next {
                let resume = state.episode(next).is_some_and(|row| row.position > 0.0);
                let button = if playing_here {
                    filled("Pause", Icon::Pause)
                } else {
                    filled(if resume { "Resume" } else { "Play latest" }, Icon::Play)
                };
                if ui.add(button).clicked() {
                    acts.push(Act::Play(next.clone(), None));
                }
            }
        });
    // the cover beside what the show is about
    let t = tokens(ui.ctx());
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 24.0;
        ui.add(Artwork::of(show.cover).size(vec2(128.0, 128.0)).framed(true));
        prose(ui, &show.about, "muted-foreground");
    });
    let _ = t;
    Stack::new(Gap::Md).show(ui, |ui| {
        let mut shown = views.shown;
        toolbar(
            ui,
            |ui| {
                ui.add(Segmented::new(&mut shown, SHOWN));
            },
            |ui| {
                if ui
                    .add(outline("Mark all played", Some(Icon::Check)).enabled(!unplayed.is_empty()))
                    .clicked()
                {
                    acts.push(Act::MarkAllPlayed(unplayed.clone()));
                }
            },
        );
        views.shown = shown;
        if listed.is_empty() {
            let which = SHOWN[views.shown.min(3)].to_lowercase();
            if nothing(
                ui,
                Icon::Filter,
                "No episodes here",
                &format!("None of the episodes of {} is {which}.", show.title),
                Some(("Show all", None)),
            ) {
                views.shown = 0;
            }
        } else {
            let look = ListLook {
                linked: true,
                ..ListLook::default()
            };
            episode_list(ui, here, "show-episodes", &listed, look, acts);
        }
    });
}

/// A paragraph at the body's size, relaxed, at most a prose measure wide, in an ink token.
fn prose(ui: &mut Ui, text: &str, ink: &str) {
    let t = tokens(ui.ctx());
    let body = TextRole::Body.font(&t);
    let zero = ui
        .painter()
        .layout_no_wrap("0".into(), body.clone(), egui::Color32::WHITE)
        .size()
        .x;
    let width = ui.available_width().min(65.0 * zero);
    let galley = egui::WidgetText::from(
        TextRole::Body
            .rich(&t, text)
            .line_height(Some(body.size * 1.625))
            .color(t.color(ink)),
    )
    .into_galley(ui, Some(egui::TextWrapMode::Wrap), width, body);
    let (rect, _) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
    ui.painter().galley(rect.min, galley, t.color(ink));
}

/// An episode's notes in markdown: its chapters as times that seek, what it is about, and where to
/// go next.
fn notes_of(episode: &Episode, show: &Show, script: &Script) -> String {
    let before = episodes_of(&show.id)
        .into_iter()
        .find(|other| other.number + 1 == episode.number);
    let mut lines = vec!["### Chapters".to_owned(), String::new()];
    for chapter in &script.chapters {
        lines.push(format!(
            "- [{}](#t={}) {}",
            format_time(chapter.start),
            crate::js::number_to_string(chapter.start),
            chapter.title
        ));
    }
    lines.push(String::new());
    lines.push("### In this episode".into());
    lines.push(String::new());
    lines.push(format!(
        "With {}, recorded at {}. Each line of the transcript plays from where it is said.",
        hosts_of(show),
        episode.place
    ));
    lines.push(String::new());
    lines.push(match before {
        None => format!("More from [{}](#/show/{}).", show.title, show.id),
        Some(before) => format!(
            "More from [{}](#/show/{}), or the episode before this one, [{}](#/episode/{}).",
            show.title, show.id, before.title, before.id
        ),
    });
    lines.join("\n")
}

/// An episode's page: what it is, its notes with times that seek, its chapters, and its transcript
/// following the player.
fn episode_page(ui: &mut Ui, here: &Here, episode: &Episode, acts: &mut Vec<Act>) {
    let state = here.state;
    let id = &episode.id;
    let (Some(show), Some(row), Some(script)) = (show_by_id(&episode.show), state.episode(id), script_of(id)) else {
        return;
    };
    let current = state.current() == Some(id.as_str());
    let position = if current {
        state.position()
    } else if row.played {
        0.0
    } else {
        row.position
    };
    let playing_here = current && state.playing;
    let seek = |seconds: f64| {
        if current {
            Act::Seek(seconds)
        } else {
            Act::Play(id.clone(), Some(seconds))
        }
    };
    PageHeader::new(&episode.title)
        .meta_line(
            MetaLine::new(
                [
                    Some(format!("Episode {}", episode.number)),
                    Some(ago(row.out_at, here.now)),
                    Some(spoken(script.duration)),
                    standing(state, id),
                ]
                .into_iter()
                .flatten(),
            )
            .sm(),
        )
        .show(ui, |ui| {
            episode_menu_button(ui, state, id, "More", false, acts);
            episode_download(ui, state, id, acts);
            let mut star = row.starred;
            ui.add(StarButton::new(&mut star).outline().words(star_words()));
            if star != row.starred {
                acts.push(Act::Star(id.clone(), star));
            }
            let button = if playing_here {
                filled("Pause", Icon::Pause)
            } else if position > 0.0 {
                filled(&format!("Resume at {}", format_time(position)), Icon::Play)
            } else {
                filled("Play", Icon::Play)
            };
            if ui.add(button).clicked() {
                acts.push(Act::Play(id.clone(), None));
            }
        });
    // the episode beside its transcript: 32 apart, the transcript 512 wide
    let gap = 32.0;
    let width = ui.available_width();
    let side = 512.0_f32.min((width - gap) / 2.0);
    let main = width - gap - side;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.allocate_ui_with_layout(vec2(main, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_width(main);
            Stack::new(Gap::Section).show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    ui.add(Artwork::of(show.cover).size(vec2(80.0, 80.0)).framed(true));
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 8.0;
                        prose(ui, &episode.summary, "foreground");
                        let speaking = here.speaking == Some(id.as_str());
                        let words = sokkel_egui::SpeakButtonWords {
                            speak: "Hear the summary".into(),
                            ..Default::default()
                        };
                        let state = if speaking {
                            SpeakState::Playing
                        } else {
                            SpeakState::Idle
                        };
                        if ui.add(SpeakButton::new(state).labeled(true).words(words)).clicked() {
                            acts.push(Act::Speak(id.clone()));
                        }
                    });
                });
                Section::new("Show notes").show(ui, |ui| {
                    let notes = notes_of(episode, show, &script);
                    let out = Markdown::new(&notes).open_urls(false).show(ui);
                    if let Some(url) = out.url {
                        if let Some(time) = url.strip_prefix("#t=").and_then(|t| t.parse::<f64>().ok()) {
                            acts.push(seek(time));
                        } else if let Some(path) = url.strip_prefix("#/") {
                            acts.push(Act::Go(path.to_owned()));
                        }
                    }
                });
                Section::new("Chapters")
                    .meta(script.chapters.len().to_string())
                    .flush(true)
                    .show(ui, |ui| {
                        let out = ChapterList::new(("chapters", id), &script.chapters, script.duration)
                            .position(position)
                            .playing(playing_here)
                            .progress(TrackProgress::Left)
                            .follow(false)
                            .variant(TrackListVariant::Plain)
                            .show(ui);
                        if let Some(at) = out.seek {
                            acts.push(seek(at));
                        }
                    });
            });
        });
        ui.allocate_ui_with_layout(vec2(side, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_width(side);
            Section::new("Transcript").flush(true).show(ui, |ui| {
                ui.allocate_ui(vec2(ui.available_width(), 448.0), |ui| {
                    ui.set_height(448.0);
                    let out = Transcript::new(("transcript", id), &script.lines)
                        .position(position)
                        .show(ui);
                    if let Some(at) = out.seek {
                        acts.push(seek(at));
                    }
                });
            });
        });
    });
}

/// The episodes the listener starred, newest first.
fn starred(ui: &mut Ui, here: &Here, acts: &mut Vec<Act>) {
    let state = here.state;
    let starred = newest(
        here,
        episodes()
            .iter()
            .filter(|episode| state.episode(&episode.id).is_some_and(|row| row.starred))
            .collect(),
    );
    PageHeader::new("Starred")
        .count(episodes_word(starred.len()))
        .show(ui, |_| {});
    if starred.is_empty() {
        nothing(
            ui,
            Icon::Star,
            "Nothing starred",
            "Star an episode from its row or its page, and it is kept here.",
            None,
        );
        return;
    }
    let look = ListLook {
        with_show: true,
        ..ListLook::default()
    };
    episode_list(ui, here, "starred", &starred, look, acts);
}

/// Downloads: the episodes kept on this device and those on their way, with the room they take.
fn downloads(ui: &mut Ui, here: &Here, acts: &mut Vec<Act>) {
    let state = here.state;
    let of = |wanted: DownloadState| {
        newest(
            here,
            episodes()
                .iter()
                .filter(|episode| state.episode(&episode.id).is_some_and(|row| row.download == wanted))
                .collect(),
        )
    };
    let running = of(DownloadState::Running);
    let kept = of(DownloadState::Kept);
    let size: f64 = kept.iter().map(|episode| size_of(&episode.id)).sum();
    PageHeader::new("Downloads")
        .count(format!("{} kept, {}", kept.len(), megabytes(size)))
        .show(ui, |ui| {
            if ui
                .add(outline("Remove all", Some(Icon::Trash2)).enabled(kept.len() + running.len() > 0))
                .clicked()
            {
                acts.push(Act::Ask(Asking::RemoveDownloads));
            }
        });
    let look = ListLook {
        with_show: true,
        plain: true,
        ..ListLook::default()
    };
    if !running.is_empty() {
        Section::new("Downloading")
            .flush(true)
            .meta(running.len().to_string())
            .show(ui, |ui| episode_list(ui, here, "downloading", &running, look, acts));
    }
    if !kept.is_empty() {
        Section::new("On this device")
            .flush(true)
            .meta(megabytes(size))
            .show(ui, |ui| episode_list(ui, here, "kept", &kept, look, acts));
    }
    if kept.is_empty()
        && running.is_empty()
        && nothing(
            ui,
            Icon::Download,
            "No downloads",
            "Download an episode from its row to keep it on this device, or turn on automatic downloads in Settings.",
            Some(("Settings", None)),
        )
    {
        acts.push(Act::Go("settings".into()));
    }
}

/// Stats: the time listened, this week and in all, the episodes finished, the time speed saved, and
/// the shows heard most.
fn stats(ui: &mut Ui, here: &Here) {
    let numbers = Stats::of(&here.state.listens, here.now);
    PageHeader::new("Stats")
        .description("Time listened, counted as you play, kept with your settings.")
        .show(ui, |_| {});
    if numbers.total == 0 {
        nothing(
            ui,
            Icon::ChartNoAxesColumn,
            "Nothing listened yet",
            "Play an episode, and the time you listen is counted here.",
            None,
        );
        return;
    }
    let tiles = numbers.tiles();
    Columns::new(4).min(160.0).show(ui, tiles.len(), |ui, index, width| {
        let (label, value, note) = &tiles[index];
        Stat::new(*label, value).note(*note).tile(ui, width);
    });
    let bars = |bars: Vec<crate::stats::Bar>| -> (Vec<BarListItem>, Vec<String>) {
        let written = bars.iter().map(|bar| bar.written.clone()).collect();
        let items = bars
            .into_iter()
            .map(|bar| BarListItem {
                label: bar.label,
                value: bar.value,
                chosen: bar.chosen,
                note: None,
            })
            .collect();
        (items, written)
    };
    let (shows, shows_written) = bars(numbers.show_bars());
    let (days, days_written) = bars(numbers.day_bars(here.now));
    Columns::new(2).min(448.0).show(ui, 2, |ui, index, _| {
        let (title, items, written) = if index == 0 {
            ("Top shows", &shows, &shows_written)
        } else {
            ("The last seven days", &days, &days_written)
        };
        Section::new(title).show(ui, |ui| {
            BarList::new(items)
                .format(|share| {
                    items
                        .iter()
                        .position(|item| item.value == share)
                        .map(|at| written[at].clone())
                        .unwrap_or_default()
                })
                .show(ui);
        });
    });
}
