//! A queue, as sokkel-ui-media's `queue.ts` keeps one: the items in the list's own order, as an
//! album or a playlist draws them, and the order they play in, kept apart as indexes into the list.
//! Shuffle rearranges only the play order, with the item playing first, so turning it off carries
//! on from that item's place in the list; repeat says what the end of the order does. Every change
//! keeps the current item current. Pure: each change answers a new queue.
//!
//! Library logic: this belongs in the media crate, and is kept here until it moves there.

use serde::{Deserialize, Serialize};

use crate::js;
use crate::synth::Seeded;

/// How a queue repeats: not at all, the whole queue, or the one item playing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepeatMode {
    #[default]
    Off,
    All,
    One,
}

/// The modes in the order a repeat button steps through them.
pub const REPEAT_MODES: [RepeatMode; 3] = [RepeatMode::Off, RepeatMode::All, RepeatMode::One];

impl RepeatMode {
    /// The mode after this one: off, then the whole queue, then the one item, then off again.
    pub fn next(self) -> RepeatMode {
        match self {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            RepeatMode::Off => "off",
            RepeatMode::All => "all",
            RepeatMode::One => "one",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Queue<T> {
    /// The items in the list's own order.
    pub items: Vec<T>,
    /// The play order, each entry an index into `items`: the list's own order unless shuffled.
    pub order: Vec<usize>,
    /// Where the current item stands in `order`, or -1 in an empty queue.
    pub at: i64,
    pub shuffle: bool,
    pub repeat: RepeatMode,
}

/// One item of a queue with its index in the list and its position in the play order.
#[derive(Clone, Debug, PartialEq)]
pub struct QueueEntry<'a, T> {
    pub item: &'a T,
    pub index: usize,
    pub position: usize,
}

/// What a shuffle is drawn from: a number, or a text hashed first.
#[derive(Clone, Debug, PartialEq)]
pub enum QueueSeed {
    Number(f64),
    Text(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct QueueOptions {
    /// The index in the list of the item to start on, the first in the play order by default.
    pub start: Option<f64>,
    pub shuffle: Option<bool>,
    pub repeat: Option<RepeatMode>,
    /// What a shuffle is drawn from, so the same seed gives the same order; a random one by default.
    pub seed: Option<QueueSeed>,
    /// A play order given whole, each entry an index into the list, such as one kept from before.
    pub order: Option<Vec<i64>>,
}

/// A generator of numbers from 0 up to 1 that gives the same run for the same seed (mulberry32), so
/// a shuffle can be told again. A text seed is hashed first.
pub fn seeded_random(seed: &QueueSeed) -> Seeded {
    let state = match seed {
        QueueSeed::Text(text) => sokkel_egui_media::seed_hash(text),
        QueueSeed::Number(number) => js::to_uint32(number.floor()),
    };
    Seeded::new(state)
}

/// A generator where the web draws from `Math.random`: seeded from the clock.
fn any_random() -> Seeded {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.subsec_nanos() ^ since.as_secs() as u32);
    Seeded::new(nanos)
}

fn random_of(seed: Option<&QueueSeed>) -> Seeded {
    seed.map_or_else(any_random, seeded_random)
}

/// The indexes 0 up to `count`, in order.
fn in_order(count: usize) -> Vec<usize> {
    (0..count).collect()
}

/// A shuffled play order of `count` items with `first` leading it and the rest in a Fisher-Yates
/// shuffle drawn from `random`: shuffle says what comes after the item playing, not what plays.
pub fn shuffled_order(count: usize, first: f64, random: &mut dyn FnMut() -> f64) -> Vec<usize> {
    if count == 0 {
        return Vec::new();
    }
    let lead = if first.is_finite() {
        first.floor().max(0.0).min((count - 1) as f64) as usize
    } else {
        0
    };
    let mut rest: Vec<usize> = in_order(count).into_iter().filter(|index| *index != lead).collect();
    let mut index = rest.len();
    while index > 1 {
        index -= 1;
        let other = (random() * (index + 1) as f64).floor() as usize;
        rest.swap(index, other);
    }
    let mut order = vec![lead];
    order.extend(rest);
    order
}

/// The order given, when it holds each index from 0 up to `count` once.
fn as_order(order: &[i64], count: usize) -> Option<Vec<usize>> {
    if order.len() != count {
        return None;
    }
    let mut seen = vec![false; count];
    for index in order {
        if *index < 0 || *index as usize >= count || seen[*index as usize] {
            return None;
        }
        seen[*index as usize] = true;
    }
    Some(order.iter().map(|index| *index as usize).collect())
}

fn is_in_order(order: &[usize]) -> bool {
    order.iter().enumerate().all(|(position, index)| *index == position)
}

fn position_of(order: &[usize], index: usize) -> i64 {
    order.iter().position(|one| *one == index).map_or(-1, |at| at as i64)
}

/// A queue of `items`. Shuffled, the play order is drawn from `seed` with the `start` item first,
/// unless `order` gives it whole; a given order that is not one of the items is drawn instead.
pub fn create_queue<T>(items: Vec<T>, options: QueueOptions) -> Queue<T> {
    let count = items.len();
    let repeat = options.repeat.unwrap_or_default();
    if count == 0 {
        return Queue {
            items,
            order: Vec::new(),
            at: -1,
            shuffle: options.shuffle.unwrap_or(false),
            repeat,
        };
    }
    let given = options.order.as_deref().and_then(|order| as_order(order, count));
    let shuffle = options
        .shuffle
        .unwrap_or_else(|| given.as_deref().is_some_and(|order| !is_in_order(order)));
    let start = options
        .start
        .filter(|start| start.is_finite())
        .map(|start| start.floor().max(0.0).min((count - 1) as f64) as usize);
    if shuffle {
        let order = given.unwrap_or_else(|| {
            let mut random = random_of(options.seed.as_ref());
            shuffled_order(count, start.unwrap_or(0) as f64, &mut || random.next_f64())
        });
        let at = start.map_or(0, |start| position_of(&order, start));
        return Queue {
            items,
            order,
            at,
            shuffle,
            repeat,
        };
    }
    Queue {
        items,
        order: in_order(count),
        at: start.unwrap_or(0) as i64,
        shuffle,
        repeat,
    }
}

impl<T: Clone> Queue<T> {
    /// The current item's index in the list, or -1 in an empty queue.
    pub fn index(&self) -> i64 {
        if self.at < 0 {
            return -1;
        }
        self.order.get(self.at as usize).map_or(-1, |index| *index as i64)
    }

    /// The current item, or nothing in an empty queue.
    pub fn item(&self) -> Option<&T> {
        let index = self.index();
        if index < 0 {
            None
        } else {
            self.items.get(index as usize)
        }
    }

    /// The items in the order they play; skip past `at` for the ones to come.
    pub fn ordered(&self) -> Vec<QueueEntry<'_, T>> {
        self.order
            .iter()
            .enumerate()
            .map(|(position, index)| QueueEntry {
                item: &self.items[*index],
                index: *index,
                position,
            })
            .collect()
    }

    fn with_at(&self, at: i64) -> Self {
        Self { at, ..self.clone() }
    }

    /// The queue with the item at `index` in the list current, where it stands in the play order.
    pub fn play(&self, index: usize) -> Self {
        let at = position_of(&self.order, index);
        if at < 0 || at == self.at {
            self.clone()
        } else {
            self.with_at(at)
        }
    }

    /// The queue after a press of next: the item after this one in the play order. At its end the
    /// order starts over while it repeats all or one, and there is nowhere to go when it does not.
    pub fn next(&self) -> Option<Self> {
        if self.at < 0 {
            return None;
        }
        if ((self.at + 1) as usize) < self.order.len() {
            return Some(self.with_at(self.at + 1));
        }
        (self.repeat != RepeatMode::Off).then(|| self.with_at(0))
    }

    /// The queue after a press of previous: the item before this one in the play order. At its
    /// start the order goes round to its end only while it repeats all.
    pub fn previous(&self) -> Option<Self> {
        if self.at < 0 {
            return None;
        }
        if self.at > 0 {
            return Some(self.with_at(self.at - 1));
        }
        (self.repeat == RepeatMode::All).then(|| self.with_at(self.order.len() as i64 - 1))
    }

    /// The queue once the current item has played to its end: the same item again while it repeats
    /// one, the next in the play order, the first again at the end while it repeats all, and
    /// nothing when it is finished.
    pub fn ended(&self) -> Option<Self> {
        if self.at < 0 {
            return None;
        }
        if self.repeat == RepeatMode::One {
            return Some(self.clone());
        }
        if ((self.at + 1) as usize) < self.order.len() {
            return Some(self.with_at(self.at + 1));
        }
        (self.repeat == RepeatMode::All).then(|| self.with_at(0))
    }

    /// The queue shuffled or put back in the list's order, the current item playing on. Shuffled,
    /// the current item leads an order drawn from `seed` (or `random`), unless `order` gives it
    /// whole; unshuffled, the order is the list's and the current item keeps its place in it.
    pub fn shuffled(
        &self,
        shuffle: bool,
        seed: Option<&QueueSeed>,
        order: Option<&[i64]>,
        random: Option<&mut dyn FnMut() -> f64>,
    ) -> Self {
        let count = self.items.len();
        let current = self.index();
        if !shuffle {
            return Self {
                order: in_order(count),
                at: current,
                shuffle,
                ..self.clone()
            };
        }
        if count == 0 {
            return Self {
                shuffle,
                ..self.clone()
            };
        }
        let given = order.and_then(|order| as_order(order, count));
        let order = given.unwrap_or_else(|| match random {
            Some(random) => shuffled_order(count, current as f64, random),
            None => {
                let mut generator = random_of(seed);
                shuffled_order(count, current as f64, &mut || generator.next_f64())
            }
        });
        let at = if current < 0 {
            -1
        } else {
            position_of(&order, current as usize)
        };
        Self {
            order,
            at,
            shuffle,
            ..self.clone()
        }
    }

    /// The queue repeating by `repeat`.
    pub fn repeating(&self, repeat: RepeatMode) -> Self {
        Self { repeat, ..self.clone() }
    }

    /// The queue with `added` put in: right after the current item, in the list and in the play
    /// order, when `next`, or at the end of both. In an empty queue the first added becomes current.
    pub fn add(&self, added: &[T], next: bool) -> Self {
        let count = added.len();
        if count == 0 {
            return self.clone();
        }
        let current = self.index();
        let next = next && current >= 0;
        let place = if next { current as usize + 1 } else { self.items.len() };
        let mut items: Vec<T> = self.items[..place].to_vec();
        items.extend_from_slice(added);
        items.extend_from_slice(&self.items[place..]);
        let at = self.at.max(0);
        if !self.shuffle {
            return Self {
                order: in_order(items.len()),
                items,
                at,
                ..self.clone()
            };
        }
        let moved: Vec<usize> = self
            .order
            .iter()
            .map(|index| if *index >= place { index + count } else { *index })
            .collect();
        let fresh = (0..count).map(|index| place + index);
        let order = if next {
            let cut = (self.at + 1) as usize;
            let mut order = moved[..cut].to_vec();
            order.extend(fresh);
            order.extend_from_slice(&moved[cut..]);
            order
        } else {
            let mut order = moved;
            order.extend(fresh);
            order
        };
        Self {
            items,
            order,
            at,
            ..self.clone()
        }
    }

    /// The queue without the items at `indexes` in the list. The current item stays current; when
    /// it is removed, the item after it in the play order takes its place, or the one before it
    /// at the end.
    pub fn remove(&self, indexes: &[i64]) -> Self {
        let count = self.items.len();
        let gone: std::collections::BTreeSet<usize> = indexes
            .iter()
            .filter(|index| **index >= 0 && (**index as usize) < count)
            .map(|index| *index as usize)
            .collect();
        if gone.is_empty() {
            return self.clone();
        }
        // Each kept index moves down by the removed ones before it.
        let mut shift = Vec::with_capacity(count);
        let mut removed = 0;
        for index in 0..count {
            shift.push(index - removed);
            if gone.contains(&index) {
                removed += 1;
            }
        }
        let items: Vec<T> = self
            .items
            .iter()
            .enumerate()
            .filter(|(index, _)| !gone.contains(index))
            .map(|(_, item)| item.clone())
            .collect();
        let order: Vec<usize> = self
            .order
            .iter()
            .filter(|index| !gone.contains(index))
            .map(|index| shift[*index])
            .collect();
        if order.is_empty() {
            return Self {
                items,
                order,
                at: -1,
                ..self.clone()
            };
        }
        let current = self.index();
        if current < 0 || !gone.contains(&(current as usize)) {
            let at = if current < 0 {
                -1
            } else {
                position_of(&order, shift[current as usize])
            };
            return Self {
                items,
                order,
                at,
                ..self.clone()
            };
        }
        let before = self.order[..self.at.max(0) as usize]
            .iter()
            .filter(|index| !gone.contains(index))
            .count();
        let at = before.min(order.len() - 1) as i64;
        Self {
            items,
            order,
            at,
            ..self.clone()
        }
    }

    /// The queue with the item at position `from` in the play order moved to position `to`, as a
    /// view of what comes next reorders it. In the list's order the list moves with it; shuffled,
    /// only the play order does. The current item stays current.
    pub fn moved(&self, from: usize, to: usize) -> Self {
        let count = self.order.len();
        if from == to || from >= count || to >= count {
            return self.clone();
        }
        let current = self.index();
        if self.shuffle {
            let order = moved(&self.order, from, to);
            let at = if current < 0 {
                -1
            } else {
                position_of(&order, current as usize)
            };
            return Self {
                order,
                at,
                ..self.clone()
            };
        }
        let places = moved(&in_order(count), from, to);
        let items = places.iter().map(|index| self.items[*index].clone()).collect();
        let at = if current < 0 {
            -1
        } else {
            position_of(&places, current as usize)
        };
        Self {
            items,
            at,
            ..self.clone()
        }
    }
}

/// `list` with the entry at `from` taken out and put back at `to`.
pub fn moved<V: Clone>(list: &[V], from: usize, to: usize) -> Vec<V> {
    let mut next = list.to_vec();
    let entry = next.remove(from);
    next.insert(to.min(next.len()), entry);
    next
}
