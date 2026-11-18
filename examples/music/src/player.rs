//! The player, as the web's `player.ts` runs it: the queue it plays from (`queue`), shuffle and
//! repeat, where along the song it is, and radio, which keeps making the next song. The web plays
//! through two audio elements; here the app makes the sound (`synth::render`) and plays it, and
//! tells the player what the sound did: `heard` as it plays, `ended` at its end. What the player is
//! doing is written to `Library::player` wherever the web keeps it, so a new session picks up
//! where it was.
//!
//! Where the web draws from `Math.random` (a shuffle's seed, where a shuffled play starts), the
//! caller passes the draw, so the same draws give the same queue.

use crate::catalog::{Song, radio_song, song_of, station_of};
use crate::data::{Library, PlayerRow, QueueEntry, Radio, Source, SourceKind};
use crate::js;
use crate::queue::{Queue, QueueOptions, QueueSeed, RepeatMode, create_queue};

/// How many songs a station keeps waiting after the one playing.
pub const RADIO_AHEAD: usize = 3;

/// Seconds into a song past which previous starts it again rather than going back.
const RESTART_AFTER: f64 = 3.0;

/// How often the position is kept while a song plays, in milliseconds.
const KEEP_EVERY: i64 = 5000;

/// The quiet the web waits for before it keeps a change, in milliseconds.
const KEEP_QUIET: i64 = 400;

#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    pub queue: Queue<QueueEntry>,
    pub source: Option<Source>,
    pub radio: Option<Radio>,
    pub playing: bool,
    /// Seconds the current song lasts, or 0 with none.
    pub duration: f64,
    /// Seconds into what plays.
    pub position: f64,
    /// The song whose sound is cued (the web's element has its source).
    pub cued: Option<String>,
    /// Whether the sound is going (the web's element is not paused), which `playing` follows
    /// except where the web's does not.
    sounding: bool,
    /// When the position was last kept.
    last_kept: i64,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            queue: create_queue(Vec::new(), QueueOptions::default()),
            source: None,
            radio: None,
            playing: false,
            duration: 0.0,
            position: 0.0,
            cued: None,
            sounding: false,
            last_kept: 0,
        }
    }
}

/// Where a list of songs starts playing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayFrom {
    /// The index of the song to start on, unless shuffle is set on.
    pub start: usize,
    /// Shuffle set on or off; none keeps it as it is.
    pub shuffle: Option<bool>,
}

/// Where songs put in the queue go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// Right after the one playing.
    Next,
    /// At the end.
    End,
}

impl Player {
    /// The song playing or paused, if any.
    pub fn current_song(&self, library: &Library) -> Option<Song> {
        self.queue
            .item()
            .and_then(|entry| song_of(&entry.song, &library.stations))
    }

    /// The song the queue goes to when this one ends, for its sound to be made ahead.
    pub fn next_song(&self, library: &Library) -> Option<Song> {
        self.queue
            .ended()
            .and_then(|next| next.item().and_then(|entry| song_of(&entry.song, &library.stations)))
    }

    /// What the player is doing, as it is kept.
    pub fn row(&self) -> PlayerRow {
        PlayerRow {
            id: "now".to_owned(),
            entries: self.queue.items.clone(),
            order: self.queue.order.iter().map(|index| *index as i64).collect(),
            at: self.queue.at,
            shuffle: self.queue.shuffle,
            repeat: self.queue.repeat,
            position: self.position,
            source: self.source.clone(),
            radio: self.radio.clone(),
        }
    }

    /// Keeps what the player is doing at once.
    fn keep_now(&mut self, library: &mut Library, now: i64) {
        self.last_kept = now;
        library.player = self.row();
    }

    /// Keeps what the player is doing; the web writes it after a moment's quiet, so that is when
    /// it counts as kept.
    fn keep_soon(&mut self, library: &mut Library, now: i64) {
        self.last_kept = now + KEEP_QUIET;
        library.player = self.row();
    }

