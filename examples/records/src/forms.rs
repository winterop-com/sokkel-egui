//! The forms the server serves, as the web's `data/forms.ts`: their sections and questions, a
//! periodic report's grid of counts, what is wrong with an answer, and answers made up from a seed.

use sokkel_egui::Day;
use sokkel_egui::periods::{PeriodType, recent_periods};

use crate::geo::{Level, bank_height, places};

/// `Math.round`: halves up, as the web rounds.
pub fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

/// How a form is filed: counts for a place over a period, one event, a person's registration, or
/// a visit to a registered person.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FormKind {
    Periodic,
    Event,
    Registration,
    Followup,
}

impl FormKind {
    pub fn label(self) -> &'static str {
        match self {
            FormKind::Periodic => "Periodic report",
            FormKind::Event => "Single event",
            FormKind::Registration => "Registration",
            FormKind::Followup => "Follow-up",
        }
    }

    /// The kind token its chip wears.
    pub fn color(self) -> &'static str {
        match self {
            FormKind::Periodic => "teal",
            FormKind::Event => "pink",
            FormKind::Registration => "violet",
            FormKind::Followup => "green",
        }
    }

    /// The group the forms of the kind are listed under.
    pub fn group(self) -> &'static str {
        match self {
            FormKind::Periodic => "Periodic reports",
            FormKind::Event => "Single events",
            FormKind::Registration | FormKind::Followup => "Programs",
        }
    }

    pub fn note(self) -> &'static str {
        match self {
            FormKind::Periodic => "Counts for a place over a period. No person involved.",
            FormKind::Event => "One visit or sample, recorded without registering anyone.",
            FormKind::Registration => {
                "A person is registered once, then each form under the registration records one of their visits."
            }
            FormKind::Followup => "",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestionType {
    Text,
    Multiline,
    Integer,
    Decimal,
    Date,
    Time,
    YesNo,
    Choice,
    Computed,
}

impl QuestionType {
    /// The type as the API names it.
    pub fn key(self) -> &'static str {
        match self {
            QuestionType::Text => "text",
            QuestionType::Multiline => "multiline",
            QuestionType::Integer => "integer",
            QuestionType::Decimal => "decimal",
            QuestionType::Date => "date",
            QuestionType::Time => "time",
            QuestionType::YesNo => "yesno",
            QuestionType::Choice => "choice",
            QuestionType::Computed => "computed",
        }
    }
}

/// What a question holds: text, a number, yes or no, or the texts of a question answered more
/// than once. Nothing yet is no entry at all.
#[derive(Clone, Debug, PartialEq)]
pub enum AnswerValue {
    Text(String),
    Number(f64),
    Bool(bool),
    List(Vec<String>),
}

impl AnswerValue {
    pub fn number(&self) -> Option<f64> {
        match self {
            AnswerValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            AnswerValue::Text(t) => Some(t),
            _ => None,
        }
    }

    /// As `String(value)` writes it: a number as JavaScript writes one.
    pub fn written(&self) -> String {
        match self {
            AnswerValue::Text(t) => t.clone(),
            AnswerValue::Number(n) => js_number(*n),
            AnswerValue::Bool(b) => b.to_string(),
            AnswerValue::List(l) => l.join(","),
        }
    }
}

/// A number as JavaScript's `String` writes it: whole numbers with no point.
pub fn js_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e21 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// A record's answers by question id, in the order they were given.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Answers(pub Vec<(String, AnswerValue)>);

impl Answers {
    pub fn get(&self, id: &str) -> Option<&AnswerValue> {
        self.0.iter().find(|(k, _)| k == id).map(|(_, v)| v)
    }

    pub fn set(&mut self, id: &str, value: Option<AnswerValue>) {
        let at = self.0.iter().position(|(k, _)| k == id);
        match (at, value) {
            (Some(i), Some(v)) => self.0[i].1 = v,
            (None, Some(v)) => self.0.push((id.to_owned(), v)),
            (Some(i), None) => {
                self.0.remove(i);
            }
            (None, None) => {}
        }
    }

