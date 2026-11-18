//! The places records are made at, as the web's `pages/Places.tsx`: the country, its regions and
//! their sites, in a tree and on a map that share one selection, the selected place's facts beside.

use std::collections::HashSet;

use eframe::egui::{self, Ui, vec2};
use sokkel_egui::{
    ApiLink, Chip, DescriptionList, Inspector, PageHeader, SearchField, Section, SubNav, TextLink, TextRole, Tree,
    inspector_section, tokens,
};
use sokkel_egui_maps::{Area, Emphasis, Marker};

use crate::app::{Foot, RecordsApp};
use crate::backend::STATES;
use crate::form_fill::place_tree;
use crate::forms::{Form, forms};
use crate::geo::{COUNTRY, Level, Place, boundaries, children_of, path_of, place_of, places, under};
use crate::ui::{count_as, state_chip, when};

/// What the places page holds between frames: the column's width, the search, the tree's open
/// branches, the map, and the place the map was last framed for.
pub struct PlacesView {
    pub column: f32,
    pub query: String,
    pub open: HashSet<String>,
    pub map: sokkel_egui_maps::MapView,
    pub framed_for: Option<String>,
}

impl Default for PlacesView {
    fn default() -> Self {
        let mut map = sokkel_egui_maps::MapView::new(6.4, -9.4, 6.0).framed(false);
        map.bare = true;
        Self {
            column: 272.0,
            query: String::new(),
            open: HashSet::new(),
            map,
            framed_for: None,
        }
    }
}

/// What the map frames for a place, the areas and sites lit with it, and a note when it has no
/// geometry of its own.
fn framing(place: &Place) -> (Vec<String>, Option<[f64; 4]>, Option<String>) {
    let union = |ids: &[&str]| {
        boundaries()
            .iter()
            .filter(|b| ids.contains(&b.id.as_str()))
            .filter_map(|b| Area::new(&b.id, &b.name, outer_rings(b)).bounds())
            .reduce(|a, b| [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])])
    };
    match place.level {
        Level::Country => (
            boundaries().iter().map(|b| b.id.clone()).collect(),
            union(&boundaries().iter().map(|b| b.id.as_str()).collect::<Vec<_>>()),
            Some(format!(
                "{} has no boundary of its own here, so the map frames the regions under it.",
                place.name
            )),
        ),
        Level::Region => (
            children_of(&place.id).iter().map(|s| s.id.clone()).collect(),
            union(&[place.id.as_str()]),
            None,
        ),
        Level::Site => match place.position {
            Some((lon, lat)) => (Vec::new(), Some([lon - 0.05, lat - 0.05, lon + 0.05, lat + 0.05]), None),
            None => {
                let parent = place.parent.as_deref().unwrap_or(COUNTRY);
                (
                    Vec::new(),
                    union(&[parent]),
                    Some(format!(
                        "{} has no position that can be read, so the map frames {}, the region it is in.",
                        place.name,
                        place_of(parent).map_or("its region", |p| p.name.as_str())
                    )),
                )
            }
        },
    }
}

fn outer_rings(b: &crate::geo::Boundary) -> Vec<Vec<(f64, f64)>> {
    b.polygons.iter().filter_map(|p| p.first().cloned()).collect()
}

/// The forms a place reports: periodic reports from a region or a site, events and visits from a site.
fn forms_at(place: &Place) -> Vec<&'static Form> {
    forms()
        .iter()
        .filter(|f| match place.level {
            Level::Country => false,
            _ if f.id == "FRM-STOCK" => place.level == Level::Region,
            _ => place.level == Level::Site,
        })
        .collect()
}