    /// Puts the current song in place from `at` seconds, playing if asked.
    fn cue(&mut self, library: &mut Library, at: f64, play: bool, now: i64) {
        let Some(song) = self.current_song(library) else {
            self.pause(library, now);
            self.playing = false;
            self.duration = 0.0;
            self.position = 0.0;
            self.cued = None;
            return;
        };
        self.duration = song.duration;
        // A new song's sound stops the last one's, without the pause being heard.
        if self.cued.as_deref() != Some(song.id.as_str()) {
            self.cued = Some(song.id.clone());
            self.sounding = false;
        }
        self.heard(library, at.min((song.duration - 0.5).max(0.0)), now);
        if play {
            self.sound();
            library.played(&song.id, now);
        }
    }

    /// The sound starts.
    fn sound(&mut self) {
        self.sounding = true;
        self.playing = true;
    }

    /// The sound pauses, if it was going, and that is kept.
    fn pause(&mut self, library: &mut Library, now: i64) {
        if self.sounding {
            self.sounding = false;
            self.playing = false;
            self.keep_now(library, now);
        }
    }

    /// Moves to a new queue state and plays its current song from the start.
    fn go(&mut self, library: &mut Library, queue: Queue<QueueEntry>, now: i64) {
        self.queue = queue;
        self.top_up_radio(library);
        self.cue(library, 0.0, true, now);
        self.keep_now(library, now);
    }

    /// Makes more of the station's songs as the queue nears its end, so it never runs out.
    fn top_up_radio(&mut self, library: &Library) {
        let Some(radio) = self.radio.clone() else { return };
        let Some(station) = station_of(&radio.station, &library.stations) else {
            return;
        };
        let ahead = self.queue.order.len() as i64 - 1 - self.queue.at;
        if ahead >= RADIO_AHEAD as i64 {
            return;
        }
        let count = (RADIO_AHEAD as i64 - ahead) as u64;
        let fresh: Vec<QueueEntry> = (0..count)
            .map(|index| {
                let song = radio_song(&station, radio.made + index);
                QueueEntry {
                    key: song.id.clone(),
                    song: song.id,
                }
            })
            .collect();
        self.queue = self.queue.add(&fresh, false);
        self.radio = Some(Radio {
            station: radio.station,
            made: radio.made + count,
        });
    }

    /// Plays `songs` from the one at `from.start`, as the queue from `source`. `from.shuffle` sets
    /// shuffle, none keeps it as it is; when it is set on, the song to start on is drawn from
    /// `random`, and the shuffle's seed is drawn after.
    pub fn play_songs(
        &mut self,
        library: &mut Library,
        songs: &[String],
        source: Source,
        from: PlayFrom,
        now: i64,
        random: &mut dyn FnMut() -> f64,
    ) {
        if songs.is_empty() {
            return;
        }
        let stamp = js::base36(now);
        let entries: Vec<QueueEntry> = songs
            .iter()
            .enumerate()
            .map(|(index, song)| QueueEntry {
                key: format!("{song}~{index}~{stamp}"),
                song: song.clone(),
            })
            .collect();
        let PlayFrom { start, shuffle } = from;
        let start = if shuffle == Some(true) {
            (random() * songs.len() as f64).floor()
        } else {
            start as f64
        };
        let seed = (random() * 1e9).floor();
        let repeat = if self.queue.repeat == RepeatMode::One {
            RepeatMode::Off
        } else {
            self.queue.repeat
        };
        let queue = create_queue(
            entries,
            QueueOptions {
                start: Some(start),
                shuffle: Some(shuffle.unwrap_or(self.queue.shuffle)),
                repeat: Some(repeat),
                seed: Some(QueueSeed::Number(seed)),
                order: None,
            },
        );
        self.source = Some(source);
        self.radio = None;
        self.go(library, queue, now);
    }

