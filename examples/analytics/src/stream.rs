//! Tidevann's analytics server, faked in the app, as the web's `backend.ts`: the changes a person
//! makes, and a live stream that sends a sign-up, an order of seats, an upgrade or a team leaving
//! every second or two while the app is open. Its random source is seeded from the moment the app
//! starts, and each event lands when the web's would: after its wait, and the backend's 150 ms for
//! each thing it keeps.

use crate::days::day_of;
use crate::model::{Change, ChangeKind, Pace, Person, Plan, TeamRecord, TeamStatus};
use crate::seed::{Random, hash, pick, place_of};
use crate::world::{Kept, World, standing};

/// How long the backend takes to answer, in milliseconds.
pub const LATENCY: i64 = 150;

/// How long the world waits for more changes before it is built again, in milliseconds.
pub const SETTLE: i64 = 20;

/// The most live changes the demo keeps; the stream stops there until the demo data is reset.
pub const LIVE_LIMIT: usize = 5000;

/// How long the stream waits between events at a pace, in milliseconds: the shortest and longest.
pub fn wait_of(pace: Pace) -> Option<(f64, f64)> {
    match pace {
        Pace::Fast => Some((1000.0, 2000.0)),
        Pace::Slow => Some((5000.0, 10_000.0)),
        Pace::Off => None,
    }
}

/// Whether the stream is sending, and why not when it is not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StreamState {
    #[default]
    Live,
    Off,
    Hidden,
    Full,
}

/// What the stream sent: the change, the team's name and plan before, and what it billed at once.
#[derive(Clone, Debug, PartialEq)]
pub struct StreamEvent {
    pub change: Change,
    pub team: String,
    /// What it billed at once, in dollars: a move to a paid plan, or seats bought.
    pub billed: i64,
    /// What the team paid a month before a churn, for its note.
    pub paid_before: Option<(Plan, i64)>,
}

