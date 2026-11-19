//! The shows and their episodes, as the web's `catalog.ts` lists them, as a directory of podcasts
//! would. Every show and host is made up; the episodes are about Tidevann's world, the sea and
//! plain geography. What a listener does with them (following, playing, starring, downloading) is
//! kept by the backend (`backend`).

use std::sync::OnceLock;

use serde::{Serialize, Serializer};
use sokkel_egui_media::{ARTWORK_HUES, ARTWORK_MOTIFS, ArtworkCover, ArtworkHue, ArtworkMotif};

/// The four hosts, who are also the voices (`script::VOICES`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Host {
    Ada,
    Grace,
    Linus,
    Margaret,
}

/// Every host, in the web's order.
pub const HOSTS: [Host; 4] = [Host::Ada, Host::Grace, Host::Linus, Host::Margaret];

impl Host {
    /// The host's name, as the web writes it.
    pub fn name(self) -> &'static str {
        match self {
            Host::Ada => "ada",
            Host::Grace => "grace",
            Host::Linus => "linus",
            Host::Margaret => "margaret",
        }
    }
}

/// What a show is about, for Discover's chips.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Sea,
    Geography,
    Nature,
    Weather,
    Work,
    History,
}

/// Every category with its label, in the web's order.
pub const CATEGORIES: [(Category, &str); 6] = [
    (Category::Sea, "Sea and coast"),
    (Category::Geography, "Geography"),
    (Category::Nature, "Nature"),
    (Category::Weather, "Weather"),
    (Category::Work, "Work"),
    (Category::History, "History"),
];

impl Category {
    /// The category's id, as the web writes it.
    pub fn id(self) -> &'static str {
        match self {
            Category::Sea => "sea",
            Category::Geography => "geography",
            Category::Nature => "nature",
            Category::Weather => "weather",
            Category::Work => "work",
            Category::History => "history",
        }
    }

    /// The category from its id.
    pub fn from_id(id: &str) -> Option<Category> {
        CATEGORIES.iter().map(|(one, _)| *one).find(|one| one.id() == id)
    }
}

/// A category's label.
pub fn category_label(category: Category) -> &'static str {
    CATEGORIES
        .iter()
        .find(|(one, _)| *one == category)
        .map_or(category.id(), |(_, label)| label)
}

/// Which words a show's hosts reach for when they talk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Talk {
    Coast,
    Weather,
    Ops,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Show {
    pub id: String,
    pub title: String,
    pub hosts: Vec<Host>,
    pub category: Category,
    /// One line, for a card or a search result.
    pub blurb: String,
    /// A paragraph, for the show's page.
    pub about: String,
    #[serde(serialize_with = "cover_serialize")]
    pub cover: ArtworkCover,
    /// The steps of the show's jingle, in semitones above its key.
    pub jingle: Vec<i32>,
    pub talk: Talk,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Episode {
    pub id: String,
    pub show: String,
    pub number: u32,
    pub title: String,
    pub summary: String,
    /// Where it was recorded, which the hosts mention.
    pub place: String,
    pub chapters: Vec<String>,
    /// How many days ago it came out; none for one still to come, which Check for new brings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<i64>,
}

/// A cover's motif as the web names it.
pub fn motif_key(motif: ArtworkMotif) -> &'static str {
    match motif {
        ArtworkMotif::Sun => "sun",
        ArtworkMotif::Rings => "rings",
        ArtworkMotif::Bands => "bands",
        ArtworkMotif::Dots => "dots",
        ArtworkMotif::Wave => "wave",
        ArtworkMotif::Peaks => "peaks",
    }
}

/// A cover's hue as the web names it.
pub fn hue_key(hue: ArtworkHue) -> &'static str {
    match hue {
        ArtworkHue::Blue => "blue",
        ArtworkHue::Teal => "teal",
        ArtworkHue::Green => "green",
        ArtworkHue::Jade => "jade",
        ArtworkHue::Violet => "violet",
        ArtworkHue::Pink => "pink",
        ArtworkHue::Amber => "amber",
        ArtworkHue::Coral => "coral",
    }
}

