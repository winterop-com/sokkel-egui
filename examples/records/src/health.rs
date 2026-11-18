//! The metadata's health, as the web's `data/health.ts`: what the server's metadata holds that a
//! record cannot carry cleanly, and how far the forms are translated.

use crate::backend::Household;
use crate::forms::{FormKind, QuestionType, forms, questions_of};
use crate::geo::{Level, place_name, places};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl Severity {
    pub const ORDER: [Severity; 3] = [Severity::Error, Severity::Warning, Severity::Note];

    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "Errors",
            Severity::Warning => "Warnings",
            Severity::Note => "Notes",
        }
    }

    /// The token its dot is drawn in.
    pub fn dot(self) -> &'static str {
        match self {
            Severity::Error => "critical",
            Severity::Warning => "warning",
            Severity::Note => "faint",
        }
    }
}

/// Something the server's metadata holds that a record cannot carry cleanly.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    /// The kind of object: forms, questions, places, code lists, households.
    pub kind: &'static str,
    pub object: String,
    pub object_id: String,
    pub field: &'static str,
    pub problem: String,
    pub cost: &'static str,
    /// Where the object is shown in the app.
    pub href: Option<String>,
}

/// Each language's share of the strings the forms say, translated.
#[derive(Clone, Debug, PartialEq)]
pub struct Coverage {
    pub language: &'static str,
    pub name: &'static str,
    pub translated: usize,
    pub total: usize,
}

struct Found {
    severity: Severity,
    kind: &'static str,
    object: String,
    object_id: String,
    field: &'static str,
    problem: String,
    cost: &'static str,
    href: String,
}

/// Runs every check over the metadata and the households the server holds.
pub fn check_metadata(households: &[Household]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut add = |found: Found| {
        findings.push(Finding {
            id: format!("{}:{}:{}", found.kind, found.object_id, found.field),
            severity: found.severity,
            kind: found.kind,
            object: found.object,
            object_id: found.object_id,
            field: found.field,
            problem: found.problem,
            cost: found.cost,
            href: Some(found.href),
        })
    };

    for place in places() {
        if place.level == Level::Site && place.position.is_none() {
            add(Found {
                severity: Severity::Warning,
                kind: "places",
                object: place.name.clone(),
                object_id: place.id.clone(),
                field: "position",
                problem: "The position cannot be read: a coordinate is not a number.".into(),
                cost: "The site is left off the map, and the map frames its region in its place.",
                href: format!("places/{}", place.id),
            });
        }
    }

    let mut seen: Vec<(String, String)> = Vec::new();
    for place in places().iter().filter(|one| one.level == Level::Site) {
        let key = format!(
            "{}:{}",
            place.parent.as_deref().unwrap_or(""),
            place.name.to_lowercase()
        );
        match seen.iter().find(|(k, _)| *k == key) {
            None => seen.push((key, place.id.clone())),
            Some((_, first)) => add(Found {
                severity: Severity::Note,
                kind: "places",
                object: place.name.clone(),
                object_id: place.id.clone(),
                field: "name",
                problem: format!(
                    "Shares its name with {first} in {}.",
                    place.parent.as_deref().map(place_name).unwrap_or_default()
                ),
                cost: "Pickers tell the two apart by their ids only.",
                href: format!("places/{}", place.id),
            }),
        }
    }

    for form in forms() {
        for question in questions_of(form) {
            if question.kind == QuestionType::Computed {
                continue;
            }
            if !question.required && question.description.is_none() && question.help.is_none() {
                add(Found {
                    severity: Severity::Note,
                    kind: "questions",
                    object: question.label.into(),
                    object_id: format!("{}.{}", form.id, question.id),
                    field: "description",
                    problem: "An optional question with no description or help.".into(),
                    cost: "Nothing tells the person filling it in when to leave it empty.",
                    href: format!("forms/{}", form.id),
                });
            }
            if matches!(question.kind, QuestionType::Integer | QuestionType::Decimal) && question.max.is_none() {
                add(Found {
                    severity: Severity::Warning,
                    kind: "questions",
                    object: question.label.into(),
                    object_id: format!("{}.{}", form.id, question.id),
                    field: "max",
                    problem: "A number with no highest value.".into(),
                    cost: "A slip of the finger is filed as it is typed.",
                    href: format!("forms/{}", form.id),
                });
            }
        }
        if form.kind == FormKind::Followup && !form.repeats && form.id == "FRM-EXIT" {
            add(Found {
                severity: Severity::Note,
                kind: "forms",
                object: form.title.into(),
                object_id: form.id.into(),
                field: "translations",
                problem: "Not yet translated into any language.".into(),
                cost: "It is shown in English to everyone.",
                href: format!("forms/{}", form.id),
            });
        }
    }

    add(Found {
        severity: Severity::Warning,
        kind: "code lists",
        object: "Main water source".into(),
        object_id: "LST-SOURCE.standpipe".into(),
        field: "code",
        problem: "The code \"public standpipe\" holds a space.".into(),
        cost: "The register files it as \"public_standpipe\", so a search by the code misses it.",
        href: "forms/FRM-ENROLL".into(),
    });

    let mut by_identifier: Vec<(&str, Vec<&Household>)> = Vec::new();
    for household in households {
        match by_identifier.iter_mut().find(|(id, _)| *id == household.identifier) {
            Some((_, twins)) => twins.push(household),
            None => by_identifier.push((&household.identifier, vec![household])),
        }
    }
    for (identifier, twins) in &by_identifier {
        if twins.len() > 1 {
            add(Found {
                severity: Severity::Error,
                kind: "households",
                object: (*identifier).into(),
                object_id: twins.iter().map(|twin| twin.id.as_str()).collect::<Vec<_>>().join(", "),
                field: "identifier",
                problem: format!("{} households carry the same identifier.", twins.len()),
                cost: "The register refuses the second registration and every visit filed against it.",
                href: format!("people/{}", twins[1].id),
            });
        }
    }

    for household in households {
        if household.size > 20 {
            add(Found {
                severity: Severity::Warning,
                kind: "households",
                object: household.identifier.clone(),
                object_id: household.id.clone(),
                field: "size",
                problem: format!(
                    "{} people in one household, more than the survey expects.",
                    household.size
                ),
                cost: "Water per person is worked out low for every visit to it.",
                href: format!("people/{}", household.id),
            });
        }
    }

    findings
}

/// The strings the forms say, counted, and how many of them each language has.
pub fn coverage() -> Vec<Coverage> {
    let total: usize = forms()
        .iter()
        .map(|form| 2 + questions_of(form).count() * 2 + form.grid.as_ref().map_or(0, |g| g.rows.len()))
        .sum();
    let share = |part: f64| (total as f64 * part).round() as usize;
    vec![
        Coverage {
            language: "pt",
            name: "Portuguese",
            translated: share(0.08),
            total,
        },
        Coverage {
            language: "ar",
            name: "Arabic",
            translated: share(0.31),
            total,
        },
        Coverage {
            language: "fr",
            name: "French",
            translated: share(0.74),
            total,
        },
        Coverage {
            language: "en-GB",
            name: "British English",
            translated: total,
            total,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_the_web_finds() {
        let backend = crate::backend::Backend::seeded(crate::app::DEMO_START);
        let found = check_metadata(&backend.households);
        let of = |s: Severity| found.iter().filter(|f| f.severity == s).count();
        assert_eq!(
            (of(Severity::Error), of(Severity::Warning), of(Severity::Note)),
            (1, 2, 10)
        );
        assert_eq!(coverage()[0].total, 84);
        assert_eq!(coverage()[1].translated, 26);
    }
}
