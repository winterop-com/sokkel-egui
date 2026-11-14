//! Each service's last days, after sokkel-ui's status example (`history.ts`): the uptime the
//! incidents give each day, weighed by their severity.

use sokkel_ui_egui::{Day, UptimeDay};

use crate::data::{Incident, Severity};

const DAY: f64 = 24.0 * 60.0 * 60.0 * 1000.0;

/// How much of an incident's time counts as down.
fn weight(severity: Severity) -> f64 {
    match severity {
        Severity::Minor => 0.3,
        Severity::Major => 0.7,
        Severity::Critical => 1.0,
    }
}

/// A number from 0 to 1 that is always the same for the same text (FNV-1a, as the web's).
fn hashed(text: &str) -> f64 {
    let mut hash: u32 = 2_166_136_261;
    for unit in text.encode_utf16() {
        hash = (hash ^ u32::from(unit)).wrapping_mul(16_777_619);
    }
    f64::from(hash) / f64::from(u32::MAX)
}

/// How many milliseconds two spans share.
fn overlap(from: f64, to: f64, start: f64, end: f64) -> f64 {
    (to.min(end) - from.max(start)).max(0.0)
}

/// A service's last `count` days, oldest first, today last, `now` in milliseconds of the board's
/// clock (today's midnight its zero): each day's uptime from the incidents that touched it, weighed
/// by their severity, a stray minute lost here and there, and the day drawn in the state of the
/// worst incident it had.
pub fn uptime_days(service: &str, incidents: &[Incident], today: Day, now: f64, count: usize) -> Vec<UptimeDay> {
    let touching: Vec<&Incident> = incidents.iter().filter(|i| i.services.contains(&service)).collect();
    (0..count)
        .map(|index| {
            let back = (count - 1 - index) as i32;
            let date = today.plus(-back);
            let from = -f64::from(back) * DAY;
            let to = (from + DAY).min(now);
            let length = (to - from).max(1.0);
            let mut lost = 0.0;
            let mut worst: Option<Severity> = None;
            let mut notes = Vec::new();
            for incident in &touching {
                let closed = incident.resolved.unwrap_or(now);
                let shared = overlap(incident.opened, closed, from, to);
                if shared <= 0.0 {
                    continue;
                }
                lost += shared * weight(incident.severity);
                notes.push(incident.title.clone());
                if worst.is_none_or(|w| weight(incident.severity) > weight(w)) {
                    worst = Some(incident.severity);
                }
            }
            let blip = if hashed(&format!("{service}:{date}")) < 0.05 {
                0.0003
            } else {
                0.0
            };
            let uptime = (1.0 - lost / length - blip).max(0.0);
            let mut day = UptimeDay::new(date, Some(uptime));
            if let Some(worst) = worst {
                // the status map's colors: degraded a warning, an outage critical
                day.status = Some(if worst == Severity::Minor {
                    "warning"
                } else {
                    "critical"
                });
                day.note = Some(notes.join("; "));
            }
            day
        })
        .collect()
}
