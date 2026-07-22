//! GitHub Releases metadata fetch for `herdr-go update` (D1).
//!
//! The network call and the JSON parse are deliberately split: [`parse_tag_name`]
//! is a pure, synchronously-testable function over a response-body `&str`, and
//! [`fetch_latest_tag`] is a thin, intentionally-untested async wrapper around it.
//! This keeps version-awareness logic unit-testable with fixture strings, with no
//! network in tests. Asset-list / checksum parsing is out of scope here (EP3).

use serde::Deserialize;

use super::{compare, UpdateStatus};

/// GitHub Releases REST endpoint for the latest published release of
/// `vantt/herdr-go` — the same `releases/latest` channel `install.sh` uses (D1).
const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/vantt/herdr-go/releases/latest";

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("release response was not valid JSON: {0}")]
    MalformedJson(String),
    #[error("release response had no tag_name field")]
    MissingTag,
    #[error("release tag {0:?} is not a parseable version")]
    UnparsableTag(String),
    #[error("release request failed: {0}")]
    Request(String),
}

/// Minimal projection of the GitHub release payload — only the one field
/// `update` needs. `tag_name` is optional so a payload that omits it yields a
/// precise [`FetchError::MissingTag`] rather than a generic parse error; serde
/// ignores every other field, so added/renamed response fields don't break this.
#[derive(Deserialize)]
struct ReleasePayload {
    tag_name: Option<String>,
}

/// Extracts `tag_name` from a raw GitHub release response body. Pure and
/// synchronous — returns a clear error on malformed JSON or a missing/empty
/// tag, and never panics.
pub fn parse_tag_name(body: &str) -> Result<String, FetchError> {
    let payload: ReleasePayload =
        serde_json::from_str(body).map_err(|e| FetchError::MalformedJson(e.to_string()))?;
    match payload.tag_name {
        Some(tag) if !tag.is_empty() => Ok(tag),
        _ => Err(FetchError::MissingTag),
    }
}

/// Thin async wrapper around [`parse_tag_name`]: fetches the latest release
/// payload and returns its tag. Deliberately untested — all logic worth testing
/// lives in the pure parser. GitHub's REST API rejects requests without a
/// `User-Agent`, so one is always sent.
pub async fn fetch_latest_tag() -> Result<String, FetchError> {
    let body = reqwest::Client::new()
        .get(LATEST_RELEASE_URL)
        .header(reqwest::header::USER_AGENT, "herdr-go")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| FetchError::Request(e.to_string()))?
        .error_for_status()
        .map_err(|e| FetchError::Request(e.to_string()))?
        .text()
        .await
        .map_err(|e| FetchError::Request(e.to_string()))?;
    parse_tag_name(&body)
}

/// Fetches the latest release tag and compares it against the running binary's
/// version fingerprint via the version-compare logic (D2), producing the overall
/// "is an update available" decision. A tag that doesn't parse as semver becomes
/// [`FetchError::UnparsableTag`]. Async and network-bound — the composition is
/// covered by the pure [`parse_tag_name`] tests plus `super`'s compare tests.
pub async fn check_for_update(running_fingerprint: &str) -> Result<UpdateStatus, FetchError> {
    let tag = fetch_latest_tag().await?;
    compare(running_fingerprint, &tag).ok_or(FetchError::UnparsableTag(tag))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A representative slice of a real GitHub `releases/latest` payload,
    /// including fields `update` never reads — proving serde ignores them.
    const REAL_RELEASE_BODY: &str = r#"{
        "url": "https://api.github.com/repos/vantt/herdr-go/releases/123",
        "id": 123,
        "tag_name": "v0.1.2",
        "name": "herdr-go 0.1.2",
        "draft": false,
        "prerelease": false,
        "assets": [
            {"name": "herdr-go-linux-x86_64.tar.gz", "size": 4096}
        ]
    }"#;

    #[test]
    fn parses_tag_name_from_real_payload() {
        assert_eq!(parse_tag_name(REAL_RELEASE_BODY).unwrap(), "v0.1.2");
    }

    #[test]
    fn malformed_json_returns_error_not_panic() {
        let err = parse_tag_name("{ not valid json ").unwrap_err();
        assert!(matches!(err, FetchError::MalformedJson(_)));
    }

    #[test]
    fn missing_tag_name_returns_clear_error() {
        let body = r#"{"id": 123, "name": "no tag here"}"#;
        assert!(matches!(parse_tag_name(body).unwrap_err(), FetchError::MissingTag));
    }

    #[test]
    fn empty_tag_name_returns_clear_error() {
        let body = r#"{"tag_name": ""}"#;
        assert!(matches!(parse_tag_name(body).unwrap_err(), FetchError::MissingTag));
    }
}
