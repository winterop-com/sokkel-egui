//! What the site holds, as the web's `catalog.ts` keeps it: Tidevann's channels and their videos.
//! Each video names the scene its clip is painted from (`tidevann_shared::clips`), so every video
//! plays a picture and a sound of its own.

use serde::{Deserialize, Serialize, Serializer};
use tidevann_shared::clips::SceneKind;

/// The people of the Tidevann workspace: the viewer and the channels' owners.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Person {
    Ada,
    Grace,
    Linus,
    Margaret,
}

impl Person {
    pub const ALL: [Person; 4] = [Person::Ada, Person::Grace, Person::Linus, Person::Margaret];

    pub fn name(self) -> &'static str {
        match self {
            Person::Ada => "ada",
            Person::Grace => "grace",
            Person::Linus => "linus",
            Person::Margaret => "margaret",
        }
    }

    pub fn from_name(name: &str) -> Option<Person> {
        Person::ALL.into_iter().find(|one| one.name() == name)
    }

    /// The person's color, a kind color's name (`PEOPLE_COLORS`).
    pub fn color(self) -> &'static str {
        match self {
            Person::Ada => "blue",
            Person::Grace => "violet",
            Person::Linus => "green",
            Person::Margaret => "pink",
        }
    }
}

/// Who watches: the account in the rail's foot.
pub const VIEWER: Person = Person::Ada;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Channel {
    pub id: &'static str,
    pub name: &'static str,
    pub handle: &'static str,
    /// Who runs it, marked as the creator in its comments.
    pub owner: Person,
    /// A kind color's name.
    pub color: &'static str,
    /// How many followed it before the viewer did anything.
    pub followers: i64,
    pub about: &'static str,
    /// Where it films, by plain geography.
    pub region: &'static str,
    /// Days before the demo began that it was started.
    pub started: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Topic {
    Coasts,
    Harbors,
    Rail,
    Mountains,
    Night,
    Tides,
    Islands,
    Weather,
    Countryside,
}

/// The topics and their names, in the web's order.
pub const TOPICS: [(Topic, &str); 9] = [
    (Topic::Coasts, "Coasts"),
    (Topic::Harbors, "Harbors"),
    (Topic::Rail, "Rail"),
    (Topic::Mountains, "Mountains"),
    (Topic::Night, "Night sky"),
    (Topic::Tides, "Tides"),
    (Topic::Islands, "Islands"),
    (Topic::Weather, "Weather"),
    (Topic::Countryside, "Countryside"),
];

impl Topic {
    pub fn id(self) -> &'static str {
        match self {
            Topic::Coasts => "coasts",
            Topic::Harbors => "harbors",
            Topic::Rail => "rail",
            Topic::Mountains => "mountains",
            Topic::Night => "night",
            Topic::Tides => "tides",
            Topic::Islands => "islands",
            Topic::Weather => "weather",
            Topic::Countryside => "countryside",
        }
    }

    pub fn from_id(id: &str) -> Option<Topic> {
        TOPICS.iter().map(|(topic, _)| *topic).find(|one| one.id() == id)
    }

    /// Its name, as `TOPICS` writes it.
    pub fn label(self) -> &'static str {
        TOPICS
            .iter()
            .find(|(one, _)| *one == self)
            .map_or("", |(_, label)| label)
    }
}

fn scene_name<S: Serializer>(scene: &SceneKind, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(scene.name())
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Video {
    pub id: &'static str,
    pub title: &'static str,
    pub channel: &'static str,
    pub topic: Topic,
    #[serde(serialize_with = "scene_name")]
    pub scene: SceneKind,
    /// The clip's two hues, in degrees.
    pub hue: f64,
    pub accent: f64,
    /// Its length, in seconds.
    pub duration: f64,
    /// Days before the demo began that it was published.
    pub days: f64,
    pub views: i64,
    pub likes: i64,
    pub description: &'static str,
    /// What is said over the picture, spread evenly over its length.
    pub captions: &'static [&'static str],
}