/// `value` in base 36, as JavaScript's `toString(36)`.
fn base36(mut value: i64) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".into();
    }
    let mut out = Vec::new();
    while value > 0 {
        out.push(DIGITS[(value % 36) as usize]);
        value /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// The stream and the ids it gives out.
pub struct Stream {
    random: Random,
    made: u64,
    /// When the next event is due.
    pub next: Option<i64>,
    pub state: StreamState,
}

const EVENT_WEIGHT: [(Event, f64); 4] = [
    (Event::Signup, 42.0),
    (Event::Order, 33.0),
    (Event::Upgrade, 15.0),
    (Event::Churn, 10.0),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Event {
    Signup,
    Order,
    Upgrade,
    Churn,
}

impl Stream {
    /// The stream as the app starts at `now`, its first wait drawn.
    pub fn new(now: i64, pace: Pace) -> Self {
        let mut stream = Self {
            random: Random::new(hash(&[now, 1])),
            made: 0,
            next: None,
            state: StreamState::Live,
        };
        stream.wait(now, pace);
        stream
    }

    /// Draws the wait before the next event, from `now`.
    pub fn wait(&mut self, now: i64, pace: Pace) {
        let (low, high) = wait_of(pace).unwrap_or((1000.0, 1000.0));
        self.next = Some(now + (low + self.random.draw() * (high - low)) as i64);
    }

    fn change_id(&mut self, at: i64) -> String {
        self.made += 1;
        format!("c-{}-{}", base36(at), self.made)
    }

    /// A person's move of a team to a plan and seats: an upgrade, a downgrade, or seats.
    pub fn change_plan(
        &mut self,
        team: &crate::model::Team,
        plan: Plan,
        seats: i64,
        by: Option<Person>,
        at: i64,
        live: bool,
    ) -> Change {
        let was = standing(team, at);
        let kind = if was.status == TeamStatus::Churned || plan.price() > was.plan.price() {
            ChangeKind::Upgrade
        } else if plan.price() < was.plan.price() {
            ChangeKind::Downgrade
        } else {
            ChangeKind::Seats
        };
        Change {
            id: self.change_id(at),
            team: team.id.clone(),
            kind,
            at,
            plan,
            seats,
            by,
            live,
        }
    }

    /// Ends a team's paid plan: it leaves, keeping the plan it had.
    pub fn cancel_plan(&mut self, team: &crate::model::Team, by: Option<Person>, at: i64, live: bool) -> Change {
        let was = standing(team, at);
        Change {
            id: self.change_id(at),
            team: team.id.clone(),
            kind: ChangeKind::Churn,
            at,
            plan: was.plan,
            seats: 0,
            by,
            live,
        }
    }

    /// A team at random that `keep` takes, from the latest `recent` teams.
    fn any_of(&mut self, world: &World, keep: impl Fn(&crate::model::Team) -> bool, recent: usize) -> Option<usize> {
        let from = world.teams.len().saturating_sub(recent);
        for _ in 0..80 {
            let at = from + (self.random.draw() * (world.teams.len() - from) as f64).floor() as usize;
            if let Some(team) = world.teams.get(at)
                && keep(team)
            {
                return Some(at);
            }
        }
        None
    }

    /// The stream's tick at `at`: where it stands, and the event it sends, kept in `kept`.
    pub fn tick(&mut self, world: &World, kept: &mut Kept, pace: Pace, at: i64) -> Option<StreamEvent> {
        if wait_of(pace).is_none() {
            self.state = StreamState::Off;
            return None;
        }
        if world.live_count >= LIVE_LIMIT {
            self.state = StreamState::Full;
            return None;
        }
        self.state = StreamState::Live;
        self.send(world, kept, at)
    }

    fn send(&mut self, world: &World, kept: &mut Kept, at: i64) -> Option<StreamEvent> {
        let kind = pick(&mut self.random, &EVENT_WEIGHT, |(_, w)| w).0;
        let on_plan = |team: &crate::model::Team, plans: &[Plan]| {
            let now = standing(team, at);
            now.status == TeamStatus::Active && plans.contains(&now.plan)
        };
        match kind {
            Event::Signup => {
                let place = place_of(&mut self.random);
                let today = day_of(at);
                self.made += 1;
                let id = format!("live-{}-{}", base36(at), self.made);
                let mut names = world.names.clone();
                for record in &kept.teams {
                    names.keep(&record.name);
                }
                let name = names.take((self.random.draw() * 1_000_000.0).floor() as u64);
                let project = (self.random.draw() < 0.7).then_some(today);
                let invited = (self.random.draw() < 0.4).then_some(today);
                let record = TeamRecord {
                    id: id.clone(),
                    name: name.clone(),
                    country: place.name.to_owned(),
                    region: place.region,
                    owner: place.region.owner(),
                    created: at,
                    project,
                    invited,
                    dormant: None,
                    live: true,
                };
                let seats = 1 + (self.random.draw() * 4.0).floor() as i64;
                let change = Change {
                    id: self.change_id(at),
                    team: id,
                    kind: ChangeKind::Signup,
                    at,
                    plan: Plan::Free,
                    seats,
                    by: None,
                    live: true,
                };
                kept.teams.push(record);
                kept.changes.push(change.clone());
                Some(StreamEvent {
                    change,
                    team: name,
                    billed: 0,
                    paid_before: None,
                })
            }
            Event::Upgrade => {
                let moving = if self.random.draw() < 0.25 {
                    self.any_of(world, |team| on_plan(team, &[Plan::Team]), usize::MAX)
                } else {
                    self.any_of(
                        world,
                        |team| on_plan(team, &[Plan::Free]) && team.invited.is_some(),
                        4000,
                    )
                }?;
                let team = &world.teams[moving];
                let was = standing(team, at);
                let plan = if was.plan == Plan::Team || self.random.draw() < 0.2 {
                    Plan::Business
                } else {
                    Plan::Team
                };
                let seats = if plan == Plan::Business {
                    was.seats.max(8 + (self.random.draw() * 16.0).floor() as i64)
                } else {
                    3 + (self.random.draw() * 8.0).floor() as i64
                };
                let change = self.change_plan(team, plan, seats, None, at, true);
                kept.changes.push(change.clone());
                Some(StreamEvent {
                    change,
                    team: team.name.clone(),
                    billed: (plan.price() * seats - was.mrr).max(0),
                    paid_before: None,
                })
            }
            Event::Order | Event::Churn => {
                let at_team = self.any_of(world, |one| on_plan(one, &[Plan::Team, Plan::Business]), usize::MAX)?;
                let team = &world.teams[at_team];
                let was = standing(team, at);
                if kind == Event::Order {
                    let added = if was.plan == Plan::Team {
                        1 + (self.random.draw() * 3.0).floor() as i64
                    } else {
                        1 + (self.random.draw() * 5.0).floor() as i64
                    };
                    let change = self.change_plan(team, was.plan, was.seats + added, None, at, true);
                    kept.changes.push(change.clone());
                    return Some(StreamEvent {
                        change,
                        team: team.name.clone(),
                        billed: crate::seed::round(was.plan.price() as f64 * added as f64 * 0.5) as i64,
                        paid_before: None,
                    });
                }
                let before = team.changes.last().map(|c| (c.plan, c.plan.price() * c.seats));
                let change = self.cancel_plan(team, None, at, true);
                kept.changes.push(change.clone());
                Some(StreamEvent {
                    change,
                    team: team.name.clone(),
                    billed: 0,
                    paid_before: before,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ids_read_as_javascript_writes_them() {
        assert_eq!(super::base36(1_794_992_401_234), "mwlw4oya");
    }
}
