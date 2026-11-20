//! Tidevann's music library, as the web's `catalog.ts` makes it: the artists, their albums and
//! every song on them, made up and fixed, so the same library is there on every visit. Each song's
//! sound is drawn from its seed by the synthesizer when it is first played (`synth`), so its length
//! is known here before a note is made. Radio songs are made the same way on the fly, one after
//! another.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sokkel_egui_media::{ARTWORK_HUES, ARTWORK_MOTIFS, ArtworkCover, ArtworkHue, ArtworkMotif, cover_for, seed_hash};

use crate::data::MadeStation;
use crate::js;
use crate::synth::{Seeded, Style, length_of};

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

fn motif(key: &str) -> ArtworkMotif {
    ARTWORK_MOTIFS
        .into_iter()
        .find(|one| motif_key(*one) == key)
        .unwrap_or(ArtworkMotif::Sun)
}

fn hue(key: &str) -> ArtworkHue {
    ARTWORK_HUES
        .into_iter()
        .find(|one| hue_key(*one) == key)
        .unwrap_or(ArtworkHue::Blue)
}

/// A cover written as the web keeps it: `{ motif, from, to }`, each by name.
#[derive(Serialize, Deserialize)]
struct CoverKept {
    motif: String,
    from: String,
    to: String,
}

/// Serde for an `ArtworkCover` field, as the web keeps a cover.
pub mod cover_serde {
    use super::*;

