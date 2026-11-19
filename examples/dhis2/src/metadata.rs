//! The simulated server's metadata, as the web's `data/metadata.ts` keeps it: organization units by
//! plain geography, category option combos, data elements, a data set, constants and the months
//! data is held for. Every uid is made here from the thing's code, so it is the same on every load
//! and names nothing outside the example.

use std::sync::OnceLock;

use serde::Serialize;

use crate::js::{civil_from_days, date_utc, number_of_text, number_to_string};

const LETTERS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const ALPHANUMERIC: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// A number from 0 to 1 that is always the same for the same text.
pub fn hashed(text: &str) -> f64 {
    let mut hash: u32 = 2_166_136_261;
    for unit in text.encode_utf16() {
        hash = (hash ^ u32::from(unit)).wrapping_mul(16_777_619);
    }
    // Mixed once more, so texts that differ only at their end land far apart.
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x85eb_ca6b);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(0xc2b2_ae35);
    hash ^= hash >> 16;
    f64::from(hash) / f64::from(u32::MAX)
}

fn pick(set: &[u8], at: f64) -> char {
    let len = set.len();
    char::from(set[((at * len as f64).floor() as usize) % len])
}

/// An eleven-character uid in the server's shape, a letter first, made from a code.
pub fn uid(code: &str) -> String {
    let mut out = String::with_capacity(11);
    out.push(pick(LETTERS, hashed(&format!("{code}:0"))));
    for index in 1..11 {
        out.push(pick(ALPHANUMERIC, hashed(&format!("{code}:{index}"))));
    }
    out
}

// --- Organization units ---------------------------------------------------------------------------

/// The levels of the hierarchy, from 1 for the country to 4 for a site.
pub fn level_name(level: u8) -> &'static str {
    match level {
        1 => "Country",
        2 => "Region",
        3 => "District",
        _ => "Site",
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OrgUnit {
    pub id: String,
    pub code: String,
    pub name: String,
    pub level: u8,
    pub parent: Option<String>,
}

type District = (&'static str, &'static [&'static str]);

const HIERARCHY: &[(&str, &[District])] = &[
    (
        "Vestland",
        &[
            ("Bergen", &["Sandviken", "Fana", "Laksevåg"]),
            ("Voss", &["Voss town", "Vossestrand"]),
        ],
    ),
    (
        "Nordland",
        &[
            ("Bodø", &["Bodø center", "Saltstraumen"]),
            ("Lofoten", &["Svolvær", "Leknes"]),
        ],
    ),
    (
        "Innlandet",
        &[
            ("Hamar", &["Hamar town", "Stange"]),
            ("Lillehammer", &["Lillehammer town", "Fåberg"]),
        ],
    ),
];

/// A name as a code: its accents dropped, ø as o and å as a, upper case, and every run of anything
/// but letters and digits an underscore.
pub fn code_of(name: &str) -> String {
    let mut plain = String::new();
    for c in name.chars() {
        match c {
            'ø' | 'ó' | 'ò' | 'ô' | 'ö' | 'õ' => plain.push('o'),
            'Ø' | 'Ó' | 'Ò' | 'Ô' | 'Ö' | 'Õ' => plain.push('O'),
            'å' | 'á' | 'à' | 'â' | 'ä' | 'ã' => plain.push('a'),
            'Å' | 'Á' | 'À' | 'Â' | 'Ä' | 'Ã' => plain.push('A'),
            'é' | 'è' | 'ê' | 'ë' => plain.push('e'),
            'É' | 'È' | 'Ê' | 'Ë' => plain.push('E'),
            'í' | 'ì' | 'î' | 'ï' => plain.push('i'),
            'ú' | 'ù' | 'û' | 'ü' => plain.push('u'),
            '\u{300}'..='\u{36f}' => {}
            other => plain.push(other),
        }
    }
    let mut out = String::new();
    let mut gap = false;
    for c in plain.to_uppercase().chars() {
        if c.is_ascii_uppercase() || c.is_ascii_digit() {
            out.push(c);
            gap = false;
        } else if !gap {
            out.push('_');
            gap = true;
        }
    }
    out
}

const COUNTRY_CODE: &str = "NO";

fn unit(code: String, name: &str, level: u8, parent: Option<&str>) -> OrgUnit {
    OrgUnit {
        id: uid(&format!("ou:{code}")),
        code,
        name: name.to_owned(),
        level,
        parent: parent.map(str::to_owned),
    }
}

/// Every organization unit, the country first and each unit before those under it.
pub fn org_units() -> &'static [OrgUnit] {
    static UNITS: OnceLock<Vec<OrgUnit>> = OnceLock::new();
    UNITS.get_or_init(|| {
        let country = unit(COUNTRY_CODE.to_owned(), "Norway", 1, None);
        let mut all = vec![country.clone()];
        for (region, districts) in HIERARCHY {
            let region_unit = unit(
                format!("{COUNTRY_CODE}_{}", code_of(region)),
                region,
                2,
                Some(&country.id),
            );
            all.push(region_unit.clone());
            for (district, sites) in *districts {
                let district_unit = unit(
                    format!("{}_{}", region_unit.code, code_of(district)),
                    district,
                    3,
                    Some(&region_unit.id),
                );
                all.push(district_unit.clone());
                for site in *sites {
                    all.push(unit(
                        format!("{}_{}", district_unit.code, code_of(site)),
                        site,
                        4,
                        Some(&district_unit.id),
                    ));
                }
            }
        }
        all
    })
}

/// The country's uid, the root of the hierarchy.
pub fn root() -> &'static str {
    &org_units()[0].id
}

pub fn org_unit(id: &str) -> Option<&'static OrgUnit> {
    org_units().iter().find(|one| one.id == id)
}

