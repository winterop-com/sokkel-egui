//! A queue, after sokkel-ui-media's `queue`: the items in the list's own order, as an album or a
//! playlist draws them, and the order they play in, kept apart as indexes into the list. Shuffle
//! rearranges only the play order, with the item playing first, so turning it off carries on from
//! that item's place in the list; repeat says what the end of the order does. Every change keeps
//! the current item current. Pure, so an app's own store and the stories share the same arithmetic.

use crate::artwork::seed_hash;

/// How a queue repeats: not at all, the whole queue, or the one item playing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }
}

/// What a shuffle is drawn from, so the same seed gives the same order.
#[derive(Clone, Debug, PartialEq)]
pub enum Seed {
    Number(u32),
    Text(String),
}

impl From<u32> for Seed {
    fn from(n: u32) -> Self {
        Self::Number(n)
    }
}

impl From<&str> for Seed {
    fn from(s: &str) -> Self {
        Self::Text(s.to_owned())
    }
}

impl From<String> for Seed {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}

/// A generator of numbers from 0 up to 1 that gives the same run for the same seed (mulberry32), so
/// a shuffle can be told again. A text seed is hashed first.
pub fn seeded_random(seed: impl Into<Seed>) -> impl FnMut() -> f64 {
    let mut state = match seed.into() {
        Seed::Number(n) => n,
        Seed::Text(s) => seed_hash(&s),
    };
    move || {
        state = state.wrapping_add(0x6d2b_79f5);
        let mut mixed = (state ^ (state >> 15)).wrapping_mul(1 | state);
        mixed ^= mixed.wrapping_add((mixed ^ (mixed >> 7)).wrapping_mul(61 | mixed));
        f64::from(mixed ^ (mixed >> 14)) / 4_294_967_296.0
    }
}

/// A generator seeded from the clock, for a shuffle that need not be told again.
fn clock_random() -> impl FnMut() -> f64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u32)
        .unwrap_or(0x9e37_79b9);
    seeded_random(nanos)
}

/// A shuffled play order of `count` items with `first` leading it and the rest in a Fisher-Yates
/// shuffle drawn from `random`: shuffle says what comes after the item playing, not what plays.
pub fn shuffled_order(count: usize, first: usize, random: &mut dyn FnMut() -> f64) -> Vec<usize> {
    if count == 0 {
        return Vec::new();
    }
    let lead = first.min(count - 1);
    let mut rest: Vec<usize> = (0..count).filter(|i| *i != lead).collect();
    for index in (1..rest.len()).rev() {
        let other = ((random() * (index + 1) as f64).floor() as usize).min(index);
        rest.swap(index, other);
    }
    let mut order = Vec::with_capacity(count);
    order.push(lead);
    order.extend(rest);
    order
}

/// Whether `order` holds each index from 0 up to `count` once.
fn is_order(order: &[usize], count: usize) -> bool {
    if order.len() != count {
        return false;
    }
    let mut seen = vec![false; count];
    for &index in order {
        if index >= count || seen[index] {
            return false;
        }
        seen[index] = true;
    }
    true
}

fn is_in_order(order: &[usize]) -> bool {
    order.iter().enumerate().all(|(position, index)| *index == position)
}

/// The random a seed draws from, or the clock's without one.
fn random_of(seed: Option<Seed>) -> Box<dyn FnMut() -> f64> {
    match seed {
        Some(seed) => Box::new(seeded_random(seed)),
        None => Box::new(clock_random()),
    }
}

/// How a new queue starts.
#[derive(Clone, Debug, Default)]
pub struct QueueOptions {
    /// The index in the list of the item to start on, the first in the play order by default.
    pub start: Option<usize>,
    /// Whether it plays shuffled; by default, shuffled when `order` is given out of the list's order.
    pub shuffle: Option<bool>,
    pub repeat: RepeatMode,
    /// What a shuffle is drawn from, so the same seed gives the same order; the clock by default.
    pub seed: Option<Seed>,
    /// A play order given whole, each entry an index into the list, such as one kept from before.
    pub order: Option<Vec<usize>>,
}

/// How [`Queue::shuffled`] draws its order.
#[derive(Clone, Debug, Default)]
pub struct ShuffleOptions {
    pub seed: Option<Seed>,
    /// A shuffled order given whole.
    pub order: Option<Vec<usize>>,
}

