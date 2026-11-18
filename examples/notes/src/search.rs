//! The vault searched, as the web's Search page and its top bar search it (`searchVault`), on the
//! parts of sokkel-ui's `lib/search.ts` it uses: a query's words, where they occur, results grouped
//! by kind and narrowed and counted by facet.

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;

use crate::model::{is_space, locale_compare, lowercase, trim};
use crate::vault::{Vault, line_around};

/// One thing a search found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    /// The line under the title: where it lives, what it is, when it last changed.
    pub detail: Option<String>,
    /// A quiet fact at the row's end, such as a state or a time.
    pub meta: Option<String>,
    /// Whether it is a daily note, drawn with a calendar where another note has a page.
    pub daily: bool,
    /// The result's value for each of the app's facets, such as `folder: Runbooks`.
    pub facets: BTreeMap<String, String>,
}

/// The results of one kind, under that kind's heading.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchGroup {
    /// The kind's key, the value the kind facet filters on.
    pub kind: String,
    /// The kind's heading.
    pub title: String,
    pub results: Vec<SearchResult>,
    /// How many of this kind match in all, when more match than are listed.
    pub total: Option<usize>,
}

/// One value of a facet with how many results carry it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchFacetValue {
    pub value: String,
    /// What the value is called; the value itself otherwise.
    pub label: Option<String>,
    pub count: usize,
}

/// A facet the results page filters by: the kind, or one of the app's, such as the folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchFacet {
    pub id: String,
    pub title: String,
    pub values: Vec<SearchFacetValue>,
}

/// One of the app's own facets, as `countFacets` is given it: its id, title and the labels of its
/// values.
#[derive(Clone, Debug, Default)]
pub struct FacetSpec {
    pub id: String,
    pub title: String,
    pub labels: HashMap<String, String>,
}

/// The facet a results page builds from the groups themselves: one value per kind.
pub const KIND_FACET: &str = "kind";

/// The values chosen in each facet, by the facet's id; a facet with none chosen keeps every result.
pub type SearchChosen = BTreeMap<String, Vec<String>>;

/// The words of a query, lower-cased, without repeats; a word inside another is dropped.
pub fn query_words(query: &str) -> Vec<String> {
    let lower = lowercase(query);
    let mut words: Vec<String> = Vec::new();
    for word in lower.split(is_space).filter(|word| !word.is_empty()) {
        if !words.iter().any(|one| one == word) {
            words.push(word.to_owned());
        }
    }
    words
        .iter()
        .filter(|word| {
            !words
                .iter()
                .any(|other| other != *word && other.contains(word.as_str()))
        })
        .cloned()
        .collect()
}

/// Where a term occurs in a text, every time and in any case; a blank term occurs nowhere.
pub fn literal_ranges(text: &str, term: &str) -> Vec<Range<usize>> {
    let needle = lowercase(trim(term));
    if needle.is_empty() {
        return Vec::new();
    }
    let lower = lowercase(text);
    let mut ranges = Vec::new();
    let mut from = 0;
    while let Some(at) = lower
        .get(from..)
        .and_then(|rest| rest.find(&needle))
        .map(|at| at + from)
    {
        ranges.push(at..at + needle.len());
        from = at + needle.len();
    }
    ranges
}

/// Where the query's words occur in a text, in any case, in order.
pub fn query_ranges(text: &str, query: &str) -> Vec<Range<usize>> {
    let mut ranges: Vec<Range<usize>> = query_words(query)
        .iter()
        .flat_map(|word| literal_ranges(text, word))
        .collect();
    ranges.sort_by_key(|range| range.start);
    ranges
}

/// Whether a result in a group is kept by the chosen facet values, `skip` aside.
fn kept(group: &SearchGroup, result: &SearchResult, chosen: &SearchChosen, skip: Option<&str>) -> bool {
    chosen.iter().all(|(facet, values)| {
        if Some(facet.as_str()) == skip || values.is_empty() {
            return true;
        }
        let value = if facet == KIND_FACET {
            Some(&group.kind)
        } else {
            result.facets.get(facet)
        };
        value.is_some_and(|value| values.contains(value))
    })
}

/// The groups narrowed to the results the chosen facet values keep, dropping groups left empty.
pub fn filter_groups(groups: &[SearchGroup], chosen: &SearchChosen) -> Vec<SearchGroup> {
    if chosen.values().all(Vec::is_empty) {
        return groups
            .iter()
            .filter(|group| !group.results.is_empty())
            .cloned()
            .collect();
    }
    groups
        .iter()
        .map(|group| {
            let results: Vec<SearchResult> = group
                .results
                .iter()
                .filter(|result| kept(group, result, chosen, None))
                .cloned()
                .collect();
            SearchGroup {
                kind: group.kind.clone(),
                title: group.title.clone(),
                total: Some(results.len()),
                results,
            }
        })
        .filter(|group| !group.results.is_empty())
        .collect()
}

