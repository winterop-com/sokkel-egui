//! A form drawn from a JSON Schema: a pipeline step's settings as the studio draws them (fetch and
//! validate), a form with a list and a map, and a form with its issues shown; and what it does: it
//! writes each change into the value it is given, shows issues when the web shows them, leaves
//! deferred values unchecked, edits lists and maps, and puts in a fix.

use egui::accesskit::Role;
use egui_kittest::{Harness, kittest::Queryable};
use serde_json::{Value, json};
use sokkel_egui::{Appearance, IssueTone, SchemaForm, SchemaFormResponse, ServerIssue, install};

/// The studio's pipeline schema's step and kinds, as `examples/studio/web/src/pipeline.ts` has them.
fn pipeline_defs() -> Value {
    json!({
        "step": {
            "type": "object",
            "required": ["id", "kind"],
            "additionalProperties": false,
            "properties": {
                "id": {"type": "string", "pattern": "^[a-z][a-z0-9-]*$", "description": "The step's name in this pipeline, which needs and references use."},
                "kind": {"enum": ["fetch", "transform", "validate", "upload", "notify"], "description": "What the step does; its kind decides what goes in with."},
                "needs": {"type": "array", "uniqueItems": true, "items": {"type": "string"}, "description": "The steps that finish before this one starts."},
                "retries": {"type": "integer", "minimum": 0, "maximum": 5, "default": 0, "description": "How many times a failed try is tried again, up to 5."},
                "timeout": {"type": "string", "pattern": "^[0-9]+(s|m|h)$", "default": "10m", "description": "How long one try may take: 30s, 10m, 1h."},
                "with": {"type": "object", "description": "The step's settings, which its kind decides."}
            }
        },
        "fetch": {
            "type": "object",
            "required": ["url"],
            "additionalProperties": false,
            "properties": {
                "url": {"type": "string", "pattern": "^https://", "description": "Where the rows are read from, over https."},
                "format": {"enum": ["csv", "json", "ndjson"], "default": "json", "description": "How the answer is written."},
                "since": {"type": "string", "description": "Only rows changed since this long ago, such as 1d."}
            }
        },
        "validate": {
            "type": "object",
            "required": ["rules"],
            "additionalProperties": false,
            "properties": {
                "rules": {
                    "type": "array",
                    "minItems": 1,
                    "description": "The checks each row passes.",
                    "items": {
                        "type": "object",
                        "required": ["field", "check"],
                        "additionalProperties": false,
                        "properties": {
                            "field": {"type": "string", "description": "The field the rule checks."},
                            "check": {"enum": ["required", "unique", "positive", "email", "date"], "description": "What the field must be."}
                        }
                    }
                },
                "on_invalid": {"enum": ["fail", "skip", "warn"], "default": "fail", "description": "What refused rows do: fail the run past max_rejected, be left out, or pass with a warning."},
                "max_rejected": {"type": "integer", "minimum": 0, "default": 0, "description": "How many refused rows fail allows before it fails."}
            }
        }
    })
}

/// What a step of a kind is set with, as the studio's `settingsSchema(kind)`: its retries and
/// timeout, and the settings its kind takes.
fn settings_schema(kind: &str) -> Value {
    let defs = pipeline_defs();
    let step = &defs["step"]["properties"];
    let titled = |schema: &Value, title: &str| {
        let mut out = schema.clone();
        out["title"] = json!(title);
        out
    };
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "retries": titled(&step["retries"], "Retries"),
            "timeout": titled(&step["timeout"], "Timeout"),
            "with": titled(&defs[kind], "Settings")
        }
    })
}

/// A value holding a `${...}` reference, filled in when the pipeline runs, as the studio's
/// `holdsReference`.
fn holds_reference(value: &Value) -> bool {
    value.as_str().is_some_and(|s| s.contains("${"))
}

struct Page {
    schema: Value,
    value: Value,
    response: SchemaFormResponse,
    submitted: bool,
    deferred: bool,
    fold: bool,
    open: Option<Vec<&'static str>>,
    server: Vec<ServerIssue>,
}

fn page(schema: Value, value: Value) -> Page {
    Page {
        schema,
        value,
        response: SchemaFormResponse::default(),
        submitted: false,
        deferred: false,
        fold: false,
        open: None,
        server: Vec::new(),
    }
}