    /// Plays a station: one of its songs after another, without end.
    pub fn play_station(&mut self, library: &mut Library, id: &str, label: &str, now: i64) {
        let Some(station) = station_of(id, &library.stations) else {
            return;
        };
        let first = radio_song(&station, 0);
        self.source = Some(Source::new(SourceKind::Radio, label, Some("/radio".to_owned())));
        self.radio = Some(Radio {
            station: id.to_owned(),
            made: 1,
        });
        let queue = create_queue(
            vec![QueueEntry {
                key: first.id.clone(),
                song: first.id,
            }],
            QueueOptions::default(),
        );
        self.go(library, queue, now);
    }

    /// Plays or pauses.
    pub fn toggle_play(&mut self, library: &mut Library, now: i64) {
        if self.current_song(library).is_none() {
            return;
        }
        if self.playing {
            self.pause(library, now);
            return;
        }
        if self.cued.is_some() {
            self.sound();
        } else {
            self.cue(library, self.position, true, now);
        }
    }

    pub fn set_playing(&mut self, library: &mut Library, on: bool, now: i64) {
        if on != self.playing {
            self.toggle_play(library, now);
        }
    }

    /// Goes to `seconds` into the song.
    pub fn seek(&mut self, library: &mut Library, seconds: f64, now: i64) {
        self.heard(library, seconds.min(self.duration).max(0.0), now);
        self.keep_soon(library, now);
    }

    pub fn seek_by(&mut self, library: &mut Library, seconds: f64, now: i64) {
        self.seek(library, self.position + seconds, now);
    }

    pub fn next(&mut self, library: &mut Library, now: i64) {
        if let Some(next) = self.queue.next() {
            self.go(library, next, now);
        }
    }

    /// Goes back a song, or to this one's start when it has played a while, on radio, or at the
    /// start of the queue.
    pub fn previous(&mut self, library: &mut Library, now: i64) {
        let before = self.queue.previous();
        match before {
            Some(before) if self.position <= RESTART_AFTER && self.radio.is_none() => self.go(library, before, now),
            _ => self.seek(library, 0.0, now),
        }
    }

    pub fn has_next(&self) -> bool {
        self.queue.next().is_some()
    }

    /// Plays the entry at `index` in the queue's list.
    pub fn play_index(&mut self, library: &mut Library, index: usize, now: i64) {
        let queue = self.queue.play(index);
        self.go(library, queue, now);
    }

    /// Plays the entry at `at` in the play order, as a press on a row of Up next asks.
    pub fn play_position(&mut self, library: &mut Library, at: usize, now: i64) {
        if let Some(index) = self.queue.order.get(at).copied() {
            self.play_index(library, index, now);
        }
    }

    /// Shuffles or unshuffles what plays; a shuffle's seed is drawn from `random`.
    pub fn set_shuffle(&mut self, library: &mut Library, on: bool, now: i64, random: &mut dyn FnMut() -> f64) {
        let seed = QueueSeed::Number((random() * 1e9).floor());
        self.queue = self.queue.shuffled(on, Some(&seed), None, None);
        self.keep_soon(library, now);
    }

    pub fn set_repeat(&mut self, library: &mut Library, mode: RepeatMode, now: i64) {
        self.queue = self.queue.repeating(mode);
        self.keep_soon(library, now);
    }

    /// Puts songs in the queue: right after the one playing, or at the end.
    pub fn enqueue(&mut self, library: &mut Library, songs: &[String], place: Place, now: i64) {
        let stamp = js::base36(now);
        let entries: Vec<QueueEntry> = songs
            .iter()
            .enumerate()
            .map(|(index, song)| QueueEntry {
                key: format!("{song}~{stamp}~{index}"),
                song: song.clone(),
            })
            .collect();
        let was = self.queue.at;
        self.queue = self.queue.add(&entries, place == Place::Next);
        if was < 0 {
            self.cue(library, 0.0, false, now);
        }
        self.keep_soon(library, now);
    }

    /// Takes the entries at `positions` in the play order out of the queue.
    pub fn dequeue(&mut self, library: &mut Library, positions: &[usize], now: i64) {
        let indexes: Vec<i64> = positions
            .iter()
            .filter_map(|at| self.queue.order.get(*at).map(|index| *index as i64))
            .collect();
        let playing_gone = indexes.contains(&self.queue.index());
        self.queue = self.queue.remove(&indexes);
        if playing_gone {
            let playing = self.playing;
            self.cue(library, 0.0, playing, now);
        }
        self.keep_soon(library, now);
    }

