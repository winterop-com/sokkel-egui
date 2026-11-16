//! The board's live world, after sokkel-ui's status example (`live.ts`): each service's readings
//! and each site's round trips, taken every interval and drifting gently around their resting
//! values; things going wrong (scenarios) rising, holding and falling back, felt by a service and
//! in part by what calls on it; and each one's health from its latest readings.

use std::collections::HashMap;

use crate::data::{SERVICES, SITES, Site};

/// How many readings each chart keeps.
pub const KEEP: usize = 60;

/// One reading of a service.
#[derive(Clone, Copy, Debug)]
pub struct Reading {
    /// When it was taken, in milliseconds of the board's clock.
    pub at: f64,
    /// Median latency, in milliseconds.
    pub p50: f64,
    /// 95th percentile latency, in milliseconds.
    pub p95: f64,
    /// Failed requests, in percent.
    pub errors: f64,
    /// Requests per second.
    pub load: f64,
}

/// A service's state, from the best to the worst, maintenance aside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health {
    Operational,
    Maintenance,
    Degraded,
    Outage,
}

impl Health {
    pub fn words(self) -> &'static str {
        match self {
            Health::Operational => "Operational",
            Health::Degraded => "Degraded",
            Health::Outage => "Outage",
            Health::Maintenance => "Maintenance",
        }
    }

    /// The token its color is, as the web's `HEALTH_COLOR`.
    pub fn color(self) -> &'static str {
        match self {
            Health::Operational => "good",
            Health::Degraded => "warning",
            Health::Outage => "critical",
            Health::Maintenance => "info",
        }
    }

    /// The tone its trend line is drawn in.
    pub fn tone(self) -> &'static str {
        match self {
            Health::Maintenance => "muted-foreground",
            other => other.color(),
        }
    }
}

/// What a scenario does: a service failing more requests, a service slowing, or a site slowing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScenarioKind {
    Spike,
    Drift,
    Region,
}

/// Something going wrong for a while: rising, holding and falling back, as the web's `Scenario`.
#[derive(Clone, Debug, PartialEq)]
pub struct Scenario {
    pub id: String,
    pub kind: ScenarioKind,
    /// The service it hits, by its index in `SERVICES`, for a spike or a drift.
    pub service: Option<usize>,
    /// The site it hits, by its index in `SITES`, for a region.
    pub site: Option<usize>,
    /// When it started, in milliseconds of the board's clock.
    pub started: f64,
    /// How long it lasts, in milliseconds.
    pub length: f64,
    /// How hard it hits at its height.
    pub peak: f64,
}

/// How hard a scenario hits at a share of its length: rising fast, holding, then falling back.
pub fn envelope(progress: f64) -> f64 {
    if progress <= 0.0 || progress >= 1.0 {
        0.0
    } else if progress < 0.15 {
        progress / 0.15
    } else if progress < 0.7 {
        1.0
    } else {
        (1.0 - progress) / 0.3
    }
}

/// What the world knows besides its readings when it takes them: the services with a deploy
/// running, and those in a maintenance window.
#[derive(Clone, Debug, Default)]
pub struct Known {
    pub deploying: Vec<bool>,
    pub maintenance: Vec<bool>,
}

/// The live world: each service's readings, oldest first, and its health.
pub struct Live {
    pub readings: Vec<Vec<Reading>>,
    pub health: Vec<Health>,
    /// Each site's round trips, oldest first, in milliseconds.
    pub sites: Vec<Vec<f64>>,
    /// Each site's state by its latest round trips.
    pub site_health: Vec<Health>,
    /// What is going wrong now.
    pub scenarios: Vec<Scenario>,
    started: usize,
    wandered: HashMap<String, f64>,
    seed: u64,
}

impl Live {
    /// A history of calm readings ending at `now`, `interval` apart, so the charts start full.
    pub fn new(now: f64, interval: f64) -> Self {
        let mut live = Self {
            readings: Vec::new(),
            health: vec![Health::Operational; SERVICES.len()],
            sites: Vec::new(),
            site_health: vec![Health::Operational; SITES.len()],
            scenarios: Vec::new(),
            started: 0,
            wandered: HashMap::new(),
            seed: 20261116,
        };
        live.readings = (0..SERVICES.len())
            .map(|index| {
                (0..KEEP)
                    .map(|i| live.calm(index, now - (KEEP - 1 - i) as f64 * interval))
                    .collect()
            })
            .collect();
        live.sites = SITES
            .iter()
            .map(|site| (0..KEEP).map(|_| live.round_trip(site)).collect())
            .collect();
        live
    }