/// The form in a column `width` wide, at the left of a window wide enough not to be a phone's.
fn harness(page: Page, [width, height]: [f32; 2]) -> Harness<'static, Page> {
    let mut harness = Harness::builder().with_size([800.0, height]).build_ui_state(
        move |ui, page: &mut Page| {
            install(ui.ctx(), &Appearance::default());
            egui::Frame::new().inner_margin(16.0).show(ui, |ui| {
                ui.set_max_width(width - 32.0);
                let reference: &dyn Fn(&Value) -> bool = &holds_reference;
                let mut form = SchemaForm::new("settings", &page.schema, &mut page.value)
                    .submitted(page.submitted)
                    .fold(page.fold)
                    .issues(&page.server);
                if page.deferred {
                    form = form.deferred(reference);
                }
                if let Some(open) = &page.open {
                    form = form.open(open);
                }
                page.response = form.show(ui);
            });
        },
        page,
    );
    harness.run_steps(4);
    harness
}

/// Types into the text box a label names, as a person does: pressing it first.
fn type_into(harness: &mut Harness<'static, Page>, label: &str, text: &str) {
    harness.get_by_role_and_label(Role::TextInput, label).click();
    harness.run_steps(1);
    harness.get_by_role_and_label(Role::TextInput, label).type_text(text);
    harness.run_steps(3);
}

fn texts(harness: &Harness<'static, Page>) -> Vec<String> {
    harness.state().response.issues.iter().map(|i| i.text.clone()).collect()
}

// --- What it draws -----------------------------------------------------------------------------

#[test]
fn a_fetch_steps_settings() {
    let mut page = page(
        settings_schema("fetch"),
        json!({"retries": 2, "with": {"url": "${params.source}", "format": "csv"}}),
    );
    page.submitted = true;
    page.deferred = true;
    let mut harness = harness(page, [440.0, 500.0]);
    assert!(harness.state().response.issues.is_empty(), "{:?}", texts(&harness));
    harness.snapshot("schema_form_fetch");
}

#[test]
fn a_validate_steps_settings() {
    let mut page = page(
        settings_schema("validate"),
        json!({
            "with": {
                "rules": [{"field": "order_id", "check": "unique"}, {"field": "email", "check": "email"}],
                "on_invalid": "skip"
            }
        }),
    );
    page.submitted = true;
    page.deferred = true;
    let mut harness = harness(page, [440.0, 900.0]);
    assert!(harness.state().response.issues.is_empty(), "{:?}", texts(&harness));
    harness.snapshot("schema_form_validate");
}

fn lists_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "labels": {"type": "array", "title": "Labels", "items": {"type": "string"}, "description": "Shown beside the job in the runs list."},
            "headers": {"type": "object", "title": "Headers", "additionalProperties": {"type": "string"}},
            "limits": {"type": "object", "title": "Limits", "additionalProperties": {"type": "integer", "minimum": 0}}
        }
    })
}

#[test]
fn a_form_with_a_list_and_maps() {
    let page = page(
        lists_schema(),
        json!({
            "labels": ["exports", "orders"],
            "headers": {"x-team": "harbor", "accept": "text/csv"},
            "limits": {"worker": 4}
        }),
    );
    let mut harness = harness(page, [640.0, 520.0]);
    assert!(harness.query_by_label("Headers key 1").is_some());
    assert!(harness.query_by_label("Headers value 2").is_some());
    assert!(harness.query_by_label("Remove Headers x-team").is_some());
    assert!(harness.query_by_label("Limits key 1").is_some());
    assert!(harness.query_by_label("Move down: Item 1").is_some());
    assert!(harness.query_by_label("Remove Item 2").is_some());
    harness.snapshot("schema_form_lists");
}

fn issues_schema() -> Value {
    json!({
        "type": "object",
        "required": ["code", "owner"],
        "properties": {
            "owner": {"type": "string", "title": "Owner", "format": "email"},
            "retries": {"type": "integer", "title": "Retries", "minimum": 0, "maximum": 5},
            "priority": {"title": "Priority", "enum": ["low", "normal", "high"]},
            "legacy": {"type": "boolean", "title": "Legacy mode", "deprecated": true},
            "tags": {"type": "array", "title": "Tags", "minItems": 1, "items": {"type": "string"}},
            "code": {"type": "string", "title": "Code", "pattern": "^[a-z][a-z0-9-]*$", "description": "Lowercase letters, digits and dashes."}
        }
    })
}

#[test]
fn a_form_with_its_issues_shown() {
    let mut page = page(
        issues_schema(),
        json!({"code": "Nightly Export", "owner": "ada", "retries": 9, "priority": "urgent", "legacy": true, "tags": []}),
    );
    page.submitted = true;
    page.open = Some(vec!["code"]);
    let mut harness = harness(page, [640.0, 560.0]);
    let said = texts(&harness);
    for text in [
        "Not in the expected form",
        "Enter an email address",
        "Must be at most 5",
        "Choose one of the values",
        "Deprecated",
        "Add at least 1 item",
    ] {
        assert!(said.contains(&text.to_owned()), "{text} in {said:?}");
    }
    assert_eq!(harness.state().response.warning, 1);
    harness.snapshot("schema_form_issues");
}

