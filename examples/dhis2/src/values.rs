//! The simulated server's data values, as the web's `data/values.ts` makes them: each site's monthly
//! report, made from its code and the month so it is the same on every load, with a few months
//! unreported and a few values that break a validation rule, as real reports have. Values above a
//! site are the sums of the sites under it.

use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use std::sync::OnceLock;

use crate::js::round;
use crate::metadata::{OrgUnit, age_combo, data_element, de, hashed, org_units, periods, sites_under};

struct Held {
    values: HashMap<String, f64>,
    reported: HashSet<String>,
}

fn key(element: &str, combo: &str, site: &str, period: &str) -> String {
    format!("{element}.{combo}.{site}.{period}")
}

fn fill(site: &OrgUnit, held: &mut Held) {
    let size = 0.6 + hashed(&site.code) * 1.2;
    let mut on_hand = round(60.0 * size);
    let de = de();
    for (index, period) in periods().iter().enumerate() {
        let noise = |what: &str| hashed(&format!("{what}:{}:{period}", site.code));
        let season = 1.0 + 0.25 * ((2.0 * PI * index as f64) / 12.0).sin();
        let under5 = &age_combo().combos[0];
        let over5 = &age_combo().combos[1];
        let young = round(40.0 * size * season * (0.85 + 0.3 * noise("young")));
        let old = round(110.0 * size * season * (0.85 + 0.3 * noise("old")));
        let due = round(18.0 * size * (0.8 + 0.4 * noise("due")));
        let mut vaccinated = round(due * (0.74 + 0.24 * noise("vaccinated")));
        if noise("vaccinated-plant") < 0.03 {
            vaccinated = due + 1.0 + (noise("extra") * 3.0).floor();
        }
        let taken = 3.0 + (noise("taken") * 3.0).floor();
        let mut failed = round(taken * noise("failed") * 0.4);
        if noise("failed-plant") < 0.025 {
            failed = taken + 1.0;
        }
        let received = if noise("delivered") < 0.15 {
            0.0
        } else {
            round(40.0 * size * (0.7 + 0.6 * noise("received")))
        };
        let mut used = (on_hand + received).min(round(35.0 * size * (0.7 + 0.6 * noise("used"))));
        if noise("used-plant") < 0.03 {
            used = on_hand + received + 5.0;
        }
        let end = (on_hand + received - used).max(0.0);
        on_hand = end;
        let reported = noise("reported") > 0.07 && !(site.code.ends_with("VOSSESTRAND") && (4..=6).contains(&index));
        if !reported {
            continue;
        }
        held.reported.insert(format!("{}.{period}", site.id));
        let plain = &de.referrals.category_combo.combos[0].id;
        let mut put = |element: &str, combo: &str, value: f64| {
            held.values.insert(key(element, combo, &site.id, period), value);
        };
        put(&de.visits.id, &under5.id, young);
        put(&de.visits.id, &over5.id, old);
        put(
            &de.referrals.id,
            plain,
            round((young + old) * (0.02 + 0.03 * noise("referrals"))),
        );
        put(&de.due.id, plain, due);
        put(&de.vaccinated.id, plain, vaccinated);
        put(&de.samples_taken.id, plain, taken);
        put(&de.samples_failed.id, plain, failed);
        put(&de.stock_received.id, plain, received);
        put(&de.stock_used.id, plain, used);
        put(&de.stock_on_hand.id, plain, end);
    }
}

fn held() -> &'static Held {
    static HELD: OnceLock<Held> = OnceLock::new();
    HELD.get_or_init(|| {
        let mut held = Held {
            values: HashMap::new(),
            reported: HashSet::new(),
        };
        for site in org_units().iter().filter(|one| one.level == 4) {
            fill(site, &mut held);
        }
        held
    })
}

/// A data element's value for a unit and a month: one category option combo's, or every combo's
/// summed when `combo` is none, over the sites at or under the unit. None when no site reported it.
pub fn data_value(element: &str, combo: Option<&str>, unit: &str, period: &str) -> Option<f64> {
    let found = data_element(element)?;
    let combos: Vec<&str> = match combo {
        None => found.category_combo.combos.iter().map(|one| one.id.as_str()).collect(),
        Some(one) => vec![one],
    };
    let held = held();
    let mut sum: Option<f64> = None;
    for site in sites_under(unit) {
        for one in &combos {
            if let Some(value) = held.values.get(&key(element, one, &site.id, period)) {
                sum = Some(sum.unwrap_or(0.0) + value);
            }
        }
    }
    sum
}

/// How many of the sites at or under a unit sent the month's report, of how many should have.
pub fn reports(unit: &str, period: &str) -> (usize, usize) {
    let sites = sites_under(unit);
    if !periods().iter().any(|one| one == period) {
        return (0, 0);
    }
    let held = held();
    let actual = sites
        .iter()
        .filter(|site| held.reported.contains(&format!("{}.{period}", site.id)))
        .count();
    (actual, sites.len())
}
