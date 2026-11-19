//! The resource types the server keeps and their elements, as the web's `data/types.ts` lists them:
//! what each element holds, whether it repeats or is required, the codes it takes and the types it
//! may point at. The server checks a resource against them, the editor gets them as a JSON Schema,
//! and the listings read them.

use serde_json::{Map, Value, json};

/// One element of a type: its name, what it holds, and what the server asks of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElementDef {
    pub name: &'static str,
    /// A primitive (`string`, `code`, `date`, ...) or a data type made of elements (`HumanName`, ...).
    pub kind: &'static str,
    pub array: bool,
    pub required: bool,
    /// The codes a `code` element takes, from its required value set.
    pub codes: Option<&'static [&'static str]>,
    /// The resource types a Reference may point at.
    pub targets: Option<&'static [&'static str]>,
    pub doc: &'static str,
}

/// A resource type: its name, what one is, and its elements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDef {
    pub name: &'static str,
    /// What one resource of the type is, for the listings.
    pub about: &'static str,
    pub elements: Vec<ElementDef>,
}

const fn el(name: &'static str, kind: &'static str, doc: &'static str) -> ElementDef {
    ElementDef {
        name,
        kind,
        array: false,
        required: false,
        codes: None,
        targets: None,
        doc,
    }
}

const fn many(mut one: ElementDef) -> ElementDef {
    one.array = true;
    one
}

const fn must(mut one: ElementDef) -> ElementDef {
    one.required = true;
    one
}

const fn coded(mut one: ElementDef, codes: &'static [&'static str]) -> ElementDef {
    one.codes = Some(codes);
    one
}

const fn to(mut one: ElementDef, targets: &'static [&'static str]) -> ElementDef {
    one.targets = Some(targets);
    one
}

const GENDERS: &[&str] = &["male", "female", "other", "unknown"];

fn base() -> Vec<ElementDef> {
    vec![
        must(el("resourceType", "code", "The type of the resource.")),
        el("id", "id", "The logical id of the resource, as the server names it."),
        el(
            "meta",
            "Meta",
            "The version and the time of the last change, which the server keeps.",
        ),
        el("text", "Narrative", "A summary for a person to read."),
    ]
}

/// The data types made of elements, in the order the web lists them.
pub const DATATYPE_NAMES: [&str; 16] = [
    "HumanName",
    "Identifier",
    "ContactPoint",
    "Address",
    "CodeableConcept",
    "Coding",
    "Reference",
    "Period",
    "Quantity",
    "Attachment",
    "Meta",
    "Narrative",
    "Annotation",
    "Participant",
    "Qualification",
    "Component",
];

/// The elements a data type is made of; none for a primitive.
pub fn datatype(name: &str) -> Option<Vec<ElementDef>> {
    Some(match name {
        "HumanName" => vec![
            coded(
                el("use", "code", "The purpose of this name."),
                &["usual", "official", "temp", "nickname", "anonymous", "old", "maiden"],
            ),
            el("text", "string", "The name as a whole."),
            el("family", "string", "The family name."),
            many(el("given", "string", "Given names, the first first.")),
            many(el("prefix", "string", "Parts before the name.")),
            many(el("suffix", "string", "Parts after the name.")),
            el("period", "Period", "When the name was in use."),
        ],
        "Identifier" => vec![
            coded(
                el("use", "code", "The purpose of this identifier."),
                &["usual", "official", "temp", "secondary", "old"],
            ),
            el("system", "uri", "The namespace of the value."),
            el("value", "string", "The value, unique in the system."),
        ],
        "ContactPoint" => vec![
            coded(
                el("system", "code", "The kind of contact."),
                &["phone", "fax", "email", "pager", "url", "sms", "other"],
            ),
            el("value", "string", "The address or number."),
            coded(
                el("use", "code", "The purpose of this contact."),
                &["home", "work", "temp", "old", "mobile"],
            ),
        ],
        "Address" => vec![
            coded(
                el("use", "code", "The purpose of this address."),
                &["home", "work", "temp", "old", "billing"],
            ),
            many(el("line", "string", "Street, number and the like.")),
            el("city", "string", "The city or town."),
            el("postalCode", "string", "The postal code."),
            el("country", "string", "The country."),
        ],
        "CodeableConcept" => vec![
            many(el("coding", "Coding", "Codes from terminologies.")),
            el("text", "string", "The concept in plain words."),
        ],
        "Coding" => vec![
            el("system", "uri", "The terminology the code is from."),
            el("code", "code", "The code."),
            el("display", "string", "The code in words."),
        ],
        "Reference" => vec![
            el("reference", "string", "The target as `Type/id`."),
            el("display", "string", "The target in words."),
        ],
        "Period" => vec![
            el("start", "dateTime", "When it starts."),
            el("end", "dateTime", "When it ends."),
        ],
        "Quantity" => vec![
            el("value", "decimal", "The amount."),
            el("unit", "string", "The unit in words."),
            el("system", "uri", "The system the unit is coded in."),
            el("code", "code", "The unit as a code."),
        ],
        "Attachment" => vec![
            el("contentType", "code", "The media type of the content."),
            el("data", "string", "The content in base64."),
            el("title", "string", "What to call the content."),
        ],
        "Meta" => vec![
            el("versionId", "id", "The version, which the server sets."),
            el(
                "lastUpdated",
                "instant",
                "When the version was saved, which the server sets.",
            ),
        ],
        "Narrative" => vec![
            coded(
                el("status", "code", "Where the text came from."),
                &["generated", "extensions", "additional", "empty"],
            ),
            el("div", "string", "The text as limited XHTML."),
        ],
        "Annotation" => vec![
            el("authorString", "string", "Who wrote it."),
            el("time", "dateTime", "When it was written."),
            el("text", "markdown", "The note."),
        ],
        "Participant" => vec![
            many(el("type", "CodeableConcept", "The role in the encounter.")),
            to(el("individual", "Reference", "Who took part."), &["Practitioner"]),
        ],
        "Qualification" => vec![
            el("code", "CodeableConcept", "The qualification."),
            to(el("issuer", "Reference", "Who issued it."), &["Organization"]),
        ],
        "Component" => vec![
            must(el("code", "CodeableConcept", "What the component measures.")),
            el("valueQuantity", "Quantity", "The component as a quantity."),
        ],
        _ => return None,
    })
}