    /// A site's round trip with nothing wrong.
    fn round_trip(&mut self, site: &Site) -> f64 {
        site.latency * self.wander(site.id.to_owned(), 0.04)
    }

    /// Starts something going wrong at `now`: on the service or site given, or on one nothing
    /// is going wrong with yet, as the web's `trigger`.
    pub fn trigger(&mut self, kind: ScenarioKind, target: Option<usize>, now: f64) -> Scenario {
        self.started += 1;
        let busy_services: Vec<usize> = self.scenarios.iter().filter_map(|s| s.service).collect();
        let busy_sites: Vec<usize> = self.scenarios.iter().filter_map(|s| s.site).collect();
        let pick = |live: &mut Self, from: Vec<usize>, all: usize| {
            let from = if from.is_empty() { (0..all).collect() } else { from };
            from[((live.random() * from.len() as f64) as usize).min(from.len() - 1)]
        };
        let second = 1000.0;
        let scenario = match kind {
            ScenarioKind::Region => {
                let free = (0..SITES.len()).filter(|i| !busy_sites.contains(i)).collect();
                let site = target.unwrap_or_else(|| pick(self, free, SITES.len()));
                Scenario {
                    id: format!("scenario-{}", self.started),
                    kind,
                    service: None,
                    site: Some(site),
                    started: now,
                    length: self.between(30.0, 50.0) * second,
                    peak: self.between(2.5, 4.5),
                }
            }
            _ => {
                let free = (0..SERVICES.len())
                    .filter(|i| !busy_services.contains(i) && SERVICES[*i].id != "queue")
                    .collect();
                let service = target.unwrap_or_else(|| pick(self, free, SERVICES.len()));
                let spike = kind == ScenarioKind::Spike;
                Scenario {
                    id: format!("scenario-{}", self.started),
                    kind,
                    service: Some(service),
                    site: None,
                    started: now,
                    length: if spike {
                        self.between(25.0, 40.0)
                    } else {
                        self.between(35.0, 55.0)
                    } * second,
                    peak: if spike {
                        self.between(6.0, 10.0)
                    } else {
                        self.between(2.0, 3.5)
                    },
                }
            }
        };
        self.scenarios.push(scenario.clone());
        scenario
    }

    /// Ends everything going wrong, so the world calms down.
    pub fn calm_down(&mut self) {
        self.scenarios.clear();
    }

    /// How hard what goes wrong hits each service now (its own, and part of the worst of what it
    /// calls on) and each site, as the web's `pressure`: errors and slowing for each service, the
    /// slowing for each site.
    fn pressure(&self, now: f64, known: &Known) -> (Vec<(f64, f64)>, Vec<f64>) {
        let mut own = vec![(0.0, 0.0); SERVICES.len()];
        let mut sites = vec![0.0; SITES.len()];
        for scenario in &self.scenarios {
            let strength = envelope((now - scenario.started) / scenario.length);
            match (scenario.kind, scenario.service, scenario.site) {
                (ScenarioKind::Region, _, Some(site)) => sites[site] += scenario.peak * strength,
                (ScenarioKind::Spike, Some(service), _) => {
                    own[service].0 += scenario.peak * strength;
                    own[service].1 += 0.5 * strength;
                }
                (ScenarioKind::Drift, Some(service), _) => own[service].1 += scenario.peak * strength,
                _ => {}
            }
        }
        for (index, deploying) in known.deploying.iter().enumerate() {
            if *deploying {
                own[index].1 += 0.3;
            }
        }
        // a service slows with what it calls on, at part of the strength
        let felt = SERVICES
            .iter()
            .enumerate()
            .map(|(index, service)| {
                let from: Vec<(f64, f64)> = service
                    .depends_on
                    .iter()
                    .filter_map(|id| SERVICES.iter().position(|s| s.id == *id))
                    .map(|i| own[i])
                    .collect();
                let worst = |pick: fn(&(f64, f64)) -> f64| from.iter().map(pick).fold(0.0, f64::max);
                (
                    own[index].0 + 0.35 * worst(|o| o.0),
                    own[index].1 + 0.4 * worst(|o| o.1),
                )
            })
            .collect();
        (felt, sites)
    }