// --- What it does ------------------------------------------------------------------------------

#[test]
fn a_change_comes_back_to_the_caller_each_frame() {
    let page = page(lists_schema(), json!({"labels": ["exports"]}));
    let mut harness = harness(page, [640.0, 400.0]);
    harness.get_all_by_label("Add").next().unwrap().click();
    harness.run_steps(2);
    assert_eq!(harness.state().value["labels"], json!(["exports", ""]));
    type_into(&mut harness, "Labels 2", "orders");
    assert_eq!(harness.state().value["labels"], json!(["exports", "orders"]));
    harness.get_by_label("Move up: Item 2").click();
    harness.run_steps(2);
    assert_eq!(harness.state().value["labels"], json!(["orders", "exports"]));
    harness.get_by_label("Remove Item 1").click();
    harness.run_steps(2);
    assert_eq!(harness.state().value["labels"], json!(["exports"]));
}

#[test]
fn issues_wait_until_submitted_but_the_block_is_known() {
    let schema = json!({"type": "object", "required": ["code"], "properties": {"code": {"type": "string"}}});
    let harness = harness(page(schema.clone(), json!({})), [480.0, 200.0]);
    assert!(harness.state().response.blocking);
    assert_eq!(harness.state().response.critical, 0);
    let harness = self::harness(page(schema, json!({"code": "send-digest"})), [480.0, 200.0]);
    assert!(!harness.state().response.blocking);
}

#[test]
fn a_form_submitted_shows_every_issue() {
    let schema = json!({"type": "object", "required": ["code", "owner"], "properties": {"code": {"type": "string"}, "owner": {"type": "string"}}});
    let mut page = page(schema, json!({}));
    page.submitted = true;
    let harness = harness(page, [480.0, 260.0]);
    assert_eq!(harness.state().response.critical, 2);
    assert_eq!(texts(&harness), vec!["Enter a value", "Enter a value"]);
    assert_eq!(harness.query_all_by_label("Error: Enter a value").count(), 2);
}

#[test]
fn a_required_boolean_without_a_default_asks_for_nothing_more() {
    let schema = json!({
        "type": "object",
        "required": ["paused"],
        "properties": {"paused": {"type": "boolean", "title": "Paused"}, "notify": {"type": "boolean", "title": "Notify"}}
    });
    let mut page = page(schema.clone(), sokkel_egui::initial_value(&schema));
    page.submitted = true;
    let harness = harness(page, [480.0, 200.0]);
    assert!(!harness.state().response.blocking);
    assert_eq!(harness.state().response.critical, 0);
}

#[test]
fn an_empty_row_in_a_list_of_text_says_it_is_empty() {
    let schema = json!({"type": "object", "properties": {"tags": {"type": "array", "title": "Tags", "items": {"type": "string"}}}});
    let mut page = page(schema, json!({"tags": ["nightly", ""]}));
    page.submitted = true;
    let harness = harness(page, [480.0, 260.0]);
    let issues = &harness.state().response.issues;
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].path, vec!["tags".to_owned(), "1".to_owned()]);
    assert_eq!(issues[0].text, "Enter a value");
    assert_eq!(harness.state().value, json!({"tags": ["nightly", ""]}));
}

#[test]
fn a_deferred_reference_is_left_unchecked() {
    let mut page = page(
        settings_schema("fetch"),
        json!({"retries": "${params.retries}", "with": {"url": "${params.source}"}}),
    );
    page.submitted = true;
    page.deferred = true;
    let harness = harness(page, [440.0, 420.0]);
    assert!(harness.state().response.issues.is_empty(), "{:?}", texts(&harness));
    let mut page = self::page(settings_schema("fetch"), json!({"with": {"url": "${params.source}"}}));
    page.submitted = true;
    let harness = self::harness(page, [440.0, 420.0]);
    assert_eq!(texts(&harness), vec!["Not in the expected form"]);
}

