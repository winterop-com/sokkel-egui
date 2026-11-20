//! What the demo server starts with, as the web's `data/seed.ts` makes it: made-up patients of two
//! clinics, their visits, vital signs and conditions, the practitioners who saw them, and CQL
//! libraries in a few versions. Every person is invented, and the data is drawn from fixed numbers,
//! so it is the same on every reset.

use serde_json::{Value, json};

use crate::js::{days_from_civil, iso_day, json_number as number, round};
use crate::model::{Resource, to_base64};

pub const LOINC: &str = "http://loinc.org";
pub const SNOMED: &str = "http://snomed.info/sct";
pub const UCUM: &str = "http://unitsofmeasure.org";
const PATIENT_NUMBER: &str = "https://example.org/fhir/sid/patient-number";
const STAFF_NUMBER: &str = "https://example.org/fhir/sid/staff-number";
const CATEGORY: &str = "http://terminology.hl7.org/CodeSystem/observation-category";
const CLINICAL: &str = "http://terminology.hl7.org/CodeSystem/condition-clinical";
const VERIFICATION: &str = "http://terminology.hl7.org/CodeSystem/condition-ver-status";
const ACT_CODE: &str = "http://terminology.hl7.org/CodeSystem/v3-ActCode";
pub const LIBRARY_TYPE: &str = "http://terminology.hl7.org/CodeSystem/library-type";
pub const LIBRARY_BASE: &str = "https://example.org/fhir/Library";

/// A resource as the server first saw it, with its earlier versions, oldest first, each with the
/// instant it was saved.
#[derive(Clone, Debug, PartialEq)]
pub struct Seeded {
    pub resource: Resource,
    pub earlier: Vec<(String, Resource)>,
}

/// A vital sign the clinics measure, with its LOINC code and unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Vital {
    pub id: &'static str,
    pub code: &'static str,
    pub display: &'static str,
    pub unit: &'static str,
}

/// The vital signs the clinics measure.
pub const VITALS: [Vital; 3] = [
    Vital {
        id: "weight",
        code: "29463-7",
        display: "Body weight",
        unit: "kg",
    },
    Vital {
        id: "heart-rate",
        code: "8867-4",
        display: "Heart rate",
        unit: "/min",
    },
    Vital {
        id: "systolic",
        code: "8480-6",
        display: "Systolic blood pressure",
        unit: "mm[Hg]",
    },
];

struct Org {
    id: &'static str,
    name: &'static str,
    city: &'static str,
    line: &'static str,
}

const ORGANIZATIONS: [Org; 2] = [
    Org {
        id: "org-harbor",
        name: "Harbor Clinic",
        city: "Bergen",
        line: "Kaigaten 4",
    },
    Org {
        id: "org-juniper",
        name: "Juniper Health Center",
        city: "Troms\u{f8}",
        line: "Storgata 81",
    },
];

struct Prac {
    id: &'static str,
    given: &'static str,
    family: &'static str,
    gender: &'static str,
    org: &'static str,
}

const PRACTITIONERS: [Prac; 3] = [
    Prac {
        id: "prac-01",
        given: "Hedda",
        family: "Solberg",
        gender: "female",
        org: "org-harbor",
    },
    Prac {
        id: "prac-02",
        given: "Viktor",
        family: "Aasen",
        gender: "male",
        org: "org-harbor",
    },
    Prac {
        id: "prac-03",
        given: "Runa",
        family: "Eide",
        gender: "female",
        org: "org-juniper",
    },
];

/// An invented patient: name, gender, birth date, city, and a base for their measurements.
struct Pat {
    given: &'static [&'static str],
    family: &'static str,
    gender: &'static str,
    born: &'static str,
    city: &'static str,
    weight: f64,
    pulse: f64,
    systolic: f64,
}

const fn pat(
    given: &'static [&'static str],
    family: &'static str,
    gender: &'static str,
    born: &'static str,
    city: &'static str,
    [weight, pulse, systolic]: [f64; 3],
) -> Pat {
    Pat {
        given,
        family,
        gender,
        born,
        city,
        weight,
        pulse,
        systolic,
    }
}

