//! The code editor: drawn in light and dark (a YAML pipeline checked against its schema with a
//! mark and folding, a markdown note, a jq line), and its keys: Tab indents, Enter keeps the
//! indent, Mod-Enter runs, the schema's problems are reported, and a word offered goes in.

use egui::{Key, Modifiers};
use egui_kittest::{Harness, kittest::Queryable};
use serde_json::{Value, json};
use sokkel_egui::{
    ActiveLine, Appearance, CodeCompletion, CodeEditor, CodeLanguage, CodeMark, CodeSize, CodeVariant, MarkTone,
    Scheme, install,
};

fn schema() -> Value {
    json!({
        "type": "object",
        "required": ["name", "steps"],
        "additionalProperties": false,
        "properties": {
            "name": {"type": "string", "pattern": "^[a-z][a-z0-9-]*$", "description": "The pipeline's name."},
            "schedule": {"type": "string"},
            "steps": {"type": "array", "minItems": 1, "items": {"$ref": "#/$defs/step"}}
        },
        "$defs": {
            "step": {
                "type": "object",
                "required": ["id", "kind"],
                "additionalProperties": false,
                "properties": {
                    "id": {"type": "string"},
                    "kind": {"enum": ["fetch", "transform", "upload"]},
                    "retries": {"type": "integer", "minimum": 0, "maximum": 5}
                }
            }
        }
    })
}

struct Pages {
    pipeline: String,
    note: String,
    query: String,
    schema: Value,
    marks: Vec<CodeMark>,
}

fn pages() -> Pages {
    Pages {
        pipeline: "# nightly-export\nname: nightly-export\nschedule: \"0 2 * * *\"\nsteps:\n  - id: fetch\n    kind: fetch\n    retries: 9\n  - id: upload\n    kind: upload\n    with: { bucket: harbor }\n".into(),
        note: "# Nightly export\n\nRuns at **02:00** for [[Harbor]] and #juniper.\n- [x] check `sync-inventory`\n".into(),
        query: ".runs[] | select(.state == \"failed\") | .id".into(),
        schema: schema(),
        marks: vec![CodeMark {
            line: Some(5),
            range: None,
            path: None,
            tone: MarkTone::Warning,
            message: Some("fetch ran slow last night".into()),
        }],
    }
}

fn draw(scheme: Scheme) -> Harness<'static, Pages> {
    let notes = vec!["Harbor".to_owned(), "Juniper".to_owned()];
    let mut harness = Harness::builder().with_size([640.0, 520.0]).build_ui_state(
        move |ui, pages: &mut Pages| {
            install(
                ui.ctx(),
                &Appearance {
                    scheme,
                    ..Default::default()
                },
            );
            ui.spacing_mut().item_spacing.y = 12.0;
            CodeEditor::new("pipeline", &mut pages.pipeline)
                .label("nightly-export.yaml")
                .language(CodeLanguage::Yaml)
                .schema(&pages.schema)
                .marks(&pages.marks)
                .folding(true)
                .indent_guides(true)
                .active_line(ActiveLine::Always)
                .height(236.0)
                .show(ui);
            CodeEditor::new("note", &mut pages.note)
                .label("Nightly export as markdown")
                .language(CodeLanguage::Markdown)
                .links(&notes)
                .size(CodeSize::Sm)
                .show(ui);
            CodeEditor::new("query", &mut pages.query)
                .label("Query")
                .language(CodeLanguage::Jq)
                .variant(CodeVariant::Line)
                .bracket_colors(true)
                .show(ui);
        },
        pages(),
    );
    harness.run_steps(4);
    harness
}

#[test]
fn the_editor_in_light() {
    let mut harness = draw(Scheme::Light);
    harness.snapshot("code_editor_light");
}

#[test]
fn the_editor_in_dark() {
    let mut harness = draw(Scheme::Dark);
    harness.snapshot("code_editor_dark");
}

struct One {
    text: String,
    language: CodeLanguage,
    words: Vec<CodeCompletion>,
    schema: Option<Value>,
    run: bool,
    problems: Vec<String>,
}

fn one(text: &str, language: CodeLanguage) -> Harness<'static, One> {
    let mut harness = Harness::builder().with_size([480.0, 200.0]).build_ui_state(
        |ui, one: &mut One| {
            install(ui.ctx(), &Appearance::default());
            let mut editor = CodeEditor::new("one", &mut one.text)
                .label("Script")
                .language(one.language)
                .completions(&one.words);
            if let Some(schema) = &one.schema {
                editor = editor.schema(schema);
            }
            let shown = editor.show(ui);
            one.run |= shown.run;
            one.problems = shown.problems.into_iter().map(|p| p.message).collect();
        },
        One {
            text: text.into(),
            language,
            words: Vec::new(),
            schema: None,
            run: false,
            problems: Vec::new(),
        },
    );
    harness.run_steps(2);
    harness
}

