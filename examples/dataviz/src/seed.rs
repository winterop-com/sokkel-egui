//! The demo's first data, as the web's `seed.ts` makes it: four invented datasets in Tidevann's
//! world and plain geography, the charts saved from them and two boards. Everything is made the
//! same way every time: the figures come from a hash of where they stand, never from the clock or
//! chance.

use crate::js;
use crate::model::{
    Aggregate, Board, BoardItem, Column, ColumnType, DataRow, Dataset, DatasetSource, DateBin, PanelSpan, SavedChart,
    SortOrder, ViewKind, ViewSpec,
};

pub const SEEDED_AT: &str = "2025-11-03T09:00:00.000Z";

/// A number in [0, 1) that is always the same for the same text: FNV-1a over its UTF-16 units.
pub fn noise(text: &str) -> f64 {
    let mut hash: u32 = 2_166_136_261;
    for unit in text.encode_utf16() {
        hash = (hash ^ u32::from(unit)).wrapping_mul(16_777_619);
    }
    f64::from(hash % 10_000) / 10_000.0
}

fn pad(value: i64) -> String {
    format!("{value:02}")
}

/// The day `offset` days after `start`, as `YYYY-MM-DD`.
pub fn day_after(start: &str, offset: i64) -> String {
    let parts: Vec<i64> = start.split('-').map(|part| part.parse().unwrap_or(0)).collect();
    let days = js::date_utc(parts[0], parts[1] - 1, parts[2] + offset);
    let (year, month, day) = js::civil_from_days(days);
    format!("{year}-{}-{}", pad(month), pad(day))
}

fn dataset(id: &str, name: &str, description: &str, columns: Vec<Column>, values: Vec<Vec<String>>) -> Dataset {
    let rows = values
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let mut out = DataRow::new(format!("{id}-r{}", index + 1));
            for (at, column) in columns.iter().enumerate() {
                out.set(&column.id, row.get(at).cloned().unwrap_or_default());
            }
            out
        })
        .collect();
    Dataset {
        id: id.to_owned(),
        name: name.to_owned(),
        description: description.to_owned(),
        source: DatasetSource::Seed,
        columns,
        rows,
        updated_at: SEEDED_AT.to_owned(),
    }
}

pub const REGION_NAMES: [&str; 15] = [
    "Bomi",
    "Bong",
    "Gbapolu",
    "Grand Bassa",
    "Grand Cape Mount",
    "Grand Gedeh",
    "Grand Kru",
    "Lofa",
    "Margibi",
    "Maryland",
    "Montserrado",
    "Nimba",
    "River Cess",
    "River Gee",
    "Sinoe",
];

/// Each month's share of a wet year, peaking in August.
const SEASON: [f64; 12] = [0.08, 0.1, 0.22, 0.45, 0.8, 1.25, 1.75, 1.95, 1.6, 1.1, 0.5, 0.18];

fn num(value: f64) -> String {
    js::number_to_string(value)
}

fn rainfall() -> Dataset {
    let columns = vec![
        Column::new("c1", "Month", ColumnType::Date),
        Column::new("c2", "Region", ColumnType::Region),
        Column::new("c3", "Rainfall", ColumnType::Number),
        Column::new("c4", "Rain days", ColumnType::Number),
    ];
    let mut values = Vec::new();
    for (at, region) in REGION_NAMES.iter().enumerate() {
        let wetness = 160.0 + at as f64 * 9.0 + noise(region) * 70.0;
        for (month, share) in SEASON.iter().enumerate() {
            let wobble = 0.85 + noise(&format!("{region}-{month}")) * 0.3;
            let rain = js::round(wetness * share * wobble);
            let days = js::round(2.0 + share * 12.0 * wobble).min(30.0);
            values.push(vec![
                format!("2025-{}-01", pad(month as i64 + 1)),
                (*region).to_owned(),
                num(rain),
                num(days),
            ]);
        }
    }
    dataset(
        "ds-rainfall",
        "Rainfall by region",
        "Rain at each region’s stations in 2025, a row a month: millimeters and the days it rained.",
        columns,
        values,
    )
}

const HARBORS: [&str; 4] = ["Bergen", "Lisbon", "Cork", "Tallinn"];
const VESSELS: [&str; 3] = ["cargo", "ferry", "fishing"];

