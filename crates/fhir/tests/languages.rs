//! The two languages read as sokkel-ui-fhir's `languages.test.ts` reads them.

use sokkel_egui_fhir::program::{Syntax, read};
use sokkel_egui_fhir::{CQL, FHIRPATH, cql, fhir, fhirpath};

type Read = Vec<(String, &'static str)>;

/// Each line of the source as the parser reads it: every token's text and tag, whitespace dropped.
fn tokens(syntax: &Syntax, source: &str) -> Vec<Read> {
    let found = read(source, syntax);
    let mut offset = 0;
    source
        .split('\n')
        .map(|line| {
            let end = offset + line.len();
            let out = found
                .iter()
                .filter(|(from, _, _)| *from >= offset && *from <= end)
                .map(|(from, to, tag)| (source[*from..*to].trim().to_owned(), *tag))
                .filter(|(text, _)| !text.is_empty())
                .collect();
            offset = end + 1;
            out
        })
        .collect()
}

fn one_line(syntax: &Syntax, source: &str) -> Read {
    tokens(syntax, source).remove(0)
}

fn pairs(list: &[(&str, &'static str)]) -> Read {
    list.iter().map(|(text, tag)| ((*text).to_owned(), *tag)).collect()
}

const SPECIAL: &str = "variableName.special";
const NAME: &str = "variableName";
const FUNCTION: &str = "variableName.function";

#[test]
fn fhirpath_reads_a_path_an_operator_and_a_bound_variable() {
    assert_eq!(
        one_line(&fhirpath::syntax(), "valueQuantity.value > %limit"),
        pairs(&[
            ("valueQuantity", NAME),
            (".", "punctuation"),
            ("value", "propertyName"),
            (">", "operator"),
            ("%limit", SPECIAL),
        ])
    );
}

#[test]
fn fhirpath_reads_a_name_before_a_parenthesis_as_a_function_after_a_dot_or_not() {
    assert_eq!(
        one_line(
            &fhirpath::syntax(),
            "name.given.distinct().where(exists() and $this != 'x')"
        ),
        pairs(&[
            ("name", NAME),
            (".", "punctuation"),
            ("given", "propertyName"),
            (".", "punctuation"),
            ("distinct", FUNCTION),
            ("(", "bracket"),
            (")", "bracket"),
            (".", "punctuation"),
            ("where", FUNCTION),
            ("(", "bracket"),
            ("exists", FUNCTION),
            ("(", "bracket"),
            (")", "bracket"),
            ("and", "keyword"),
            ("$this", SPECIAL),
            ("!=", "operator"),
            ("'x'", "string"),
            (")", "bracket"),
        ])
    );
}

#[test]
fn fhirpath_keeps_a_keyword_a_keyword_between_operands_and_a_member_a_member_after_a_dot() {
    assert_eq!(
        one_line(&fhirpath::syntax(), "text.div contains name is Patient"),
        pairs(&[
            ("text", NAME),
            (".", "punctuation"),
            ("div", "propertyName"),
            ("contains", "keyword"),
            ("name", NAME),
            ("is", "keyword"),
            ("Patient", NAME),
        ])
    );
}

#[test]
fn fhirpath_keeps_a_keyword_before_a_parenthesis_a_keyword_and_a_member_called_as_one_a_function() {
    assert_eq!(
        one_line(&fhirpath::syntax(), "status in ('final') and value.as(Quantity)"),
        pairs(&[
            ("status", NAME),
            ("in", "keyword"),
            ("(", "bracket"),
            ("'final'", "string"),
            (")", "bracket"),
            ("and", "keyword"),
            ("value", NAME),
            (".", "punctuation"),
            ("as", FUNCTION),
            ("(", "bracket"),
            ("Quantity", NAME),
            (")", "bracket"),
        ])
    );
}

#[test]
fn fhirpath_reads_a_backticked_name_quoted_and_backticked_variables_a_literal_and_numbers() {
    assert_eq!(
        one_line(
            &fhirpath::syntax(),
            "`given name` = %`vs-codes` or %'us-zip' or true or 4.5 > 3"
        ),
        pairs(&[
            ("`given name`", NAME),
            ("=", "operator"),
            ("%`vs-codes`", SPECIAL),
            ("or", "keyword"),
            ("%'us-zip'", SPECIAL),
            ("or", "keyword"),
            ("true", "bool"),
            ("or", "keyword"),
            ("4.5", "number"),
            (">", "operator"),
            ("3", "number"),
        ])
    );
}

#[test]
fn fhirpath_reads_dates_date_times_times_and_partial_dates_as_literals() {
    for literal in [
        "@2026",
        "@2026-01",
        "@2026-01-01",
        "@2026-01-01T10:00:00Z",
        "@2026-01-01T10:00:00.250+01:00",
        "@T10:00",
        "@T10:00:00.5",
    ] {
        assert_eq!(
            one_line(&fhirpath::syntax(), &format!("birthDate < {literal}")),
            pairs(&[("birthDate", NAME), ("<", "operator"), (literal, "number")])
        );
    }
}

#[test]
fn fhirpath_keeps_an_escaped_quote_inside_a_string_and_marks_a_string_left_open() {
    assert_eq!(
        one_line(&fhirpath::syntax(), "family = 'O\\'Brien'"),
        pairs(&[("family", NAME), ("=", "operator"), ("'O\\'Brien'", "string")])
    );
    assert_eq!(
        one_line(&fhirpath::syntax(), "family = 'open"),
        pairs(&[("family", NAME), ("=", "operator"), ("'open", "invalid")])
    );
}

#[test]
fn fhirpath_reads_a_line_comment_and_a_block_comment_across_lines() {
    assert_eq!(
        tokens(&fhirpath::syntax(), "value // the reading\n/* one\ntwo */ code"),
        vec![
            pairs(&[("value", NAME), ("// the reading", "comment")]),
            pairs(&[("/* one", "comment")]),
            pairs(&[("two */", "comment"), ("code", NAME)]),
        ]
    );
}

#[test]
fn cql_reads_a_declaration_keywords_a_quoted_name_and_a_string() {
    assert_eq!(
        one_line(&cql::syntax(), "code \"Body weight\": '29463-7' from \"LOINC\""),
        pairs(&[
            ("code", "keyword"),
            ("\"Body weight\"", NAME),
            (":", "punctuation"),
            ("'29463-7'", "string"),
            ("from", "keyword"),
            ("\"LOINC\"", NAME),
        ])
    );
}

#[test]
fn cql_reads_less_more_and_external_as_keywords() {
    let read = one_line(&cql::syntax(), "A starts 3 days or less before B or more external");
    let found: Read = read
        .into_iter()
        .filter(|(text, _)| ["less", "more", "external"].contains(&text.as_str()))
        .collect();
    assert_eq!(
        found,
        pairs(&[("less", "keyword"), ("more", "keyword"), ("external", "keyword")])
    );
}

#[test]
fn cql_reads_a_function_a_keyword_before_a_parenthesis_a_member_and_a_number() {
    assert_eq!(
        one_line(
            &cql::syntax(),
            "define \"Adult\": exists (O where AgeInYears() >= 18.5 and O.code is not null)"
        ),
        pairs(&[
            ("define", "keyword"),
            ("\"Adult\"", NAME),
            (":", "punctuation"),
            ("exists", "keyword"),
            ("(", "bracket"),
            ("O", NAME),
            ("where", "keyword"),
            ("AgeInYears", FUNCTION),
            ("(", "bracket"),
            (")", "bracket"),
            (">=", "operator"),
            ("18.5", "number"),
            ("and", "keyword"),
            ("O", NAME),
            (".", "punctuation"),
            ("code", "propertyName"),
            ("is", "keyword"),
            ("not", "keyword"),
            ("null", "null"),
            (")", "bracket"),
        ])
    );
}

#[test]
fn cql_reads_a_retrieve_date_literals_and_a_backticked_name() {
    assert_eq!(
        one_line(
            &cql::syntax(),
            "[Observation: `Body weight`] during Interval[@2026-01-01, @2026-12-31T23:59:59]"
        ),
        pairs(&[
            ("[", "bracket"),
            ("Observation", NAME),
            (":", "punctuation"),
            ("`Body weight`", NAME),
            ("]", "bracket"),
            ("during", "keyword"),
            ("Interval", NAME),
            ("[", "bracket"),
            ("@2026-01-01", "number"),
            (",", "punctuation"),
            ("@2026-12-31T23:59:59", "number"),
            ("]", "bracket"),
        ])
    );
}

#[test]
fn cql_reads_comments_on_a_line_of_their_own_after_code_and_across_lines_and_a_long_integer() {
    assert_eq!(
        tokens(
            &cql::syntax(),
            "// weights\nparameter Limit Decimal default 100L /* kg\nor lb */ define"
        ),
        vec![
            pairs(&[("// weights", "comment")]),
            pairs(&[
                ("parameter", "keyword"),
                ("Limit", NAME),
                ("Decimal", NAME),
                ("default", "keyword"),
                ("100L", "number"),
                ("/* kg", "comment"),
            ]),
            pairs(&[("or lb */", "comment"), ("define", "keyword")]),
        ]
    );
}

#[test]
fn the_plugin_gives_the_editor_both_languages_by_name_and_by_media_type_once_installed() {
    use sokkel_egui::CodeLanguage;
    use sokkel_egui::plugins::{language_for_media_type, plugin_language, register_plugins};
    let ctx = egui::Context::default();
    assert_eq!(language_for_media_type(&ctx, "text/cql"), None);
    register_plugins(&ctx, [fhir()]);
    assert!(plugin_language(&ctx, FHIRPATH).is_some());
    assert!(plugin_language(&ctx, CQL).is_some());
    assert_eq!(
        language_for_media_type(&ctx, "application/fhirpath"),
        Some(CodeLanguage::Plugin(FHIRPATH))
    );
    assert_eq!(
        language_for_media_type(&ctx, "text/cql"),
        Some(CodeLanguage::Plugin(CQL))
    );
    assert_eq!(
        language_for_media_type(&ctx, "Text/CQL-Expression"),
        Some(CodeLanguage::Plugin(CQL))
    );
    // a plugin's language reads the document into the editor's tags
    let language = plugin_language(&ctx, FHIRPATH).expect("installed");
    let read = (language.highlight)("name.where(use = 'official')");
    assert_eq!(read[0].kind, sokkel_egui::CodeTag::Punctuation);
    assert_eq!(read[1].kind, sokkel_egui::CodeTag::Function);
}