#[test]
fn the_servers_issues_show_until_their_field_changes() {
    let schema = json!({"type": "object", "properties": {"code": {"type": "string", "title": "Code"}}});
    let mut page = page(schema, json!({"code": "x"}));
    page.server = vec![ServerIssue {
        path: vec!["code".into()],
        text: "Taken".into(),
        detail: None,
        tone: IssueTone::Critical,
    }];
    let mut harness = harness(page, [480.0, 200.0]);
    assert_eq!(texts(&harness), vec!["Taken"]);
    assert_eq!(harness.state().response.issues[0].label, "Code");
    assert!(harness.query_by_role_and_label(Role::Button, "Error: Taken").is_some());
    type_into(&mut harness, "Code", "y");
    assert!(harness.state().response.issues.is_empty());
}

#[test]
fn an_unreadable_number_in_a_map_row_blocks_until_its_row_is_removed() {
    let schema = json!({"type": "object", "properties": {"limits": {"type": "object", "title": "Limits", "additionalProperties": {"type": "number"}}}});
    let mut harness = harness(page(schema, json!({"limits": {"api": 1, "worker": 2}})), [640.0, 300.0]);
    assert!(!harness.state().response.blocking);
    type_into(&mut harness, "Limits value 1", "abc");
    assert!(harness.state().response.blocking);
    harness.get_by_label("Remove Limits api").click();
    harness.run_steps(3);
    assert_eq!(harness.state().value, json!({"limits": {"worker": 2}}));
    assert!(!harness.state().response.blocking);
}

#[test]
fn a_repeated_map_key_stops_the_submit() {
    let schema = json!({"type": "object", "properties": {"headers": {"type": "object", "title": "Headers", "additionalProperties": {"type": "string"}}}});
    let mut page = page(schema, json!({"headers": {"x-team": "harbor"}}));
    page.submitted = true;
    let mut harness = harness(page, [640.0, 300.0]);
    harness.get_by_label("Add").click();
    harness.run_steps(2);
    type_into(&mut harness, "Headers key 2", "x-team");
    assert_eq!(texts(&harness), vec!["Each key only once"]);
    assert_eq!(harness.state().value, json!({"headers": {"x-team": "harbor"}}));
}

#[test]
fn a_fix_puts_in_the_value_it_offers() {
    let schema =
        json!({"type": "object", "properties": {"retries": {"type": "integer", "title": "Retries", "maximum": 5}}});
    let mut page = page(schema, json!({"retries": 9}));
    page.submitted = true;
    let mut harness = harness(page, [480.0, 320.0]);
    assert_eq!(texts(&harness), vec!["Must be at most 5"]);
    harness
        .get_by_role_and_label(Role::Button, "Error: Must be at most 5")
        .click();
    // the popover takes a frame to measure itself before it shows
    harness.run_steps(4);
    harness.get_by_label("Use 5").click();
    harness.run_steps(3);
    assert_eq!(harness.state().value, json!({"retries": 5}));
    assert!(harness.state().response.issues.is_empty());
}

#[test]
fn a_read_only_group_offers_no_add_or_remove() {
    let schema = json!({
        "type": "object",
        "properties": {
            "labels": {"type": "array", "readOnly": true, "items": {"type": "string"}},
            "steps": {"type": "array", "readOnly": true, "items": {"type": "object", "properties": {"step": {"type": "string"}, "retries": {"type": "integer"}}}}
        }
    });
    let harness = harness(
        page(
            schema,
            json!({"labels": ["exports", "orders"], "steps": [{"step": "fetch", "retries": 2}]}),
        ),
        [480.0, 400.0],
    );
    assert!(harness.query_by_label("Add").is_none());
    assert!(harness.query_by_label("Remove").is_none());
    assert!(harness.query_by_label("Remove Item 1").is_none());
}

#[test]
fn a_folded_form_puts_the_rest_behind_one_disclosure() {
    let schema = json!({
        "type": "object",
        "required": ["code"],
        "properties": {
            "code": {"type": "string", "title": "Code"},
            "name": {"type": "string", "title": "Name"},
            "region": {"type": "string", "title": "Region"},
            "retries": {"type": "integer", "title": "Retries"}
        }
    });
    let mut page = page(schema.clone(), json!({"name": "Nightly export"}));
    page.fold = true;
    let harness = harness(page, [480.0, 300.0]);
    assert!(harness.query_by_label("2 more fields").is_some());
    assert!(harness.query_all_by_label("Region").next().is_none());
    let mut page = self::page(schema, json!({"code": "nightly-export"}));
    page.fold = true;
    page.server = vec![ServerIssue {
        path: vec!["region".into()],
        text: "No such region".into(),
        detail: None,
        tone: IssueTone::Critical,
    }];
    let harness = self::harness(page, [480.0, 400.0]);
    assert!(harness.query_all_by_label("Region").next().is_some());
    assert!(harness.query_by_label("Fewer fields").is_some());
}
