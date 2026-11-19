//! The two languages read as sokkel-ui-dhis2's `languages.test.ts` reads them.

use sokkel_egui_dhis2::names::Dhis2Names;
use sokkel_egui_dhis2::tokens::{Known, read};
use sokkel_egui_dhis2::{DHIS2_EXPRESSION, DHIS2_RULE, dhis2, dhis2_plugin};

fn expression() -> Known {
    let names = Dhis2Names::default();
    Known::new(
        names.functions.iter().filter(|one| !one.starts_with("d2:")),
        std::iter::empty::<&str>(),
    )
}

fn rule() -> Known {
    let names = Dhis2Names::default();
    Known::new(&names.functions, &names.variables)
}

/// Each token of the source as its text and tag.
fn one_line(known: &Known, source: &str) -> Vec<(String, &'static str)> {
    read(source, known)
        .into_iter()
        .map(|(from, to, tag)| (source[from..to].trim().to_owned(), tag))
        .filter(|(text, _)| !text.is_empty())
        .collect()
}

fn pairs(list: &[(&str, &'static str)]) -> Vec<(String, &'static str)> {
    list.iter().map(|(text, tag)| ((*text).to_owned(), *tag)).collect()
}

const SPECIAL: &str = "variableName.special";

#[test]
fn reads_data_elements_with_and_without_disaggregations_and_the_wildcard() {
    assert_eq!(
        one_line(&expression(), "#{FH8ab5Rog83} + #{FH8ab5Rog83.*.lTHuSO6awqE}"),
        pairs(&[
            ("#{", SPECIAL),
            ("FH8ab5Rog83", SPECIAL),
            ("}", SPECIAL),
            ("+", "operator"),
            ("#{", SPECIAL),
            ("FH8ab5Rog83", SPECIAL),
            (".", "punctuation"),
            ("*", "atom"),
            (".", "punctuation"),
            ("lTHuSO6awqE", SPECIAL),
            ("}", SPECIAL),
        ])
    );
}

#[test]
fn reads_every_kind_of_item_a_metric_and_the_period_counts() {
    assert_eq!(
        one_line(
            &expression(),
            "R{BfMAe6Itzgt.REPORTING_RATE} * C{bCqvfPR02Im} / OUG{oRVt7g429ZO} + [days]"
        ),
        pairs(&[
            ("R{", SPECIAL),
            ("BfMAe6Itzgt", SPECIAL),
            (".", "punctuation"),
            ("REPORTING_RATE", "atom"),
            ("}", SPECIAL),
            ("*", "operator"),
            ("C{", SPECIAL),
            ("bCqvfPR02Im", SPECIAL),
            ("}", SPECIAL),
            ("/", "operator"),
            ("OUG{", SPECIAL),
            ("oRVt7g429ZO", SPECIAL),
            ("}", SPECIAL),
            ("+", "operator"),
            ("[days]", SPECIAL),
        ])
    );
    let tags: Vec<&str> = one_line(&expression(), "#{deGroup:oDkJh5Ddh7d.coGroup:OK2Nr4wdfrZ&cSLqvJ5wR2x}")
        .into_iter()
        .map(|(_, tag)| tag)
        .collect();
    assert_eq!(
        tags,
        [
            SPECIAL,
            "keyword",
            SPECIAL,
            "punctuation",
            "keyword",
            SPECIAL,
            "punctuation",
            SPECIAL,
            SPECIAL
        ]
    );
}

#[test]
fn reads_functions_the_dotted_ones_and_those_after_a_value() {
    assert_eq!(
        one_line(
            &expression(),
            "if(isNull(#{a}), 0, #{a}.periodOffset(-1)) && orgUnit.group(CXw2yu5fodb)"
        ),
        pairs(&[
            ("if", "variableName.function"),
            ("(", "bracket"),
            ("isNull", "variableName.function"),
            ("(", "bracket"),
            ("#{", SPECIAL),
            ("a", SPECIAL),
            ("}", SPECIAL),
            (")", "bracket"),
            (",", "punctuation"),
            ("0", "number"),
            (",", "punctuation"),
            ("#{", SPECIAL),
            ("a", SPECIAL),
            ("}", SPECIAL),
            (".", "punctuation"),
            ("periodOffset", "variableName.function"),
            ("(", "bracket"),
            ("-", "operator"),
            ("1", "number"),
            (")", "bracket"),
            (")", "bracket"),
            ("&&", "operator"),
            ("orgUnit.group", "variableName.function"),
            ("(", "bracket"),
            ("CXw2yu5fodb", "variableName"),
            (")", "bracket"),
        ])
    );
}

#[test]
fn reads_dates_distinct_and_in() {
    let read = one_line(
        &expression(),
        "#{a}.aggregationType(LAST).minDate(2021-1-1) + count(distinct #{b})",
    );
    assert!(read.contains(&("LAST".to_owned(), "atom")));
    assert!(read.contains(&("2021-1-1".to_owned(), "number")));
    assert!(read.contains(&("minDate".to_owned(), "variableName.function")));
    assert!(read.contains(&("distinct".to_owned(), "keyword")));
}

#[test]
fn reads_literals_keywords_operators_and_numbers() {
    assert_eq!(
        one_line(&expression(), r#"not true or null == "a\"b" and 'c' != 1.5e3 ^ .5 % 2"#),
        pairs(&[
            ("not", "keyword"),
            ("true", "bool"),
            ("or", "keyword"),
            ("null", "null"),
            ("==", "operator"),
            (r#""a\"b""#, "string"),
            ("and", "keyword"),
            ("'c'", "string"),
            ("!=", "operator"),
            ("1.5e3", "number"),
            ("^", "operator"),
            (".5", "number"),
            ("%", "operator"),
            ("2", "number"),
        ])
    );
    assert_eq!(
        one_line(&expression(), "#{a} == 'open").last().cloned(),
        Some(("'open".to_owned(), "invalid"))
    );
}

#[test]
fn reads_a_block_comment_across_lines_and_ends_an_open_item_with_its_line() {
    assert_eq!(
        one_line(&expression(), "#{a} /* the first\nand the second */ + 1"),
        pairs(&[
            ("#{", SPECIAL),
            ("a", SPECIAL),
            ("}", SPECIAL),
            ("/* the first", "comment"),
            ("and the second */", "comment"),
            ("+", "operator"),
            ("1", "number"),
        ])
    );
    assert_eq!(
        one_line(&expression(), "#{a\n1"),
        pairs(&[("#{", SPECIAL), ("a", SPECIAL), ("1", "number")])
    );
}

#[test]
fn leaves_the_program_functions_to_the_program_language() {
    assert_eq!(
        one_line(&expression(), "d2:zing(1)")[0],
        ("d2:zing".to_owned(), "variableName")
    );
    assert_eq!(
        one_line(&rule(), "d2:zing(1)")[0],
        ("d2:zing".to_owned(), "variableName.function")
    );
}

#[test]
fn reads_program_rule_variables_program_variables_and_functions() {
    assert_eq!(
        one_line(
            &rule(),
            "d2:hasValue('lmp') && d2:daysBetween(#{lmp},V{event_date}) <= 0"
        ),
        pairs(&[
            ("d2:hasValue", "variableName.function"),
            ("(", "bracket"),
            ("'lmp'", "string"),
            (")", "bracket"),
            ("&&", "operator"),
            ("d2:daysBetween", "variableName.function"),
            ("(", "bracket"),
            ("#{", SPECIAL),
            ("lmp", SPECIAL),
            ("}", SPECIAL),
            (",", "punctuation"),
            ("V{", SPECIAL),
            ("event_date", SPECIAL),
            ("}", SPECIAL),
            (")", "bracket"),
            ("<=", "operator"),
            ("0", "number"),
        ])
    );
    let read = one_line(&rule(), "!#{blood pressure-high} || PS_EVENTDATE:A03MvHHogjR > 7");
    assert_eq!(read[0], ("!".to_owned(), "operator"));
    assert_eq!(read[2], ("blood".to_owned(), SPECIAL));
    assert_eq!(read[3], ("pressure-high".to_owned(), SPECIAL));
    assert!(read.contains(&("PS_EVENTDATE:".to_owned(), "keyword")));
}

#[test]
fn a_name_an_older_server_lacks_is_a_plain_name() {
    let names = Dhis2Names::new(["if"], ["event_date"]);
    let known = Known::new(&names.functions, &names.variables);
    assert_eq!(one_line(&known, "removeZeros(1)")[0].1, "variableName");
    assert_eq!(one_line(&known, "V{org_unit_count}")[1].1, "variableName");
}

#[test]
fn the_plugin_gives_both_languages() {
    let plugin = dhis2();
    let names: Vec<&str> = plugin.languages.iter().map(|one| one.name).collect();
    assert_eq!(names, [DHIS2_EXPRESSION, DHIS2_RULE]);
    let older = dhis2_plugin(&Dhis2Names::new(["if"], Vec::<String>::new()));
    let read = (older.languages[0].highlight)("if(1)");
    assert_eq!(read[0].kind, sokkel_egui::CodeTag::Function);
}