    pub fn serialize<S: Serializer>(cover: &ArtworkCover, serializer: S) -> Result<S::Ok, S::Error> {
        CoverKept {
            motif: motif_key(cover.motif).to_owned(),
            from: hue_key(cover.from).to_owned(),
            to: hue_key(cover.to).to_owned(),
        }
        .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<ArtworkCover, D::Error> {
        let kept = CoverKept::deserialize(deserializer)?;
        Ok(cover(&kept.motif, &kept.from, &kept.to))
    }
}

/// A cover from its motif's and hues' names.
pub fn cover(motif_name: &str, from: &str, to: &str) -> ArtworkCover {
    ArtworkCover {
        motif: motif(motif_name),
        from: hue(from),
        to: hue(to),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub style: Style,
    /// A sentence on who they are.
    pub about: String,
    #[serde(with = "cover_serde")]
    pub cover: ArtworkCover,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub year: i32,
    #[serde(with = "cover_serde")]
    pub cover: ArtworkCover,
    /// How many discs it spans.
    pub discs: u32,
}

/// Whether a song has words, and whether they are timed to the music.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Words {
    Synced,
    Plain,
    None,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Song {
    pub id: String,
    pub title: String,
    pub album: String,
    pub artist: String,
    /// Another artist singing or playing on it, credited under the title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with: Option<String>,
    pub disc: u32,
    pub number: u32,
    /// Seconds, as the synthesizer will make it.
    pub duration: f64,
    /// What its sound is drawn from.
    pub seed: u32,
    pub style: Style,
    pub words: Words,
}

const ARTISTS_GIVEN: [(&str, Style, &str); 13] = [
    (
        "Ada",
        Style::Folk,
        "Songs written on a kitchen table on the coast, recorded in one take.",
    ),
    (
        "Brackwater Duo",
        Style::Electronic,
        "Two synthesizers and a drum machine, between the river and the sea.",
    ),
    (
        "Eelgrass",
        Style::Ambient,
        "Long, slow pieces that drift like weed in the current.",
    ),
    (
        "Grace",
        Style::Pop,
        "Bright songs about leaving harbors and coming home.",
    ),
    (
        "Harbor Quartet",
        Style::Chamber,
        "Strings and a piano, playing music for the waterfront.",
    ),
    (
        "Juniper Trio",
        Style::Folk,
        "Guitar, fiddle and three voices from the high moors.",
    ),
    (
        "Kelpwood",
        Style::Pop,
        "A four-piece with a soft spot for big choruses.",
    ),
    ("Larkspur", Style::Pop, "Summer records from a studio by the dunes."),
    ("Linus", Style::Electronic, "Night music for ferries and empty roads."),
    (
        "Margaret",
        Style::Ambient,
        "Field recordings and drones from the lighthouse years.",
    ),
    (
        "Northwater Choir",
        Style::Choir,
        "Thirty voices in a stone church by the fjord.",
    ),
    (
        "Saltmarsh Ensemble",
        Style::Chamber,
        "A small orchestra that plays where the tide comes in.",
    ),
    (
        "The Tidelines",
        Style::Band,
        "Loud guitars, louder drums, and songs about the weather.",
    ),
];

/// An id from a name: lower case, spaces as dashes.
fn id_of(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

type AlbumGiven = (
    &'static str,
    &'static str,
    i32,
    (&'static str, &'static str, &'static str),
    u32,
);

const ALBUMS_GIVEN: [AlbumGiven; 28] = [
    ("Open Water", "Grace", 2025, ("wave", "blue", "teal"), 1),
    ("Northern Coast", "Ada", 2024, ("sun", "violet", "pink"), 1),
    ("High Desert", "Linus", 2024, ("peaks", "amber", "coral"), 1),
    ("Glass Harbor", "Harbor Quartet", 2023, ("rings", "teal", "green"), 1),
    ("Slow Current", "Margaret", 2023, ("bands", "jade", "blue"), 1),
    ("Archipelago", "Juniper Trio", 2022, ("dots", "pink", "amber"), 2),
    ("Lowlands", "Ada", 2022, ("bands", "green", "jade"), 1),
    ("River Delta", "Grace", 2021, ("sun", "teal", "blue"), 1),
    ("Meridian", "Larkspur", 2021, ("rings", "coral", "violet"), 1),
    ("Salt and Cedar", "Linus", 2020, ("peaks", "jade", "teal"), 1),
    ("Equinox", "Margaret", 2020, ("dots", "violet", "blue"), 1),
    ("Morning Fjord", "Harbor Quartet", 2019, ("wave", "pink", "violet"), 1),
    ("Highland", "Juniper Trio", 2019, ("peaks", "blue", "violet"), 1),
    ("Atoll", "Grace", 2018, ("rings", "jade", "amber"), 1),
    ("Coastal Plain", "Ada", 2017, ("wave", "green", "teal"), 1),
    ("Tidewater", "Larkspur", 2016, ("sun", "coral", "pink"), 2),
    ("Shoals", "Brackwater Duo", 2024, ("wave", "teal", "blue"), 1),
    ("Neap Tide", "Brackwater Duo", 2021, ("bands", "blue", "violet"), 1),
    ("Sea Glass", "Eelgrass", 2023, ("dots", "jade", "teal"), 1),
    ("Quayside", "Eelgrass", 2020, ("rings", "amber", "pink"), 1),
    ("Long Swell", "Eelgrass", 2018, ("wave", "violet", "blue"), 1),
    ("Ebb and Flow", "Kelpwood", 2023, ("sun", "green", "amber"), 1),
    ("Lantern Rock", "Northwater Choir", 2022, ("peaks", "coral", "amber"), 1),
    ("Estuary", "Saltmarsh Ensemble", 2025, ("bands", "teal", "jade"), 2),
    ("Skerries", "Saltmarsh Ensemble", 2019, ("rings", "blue", "teal"), 1),
    ("Pilot Light", "The Tidelines", 2024, ("sun", "pink", "coral"), 1),
    ("Driftwood", "The Tidelines", 2022, ("peaks", "jade", "green"), 1),
    ("Shingle Beach", "The Tidelines", 2017, ("dots", "violet", "pink"), 1),
];

const PLACES: [&str; 36] = [
    "Harbor",
    "Tide",
    "Shore",
    "Breakwater",
    "Lighthouse",
    "Quay",
    "Ferry",
    "Island",
    "Current",
    "Inlet",
    "Dune",
    "Cove",
    "Headland",
    "Marsh",
    "Reef",
    "Pier",
    "Channel",
    "Lagoon",
    "Fjord",
    "Moor",
    "Valley",
    "Ridge",
    "Meadow",
    "Orchard",
    "River",
    "Bay",
    "Sound",
    "Estuary",
    "Jetty",
    "Beacon",
    "Causeway",
    "Skerry",
    "Mooring",
    "Shingle",
    "Kelp",
    "Salt",
];
const QUALITIES: [&str; 26] = [
    "Low", "High", "Slow", "Quiet", "Northern", "Silver", "Long", "Last", "First", "Open", "Still", "Cold", "Late",
    "Early", "Grey", "Golden", "Hidden", "Distant", "Bright", "Small", "Far", "Rising", "Turning", "Western", "Summer",
    "Winter",
];
const TIMES: [&str; 10] = [
    "Dusk",
    "Dawn",
    "Night",
    "Noon",
    "Morning",
    "Evening",
    "Midnight",
    "First Light",
    "Low Water",
    "High Water",
];
const WORDS_ALONE: [&str; 19] = [
    "Undertow",
    "Passage",
    "Driftwood",
    "Slack Water",
    "Homeward",
    "Afterglow",
    "Crosswind",
    "Landfall",
    "Wayfinder",
    "Ebb",
    "Swell",
    "Squall",
    "Mainland",
    "Halyard",
    "Spindrift",
    "Stillwater",
    "Seaward",
    "Windward",
    "Leeward",
];

fn pick<'a>(list: &[&'a str], random: &mut Seeded) -> &'a str {
    list[(random.next_f64() * list.len() as f64).floor() as usize]
}

/// A song's title, drawn from `random` in one of a few shapes, no word in it twice.
pub fn title_from(random: &mut Seeded) -> String {
    loop {
        let title = shape_of(random);
        let lower = title.to_lowercase();
        let words: Vec<&str> = lower.split(' ').collect();
        let mut unique = words.clone();
        unique.sort_unstable();
        unique.dedup();
        if unique.len() == words.len() {
            return title;
        }
    }
}

fn shape_of(random: &mut Seeded) -> String {
    let shape = (random.next_f64() * 7.0).floor() as u32;
    match shape {
        0 => {
            let quality = pick(&QUALITIES, random);
            format!("{quality} {}", pick(&PLACES, random))
        }
        1 => {
            let quality = pick(&QUALITIES, random);
            format!("The {quality} {}", pick(&PLACES, random))
        }
        2 => {
            let place = pick(&PLACES, random);
            format!("{place} at {}", pick(&TIMES, random))
        }
        3 => {
            let place = pick(&PLACES, random);
            format!("{place} and {}", pick(&PLACES, random))
        }
        4 => pick(&WORDS_ALONE, random).to_owned(),
        5 => format!("Over the {}", pick(&PLACES, random)),
        _ => {
            let quality = pick(&QUALITIES, random);
            format!("{quality} {}", pick(&TIMES, random))
        }
    }
}

const PEOPLE: [&str; 4] = ["Ada", "Grace", "Linus", "Margaret"];

fn songs_of(album: &Album, artist: &Artist) -> Vec<Song> {
    let mut random = Seeded::new(seed_hash(&format!("album:{}", album.id)));
    let per_disc = if album.discs > 1 {
        6 + (random.next_f64() * 2.0).floor() as u32
    } else {
        7 + (random.next_f64() * 4.0).floor() as u32
    };
    let mut titles: Vec<String> = Vec::new();
    let mut songs = Vec::new();
    for disc in 1..=album.discs {
        for number in 1..=per_disc {
            let mut title = title_from(&mut random);
            while titles.contains(&title) || title == album.title {
                title = title_from(&mut random);
            }
            titles.push(title.clone());
            let seed = seed_hash(&format!("song:{}:{disc}:{number}", album.id));
            let guest = (random.next_f64() < 0.15).then(|| {
                let others: Vec<&str> = PEOPLE.into_iter().filter(|person| *person != artist.name).collect();
                others
                    .get((random.next_f64() * 3.0).floor() as usize)
                    .map(|one| (*one).to_owned())
            });
            let lyric = match artist.style {
                Style::Ambient | Style::Electronic => 0.25,
                Style::Chamber => 0.2,
                _ => 0.85,
            };
            let words = if random.next_f64() < lyric {
                if random.next_f64() < 0.8 {
                    Words::Synced
                } else {
                    Words::Plain
                }
            } else {
                Words::None
            };
            songs.push(Song {
                id: format!("{}-{disc}-{number}", album.id),
                title,
                album: album.id.clone(),
                artist: artist.id.clone(),
                with: guest.flatten(),
                disc,
                number,
                duration: length_of(seed, artist.style),
                seed,
                style: artist.style,
                words,
            });
        }
    }
    songs
}

struct Library {
    artists: Vec<Artist>,
    albums: Vec<Album>,
    songs: Vec<Song>,
    stations: Vec<Station>,
    artist_by_id: HashMap<String, usize>,
    album_by_id: HashMap<String, usize>,
    song_by_id: HashMap<String, usize>,
}

fn library() -> &'static Library {
    static LIBRARY: OnceLock<Library> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        let artists: Vec<Artist> = ARTISTS_GIVEN
            .iter()
            .map(|(name, style, about)| Artist {
                id: id_of(name),
                name: (*name).to_owned(),
                style: *style,
                about: (*about).to_owned(),
                cover: cover_for(&format!("artist:{name}")),
            })
            .collect();
        let albums: Vec<Album> = ALBUMS_GIVEN
            .iter()
            .map(|(title, artist, year, (motif_name, from, to), discs)| Album {
                id: id_of(title),
                title: (*title).to_owned(),
                artist: id_of(artist),
                year: *year,
                cover: cover(motif_name, from, to),
                discs: *discs,
            })
            .collect();
        let artist_by_id: HashMap<String, usize> = artists
            .iter()
            .enumerate()
            .map(|(at, artist)| (artist.id.clone(), at))
            .collect();
        let songs: Vec<Song> = albums
            .iter()
            .flat_map(|album| {
                artist_by_id
                    .get(&album.artist)
                    .map(|at| songs_of(album, &artists[*at]))
                    .unwrap_or_default()
            })
            .collect();
        let by_style = |styles: &[Style]| -> Vec<String> {
            artists
                .iter()
                .filter(|artist| styles.contains(&artist.style))
                .map(|artist| artist.id.clone())
                .collect()
        };
        let station = |id: &str, name: &str, style, about: &str, (motif_name, from, to), styles: &[Style]| Station {
            id: id.to_owned(),
            name: name.to_owned(),
            style,
            about: about.to_owned(),
            cover: cover(motif_name, from, to),
            artists: by_style(styles),
        };
        let stations = vec![
            station(
                "slack-water",
                "Slack Water",
                Style::Ambient,
                "Drones and slow pads for reading and sleep",
                ("wave", "blue", "violet"),
                &[Style::Ambient],
            ),
            station(
                "harbor-lights",
                "Harbor Lights",
                Style::Pop,
                "Bright songs with big choruses",
                ("sun", "amber", "pink"),
                &[Style::Pop],
            ),
            station(
                "night-ferry",
                "Night Ferry",
                Style::Electronic,
                "Four on the floor until morning",
                ("rings", "violet", "teal"),
                &[Style::Electronic],
            ),
            station(
                "fireside",
                "Fireside",
                Style::Folk,
                "Guitars, fiddles and voices",
                ("peaks", "coral", "amber"),
                &[Style::Folk],
            ),
            station(
                "stone-church",
                "Stone Church",
                Style::Choir,
                "Voices and strings in a high room",
                ("bands", "jade", "blue"),
                &[Style::Choir, Style::Chamber],
            ),
            station(
                "loud-weather",
                "Loud Weather",
                Style::Band,
                "Guitars and drums, turned up",
                ("dots", "pink", "coral"),
                &[Style::Band, Style::Pop],
            ),
        ];
        let album_by_id = albums
            .iter()
            .enumerate()
            .map(|(at, album)| (album.id.clone(), at))
            .collect();
        let song_by_id = songs
            .iter()
            .enumerate()
            .map(|(at, song)| (song.id.clone(), at))
            .collect();
        Library {
            artists,
            albums,
            songs,
            stations,
            artist_by_id,
            album_by_id,
            song_by_id,
        }
    })
}

