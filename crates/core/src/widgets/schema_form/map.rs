//! A map's rows, after sokkel-ui's `schema/MapField.tsx`: an object of any keys whose values are
//! each one scalar, such as headers or environment variables, as a key and a value per row.

use serde_json::{Map, Value};

use super::read::{get, types};

/// One row of a map: its key as typed and its value, `None` while it has none.
#[derive(Clone, Debug, PartialEq)]
pub struct MapRow {
    pub id: u64,
    pub key: String,
    pub value: Option<Value>,
}

/// The rows of a map's value, one per key in its order, numbered on from `next`.
pub fn rows_of(value: Option<&Value>, next: &mut u64) -> Vec<MapRow> {
    let Some(Value::Object(map)) = value else {
        return Vec::new();
    };
    map.iter()
        .map(|(key, one)| {
            *next += 1;
            MapRow {
                id: *next,
                key: key.clone(),
                value: Some(one.clone()),
            }
        })
        .collect()
}

/// The ids of the rows whose key an earlier row already has.
pub fn repeated_keys(rows: &[MapRow]) -> Vec<u64> {
    let mut seen: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for row in rows {
        if row.key.is_empty() {
            continue;
        }
        if seen.contains(&row.key.as_str()) {
            out.push(row.id);
        }
        seen.push(&row.key);
    }
    out
}

/// The map's value from its rows: a row without a key, or with a key an earlier row has, is left
/// out; a row without a value takes `blank` (an empty string for a map of text), else is left out.
pub fn object_of(rows: &[MapRow], blank: Option<&Value>) -> Value {
    let mut out = Map::new();
    for row in rows {
        let Some(value) = row.value.as_ref().or(blank) else {
            continue;
        };
        if row.key.is_empty() || out.contains_key(&row.key) {
            continue;
        }
        out.insert(row.key.clone(), value.clone());
    }
    Value::Object(out)
}

/// What a row holds before it is given a value: `""` for text, its default or false for a switch,
/// else nothing.
pub(crate) fn blank_of(item: &Value) -> Option<Value> {
    if get(item, "anyOf").is_some() || get(item, "oneOf").is_some() {
        return None;
    }
    match types(item).0.first().map(String::as_str) {
        Some("string") => Some(Value::String(String::new())),
        Some("boolean") if get(item, "enum").is_none() => {
            Some(Value::Bool(get(item, "default") == Some(&Value::Bool(true))))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn row(id: u64, key: &str, value: Option<Value>) -> MapRow {
        MapRow {
            id,
            key: key.into(),
            value,
        }
    }

    #[test]
    fn the_first_of_a_repeated_key_is_kept_and_a_row_without_a_key_left_out() {
        let rows = [
            row(1, "x-team", Some(json!("harbor"))),
            row(2, "", Some(json!("lost"))),
            row(3, "x-team", Some(json!("juniper"))),
            row(4, "accept", None),
        ];
        assert_eq!(repeated_keys(&rows), vec![3]);
        assert_eq!(
            object_of(&rows, Some(&json!(""))),
            json!({"x-team": "harbor", "accept": ""})
        );
        assert_eq!(object_of(&rows, None), json!({"x-team": "harbor"}));
        let mut next = 0;
        let read: Vec<(String, Option<Value>)> = rows_of(Some(&json!({"a": 1, "b": 2})), &mut next)
            .into_iter()
            .map(|r| (r.key, r.value))
            .collect();
        assert_eq!(read, vec![("a".into(), Some(json!(1))), ("b".into(), Some(json!(2)))]);
        assert!(rows_of(None, &mut next).is_empty());
    }

    #[test]
    fn a_key_an_object_already_has_is_a_key_of_the_map_like_any_other() {
        let rows = [
            row(1, "constructor", Some(json!("a"))),
            row(2, "__proto__", Some(json!("b"))),
            row(3, "toString", Some(json!("c"))),
        ];
        assert_eq!(
            serde_json::to_string(&object_of(&rows, None)).unwrap(),
            r#"{"constructor":"a","__proto__":"b","toString":"c"}"#
        );
    }
}
