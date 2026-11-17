//! Two years of Tidevann's teams, as the web's `seed.ts` makes them: the same way every time from
//! the day each signed up, who they are, where they work, and every change to their plans up to
//! now. Every number is the web's to the last bit: its random source, its hash and its rounding.

use std::collections::HashSet;

use crate::days::{DAY, HOUR, day_from, day_of, start_of, weekday};
use crate::model::{COUNTRIES, Change, ChangeKind, Country, Plan, REGIONS, Region, TeamRecord};

/// A random source that gives the same numbers for the same seed (mulberry32).
#[derive(Clone)]
pub struct Random(u32);

impl Random {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    /// The next number, at least 0 and below 1.
    pub fn draw(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6d2b_79f5);
        let mut value = self.0;
        value = (value ^ (value >> 15)).wrapping_mul(value | 1);
        value ^= value.wrapping_add((value ^ (value >> 7)).wrapping_mul(value | 61));
        f64::from(value ^ (value >> 14)) / 4_294_967_296.0
    }
}

/// The first number of the source `seed` starts.
pub fn first(seed: u32) -> f64 {
    Random::new(seed).draw()
}

/// A number made from several, spread across the whole range, for seeding. Each part is read as
/// JavaScript's bit operators read a number: its low 32 bits.
pub fn hash(parts: &[i64]) -> u32 {
    let mut value: u32 = 2_166_136_261;
    for part in parts {
        value ^= *part as u32;
        value = value.wrapping_mul(16_777_619);
        value ^= value >> 13;
    }
    value
}

/// `Math.round`: halves go up.
pub fn round(value: f64) -> f64 {
    (value + 0.5).floor()
}

/// One of `items`, each as likely as its weight.
pub fn pick<T: Copy>(random: &mut Random, items: &[T], weight: impl Fn(T) -> f64) -> T {
    let total = items.iter().fold(0.0, |sum, item| sum + weight(*item));
    let mut left = random.draw() * total;
    for item in items {
        left -= weight(*item);
        if left <= 0.0 {
            return *item;
        }
    }
    items[items.len() - 1]
}

fn between(random: &mut Random, low: i64, high: i64) -> i64 {
    low + (random.draw() * (high - low + 1) as f64).floor() as i64
}

/// How far back the history reaches, in days.
pub const HISTORY: i64 = 730;

/// The day sign-ups are counted from: about 22 a day then, a little more each day since.
fn anchor() -> i64 {
    day_from(2026, 1, 1)
}

fn region_weight(region: Region) -> f64 {
    match region {
        Region::Europe => 34.0,
        Region::NorthAmerica => 30.0,
        Region::AsiaPacific => 18.0,
        Region::SouthAmerica => 10.0,
        Region::MiddleEastAfrica => 8.0,
    }
}

const FIRST: [&str; 80] = [
    "Alder", "Amber", "Ash", "Aspen", "Basalt", "Birch", "Bramble", "Briar", "Cedar", "Cinder", "Clover", "Cobalt",
    "Copper", "Coral", "Cypress", "Dune", "Elm", "Ember", "Fern", "Fjord", "Flint", "Frost", "Garnet", "Glacier",
    "Granite", "Hazel", "Heath", "Heron", "Holly", "Iris", "Ivory", "Jasper", "Kelp", "Kestrel", "Laurel", "Linden",
    "Lumen", "Maple", "Marble", "Meadow", "Moss", "Nimbus", "North", "Oak", "Onyx", "Orchid", "Osprey", "Pebble",
    "Pine", "Plover", "Quarry", "Quartz", "Raven", "Reed", "Ridge", "Rowan", "Sable", "Saffron", "Sage", "Shale",
    "Slate", "Sorrel", "Spruce", "Summit", "Tansy", "Thistle", "Thorn", "Timber", "Tundra", "Umber", "Vale", "Willow",
    "Wren", "Yarrow", "Zephyr", "Beacon", "Comet", "Delta", "Echo", "Falcon",
];
const MIDDLE: [&str; 8] = ["", "wood", "field", "brook", "stone", "ford", "gate", "mere"];
const LAST: [&str; 50] = [
    "Labs",
    "Studio",
    "Works",
    "Systems",
    "Logistics",
    "Analytics",
    "Foods",
    "Health",
    "Design",
    "Freight",
    "Bakery",
    "Clinic",
    "Robotics",
    "Energy",
    "Media",
    "Ventures",
    "Partners",
    "Supply",
    "Outdoors",
    "Textiles",
    "Brewing",
    "Architects",
    "Dental",
    "Learning",
    "Travel",
    "Games",
    "Software",
    "Print",
    "Florists",
    "Legal",
    "Motors",
    "Optics",
    "Records",
    "Coffee",
    "Pharmacy",
    "Builders",
    "Finance",
    "Insurance",
    "Marine",
    "Farms",
    "Audio",
    "Furniture",
    "Fitness",
    "Books",
    "Interiors",
    "Engineering",
    "Consulting",
    "Aviation",
    "Bikes",
    "Gardens",
];