pub const CHANNELS: [Channel; 6] = [
    Channel {
        id: "harbor",
        name: "Harbor Workshop",
        handle: "@harborworkshop",
        owner: Person::Grace,
        color: "teal",
        followers: 48_200,
        about: "Boats, quays and the people who keep them working, filmed in the harbor towns of the northern coast. A new video most weeks, usually at first light.",
        region: "The northern coast",
        started: 2210,
    },
    Channel {
        id: "juniper",
        name: "Juniper Field Notes",
        handle: "@juniperfieldnotes",
        owner: Person::Linus,
        color: "green",
        followers: 9600,
        about: "Walks through forests, fields and mountain valleys, with notes on what grows there and how the weather shapes it. Filmed on foot, with the sound of the place.",
        region: "Mountain valleys of the north",
        started: 1340,
    },
    Channel {
        id: "larkspur",
        name: "Larkspur Rail",
        handle: "@larkspurrail",
        owner: Person::Margaret,
        color: "violet",
        followers: 127_000,
        about: "Train journeys across continents, filmed from the window seat: night trains, valley lines and the long crossings of high deserts.",
        region: "Across three continents",
        started: 3105,
    },
    Channel {
        id: "tides",
        name: "Tidevann Tides",
        handle: "@tidevann",
        owner: Person::Margaret,
        color: "blue",
        followers: 312_000,
        about: "How the sea rises and falls: tide tables read hour by hour, spring tides at the harbor wall and the reasons behind them, from the people who make Tidevann.",
        region: "Coasts everywhere",
        started: 2890,
    },
    Channel {
        id: "atlas",
        name: "Coastline Atlas",
        handle: "@coastlineatlas",
        owner: Person::Linus,
        color: "pink",
        followers: 21_400,
        about: "Coasts, islands and the cities on them, one place at a time: where they are, how people reach them and what the water does there.",
        region: "Islands and coasts of the southern sea",
        started: 980,
    },
    Channel {
        id: "night",
        name: "Night Watch",
        handle: "@nightwatchsky",
        owner: Person::Grace,
        color: "jade",
        followers: 64_800,
        about: "The sky after dark from the far north: northern lights, lighthouses and the stars over still water. Long nights, cut short.",
        region: "The far north",
        started: 1720,
    },
];