pub fn children_of(id: &str) -> Vec<&'static OrgUnit> {
    org_units()
        .iter()
        .filter(|one| one.parent.as_deref() == Some(id))
        .collect()
}

/// The units from the country down to this one.
pub fn path_of(id: &str) -> Vec<&'static OrgUnit> {
    let mut path = Vec::new();
    let mut at = org_unit(id);
    while let Some(one) = at {
        path.insert(0, one);
        at = one.parent.as_deref().and_then(org_unit);
    }
    path
}

/// The sites at or under a unit, where data is entered.
pub fn sites_under(id: &str) -> Vec<&'static OrgUnit> {
    let Some(here) = org_unit(id) else {
        return Vec::new();
    };
    if here.level == 4 {
        return vec![here];
    }
    children_of(id)
        .into_iter()
        .flat_map(|child| sites_under(&child.id))
        .collect()
}

/// A node of the hierarchy's tree: a unit with its level beside its name.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TreeNode {
    pub id: String,
    pub label: String,
    pub meta: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<TreeNode>>,
}

fn tree_of(one: &OrgUnit) -> TreeNode {
    let children = children_of(&one.id);
    TreeNode {
        id: one.id.clone(),
        label: one.name.clone(),
        meta: level_name(one.level).to_owned(),
        children: (!children.is_empty()).then(|| children.into_iter().map(tree_of).collect()),
    }
}

/// The hierarchy as tree nodes, each with its level beside its name.
pub fn org_unit_tree() -> Vec<TreeNode> {
    vec![tree_of(&org_units()[0])]
}

// --- Categories -----------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CategoryOptionCombo {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CategoryCombo {
    pub id: String,
    pub name: String,
    pub combos: Vec<CategoryOptionCombo>,
}

fn coc(code: &str, name: &str) -> CategoryOptionCombo {
    CategoryOptionCombo {
        id: uid(&format!("coc:{code}")),
        name: name.to_owned(),
    }
}

/// The two category combos: `default`, and `Age` with under five and five and over.
pub fn category_combos() -> &'static [CategoryCombo] {
    static COMBOS: OnceLock<Vec<CategoryCombo>> = OnceLock::new();
    COMBOS.get_or_init(|| {
        vec![
            CategoryCombo {
                id: uid("cc:default"),
                name: "default".to_owned(),
                combos: vec![coc("default", "default")],
            },
            CategoryCombo {
                id: uid("cc:age"),
                name: "Age".to_owned(),
                combos: vec![coc("under5", "Under 5"), coc("over5", "5 and over")],
            },
        ]
    })
}

pub fn default_combo() -> &'static CategoryCombo {
    &category_combos()[0]
}

pub fn age_combo() -> &'static CategoryCombo {
    &category_combos()[1]
}