/// Where [`Queue::add`] puts what is added: right after the current item, or at the end.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QueueWhere {
    Next,
    #[default]
    End,
}

/// One item of a queue with its index in the list and its position in the play order.
#[derive(Clone, Debug, PartialEq)]
pub struct QueueEntry<T> {
    pub item: T,
    pub index: usize,
    pub position: usize,
}

/// The items in the list's own order and the order they play in.
#[derive(Clone, Debug, PartialEq)]
pub struct Queue<T> {
    /// The items in the list's own order.
    pub items: Vec<T>,
    /// The play order, each entry an index into `items`: the list's own order unless shuffled.
    pub order: Vec<usize>,
    /// Where the current item stands in `order`; none in an empty queue.
    pub at: Option<usize>,
    pub shuffle: bool,
    pub repeat: RepeatMode,
}

impl<T: Clone> Queue<T> {
    /// A queue of `items`. Shuffled, the play order is drawn from `seed` with the `start` item
    /// first, unless `order` gives it whole; a given order that is not one of the items is drawn
    /// instead.
    pub fn new(items: Vec<T>, options: QueueOptions) -> Self {
        let count = items.len();
        let repeat = options.repeat;
        if count == 0 {
            return Self {
                items,
                order: Vec::new(),
                at: None,
                shuffle: options.shuffle.unwrap_or(false),
                repeat,
            };
        }
        let given = options.order.filter(|order| is_order(order, count));
        let shuffle = options
            .shuffle
            .unwrap_or_else(|| given.as_ref().is_some_and(|g| !is_in_order(g)));
        let start = options.start.map(|s| s.min(count - 1));
        if shuffle {
            let order = given.unwrap_or_else(|| {
                let mut random = random_of(options.seed);
                shuffled_order(count, start.unwrap_or(0), &mut random)
            });
            let at = start.map_or(0, |s| order.iter().position(|i| *i == s).unwrap_or(0));
            return Self {
                items,
                order,
                at: Some(at),
                shuffle,
                repeat,
            };
        }
        Self {
            items,
            order: (0..count).collect(),
            at: Some(start.unwrap_or(0)),
            shuffle,
            repeat,
        }
    }

    /// The current item's index in the list; none in an empty queue.
    pub fn index(&self) -> Option<usize> {
        self.at.and_then(|at| self.order.get(at).copied())
    }

    /// The current item; none in an empty queue.
    pub fn item(&self) -> Option<&T> {
        self.index().and_then(|i| self.items.get(i))
    }

    /// The items in the order they play, as a view of what comes next draws them; skip past `at`
    /// for the ones to come.
    pub fn ordered(&self) -> Vec<QueueEntry<T>> {
        self.order
            .iter()
            .enumerate()
            .map(|(position, &index)| QueueEntry {
                item: self.items[index].clone(),
                index,
                position,
            })
            .collect()
    }

    /// What plays after the current item, in the order it plays.
    pub fn up_next(&self) -> Vec<QueueEntry<T>> {
        let skip = self.at.map_or(0, |at| at + 1);
        self.ordered().into_iter().skip(skip).collect()
    }

    fn at(&self, at: usize) -> Self {
        Self {
            at: Some(at),
            ..self.clone()
        }
    }

    /// The queue with the item at `index` in the list current, where it stands in the play order.
    pub fn play(&self, index: usize) -> Self {
        match self.order.iter().position(|i| *i == index) {
            Some(at) if Some(at) != self.at => self.at(at),
            _ => self.clone(),
        }
    }

    /// The queue after a press of next: the item after this one in the play order. At its end the
    /// order starts over while it repeats all or one, and there is nowhere to go when it does not.
    /// Repeating one item does not hold next on it: a press means enough of this one.
    pub fn next(&self) -> Option<Self> {
        let at = self.at?;
        if at + 1 < self.order.len() {
            return Some(self.at(at + 1));
        }
        (self.repeat != RepeatMode::Off).then(|| self.at(0))
    }

    /// The queue after a press of previous: the item before this one in the play order. At its
    /// start the order goes round to its end only while it repeats all; otherwise there is nowhere
    /// to go, which a player takes as starting this item again.
    pub fn previous(&self) -> Option<Self> {
        let at = self.at?;
        if at > 0 {
            return Some(self.at(at - 1));
        }
        (self.repeat == RepeatMode::All).then(|| self.at(self.order.len() - 1))
    }