/// The resource types the server keeps, in the order the web lists them.
pub fn types() -> Vec<TypeDef> {
    let with = |rest: Vec<ElementDef>| {
        let mut all = base();
        all.extend(rest);
        all
    };
    vec![
        TypeDef {
            name: "Patient",
            about: "A person the clinics care for",
            elements: with(vec![
                many(el("identifier", "Identifier", "Numbers the patient is known by.")),
                el("active", "boolean", "Whether the record is in use."),
                many(el("name", "HumanName", "The names the patient goes by.")),
                many(el("telecom", "ContactPoint", "How to reach the patient.")),
                coded(el("gender", "code", "The gender for administration."), GENDERS),
                el("birthDate", "date", "The date of birth."),
                el("deceasedBoolean", "boolean", "Whether the patient has died."),
                many(el("address", "Address", "Where the patient lives.")),
                to(
                    many(el(
                        "generalPractitioner",
                        "Reference",
                        "The patient\u{2019}s own practitioner or clinic.",
                    )),
                    &["Practitioner", "Organization"],
                ),
                to(
                    el("managingOrganization", "Reference", "The clinic that keeps the record."),
                    &["Organization"],
                ),
            ]),
        },
        TypeDef {
            name: "Practitioner",
            about: "A person who gives care",
            elements: with(vec![
                many(el("identifier", "Identifier", "Numbers the practitioner is known by.")),
                el("active", "boolean", "Whether the record is in use."),
                many(el("name", "HumanName", "The names the practitioner goes by.")),
                many(el("telecom", "ContactPoint", "How to reach the practitioner.")),
                coded(el("gender", "code", "The gender for administration."), GENDERS),
                many(el("qualification", "Qualification", "Degrees, licenses and the like.")),
            ]),
        },
        TypeDef {
            name: "Organization",
            about: "A clinic or another body",
            elements: with(vec![
                many(el("identifier", "Identifier", "Numbers the organization is known by.")),
                el("active", "boolean", "Whether the record is in use."),
                el("name", "string", "The name the organization goes by."),
                many(el("telecom", "ContactPoint", "How to reach the organization.")),
                many(el("address", "Address", "Where the organization is.")),
            ]),
        },
        TypeDef {
            name: "Encounter",
            about: "A visit, a stay or a call",
            elements: with(vec![
                must(coded(
                    el("status", "code", "Where the encounter stands."),
                    &[
                        "planned",
                        "arrived",
                        "triaged",
                        "in-progress",
                        "onleave",
                        "finished",
                        "cancelled",
                        "entered-in-error",
                        "unknown",
                    ],
                )),
                must(el("class", "Coding", "Ambulatory, inpatient, virtual and the like.")),
                many(el("type", "CodeableConcept", "What kind of encounter it is.")),
                to(el("subject", "Reference", "The patient."), &["Patient"]),
                many(el("participant", "Participant", "Who took part.")),
                el("period", "Period", "When it took place."),
                many(el("reasonCode", "CodeableConcept", "Why it took place.")),
                to(
                    el("serviceProvider", "Reference", "The clinic that gave the care."),
                    &["Organization"],
                ),
            ]),
        },
        TypeDef {
            name: "Observation",
            about: "A measurement or a finding",
            elements: with(vec![
                must(coded(
                    el("status", "code", "Where the result stands."),
                    &[
                        "registered",
                        "preliminary",
                        "final",
                        "amended",
                        "corrected",
                        "cancelled",
                        "entered-in-error",
                        "unknown",
                    ],
                )),
                many(el(
                    "category",
                    "CodeableConcept",
                    "The kind of observation, such as vital signs.",
                )),
                must(el("code", "CodeableConcept", "What was observed.")),
                to(el("subject", "Reference", "Who it is about."), &["Patient"]),
                to(
                    el("encounter", "Reference", "The encounter it was made in."),
                    &["Encounter"],
                ),
                el("effectiveDateTime", "dateTime", "When it was observed."),
                el("issued", "instant", "When it was made available."),
                to(
                    many(el("performer", "Reference", "Who is responsible.")),
                    &["Practitioner", "Organization"],
                ),
                el("valueQuantity", "Quantity", "The result as a quantity."),
                el("valueString", "string", "The result as text."),
                el("valueCodeableConcept", "CodeableConcept", "The result as a code."),
                many(el(
                    "interpretation",
                    "CodeableConcept",
                    "High, low, normal and the like.",
                )),
                many(el("note", "Annotation", "Comments on the result.")),
                many(el(
                    "component",
                    "Component",
                    "Results measured together, such as both blood pressures.",
                )),
            ]),
        },
        TypeDef {
            name: "Condition",
            about: "A problem, a diagnosis or a concern",
            elements: with(vec![
                el(
                    "clinicalStatus",
                    "CodeableConcept",
                    "Active, recurrence, remission or resolved.",
                ),
                el("verificationStatus", "CodeableConcept", "How certain it is."),
                many(el(
                    "category",
                    "CodeableConcept",
                    "A problem list item or an encounter diagnosis.",
                )),
                el("severity", "CodeableConcept", "How severe it is."),
                el("code", "CodeableConcept", "The condition."),
                must(to(el("subject", "Reference", "Who has the condition."), &["Patient"])),
                to(
                    el("encounter", "Reference", "The encounter it was recorded in."),
                    &["Encounter"],
                ),
                el("onsetDateTime", "dateTime", "When it started."),
                el("abatementDateTime", "dateTime", "When it ended."),
                el("recordedDate", "dateTime", "When it was first recorded."),
                to(el("recorder", "Reference", "Who recorded it."), &["Practitioner"]),
                many(el("note", "Annotation", "Comments on the condition.")),
            ]),
        },
        TypeDef {
            name: "Library",
            about: "A body of logic, such as CQL",
            elements: with(vec![
                el("url", "uri", "The canonical address of the library."),
                el("version", "string", "The business version, which the CQL names."),
                el("name", "string", "The name for machines, which the CQL names."),
                el("title", "string", "The name for people."),
                must(coded(
                    el("status", "code", "Where the library stands."),
                    &["draft", "active", "retired", "unknown"],
                )),
                must(el(
                    "type",
                    "CodeableConcept",
                    "Logic library, model definition and the like.",
                )),
                el("date", "dateTime", "When it last changed in substance."),
                el("publisher", "string", "Who publishes it."),
                el("description", "markdown", "What the library is for."),
                many(el("content", "Attachment", "The library\u{2019}s text, such as CQL.")),
            ]),
        },
    ]
}

