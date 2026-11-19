//! The workbench's screens at the web's routes, at 1440 by 900 on the parity day
//! (`parity/fhir.json`), each a second after the page opened as the server's first answers come:
//! `bun scripts/parity.ts board fhir` puts them beside the web's. Then a few things the reader does:
//! a patient's JSON edited and saved is version 2 in its history, an invalid edit is a problem and
//! is refused, a FHIRPath expression selects what it should, a search finds what matches, and a
//! library saved is a new version the diff shows.

use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, SnapshotResults};
use fhir::WorkbenchApp;
use fhir::evaluate::{Evaluation, evaluate};
use fhir::pages::{self, PageState};
use fhir::workbench::DEMO_START;
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
    settle(&mut harness);
    harness
}

/// Moves the clock on past the server's answers and draws again.
fn settle(harness: &mut Harness<'static, WorkbenchApp>) {
    harness.run_steps(2);
    harness.state_mut().wait(1_000);
    harness.run_steps(4);
}

#[test]
fn every_screen() {
    let mut results = SnapshotResults::new();
    for (name, path) in [
        ("fhir_overview", "overview"),
        ("fhir_resources", "resources"),
        ("fhir_patients", "resources/Patient"),
        ("fhir_patient", "resources/Patient/pat-001"),
        ("fhir_fhirpath", "fhirpath"),
        ("fhir_cql", "cql"),
        ("fhir_search", "search"),
        ("fhir_cql_library", "cql/lib-hypertension"),
    ] {
        let mut harness = render(path);
        results.add(harness.try_snapshot(name));
    }
    let mut harness = render("overview");
    harness.state_mut().go("settings");
    harness.run_steps(4);
    results.add(harness.try_snapshot("fhir_settings"));
}

/// The text the resource's editor holds.
fn text_of(harness: &Harness<'static, WorkbenchApp>, kind: &str, id: &str) -> String {
    pages::resource_view(&harness.state().app, kind, id).text
}

#[test]
fn an_edited_patient_saved_is_version_two_in_its_history() {
    let mut harness = render("resources/Patient/pat-001");
    let text = text_of(&harness, "Patient", "pat-001").replace("\"Brekke\"", "\"Brekke-Lie\"");
    pages::resource_edit(&mut harness.state_mut().app, "Patient", "pat-001", &text);
    harness.run_steps(2);
    assert!(pages::resource_view(&harness.state().app, "Patient", "pat-001").dirty);
    pages::resource_save(&mut harness.state_mut().app, "Patient", "pat-001");
    settle(&mut harness);
    let view = pages::resource_view(&harness.state().app, "Patient", "pat-001");
    assert_eq!(view.version, 2.0);
    assert!(!view.dirty);
    assert!(view.text.contains("Brekke-Lie"));
    let versions = pages::versions_of(harness.state().app.backend.history.seen(), "Patient/pat-001");
    assert_eq!(versions.first().map(|one| one.version_id), Some(2));
    // the history tab lists the new version
    harness.state_mut().views.tab = Some("history");
    harness.run_steps(3);
    assert!(harness.query_by_label_contains("v2").is_some());
}

#[test]
fn an_invalid_edit_is_a_problem_and_is_refused() {
    let mut harness = render("resources/Patient/pat-001");
    let text = text_of(&harness, "Patient", "pat-001").replace("\"gender\": \"female\"", "\"gender\": \"sometimes\"");
    assert!(text.contains("sometimes"));
    pages::resource_edit(&mut harness.state_mut().app, "Patient", "pat-001", &text);
    harness.run_steps(3);
    // the editor's schema check finds it as it is typed
    assert!(
        harness
            .state()
            .views
            .problems
            .iter()
            .any(|one| one.message.contains("Expected one of")),
        "{:?}",
        harness.state().views.problems
    );
    pages::resource_save(&mut harness.state_mut().app, "Patient", "pat-001");
    settle(&mut harness);
    let view = pages::resource_view(&harness.state().app, "Patient", "pat-001");
    assert_eq!(view.version, 1.0, "the server refuses it");
    assert!(view.dirty, "the edit stays");
    assert!(!harness.state().app.editing.answer("Patient/pat-001").is_empty());
}

#[test]
fn a_fhirpath_expression_selects_the_official_name() {
    let harness = render("fhirpath");
    let app = &harness.state().app;
    let (_, subject) = pages::fhirpath_subject(app, None);
    match evaluate(pages::FHIRPATH_EXAMPLES[0].2, subject.as_ref(), app.backend.now()) {
        Evaluation::Done { value, .. } => assert_eq!(value, vec![serde_json::json!("Solveig Brekke")]),
        other => panic!("{other:?}"),
    }
    // the page shows the result
    assert!(harness.query_by_label_contains("1 item").is_some());
}

#[test]
fn a_search_finds_the_resources_that_match() {
    let mut harness = render("search");
    pages::search_set(&mut harness.state_mut().app, "Patient?address-city=Troms&_sort=family");
    harness.run_steps(3);
    let result = pages::search_result(&harness.state().app).expect("a result");
    assert_eq!(result.total, 6);
    assert!(
        result
            .matches
            .iter()
            .all(|row| row.resource.to_string().contains("Troms\u{f8}"))
    );
    assert!(harness.query_by_label_contains("6 matches").is_some());
}

#[test]
fn a_library_saved_is_a_new_version_the_diff_shows() {
    let mut harness = render("cql/lib-weight");
    let view = pages::cql_view(&harness.state().app, "lib-weight");
    let before = view.own.len();
    let text = format!("{}\n// checked by ada\n", view.text.trim_end());
    pages::cql_edit(&mut harness.state_mut().app, "lib-weight", &text);
    harness.run_steps(2);
    // the diff against the editor counts the line added
    let look = pages::cql_look(&harness.state().app, "lib-weight");
    assert!(look.diff.as_ref().is_some_and(|diff| diff.added >= 1));
    pages::cql_save(&mut harness.state_mut().app, "lib-weight", None);
    settle(&mut harness);
    let view = pages::cql_view(&harness.state().app, "lib-weight");
    assert_eq!(view.own.len(), before + 1);
    assert!(!view.dirty);
    assert!(matches!(
        harness.state().app.page,
        PageState::CqlEditor { saving: false, .. }
    ));
    // compared with the version before, the new line is added
    let older = &view.own[1];
    let lines = sokkel_egui::diff_lines(&pages::cql_of(older.resource.as_ref()).unwrap_or_default(), &view.text);
    assert!(
        lines
            .iter()
            .any(|one| one.kind == sokkel_egui::DiffKind::Added && one.text.contains("checked by ada"))
    );
}