    /// The next reading of every service and site, at `at`, the oldest let go, with what goes
    /// wrong felt in them and each one's health from its latest readings.
    pub fn tick(&mut self, at: f64, known: &Known) {
        self.scenarios.retain(|s| at < s.started + s.length);
        let (felt, hits) = self.pressure(at, known);
        for index in 0..SERVICES.len() {
            let base = self.calm(index, at);
            let (errors, latency) = felt[index];
            let reading = Reading {
                at,
                p95: base.p95 * (1.0 + latency),
                p50: base.p50 * (1.0 + latency * 0.6),
                errors: base.errors + errors,
                load: base.load * (1.0 - (errors / 20.0).min(0.5)),
            };
            let readings = &mut self.readings[index];
            readings.push(reading);
            if readings.len() > KEEP {
                readings.remove(0);
            }
            let in_window = known.maintenance.get(index).copied().unwrap_or(false);
            self.health[index] = health_of(index, readings, in_window);
        }
        for (index, site) in SITES.iter().enumerate() {
            let trip = self.round_trip(site) * (1.0 + hits[index]);
            let trips = &mut self.sites[index];
            trips.push(trip);
            if trips.len() > KEEP {
                trips.remove(0);
            }
            self.site_health[index] = site_health_of(site, trips);
        }
    }

    fn between(&mut self, low: f64, high: f64) -> f64 {
        low + self.random() * (high - low)
    }

    /// A number in 0..1, the same run after run.
    fn random(&mut self) -> f64 {
        // splitmix64
        self.seed = self.seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        (z ^ (z >> 31)) as f64 / u64::MAX as f64
    }

    /// A factor near one that wanders from reading to reading rather than jumping.
    fn wander(&mut self, key: String, spread: f64) -> f64 {
        let r = self.random();
        let next = self.wandered.get(&key).copied().unwrap_or(0.0) * 0.8 + (r * 2.0 - 1.0) * spread;
        self.wandered.insert(key, next);
        1.0 + next
    }

    /// The reading a service takes with nothing wrong.
    fn calm(&mut self, index: usize, at: f64) -> Reading {
        let service = SERVICES[index];
        let p95 = service.latency * self.wander(format!("{}:p95", service.id), 0.03);
        let p50 = p95 * 0.42 * self.wander(format!("{}:p50", service.id), 0.015);
        let errors = (service.errors * self.wander(format!("{}:errors", service.id), 0.18)).max(0.0);
        let load = service.load
            * (1.0 + 0.12 * (at / 40_000.0 + service.latency).sin())
            * self.wander(format!("{}:load", service.id), 0.015);
        Reading {
            at,
            p50,
            p95,
            errors,
            load,
        }
    }
}

/// The mean of the last three values, as the web's `recent`.
fn recent(values: &[f64]) -> f64 {
    let tail = &values[values.len().saturating_sub(3)..];
    tail.iter().sum::<f64>() / tail.len().max(1) as f64
}

/// How a service is doing by its latest readings and its maintenance, as the web's `healthOf`.
pub fn health_of(index: usize, readings: &[Reading], in_window: bool) -> Health {
    if in_window {
        return Health::Maintenance;
    }
    let service = SERVICES[index];
    let errors = recent(&readings.iter().map(|r| r.errors).collect::<Vec<_>>());
    let p95 = recent(&readings.iter().map(|r| r.p95).collect::<Vec<_>>());
    if errors >= 5.0 || p95 >= service.latency * 3.5 {
        Health::Outage
    } else if errors >= 1.5 || p95 >= service.latency * 1.8 {
        Health::Degraded
    } else {
        Health::Operational
    }
}

/// How a site is doing by its latest round trips, as the web's `siteHealthOf`.
pub fn site_health_of(site: &Site, trips: &[f64]) -> Health {
    let factor = recent(trips) / site.latency;
    if factor >= 3.2 {
        Health::Outage
    } else if factor >= 1.7 {
        Health::Degraded
    } else {
        Health::Operational
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> (Live, Known) {
        let known = Known {
            deploying: vec![false; SERVICES.len()],
            maintenance: vec![false; SERVICES.len()],
        };
        (Live::new(0.0, 2000.0), known)
    }

    #[test]
    fn a_spike_fails_a_service_and_slows_what_calls_on_it() {
        let (mut live, known) = world();
        let api = SERVICES.iter().position(|s| s.id == "api").unwrap();
        let web = SERVICES.iter().position(|s| s.id == "web").unwrap();
        let calm_web = live.readings[web].last().unwrap().p95;
        live.trigger(ScenarioKind::Spike, Some(api), 0.0);
        for step in 1..=8 {
            live.tick(step as f64 * 2000.0, &known);
        }
        assert_ne!(live.health[api], Health::Operational, "the service it hits");
        assert!(
            live.readings[web].last().unwrap().p95 > calm_web * 1.1,
            "and what calls on it slows"
        );
    }

    #[test]
    fn a_window_going_on_is_maintenance_whatever_the_readings() {
        let (mut live, mut known) = world();
        known.maintenance[0] = true;
        live.tick(2000.0, &known);
        assert_eq!(live.health[0], Health::Maintenance);
        live.calm_down();
        assert!(live.scenarios.is_empty());
    }
}