const TROMSO: &str = "Troms\u{f8}";

const PATIENTS: [Pat; 12] = [
    pat(
        &["Solveig"],
        "Brekke",
        "female",
        "1958-03-14",
        "Bergen",
        [68.0, 71.0, 148.0],
    ),
    pat(
        &["Tor", "Erik"],
        "Nyland",
        "male",
        "1971-09-02",
        "Bergen",
        [92.0, 78.0, 139.0],
    ),
    pat(
        &["Ingrid"],
        "Vik",
        "female",
        "1989-12-21",
        "Bergen",
        [61.0, 64.0, 117.0],
    ),
    pat(
        &["Emil"],
        "Haugland",
        "male",
        "2004-05-30",
        "Bergen",
        [74.0, 59.0, 121.0],
    ),
    pat(
        &["Mira"],
        "Lunde",
        "female",
        "1966-07-08",
        "Bergen",
        [83.0, 76.0, 152.0],
    ),
    pat(
        &["Jonas"],
        "Berget",
        "male",
        "1949-01-17",
        "Bergen",
        [79.0, 69.0, 161.0],
    ),
    pat(&["Astrid"], "Moen", "female", "1995-10-03", TROMSO, [57.0, 66.0, 112.0]),
    pat(
        &["Liam"],
        "Strandli",
        "male",
        "1982-04-26",
        TROMSO,
        [101.0, 84.0, 144.0],
    ),
    pat(&["Nora"], "Dalen", "female", "1977-02-11", TROMSO, [72.0, 73.0, 128.0]),
    pat(
        &["Henrik"],
        "Fjellby",
        "male",
        "1963-11-29",
        TROMSO,
        [88.0, 81.0, 157.0],
    ),
    pat(&["Selma"], "Aune", "female", "2011-06-15", TROMSO, [39.0, 88.0, 104.0]),
    pat(&["Oskar"], "Lie", "unknown", "1938-08-09", TROMSO, [66.0, 62.0, 166.0]),
];

/// A condition a patient has, by the patient's place in the list.
struct Cond {
    patient: usize,
    code: &'static str,
    display: &'static str,
    onset: &'static str,
    status: &'static str,
}

const fn cond(
    patient: usize,
    code: &'static str,
    display: &'static str,
    onset: &'static str,
    status: &'static str,
) -> Cond {
    Cond {
        patient,
        code,
        display,
        onset,
        status,
    }
}

const HYPERTENSIVE: &str = "Hypertensive disorder";
const DIABETES: &str = "Diabetes mellitus type 2";

const CONDITIONS: [Cond; 11] = [
    cond(0, "38341003", HYPERTENSIVE, "2019-04-02", "active"),
    cond(1, "44054006", DIABETES, "2021-01-19", "active"),
    cond(1, "38341003", HYPERTENSIVE, "2022-08-30", "active"),
    cond(4, "38341003", HYPERTENSIVE, "2017-10-11", "active"),
    cond(5, "38341003", HYPERTENSIVE, "2012-03-05", "active"),
    cond(5, "44054006", DIABETES, "2015-06-22", "active"),
    cond(6, "195967001", "Asthma", "2008-09-14", "active"),
    cond(7, "44054006", DIABETES, "2024-02-07", "active"),
    cond(9, "38341003", HYPERTENSIVE, "2020-12-01", "active"),
    cond(10, "195967001", "Asthma", "2016-03-28", "resolved"),
    cond(11, "38341003", HYPERTENSIVE, "2005-07-19", "active"),
];

/// The days of the year's six visits, the same for every patient but shifted by a few days each.
const VISIT_DAYS: [&str; 6] = [
    "2026-01-12",
    "2026-03-09",
    "2026-05-04",
    "2026-07-06",
    "2026-09-07",
    "2026-10-26",
];

/// A fixed wobble for the n-th measurement, so the series look measured without being random.
fn wobble(n: f64, size: f64) -> f64 {
    round((n * 1.7 + 0.4).sin() * size * 10.0) / 10.0
}

fn pad(n: usize, width: usize) -> String {
    format!("{n:0width$}")
}