fn arrivals() -> Dataset {
    let columns = vec![
        Column::new("c1", "Date", ColumnType::Date),
        Column::new("c2", "Harbor", ColumnType::Text),
        Column::new("c3", "Vessel", ColumnType::Text),
        Column::new("c4", "Arrivals", ColumnType::Number),
        Column::new("c5", "Tonnage", ColumnType::Number),
    ];
    let size = |harbor: &str| match harbor {
        "Bergen" => 1.1,
        "Lisbon" => 1.5,
        "Cork" => 0.7,
        _ => 0.9,
    };
    let base = |vessel: &str| match vessel {
        "cargo" => 6.0,
        "ferry" => 9.0,
        _ => 4.0,
    };
    let tons = |vessel: &str| match vessel {
        "cargo" => 18.0,
        "ferry" => 4.0,
        _ => 0.6,
    };
    let mut values = Vec::new();
    for day in 0..90 {
        let date = day_after("2025-01-01", day);
        let weekday = js::weekday(js::day_number(&date).unwrap_or(0));
        let weekend = weekday == 0 || weekday == 6;
        for harbor in HARBORS {
            for vessel in VESSELS {
                let spring = 1.0 + day as f64 / 180.0;
                let busy = if vessel == "ferry" && weekend {
                    1.4
                } else if vessel == "cargo" && weekend {
                    0.55
                } else {
                    1.0
                };
                let count = js::round(
                    base(vessel)
                        * size(harbor)
                        * spring
                        * busy
                        * (0.6 + noise(&format!("{date}{harbor}{vessel}")) * 0.8),
                )
                .max(0.0);
                let tonnage =
                    js::round(count * tons(vessel) * (0.8 + noise(&format!("{harbor}{vessel}{date}t")) * 0.4) * 10.0)
                        / 10.0;
                values.push(vec![
                    date.clone(),
                    harbor.to_owned(),
                    vessel.to_owned(),
                    num(count),
                    num(tonnage),
                ]);
            }
        }
    }
    dataset(
        "ds-arrivals",
        "Harbor arrivals",
        "Vessels arriving at four harbors each day in the first quarter of 2025, and the tonnage they carried in thousands of metric tons.",
        columns,
        values,
    )
}

const JOBS: [&str; 5] = [
    "nightly-export",
    "sync-inventory",
    "rebuild-index",
    "send-digest",
    "import-orders",
];
const STARTERS: [&str; 6] = ["schedule", "schedule", "ada", "grace", "linus", "margaret"];

fn project_of(job: &str) -> &'static str {
    match job {
        "nightly-export" | "sync-inventory" => "harbor",
        "rebuild-index" | "import-orders" => "juniper",
        _ => "larkspur",
    }
}

fn runs() -> Dataset {
    let columns = vec![
        Column::new("c1", "Day", ColumnType::Date),
        Column::new("c2", "Job", ColumnType::Text),
        Column::new("c3", "Project", ColumnType::Text),
        Column::new("c4", "State", ColumnType::Text),
        Column::new("c5", "Duration", ColumnType::Number),
        Column::new("c6", "Rows", ColumnType::Number),
        Column::new("c7", "Started by", ColumnType::Text),
    ];
    let typical = |job: &str| match job {
        "nightly-export" => 640.0,
        "sync-inventory" => 210.0,
        "rebuild-index" => 1180.0,
        "send-digest" => 45.0,
        _ => 380.0,
    };
    let mut values = Vec::new();
    for day in 0..31 {
        let date = day_after("2025-10-01", day);
        for job in JOBS {
            let times = match job {
                "sync-inventory" => 3,
                "import-orders" => 2,
                _ => 1,
            };
            for run in 0..times {
                let key = format!("{date}{job}{run}");
                let roll = noise(&key);
                let state = if roll < 0.08 {
                    "failed"
                } else if roll < 0.11 {
                    "cancelled"
                } else if roll < 0.14 {
                    "skipped"
                } else {
                    "succeeded"
                };
                let duration = if state == "skipped" {
                    0.0
                } else {
                    js::round(
                        typical(job)
                            * (0.7 + noise(&format!("{key}d")) * 0.7)
                            * if state == "failed" { 0.4 } else { 1.0 },
                    )
                };
                let rows = if state == "succeeded" {
                    js::round(duration * (20.0 + noise(&format!("{key}r")) * 60.0))
                } else {
                    0.0
                };
                let by = STARTERS[(noise(&format!("{key}b")) * STARTERS.len() as f64).floor() as usize];
                values.push(vec![
                    date.clone(),
                    job.to_owned(),
                    project_of(job).to_owned(),
                    state.to_owned(),
                    num(duration),
                    num(rows),
                    by.to_owned(),
                ]);
            }
        }
    }
    dataset(
        "ds-runs",
        "Runs by job",
        "Every run of Tidevann’s jobs in October 2025: its state, how long it took in seconds and the rows it moved.",
        columns,
        values,
    )
}