pub const VIDEOS: [Video; 20] = [
    Video {
        id: "first-light",
        title: "First light over the northern coast",
        channel: "atlas",
        topic: Topic::Coasts,
        scene: SceneKind::Sea,
        hue: 222.0,
        accent: 28.0,
        duration: 44.0,
        days: 3.0,
        views: 52_400,
        likes: 4100,
        description: "Four mornings on one headland of the northern coast, cut into one: the first light on the water, the sun lifting off the horizon and a single boat crossing the bay. Filmed from the same spot each day, with nothing added but the sound of the place. The next part follows the boats back in, and the one after that the harbor as it wakes.",
        captions: &[
            "The sea before sunrise, still and dark.",
            "Light reaches the horizon first.",
            "A boat leaves the bay, heading east.",
            "The sun clears the water.",
            "The path of light reaches the shore.",
            "By now the harbor behind us is awake.",
        ],
    },
    Video {
        id: "harbor-wakes",
        title: "How a harbor wakes up",
        channel: "harbor",
        topic: Topic::Harbors,
        scene: SceneKind::Sea,
        hue: 198.0,
        accent: 12.0,
        duration: 38.0,
        days: 9.0,
        views: 31_200,
        likes: 2380,
        description: "From the first engine to the last boat out: an hour on the quay of a small harbor town, shortened to under a minute. The boats leave in the same order every morning, and nobody writes it down.",
        captions: &[
            "The quay at half past five.",
            "The first engine starts across the water.",
            "Boats leave in the order they came in.",
            "The light turns the water gold.",
            "The last boat clears the breakwater.",
        ],
    },
    Video {
        id: "western-cape",
        title: "Lighthouses of the western cape",
        channel: "night",
        topic: Topic::Coasts,
        scene: SceneKind::Lighthouse,
        hue: 228.0,
        accent: 45.0,
        duration: 40.0,
        days: 21.0,
        views: 19_000,
        likes: 1640,
        description: "The western cape has four lighthouses within sight of each other, and each turns at its own pace so a ship can tell them apart. This is the oldest, filmed through one clear night.",
        captions: &[
            "The lamp is lit an hour after sunset.",
            "One turn every six seconds.",
            "Ships read the timing, not the color.",
            "The moon rises over the headland.",
            "The beam reaches twenty nautical miles.",
            "At dawn the lamp is put out by hand.",
        ],
    },
    Video {
        id: "night-train",
        title: "Crossing the high desert by night train",
        channel: "larkspur",
        topic: Topic::Rail,
        scene: SceneKind::Train,
        hue: 262.0,
        accent: 22.0,
        duration: 46.0,
        days: 2.0,
        views: 8100,
        likes: 912,
        description: "The overnight train crosses the high desert in eleven hours. We filmed the last hour before dark from the dining car, as the hills turned violet and the poles went by.",
        captions: &[
            "The dining car, an hour before dark.",
            "The hills go violet first.",
            "Every pole is fifty meters from the last.",
            "The train leans into the long curve.",
            "Somewhere ahead is the border.",
            "The lights come on in the carriages.",
        ],
    },
    Video {
        id: "valley-line",
        title: "The valley line in autumn",
        channel: "larkspur",
        topic: Topic::Rail,
        scene: SceneKind::Train,
        hue: 28.0,
        accent: 12.0,
        duration: 34.0,
        days: 30.0,
        views: 44_900,
        likes: 3020,
        description: "A branch line that follows the river up the valley, one train an hour. In autumn the whole valley turns copper and the train runs slower, to give the passengers time to look.",
        captions: &[
            "The valley line leaves the coast at noon.",
            "The river runs beside the track the whole way.",
            "Copper hills, and one train an hour.",
            "The last stop is a village of forty houses.",
        ],
    },
    Video {
        id: "snow-pass",
        title: "A mountain pass before the snow closes it",
        channel: "juniper",
        topic: Topic::Mountains,
        scene: SceneKind::Peaks,
        hue: 214.0,
        accent: 330.0,
        duration: 42.0,
        days: 240.0,
        views: 88_000,
        likes: 6700,
        description: "The pass closes on the first heavy snow and opens again in late spring. We crossed it on the last open day, as the first flakes fell over the ridges.",
        captions: &[
            "The road up the pass, the last open day.",
            "The first snow, at two thousand meters.",
            "Three ridges, one behind another.",
            "The light goes pink in the late afternoon.",
            "Tomorrow the gate at the bottom is shut.",
        ],
    },
    Video {
        id: "aurora-fjord",
        title: "Northern lights over a still fjord",
        channel: "night",
        topic: Topic::Night,
        scene: SceneKind::Aurora,
        hue: 226.0,
        accent: 140.0,
        duration: 48.0,
        days: 5.0,
        views: 140_000,
        likes: 12_400,
        description: "A windless night at the head of a fjord, the water still enough to hold the whole sky. The lights came at eleven and stayed until two; this is the best twenty minutes, sped up.",
        captions: &[
            "Eleven at night, no wind at all.",
            "The first green band, low in the north.",
            "It reaches across the fjord.",
            "The water holds every star.",
            "A second curtain, higher and fainter.",
            "By two in the morning it is gone.",
        ],
    },
    Video {
        id: "aurora-night",
        title: "A whole night of northern lights in under a minute",
        channel: "night",
        topic: Topic::Night,
        scene: SceneKind::Aurora,
        hue: 248.0,
        accent: 290.0,
        duration: 52.0,
        days: 60.0,
        views: 506_000,
        likes: 31_200,
        description: "Eight hours of sky from one camera on the shore, from dusk to the first light. The violet at the top of the curtains only shows on the strongest nights.",
        captions: &[
            "Dusk, and the camera starts.",
            "Violet at the top of the curtains.",
            "The strongest hour, just after midnight.",
            "The lights fade as the moon rises.",
            "First light, eight hours later.",
        ],
    },
    Video {
        id: "city-rain",
        title: "Rain on a harbor city at night",
        channel: "atlas",
        topic: Topic::Weather,
        scene: SceneKind::Rain,
        hue: 222.0,
        accent: 38.0,
        duration: 36.0,
        days: 14.0,
        views: 27_300,
        likes: 2150,
        description: "The harbor city gets two hundred days of rain a year, most of it at night. One evening of it from a window above the old quay, the windows across the water going on and off.",
        captions: &[
            "Rain from the west, as usual.",
            "The office lights go out one floor at a time.",
            "Puddles hold the street lamps.",
            "It rains here two hundred days a year.",
        ],
    },
    Video {
        id: "tide-table",
        title: "Reading a tide table in one day",
        channel: "tides",
        topic: Topic::Tides,
        scene: SceneKind::Tide,
        hue: 214.0,
        accent: 192.0,
        duration: 40.0,
        days: 1.0,
        views: 3900,
        likes: 410,
        description: "A whole day of water at one harbor, drawn as it happens: two high waters, two low waters and the hours between. By the end you can read any tide table at a glance.",
        captions: &[
            "Midnight, and the water is falling.",
            "Low water, just after three.",
            "The flood runs fastest in the middle hours.",
            "High water, a little after nine.",
            "Twelve hours and twenty-five minutes later, again.",
            "That is why the times move every day.",
        ],
    },
    Video {
        id: "spring-tide",
        title: "Spring tide at the harbor wall",
        channel: "tides",
        topic: Topic::Tides,
        scene: SceneKind::Tide,
        hue: 200.0,
        accent: 160.0,
        duration: 32.0,
        days: 45.0,
        views: 61_000,
        likes: 4870,
        description: "Twice a month the sun and the moon pull together, and the water climbs higher up the harbor wall than on any other day. This one reached four meters.",
        captions: &[
            "A new moon, and a spring tide.",
            "The water climbs the wall fast.",
            "Four meters at the top.",
            "The steps at the end of the quay disappear.",
        ],
    },
    Video {
        id: "island-ferry",
        title: "Island hopping by ferry in the southern sea",
        channel: "atlas",
        topic: Topic::Islands,
        scene: SceneKind::Islands,
        hue: 196.0,
        accent: 48.0,
        duration: 44.0,
        days: 75.0,
        views: 1_200_000,
        likes: 58_000,
        description: "Six islands, one ferry and a day to see them all. The route goes round in a loop, and the ferry waits twenty minutes at each quay, long enough for a coffee and not much else.",
        captions: &[
            "The first island, at seven.",
            "Twenty minutes at each quay.",
            "The longest crossing takes forty minutes.",
            "The smallest island has eleven people.",
            "Back where we started, after dark.",
        ],
    },
    Video {
        id: "archipelago",
        title: "The archipelago from above",
        channel: "atlas",
        topic: Topic::Islands,
        scene: SceneKind::Islands,
        hue: 176.0,
        accent: 90.0,
        duration: 30.0,
        days: 160.0,
        views: 72_500,
        likes: 5300,
        description: "The archipelago drawn as a map, with the ferry route between the islands and the shallows around them.",
        captions: &[
            "Six islands, seen from above.",
            "The pale water is shallow.",
            "The ferry route makes a loop.",
        ],
    },
    Video {
        id: "wind-plain",
        title: "Wind across the coastal plain",
        channel: "juniper",
        topic: Topic::Countryside,
        scene: SceneKind::Fields,
        hue: 205.0,
        accent: 98.0,
        duration: 34.0,
        days: 12.0,
        views: 6400,
        likes: 520,
        description: "The plain behind the coast is the windiest place in the region, which is why the turbines are here. A spring afternoon of clouds, grass and slow blades.",
        captions: &[
            "The plain behind the coast.",
            "Four turbines, each turning at its own pace.",
            "The wind comes off the sea.",
            "Clouds cross in about an hour.",
        ],
    },
    Video {
        id: "green-hills",
        title: "A long walk over the green hills",
        channel: "juniper",
        topic: Topic::Countryside,
        scene: SceneKind::Fields,
        hue: 196.0,
        accent: 120.0,
        duration: 46.0,
        days: 90.0,
        views: 15_800,
        likes: 1190,
        description: "Twenty kilometers over the hills between two villages, on a path older than either of them. Most of it is grass, sky and the turbines on the far ridge.",
        captions: &[
            "The path leaves the first village at nine.",
            "Grass, sky and nothing else for an hour.",
            "The turbines on the far ridge.",
            "Halfway, and the wind picks up.",
            "The second village, by late afternoon.",
        ],
    },
    Video {
        id: "boat-afloat",
        title: "Keeping a small boat afloat",
        channel: "harbor",
        topic: Topic::Harbors,
        scene: SceneKind::Sea,
        hue: 18.0,
        accent: 340.0,
        duration: 40.0,
        days: 120.0,
        views: 23_700,
        likes: 1980,
        description: "What a small wooden boat needs each season to keep going: a look at the hull, new rope and a slow trip round the bay at sunset to see that it all holds.",
        captions: &[
            "The boat is fifty years old.",
            "New rope every spring.",
            "A slow trip round the bay at sunset.",
            "Everything holds.",
            "Back to the mooring before dark.",
        ],
    },
    Video {
        id: "storm-watch",
        title: "Watching a storm come in from the sea",
        channel: "harbor",
        topic: Topic::Weather,
        scene: SceneKind::Lighthouse,
        hue: 205.0,
        accent: 200.0,
        duration: 38.0,
        days: 33.0,
        views: 41_000,
        likes: 3210,
        description: "The storm was forecast for midnight and came at ten. From the harbor master’s window you can see the lighthouse on the point and every wave that reaches it.",
        captions: &[
            "The forecast said midnight.",
            "It came at ten.",
            "The light on the point keeps turning.",
            "Waves reach the top of the breakwater.",
        ],
    },
    Video {
        id: "high-water",
        title: "Why high water comes twice a day",
        channel: "tides",
        topic: Topic::Tides,
        scene: SceneKind::Tide,
        hue: 230.0,
        accent: 32.0,
        duration: 46.0,
        days: 400.0,
        views: 230_000,
        likes: 17_400,
        description: "The moon pulls the sea toward it, and the earth turns under that pull, so most coasts see two high waters a day. This video draws one day of it and explains the rest as it goes.",
        captions: &[
            "The moon pulls the sea toward it.",
            "The earth turns under that pull.",
            "So the water rises twice a day.",
            "Each high water is about fifty minutes later than yesterday.",
            "The shape of the coast does the rest.",
            "Some harbors see only one tide a day.",
        ],
    },
    Video {
        id: "salt-flats",
        title: "Salt flats at dawn, from the train",
        channel: "larkspur",
        topic: Topic::Rail,
        scene: SceneKind::Train,
        hue: 200.0,
        accent: 40.0,
        duration: 30.0,
        days: 6.0,
        views: 4400,
        likes: 388,
        description: "The line crosses a salt flat for an hour just after dawn, and from the window it looks like the train is running over water.",
        captions: &[
            "Dawn over the salt flat.",
            "It looks like water from here.",
            "An hour of white, then the hills again.",
        ],
    },
    Video {
        id: "glacier",
        title: "Under the glacier in midsummer",
        channel: "juniper",
        topic: Topic::Mountains,
        scene: SceneKind::Peaks,
        hue: 190.0,
        accent: 200.0,
        duration: 36.0,
        days: 210.0,
        views: 37_600,
        likes: 2910,
        description: "Even in midsummer it snows under the glacier. A day at the foot of the ice, with the ridges stacked behind it.",
        captions: &[
            "Midsummer, and still snowing.",
            "The ice is moving, slowly.",
            "Three ridges behind the glacier.",
            "The valley below is green.",
        ],
    },
];

/// The channel a video is on, the first channel when it names none there is.
pub fn channel_of(video: &Video) -> &'static Channel {
    channel_by_id(video.channel).unwrap_or(&CHANNELS[0])
}

pub fn video_by_id(id: &str) -> Option<&'static Video> {
    VIDEOS.iter().find(|one| one.id == id)
}

pub fn channel_by_id(id: &str) -> Option<&'static Channel> {
    CHANNELS.iter().find(|one| one.id == id)
}

/// A channel's videos, in the catalog's order.
pub fn videos_of(channel: &Channel) -> Vec<&'static Video> {
    VIDEOS.iter().filter(|video| video.channel == channel.id).collect()
}

/// A stable number from a video's id, the seed its clip is drawn from: FNV-1a over the first
/// UTF-16 unit of each character, as the web's `seedOf` reads them.
pub fn seed_of(id: &str) -> u32 {
    let mut hash: u32 = 2_166_136_261;
    for c in id.chars() {
        let mut units = [0u16; 2];
        let first = c.encode_utf16(&mut units)[0];
        hash = (hash ^ u32::from(first)).wrapping_mul(16_777_619);
    }
    hash
}