    /// The queue once the current item has played to its end: the same item again while it
    /// repeats one, the next in the play order, the first again at the end while it repeats all,
    /// and nothing when it is finished.
    pub fn ended(&self) -> Option<Self> {
        let at = self.at?;
        if self.repeat == RepeatMode::One {
            return Some(self.clone());
        }
        if at + 1 < self.order.len() {
            return Some(self.at(at + 1));
        }
        (self.repeat == RepeatMode::All).then(|| self.at(0))
    }

    /// The queue shuffled or put back in the list's order, the current item playing on. Shuffled,
    /// the current item leads an order drawn from the seed, unless `order` gives it whole;
    /// unshuffled, the order is the list's and the current item keeps its place in it.
    pub fn shuffled(&self, shuffle: bool, options: ShuffleOptions) -> Self {
        let count = self.items.len();
        let current = self.index();
        if !shuffle {
            return Self {
                order: (0..count).collect(),
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
        let lead = current.unwrap_or(0);
        let order = options.order.filter(|o| is_order(o, count)).unwrap_or_else(|| {
            let mut random = random_of(options.seed);
            shuffled_order(count, lead, &mut random)
        });
        let at = order.iter().position(|i| *i == lead);
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
    /// order, for [`QueueWhere::Next`], or at the end of both. In an empty queue the first added
    /// becomes current.
    pub fn add(&self, added: &[T], place: QueueWhere) -> Self {
        let count = added.len();
        if count == 0 {
            return self.clone();
        }
        let current = self.index();
        let next = place == QueueWhere::Next && current.is_some();
        let at_list = match (next, current) {
            (true, Some(c)) => c + 1,
            _ => self.items.len(),
        };
        let mut items = self.items[..at_list].to_vec();
        items.extend_from_slice(added);
        items.extend_from_slice(&self.items[at_list..]);
        // what is added goes after the current item, so it keeps its place; an empty queue starts
        // on the first added
        let at = Some(self.at.unwrap_or(0));
        if !self.shuffle {
            return Self {
                order: (0..items.len()).collect(),
                items,
                at,
                ..self.clone()
            };
        }
        let moved: Vec<usize> = self
            .order
            .iter()
            .map(|&i| if i >= at_list { i + count } else { i })
            .collect();
        let fresh = (0..count).map(|i| at_list + i);
        let order = if next {
            let split = self.at.map_or(0, |a| a + 1);
            let mut order = moved[..split].to_vec();
            order.extend(fresh);
            order.extend_from_slice(&moved[split..]);
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
    /// it is removed, the item after it in the play order takes its place, or the one before it at
    /// the end.
    pub fn remove(&self, indexes: &[usize]) -> Self {
        let len = self.items.len();
        let gone: std::collections::HashSet<usize> = indexes.iter().copied().filter(|i| *i < len).collect();
        if gone.is_empty() {
            return self.clone();
        }
        // each kept index moves down by the removed ones before it
        let mut shift = Vec::with_capacity(len);
        let mut removed = 0;
        for index in 0..len {
            shift.push(index - removed);
            if gone.contains(&index) {
                removed += 1;
            }
        }
        let items: Vec<T> = self
            .items
            .iter()
            .enumerate()
            .filter(|(i, _)| !gone.contains(i))
            .map(|(_, item)| item.clone())
            .collect();
        let order: Vec<usize> = self
            .order
            .iter()
            .filter(|i| !gone.contains(i))
            .map(|&i| shift[i])
            .collect();
        if order.is_empty() {
            return Self {
                items,
                order,
                at: None,
                ..self.clone()
            };
        }
        let at = match self.index() {
            Some(current) if !gone.contains(&current) => order.iter().position(|i| *i == shift[current]),
            _ => {
                let before = self.order[..self.at.unwrap_or(0)]
                    .iter()
                    .filter(|i| !gone.contains(i))
                    .count();
                Some(before.min(order.len() - 1))
            }
        };
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
            let at = current.and_then(|c| order.iter().position(|i| *i == c));
            return Self {
                order,
                at,
                ..self.clone()
            };
        }
        let places = moved(&(0..count).collect::<Vec<_>>(), from, to);
        Self {
            items: places.iter().map(|&i| self.items[i].clone()).collect(),
            at: current.and_then(|c| places.iter().position(|i| *i == c)),
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

#[cfg(test)]
mod tests {
    use super::*;

    const ITEMS: [&str; 5] = ["a", "b", "c", "d", "e"];

    fn items() -> Vec<&'static str> {
        ITEMS.to_vec()
    }

    fn queue(options: QueueOptions) -> Queue<&'static str> {
        Queue::new(items(), options)
    }

    fn start(at: usize) -> QueueOptions {
        QueueOptions {
            start: Some(at),
            ..Default::default()
        }
    }

    fn given(order: [usize; 5], at: usize) -> QueueOptions {
        QueueOptions {
            order: Some(order.to_vec()),
            start: Some(at),
            ..Default::default()
        }
    }

    /// The items in the order they play.
    fn played(queue: &Queue<&str>) -> String {
        queue.order.iter().map(|&i| queue.items[i]).collect()
    }

    fn joined(queue: &Queue<&str>) -> String {
        queue.items.concat()
    }

    #[test]
    fn repeat_steps_from_off_to_all_to_one_and_back() {
        assert_eq!(RepeatMode::Off.next(), RepeatMode::All);
        assert_eq!(RepeatMode::All.next(), RepeatMode::One);
        assert_eq!(RepeatMode::One.next(), RepeatMode::Off);
    }

    #[test]
    fn a_seeded_shuffle_gives_the_same_run_for_the_same_seed() {
        let mut first = seeded_random("harbor");
        let mut second = seeded_random("harbor");
        for _ in 0..4 {
            let (one, two) = (first(), second());
            assert_eq!(one, two);
            assert!((0.0..1.0).contains(&one));
        }
        assert_ne!(seeded_random("juniper")(), seeded_random("harbor")());
    }

    #[test]
    fn a_seeded_shuffle_puts_the_given_item_first_and_every_other_once_after_it() {
        let order = shuffled_order(8, 5, &mut seeded_random(7));
        assert_eq!(order[0], 5);
        let mut sorted = order.clone();
        sorted.sort();
        assert_eq!(sorted, (0..8).collect::<Vec<_>>());
        assert_eq!(shuffled_order(8, 5, &mut seeded_random(7)), order);
        assert!(shuffled_order(0, 0, &mut seeded_random(1)).is_empty());
        assert_eq!(shuffled_order(3, 9, &mut seeded_random(1))[0], 2);
    }

    #[test]
    fn a_new_queue_plays_the_list_in_its_own_order_from_the_start() {
        let q = queue(start(2));
        assert_eq!(q.order, [0, 1, 2, 3, 4]);
        assert_eq!(q.index(), Some(2));
        assert_eq!(q.item(), Some(&"c"));
    }

    #[test]
    fn a_new_queue_shuffled_leads_with_the_start_and_leaves_the_list_alone() {
        let options = || QueueOptions {
            start: Some(3),
            shuffle: Some(true),
            seed: Some("harbor".into()),
            ..Default::default()
        };
        let q = queue(options());
        assert_eq!(q.at, Some(0));
        assert_eq!(q.item(), Some(&"d"));
        assert_eq!(q.items, ITEMS);
        assert_eq!(queue(options()).order, q.order);
    }

    #[test]
    fn a_new_queue_takes_an_order_given_whole_and_draws_one_in_place_of_an_order_that_is_not() {
        let kept = queue(given([4, 2, 0, 1, 3], 0));
        assert!(kept.shuffle);
        assert_eq!(kept.at, Some(2));
        let plain = queue(QueueOptions {
            order: Some(vec![0, 1, 2, 3, 4]),
            ..Default::default()
        });
        assert!(!plain.shuffle);
        let broken = queue(QueueOptions {
            order: Some(vec![0, 0, 1]),
            shuffle: Some(true),
            start: Some(1),
            seed: Some(1.into()),
            ..Default::default()
        });
        assert_eq!(broken.order.len(), 5);
        assert_eq!(broken.item(), Some(&"b"));
    }

    #[test]
    fn a_new_queue_starts_from_the_first_item_without_a_start() {
        assert_eq!(queue(QueueOptions::default()).at, Some(0));
        let shuffled = Queue::new(
            vec!["a", "b"],
            QueueOptions {
                shuffle: Some(true),
                seed: Some(1.into()),
                ..Default::default()
            },
        );
        let mut sorted = shuffled.order.clone();
        sorted.sort();
        assert_eq!(sorted, [0, 1]);
        assert_eq!(shuffled.at, Some(0));
    }

    #[test]
    fn an_empty_queue_holds_nothing() {
        let q = Queue::<&str>::new(Vec::new(), QueueOptions::default());
        assert_eq!(q.at, None);
        assert_eq!(q.item(), None);
        assert!(q.next().is_none());
        assert!(q.previous().is_none());
        assert!(q.ended().is_none());
    }

    #[test]
    fn steps_along_the_play_order() {
        let q = queue(QueueOptions {
            order: Some(vec![2, 0, 4, 1, 3]),
            ..Default::default()
        });
        assert_eq!(q.next().unwrap().item(), Some(&"a"));
        assert_eq!(q.next().unwrap().next().unwrap().item(), Some(&"e"));
        assert_eq!(q.next().unwrap().previous().unwrap().item(), Some(&"c"));
    }

    #[test]
    fn stops_at_the_ends_without_repeat() {
        let last = queue(start(4));
        assert!(last.next().is_none());
        assert!(last.ended().is_none());
        assert!(queue(QueueOptions::default()).previous().is_none());
    }

    #[test]
    fn goes_round_the_order_while_it_repeats_all() {
        let last = queue(QueueOptions {
            start: Some(4),
            repeat: RepeatMode::All,
            ..Default::default()
        });
        assert_eq!(last.next().unwrap().index(), Some(0));
        assert_eq!(last.ended().unwrap().index(), Some(0));
        let first = queue(QueueOptions {
            repeat: RepeatMode::All,
            ..Default::default()
        });
        assert_eq!(first.previous().unwrap().index(), Some(4));
    }

    fn one(at: usize) -> QueueOptions {
        QueueOptions {
            start: Some(at),
            repeat: RepeatMode::One,
            ..Default::default()
        }
    }

    #[test]
    fn plays_one_item_again_at_its_end_but_moves_on_at_a_press_of_next() {
        let q = queue(one(1));
        assert_eq!(q.ended().unwrap(), q);
        assert_eq!(q.next().unwrap().index(), Some(2));
        assert_eq!(queue(one(4)).next().unwrap().index(), Some(0));
    }

    #[test]
    fn repeating_one_steps_back_within_the_order_but_not_round_from_the_first_item() {
        assert_eq!(queue(one(2)).previous().unwrap().index(), Some(1));
        assert!(queue(one(0)).previous().is_none());
        let shuffled = queue(QueueOptions {
            repeat: RepeatMode::One,
            ..given([2, 0, 4, 1, 3], 2)
        });
        assert!(shuffled.previous().is_none());
    }

    #[test]
    fn jumps_to_an_item_where_it_stands_in_the_play_order() {
        let q = queue(QueueOptions {
            order: Some(vec![2, 0, 4, 1, 3]),
            ..Default::default()
        });
        let jumped = q.play(1);
        assert_eq!(jumped.at, Some(3));
        assert_eq!(jumped.next().unwrap().item(), Some(&"d"));
        assert_eq!(q.play(9), q);
    }

    #[test]
    fn keeps_the_current_item_and_puts_it_first_when_shuffled() {
        let q = queue(start(3)).shuffled(
            true,
            ShuffleOptions {
                seed: Some(4.into()),
                order: None,
            },
        );
        assert_eq!(q.at, Some(0));
        assert_eq!(q.item(), Some(&"d"));
        assert_eq!(q.items, ITEMS);
    }

    #[test]
    fn carries_on_from_the_item_in_the_list_once_unshuffled() {
        let back = queue(given([2, 0, 4, 1, 3], 4)).shuffled(false, ShuffleOptions::default());
        assert_eq!(back.order, [0, 1, 2, 3, 4]);
        assert_eq!(back.item(), Some(&"e"));
        assert_eq!(back.at, Some(4));
    }

    #[test]
    fn takes_a_shuffled_order_given_whole() {
        let q = queue(start(1)).shuffled(
            true,
            ShuffleOptions {
                seed: None,
                order: Some(vec![3, 1, 0, 2, 4]),
            },
        );
        assert_eq!(q.at, Some(1));
        assert_eq!(q.item(), Some(&"b"));
    }

    #[test]
    fn changes_how_it_repeats_and_nothing_else() {
        let q = queue(QueueOptions::default());
        assert_eq!(q.repeating(RepeatMode::Off), q);
        let all = q.repeating(RepeatMode::All);
        assert_eq!(all.repeat, RepeatMode::All);
        assert_eq!(all.order, q.order);
        assert_eq!(all.at, q.at);
    }

    #[test]
    fn adding_in_order_puts_items_after_the_current_one_or_at_the_end() {
        let q = queue(start(1));
        let next = q.add(&["x", "y"], QueueWhere::Next);
        assert_eq!(joined(&next), "abxycde");
        assert_eq!(next.item(), Some(&"b"));
        assert_eq!(next.next().unwrap().item(), Some(&"x"));
        assert_eq!(joined(&q.add(&["x"], QueueWhere::End)), "abcdex");
        assert_eq!(q.add(&[], QueueWhere::End), q);
    }

    #[test]
    fn adding_shuffled_plays_the_added_next_or_last_keeping_the_current_item() {
        let q = queue(given([2, 0, 4, 1, 3], 0));
        let next = q.add(&["x"], QueueWhere::Next);
        assert_eq!(joined(&next), "axbcde");
        assert_eq!(played(&next), "caxebd");
        assert_eq!(next.item(), Some(&"a"));
        let last = q.add(&["x"], QueueWhere::End);
        assert_eq!(played(&last), "caebdx");
        assert_eq!(last.item(), Some(&"a"));
    }

    #[test]
    fn adding_starts_an_empty_queue_on_the_first_added() {
        let q = Queue::new(Vec::new(), QueueOptions::default()).add(&["x", "y"], QueueWhere::End);
        assert_eq!(q.at, Some(0));
        assert_eq!(q.item(), Some(&"x"));
    }

    #[test]
    fn removing_keeps_the_current_item_current() {
        let q = queue(start(3)).remove(&[0, 4]);
        assert_eq!(joined(&q), "bcd");
        assert_eq!(q.item(), Some(&"d"));
        assert_eq!(q.order, [0, 1, 2]);
    }

    #[test]
    fn removing_the_current_item_moves_on_to_the_next_in_the_play_order() {
        let removed = queue(given([2, 0, 4, 1, 3], 0)).remove(&[0]);
        assert_eq!(played(&removed), "cebd");
        assert_eq!(removed.item(), Some(&"e"));
        let last_gone = queue(start(4)).remove(&[4]);
        assert_eq!(last_gone.item(), Some(&"d"));
    }

    #[test]
    fn removing_everything_holds_nothing_and_unknown_indexes_are_ignored() {
        let q = queue(QueueOptions::default());
        assert_eq!(q.remove(&[0, 1, 2, 3, 4]).at, None);
        assert_eq!(q.remove(&[9]), q);
    }

    #[test]
    fn moving_in_order_moves_the_list_with_the_play_order() {
        let q = queue(start(2)).moved(4, 1);
        assert_eq!(joined(&q), "aebcd");
        assert_eq!(q.item(), Some(&"c"));
        assert_eq!(q.at, Some(3));
    }

    #[test]
    fn moving_shuffled_moves_only_the_play_order() {
        let m = queue(given([2, 0, 4, 1, 3], 0)).moved(4, 2);
        assert_eq!(m.items, ITEMS);
        assert_eq!(played(&m), "cadeb");
        assert_eq!(m.item(), Some(&"a"));
    }

    #[test]
    fn moving_the_current_item_keeps_it_current() {
        let q = queue(start(0)).moved(0, 3);
        assert_eq!(joined(&q), "bcdae");
        assert_eq!(q.item(), Some(&"a"));
        assert_eq!(q.moved(1, 9), q);
    }

    #[test]
    fn lists_what_plays_with_its_place_in_the_list_and_in_the_order() {
        let q = queue(given([2, 0, 4, 1, 3], 0));
        assert_eq!(
            q.up_next(),
            [
                QueueEntry {
                    item: "e",
                    index: 4,
                    position: 2
                },
                QueueEntry {
                    item: "b",
                    index: 1,
                    position: 3
                },
                QueueEntry {
                    item: "d",
                    index: 3,
                    position: 4
                },
            ]
        );
    }
}