/// Every artist, in the web's order.
pub fn artists() -> &'static [Artist] {
    &library().artists
}

/// Every album, in the web's order.
pub fn albums() -> &'static [Album] {
    &library().albums
}

/// Every song in the library, album by album.
pub fn songs() -> &'static [Song] {
    &library().songs
}

pub fn artist_by_id(id: &str) -> Option<&'static Artist> {
    let library = library();
    library.artist_by_id.get(id).map(|at| &library.artists[*at])
}

pub fn album_by_id(id: &str) -> Option<&'static Album> {
    let library = library();
    library.album_by_id.get(id).map(|at| &library.albums[*at])
}

/// A song in the library (not a radio song) by its id.
pub fn library_song(id: &str) -> Option<&'static Song> {
    let library = library();
    library.song_by_id.get(id).map(|at| &library.songs[*at])
}

/// A radio station: an endless run of new songs in one style, or in the style of what it was
/// started from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Station {
    pub id: String,
    pub name: String,
    pub style: Style,
    /// What it is about, in a line.
    pub about: String,
    #[serde(with = "cover_serde")]
    pub cover: ArtworkCover,
    /// The artists its songs are credited to.
    pub artists: Vec<String>,
}

/// The stations every listener has.
pub fn stations() -> &'static [Station] {
    &library().stations
}