/// Every name a team can have, in an order the seed spreads them in.
pub const NAMES_IN_ALL: u64 = (FIRST.len() * MIDDLE.len() * LAST.len()) as u64;

fn name_at(index: u64) -> String {
    let index = index as usize;
    let first = FIRST[index % FIRST.len()];
    let middle = MIDDLE[(index / FIRST.len()) % MIDDLE.len()];
    let last = LAST[(index / (FIRST.len() * MIDDLE.len())) % LAST.len()];
    let joined = if first.to_lowercase().ends_with(middle) {
        first.to_owned()
    } else {
        format!("{first}{middle}")
    };
    format!("{joined} {last}")
}

/// Names given out so far; each team gets the first free one from where its seed points.
#[derive(Clone, Debug, Default)]
pub struct Names {
    taken: HashSet<String>,
}

impl Names {
    pub fn take(&mut self, seed: u64) -> String {
        for step in 0..NAMES_IN_ALL {
            let name = name_at((seed + step * 7919) % NAMES_IN_ALL);
            if !self.taken.contains(&name) {
                self.taken.insert(name.clone());
                return name;
            }
        }
        format!("Team {seed}")
    }

    pub fn keep(&mut self, name: &str) {
        self.taken.insert(name.to_owned());
    }
}

/// A country, by the sign-ups each region and country has.
pub fn place_of(random: &mut Random) -> &'static Country {
    let region = pick(random, &REGIONS, region_weight);
    let within: Vec<usize> = (0..COUNTRIES.len())
        .filter(|i| COUNTRIES[*i].region == region)
        .collect();
    let at = pick(random, &within, |i| COUNTRIES[i].weight);
    &COUNTRIES[at]
}

/// About how many teams sign up on a day: growing a little each day, quieter on weekends.
fn signups_on(day: i64, random: &mut Random) -> i64 {
    let weekend = if matches!(weekday(day), 0 | 6) { 0.55 } else { 1.0 };
    round(22.0 * 1.0011f64.powf((day - anchor()) as f64) * weekend * (0.8 + random.draw() * 0.4)) as i64
}

/// A moment in a day's working hours.
fn moment_in(day: i64, random: &mut Random) -> i64 {
    start_of(day) + round((7.0 + random.draw() * 14.0) * HOUR as f64) as i64
}