fn focus(harness: &mut Harness<'static, One>) {
    harness.get_by_label("Script").click();
    harness.run_steps(2);
}

#[test]
fn tab_indents_the_line_and_enter_keeps_its_indent() {
    let mut harness = one("steps:", CodeLanguage::Yaml);
    focus(&mut harness);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.key_press(Key::ArrowRight);
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    harness.get_by_label("Script").type_text("- id: fetch");
    harness.run_steps(2);
    assert_eq!(harness.state().text, "steps:\n  - id: fetch");
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    assert_eq!(harness.state().text, "steps:\n    - id: fetch");
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    harness.run_steps(2);
    assert_eq!(harness.state().text, "steps:\n  - id: fetch");
}

#[test]
fn mod_enter_runs() {
    let mut harness = one(".runs", CodeLanguage::Jq);
    focus(&mut harness);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run_steps(2);
    assert!(harness.state().run);
    assert_eq!(harness.state().text, ".runs", "and breaks no line");
}

#[test]
fn the_schema_s_problems_are_reported() {
    let mut harness = one("name: Nightly\n", CodeLanguage::Yaml);
    harness.state_mut().schema = Some(schema());
    harness.run_steps(2);
    assert_eq!(
        harness.state().problems,
        ["Missing the property steps", "Must match ^[a-z][a-z0-9-]*$"]
    );
}

#[test]
fn a_word_offered_goes_in_with_enter() {
    let mut harness = one("", CodeLanguage::Text);
    harness.state_mut().words = vec![CodeCompletion {
        label: "nightly-export".into(),
        kind: Some("variable".into()),
        detail: Some("job".into()),
        ..Default::default()
    }];
    focus(&mut harness);
    harness.get_by_label("Script").type_text("nig");
    harness.run_steps(3);
    assert!(harness.query_by_label("nightly-export").is_some(), "the list is open");
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().text, "nightly-export");
}

struct Find {
    text: String,
    schema: Value,
}

#[test]
fn the_find_bar_and_the_caret_s_line() {
    let mut harness = Harness::builder().with_size([640.0, 260.0]).build_ui_state(
        |ui, find: &mut Find| {
            install(ui.ctx(), &Appearance::default());
            CodeEditor::new("find", &mut find.text)
                .label("nightly-export.json")
                .language(CodeLanguage::Json)
                .schema(&find.schema)
                .search(true)
                .bracket_colors(true)
                .folding(true)
                .height(240.0)
                .show(ui);
        },
        Find {
            text: "{\n  \"name\": \"nightly-export\",\n  \"steps\": [\n    {\"id\": \"fetch\", \"kind\": \"fetch\"},\n    {\"id\": \"upload\", \"kind\": \"upload\"}\n  ]\n}".into(),
            schema: schema(),
        },
    );
    harness.run_steps(2);
    harness.get_by_label("nightly-export.json").click();
    harness.run_steps(2);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
    harness.run_steps(2);
    harness.get_by_label("Find").type_text("fetch");
    harness.run_steps(2);
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    harness.snapshot("code_editor_find");
}

#[test]
fn the_words_offered_while_typing() {
    let mut harness = one("{\"id\": \"fetch\", ", CodeLanguage::Json);
    harness.state_mut().schema = Some(schema()["$defs"]["step"].clone());
    harness.run_steps(2);
    focus(&mut harness);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.key_press(Key::ArrowRight);
    harness.run_steps(2);
    harness.get_by_label("Script").type_text("\"");
    harness.run_steps(3);
    harness.snapshot("code_editor_completion");
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(
        harness.state().text,
        "{\"id\": \"fetch\", \"kind\": ",
        "the required key first, its quotes whole, opened for its value"
    );
}

#[test]
fn a_problem_says_itself_on_hover() {
    let mut harness = one(
        "name: Nightly\nsteps:\n  - id: fetch\n    kind: fetch\n",
        CodeLanguage::Yaml,
    );
    harness.state_mut().schema = Some(schema());
    harness.run_steps(2);
    // the pointer rests on the name's value, past the hover's wait
    let at = harness.get_by_label("Script").rect().min + egui::vec2(56.0 + 12.0 + 9.0 * 8.0, 8.0 + 11.0);
    harness.input_mut().events.push(egui::Event::PointerMoved(at));
    for _ in 0..30 {
        harness.step();
    }
    harness.snapshot("code_editor_hover");
}