/// What a station is started from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StationKind {
    Artist,
    Album,
    Song,
}

/// A station started from an artist, an album or a song: new songs in its style, credited to it
/// and its kind. Its id is the caller's to give.
pub fn station_from(kind: StationKind, reference: &str) -> Option<Station> {
    let song = (kind == StationKind::Song).then(|| library_song(reference)).flatten();
    let album = if kind == StationKind::Album {
        album_by_id(reference)
    } else {
        song.and_then(|song| album_by_id(&song.album))
    };
    let artist = artist_by_id(if kind == StationKind::Artist {
        reference
    } else {
        album.map_or("", |album| album.artist.as_str())
    })?;
    let name = match (kind, song, album) {
        (StationKind::Song, Some(song), _) => song.title.as_str(),
        (StationKind::Album, _, Some(album)) => album.title.as_str(),
        _ => artist.name.as_str(),
    };
    let mut credited = vec![artist.id.clone()];
    credited.extend(
        artists()
            .iter()
            .filter(|other| other.style == artist.style && other.id != artist.id)
            .map(|other| other.id.clone()),
    );
    let kind_key = match kind {
        StationKind::Artist => "artist",
        StationKind::Album => "album",
        StationKind::Song => "song",
    };
    Some(Station {
        id: String::new(),
        name: format!("{name} radio"),
        style: artist.style,
        about: format!(
            "New {} songs in the spirit of {name}",
            artist.style.name().to_lowercase()
        ),
        cover: cover_for(&format!("station:{kind_key}:{reference}")),
        artists: credited,
    })
}

