//! What the board does on its own each reading, as the web's `live.ts` and `App.tsx`: things going
//! wrong at the weather's pace, an incident opened for what goes wrong, the on-call responder
//! moving those incidents along, and a notice for each change of state the settings ask to hear.

use eframe::egui::Context;
use sokkel_ui_egui::{Toast, ToastKind, toast_acted};

use crate::data::{self, PEOPLE, SERVICES, SITES, Severity, State};
use crate::live::{Health, ScenarioKind};
use crate::{Notify, StatusApp, Weather};

/// How long a responder takes to find the cause, and how long it watches a recovery, in
/// milliseconds, as the web's `TO_IDENTIFY` and `TO_RESOLVE`.
const TO_IDENTIFY: f64 = 12_000.0;
const TO_RESOLVE: f64 = 20_000.0;

/// A notice waiting to be shown: what it says, how it looks, how long it stays, and where its
/// Open goes.
#[derive(Clone, Debug, PartialEq)]
pub struct Notice {
    pub title: String,
    pub description: String,
    pub kind: ToastKind,
    pub timeout: f64,
    pub route: String,
}

/// The incident a scenario opens: its title and first update, by its kind and what it hits.
fn titles(kind: ScenarioKind, name: &str) -> (String, String) {
    match kind {
        ScenarioKind::Spike => (
            format!("Elevated error rates on {name}"),
            format!("{name} is answering more requests with an error than usual. We are looking into it."),
        ),
        ScenarioKind::Drift => (
            format!("Slow responses from {name}"),
            format!("{name} is answering slowly. We are looking into it."),
        ),
        ScenarioKind::Region => (
            format!("Degraded performance in {name}"),
            format!("Requests through {name} are slow or failing. We are looking into it."),
        ),
    }
}

/// What the responder posts as an incident the board opened moves to `state`.
fn next_words(kind: ScenarioKind, state: State, name: &str) -> String {
    match (kind, state) {
        (ScenarioKind::Spike, State::Identified) => {
            format!("The errors follow the last deploy of {name}. We are rolling it back.")
        }
        (ScenarioKind::Spike, State::Monitoring) => {
            format!("The rollback is out and {name} answers normally again. We are watching it.")
        }
        (ScenarioKind::Drift, State::Identified) => format!("A slow query holds {name} up. We are adding capacity."),
        (ScenarioKind::Drift, State::Monitoring) => {
            format!("The capacity is in and {name} is fast again. We are watching it.")
        }
        (ScenarioKind::Drift, State::Resolved) => format!("{name} has stayed fast. This incident is resolved."),
        (ScenarioKind::Region, State::Identified) => {
            format!("A network path into {name} is dropping packets. Traffic is moving to a neighbor.")
        }
        (ScenarioKind::Region, State::Monitoring) => {
            format!("Traffic for {name} is flowing again. We are watching it.")
        }
        (_, State::Resolved) => format!("{name} has answered normally for a while. This incident is resolved."),
        _ => String::new(),
    }
}

/// How a state is said in a notice's title, and how the notice looks.
fn said(health: Health) -> (&'static str, ToastKind) {
    match health {
        Health::Operational => ("is operational again", ToastKind::Success),
        Health::Degraded => ("is degraded", ToastKind::Warning),
        Health::Outage => ("is down", ToastKind::Error),
        Health::Maintenance => ("is in maintenance", ToastKind::Info),
    }
}

impl StatusApp {
    /// Something going wrong by itself before a reading, at the pace the settings' weather sets.
    pub(crate) fn weather(&mut self, now: f64) {
        let (every, at_once) = match self.options.weather {
            Weather::Calm => (120_000.0, 1),
            Weather::Normal => (45_000.0, 2),
            Weather::Stormy => (15_000.0, 3),
        };
        let interval = f64::from(self.options.interval) * 1000.0;
        self.live_world.weather(every, at_once, interval, now);
    }

    /// After a reading: the incidents the board opens and moves along, and the notices for what
    /// changed since `health` and `site_health`, the states before it.
    pub(crate) fn watch(&mut self, now: f64, health: &[Health], site_health: &[Health]) {
        let states: Vec<(String, State)> = self.incidents.iter().map(|i| (i.id.clone(), i.state)).collect();
        if self.options.auto_incidents {
            self.open_incidents(now);
        }
        if self.options.responder {
            self.respond(now);
        }
        // a change of a service's or a site's state, as the settings ask
        for (index, service) in SERVICES.iter().enumerate() {
            let (from, to) = (health[index], self.live_world.health[index]);
            if from != to {
                self.tell_change(service.id, from, to, format!("services/{}", service.id));
            }
        }
        for (index, site) in SITES.iter().enumerate() {
            let (from, to) = (site_health[index], self.live_world.site_health[index]);
            if from != to {
                self.tell_change(site.city, from, to, "regions".into());
            }
        }
        // an incident opened or resolved
        if self.options.notify == Notify::Off {
            return;
        }
        let mut told = Vec::new();
        for incident in &self.incidents {
            let was = states.iter().find(|(id, _)| *id == incident.id).map(|(_, s)| *s);
            let route = format!("incidents/{}", incident.id);
            match was {
                None if incident.state != State::Resolved => told.push(Notice {
                    title: format!("{} opened", incident.code),
                    description: incident.title.clone(),
                    kind: if incident.severity == Severity::Minor {
                        ToastKind::Warning
                    } else {
                        ToastKind::Error
                    },
                    timeout: 8.0,
                    route,
                }),
                Some(was) if was != State::Resolved && incident.state == State::Resolved => told.push(Notice {
                    title: format!("{} resolved", incident.code),
                    description: incident.title.clone(),
                    kind: ToastKind::Success,
                    timeout: 6.0,
                    route,
                }),
                _ => {}
            }
        }
        self.notices.extend(told);
    }

