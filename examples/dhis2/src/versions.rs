//! The versions the simulated server can run as, and the function and variable names each knows, as
//! the web's `data/versions.ts` keeps them. The latest knows every name the dhis2 package lists
//! (`DHIS2_FUNCTIONS` and `DHIS2_VARIABLES`, carried here from sokkel-ui's
//! `packages/dhis2/src/names.ts`); each older one lacks a few that came later on this simulated
//! server.

use serde::Serialize;

pub const VERSIONS: [&str; 4] = ["2.41", "2.42", "2.43", "2.44"];
pub const LATEST: &str = "2.44";

/// The functions every expression takes: indicators, validation rules, predictors and program indicators.
const COMMON: &[&str] = &[
    "contains",
    "containsItems",
    "firstNonNull",
    "greatest",
    "if",
    "is",
    "isNotNull",
    "isNull",
    "least",
    "log",
    "log10",
    "normDistCum",
    "normDistDen",
    "removeZeros",
    "subExpression",
];

/// The functions that test the organization unit a validation rule or predictor runs for.
const ORG_UNIT: &[&str] = &[
    "orgUnit.ancestor",
    "orgUnit.dataSet",
    "orgUnit.group",
    "orgUnit.program",
];

/// The functions that aggregate, in predictors and in program indicators with a custom aggregation.
const AGGREGATION: &[&str] = &[
    "avg",
    "count",
    "max",
    "median",
    "min",
    "percentileCont",
    "stddev",
    "stddevPop",
    "stddevSamp",
    "sum",
    "variance",
];

/// The functions written after a value or a parenthesis, as `#{uid}.periodOffset(-1)`.
const AFTER_VALUE: &[&str] = &[
    ".aggregationType",
    ".maxDate",
    ".minDate",
    ".periodOffset",
    ".stageOffset",
    ".yearToDate",
];

/// The program functions of program rules and program indicators.
const PROGRAM: &[&str] = &[
    "d2:addDays",
    "d2:ceil",
    "d2:concatenate",
    "d2:condition",
    "d2:contains",
    "d2:containsItems",
    "d2:count",
    "d2:countIfCondition",
    "d2:countIfValue",
    "d2:countIfZeroPos",
    "d2:daysBetween",
    "d2:exponent",
    "d2:extractDataMatrixValue",
    "d2:floor",
    "d2:hasUserRole",
    "d2:hasValue",
    "d2:inOrgUnitGroup",
    "d2:inUserGroup",
    "d2:lastEventDate",
    "d2:left",
    "d2:length",
    "d2:log",
    "d2:maxValue",
    "d2:minutesBetween",
    "d2:minValue",
    "d2:modulus",
    "d2:monthsBetween",
    "d2:oizp",
    "d2:relationshipCount",
    "d2:right",
    "d2:round",
    "d2:split",
    "d2:substring",
    "d2:validatePattern",
    "d2:weeksBetween",
    "d2:yearsBetween",
    "d2:zing",
    "d2:zpvc",
    "d2:zScoreHFA",
    "d2:zScoreWFA",
    "d2:zScoreWFH",
];

/// Every function the latest server knows, in all its expression languages.
pub fn dhis2_functions() -> Vec<&'static str> {
    [COMMON, ORG_UNIT, AGGREGATION, AFTER_VALUE, PROGRAM].concat()
}

/// Every program variable the latest server knows in `V{}`.
pub const DHIS2_VARIABLES: &[&str] = &[
    "analytics_period_end",
    "analytics_period_start",
    "completed_date",
    "creation_date",
    "current_date",
    "due_date",
    "enrollment_count",
    "enrollment_date",
    "enrollment_id",
    "enrollment_status",
    "environment",
    "event_count",
    "event_date",
    "event_id",
    "event_status",
    "execution_date",
    "incident_date",
    "org_unit",
    "org_unit_count",
    "orgunit_code",
    "program_name",
    "program_stage_id",
    "program_stage_name",
    "scheduled_date",
    "scheduled_event_count",
    "sync_date",
    "tei_count",
    "value_count",
    "zero_pos_value_count",
];

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
