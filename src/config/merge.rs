//! Additive-only config merge for `herdr-go update` (D5, D6): seed fields the
//! running config is missing from the new version's default, never touch a
//! field the user already has, never drop an orphaned field the new default
//! no longer knows about.

use serde_json::Value;

/// Every reason a merge is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeError {
    /// `existing_json` or `default_json` failed to parse, or parsed to
    /// something other than a JSON object.
    NotAnObject(String),
}

impl std::fmt::Display for MergeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergeError::NotAnObject(which) => {
                write!(f, "{which} is not a JSON object — refusing to merge")
            }
        }
    }
}

impl std::error::Error for MergeError {}

/// Merge `default_json`'s fields into `existing_json`, additive-only.
///
/// A key present in `default_json` but absent from `existing_json` is seeded
/// with the default's value. A key already present in `existing_json` is
/// never modified (D6 — user values are sacred), regardless of its value or
/// type. A key present only in `existing_json` (an orphaned field from an
/// older config shape) is preserved untouched (D6 — no rename-mapping in
/// v1). Returns the merged object serialized back to a pretty JSON string.
pub fn merge_missing_fields(existing_json: &str, default_json: &str) -> Result<String, MergeError> {
    let existing: Value = serde_json::from_str(existing_json)
        .map_err(|e| MergeError::NotAnObject(format!("existing_json ({e})")))?;
    let default: Value = serde_json::from_str(default_json)
        .map_err(|e| MergeError::NotAnObject(format!("default_json ({e})")))?;

    let mut existing_obj = existing
        .as_object()
        .cloned()
        .ok_or_else(|| MergeError::NotAnObject("existing_json".to_string()))?;
    let default_obj = default
        .as_object()
        .ok_or_else(|| MergeError::NotAnObject("default_json".to_string()))?;

    for (key, value) in default_obj {
        existing_obj
            .entry(key.clone())
            .or_insert_with(|| value.clone());
    }

    serde_json::to_string_pretty(&Value::Object(existing_obj))
        .map_err(|e| MergeError::NotAnObject(format!("serializing merged result ({e})")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_adds_missing_field_with_default_value() {
        let existing = r#"{"bind_addr": "0.0.0.0:8787"}"#;
        let default = r#"{"bind_addr": "0.0.0.0:9999", "poll_interval_ms": 500}"#;

        let merged = merge_missing_fields(existing, default).expect("merge should succeed");
        let value: Value = serde_json::from_str(&merged).expect("merged output is valid JSON");

        assert_eq!(value["poll_interval_ms"], 500);
    }

    #[test]
    fn merge_preserves_existing_user_value_unchanged() {
        let existing = r#"{"bind_addr": "127.0.0.1:1234"}"#;
        let default = r#"{"bind_addr": "0.0.0.0:8787"}"#;

        let merged = merge_missing_fields(existing, default).expect("merge should succeed");
        let value: Value = serde_json::from_str(&merged).expect("merged output is valid JSON");

        assert_eq!(value["bind_addr"], "127.0.0.1:1234");
    }

    #[test]
    fn merge_preserves_orphaned_field_not_in_default() {
        let existing = r#"{"legacy_field": "keep-me", "bind_addr": "0.0.0.0:8787"}"#;
        let default = r#"{"bind_addr": "0.0.0.0:8787"}"#;

        let merged = merge_missing_fields(existing, default).expect("merge should succeed");
        let value: Value = serde_json::from_str(&merged).expect("merged output is valid JSON");

        assert_eq!(value["legacy_field"], "keep-me");
    }

    #[test]
    fn merge_errors_when_existing_is_not_a_json_object() {
        let existing = r#"["not", "an", "object"]"#;
        let default = r#"{"bind_addr": "0.0.0.0:8787"}"#;

        let result = merge_missing_fields(existing, default);

        assert_eq!(
            result,
            Err(MergeError::NotAnObject("existing_json".to_string()))
        );
    }
}