    pub fn number(&self, id: &str) -> Option<f64> {
        self.get(id).and_then(AnswerValue::number)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Choice {
    pub value: &'static str,
    pub label: &'static str,
}

const fn choice(value: &'static str, label: &'static str) -> Choice {
    Choice { value, label }
}

/// What a form is filled for: where, when, and for whom.
#[derive(Clone, Debug, Default)]
pub struct FillContext {
    pub place: Option<String>,
    pub period: Option<String>,
    pub visit_date: Option<String>,
    pub household_size: Option<f64>,
}

/// A rule that works an answer out from the others, with why in words.
#[derive(Clone, Copy)]
pub struct Formula {
    pub why: &'static str,
    pub expression: &'static str,
    pub compute: fn(&Answers, &FillContext) -> Option<f64>,
}

impl std::fmt::Debug for Formula {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.expression)
    }
}

#[derive(Clone, Debug)]
pub struct Question {
    /// The answer's key, which the record and the API name it by.
    pub id: &'static str,
    pub label: &'static str,
    pub kind: QuestionType,
    pub required: bool,
    pub description: Option<&'static str>,
    pub help: Option<&'static str>,
    pub unit: Option<&'static str>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub options: &'static [Choice],
    pub formula: Option<Formula>,
    /// A yes or no that keeps no No.
    pub yes_only: bool,
    /// A text question answered more than once, its answer a list.
    pub repeats: bool,
}

const fn question(id: &'static str, label: &'static str, kind: QuestionType) -> Question {
    Question {
        id,
        label,
        kind,
        required: false,
        description: None,
        help: None,
        unit: None,
        min: None,
        max: None,
        options: &[],
        formula: None,
        yes_only: false,
        repeats: false,
    }
}

impl Question {
    const fn required(mut self) -> Self {
        self.required = true;
        self
    }
    const fn description(mut self, text: &'static str) -> Self {
        self.description = Some(text);
        self
    }
    const fn help(mut self, text: &'static str) -> Self {
        self.help = Some(text);
        self
    }
    const fn unit(mut self, unit: &'static str) -> Self {
        self.unit = Some(unit);
        self
    }
    const fn range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }
    const fn options(mut self, options: &'static [Choice]) -> Self {
        self.options = options;
        self
    }
    const fn formula(mut self, formula: Formula) -> Self {
        self.formula = Some(formula);
        self
    }
    const fn yes_only(mut self) -> Self {
        self.yes_only = true;
        self
    }
    const fn repeats(mut self) -> Self {
        self.repeats = true;
        self
    }
}

#[derive(Clone, Debug)]
pub struct FormSection {
    pub id: &'static str,
    pub title: &'static str,
    pub description: Option<&'static str>,
    pub questions: Vec<Question>,
}

/// A grid of counts: a row per question under its part, a column per cell of its cut.
#[derive(Clone, Debug)]
pub struct CountGrid {
    pub by: &'static str,
    pub categories: &'static [Choice],
    /// A second category the first is crossed with, so each cell is one option of each.
    pub within: Option<(&'static str, &'static [Choice])>,
    pub rows: &'static [GridRow],
}

#[derive(Clone, Copy, Debug)]
pub struct GridRow {
    pub id: &'static str,
    pub part: &'static str,
    pub label: &'static str,
}

/// One cell of a grid's cut: its key, what it is called, and the band it sits in, if any.
#[derive(Clone, Debug, PartialEq)]
pub struct GridCell {
    pub value: String,
    pub label: String,
    pub band: Option<String>,
}

/// A cut of this many cells or more is drawn in bands, by the option of its smaller category.
const BAND_FROM: usize = 5;

