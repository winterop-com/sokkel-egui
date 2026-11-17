//! The places records are made at, as the web's `data/geo.ts`: the country, its regions with their
//! boundaries, and the sites, one of them with a position that cannot be read.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde_json::Value;

pub const COUNTRY: &str = "sierra-leone";
pub const COUNTRY_NAME: &str = "Sierra Leone";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Level {
    Country,
    Region,
    Site,
}

impl Level {
    pub fn label(self) -> &'static str {
        match self {
            Level::Country => "Country",
            Level::Region => "Region",
            Level::Site => "Site",
        }
    }
}

/// A place in the hierarchy: the country, a region or a site.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub id: String,
    pub name: String,
    pub level: Level,
    /// The region a site is in, or the country a region is in.
    pub parent: Option<String>,
    /// A site's position as longitude and latitude, `None` where it cannot be read.
    pub position: Option<(f64, f64)>,
}

/// A region's boundary: its rings of longitude and latitude, the first of each polygon its outside.
#[derive(Clone, Debug)]
pub struct Boundary {
    pub id: String,
    pub name: String,
    pub polygons: Vec<Vec<Vec<(f64, f64)>>>,
}

struct Geo {
    places: Vec<Place>,
    by_id: HashMap<String, usize>,
    boundaries: Vec<Boundary>,
}

fn ring(value: &Value) -> Vec<(f64, f64)> {
    value
        .as_array()
        .map(|points| {
            points
                .iter()
                .filter_map(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?)))
                .collect()
        })
        .unwrap_or_default()
}

fn polygon(value: &Value) -> Vec<Vec<(f64, f64)>> {
    value
        .as_array()
        .map(|rings| rings.iter().map(ring).collect())
        .unwrap_or_default()
}

fn geo() -> &'static Geo {
    static GEO: OnceLock<Geo> = OnceLock::new();
    GEO.get_or_init(|| {
        let regions: Value = serde_json::from_str(include_str!("../data/regions.json")).expect("regions.json");
        let sites: Value = serde_json::from_str(include_str!("../data/places.json")).expect("places.json");
        let features = |v: &Value| v["features"].as_array().cloned().unwrap_or_default();
        let boundaries: Vec<Boundary> = features(&regions)
            .iter()
            .map(|f| {
                let geometry = &f["geometry"];
                let polygons = match geometry["type"].as_str() {
                    Some("MultiPolygon") => geometry["coordinates"]
                        .as_array()
                        .map(|all| all.iter().map(polygon).collect())
                        .unwrap_or_default(),
                    _ => vec![polygon(&geometry["coordinates"])],
                };
                Boundary {
                    id: f["id"].as_str().unwrap_or_default().to_owned(),
                    name: f["properties"]["name"].as_str().unwrap_or_default().to_owned(),
                    polygons,
                }
            })
            .collect();
        let mut region_places: Vec<Place> = boundaries
            .iter()
            .map(|b| Place {
                id: b.id.clone(),
                name: b.name.clone(),
                level: Level::Region,
                parent: Some(COUNTRY.to_owned()),
                position: None,
            })
            .collect();
        region_places.sort_by(|a, b| a.name.cmp(&b.name));
        let mut site_places: Vec<Place> = features(&sites)
            .iter()
            .map(|f| {
                let c = &f["geometry"]["coordinates"];
                Place {
                    id: f["id"].as_str().unwrap_or_default().to_owned(),
                    name: f["properties"]["name"].as_str().unwrap_or_default().to_owned(),
                    level: Level::Site,
                    parent: f["properties"]["region"].as_str().map(str::to_owned),
                    position: c[0].as_f64().zip(c[1].as_f64()),
                }
            })
            .collect();
        // one site whose position came back unreadable, as data from the field sometimes does
        site_places.push(Place {
            id: "bo/gerihun".into(),
            name: "Gerihun".into(),
            level: Level::Site,
            parent: Some("bo".into()),
            position: None,
        });
        site_places.sort_by(|a, b| a.name.cmp(&b.name));
        let mut places = vec![Place {
            id: COUNTRY.into(),
            name: COUNTRY_NAME.into(),
            level: Level::Country,
            parent: None,
            position: None,
        }];
        places.extend(region_places);
        places.extend(site_places);
        let by_id = places.iter().enumerate().map(|(i, p)| (p.id.clone(), i)).collect();
        Geo {
            places,
            by_id,
            boundaries,
        }
    })
}

/// Every place: the country, the regions by name, then the sites by name.
pub fn places() -> &'static [Place] {
    &geo().places
}

/// The regions' boundaries, in the file's order.
pub fn boundaries() -> &'static [Boundary] {
    &geo().boundaries
}

pub fn place_of(id: &str) -> Option<&'static Place> {
    geo().by_id.get(id).map(|i| &geo().places[*i])
}

/// A place's name, or its id when no place has it.
pub fn place_name(id: &str) -> String {
    place_of(id).map_or_else(|| id.to_owned(), |p| p.name.clone())
}

pub fn children_of(id: &str) -> Vec<&'static Place> {
    places().iter().filter(|p| p.parent.as_deref() == Some(id)).collect()
}

/// The places from the country down to this one.
pub fn path_of(id: &str) -> Vec<&'static Place> {
    let mut path = Vec::new();
    let mut at = place_of(id);
    while let Some(place) = at {
        path.insert(0, place);
        at = place.parent.as_deref().and_then(place_of);
    }
    path
}

/// Every place under one, at any depth.
pub fn under(id: &str) -> Vec<&'static Place> {
    children_of(id)
        .into_iter()
        .flat_map(|child| std::iter::once(child).chain(under(&child.id)))
        .collect()
}

/// A number from 0 to 1 that is always the same for the same text.
pub fn hashed(text: &str) -> f64 {
    let mut hash: u32 = 2_166_136_261;
    for unit in text.encode_utf16() {
        hash = (hash ^ u32::from(unit)).wrapping_mul(16_777_619);
    }
    f64::from(hash) / f64::from(u32::MAX)
}

/// The height of a site's bank above the gauge's zero, in meters, the same every time.
pub fn bank_height(site: &str) -> f64 {
    ((2.4 + hashed(&format!("{site}:bank")) * 2.2) * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_hierarchy_reads_as_the_web_reads_it() {
        assert_eq!(super::places()[0].id, super::COUNTRY);
        assert_eq!(super::boundaries().len(), 13);
        assert!(super::place_of("bo/gerihun").is_some_and(|p| p.position.is_none()));
        assert_eq!(super::path_of("bo/bumpe").len(), 3);
    }
}