/// A cover from its motif's and hues' names.
pub fn cover(motif: &str, from: &str, to: &str) -> ArtworkCover {
    let hue = |key: &str| {
        ARTWORK_HUES
            .into_iter()
            .find(|one| hue_key(*one) == key)
            .unwrap_or(ArtworkHue::Blue)
    };
    ArtworkCover {
        motif: ARTWORK_MOTIFS
            .into_iter()
            .find(|one| motif_key(*one) == motif)
            .unwrap_or(ArtworkMotif::Sun),
        from: hue(from),
        to: hue(to),
    }
}

fn cover_serialize<S: Serializer>(cover: &ArtworkCover, serializer: S) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct Kept<'a> {
        motif: &'a str,
        from: &'a str,
        to: &'a str,
    }
    Kept {
        motif: motif_key(cover.motif),
        from: hue_key(cover.from),
        to: hue_key(cover.to),
    }
    .serialize(serializer)
}

/// Every show, in the web's order.
pub fn shows() -> &'static [Show] {
    static SHOWS: OnceLock<Vec<Show>> = OnceLock::new();
    SHOWS.get_or_init(|| {
        SHOW_ROWS
            .iter()
            .map(|row| Show {
                id: row.id.to_owned(),
                title: row.title.to_owned(),
                hosts: row.hosts.to_vec(),
                category: row.category,
                blurb: row.blurb.to_owned(),
                about: row.about.to_owned(),
                cover: cover(row.cover.0, row.cover.1, row.cover.2),
                jingle: row.jingle.to_vec(),
                talk: row.talk,
            })
            .collect()
    })
}

/// Every episode, in the web's order: show by show, oldest first.
pub fn episodes() -> &'static [Episode] {
    static EPISODES: OnceLock<Vec<Episode>> = OnceLock::new();
    EPISODES.get_or_init(|| {
        EPISODE_ROWS
            .iter()
            .map(|(show, number, title, summary, place, chapters, age)| Episode {
                id: format!("{show}-{number}"),
                show: (*show).to_owned(),
                number: *number,
                title: (*title).to_owned(),
                summary: (*summary).to_owned(),
                place: (*place).to_owned(),
                chapters: chapters.iter().map(|one| (*one).to_owned()).collect(),
                age: *age,
            })
            .collect()
    })
}

pub fn show_by_id(id: &str) -> Option<&'static Show> {
    shows().iter().find(|show| show.id == id)
}

pub fn episode_by_id(id: &str) -> Option<&'static Episode> {
    episodes().iter().find(|episode| episode.id == id)
}

/// A show's episodes, newest first.
pub fn episodes_of(show: &str) -> Vec<&'static Episode> {
    let mut found: Vec<&Episode> = episodes().iter().filter(|episode| episode.show == show).collect();
    found.sort_by_key(|episode| std::cmp::Reverse(episode.number));
    found
}