/// One team and its changes up to `now`, from its own seed alone.
fn team(day: i64, index: i64, names: &mut Names, now: i64) -> (TeamRecord, Vec<Change>) {
    let mut random = Random::new(hash(&[day, index, 7]));
    let id = format!("t{day}-{index}");
    let place = place_of(&mut random);
    let created = moment_in(day, &mut random);
    let free_seats = 1 + (random.draw().powf(2.0) * 5.0).floor() as i64;
    let project = (random.draw() < 0.75).then(|| day + (random.draw().powf(2.0) * 4.0).floor() as i64);
    let invited = match project {
        Some(project) if random.draw() < 0.55 => Some(project + (random.draw().powf(2.0) * 10.0).floor() as i64),
        _ => None,
    };
    let mut changes = vec![Change {
        id: format!("{id}-0"),
        team: id.clone(),
        kind: ChangeKind::Signup,
        at: created,
        plan: Plan::Free,
        seats: free_seats,
        by: None,
        live: false,
    }];
    let upgrade_chance = if invited.is_some() {
        0.3
    } else if project.is_some() {
        0.08
    } else {
        0.02
    };
    let mut dormant = None;
    if random.draw() < upgrade_chance {
        let from = invited.or(project).unwrap_or(day);
        let mut at = moment_in(from + between(&mut random, 2, 45), &mut random);
        let mut plan = if random.draw() < 0.78 {
            Plan::Team
        } else {
            Plan::Business
        };
        let mut seats = if plan == Plan::Team {
            between(&mut random, 3, 6) + (random.draw().powf(3.0) * 18.0).floor() as i64
        } else {
            between(&mut random, 8, 16) + (random.draw().powf(3.0) * 70.0).floor() as i64
        };
        let mut made = 1;
        let mut add = |changes: &mut Vec<Change>, at: i64, kind: ChangeKind, plan: Plan, seats: i64| {
            if at > now {
                return false;
            }
            changes.push(Change {
                id: format!("{id}-{made}"),
                team: id.clone(),
                kind,
                at,
                plan,
                seats,
                by: None,
                live: false,
            });
            made += 1;
            true
        };
        if add(&mut changes, at, ChangeKind::Upgrade, plan, seats) {
            loop {
                at += (28 + (random.draw() * 6.0).floor() as i64) * DAY;
                if at > now {
                    break;
                }
                let roll = random.draw();
                let churn = if plan == Plan::Team { 0.022 } else { 0.012 };
                if roll < churn {
                    add(&mut changes, at, ChangeKind::Churn, plan, 0);
                    break;
                }
                if roll < churn + 0.1 {
                    seats += if plan == Plan::Team {
                        between(&mut random, 1, 4)
                    } else {
                        between(&mut random, 2, 10)
                    };
                    add(&mut changes, at, ChangeKind::Seats, plan, seats);
                } else if roll < churn + 0.14 && seats > 3 {
                    seats -= between(&mut random, 1, 3.min(seats - 3));
                    add(&mut changes, at, ChangeKind::Seats, plan, seats);
                } else if roll < churn + 0.155 && plan == Plan::Team {
                    plan = Plan::Business;
                    seats = seats.max(8);
                    add(&mut changes, at, ChangeKind::Upgrade, plan, seats);
                } else if roll < churn + 0.16 && plan == Plan::Business {
                    plan = Plan::Team;
                    add(&mut changes, at, ChangeKind::Downgrade, plan, seats);
                }
            }
        }
    } else if random.draw() < 0.72 {
        dormant = Some(day + between(&mut random, 5, 120));
    }
    let record = TeamRecord {
        name: names.take(u64::from(hash(&[day, index, 11])) % NAMES_IN_ALL),
        id,
        country: place.name.to_owned(),
        region: place.region,
        owner: place.region.owner(),
        created,
        project,
        invited,
        dormant,
        live: false,
    };
    changes.retain(|change| change.at <= now);
    (record, changes)
}

/// Every team that signed up in the last two years, up to `now`, with their changes and the names
/// given out.
pub struct History {
    pub teams: Vec<TeamRecord>,
    pub changes: Vec<Change>,
    pub names: Names,
}

pub fn history(now: i64) -> History {
    let today = day_of(now);
    let mut names = Names::default();
    let mut teams = Vec::new();
    let mut changes = Vec::new();
    for day in today - HISTORY + 1..=today {
        let count = signups_on(day, &mut Random::new(hash(&[day, 3])));
        for index in 0..count {
            let (record, made) = team(day, index, &mut names, now);
            if record.created > now {
                continue;
            }
            teams.push(record);
            changes.extend(made);
        }
    }
    History { teams, changes, names }
}