/// The names of the types the server keeps.
pub const TYPE_NAMES: [&str; 7] = [
    "Patient",
    "Practitioner",
    "Organization",
    "Encounter",
    "Observation",
    "Condition",
    "Library",
];

/// A type the server keeps, by its name.
pub fn type_def(name: &str) -> Option<TypeDef> {
    types().into_iter().find(|one| one.name == name)
}

/// Whether a type is a primitive rather than one made of elements.
pub fn is_primitive(kind: &str) -> bool {
    !DATATYPE_NAMES.contains(&kind)
}

/// The pattern a primitive's text must match, as the web writes it.
pub fn pattern_text(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "id" => "^[A-Za-z0-9\\-.]{1,64}$",
        "date" => "^\\d{4}(-\\d{2}(-\\d{2})?)?$",
        "dateTime" => "^\\d{4}(-\\d{2}(-\\d{2}(T\\d{2}:\\d{2}(:\\d{2}(\\.\\d+)?)?(Z|[+-]\\d{2}:\\d{2}))?)?)?$",
        "instant" => "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}(\\.\\d+)?(Z|[+-]\\d{2}:\\d{2})$",
        _ => return None,
    })
}

/// A reader over a text's characters, for the patterns below.
struct Scan {
    chars: Vec<char>,
    at: usize,
}

impl Scan {
    fn new(text: &str) -> Self {
        Scan {
            chars: text.chars().collect(),
            at: 0,
        }
    }