/// A grid's cells: each option of its category, crossed with each of `within`'s when it has one.
/// A cut of five cells or more is banded by the smaller category, its options as the bands and the
/// larger one's under each; a smaller cut names each cell by both options.
pub fn grid_cells(grid: &CountGrid) -> Vec<GridCell> {
    let Some((_, within)) = grid.within else {
        return grid
            .categories
            .iter()
            .map(|c| GridCell {
                value: c.value.into(),
                label: c.label.into(),
                band: None,
            })
            .collect();
    };
    let outer_smaller = grid.categories.len() <= within.len();
    let (bands, across) = if outer_smaller {
        (grid.categories, within)
    } else {
        (within, grid.categories)
    };
    let banded = grid.categories.len() * within.len() >= BAND_FROM;
    bands
        .iter()
        .flat_map(|band| {
            across.iter().map(move |option| GridCell {
                value: if outer_smaller {
                    format!("{}-{}", band.value, option.value)
                } else {
                    format!("{}-{}", option.value, band.value)
                },
                label: if banded {
                    option.label.into()
                } else {
                    format!("{}, {}", band.label, option.label)
                },
                band: banded.then(|| band.label.to_owned()),
            })
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct Form {
    pub id: &'static str,
    pub title: &'static str,
    pub kind: FormKind,
    pub description: &'static str,
    /// The period a periodic report is made for.
    pub period: Option<PeriodType>,
    /// The levels of place the form is assigned to; every level when empty.
    pub levels: &'static [Level],
    pub sections: Vec<FormSection>,
    pub grid: Option<CountGrid>,
    /// The program a registration or a follow-up belongs to.
    pub program: Option<&'static str>,
    /// A follow-up taken more than once per person.
    pub repeats: bool,
}

pub const PROGRAM_WATER: (&str, &str, &str) = (
    "PRG-WATER",
    "Household water survey",
    "Households registered once, then visited at home through the year.",
);

const SOURCES: &[Choice] = &[
    choice("tap", "Tap at home"),
    choice("standpipe", "Public standpipe"),
    choice("well", "Protected well"),
    choice("river", "River or stream"),
    choice("rain", "Rainwater"),
];

const SITE_KINDS: &[Choice] = &[
    choice("river", "River"),
    choice("lake", "Lake"),
    choice("well", "Well"),
    choice("coast", "Coast"),
];

const ACCESS: &[Choice] = &[
    choice("open", "Yes, on the usual path"),
    choice("partly", "Only part of it"),
    choice("closed", "No, the way was closed"),
];
const WEATHER: &[Choice] = &[
    choice("dry", "Dry"),
    choice("rain", "Light rain"),
    choice("storm", "Heavy rain or wind"),
];
const LABS: &[Choice] = &[choice("north", "North lab"), choice("south", "South lab")];
const CHLORINE: &[Choice] = &[
    choice("none", "None found"),
    choice("low", "Low"),
    choice("good", "Good"),
    choice("high", "Too high"),
];
const REASONS: &[Choice] = &[
    choice("moved", "Moved away"),
    choice("declined", "Declined further visits"),
    choice("ended", "The survey ended"),
];

const TIMES: &[Choice] = &[choice("am", "Morning"), choice("pm", "Evening")];

const MONTH_ROWS: &[GridRow] = &[
    GridRow {
        id: "readings",
        part: "Readings",
        label: "Gauge readings taken",
    },
    GridRow {
        id: "missed",
        part: "Readings",
        label: "Readings missed",
    },
    GridRow {
        id: "flagged",
        part: "Readings",
        label: "Readings flagged for review",
    },
    GridRow {
        id: "late",
        part: "Readings",
        label: "Readings sent in late",
    },
    GridRow {
        id: "samples",
        part: "Samples",
        label: "Water samples sent to the lab",
    },
    GridRow {
        id: "refused",
        part: "Samples",
        label: "Samples the lab refused",
    },
    GridRow {
        id: "kits",
        part: "Samples",
        label: "Field kits used",
    },
    GridRow {
        id: "visits",
        part: "Upkeep",
        label: "Visits by observers",
    },
    GridRow {
        id: "repairs",
        part: "Upkeep",
        label: "Repairs made",
    },
    GridRow {
        id: "damage",
        part: "Upkeep",
        label: "Damage reported",
    },
];

fn clearance(answers: &Answers, context: &FillContext) -> Option<f64> {
    let level = answers.number("level")?;
    let place = context.place.as_deref()?;
    Some(js_round((bank_height(place) - level) * 100.0) / 100.0)
}

fn per_person(answers: &Answers, context: &FillContext) -> Option<f64> {
    let stored = answers.number("stored")?;
    let size = context.household_size.filter(|s| *s > 0.0)?;
    Some(js_round(stored / size * 10.0) / 10.0)
}

fn section(
    id: &'static str,
    title: &'static str,
    description: Option<&'static str>,
    questions: Vec<Question>,
) -> FormSection {
    FormSection {
        id,
        title,
        description,
        questions,
    }
}

fn form(id: &'static str, title: &'static str, kind: FormKind, description: &'static str) -> Form {
    Form {
        id,
        title,
        kind,
        description,
        period: None,
        levels: &[],
        sections: Vec::new(),
        grid: None,
        program: None,
        repeats: false,
    }
}

fn build_forms() -> Vec<Form> {
    use QuestionType::*;
    vec![
        Form {
            period: Some(PeriodType::Monthly),
            grid: Some(CountGrid {
                by: "kind of site",
                categories: SITE_KINDS,
                within: Some(("time of day", TIMES)),
                rows: MONTH_ROWS,
            }),
            ..form(
                "FRM-MONTH",
                "Monthly station summary",
                FormKind::Periodic,
                "A month's counts at one station, by the kind of site each was taken at.",
            )
        },
        Form {
            period: Some(PeriodType::Quarterly),
            levels: &[Level::Region],
            sections: vec![
                section(
                    "gauges",
                    "Gauges",
                    None,
                    vec![
                        question("gauges_store", "Gauge boards in store", Integer)
                            .required()
                            .range(0.0, 500.0),
                        question("gauges_broken", "Gauge boards broken", Integer)
                            .range(0.0, 500.0)
                            .help("Count each board once."),
                    ],
                ),
                section(
                    "kits",
                    "Sampling kits",
                    Some("Counted on the last working day of the quarter."),
                    vec![
                        question("bottles", "Sample bottles", Integer).required().range(0.0, 5000.0),
                        question("strips", "Test strips", Integer).range(0.0, 20000.0).unit("strips"),
                        question("reorder", "Reorder needed", YesNo)
                            .description("Whether the store runs out before the next delivery."),
                    ],
                ),
            ],
            ..form(
                "FRM-STOCK",
                "Gauge and kit stock",
                FormKind::Periodic,
                "What a region holds in store at the end of a quarter.",
            )
        },
        Form {
            sections: vec![
                section(
                    "reading",
                    "Measurements",
                    Some("Read at the gauge, in the units beside each label."),
                    vec![
                        question("level", "Water level", Decimal)
                            .unit("m")
                            .required()
                            .range(0.0, 10.0)
                            .description("Read off the gauge board before the sample is taken."),
                        question("clearance", "Clearance below the bank", Computed)
                            .unit("m")
                            .formula(Formula {
                                why: "The site's bank height less the water level. Below 0.5 m the site is flagged for flooding.",
                                expression: "bank_height - level",
                                compute: clearance,
                            }),
                        question("temperature", "Water temperature", Decimal)
                            .unit("°C")
                            .range(-5.0, 45.0),
                        question("turbidity", "Turbidity", Integer)
                            .unit("NTU")
                            .range(0.0, 1000.0)
                            .help("Read after the sample has settled."),
                        question("ph", "pH", Decimal).range(0.0, 14.0).unit("0 to 14"),
                    ],
                ),
                section(
                    "check",
                    "Site check",
                    Some("The conditions at the site during the visit."),
                    vec![
                        question("access", "Could you reach the site?", Choice).required().options(ACCESS),
                        question("weather", "Weather during the reading", Choice).options(WEATHER),
                        question("cleaned", "Gauge board cleaned", YesNo).description("Before the reading was taken."),
                        question("damaged", "Gauge damaged", YesNo)
                            .yes_only()
                            .help("Yes or not answered: this question keeps no No."),
                    ],
                ),
                section(
                    "notes",
                    "Notes",
                    None,
                    vec![
                        question("notes", "What else did you see?", Multiline)
                            .help("Anything the next visitor should know."),
                    ],
                ),
            ],
            ..form(
                "FRM-SITE",
                "Site visit",
                FormKind::Event,
                "One visit to a site: the readings at the gauge, the conditions, and what the observer saw.",
            )
        },
        Form {
            sections: vec![section(
                "sample",
                "Sample",
                None,
                vec![
                    question("bottle", "Bottle numbers", Text)
                        .required()
                        .repeats()
                        .description("One for each bottle the sample fills, as written on it, such as B-2041."),
                    question("taken", "Taken at", Time).required(),
                    question("preserved", "Kept cool on the way", YesNo),
                    question("lab", "Lab", Choice)
                        .options(LABS),
                ],
            )],
            ..form(
                "FRM-SAMPLE",
                "Water sample",
                FormKind::Event,
                "A sample taken for the lab, with what it was taken in and how it travels.",
            )
        },
        Form {
            program: Some(PROGRAM_WATER.0),
            sections: vec![section(
                "household",
                "Household",
                None,
                vec![
                    question("born", "Year the head of household was born", Integer)
                        .required()
                        .range(1920.0, 2010.0),
                    question("size", "People in the household", Integer).required().range(1.0, 30.0),
                    question("source", "Main water source", Choice).required().options(SOURCES),
                    question("treats", "Treats water at home", YesNo).description("Boils, filters or adds chlorine."),
                ],
            )],
            ..form(
                "FRM-ENROLL",
                "Household registration",
                FormKind::Registration,
                "Registers a household in the water survey. The household is given an identifier when the record is filed.",
            )
        },
        Form {
            program: Some(PROGRAM_WATER.0),
            repeats: true,
            sections: vec![
                section(
                    "stored",
                    "Water at home",
                    None,
                    vec![
                        question("stored", "Water stored at home", Decimal)
                            .unit("liters")
                            .required()
                            .range(0.0, 2000.0),
                        question("per_person", "Water per person", Computed)
                            .unit("liters")
                            .formula(Formula {
                                why: "The water stored at home shared over the household's people, as registered. Under 15 liters a person is a shortfall.",
                                expression: "stored / household_size",
                                compute: per_person,
                            }),
                        question("covered", "Containers covered", YesNo),
                        question("chlorine", "Chlorine test", Choice).options(CHLORINE),
                    ],
                ),
                section(
                    "health",
                    "Health",
                    None,
                    vec![
                        question("ill", "Anyone ill with diarrhea in the last two weeks", YesNo)
                            .description("Ask the head of household."),
                    ],
                ),
            ],
            ..form(
                "FRM-HOME",
                "Home visit",
                FormKind::Followup,
                "One visit to a registered household. Each visit is its own record.",
            )
        },
        Form {
            program: Some(PROGRAM_WATER.0),
            sections: vec![section(
                "exit",
                "Leaving",
                None,
                vec![
                    question("reason", "Why the household leaves", Choice).required().options(REASONS),
                    question("better", "Water at home better than at the start", YesNo),
                ],
            )],
            ..form(
                "FRM-EXIT",
                "Exit interview",
                FormKind::Followup,
                "The last visit, as a household leaves the survey. One record per household.",
            )
        },
    ]
}

/// Every form the server serves, in its order.
pub fn forms() -> &'static [Form] {
    static FORMS: std::sync::OnceLock<Vec<Form>> = std::sync::OnceLock::new();
    FORMS.get_or_init(build_forms)
}

pub fn form_of(id: &str) -> Option<&'static Form> {
    forms().iter().find(|f| f.id == id)
}

/// Every question a form asks, in order.
pub fn questions_of(form: &Form) -> impl Iterator<Item = &Question> {
    form.sections.iter().flat_map(|s| s.questions.iter())
}

/// How many questions a form asks: its fields, or its grid's cells.
pub fn question_count(form: &Form) -> usize {
    match &form.grid {
        None => questions_of(form).count(),
        Some(grid) => grid.rows.len() * grid_cells(grid).len(),
    }
}

/// A grid's key for one answer: the row and the cell of the cut.
pub fn cell_key(row: &str, category: &str) -> String {
    format!("{row}.{category}")
}

/// Whether a question holds an answer: not empty text, and for a repeated one at least one text.
pub fn is_answered(value: Option<&AnswerValue>) -> bool {
    match value {
        None => false,
        Some(AnswerValue::Text(t)) => !t.is_empty(),
        Some(AnswerValue::List(l)) => l.iter().any(|one| !one.trim().is_empty()),
        Some(_) => true,
    }
}

/// What is wrong with an answer, or `None`. Required questions left empty are said elsewhere.
pub fn answer_issue(question: &Question, value: Option<&AnswerValue>) -> Option<String> {
    if !is_answered(value) {
        return None;
    }
    let value = value?;
    if question.repeats {
        return (!matches!(value, AnswerValue::List(_))).then(|| "Give the answers as a list of texts.".into());
    }
    if matches!(value, AnswerValue::List(_)) {
        return Some("This question takes one answer, not a list.".into());
    }
    if matches!(question.kind, QuestionType::Integer | QuestionType::Decimal) {
        let Some(n) = value.number().filter(|n| n.is_finite()) else {
            return Some(format!("{} is not a number.", value.written()));
        };
        if question.kind == QuestionType::Integer && n.fract() != 0.0 {
            return Some(format!(
                "{} is not a whole number, which this question counts in.",
                js_number(n)
            ));
        }
        if let Some(min) = question.min.filter(|min| n < *min) {
            return Some(format!(
                "{} is below {}, the lowest this form takes.",
                js_number(n),
                js_number(min)
            ));
        }
        if let Some(max) = question.max.filter(|max| n > *max) {
            return Some(format!(
                "{} is above {}, the highest this form takes.",
                js_number(n),
                js_number(max)
            ));
        }
    }
    if question.kind == QuestionType::Time
        && let AnswerValue::Text(t) = value
        && !is_time(t)
    {
        return Some(format!(
            "{t} is not a time of day. Write it as 24-hour hours and minutes, such as 09:40."
        ));
    }
    None
}

fn is_time(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 5
        && b[2] == b':'
        && b.iter().enumerate().all(|(i, c)| i == 2 || c.is_ascii_digit())
        && text[..2].parse::<u32>().is_ok_and(|h| h < 24)
        && text[3..].parse::<u32>().is_ok_and(|m| m < 60)
}

/// The range a number question takes, said as a hint.
pub fn range_hint(question: &Question) -> Option<String> {
    match (question.min, question.max) {
        (Some(min), Some(max)) => Some(format!("Between {} and {}.", js_number(min), js_number(max))),
        (Some(min), None) => Some(format!("{} or more.", js_number(min))),
        (None, Some(max)) => Some(format!("{} or less.", js_number(max))),
        (None, None) => None,
    }
}

/// A seeded run of numbers from 0 to 1, the same for the same seed.
pub struct Seeded(u32);

impl Seeded {
    pub fn new(seed: u32) -> Self {
        Seeded(if seed == 0 { 1 } else { seed })
    }

    pub fn draw(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from(self.0) / 4_294_967_296.0
    }
}

/// Answers made up for a form from a seed, each within what the form takes.
pub fn test_answers(form: &Form, seed: u32) -> Answers {
    let mut next = Seeded::new(seed);
    let mut answers = Answers::default();
    if let Some(grid) = &form.grid {
        let cells = grid_cells(grid);
        for row in grid.rows {
            for cell in &cells {
                let high = if row.id == "readings" { 140.0 } else { 24.0 };
                answers.set(
                    &cell_key(row.id, &cell.value),
                    Some(AnswerValue::Number((next.draw() * high).floor())),
                );
            }
        }
        return answers;
    }
    for question in questions_of(form) {
        let low = question.min.unwrap_or(0.0);
        let high = question.max.unwrap_or(100.0);
        let value = match question.kind {
            QuestionType::Computed => continue,
            QuestionType::Integer => {
                let reach = (high - low).min(if question.id == "born" { 70.0 } else { 40.0 });
                Some(AnswerValue::Number(js_round(low + next.draw() * reach)))
            }
            QuestionType::Decimal => {
                let reach = (high - low).min(match question.id {
                    "ph" => 4.0,
                    "level" => 3.5,
                    _ => 60.0,
                });
                let shift = if question.id == "ph" { 5.0 } else { 0.0 };
                Some(AnswerValue::Number(
                    js_round((low + next.draw() * reach + shift) * 100.0) / 100.0,
                ))
            }
            QuestionType::YesNo => {
                if question.yes_only {
                    (next.draw() < 0.3).then_some(AnswerValue::Bool(true))
                } else {
                    Some(AnswerValue::Bool(next.draw() < 0.6))
                }
            }
            QuestionType::Choice => {
                let at = (next.draw() * question.options.len() as f64).floor() as usize;
                question.options.get(at).map(|o| AnswerValue::Text(o.value.into()))
            }
            QuestionType::Time => {
                let hour = 7 + (next.draw() * 10.0).floor() as u32;
                let minute = (next.draw() * 60.0).floor() as u32;
                Some(AnswerValue::Text(format!("{hour:02}:{minute:02}")))
            }
            QuestionType::Text if question.repeats => {
                let count = 1 + (next.draw() * 3.0).floor() as usize;
                Some(AnswerValue::List(
                    (0..count)
                        .map(|_| format!("B-{}", 2000 + (next.draw() * 900.0).floor() as u32))
                        .collect(),
                ))
            }
            QuestionType::Text => Some(AnswerValue::Text(format!(
                "B-{}",
                2000 + (next.draw() * 900.0).floor() as u32
            ))),
            QuestionType::Multiline => Some(AnswerValue::Text(if next.draw() < 0.5 {
                "The path to the gauge was clear. Nothing else to report.".into()
            } else {
                String::new()
            })),
            QuestionType::Date => None,
        };
        // the web keeps `null` as a key; an answer of nothing is no entry here
        answers.set(question.id, value);
    }
    answers
}

/// The sites a test fill may report from, the same for the same seed.
pub fn test_place(seed: u32) -> String {
    let sites: Vec<_> = places()
        .iter()
        .filter(|p| p.level == Level::Site && p.position.is_some())
        .collect();
    let at = (Seeded::new(seed.wrapping_add(7)).draw() * sites.len() as f64).floor() as usize;
    sites.get(at).unwrap_or(&sites[0]).id.clone()
}

/// The last whole period of a type before today, which a report is usually made for.
pub fn last_whole_period(kind: PeriodType, today: Day) -> String {
    recent_periods(kind, today, 2)
        .get(1)
        .map_or_else(|| recent_periods(kind, today, 1)[0].id.clone(), |p| p.id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_monthly_grid_has_eight_cells_in_two_bands() {
        let form = form_of("FRM-MONTH").unwrap();
        let cells = grid_cells(form.grid.as_ref().unwrap());
        assert_eq!(cells.len(), 8);
        assert_eq!(cells[0].value, "river-am");
        assert_eq!(cells[0].band.as_deref(), Some("Morning"));
        assert_eq!(question_count(form), 80);
        assert_eq!(question_count(form_of("FRM-SITE").unwrap()), 10);
    }
}
