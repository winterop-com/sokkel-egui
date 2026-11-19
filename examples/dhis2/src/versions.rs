//! The versions the simulated server can run as, and the function and variable names each knows, as
//! the web's `data/versions.ts` keeps them. The latest knows every name the dhis2 add-on lists
//! (`sokkel_egui_dhis2::names`); each older one lacks a few that came later on this simulated
//! server.

use serde::Serialize;
use sokkel_egui_dhis2::names::{DHIS2_VARIABLES, Dhis2Names, dhis2_functions};

pub const VERSIONS: [&str; 4] = ["2.41", "2.42", "2.43", "2.44"];
pub const LATEST: &str = "2.44";

/// The version each name came in on this simulated server; a name not listed is in every version.
pub const ADDED: &[(&str, &str)] = &[
    ("removeZeros", "2.42"),
    ("containsItems", "2.42"),
    ("d2:containsItems", "2.42"),
    ("d2:countIfZeroPos", "2.42"),
    ("scheduled_event_count", "2.42"),
    (".stageOffset", "2.43"),
    ("d2:minutesBetween", "2.43"),
    ("org_unit_count", "2.43"),
    ("d2:extractDataMatrixValue", "2.44"),
    ("d2:inUserGroup", "2.44"),
    ("analytics_period_end", "2.44"),
    ("analytics_period_start", "2.44"),
];

/// The version a name came in, when it came after the first.
pub fn added(name: &str) -> Option<&'static str> {
    ADDED.iter().find(|(one, _)| *one == name).map(|(_, version)| *version)
}

fn index_of(version: &str) -> Option<usize> {
    VERSIONS.iter().position(|one| *one == version)
}

/// Whether a version knows a name. A version not among `VERSIONS` stands before every one, as
/// `indexOf` gives -1 for it.
fn known(name: &str, version: &str) -> bool {
    let came = index_of(added(name).unwrap_or(VERSIONS[0])).map_or(-1, |at| at as i64);
    let running = index_of(version).map_or(-1, |at| at as i64);
    came <= running
}

/// The function and variable names a version knows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Names {
    pub functions: Vec<&'static str>,
    pub variables: Vec<&'static str>,
}

/// The function and variable names a version knows.
pub fn names_of(version: &str) -> Names {
    Names {
        functions: dhis2_functions()
            .into_iter()
            .filter(|name| known(name, version))
            .collect(),
        variables: DHIS2_VARIABLES
            .iter()
            .copied()
            .filter(|name| known(name, version))
            .collect(),
    }
}

impl Names {
    /// The names as the dhis2 add-on's languages take them.
    pub fn plugin_names(&self) -> Dhis2Names {
        Dhis2Names::new(self.functions.iter().copied(), self.variables.iter().copied())
    }
}