impl RecordsApp {
    pub fn places_page(&mut self, ui: &mut Ui, given: Option<&str>) {
        let selected = given.filter(|g| place_of(g).is_some()).unwrap_or(COUNTRY).to_owned();
        let place = place_of(&selected).expect("a place").clone();
        let path = path_of(&selected);
        self.foot = Foot {
            note: path.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(" / "),
            items: Vec::new(),
        };
        let (highlighted, bounds, note) = framing(&place);
        let mut view = std::mem::take(&mut self.places);
        if view.framed_for.as_deref() != Some(selected.as_str()) {
            // the tree opens to the place, and the map frames it
            view.open
                .extend(path.iter().take(path.len().saturating_sub(1)).map(|p| p.id.clone()));
            if place.level != Level::Site {
                view.open.insert(selected.clone());
            }
            view.map.frame(bounds);
            view.map.selected = (selected != COUNTRY).then(|| selected.clone());
            view.framed_for = Some(selected.clone());
        }
        let below: HashSet<String> = std::iter::once(selected.clone())
            .chain(under(&selected).into_iter().map(|p| p.id.clone()))
            .collect();
        let here: Vec<&crate::backend::Receipt> = self
            .backend
            .receipts
            .iter()
            .filter(|r| below.contains(&r.place))
            .collect();
        let unplaced = places()
            .iter()
            .filter(|p| p.level == Level::Site && p.position.is_none())
            .count();
        let sites = places().iter().filter(|p| p.level == Level::Site).count();
        let mut chosen: Option<String> = None;
        let mut go: Option<String> = None;
        let tree = place_tree();
        let t = tokens(ui.ctx());
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let mut column = view.column;
            let mut query = view.query.clone();
            let shown_query = view.query.clone();
            SubNav::new("places-hierarchy", &mut column).show(
                ui,
                |ui| {
                    SearchField::new("places-search", &mut query).keys("/").show(ui);
                },
                |ui| {
                    let mut picked = Some(selected.clone());
                    let out = Tree::new("places-tree", &tree, &mut picked, &mut view.open)
                        .query(&shown_query)
                        .show(ui);
                    if let Some(id) = out.chosen {
                        chosen = Some(id);
                    }
                },
            );
            view.column = column;
            view.query = query;
            let room = ui.available_rect_before_wrap();
            let mut page = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(room)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            sokkel_egui::Page::show(&mut page, |ui| {
                PageHeader::new("Places")
                    .description("The places records are made at, in the hierarchy the server holds them in: where a record may be filed from, and which forms it may use.")
                    .show(ui, |ui| {
                        ui.add(ApiLink::new(if place.level == Level::Country {
                            "/places".to_owned()
                        } else {
                            format!("/places/{}", place.id)
                        }));
                    });
                let height = ui.available_height().max(384.0);
                ui.horizontal_top(|ui| {
                    let gap = t.density("inspector-gap");
                    let map_width = (ui.available_width() - 360.0 - gap).max(320.0);
                    ui.spacing_mut().item_spacing.x = gap;
                    ui.allocate_ui_with_layout(vec2(map_width, height), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(map_width);
                        Section::new("Map")
                            .meta(format!(
                                "{}, {}, no basemap offered",
                                count_as(boundaries().len(), "boundary", "boundaries"),
                                count_as(sites - unplaced, "point", "points")
                            ))
                            .flush(true)
                            .show(ui, |ui| {
                                let notes: Vec<String> = note
                                    .iter()
                                    .cloned()
                                    .chain((unplaced > 0).then(|| {
                                        format!(
                                            "{} could not be read and is not drawn.",
                                            count_as(unplaced, "geometry", "geometries")
                                        )
                                    }))
                                    .collect();
                                // the notes' height as last drawn, so the map takes the rest
                                let notes_id = egui::Id::unique("places-notes-height");
                                let notes_height: f32 = ui.data(|d| d.get_temp(notes_id)).unwrap_or(64.0);
                                let map_height = (height - t.density("panel-head") - 2.0 - notes_height).max(240.0);
                                view.map.areas = boundaries()
                                    .iter()
                                    .map(|b| {
                                        let emphasis = if b.id == selected {
                                            Emphasis::Selected
                                        } else if highlighted.contains(&b.id) {
                                            Emphasis::Highlighted
                                        } else {
                                            Emphasis::Plain
                                        };
                                        Area::new(&b.id, &b.name, outer_rings(b)).emphasis(emphasis)
                                    })
                                    .collect();
                                let markers: Vec<Marker> = places()
                                    .iter()
                                    .filter(|p| p.level == Level::Site)
                                    .filter_map(|p| {
                                        let (lon, lat) = p.position?;
                                        let color = if p.id == selected {
                                            t.color("map-selection")
                                        } else if place.level == Level::Region && highlighted.contains(&p.id) {
                                            t.color("map-highlight")
                                        } else {
                                            t.color("muted-foreground")
                                        };
                                        let lit = p.id == selected || highlighted.contains(&p.id);
                                        Some(
                                            Marker::new(lat, lon, "")
                                                .id(&p.id)
                                                .radius(if lit { 4.0 } else { 3.0 })
                                                .color(color),
                                        )
                                    })
                                    .collect();
                                let size = vec2(ui.available_width(), map_height);
                                view.map.show(ui, size, &markers, &[]);
                                if view.map.selected.as_deref() != (selected != COUNTRY).then_some(selected.as_str()) {
                                    chosen = Some(view.map.selected.clone().unwrap_or_else(|| COUNTRY.to_owned()));
                                }
                                let rule = ui.available_rect_before_wrap();
                                ui.painter().hline(
                                    rule.x_range(),
                                    rule.top(),
                                    egui::Stroke::new(1.0, t.color("border")),
                                );
                                let notes_top = ui.cursor().top();
                                egui::Frame::new().inner_margin(egui::Margin::symmetric(12, 8)).show(ui, |ui| {
                                    ui.spacing_mut().item_spacing.y = 4.0;
                                    for note in &notes {
                                        ui.add(
                                            egui::Label::new(TextRole::Help.rich(&t, note).color(t.color("muted-foreground")))
                                                .wrap(),
                                        );
                                    }
                                });
                                let measured = ui.cursor().top() - notes_top;
                                if (measured - notes_height).abs() > 0.5 {
                                    ui.data_mut(|d| d.insert_temp(notes_id, measured));
                                    ui.ctx().request_repaint();
                                }
                            });
                    });
                    let status = place.level.label();
                    Inspector::new("place-inspector", &place.name)
                        .code(&place.id)
                        .status(status)
                        .show(ui, height, |ui| {
                            inspector_section(ui, place.level.label(), |ui| {
                                let mut list = DescriptionList::new().side(88.0).code("Code", &place.id);
                                list = match place.level {
                                    Level::Site => match place.position {
                                        Some((lon, lat)) => list.code("Position", format!("{lon:.4}, {lat:.4}")),
                                        None => list.text("Position", "Could not be read"),
                                    },
                                    level => {
                                        let word = if level == Level::Country { "Regions" } else { "Sites" };
                                        let list = list.text(word, children_of(&place.id).len().to_string());
                                        if level == Level::Country {
                                            list.text("Sites", sites.to_string())
                                        } else {
                                            list
                                        }
                                    }
                                };
                                let trail: Vec<(String, String)> =
                                    path.iter().map(|p| (p.id.clone(), p.name.clone())).collect();
                                let current = selected.clone();
                                let cell = std::cell::RefCell::new(None::<String>);
                                let to = &cell;
                                list.item("Path", move |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing.x = 0.0;
                                        for (index, (id, name)) in trail.iter().enumerate() {
                                            if index > 0 {
                                                ui.label(" / ");
                                            }
                                            if *id == current {
                                                ui.label(name);
                                            } else if ui.add(TextLink::new(name)).clicked() {
                                                *to.borrow_mut() = Some(id.clone());
                                            }
                                        }
                                    });
                                })
                                .show(ui);
                                if let Some(id) = cell.into_inner() {
                                    chosen = Some(id);
                                }
                            });
                            let reported = forms_at(&place);
                            inspector_section(ui, &format!("Forms reported here ({})", reported.len()), |ui| {
                                if reported.is_empty() {
                                    ui.add(
                                        egui::Label::new(
                                            TextRole::Help
                                                .rich(&t, "Records are made at a region or a site; pick one to see its forms.")
                                                .color(t.color("muted-foreground")),
                                        )
                                        .wrap(),
                                    );
                                }
                                for form in reported {
                                    ui.horizontal(|ui| {
                                        if ui.add(TextLink::new(form.title)).clicked() {
                                            go = Some(format!("forms/{}", form.id));
                                        }
                                        ui.label(
                                            TextRole::Help
                                                .rich(&t, form.kind.label())
                                                .color(t.color("muted-foreground")),
                                        );
                                    });
                                }
                            });
                            inspector_section(ui, &format!("Recorded here ({})", here.len()), |ui| {
                                if here.is_empty() {
                                    ui.add(
                                        egui::Label::new(
                                            TextRole::Help
                                                .rich(&t, "No record the server holds names this place or a place under it.")
                                                .color(t.color("muted-foreground")),
                                        )
                                        .wrap(),
                                    );
                                    return;
                                }
                                ui.spacing_mut().item_spacing.y = 8.0;
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    for state in STATES {
                                        let n = here.iter().filter(|r| r.state == state).count();
                                        if n > 0 {
                                            let words = format!("{n} {}", state.label().to_lowercase());
                                            ui.add(Chip::new(words).tone(crate::ui::state_tone(state)));
                                        }
                                    }
                                });
                                for receipt in here.iter().take(5) {
                                    let title = crate::forms::form_of(&receipt.form_id)
                                        .map_or(receipt.form_id.as_str(), |f| f.title);
                                    if ui.add(TextLink::new(title)).clicked() {
                                        go = Some(format!("responses/{}", receipt.receipt));
                                    }
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            TextRole::Help
                                                .rich(&t, format!("{} ·", when(receipt.received_at)))
                                                .color(t.color("muted-foreground")),
                                        );
                                        ui.add(state_chip(receipt.state));
                                    });
                                }
                                if ui.add(TextLink::new("All responses")).clicked() {
                                    go = Some("responses".into());
                                }
                            });
                        });
                });
            });
        });
        self.places = view;
        if let Some(id) = chosen.filter(|id| *id != selected) {
            go = Some(if id == COUNTRY {
                "places".into()
            } else {
                format!("places/{id}")
            });
        }
        if let Some(path) = go {
            self.go(&path);
        }
    }
}