fn shift(day: &str, days: i64) -> String {
    let parts: Vec<i64> = day.split('-').map(|one| one.parse().unwrap_or(0)).collect();
    iso_day(days_from_civil(parts[0], parts[1], parts[2]) + days)
}

fn meta(at: &str) -> Value {
    json!({ "versionId": "1", "lastUpdated": at })
}

fn organizations() -> Vec<Resource> {
    ORGANIZATIONS
        .iter()
        .map(|org| {
            json!({
                "resourceType": "Organization",
                "id": org.id,
                "meta": meta("2026-01-02T08:00:00Z"),
                "active": true,
                "name": org.name,
                "telecom": [{ "system": "email", "value": format!("{}@example.org", &org.id[4..]), "use": "work" }],
                "address": [{ "use": "work", "line": [org.line], "city": org.city, "country": "Norway" }],
            })
        })
        .collect()
}

fn practitioners() -> Vec<Resource> {
    PRACTITIONERS
        .iter()
        .enumerate()
        .map(|(index, prac)| {
            json!({
                "resourceType": "Practitioner",
                "id": prac.id,
                "meta": meta("2026-01-02T08:30:00Z"),
                "identifier": [{ "system": STAFF_NUMBER, "value": format!("S-{}", pad(index + 41, 3)) }],
                "active": true,
                "name": [{ "use": "official", "family": prac.family, "given": [prac.given], "prefix": ["Dr."] }],
                "gender": prac.gender,
                "telecom": [{
                    "system": "email",
                    "value": format!("{}.{}@example.org", prac.given.to_lowercase(), prac.family.to_lowercase()),
                    "use": "work",
                }],
                "qualification": [{
                    "code": { "coding": [{ "system": SNOMED, "code": "309343006", "display": "Physician" }], "text": "Physician" },
                    "issuer": { "reference": format!("Organization/{}", prac.org) },
                }],
            })
        })
        .collect()
}

fn patient_id(index: usize) -> String {
    format!("pat-{}", pad(index + 1, 3))
}

fn org_of(index: usize) -> &'static str {
    if PATIENTS.get(index).is_some_and(|one| one.city == "Bergen") {
        "org-harbor"
    } else {
        "org-juniper"
    }
}

fn prac_of(index: usize) -> &'static str {
    if org_of(index) == "org-harbor" {
        if index.is_multiple_of(2) { "prac-01" } else { "prac-02" }
    } else {
        "prac-03"
    }
}

fn name_of(index: usize) -> String {
    PATIENTS
        .get(index)
        .map(|one| format!("{} {}", one.given.join(" "), one.family))
        .unwrap_or_default()
}

fn patients() -> Vec<Resource> {
    PATIENTS
        .iter()
        .enumerate()
        .map(|(index, patient)| {
            let street = ["Fjordveien", "Sj\u{f8}gata", "Bakken", "Lia"][index % 4];
            json!({
                "resourceType": "Patient",
                "id": patient_id(index),
                "meta": meta("2026-01-05T09:00:00Z"),
                "identifier": [{ "use": "usual", "system": PATIENT_NUMBER, "value": format!("P-{}", pad(index + 1001, 4)) }],
                "active": true,
                "name": [{ "use": "official", "family": patient.family, "given": patient.given }],
                "telecom": [{
                    "system": "phone",
                    "value": format!("+47 400 {} {}", pad(index * 37 + 120, 3), pad(index * 11 + 10, 2)),
                    "use": "mobile",
                }],
                "gender": patient.gender,
                "birthDate": patient.born,
                "address": [{ "use": "home", "line": [format!("{street} {}", index + 3)], "city": patient.city, "country": "Norway" }],
                "generalPractitioner": [{ "reference": format!("Practitioner/{}", prac_of(index)) }],
                "managingOrganization": { "reference": format!("Organization/{}", org_of(index)) },
            })
        })
        .collect()
}