    /// Moves an entry in the play order, as the queue's list of what comes next is reordered.
    pub fn move_in_queue(&mut self, library: &mut Library, from: usize, to: usize, now: i64) {
        self.queue = self.queue.moved(from, to);
        self.keep_soon(library, now);
    }

    /// Takes everything after the song playing out of the queue.
    pub fn clear_up_next(&mut self, library: &mut Library, now: i64) {
        let from = (self.queue.at + 1).max(0) as usize;
        let positions: Vec<usize> = (from..self.queue.order.len()).collect();
        self.dequeue(library, &positions, now);
    }

    /// What plays after this song, in order, each with its place in the play order.
    pub fn up_next(&self) -> Vec<(QueueEntry, usize)> {
        let from = (self.queue.at + 1).max(0) as usize;
        self.queue
            .ordered()
            .into_iter()
            .skip(from)
            .map(|entry| (entry.item.clone(), entry.position))
            .collect()
    }

    /// Picks the player up as it was left: the queue, the song and where in it, paused.
    pub fn restore(&mut self, library: &mut Library, row: &PlayerRow, resume: bool, now: i64) {
        let entries: Vec<QueueEntry> = row
            .entries
            .iter()
            .filter(|entry| song_of(&entry.song, &library.stations).is_some())
            .cloned()
            .collect();
        let whole = entries.len() == row.entries.len();
        let count = entries.len();
        let queue = if whole {
            create_queue(
                entries,
                QueueOptions {
                    order: Some(row.order.clone()),
                    shuffle: Some(row.shuffle),
                    repeat: Some(row.repeat),
                    ..QueueOptions::default()
                },
            )
        } else {
            create_queue(
                entries,
                QueueOptions {
                    repeat: Some(row.repeat),
                    ..QueueOptions::default()
                },
            )
        };
        let at = if whole && row.at >= 0 && (row.at as usize) < count {
            row.at
        } else {
            0
        };
        self.queue = Queue {
            at: if count == 0 { -1 } else { at },
            ..queue
        };
        self.source = row.source.clone();
        self.radio = row.radio.clone();
        self.cue(library, if resume { row.position } else { 0.0 }, false, now);
    }

    /// The sound has played to `position` seconds: kept now and then.
    pub fn heard(&mut self, library: &mut Library, position: f64, now: i64) {
        self.position = position;
        if now - self.last_kept > KEEP_EVERY {
            self.keep_soon(library, now);
        }
    }

    /// The queue to fade into when `left` seconds of this song are left and songs cross-fade over
    /// `crossfade` seconds: the next song, unless this one plays again.
    pub fn crossfade_due(&self, crossfade: f64, left: f64) -> Option<Queue<QueueEntry>> {
        if crossfade <= 0.0 || !self.playing || left > crossfade {
            return None;
        }
        self.queue.ended().filter(|next| next.at != self.queue.at)
    }

    /// Starts the next song under this one as it fades out; the app moves the volumes.
    pub fn fade_to(&mut self, library: &mut Library, queue: Queue<QueueEntry>, now: i64) {
        self.go(library, queue, now);
    }

    /// The song has played to its end: the same again while it repeats one, the next one, or stop.
    pub fn ended(&mut self, library: &mut Library, now: i64) {
        self.sounding = false;
        match self.queue.ended() {
            None => {
                self.playing = false;
                self.heard(library, 0.0, now);
                self.keep_now(library, now);
            }
            Some(next) if next.at == self.queue.at => {
                self.heard(library, 0.0, now);
                self.sound();
            }
            Some(next) => self.go(library, next, now),
        }
    }

    /// Stops the sound, as the app is left.
    pub fn stop(&mut self, library: &mut Library, now: i64) {
        self.keep_now(library, now);
    }
}