/// Every category option combo of every category combo.
pub fn combos() -> Vec<&'static CategoryOptionCombo> {
    category_combos().iter().flat_map(|combo| combo.combos.iter()).collect()
}

pub fn option_combo(id: &str) -> Option<&'static CategoryOptionCombo> {
    category_combos()
        .iter()
        .flat_map(|combo| combo.combos.iter())
        .find(|one| one.id == id)
}

// --- Data elements --------------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataElement {
    pub id: String,
    pub code: String,
    pub name: String,
    pub short_name: String,
    pub value_type: &'static str,
    pub aggregation_type: &'static str,
    pub category_combo: CategoryCombo,
    pub description: String,
}

fn element(code: &str, name: &str, short_name: &str, combo: &CategoryCombo, description: &str) -> DataElement {
    DataElement {
        id: uid(&format!("de:{code}")),
        code: code.to_owned(),
        name: name.to_owned(),
        short_name: short_name.to_owned(),
        value_type: "INTEGER_ZERO_OR_POSITIVE",
        aggregation_type: "SUM",
        category_combo: combo.clone(),
        description: description.to_owned(),
    }
}

/// The data elements by what they hold, as the web's `DE` names them.
pub struct De {
    pub visits: DataElement,
    pub referrals: DataElement,
    pub vaccinated: DataElement,
    pub due: DataElement,
    pub samples_taken: DataElement,
    pub samples_failed: DataElement,
    pub stock_received: DataElement,
    pub stock_used: DataElement,
    pub stock_on_hand: DataElement,
}

pub fn de() -> &'static De {
    static DE: OnceLock<De> = OnceLock::new();
    DE.get_or_init(|| {
        let plain = default_combo();
        De {
            visits: element(
                "DE_VISITS",
                "Outpatient visits",
                "Visits",
                age_combo(),
                "Visits seen at the site in the month, by age.",
            ),
            referrals: element(
                "DE_REFERRALS",
                "Referrals out",
                "Referrals",
                plain,
                "People sent on to another site.",
            ),
            vaccinated: element(
                "DE_VACCINATED",
                "Children vaccinated",
                "Vaccinated",
                plain,
                "Children given their last dose.",
            ),
            due: element(
                "DE_DUE",
                "Children due for vaccination",
                "Due",
                plain,
                "Children whose last dose fell due.",
            ),
            samples_taken: element(
                "DE_SAMPLES_TAKEN",
                "Water samples taken",
                "Samples taken",
                plain,
                "Samples taken at the site.",
            ),
            samples_failed: element(
                "DE_SAMPLES_FAILED",
                "Water samples failed",
                "Samples failed",
                plain,
                "Samples that failed the test.",
            ),
            stock_received: element(
                "DE_STOCK_RECEIVED",
                "Stock received",
                "Received",
                plain,
                "Kits delivered in the month.",
            ),
            stock_used: element(
                "DE_STOCK_USED",
                "Stock used",
                "Used",
                plain,
                "Kits handed out in the month.",
            ),
            stock_on_hand: element(
                "DE_STOCK_ON_HAND",
                "Stock on hand",
                "On hand",
                plain,
                "Kits counted at the end of the month.",
            ),
        }
    })
}

/// Every data element, in the order `DE` lists them.
pub fn data_elements() -> [&'static DataElement; 9] {
    let de = de();
    [
        &de.visits,
        &de.referrals,
        &de.vaccinated,
        &de.due,
        &de.samples_taken,
        &de.samples_failed,
        &de.stock_received,
        &de.stock_used,
        &de.stock_on_hand,
    ]
}

pub fn data_element(id: &str) -> Option<&'static DataElement> {
    data_elements().into_iter().find(|one| one.id == id)
}

// --- Data sets, constants -------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSet {
    pub id: String,
    pub code: String,
    pub name: String,
    pub period_type: &'static str,
    pub elements: Vec<String>,
}

/// The one data set, the monthly site report of every data element.
pub fn data_set() -> &'static DataSet {
    static SET: OnceLock<DataSet> = OnceLock::new();
    SET.get_or_init(|| DataSet {
        id: uid("ds:MONTHLY"),
        code: "DS_MONTHLY".to_owned(),
        name: "Monthly site report".to_owned(),
        period_type: "Monthly",
        elements: data_elements().iter().map(|one| one.id.clone()).collect(),
    })
}