fn encounters() -> Vec<Resource> {
    (0..PATIENTS.len())
        .flat_map(|index| {
            VISIT_DAYS.iter().enumerate().map(move |(visit, day)| {
                let date = shift(day, (index % 5) as i64);
                json!({
                    "resourceType": "Encounter",
                    "id": format!("enc-{}-{}", pad(index + 1, 3), visit + 1),
                    "meta": meta(&format!("{date}T11:00:00Z")),
                    "status": "finished",
                    "class": { "system": ACT_CODE, "code": "AMB", "display": "ambulatory" },
                    "type": [{ "coding": [{ "system": SNOMED, "code": "185349003", "display": "Encounter for check up" }], "text": "Check-up" }],
                    "subject": { "reference": format!("Patient/{}", patient_id(index)), "display": name_of(index) },
                    "participant": [{ "individual": { "reference": format!("Practitioner/{}", prac_of(index)) } }],
                    "period": { "start": format!("{date}T09:00:00Z"), "end": format!("{date}T09:40:00Z") },
                    "serviceProvider": { "reference": format!("Organization/{}", org_of(index)) },
                })
            })
        })
        .collect()
}

/// A patient's three measurements at a visit: weight, heart rate and systolic pressure.
fn measured(index: usize, visit: usize) -> [f64; 3] {
    let patient = &PATIENTS[index];
    let n = (index * 10 + visit) as f64;
    let visit = visit as f64;
    [
        round((patient.weight + wobble(n, 1.6) - visit * 0.3) * 10.0) / 10.0,
        round(patient.pulse + wobble(n + 3.0, 6.0)),
        round(patient.systolic + wobble(n + 5.0, 9.0) - if patient.systolic > 140.0 { visit * 2.0 } else { 0.0 }),
    ]
}

fn observations() -> Vec<Resource> {
    (0..PATIENTS.len())
        .flat_map(|index| {
            VISIT_DAYS.iter().enumerate().flat_map(move |(visit, day)| {
                let date = shift(day, (index % 5) as i64);
                let at = format!("{date}T09:{}:00Z", pad(10 + (index % 5) * 5, 2));
                let values = measured(index, visit);
                VITALS.iter().zip(values).map(move |(vital, value)| {
                    json!({
                        "resourceType": "Observation",
                        "id": format!("obs-{}-{}-{}", pad(index + 1, 3), visit + 1, vital.id),
                        "meta": meta(&format!("{date}T11:00:00Z")),
                        "status": "final",
                        "category": [{ "coding": [{ "system": CATEGORY, "code": "vital-signs", "display": "Vital Signs" }] }],
                        "code": { "coding": [{ "system": LOINC, "code": vital.code, "display": vital.display }], "text": vital.display },
                        "subject": { "reference": format!("Patient/{}", patient_id(index)), "display": name_of(index) },
                        "encounter": { "reference": format!("Encounter/enc-{}-{}", pad(index + 1, 3), visit + 1) },
                        "effectiveDateTime": at,
                        "performer": [{ "reference": format!("Practitioner/{}", prac_of(index)) }],
                        "valueQuantity": { "value": number(value), "unit": vital.unit, "system": UCUM, "code": vital.unit },
                    })
                })
            })
        })
        .collect()
}

fn conditions() -> Vec<Resource> {
    CONDITIONS
        .iter()
        .enumerate()
        .map(|(index, condition)| {
            json!({
                "resourceType": "Condition",
                "id": format!("cond-{}", pad(index + 1, 3)),
                "meta": meta("2026-01-05T09:30:00Z"),
                "clinicalStatus": { "coding": [{ "system": CLINICAL, "code": condition.status }] },
                "verificationStatus": { "coding": [{ "system": VERIFICATION, "code": "confirmed" }] },
                "category": [{ "coding": [{
                    "system": "http://terminology.hl7.org/CodeSystem/condition-category",
                    "code": "problem-list-item",
                    "display": "Problem List Item",
                }] }],
                "code": { "coding": [{ "system": SNOMED, "code": condition.code, "display": condition.display }], "text": condition.display },
                "subject": { "reference": format!("Patient/{}", patient_id(condition.patient)), "display": name_of(condition.patient) },
                "onsetDateTime": condition.onset,
                "recordedDate": condition.onset,
                "recorder": { "reference": format!("Practitioner/{}", prac_of(condition.patient)) },
            })
        })
        .collect()
}

