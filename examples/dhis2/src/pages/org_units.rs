//! The organization unit hierarchy, as the web's `pages/OrgUnits.tsx` shows it: the chosen unit's
//! facts, its data values for a month and its reporting over the year.

use serde::Serialize;

use crate::metadata::{
    OrgUnit, children_of, data_elements, latest_period, level_name, org_unit, org_unit_tree, path_of, period_label,
    periods, root, sites_under,
};
use crate::ui::{figure, number};
use crate::values::{data_value, reports};

/// A data value as the table lists it: its data element and category option combo.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ValueRow {
    pub id: String,
    pub element: String,
    pub combo: String,
    pub value: Option<f64>,
    pub shown: String,
}

/// A term and its value among the unit's facts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Fact {
    pub term: String,
    pub value: String,
}

/// The page: the unit chosen and the month its values are for.
#[derive(Clone, Debug)]
pub struct OrgUnits {
    pub query: String,
    pub period: String,
}

impl Default for OrgUnits {
    fn default() -> Self {
        OrgUnits {
            query: String::new(),
            period: latest_period().to_owned(),
        }
    }
}

/// The unit a route names, or the country.
pub fn unit_of(selected: Option<&str>) -> &'static OrgUnit {
    selected
        .and_then(org_unit)
        .unwrap_or_else(|| org_unit(root()).expect("the country"))
}

/// Where choosing a unit in the tree goes: the country is the page itself.
pub fn route_of(id: Option<&str>) -> String {
    match id {
        None => "/org-units".to_owned(),
        Some(id) if id == root() => "/org-units".to_owned(),
        Some(id) => format!("/org-units/{id}"),
    }
}

impl OrgUnits {
    /// Every data element's value for each of its category option combos, for the unit and month.
    pub fn values(&self, unit: &OrgUnit) -> Vec<ValueRow> {
        data_elements()
            .iter()
            .flat_map(|element| {
                element.category_combo.combos.iter().map(|combo| {
                    let value = data_value(&element.id, Some(&combo.id), &unit.id, &self.period);
                    ValueRow {
                        id: format!("{}.{}", element.id, combo.id),
                        element: element.name.clone(),
                        combo: combo.name.clone(),
                        value,
                        shown: figure(value, ""),
                    }
                })
            })
            .collect()
    }

    /// The unit's facts: its code, level, path, the units and sites under it, and its reports.
    pub fn facts(&self, unit: &OrgUnit) -> Vec<Fact> {
        let fact = |term: &str, value: String| Fact {
            term: term.to_owned(),
            value,
        };
        let path: Vec<&str> = path_of(&unit.id).iter().map(|one| one.name.as_str()).collect();
        let (actual, expected) = reports(&unit.id, &self.period);
        let mut out = vec![
            fact("Code", unit.code.clone()),
            fact("Level", format!("{}, {}", unit.level, level_name(unit.level))),
            fact("Path", path.join(" / ")),
        ];
        if unit.level < 4 {
            out.push(fact(
                "Under it",
                children_of(&unit.id)
                    .iter()
                    .map(|one| one.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
            out.push(fact("Sites", number(sites_under(&unit.id).len() as f64)));
        }
        out.push(fact(
            &format!("Reports, {}", period_label(&self.period)),
            format!("{actual} of {expected}"),
        ));
        out
    }
}

/// The unit's reporting rate in each month, in percent to a tenth, by the month's label.
pub fn year(unit: &OrgUnit) -> Vec<(String, Option<f64>)> {
    periods()
        .iter()
        .map(|one| {
            let (actual, expected) = reports(&unit.id, one);
            (
                period_label(one),
                (expected > 0).then(|| crate::js::round((actual as f64 / expected as f64) * 1000.0) / 10.0),
            )
        })
        .collect()
}

/// The units the tree starts open at: those on the unit's path and every region.
pub fn default_expanded(unit: &OrgUnit) -> Vec<String> {
    let mut out: Vec<String> = path_of(&unit.id).iter().map(|one| one.id.clone()).collect();
    let tree = org_unit_tree();
    out.extend(tree[0].children.iter().flatten().map(|one| one.id.clone()));
    out
}

/// What the status line says: the unit's path.
pub fn foot(unit: &OrgUnit) -> String {
    path_of(&unit.id)
        .iter()
        .map(|one| one.name.as_str())
        .collect::<Vec<_>>()
        .join(" / ")
}

/// The header's description.
pub fn description(unit: &OrgUnit) -> String {
    format!(
        "{} at level {}. Data is entered at sites; above a site, every value is the sum of the sites under it.",
        level_name(unit.level),
        unit.level
    )
}

/// The data values section's description.
pub fn values_description(unit: &OrgUnit) -> String {
    if unit.level == 4 {
        "What the site reported.".to_owned()
    } else {
        format!(
            "The sums of what the {} sites under it reported.",
            sites_under(&unit.id).len()
        )
    }
}