pub fn data_sets() -> [&'static DataSet; 1] {
    [data_set()]
}

/// What a reporting rate item may ask for after its data set.
pub const REPORTING_METRICS: [&str; 3] = ["REPORTING_RATE", "ACTUAL_REPORTS", "EXPECTED_REPORTS"];

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Constant {
    pub id: String,
    pub code: String,
    pub name: String,
    pub value: f64,
}

pub fn constants() -> &'static [Constant] {
    static CONSTANTS: OnceLock<Vec<Constant>> = OnceLock::new();
    CONSTANTS.get_or_init(|| {
        vec![
            Constant {
                id: uid("c:TARGET"),
                code: "VACCINATION_TARGET".to_owned(),
                name: "Vaccination target".to_owned(),
                value: 90.0,
            },
            Constant {
                id: uid("c:SAMPLES"),
                code: "SAMPLES_PER_SITE".to_owned(),
                name: "Samples per site".to_owned(),
                value: 4.0,
            },
        ]
    })
}

pub fn constant(id: &str) -> Option<&'static Constant> {
    constants().iter().find(|one| one.id == id)
}

// --- Periods --------------------------------------------------------------------------------------

/// The months the server holds data for, oldest first: November 2025 to October 2026.
pub fn periods() -> &'static [String] {
    static PERIODS: OnceLock<Vec<String>> = OnceLock::new();
    PERIODS.get_or_init(|| {
        (0..12)
            .map(|index| {
                let month = 11 + index;
                let year = 2025 + (month - 1) / 12;
                format!("{year}{:02}", (month - 1) % 12 + 1)
            })
            .collect()
    })
}

/// The latest month the server holds.
pub fn latest_period() -> &'static str {
    &periods()[periods().len() - 1]
}

/// The day the example takes as today, so every load reads the same.
pub const TODAY: &str = "2026-11-05";

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// A text's UTF-16 units from `from` to `to`, as `slice` cuts them.
fn units(id: &str, from: usize, to: usize) -> String {
    let all: Vec<u16> = id.encode_utf16().collect();
    let to = to.min(all.len());
    let from = from.min(to);
    String::from_utf16_lossy(&all[from..to])
}

/// `Date.UTC(year, month)`'s days, as `Date.UTC` reads its year and month: whole parts, a year
/// from 0 to 99 taken as 1900 and after; none for one that is not a finite number.
fn utc_days(year: f64, month: f64, day: f64) -> Option<i64> {
    if !year.is_finite() || !month.is_finite() || !day.is_finite() {
        return None;
    }
    let mut year = year.trunc() as i64;
    if (0..=99).contains(&year) {
        year += 1900;
    }
    Some(date_utc(year, month.trunc() as i64, day.trunc() as i64))
}

/// A monthly period as `Intl.DateTimeFormat('en-US', { month: 'short', year: 'numeric' })` writes
/// it: `Nov 2025`.
pub fn period_label(id: &str) -> String {
    let year = number_of_text(&units(id, 0, 4));
    let month = number_of_text(&units(id, 4, 6)) - 1.0;
    match utc_days(year, month, 1.0) {
        Some(days) => {
            let (year, month, _) = civil_from_days(days);
            format!("{} {year}", MONTHS[(month - 1) as usize])
        }
        None => "Invalid Date".to_owned(),
    }
}

/// The period `by` months from a monthly period.
pub fn shift_period(id: &str, by: f64) -> String {
    let index = number_of_text(&units(id, 0, 4)) * 12.0 + number_of_text(&units(id, 4, 6)) - 1.0 + by;
    let month = number_to_string(index % 12.0 + 1.0);
    let padded = if month.encode_utf16().count() < 2 {
        format!("0{month}")
    } else {
        month
    };
    format!("{}{padded}", number_to_string((index / 12.0).floor()))
}

/// The days in a monthly period.
pub fn days_in(id: &str) -> f64 {
    let year = number_of_text(&units(id, 0, 4));
    let month = number_of_text(&units(id, 4, 6));
    match utc_days(year, month, 0.0) {
        Some(days) => civil_from_days(days).2 as f64,
        None => f64::NAN,
    }
}