    /// A notice that `name` went from one state to another, unless the settings keep quiet about it.
    fn tell_change(&mut self, name: &str, from: Health, to: Health, route: String) {
        let quiet = match self.options.notify {
            Notify::Off => true,
            Notify::Outages => to != Health::Outage && from != Health::Outage,
            Notify::All => false,
        };
        if quiet {
            return;
        }
        let (words, kind) = said(to);
        self.notices.push(Notice {
            title: format!("{name} {words}"),
            description: format!("It was {}.", from.words().to_lowercase()),
            kind,
            timeout: 6.0,
            route,
        });
    }

    /// An incident for each thing going wrong that has made its service or site degraded or down
    /// and that no open incident covers, given to whoever is on call, as the web's `openIncidents`.
    fn open_incidents(&mut self, now: f64) {
        let scenarios = self.live_world.scenarios.clone();
        for scenario in scenarios {
            if self.opened_for.contains(&scenario.id) {
                continue;
            }
            let state = match (scenario.service, scenario.site) {
                (Some(service), _) => self.live_world.health[service],
                (_, Some(site)) => self.live_world.site_health[site],
                _ => Health::Operational,
            };
            if state != Health::Degraded && state != Health::Outage {
                continue;
            }
            let service = scenario.service.map(|i| SERVICES[i].id);
            let site = scenario.site.map(|i| SITES[i].id);
            let covered = self.incidents.iter().any(|i| {
                i.state != State::Resolved
                    && match (service, site) {
                        (Some(service), _) => i.services.contains(&service),
                        (_, Some(site)) => i.sites.contains(&site),
                        _ => false,
                    }
            });
            if covered {
                continue;
            }
            self.opened_for.push(scenario.id.clone());
            let name = service.unwrap_or_else(|| scenario.site.map_or("a site", |i| SITES[i].city));
            let (title, first) = titles(scenario.kind, name);
            let next = self
                .incidents
                .iter()
                .filter_map(|i| i.code.strip_prefix("INC-")?.parse::<u32>().ok())
                .max()
                .unwrap_or(0)
                + 1;
            let assignee = PEOPLE
                .iter()
                .find(|p| **p == self.on_call)
                .copied()
                .unwrap_or(PEOPLE[0]);
            self.incidents.insert(
                0,
                data::Incident {
                    id: format!("incident-{next}"),
                    code: format!("INC-{next}"),
                    title,
                    severity: if state == Health::Outage {
                        Severity::Major
                    } else {
                        Severity::Minor
                    },
                    state: State::Investigating,
                    services: service.into_iter().collect(),
                    sites: site.into_iter().collect(),
                    assignee,
                    opened: now,
                    resolved: None,
                    updates: vec![data::Update {
                        at: now,
                        state: State::Investigating,
                        message: first,
                        by: assignee,
                    }],
                    public: true,
                    cause: Some(scenario.kind),
                },
            );
        }
    }

    /// The on-call responder's next update on each incident the board opened, as the web's
    /// `respond`: identified after a while, monitoring once what it covers is healthy again, and
    /// resolved once it has stayed so.
    fn respond(&mut self, now: f64) {
        let health = self.live_world.health.clone();
        let site_health = self.live_world.site_health.clone();
        for incident in &mut self.incidents {
            let Some(cause) = incident.cause else {
                continue;
            };
            if incident.state == State::Resolved {
                continue;
            }
            let since = now - incident.updates.first().map_or(incident.opened, |u| u.at);
            let healthy = incident.services.iter().all(|id| {
                SERVICES
                    .iter()
                    .position(|s| s.id == *id)
                    .is_none_or(|i| matches!(health[i], Health::Operational | Health::Maintenance))
            }) && incident.sites.iter().all(|id| {
                SITES
                    .iter()
                    .position(|s| s.id == *id)
                    .is_none_or(|i| site_health[i] == Health::Operational)
            });
            let next = match incident.state {
                State::Investigating if since >= TO_IDENTIFY => Some(State::Identified),
                State::Identified if healthy => Some(State::Monitoring),
                State::Monitoring if healthy && since >= TO_RESOLVE => Some(State::Resolved),
                _ => None,
            };
            let Some(next) = next else {
                continue;
            };
            let name = incident.services.first().copied().unwrap_or_else(|| {
                incident
                    .sites
                    .first()
                    .and_then(|id| SITES.iter().find(|s| s.id == *id))
                    .map_or("the service", |s| s.city)
            });
            incident.state = next;
            incident.resolved = (next == State::Resolved).then_some(now);
            incident.updates.insert(
                0,
                data::Update {
                    at: now,
                    state: next,
                    message: next_words(cause, next, name),
                    by: incident.assignee,
                },
            );
        }
    }

    /// The notices waiting, shown as toasts with Open, and an Open pressed going where it says.
    pub(crate) fn show_notices(&mut self, ctx: &Context) {
        for notice in std::mem::take(&mut self.notices) {
            let id = Toast::new(notice.title)
                .kind(notice.kind)
                .description(notice.description)
                .timeout(Some(notice.timeout))
                .action("Open")
                .show(ctx);
            self.notice_routes.push((id, notice.route));
        }
        for id in toast_acted(ctx) {
            if let Some(at) = self.notice_routes.iter().position(|(one, _)| *one == id) {
                let (_, route) = self.notice_routes.remove(at);
                self.go(&route);
            }
        }
    }
}
