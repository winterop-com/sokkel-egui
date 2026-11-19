//! The data elements the server holds, as the web's `pages/DataElements.tsx` lists them: searched,
//! each with the indicators and rules that read it and its value for the country in the latest
//! month, one opened in a sheet.

use serde::Serialize;

use crate::backend::{Indicator, ValidationRule};
use crate::js::trim;
use crate::metadata::{DataElement, data_element, data_elements, data_set, latest_period, period_label, root};
use crate::ui::{counted, figure};
use crate::values::data_value;

/// The indicators and validation rules whose expressions name a data element.
pub fn readers<'a>(
    element: &DataElement,
    indicators: &'a [Indicator],
    rules: &'a [ValidationRule],
) -> (Vec<&'a Indicator>, Vec<&'a ValidationRule>) {
    (
        indicators
            .iter()
            .filter(|one| format!("{} {}", one.numerator, one.denominator).contains(&element.id))
            .collect(),
        rules
            .iter()
            .filter(|one| format!("{} {}", one.left, one.right).contains(&element.id))
            .collect(),
    )
}

/// A data element as the table lists it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    pub name: String,
    pub code: String,
    /// Its category combo's name, none for the default one.
    pub disaggregation: Option<String>,
    pub value_type: &'static str,
    /// How many expressions read it, none when nothing does.
    pub read_by: Option<String>,
    pub value: String,
}

/// The data elements the search lets through, as the table lists them.
pub fn rows(query: &str, indicators: &[Indicator], rules: &[ValidationRule]) -> Vec<Row> {
    let asked = trim(query).to_lowercase();
    data_elements()
        .into_iter()
        .filter(|one| {
            asked.is_empty()
                || format!("{} {} {}", one.name, one.code, one.id)
                    .to_lowercase()
                    .contains(&asked)
        })
        .map(|one| {
            let (read_indicators, read_rules) = readers(one, indicators, rules);
            let total = read_indicators.len() + read_rules.len();
            Row {
                id: one.id.clone(),
                name: one.name.clone(),
                code: one.code.clone(),
                disaggregation: (one.category_combo.name != "default").then(|| one.category_combo.name.clone()),
                value_type: one.value_type,
                read_by: (total > 0).then(|| counted(total, "expression")),
                value: figure(data_value(&one.id, None, root(), latest_period()), ""),
            }
        })
        .collect()
}

/// What the status line says.
pub fn foot(shown: usize) -> String {
    format!("Showing {shown} of {}", counted(data_elements().len(), "data element"))
}

/// The header's description.
pub fn description() -> String {
    format!(
        "What the sites report each month in {}. An expression names one as #{{uid}}, or one of its category option combos as #{{uid.combo}}.",
        data_set().name
    )
}

/// The value column's header: the country in the latest month.
pub fn value_header() -> String {
    format!("Norway, {}", period_label(latest_period()))
}

/// What the table says when the search finds nothing.
pub const EMPTY: &str = "No data element matches the search.";

/// A term and its value in the sheet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Term {
    pub term: String,
    pub value: String,
}

/// A link to an expression that reads the data element.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Reader {
    pub href: String,
    pub name: String,
    pub kind: &'static str,
}

/// The sheet a data element opens in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sheet {
    pub title: String,
    pub code: String,
    pub description: String,
    pub facts: Vec<Term>,
    pub combos: Vec<Term>,
    pub read_title: String,
    pub readers: Vec<Reader>,
    /// What the sheet says when nothing reads the data element.
    pub none: Option<&'static str>,
}

/// The sheet of the data element `selected` names, none when it names none.
pub fn sheet(selected: Option<&str>, indicators: &[Indicator], rules: &[ValidationRule]) -> Option<Sheet> {
    let open = data_element(selected?)?;
    let (read_indicators, read_rules) = readers(open, indicators, rules);
    let term = |term: &str, value: &str| Term {
        term: term.to_owned(),
        value: value.to_owned(),
    };
    let total = read_indicators.len() + read_rules.len();
    let mut readers: Vec<Reader> = read_indicators
        .iter()
        .map(|one| Reader {
            href: format!("#/indicators/{}", one.id),
            name: one.name.clone(),
            kind: "indicator",
        })
        .collect();
    readers.extend(read_rules.iter().map(|one| Reader {
        href: format!("#/validation-rules/{}", one.id),
        name: one.name.clone(),
        kind: "validation rule",
    }));
    Some(Sheet {
        title: open.name.clone(),
        code: open.id.clone(),
        description: open.description.clone(),
        facts: vec![
            term("Code", &open.code),
            term("Short name", &open.short_name),
            term("Value type", open.value_type),
            term("Aggregation", open.aggregation_type),
            term("Category combo", &open.category_combo.name),
            term("Data set", &data_set().name),
        ],
        combos: open
            .category_combo
            .combos
            .iter()
            .map(|combo| term(&combo.name, &format!("#{{{}.{}}}", open.id, combo.id)))
            .collect(),
        read_title: format!("Read by ({total})"),
        readers,
        none: (total == 0).then_some("No indicator or validation rule reads this data element."),
    })
}