/// The hosts of a show written out: "ada", "ada and grace".
pub fn hosts_of(show: &Show) -> String {
    let names: Vec<&str> = show.hosts.iter().map(|host| host.name()).collect();
    match names.split_last() {
        None => String::new(),
        Some((last, [])) => (*last).to_owned(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    }
}

struct ShowRow {
    id: &'static str,
    title: &'static str,
    hosts: &'static [Host],
    category: Category,
    blurb: &'static str,
    about: &'static str,
    cover: (&'static str, &'static str, &'static str),
    jingle: &'static [i32],
    talk: Talk,
}

type EpisodeRow = (
    &'static str,
    u32,
    &'static str,
    &'static str,
    &'static str,
    &'static [&'static str],
    Option<i64>,
);

const SHOW_ROWS: [ShowRow; 10] = [
    ShowRow {
        id: "low-tide-hours",
        title: "Low Tide Hours",
        hosts: &[Host::Ada, Host::Grace],
        category: Category::Sea,
        blurb: "A slow walk along the shore at low water, and what the tide leaves behind.",
        about: "Ada and grace walk a different stretch of shore at every low tide: rock pools, sand bars, wrecks and harbors that empty twice a day. No hurry, and always back before the water.",
        cover: ("wave", "blue", "teal"),
        jingle: &[0, 4, 7, 12],
        talk: Talk::Coast,
    },
    ShowRow {
        id: "harbor-log",
        title: "The Harbor Log",
        hosts: &[Host::Grace, Host::Linus],
        category: Category::Work,
        blurb: "Boats in, boats out: the week at the harbor, told by the people who work it.",
        about: "A week at the harbor in a few minutes: who came in, what they brought, what broke and who fixed it. Grace keeps the log, linus asks the questions.",
        cover: ("bands", "teal", "jade"),
        jingle: &[0, 2, 4, 9],
        talk: Talk::Coast,
    },
    ShowRow {
        id: "field-notes-larkspur",
        title: "Field Notes from Larkspur",
        hosts: &[Host::Margaret],
        category: Category::Nature,
        blurb: "Walks across the larkspur hills, a notebook in one hand.",
        about: "Margaret walks the hills above larkspur in every season and reads from her notebook on the way: birds, plants, weather and the paths between the villages.",
        cover: ("peaks", "green", "jade"),
        jingle: &[0, 3, 7, 10],
        talk: Talk::Coast,
    },
    ShowRow {
        id: "juniper-weather",
        title: "Juniper Weather",
        hosts: &[Host::Linus],
        category: Category::Weather,
        blurb: "The week ahead, explained slowly, from the station on juniper ridge.",
        about: "Linus reads the instruments at the juniper ridge station and explains the week's weather without a single chart: where the wind comes from, why the fog sits in the valley, and when to bring a coat.",
        cover: ("sun", "amber", "coral"),
        jingle: &[0, 5, 7, 12],
        talk: Talk::Weather,
    },
    ShowRow {
        id: "tidevann-weekly",
        title: "Tidevann Weekly",
        hosts: &[Host::Ada, Host::Linus],
        category: Category::Work,
        blurb: "What changed in Tidevann this week: jobs, runs and the people who keep them going.",
        about: "Ada and linus go through the week in Tidevann: which jobs ran, which failed and why, what changed in the harbor, juniper and larkspur projects, and what is coming next.",
        cover: ("rings", "violet", "pink"),
        jingle: &[0, 7, 4, 12],
        talk: Talk::Ops,
    },
    ShowRow {
        id: "islands-one-by-one",
        title: "Islands, One by One",
        hosts: &[Host::Margaret, Host::Grace],
        category: Category::Geography,
        blurb: "One island an episode: how it was made, who lives there, and how to get there.",
        about: "Every episode is one island: how it was made, how many people live there and what they do, and the ferry, the bridge or the boat it takes to get there.",
        cover: ("dots", "blue", "jade"),
        jingle: &[0, 2, 7, 9],
        talk: Talk::Coast,
    },
    ShowRow {
        id: "rivers-and-deltas",
        title: "Rivers and Deltas",
        hosts: &[Host::Linus, Host::Margaret],
        category: Category::Geography,
        blurb: "Following the water from the mountains down to the sea.",
        about: "Linus and margaret follow one river an episode, from the snow where it starts to the delta where it ends: the valleys it cut, the towns on its banks and the floods they remember.",
        cover: ("wave", "jade", "blue"),
        jingle: &[0, 4, 9, 7],
        talk: Talk::Coast,
    },
    ShowRow {
        id: "lighthouse-stories",
        title: "Lighthouse Stories",
        hosts: &[Host::Grace],
        category: Category::History,
        blurb: "The keepers, the lamps and the long nights on the coast.",
        about: "Grace visits a lighthouse an episode and tells its story: who built it, who kept it, the storms it saw and what it does now that nobody lives there.",
        cover: ("sun", "violet", "blue"),
        jingle: &[0, 3, 5, 10],
        talk: Talk::Coast,
    },
    ShowRow {
        id: "salt-and-stone",
        title: "Salt and Stone",
        hosts: &[Host::Ada],
        category: Category::Nature,
        blurb: "Coastal rock, sand and the slow work of the sea on the land.",
        about: "Ada looks closely at the coast itself: the cliffs, the beaches, the dunes and the slow work the sea does on all of them, a grain of sand at a time.",
        cover: ("bands", "amber", "coral"),
        jingle: &[0, 5, 3, 8],
        talk: Talk::Coast,
    },
    ShowRow {
        id: "the-night-shift",
        title: "The Night Shift",
        hosts: &[Host::Linus, Host::Grace],
        category: Category::Work,
        blurb: "What runs while everyone sleeps: the nightly jobs and the people on call.",
        about: "The jobs that run at night and the people who answer when they fail: nightly-export, rebuild-index, the scheduler and the worker, told from the on-call rota.",
        cover: ("peaks", "violet", "pink"),
        jingle: &[0, 7, 10, 5],
        talk: Talk::Ops,
    },
];

const EPISODE_ROWS: [EpisodeRow; 41] = [
    (
        "low-tide-hours",
        1,
        "The harbor at low water",
        "Ada and grace walk out across the empty harbor and count what the tide left behind.",
        "the harbor",
        &[
            "The harbor at low water",
            "Rock pools on the north wall",
            "What the boats bring in",
        ],
        Some(40),
    ),
    (
        "low-tide-hours",
        2,
        "Sand bars of the long bay",
        "The long bay drains further than anywhere on the coast. We walk out to the farthest sand bar and back before the flood.",
        "the long bay",
        &["Out across the flats", "The farthest bar", "Racing the flood"],
        Some(26),
    ),
    (
        "low-tide-hours",
        3,
        "A wreck at the river mouth",
        "Twice a year the lowest tides uncover an old wreck at the river mouth. Grace has waited three years to see it.",
        "the river mouth",
        &[
            "Waiting for the lowest tide",
            "What is left of her",
            "Who she was",
            "Back to the dunes",
        ],
        Some(12),
    ),
    (
        "low-tide-hours",
        4,
        "The causeway to the island",
        "For two hours at low water you can walk to the island. Ada times it, grace brings the tide table.",
        "the causeway",
        &["Two hours each way", "Halfway across", "On the island"],
        Some(3),
    ),
    (
        "low-tide-hours",
        5,
        "Mussels on the breakwater",
        "The breakwater is covered in mussels at low water. We count them, badly, and ask why they grow there.",
        "the breakwater",
        &["On the breakwater", "Counting, badly", "Why there"],
        None,
    ),
    (
        "harbor-log",
        1,
        "A new crane on the quay",
        "The old crane came down this week and the new one went up. Linus asks who gets to drive it.",
        "the quay",
        &["Down comes the old crane", "Up goes the new one", "Who drives it"],
        Some(33),
    ),
    (
        "harbor-log",
        2,
        "The ferry that came in late",
        "The morning ferry came in three hours late. Grace finds out why, and who waited.",
        "the ferry dock",
        &["Three hours late", "Fog off the point", "Who waited"],
        Some(19),
    ),
    (
        "harbor-log",
        3,
        "Nets, ropes and the chandlery",
        "Inside the chandlery, where the boats get everything from rope to coffee.",
        "the chandlery",
        &["Inside the chandlery", "Rope by the meter", "Coffee for the crews"],
        Some(5),
    ),
    (
        "harbor-log",
        4,
        "Winter berths",
        "Which boats stay in for the winter, which leave, and how they all fit.",
        "the inner harbor",
        &["Who stays", "Who leaves", "How they all fit"],
        None,
    ),
    (
        "field-notes-larkspur",
        1,
        "The first frost on the hills",
        "The first frost of the year, the last of the late flowers, and a very cold notebook.",
        "the larkspur hills",
        &["Before sunrise", "The last flowers", "A cold notebook"],
        Some(30),
    ),
    (
        "field-notes-larkspur",
        2,
        "The path between two villages",
        "An old path between two villages on the ridge, walked end to end in an afternoon.",
        "the ridge path",
        &["The first village", "Along the ridge", "The second village"],
        Some(16),
    ),
    (
        "field-notes-larkspur",
        3,
        "Birds of the upper meadow",
        "Margaret sits still in the upper meadow for an hour and writes down every bird she hears.",
        "the upper meadow",
        &["Sitting still", "Every bird, one by one", "What the meadow needs"],
        Some(6),
    ),
    (
        "field-notes-larkspur",
        4,
        "The spring under the old oak",
        "There is a spring under the old oak that never runs dry. Margaret asks around about it.",
        "the old oak",
        &["The spring", "Asking around", "Never dry"],
        None,
    ),
    (
        "juniper-weather",
        1,
        "Why the fog sits in the valley",
        "The fog in the valley this week, and why it stays put on a still morning.",
        "juniper ridge",
        &["Still mornings", "Cold air runs downhill", "When it lifts"],
        Some(21),
    ),
    (
        "juniper-weather",
        2,
        "A week of westerly wind",
        "The wind has come from the west all week. Linus explains where it started and when it will turn.",
        "juniper ridge",
        &["Where it started", "Across the sea", "When it turns"],
        Some(14),
    ),
    (
        "juniper-weather",
        3,
        "The first snow on the ridge",
        "Snow on the ridge but rain in the town: one storm, two kinds of weather.",
        "juniper ridge",
        &["Snow up high", "Rain down low", "The week ahead"],
        Some(2),
    ),
    (
        "juniper-weather",
        4,
        "Reading the clouds",
        "Three kinds of cloud you can see from any window, and what each one says about tomorrow.",
        "juniper ridge",
        &["High and thin", "Low and gray", "Tall and bright"],
        None,
    ),
    (
        "tidevann-weekly",
        1,
        "Why sync-inventory failed on Tuesday",
        "Sync-inventory failed at the validate step on Tuesday night. Ada walks through the run, linus through the fix.",
        "the office",
        &["The failed run", "The validate step", "The fix"],
        Some(24),
    ),
    (
        "tidevann-weekly",
        2,
        "Faster nightly exports",
        "The nightly-export job got twice as fast this week. Linus explains what changed in the transform step.",
        "the office",
        &["Twice as fast", "The transform step", "What is next"],
        Some(10),
    ),
    (
        "tidevann-weekly",
        3,
        "A new scheduler for larkspur",
        "The larkspur project moved to the new scheduler. What moved, what broke, and what got better.",
        "the office",
        &[
            "Moving over",
            "What broke",
            "What got better",
            "Questions from the team",
        ],
        Some(1),
    ),
    (
        "tidevann-weekly",
        4,
        "The send-digest rewrite",
        "Send-digest gets a new notify step. Ada and linus argue about the wording of the email.",
        "the office",
        &["A new notify step", "The wording", "When it ships"],
        None,
    ),
    (
        "islands-one-by-one",
        1,
        "The island with one road",
        "One road, one shop, one ferry a day, and a hundred and forty people who would not live anywhere else.",
        "the northern island",
        &["One road", "One ferry a day", "Why they stay"],
        Some(28),
    ),
    (
        "islands-one-by-one",
        2,
        "An island made by a volcano",
        "Black sand, warm springs and a mountain that last woke up two hundred years ago.",
        "the volcanic island",
        &["Black sand", "Warm springs", "The sleeping mountain"],
        Some(11),
    ),
    (
        "islands-one-by-one",
        3,
        "The island you can walk to",
        "A tidal island joined to the land twice a day, and the people who plan their lives around it.",
        "the tidal island",
        &["Twice a day", "Planning around the tide", "Stuck on the wrong side"],
        Some(4),
    ),
    (
        "islands-one-by-one",
        4,
        "The smallest island with a school",
        "Six pupils, one teacher, and a classroom with a view of the sea.",
        "the smallest island",
        &["Six pupils", "One teacher", "A view of the sea"],
        None,
    ),
    (
        "rivers-and-deltas",
        1,
        "Where the river starts",
        "Up in the snowfields, where the river is a trickle you can step across.",
        "the snowfields",
        &["A trickle in the snow", "The first lake", "Down the valley"],
        Some(35),
    ),
    (
        "rivers-and-deltas",
        2,
        "The towns on the banks",
        "Three towns on the same river, each built where the water let them.",
        "the river valley",
        &["The bridge town", "The mill town", "The port town"],
        Some(18),
    ),
    (
        "rivers-and-deltas",
        3,
        "The delta and the sea",
        "Where the river slows down, splits into a hundred channels and finally meets the salt.",
        "the delta",
        &["A hundred channels", "Fresh water meets salt", "The birds of the delta"],
        Some(7),
    ),
    (
        "rivers-and-deltas",
        4,
        "The flood of the spring thaw",
        "Every spring the river rises. Margaret talks to the people who remember the biggest flood.",
        "the river valley",
        &["The thaw", "The biggest flood", "What changed after"],
        None,
    ),
    (
        "lighthouse-stories",
        1,
        "The keeper who stayed forty years",
        "One keeper, one lamp and forty years of nights on the point.",
        "the point",
        &["Forty years", "The lamp", "The last night"],
        Some(38),
    ),
    (
        "lighthouse-stories",
        2,
        "The light on the rock",
        "A lighthouse built on a bare rock out at sea, and how they got the stones there.",
        "the rock",
        &["A bare rock", "Getting the stones there", "The first light"],
        Some(15),
    ),
    (
        "lighthouse-stories",
        3,
        "When the lights went automatic",
        "The year the last keepers left, and what the lighthouses do without them.",
        "the coast",
        &["The last keepers", "A lamp on a timer", "Who looks after them now"],
        Some(8),
    ),
    (
        "lighthouse-stories",
        4,
        "Fog signals",
        "Before radios there were horns, bells and guns. Grace tries to find one that still works.",
        "the headland",
        &["Horns and bells", "One that still works", "A very loud morning"],
        None,
    ),
    (
        "salt-and-stone",
        1,
        "How a beach moves",
        "A beach is never still. Ada follows one grain of sand along the coast for a year.",
        "the south beach",
        &["Never still", "One grain of sand", "A year later"],
        Some(27),
    ),
    (
        "salt-and-stone",
        2,
        "The cliffs that fall",
        "The cliffs lose a little every winter. Ada visits the edge and the people who live near it.",
        "the cliffs",
        &["A little every winter", "At the edge", "Living near it"],
        Some(13),
    ),
    (
        "salt-and-stone",
        3,
        "Dunes that hold the sea back",
        "The dunes behind the beach, the grass that holds them together, and the storm that tested them.",
        "the dunes",
        &["The grass", "The storm", "Still standing"],
        Some(3),
    ),
    (
        "salt-and-stone",
        4,
        "Pebbles",
        "Why some beaches are sand and some are stones, and what the stones can tell you.",
        "the pebble beach",
        &["Sand or stones", "Reading a pebble", "The beach after a storm"],
        None,
    ),
    (
        "the-night-shift",
        1,
        "Paged at three in the morning",
        "Grace was paged at three in the morning when rebuild-index failed. What she did, step by step.",
        "the on-call desk",
        &["The page", "Step by step", "Back to sleep"],
        Some(20),
    ),
    (
        "the-night-shift",
        2,
        "The queue that never empties",
        "The worker queue grew all night and nobody knew why. Linus finds the import-orders job behind it.",
        "the on-call desk",
        &["A growing queue", "Finding import-orders", "Emptying it"],
        Some(9),
    ),
    (
        "the-night-shift",
        3,
        "A quiet night",
        "Nothing failed. Grace and linus talk about what a good night on call looks like.",
        "the on-call desk",
        &["Nothing failed", "What good looks like", "Handing over"],
        Some(2),
    ),
    (
        "the-night-shift",
        4,
        "Retries and backoff",
        "When a job fails at the upload step, how long should it wait before it tries again?",
        "the on-call desk",
        &["A failed upload", "Waiting", "Trying again"],
        None,
    ),
];