    fn digits(&mut self, count: usize) -> bool {
        for _ in 0..count {
            if !self.chars.get(self.at).is_some_and(char::is_ascii_digit) {
                return false;
            }
            self.at += 1;
        }
        true
    }

    fn eat(&mut self, c: char) -> bool {
        if self.chars.get(self.at) == Some(&c) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn done(&self) -> bool {
        self.at == self.chars.len()
    }

    /// `(Z|[+-]\d{2}:\d{2})`.
    fn zone(&mut self) -> bool {
        if self.eat('Z') {
            return true;
        }
        (self.eat('+') || self.eat('-')) && self.digits(2) && self.eat(':') && self.digits(2)
    }

    /// `:\d{2}(\.\d+)?`, the seconds and their fraction.
    fn seconds(&mut self) -> bool {
        if !(self.eat(':') && self.digits(2)) {
            return false;
        }
        if self.eat('.') {
            if !self.digits(1) {
                return false;
            }
            while self.chars.get(self.at).is_some_and(char::is_ascii_digit) {
                self.at += 1;
            }
        }
        true
    }
}

/// Whether a primitive's text matches its type's pattern; true for a type without one.
pub fn matches_pattern(kind: &str, value: &str) -> bool {
    let mut scan = Scan::new(value);
    match kind {
        "id" => {
            let count = scan.chars.len();
            (1..=64).contains(&count)
                && scan
                    .chars
                    .iter()
                    .all(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '.')
        }
        "date" => {
            scan.digits(4)
                && (scan.done()
                    || (scan.eat('-') && scan.digits(2) && (scan.done() || (scan.eat('-') && scan.digits(2)))))
                && scan.done()
        }
        "dateTime" => {
            if !scan.digits(4) {
                return false;
            }
            if scan.done() {
                return true;
            }
            if !(scan.eat('-') && scan.digits(2)) {
                return false;
            }
            if scan.done() {
                return true;
            }
            if !(scan.eat('-') && scan.digits(2)) {
                return false;
            }
            if scan.done() {
                return true;
            }
            if !(scan.eat('T') && scan.digits(2) && scan.eat(':') && scan.digits(2)) {
                return false;
            }
            let back = scan.at;
            if !scan.seconds() {
                scan.at = back;
            }
            scan.zone() && scan.done()
        }
        "instant" => {
            scan.digits(4)
                && scan.eat('-')
                && scan.digits(2)
                && scan.eat('-')
                && scan.digits(2)
                && scan.eat('T')
                && scan.digits(2)
                && scan.eat(':')
                && scan.digits(2)
                && scan.seconds()
                && scan.zone()
                && scan.done()
        }
        _ => true,
    }
}

/// One element's schema, without the array around a repeating one.
fn single(element: &ElementDef) -> Value {
    let kind = element.kind;
    if !is_primitive(kind) {
        return json!({ "$ref": format!("#/$defs/{kind}"), "description": element.doc });
    }
    match kind {
        "boolean" => json!({ "type": "boolean", "description": element.doc }),
        "decimal" => json!({ "type": "number", "description": element.doc }),
        "integer" => json!({ "type": "integer", "description": element.doc }),
        _ => {
            let mut out = json!({ "type": "string", "description": element.doc });
            if let Some(codes) = element.codes {
                out["enum"] = json!(codes);
            }
            if let Some(pattern) = pattern_text(kind) {
                out["pattern"] = json!(pattern);
            }
            out
        }
    }
}

fn object_schema(elements: &[ElementDef]) -> Value {
    let required: Vec<&str> = elements.iter().filter(|one| one.required).map(|one| one.name).collect();
    let mut properties = Map::new();
    for element in elements {
        properties.insert(
            element.name.to_owned(),
            if element.array {
                json!({ "type": "array", "items": single(element), "minItems": 1, "description": element.doc })
            } else {
                single(element)
            },
        );
    }
    let mut out = json!({ "type": "object", "additionalProperties": false, "properties": properties });
    if !required.is_empty() {
        out["required"] = json!(required);
    }
    out
}

/// A JSON Schema (2020-12) for one resource type, for the editor to check and complete from.
pub fn schema_for(name: &str) -> Option<Value> {
    let def = type_def(name)?;
    let mut schema = object_schema(&def.elements);
    schema["properties"]["resourceType"] = json!({ "const": name, "description": "The type of the resource." });
    let mut out = Map::new();
    out.insert(
        "$schema".to_owned(),
        json!("https://json-schema.org/draft/2020-12/schema"),
    );
    for (key, value) in schema.as_object().cloned().unwrap_or_default() {
        out.insert(key, value);
    }
    let mut defs = Map::new();
    for kind in DATATYPE_NAMES {
        defs.insert(kind.to_owned(), object_schema(&datatype(kind).unwrap_or_default()));
    }
    out.insert("$defs".to_owned(), Value::Object(defs));
    Some(Value::Object(out))
}
