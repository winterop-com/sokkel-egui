//! The health-data standards' languages as a plugin, as sokkel-ui-fhir's `plugin.ts` gives them.

use sokkel_egui::plugins::SokkelPlugin;

use crate::cql::cql_language;
use crate::fhirpath::fhirpath_language;

/// FHIRPath and CQL as a plugin: installed with `register_plugins(ctx, [fhir()])`, a `CodeEditor`
/// takes `CodeLanguage::Plugin("fhirpath")` and `CodeLanguage::Plugin("cql")`, and
/// `language_for_media_type` reads `text/fhirpath`, `application/fhirpath`, `text/cql`,
/// `text/cql-expression` and `application/cql` as them.
pub fn fhir() -> SokkelPlugin {
    SokkelPlugin {
        name: "fhir",
        languages: vec![fhirpath_language(), cql_language()],
    }
}
