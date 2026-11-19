//! The fhir add-on's story, FHIRPath and CQL in the core's code editor, and the parts the fhir
//! example brought into the core: the diff view and a rail's places under a parent.

use eframe::egui::{self, Sense, Ui, vec2};
use sokkel_egui::plugins::register_plugins;
use sokkel_egui::{
    CodeEditor, CodeLanguage, CodeSize, CodeVariant, DiffView, DiffViewWords, Field, Icon, Rail, RailEntry, RailGroup,
};
use sokkel_egui_fhir::{CQL, FHIRPATH, fhir};

use super::{State, section};

fn held<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, from: impl FnOnce() -> T) -> T {
    ui.data(|d| d.get_temp::<T>(egui::Id::unique(("story-fhir", key))))
        .unwrap_or_else(from)
}

fn hold<T: Clone + Send + Sync + 'static>(ui: &Ui, key: &str, value: T) {
    ui.data_mut(|d| d.insert_temp(egui::Id::unique(("story-fhir", key)), value));
}

const EXPRESSION: &str = "// Who had a systolic pressure of 140 or more\nentry.resource.ofType(Observation)\n  .where(code.coding.exists(system = %loinc and code = '8480-6'))\n  .where(value.ofType(Quantity).value >= 140 and effective >= @2026-01-01)\n  .subject.display\n  .distinct()";

const LIBRARY: &str = "library WeightFollowUp version '0.1.0'\n\nusing FHIR version '4.0.1'\n\ncodesystem \"LOINC\": 'http://loinc.org'\ncode \"Body weight\": '29463-7' from \"LOINC\"\n\ncontext Patient\n\n/* A patient weighed in the last\n   three months */\ndefine \"Weighed Lately\":\n  exists ([Observation: \"Body weight\"] O\n    where O.effective during Interval[Today() - 3 months, Today()]\n      and O.status in { 'final', 'amended' })\n";

/// A FHIRPath expression over the whole server and a CQL library, each colored by the add-on:
/// functions, members after a dot, bound names, dates, strings and comments.
pub fn languages(_: &mut State, ui: &mut Ui) {
    if sokkel_egui::plugins::plugin_language(ui.ctx(), FHIRPATH).is_none() {
        register_plugins(ui.ctx(), [fhir()]);
    }
    ui.set_max_width(640.0);
    section(ui, "FHIRPath: an expression over the whole server");
    let mut expression = held(ui, "expression", || EXPRESSION.to_owned());
    Field::new("Expression").show(ui, |ui| {
        CodeEditor::new("story-fhirpath", &mut expression)
            .language(CodeLanguage::Plugin(FHIRPATH))
            .size(CodeSize::Sm)
            .bracket_colors(true)
            .show(ui);
    });
    hold(ui, "expression", expression);
    ui.add_space(16.0);
    section(ui, "CQL: a library in a pane");
    let mut library = held(ui, "library", || LIBRARY.to_owned());
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 320.0), Sense::hover());
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        CodeEditor::new("story-cql", &mut library)
            .language(CodeLanguage::Plugin(CQL))
            .variant(CodeVariant::Pane)
            .size(CodeSize::Sm)
            .bracket_colors(true)
            .show(ui);
    });
    hold(ui, "library", library);
}

const BEFORE: &str = "library HypertensionControl version '1.0.0'\n\nusing FHIR version '4.0.1'\n\ncontext Patient\n\ndefine \"Latest Systolic\":\n  Last([Observation: \"Systolic blood pressure\"] O\n    where O.effective during \"Measurement Period\"\n    sort by effective).value as Quantity\n\ndefine \"Controlled\":\n  \"Latest Systolic\" < 140 'mm[Hg]'\n\ndefine \"In Numerator\":\n  \"Controlled\"\n";

/// A library's version against the text in the editor: the changes marked on their washes, the far
/// unchanged lines folded, the counts at the top end; and two texts the same.
pub fn diff_view(_: &mut State, ui: &mut Ui) {
    ui.set_max_width(560.0);
    let after = BEFORE.replace("version '1.0.0'", "version '1.1.0'").replace(
        "    where O.effective during \"Measurement Period\"\n",
        "    where O.status in { 'final', 'amended' }\n      and O.effective during \"Measurement Period\"\n",
    );
    section(ui, "A version against the editor, small");
    DiffView::new("story-diff", BEFORE, &after).small(true).show(ui);
    ui.add_space(16.0);
    section(ui, "With a title, every line drawn");
    DiffView::new("story-diff-titled", BEFORE, &after)
        .title("HypertensionControl.cql")
        .context(usize::MAX)
        .show(ui);
    ui.add_space(16.0);
    section(ui, "The same text");
    DiffView::new("story-diff-same", BEFORE, BEFORE)
        .small(true)
        .context(1)
        .words(DiffViewWords {
            unchanged: "The editor holds this version.".to_owned(),
            ..Default::default()
        })
        .show(ui);
}

/// A rail whose place holds places of its own: open while one of them is shown, the parent set
/// apart over them; folded, the parent marked for the one shown.
pub fn rail_nested(_: &mut State, ui: &mut Ui) {
    let kinds = [
        ("Patient", 12),
        ("Practitioner", 3),
        ("Organization", 2),
        ("Encounter", 72),
        ("Observation", 216),
    ];
    let groups = vec![
        RailGroup::new(
            None,
            [
                RailEntry::new("overview", "Overview", Icon::LayoutDashboard).description("What the server holds"),
                RailEntry::new("resources", "Resources", Icon::Database)
                    .description("Read and edit as JSON")
                    .entries(kinds.iter().map(|(name, count)| {
                        RailEntry::new(format!("resources-{name}"), *name, Icon::Database).count(*count)
                    })),
            ],
        ),
        RailGroup::new(
            Some("Tools"),
            [
                RailEntry::new("fhirpath", "FHIRPath", Icon::Braces).description("Evaluate expressions"),
                RailEntry::new("cql", "CQL", Icon::FileCode)
                    .description("Libraries in versions")
                    .count(2),
            ],
        ),
    ];
    let mut place: String = held(ui, "rail-place", || "resources-Patient".to_owned());
    section(ui, "Open on the place shown, and collapsed to the parent");
    ui.horizontal_top(|ui| {
        for collapsed in [false, true] {
            let (rect, _) = ui.allocate_exact_size(vec2(Rail::width(collapsed), 640.0), Sense::hover());
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                ui.add(Rail::new("FHIR Workbench", &groups, &mut place).collapsed(collapsed));
            });
            ui.add_space(24.0);
        }
    });
    hold(ui, "rail-place", place);
}
