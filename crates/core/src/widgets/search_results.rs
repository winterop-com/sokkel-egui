//! A search's results page, after sokkel-ui's `SearchResults`, and the shapes of a search's
//! answer, after its `lib/search`.

use std::collections::BTreeMap;

use egui::{Align, Layout, Rect, Sense, TextWrapMode, Ui, WidgetText, pos2, vec2};

use crate::{Checkbox, Icon, SearchField, TextRole, icons, tokens};

/// One thing a search found.
#[derive(Clone, Debug, Default)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    /// The line under the title: where it lives, what it says.
    pub detail: Option<String>,
    /// A quiet fact at the row's end, such as a place or a time.
    pub meta: Option<String>,
    /// An icon on a muted tile before the title.
    pub icon: Option<Icon>,
    /// The result's value for each of the app's facets, such as `folder: Runbooks`.
    pub facets: BTreeMap<String, String>,
}

impl PartialEq for SearchResult {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.title == other.title
            && self.detail == other.detail
            && self.meta == other.meta
            && self.icon.map(char::from) == other.icon.map(char::from)
            && self.facets == other.facets
    }
}

impl Eq for SearchResult {}

/// The results of one kind, under that kind's heading.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchGroup {
    /// The kind's key, the value the kind facet filters on.
    pub kind: String,
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

/// A facet the results are narrowed by: the kind, or one of the app's, such as the folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchFacet {
    pub id: String,
    pub title: String,
    pub values: Vec<SearchFacetValue>,
}

/// The facet a results page builds from the groups themselves: one value per kind.
pub const KIND_FACET: &str = "kind";

/// The values chosen in each facet, by the facet's id; a facet with none chosen keeps every result.
pub type SearchChosen = BTreeMap<String, Vec<String>>;

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

/// The words a results page says.
pub struct SearchResultsWords {
    pub placeholder: String,
    /// How many results match in all.
    pub count: Box<dyn Fn(usize) -> String>,
    /// Said where nothing matches the query (blank for none typed).
    pub empty: Box<dyn Fn(&str) -> String>,
    pub clear: String,
}

impl Default for SearchResultsWords {
    fn default() -> Self {
        Self {
            placeholder: "Search".into(),
            count: Box::new(|total| {
                if total == 1 {
                    "1 result".to_owned()
                } else {
                    format!("{} results", super::table::thousands(total))
                }
            }),
            empty: Box::new(|query| format!("Nothing matches \u{201c}{query}\u{201d}.")),
            clear: "Clear the search".into(),
        }
    }
}

/// What the results page was asked this frame.
#[derive(Clone, Debug, Default)]
pub struct SearchResultsResponse {
    /// The query was typed in.
    pub changed: bool,
    /// The facets' choices after a value was turned on or off.
    pub chosen: Option<SearchChosen>,
    /// A result opened: its group's kind and its id.
    pub opened: Option<(String, String)>,
}

/// Where the query's words stand in a text, in any case, in order and not overlapping.
fn marks(text: &str, query: &str) -> Vec<std::ops::Range<usize>> {
    let lower = text.to_lowercase();
    if lower.len() != text.len() {
        return Vec::new();
    }
    let mut out: Vec<std::ops::Range<usize>> = Vec::new();
    for word in query.split_whitespace().map(str::to_lowercase) {
        let mut from = 0;
        while let Some(at) = lower[from..].find(&word).map(|at| at + from) {
            out.push(at..at + word.len());
            from = at + word.len().max(1);
        }
    }
    out.sort_by_key(|r| r.start);
    let mut merged: Vec<std::ops::Range<usize>> = Vec::new();
    for range in out {
        match merged.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    merged
}

/// A text with the query's words marked on the primary wash in the primary ink, as sokkel-ui's
/// `Highlight`, cut short at `width`.
pub(crate) fn marked(
    ui: &Ui,
    text: &str,
    query: &str,
    role: TextRole,
    weight: f32,
    ink: egui::Color32,
    width: f32,
) -> egui::text::LayoutJob {
    let t = tokens(ui.ctx());
    let mut job = egui::text::LayoutJob::default();
    let mut at = 0;
    let mut add = |piece: &str, on: bool| {
        if piece.is_empty() {
            return;
        }
        let mut rich = role.rich(&t, piece).variation("wght", weight);
        rich = if on {
            rich.color(t.color("primary-ink"))
                .background_color(t.color_at("primary", 0.15))
        } else {
            rich.color(ink)
        };
        rich.append_to(&mut job, ui.style(), egui::FontSelection::Default, Align::Center);
    };
    for range in marks(text, query) {
        add(&text[at..range.start], false);
        add(&text[range.clone()], true);
        at = range.end;
    }
    add(&text[at..], false);
    job.wrap = egui::text::TextWrapping::truncate_at_width(width);
    job
}

/// The page of a search's results, as sokkel-ui's `SearchResults`: the query in a `SearchField` at
/// most 576 wide with the count beside it, then the facets in a 224 column (each a group label over
/// its values as checkboxes with their counts) beside the results grouped by kind under their
/// headings. A result's row: its icon on a 40 muted tile, its title at medium weight over its line,
/// the query's words marked in both, and its quiet fact at the end. The page draws what it is
/// given: the app narrows the groups and counts the facets.
pub struct SearchResults<'a> {
    query: &'a mut String,
    groups: &'a [SearchGroup],
    facets: &'a [SearchFacet],
    chosen: &'a SearchChosen,
    total: Option<usize>,
    words: SearchResultsWords,
}