/// Counts the facets over the results: the kind, then each of `facets`, each value counted under
/// the choices made in the other facets, so a count says what choosing that value would show.
pub fn count_facets(
    groups: &[SearchGroup],
    facets: &[FacetSpec],
    chosen: &SearchChosen,
    kind_title: &str,
) -> Vec<SearchFacet> {
    let kind = SearchFacet {
        id: KIND_FACET.to_owned(),
        title: kind_title.to_owned(),
        values: groups
            .iter()
            .map(|group| SearchFacetValue {
                value: group.kind.clone(),
                label: Some(group.title.clone()),
                count: group
                    .results
                    .iter()
                    .filter(|result| kept(group, result, chosen, Some(KIND_FACET)))
                    .count(),
            })
            .collect(),
    };
    let own = facets.iter().map(|facet| {
        let mut counts: Vec<(String, usize)> = Vec::new();
        for group in groups {
            for result in &group.results {
                let Some(value) = result.facets.get(&facet.id) else {
                    continue;
                };
                if !kept(group, result, chosen, Some(&facet.id)) {
                    continue;
                }
                match counts.iter_mut().find(|(one, _)| one == value) {
                    Some((_, count)) => *count += 1,
                    None => counts.push((value.clone(), 1)),
                }
            }
        }
        let mut values: Vec<SearchFacetValue> = counts
            .into_iter()
            .map(|(value, count)| SearchFacetValue {
                label: facet.labels.get(&value).cloned(),
                value,
                count,
            })
            .collect();
        values.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| locale_compare(&a.value, &b.value, false))
        });
        SearchFacet {
            id: facet.id.clone(),
            title: facet.title.clone(),
            values,
        }
    });
    std::iter::once(kind).chain(own).collect()
}

/// The chosen values with one value of one facet turned on or off.
pub fn toggle_chosen(chosen: &SearchChosen, facet: &str, value: &str) -> SearchChosen {
    let mut next = chosen.clone();
    let values = next.entry(facet.to_owned()).or_default();
    match values.iter().position(|one| one == value) {
        Some(at) => {
            values.remove(at);
        }
        None => values.push(value.to_owned()),
    }
    next
}

/// What a search of the vault found: its groups narrowed by the choices, and its facets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VaultSearch {
    pub groups: Vec<SearchGroup>,
    pub facets: Vec<SearchFacet>,
}

/// Every note holding every word of a query, in its name or its text, in any case: those named by
/// it first, then those that say it, each with the line it is said in, newest first. Faceted by
/// kind and by folder.
pub fn search_vault(vault: &Vault, query: &str, chosen: &SearchChosen) -> VaultSearch {
    let words = query_words(query);
    if words.is_empty() {
        return VaultSearch::default();
    }
    let mut named = Vec::new();
    let mut said = Vec::new();
    let mut newest: Vec<_> = vault.notes.iter().collect();
    newest.sort_by_key(|note| std::cmp::Reverse(note.updated));
    for note in newest {
        let title = lowercase(&note.title);
        let body = lowercase(&note.body);
        if !words
            .iter()
            .all(|word| title.contains(word.as_str()) || body.contains(word.as_str()))
        {
            continue;
        }
        let first = words.iter().find_map(|word| body.find(word.as_str()));
        let mut place = vault.path(note.folder.as_deref());
        if place.is_empty() {
            place = "Vault".to_owned();
        }
        let result = SearchResult {
            id: note.id.clone(),
            title: note.title.clone(),
            detail: Some(match first {
                Some(at) => line_around(&note.body, at, at + 1, 110),
                None => place.clone(),
            }),
            meta: Some(place.clone()),
            daily: note.day.is_some(),
            facets: BTreeMap::from([("folder".to_owned(), place)]),
        };
        if words.iter().all(|word| title.contains(word.as_str())) {
            named.push(result);
        } else {
            said.push(result);
        }
    }
    let groups = [
        SearchGroup {
            kind: "names".to_owned(),
            title: "In the name".to_owned(),
            results: named,
            total: None,
        },
        SearchGroup {
            kind: "text".to_owned(),
            title: "In the text".to_owned(),
            results: said,
            total: None,
        },
    ];
    let folder = FacetSpec {
        id: "folder".to_owned(),
        title: "Folder".to_owned(),
        labels: HashMap::new(),
    };
    VaultSearch {
        groups: filter_groups(&groups, chosen),
        facets: count_facets(&groups, &[folder], chosen, "Found"),
    }
}

/// How many results of each kind the top bar's search lists before "see all".
pub const QUICK: usize = 6;

/// The vault searched as the top bar searches it: each group cut to its first `QUICK` results.
pub fn quick_search(vault: &Vault, query: &str) -> Vec<SearchGroup> {
    search_vault(vault, query, &SearchChosen::new())
        .groups
        .into_iter()
        .map(|mut group| {
            group.results.truncate(QUICK);
            group
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_and_ranges_read_as_the_web_reads_them() {
        assert_eq!(query_words("  Run  run RUNS r "), ["runs"]);
        assert_eq!(query_words("a b ab"), ["ab"]);
        assert!(query_words("").is_empty());
        assert_eq!(query_ranges("Retry a run, run again", "run re"), [0..2, 8..11, 13..16]);
        let chosen = toggle_chosen(&SearchChosen::new(), "kind", "text");
        assert_eq!(chosen["kind"], ["text"]);
        assert!(toggle_chosen(&chosen, "kind", "text")["kind"].is_empty());
    }
}