const CITIES: [(&str, &str, &str, f64, f64); 24] = [
    ("Lagos", "Nigeria", "Africa", 15.4, 1171.0),
    ("Nairobi", "Kenya", "Africa", 4.4, 696.0),
    ("Accra", "Ghana", "Africa", 2.6, 225.0),
    ("Dakar", "Senegal", "Africa", 3.3, 547.0),
    ("Lusaka", "Zambia", "Africa", 3.0, 418.0),
    ("Hanoi", "Vietnam", "Asia", 8.4, 3359.0),
    ("Kathmandu", "Nepal", "Asia", 1.6, 50.0),
    ("Ulaanbaatar", "Mongolia", "Asia", 1.7, 4704.0),
    ("Phnom Penh", "Cambodia", "Asia", 2.3, 679.0),
    ("Vientiane", "Laos", "Asia", 1.0, 3920.0),
    ("Oslo", "Norway", "Europe", 0.7, 454.0),
    ("Lisbon", "Portugal", "Europe", 0.55, 100.0),
    ("Reykjavik", "Iceland", "Europe", 0.14, 274.0),
    ("Tallinn", "Estonia", "Europe", 0.45, 159.0),
    ("Vienna", "Austria", "Europe", 2.0, 415.0),
    ("Dublin", "Ireland", "Europe", 0.6, 118.0),
    ("Lima", "Peru", "South America", 10.9, 2672.0),
    ("Santiago", "Chile", "South America", 6.9, 641.0),
    ("Montevideo", "Uruguay", "South America", 1.4, 201.0),
    ("La Paz", "Bolivia", "South America", 0.8, 472.0),
    ("Quito", "Ecuador", "South America", 2.0, 372.0),
    ("Suva", "Fiji", "Oceania", 0.09, 2048.0),
    ("Apia", "Samoa", "Oceania", 0.04, 60.0),
    ("Nuku’alofa", "Tonga", "Oceania", 0.03, 11.0),
];

fn cities() -> Dataset {
    let columns = vec![
        Column::new("c1", "City", ColumnType::Text),
        Column::new("c2", "Country", ColumnType::Text),
        Column::new("c3", "Continent", ColumnType::Text),
        Column::new("c4", "Population", ColumnType::Number),
        Column::new("c5", "Area", ColumnType::Number),
    ];
    let values = CITIES
        .iter()
        .map(|(city, country, continent, millions, area)| {
            vec![
                (*city).to_owned(),
                (*country).to_owned(),
                (*continent).to_owned(),
                num(js::round(millions * 1_000_000.0)),
                num(*area),
            ]
        })
        .collect();
    dataset(
        "ds-cities",
        "Population by city",
        "People living in two dozen cities, and each city’s area in square kilometers. Rounded figures, for drawing rather than citing.",
        columns,
        values,
    )
}

pub fn seed_datasets() -> Vec<Dataset> {
    vec![rainfall(), arrivals(), runs(), cities()]
}

/// A seeded chart: the base spec with `patch` laid over it.
fn chart(id: &str, name: &str, dataset_id: &str, patch: impl FnOnce(&mut ViewSpec)) -> SavedChart {
    let mut spec = ViewSpec::base();
    patch(&mut spec);
    SavedChart {
        id: id.to_owned(),
        name: name.to_owned(),
        dataset_id: dataset_id.to_owned(),
        spec,
        updated_at: SEEDED_AT.to_owned(),
    }
}

fn id(text: &str) -> Option<String> {
    Some(text.to_owned())
}