/// The `index`th song a station plays, made from the station's id and the count, so it is the same
/// each time it is asked for.
pub fn radio_song(station: &Station, index: u64) -> Song {
    let seed = seed_hash(&format!("radio:{}:{index}", station.id));
    let mut random = Seeded::new(seed);
    let pick = (random.next_f64() * station.artists.len() as f64).floor() as usize;
    let artist = station.artists.get(pick).cloned().unwrap_or_else(|| "ada".to_owned());
    let title = title_from(&mut random);
    let words = if matches!(station.style, Style::Ambient | Style::Electronic) {
        Words::None
    } else if random.next_f64() < 0.6 {
        Words::Synced
    } else {
        Words::None
    };
    Song {
        id: format!("radio:{}:{index}", station.id),
        title,
        album: format!("radio:{}", station.id),
        artist,
        with: None,
        disc: 1,
        number: index as u32 + 1,
        duration: length_of(seed, station.style),
        seed,
        style: station.style,
        words,
    }
}

/// A station by its id: one every listener has, or one the listener made.
pub fn station_of(id: &str, made: &[MadeStation]) -> Option<Station> {
    stations()
        .iter()
        .find(|station| station.id == id)
        .cloned()
        .or_else(|| made.iter().find(|station| station.id == id).map(MadeStation::station))
}