impl<'a> SearchResults<'a> {
    pub fn new(query: &'a mut String, groups: &'a [SearchGroup]) -> Self {
        Self {
            query,
            groups,
            facets: &[],
            chosen: EMPTY.get_or_init(SearchChosen::new),
            total: None,
            words: SearchResultsWords::default(),
        }
    }

    pub fn facets(mut self, facets: &'a [SearchFacet], chosen: &'a SearchChosen) -> Self {
        self.facets = facets;
        self.chosen = chosen;
        self
    }

    /// How many results match in all; the listed ones are counted otherwise.
    pub fn total(mut self, total: usize) -> Self {
        self.total = Some(total);
        self
    }

    pub fn words(mut self, words: SearchResultsWords) -> Self {
        self.words = words;
        self
    }

    pub fn show(self, ui: &mut Ui) -> SearchResultsResponse {
        let t = tokens(ui.ctx());
        let mut out = SearchResultsResponse::default();
        let listed: usize = self.groups.iter().map(|g| g.results.len()).sum();
        let all = self
            .total
            .unwrap_or_else(|| self.groups.iter().map(|g| g.total.unwrap_or(g.results.len())).sum());
        let spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing = vec2(16.0, 16.0);
        ui.horizontal(|ui| {
            let before = self.query.clone();
            SearchField::new("search-results-query", self.query)
                .placeholder(&self.words.placeholder)
                .keys("/")
                .width(ui.available_width().min(576.0))
                .show(ui);
            out.changed = *self.query != before;
            ui.label(
                TextRole::Body
                    .rich(&t, (self.words.count)(all))
                    .color(t.color("muted-foreground")),
            );
        });
        let query = self.query.trim().to_owned();
        let shown: Vec<&SearchFacet> = self.facets.iter().filter(|f| !f.values.is_empty()).collect();
        let draw_results = |ui: &mut Ui, out: &mut SearchResultsResponse| {
            ui.spacing_mut().item_spacing = vec2(0.0, 20.0);
            if listed == 0 {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        TextRole::Body
                            .rich(&t, (self.words.empty)(&query))
                            .color(t.color("muted-foreground")),
                    );
                });
                return;
            }
            for group in self.groups.iter().filter(|g| !g.results.is_empty()) {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        super::group_label(ui, &group.title);
                        let total = group.total.unwrap_or(group.results.len());
                        if total > group.results.len() {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                super::group_label(ui, &total.to_string());
                            });
                        }
                    });
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for result in &group.results {
                        if self.result_row(ui, result, &query).clicked() {
                            out.opened = Some((group.kind.clone(), result.id.clone()));
                        }
                    }
                });
            }
        };
        if shown.is_empty() {
            draw_results(ui, &mut out);
        } else {
            let wide = ui.available_width() >= 768.0;
            let facets = |ui: &mut Ui, out: &mut SearchResultsResponse| {
                ui.spacing_mut().item_spacing = vec2(0.0, 16.0);
                for facet in &shown {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            super::group_label(ui, &facet.title);
                        });
                        let on: &[String] = self.chosen.get(&facet.id).map_or(&[], Vec::as_slice);
                        for value in &facet.values {
                            let chosen = on.contains(&value.value);
                            if self.facet_row(ui, facet, value, chosen) {
                                out.chosen = Some(toggle_chosen(self.chosen, &facet.id, &value.value));
                            }
                        }
                    });
                }
            };
            if wide {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 24.0;
                    ui.allocate_ui_with_layout(vec2(224.0, 0.0), Layout::top_down(Align::Min), |ui| {
                        ui.set_width(224.0);
                        facets(ui, &mut out);
                    });
                    ui.vertical(|ui| draw_results(ui, &mut out));
                });
            } else {
                facets(ui, &mut out);
                draw_results(ui, &mut out);
            }
        }
        ui.spacing_mut().item_spacing = spacing;
        out
    }

    /// One facet value: a checkbox, its name and its count at the end, 8 across and 4 over and
    /// under, the hover ground; a value that keeps nothing in the faint ink. True when pressed.
    fn facet_row(&self, ui: &mut Ui, facet: &SearchFacet, value: &SearchFacetValue, chosen: bool) -> bool {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        let height = 8.0 + TextRole::Body.line_height(&t);
        let (rect, hit) = ui.allocate_exact_size(vec2(width, height), Sense::click());
        if hit.hovered() {
            ui.painter().rect_filled(rect, t.radius("control"), t.color("hover"));
        }
        let mut checked = chosen;
        let place = Rect::from_min_size(pos2(rect.left() + 8.0, rect.center().y - 8.0), vec2(16.0, 16.0));
        let label = value.label.clone().unwrap_or_else(|| value.value.clone());
        let boxed = ui.put(place, Checkbox::new(&mut checked, &label).hide_label());
        let ink = if value.count == 0 && !chosen {
            t.color("faint")
        } else {
            t.color("foreground")
        };
        let count = WidgetText::from(TextRole::Help.rich(&t, value.count.to_string())).into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            TextRole::Help.font(&t),
        );
        let count_x = rect.right() - 8.0 - count.size().x;
        let name = WidgetText::from(TextRole::Body.rich(&t, &label)).into_galley(
            ui,
            Some(TextWrapMode::Truncate),
            (count_x - 10.0 - (place.right() + 10.0)).max(1.0),
            TextRole::Body.font(&t),
        );
        ui.painter().galley(
            pos2(place.right() + 10.0, rect.center().y - name.size().y / 2.0),
            name,
            ink,
        );
        ui.painter().galley(
            pos2(count_x, rect.center().y - count.size().y / 2.0),
            count,
            t.color("faint"),
        );
        let _ = facet;
        hit.clicked() || boxed.clicked()
    }

    /// One result's row, 8 across and over and under, the hover ground and the medium radius.
    fn result_row(&self, ui: &mut Ui, result: &SearchResult, query: &str) -> egui::Response {
        let t = tokens(ui.ctx());
        let width = ui.available_width();
        let tile = 40.0;
        let (rect, hit) = ui.allocate_exact_size(vec2(width, tile + 16.0), Sense::click());
        hit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Link, true, &result.title));
        if hit.hovered() {
            ui.painter().rect_filled(rect, t.radius("md"), t.color("hover"));
        }
        let mut x = rect.left() + 8.0;
        if let Some(icon) = result.icon {
            let square = Rect::from_min_size(pos2(x, rect.center().y - tile / 2.0), vec2(tile, tile));
            ui.painter().rect_filled(square, t.radius("md"), t.color("muted"));
            icons::paint(ui.painter(), square.center(), icon, 16.0, t.color("muted-foreground"));
            x += tile + 10.0;
        }
        let mut end = rect.right() - 8.0;
        if let Some(meta) = &result.meta {
            let galley = WidgetText::from(TextRole::Help.rich(&t, meta)).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                TextRole::Help.font(&t),
            );
            end -= galley.size().x;
            ui.painter().galley(
                pos2(end, rect.center().y - galley.size().y / 2.0),
                galley,
                t.color("muted-foreground"),
            );
            end -= 10.0;
        }
        let room = (end - x).max(1.0);
        let title = ui.painter().layout_job(marked(
            ui,
            &result.title,
            query,
            TextRole::Body,
            t.weight(crate::Weight::Medium),
            t.color("foreground"),
            room,
        ));
        let detail = result.detail.as_ref().map(|detail| {
            ui.painter().layout_job(marked(
                ui,
                detail,
                query,
                TextRole::Help,
                400.0,
                t.color("muted-foreground"),
                room,
            ))
        });
        let height = title.size().y + detail.as_ref().map_or(0.0, |d| d.size().y);
        let mut y = rect.center().y - height / 2.0;
        let title_height = title.size().y;
        ui.painter().galley(pos2(x, y), title, t.color("foreground"));
        y += title_height;
        if let Some(detail) = detail {
            ui.painter().galley(pos2(x, y), detail, t.color("muted-foreground"));
        }
        hit
    }
}

static EMPTY: std::sync::OnceLock<SearchChosen> = std::sync::OnceLock::new();