pub fn seed_charts() -> Vec<SavedChart> {
    vec![
        chart("ch-rain-map", "Rainfall across the regions", "ds-rainfall", |s| {
            s.view = ViewKind::Map;
            s.x = id("c2");
            s.y = id("c3");
        }),
        chart("ch-rain-season", "Rainfall through the year", "ds-rainfall", |s| {
            s.view = ViewKind::Area;
            s.x = id("c1");
            s.y = id("c3");
            s.bin = DateBin::Month;
        }),
        chart("ch-rain-grid", "Rain days by region and month", "ds-rainfall", |s| {
            s.view = ViewKind::Heatmap;
            s.x = id("c1");
            s.series = id("c2");
            s.y = id("c4");
            s.aggregate = Aggregate::Avg;
        }),
        chart("ch-rain-total", "Rain in the year", "ds-rainfall", |s| {
            s.view = ViewKind::Stat;
            s.x = id("c1");
            s.y = id("c3");
            s.aggregate = Aggregate::Sum;
        }),
        chart("ch-runs-state", "Runs by job and state", "ds-runs", |s| {
            s.view = ViewKind::Bar;
            s.x = id("c2");
            s.series = id("c4");
            s.stack = true;
            s.sort = SortOrder::Desc;
        }),
        chart("ch-runs-duration", "Duration through October", "ds-runs", |s| {
            s.view = ViewKind::Line;
            s.x = id("c1");
            s.y = id("c5");
            s.series = id("c3");
            s.aggregate = Aggregate::Avg;
            s.bin = DateBin::Day;
        }),
        chart("ch-runs-share", "Runs by state", "ds-runs", |s| {
            s.view = ViewKind::Pie;
            s.x = id("c4");
        }),
        chart("ch-runs-count", "Rows moved this month", "ds-runs", |s| {
            s.view = ViewKind::Stat;
            s.x = id("c1");
            s.y = id("c6");
            s.bin = DateBin::Day;
        }),
        chart("ch-runs-rows", "Rows moved by project", "ds-runs", |s| {
            s.view = ViewKind::Table;
            s.x = id("c3");
            s.y = id("c6");
            s.sort = SortOrder::Desc;
        }),
        chart("ch-arrivals-week", "Arrivals by week", "ds-arrivals", |s| {
            s.view = ViewKind::Bar;
            s.x = id("c1");
            s.y = id("c4");
            s.series = id("c2");
            s.bin = DateBin::Week;
            s.stack = true;
            s.palette = "cool".to_owned();
        }),
        chart("ch-cities", "Population against area", "ds-cities", |s| {
            s.view = ViewKind::Scatter;
            s.x = id("c5");
            s.y = id("c4");
            s.color = id("c3");
        }),
    ]
}

fn item(id: &str, chart_id: &str, span: PanelSpan) -> BoardItem {
    BoardItem {
        id: id.to_owned(),
        chart_id: chart_id.to_owned(),
        span,
    }
}

pub fn seed_boards() -> Vec<Board> {
    vec![
        Board {
            id: "bd-weather".to_owned(),
            name: "Weather".to_owned(),
            description: "Where and when the rain fell in 2025.".to_owned(),
            items: vec![
                item("it-1", "ch-rain-total", PanelSpan::Third),
                item("it-2", "ch-rain-season", PanelSpan::TwoThirds),
                item("it-3", "ch-rain-map", PanelSpan::Half),
                item("it-4", "ch-rain-grid", PanelSpan::Half),
            ],
            filters: Vec::new(),
            updated_at: SEEDED_AT.to_owned(),
        },
        Board {
            id: "bd-runs".to_owned(),
            name: "Runs in October".to_owned(),
            description: "How Tidevann’s jobs ran through the month.".to_owned(),
            items: vec![
                item("it-5", "ch-runs-count", PanelSpan::Third),
                item("it-6", "ch-runs-share", PanelSpan::Third),
                item("it-7", "ch-runs-rows", PanelSpan::Third),
                item("it-8", "ch-runs-state", PanelSpan::Half),
                item("it-9", "ch-runs-duration", PanelSpan::Half),
            ],
            filters: Vec::new(),
            updated_at: SEEDED_AT.to_owned(),
        },
    ]
}
