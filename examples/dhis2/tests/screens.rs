//! The workbench's screens at the web's routes, at 1440 by 900 on the parity day
//! (`parity/dhis2.json`), each a second after the page opened as the backend's first answers come:
//! `bun scripts/parity.ts board dhis2` puts them beside the web's. Then a few things the reader
//! does: an indicator's numerator edited changes what it is worth and a bad one is a problem, a
//! new validation rule is listed, and a server too old for a function flags it.

use dhis2::WorkbenchApp;
use dhis2::pages::validation_rules;
use dhis2::workbench::DEMO_START;
use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, SnapshotResults};
use sokkel_egui::Appearance;

pub fn render(path: &str) -> Harness<'static, WorkbenchApp> {
    let mut app = WorkbenchApp::at(DEMO_START, Appearance::default());
    app.go(path);
    let mut harness = Harness::builder().with_size([1440.0, 900.0]).build_ui_state(
        |ui, app: &mut WorkbenchApp| {
            eframe::egui::CentralPanel::default()
                .frame(eframe::egui::Frame::NONE)
                .show(ui, |ui| app.show(ui));
        },
        app,
    );
    harness.run_steps(2);
    harness.state_mut().wait(1_000);
    harness.run_steps(4);
    harness
}

/// Moves the clock on past the backend's answers and draws again.
fn settle(harness: &mut Harness<'static, WorkbenchApp>) {
    harness.run_steps(2);
    harness.state_mut().wait(1_000);
    harness.run_steps(4);
}

#[test]
fn every_screen() {
    let mut results = SnapshotResults::new();
    for (name, path) in [
        ("dhis2_indicators", "indicators"),
        ("dhis2_validation_rules", "validation-rules"),
        ("dhis2_program_rules", "program-rules"),
        ("dhis2_data_elements", "data-elements"),
        ("dhis2_org_units", "org-units"),
        ("dhis2_functions", "functions"),
        ("dhis2_indicator", "indicators/HfLcB4WfnnX"),
        ("dhis2_validation_rule", "validation-rules/wkoeUEMtnuV"),
    ] {
        let mut harness = render(path);
        results.add(harness.try_snapshot(name));
    }
    let mut harness = render("indicators");
    harness.state_mut().go("settings");
    harness.run_steps(4);
    results.add(harness.try_snapshot("dhis2_settings"));
}

#[test]
fn an_edited_numerator_changes_the_value_and_a_bad_one_is_a_problem() {
    let mut harness = render("indicators/HfLcB4WfnnX");
    let version = harness.state().version();
    let ids: Vec<String> = harness
        .state()
        .backend
        .indicators
        .seen()
        .iter()
        .map(|one| one.id.clone())
        .collect();
    let value = |harness: &Harness<'static, WorkbenchApp>| {
        let editor = harness.state().views.indicator.as_ref().expect("the editor");
        let checks = editor.checks(version, &ids);
        (editor.now(&checks).and_then(|one| one.value), checks)
    };
    let (before, checks) = value(&harness);
    assert!(before.is_some());
    assert!(checks.numerator.findings.is_empty());
    let numerator = harness
        .state()
        .views
        .indicator
        .as_ref()
        .expect("the editor")
        .draft
        .numerator
        .clone();
    harness
        .state_mut()
        .views
        .indicator
        .as_mut()
        .expect("the editor")
        .draft
        .numerator = format!("{numerator} * 2");
    harness.run_steps(2);
    let (doubled, _) = value(&harness);
    assert_eq!(
        doubled.map(|one| (one * 10.0).round()),
        before.map(|one| (one * 20.0).round())
    );
    harness
        .state_mut()
        .views
        .indicator
        .as_mut()
        .expect("the editor")
        .draft
        .numerator = "#{nothingHere1} +".to_owned();
    harness.run_steps(2);
    let (broken, checks) = value(&harness);
    assert_eq!(broken, None);
    assert!(!checks.numerator.findings.is_empty(), "a bad numerator is a problem");
    // the problem shows on the field's issue mark
    assert!(harness.query_by_label_contains("Error: ").is_some());
}

#[test]
fn a_new_validation_rule_is_listed() {
    let mut harness = render("validation-rules");
    let before = harness.state().backend.validation_rules.seen().len();
    {
        let app = harness.state_mut();
        validation_rules::create(&mut app.backend, before);
    }
    settle(&mut harness);
    let rules = harness.state().backend.validation_rules.seen().to_vec();
    assert_eq!(rules.len(), before + 1);
    let made = rules
        .iter()
        .find(|one| one.name.starts_with("New rule"))
        .expect("the rule made");
    // the list goes on to the rule made
    assert_eq!(harness.state().route, ["validation-rules", made.id.as_str()]);
    harness.state_mut().go("validation-rules");
    settle(&mut harness);
    assert!(
        harness.query_by_label_contains(&made.name).is_some() || harness.query_by_label_contains(&made.id).is_some()
    );
}

#[test]
fn an_older_server_flags_a_function_it_lacks() {
    let mut harness = render("functions");
    let known = |harness: &Harness<'static, WorkbenchApp>| {
        dhis2::pages::functions::rows(harness.state().version())
            .into_iter()
            .find(|one| one.name == "d2:extractDataMatrixValue")
            .expect("the function")
            .known
    };
    assert!(known(&harness));
    harness.state_mut().set_version("2.43");
    harness.run_steps(4);
    assert!(!known(&harness));
    assert_eq!(harness.state().version(), "2.43");
    // the editors are handed the older version's names: the function reads as a plain name
    let language =
        sokkel_egui::plugins::plugin_language(&harness.ctx, sokkel_egui_dhis2::DHIS2_RULE).expect("the language");
    let read = (language.highlight)("d2:extractDataMatrixValue(1)");
    assert!(
        read.first()
            .is_none_or(|one| one.kind != sokkel_egui::CodeTag::Function)
    );
    let read = (language.highlight)("d2:hasValue(1)");
    assert_eq!(read[0].kind, sokkel_egui::CodeTag::Function);
}
