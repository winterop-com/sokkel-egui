//! The board's live world, after sokkel-ui's status example (`live.ts`): each service's readings,
//! taken every interval and drifting gently around its resting values. Things going wrong
//! (scenarios), sites and health from the readings follow as the port goes on.

use std::collections::HashMap;

use crate::data::SERVICES;

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

/// The live world: each service's readings, oldest first, and its health.
pub struct Live {
    pub readings: Vec<Vec<Reading>>,
    pub health: Vec<Health>,
    wandered: HashMap<String, f64>,
    seed: u64,
}

impl Live {
    /// A history of calm readings ending at `now`, `interval` apart, so the charts start full.
    pub fn new(now: f64, interval: f64) -> Self {
        let mut live = Self {
            readings: Vec::new(),
            health: vec![Health::Operational; SERVICES.len()],
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
        live
    }

    /// The next reading of every service, at `at`, the oldest let go.
    pub fn tick(&mut self, at: f64) {
        for index in 0..SERVICES.len() {
            let reading = self.calm(index, at);
            let readings = &mut self.readings[index];
            readings.push(reading);
            if readings.len() > KEEP {
                readings.remove(0);
            }
        }
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