const HYPERTENSION_V1: &str = "library HypertensionControl version '1.0.0'

using FHIR version '4.0.1'

codesystem \"LOINC\": 'http://loinc.org'
codesystem \"SNOMED\": 'http://snomed.info/sct'

code \"Systolic blood pressure\": '8480-6' from \"LOINC\"
code \"Hypertensive disorder\": '38341003' from \"SNOMED\"

parameter \"Measurement Period\" Interval<DateTime>
  default Interval[@2026-01-01T00:00:00.0, @2027-01-01T00:00:00.0)

context Patient

define \"Has Hypertension\":
  exists ([Condition: \"Hypertensive disorder\"] C
    where C.clinicalStatus ~ 'active')

define \"Latest Systolic\":
  Last([Observation: \"Systolic blood pressure\"] O
    where O.effective during \"Measurement Period\"
    sort by effective).value as Quantity

define \"Controlled\":
  \"Has Hypertension\" and \"Latest Systolic\" < 140 'mm[Hg]'
";

/// `text.replace(from, to)`: the first `from` replaced, as a string pattern replaces.
fn replace_first(text: &str, from: &str, to: &str) -> String {
    text.replacen(from, to, 1)
}

fn hypertension_v2() -> String {
    replace_first(
        &replace_first(HYPERTENSION_V1, "version '1.0.0'", "version '1.1.0'"),
        "define \"Controlled\":
  \"Has Hypertension\" and \"Latest Systolic\" < 140 'mm[Hg]'
",
        "// Under 140 counts as controlled; older patients get until 150.
define \"Is Older\":
  AgeInYearsAt(start of \"Measurement Period\") >= 80

define \"Controlled\":
  \"Has Hypertension\"
    and \"Latest Systolic\" < (if \"Is Older\" then 150 'mm[Hg]' else 140 'mm[Hg]')
",
    )
}

fn hypertension_v3() -> String {
    replace_first(
        &hypertension_v2(),
        "  Last([Observation: \"Systolic blood pressure\"] O
    where O.effective during \"Measurement Period\"
    sort by effective).value as Quantity",
        "  Last([Observation: \"Systolic blood pressure\"] O
    where O.status in { 'final', 'amended' }
      and O.effective during \"Measurement Period\"
    sort by effective).value as Quantity",
    )
}

const WEIGHT_V1: &str = "library WeightFollowUp version '0.1.0'

using FHIR version '4.0.1'

codesystem \"LOINC\": 'http://loinc.org'

code \"Body weight\": '29463-7' from \"LOINC\"

context Patient

define \"Weights\":
  [Observation: \"Body weight\"] O
    sort by effective

define \"First Weight\": First(\"Weights\").value as Quantity
define \"Latest Weight\": Last(\"Weights\").value as Quantity
";

fn weight_v2() -> String {
    format!(
        "{WEIGHT_V1}
/* A loss of five percent or more over the year asks for a follow-up. */
define \"Lost Weight\":
  \"Latest Weight\" < \"First Weight\" * 0.95
"
    )
}

/// A library's text and facts, as one version holds them.
struct LibraryVersion<'a> {
    id: &'a str,
    name: &'a str,
    title: &'a str,
    version: &'a str,
    status: &'a str,
    description: &'a str,
    cql: &'a str,
    at: &'a str,
    version_id: &'a str,
}

fn library(one: &LibraryVersion) -> Resource {
    json!({
        "resourceType": "Library",
        "id": one.id,
        "meta": { "versionId": one.version_id, "lastUpdated": one.at },
        "url": format!("{LIBRARY_BASE}/{}", one.name),
        "version": one.version,
        "name": one.name,
        "title": one.title,
        "status": one.status,
        "type": { "coding": [{ "system": LIBRARY_TYPE, "code": "logic-library" }] },
        "date": one.at,
        "publisher": "Tidevann",
        "description": one.description,
        "content": [{ "contentType": "text/cql", "data": to_base64(one.cql) }],
    })
}

fn libraries() -> Vec<Seeded> {
    let hypertension = |version: &str, status: &str, cql: &str, at: &str, version_id: &str| {
        library(&LibraryVersion {
            id: "lib-hypertension",
            name: "HypertensionControl",
            title: "Hypertension control",
            version,
            status,
            description: "Whether a patient with hypertension had their systolic pressure under control at the last measurement.",
            cql,
            at,
            version_id,
        })
    };
    let weight = |cql: &str, at: &str, version_id: &str| {
        library(&LibraryVersion {
            id: "lib-weight",
            name: "WeightFollowUp",
            title: "Weight follow-up",
            version: "0.1.0",
            status: "draft",
            description: "Patients whose weight fell over the year, for a follow-up visit.",
            cql,
            at,
            version_id,
        })
    };
    vec![
        Seeded {
            resource: hypertension("1.1.0", "active", &hypertension_v3(), "2026-10-20T13:12:00Z", "3"),
            earlier: vec![
                (
                    "2026-03-02T10:00:00Z".to_owned(),
                    hypertension("1.0.0", "active", HYPERTENSION_V1, "2026-03-02T10:00:00Z", "1"),
                ),
                (
                    "2026-08-14T15:40:00Z".to_owned(),
                    hypertension("1.1.0", "draft", &hypertension_v2(), "2026-08-14T15:40:00Z", "2"),
                ),
            ],
        },
        Seeded {
            resource: weight(&weight_v2(), "2026-11-03T09:05:00Z", "2"),
            earlier: vec![(
                "2026-10-29T14:20:00Z".to_owned(),
                weight(WEIGHT_V1, "2026-10-29T14:20:00Z", "1"),
            )],
        },
    ]
}

/// Every resource the server starts with, with the earlier versions of those that have them.
pub fn seed() -> Vec<Seeded> {
    let mut plain = organizations();
    plain.extend(practitioners());
    plain.extend(patients());
    plain.extend(encounters());
    plain.extend(observations());
    plain.extend(conditions());
    let mut out: Vec<Seeded> = plain
        .into_iter()
        .map(|resource| Seeded {
            resource,
            earlier: Vec::new(),
        })
        .collect();
    out.extend(libraries());
    out
}

/// A new resource of a type, as the editor opens it: the elements it must have, and a few it
/// usually has.
pub fn template(kind: &str) -> Resource {
    let mut base = json!({ "resourceType": kind, "id": "" });
    let rest = match kind {
        "Patient" => {
            json!({ "active": true, "name": [{ "use": "official", "family": "", "given": [""] }], "gender": "unknown", "birthDate": "" })
        }
        "Practitioner" => json!({ "active": true, "name": [{ "use": "official", "family": "", "given": [""] }] }),
        "Organization" => json!({ "active": true, "name": "" }),
        "Encounter" => json!({
            "status": "planned",
            "class": { "system": ACT_CODE, "code": "AMB", "display": "ambulatory" },
            "subject": { "reference": "Patient/pat-001" },
        }),
        "Observation" => json!({
            "status": "preliminary",
            "code": { "coding": [{ "system": LOINC, "code": "8867-4", "display": "Heart rate" }] },
            "subject": { "reference": "Patient/pat-001" },
            "valueQuantity": { "value": 70, "unit": "/min", "system": UCUM, "code": "/min" },
        }),
        "Condition" => json!({
            "clinicalStatus": { "coding": [{ "system": CLINICAL, "code": "active" }] },
            "code": { "text": "" },
            "subject": { "reference": "Patient/pat-001" },
        }),
        "Library" => json!({
            "status": "draft",
            "type": { "coding": [{ "system": LIBRARY_TYPE, "code": "logic-library" }] },
            "content": [{
                "contentType": "text/cql",
                "data": to_base64("library NewLibrary version '0.1.0'\n\nusing FHIR version '4.0.1'\n\ncontext Patient\n"),
            }],
        }),
        _ => json!({}),
    };
    if let (Some(fields), Value::Object(more)) = (base.as_object_mut(), rest) {
        fields.extend(more);
    }
    base
}