/// A song by its id: one in the library, or a radio song, made again from its station and count.
pub fn song_of(id: &str, made: &[MadeStation]) -> Option<Song> {
    if let Some(found) = library_song(id) {
        return Some(found.clone());
    }
    // `^radio:(.+):(\d+)$`
    let rest = id.strip_prefix("radio:")?;
    let (station, count) = rest.rsplit_once(':')?;
    if station.is_empty() || count.is_empty() || !count.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if station.contains(['\n', '\r', '\u{2028}', '\u{2029}']) {
        return None;
    }
    let station = station_of(station, made)?;
    Some(radio_song(&station, js::number_of_text(count) as u64))
}

/// The album a song is on, or for a radio song its station dressed as one.
pub fn album_of(song: &Song, made: &[MadeStation]) -> Album {
    if let Some(album) = album_by_id(&song.album) {
        return album.clone();
    }
    let station = station_of(song.album.strip_prefix("radio:").unwrap_or(&song.album), made);
    Album {
        id: song.album.clone(),
        title: station
            .as_ref()
            .map_or_else(|| "Radio".to_owned(), |station| station.name.clone()),
        artist: song.artist.clone(),
        year: 2026,
        cover: station.map_or_else(|| cover_for(&song.album), |station| station.cover),
        discs: 1,
    }
}

/// An artist's name, or the id when there is no such artist.
pub fn artist_name(id: &str) -> String {
    artist_by_id(id).map_or_else(|| id.to_owned(), |artist| artist.name.clone())
}

pub fn songs_of_album(id: &str) -> Vec<&'static Song> {
    songs().iter().filter(|song| song.album == id).collect()
}

/// An artist's albums, the newest first.
pub fn albums_of_artist(id: &str) -> Vec<&'static Album> {
    let mut found: Vec<&Album> = albums().iter().filter(|album| album.artist == id).collect();
    found.sort_by_key(|album| std::cmp::Reverse(album.year));
    found
}

pub fn songs_of_artist(id: &str) -> Vec<&'static Song> {
    songs().iter().filter(|song| song.artist == id).collect()
}

/// Minutes, rounded, written for a line under a title.
pub fn minutes(seconds: f64) -> String {
    let whole = js::round(seconds / 60.0).max(1.0);
    format!("{} min", js::number_to_string(whole))
}

/// A total length of songs.
pub fn length_of_songs<'a>(songs: impl IntoIterator<Item = &'a Song>) -> f64 {
    songs.into_iter().fold(0.0, |sum, song| sum + song.duration)
}

/// The size a song takes kept on the device: 16-bit samples at the synthesizer's rate.
pub fn bytes_of(song: &Song) -> f64 {
    js::round(song.duration * 22_050.0 * 2.0 + 44.0)
}

/// Bytes written for people: 4.2 MB.
pub fn format_bytes(bytes: f64) -> String {
    if bytes < 1_000_000.0 {
        return format!("{} kB", js::number_to_string(js::round(bytes / 1000.0).max(1.0)));
    }
    format!("{} MB", js::to_fixed(bytes / 1_000_000.0, 1))
}
