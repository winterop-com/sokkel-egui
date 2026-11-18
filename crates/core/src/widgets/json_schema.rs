//! The JSON Schema 2020-12 meta-schema, after sokkel-ui's `json-schema`.

use std::sync::OnceLock;

/// The JSON Schema 2020-12 meta-schema as one self-contained schema, as sokkel-ui's
/// `JSON_SCHEMA_2020_12`: every vocabulary's keywords in its `properties`, each subschema reached by
/// `$dynamicRef: '#meta'` back at the root, and the shared parts in `$defs`. Given as `schema` to
/// `CodeEditor`, a JSON Schema being written completes its keywords and is checked by it.
pub fn json_schema_2020_12() -> &'static serde_json::Value {
    static SCHEMA: OnceLock<serde_json::Value> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        serde_json::from_str(include_str!("json_schema_2020_12.json")).unwrap_or(serde_json::Value::Null)
    })
}
