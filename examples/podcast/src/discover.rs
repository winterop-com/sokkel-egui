//! The search on the Discover page, as the web's `pages/Discover.tsx` makes it: every word of a
//! query in a show's or a released episode's words.

use crate::catalog::{Episode, Show, category_label, episodes, show_by_id, shows};
use crate::js;

/// Whether every word of `query` is in one of `texts`.
pub fn matches(query: &str, texts: &[&str]) -> bool {
    let query = query.to_lowercase();
    let haystack = texts.join(" ").to_lowercase();
    query
        .split(js::is_space)
        .filter(|word| !word.is_empty())
        .all(|word| haystack.contains(word))
}

/// The shows and released episodes a query finds.
pub fn find(query: &str, released: impl Fn(&str) -> bool) -> (Vec<&'static Show>, Vec<&'static Episode>) {
    let found_shows = shows()
        .iter()
        .filter(|show| {
            let hosts: Vec<&str> = show.hosts.iter().map(|host| host.name()).collect();
            matches(
                query,
                &[
                    &show.title,
                    &show.blurb,
                    &hosts.join(" "),
                    category_label(show.category),
                ],
            )
        })
        .collect();
    let found_episodes = episodes()
        .iter()
        .filter(|episode| {
            if !released(&episode.id) {
                return false;
            }
            let show = show_by_id(&episode.show).map_or("", |show| show.title.as_str());
            let mut texts: Vec<&str> = vec![&episode.title, &episode.summary, &episode.place, show];
            texts.extend(episode.chapters.iter().map(String::as_str));
            matches(query, &texts)
        })
        .collect();
    (found_shows, found_episodes)
}
