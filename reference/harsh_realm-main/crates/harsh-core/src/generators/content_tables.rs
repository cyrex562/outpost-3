//! Load weighted-table results from content for the generators.
//!
//! Ported from `src/harsh_realm/generators/content_tables.py`. A table is a
//! `{entries: [{weight, result}, ...]}` document; this returns its ordered
//! `result` strings. (The Python module memoized per path with a global cache;
//! the Rust port reads on demand, which the callers can wrap if needed.)

use crate::packs::paths::default_content_dir;
use crate::runtime::JsonValue;

/// Return the ordered `result` strings of a content table under the pack's
/// content dir, or `fallback` when the file is missing/empty.
pub fn load_table_results(relative_path: &str, fallback: &[String]) -> Vec<String> {
    let path = default_content_dir().join(relative_path);
    let results: Vec<String> = match std::fs::read_to_string(&path) {
        Ok(text) => match serde_yaml::from_str::<JsonValue>(&text) {
            Ok(JsonValue::Object(map)) => map
                .get("entries")
                .and_then(|v| v.as_array())
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|entry| entry.get("result"))
                        .map(value_to_text)
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        },
        Err(_) => Vec::new(),
    };
    if results.is_empty() {
        fallback.to_vec()
    } else {
        results
    }
}

/// Render a JSON value as Python `str()` would (for `result` coercion).
fn value_to_text(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        JsonValue::Bool(true) => "True".to_string(),
        JsonValue::Bool(false) => "False".to_string(),
        JsonValue::Null => "None".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_returns_fallback() {
        let fallback = vec!["a".to_string(), "b".to_string()];
        let results = load_table_results("definitely/missing/table.yaml", &fallback);
        assert_eq!(results, fallback);
    }

    #[test]
    fn value_to_text_matches_python_str() {
        assert_eq!(value_to_text(&JsonValue::from("hi")), "hi");
        assert_eq!(value_to_text(&JsonValue::from(3)), "3");
        assert_eq!(value_to_text(&JsonValue::Bool(true)), "True");
    }
}
