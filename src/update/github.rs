//! GitHub Releases metadata fetch for `herdr-go update` (D1).
//!
//! The network call and the JSON parse are deliberately split: [`parse_tag_name`]
//! is a pure, synchronously-testable function over a response-body `&str`, and
//! [`fetch_latest_tag`] is a thin, intentionally-untested async wrapper around it.
//! This keeps version-awareness logic unit-testable with fixture strings, with no
//! network in tests. Asset-list / checksum parsing is out of scope here (EP3).

use serde::Deserialize;

use super::{asset, checksum, compare, UpdateStatus};

/// GitHub Releases REST endpoint for the latest published release of
/// `vantt/herdr-go` — the same `releases/latest` channel `install.sh` uses (D1).
const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/vantt/herdr-go/releases/latest";

/// Literal name of the single merged checksum manifest the release pipeline
/// (EP1) always publishes as one release asset. `update` fails closed (D10)
/// when a release has no asset with exactly this name.
const CHECKSUMS_ASSET_NAME: &str = "checksums.txt";

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
    #[error("no release asset published for this platform: {0}")]
    AssetNotFound(String),
    #[error("release published no checksums.txt asset — refusing to install unverified (D10 fail-closed)")]
    ChecksumsAssetMissing,
    #[error("checksums.txt has no checksum entry for asset {0:?} — refusing to install unverified (D10 fail-closed)")]
    ChecksumEntryMissing(String),
    #[error("checksum mismatch for asset {0:?} — refusing to install corrupted or tampered bytes (D8 fail-closed)")]
    ChecksumMismatch(String),
}

/// Minimal projection of the GitHub release payload — only the one field
/// `update` needs. `tag_name` is optional so a payload that omits it yields a
/// precise [`FetchError::MissingTag`] rather than a generic parse error; serde
/// ignores every other field, so added/renamed response fields don't break this.
#[derive(Deserialize)]
struct ReleasePayload {
    tag_name: Option<String>,
    /// Published release assets. Defaulted so a payload that omits the array
    /// (or predates asset publishing) still yields a clean tag parse rather
    /// than a deserialization error.
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
}

/// One published GitHub release asset. `name` is matched exactly against the
/// expected platform filename / the checksums.txt name; `browser_download_url`
/// is the direct download link. Both are `#[serde(default)]` so tag parsing
/// never breaks on an odd asset entry — a defaulted empty field simply fails
/// the download closed downstream rather than aborting the whole payload parse.
#[derive(Debug, Deserialize)]
pub struct ReleaseAsset {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub browser_download_url: String,
}

/// Returns the asset whose `name` matches `filename` exactly, or `None` when no
/// asset matches. Pure — the composition around it is what fails closed.
pub fn find_asset<'a>(assets: &'a [ReleaseAsset], filename: &str) -> Option<&'a ReleaseAsset> {
    assets.iter().find(|a| a.name == filename)
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

/// Fetches the raw latest-release response body. Deliberately untested — the
/// single network GET both [`fetch_latest_tag`] and [`download_and_verify`]
/// share. GitHub's REST API rejects requests without a `User-Agent`, so one is
/// always sent, and `error_for_status` ensures an HTTP error can never
/// masquerade as a valid body.
async fn get_release_body() -> Result<String, FetchError> {
    reqwest::Client::new()
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
        .map_err(|e| FetchError::Request(e.to_string()))
}

/// Thin async wrapper around [`parse_tag_name`]: fetches the latest release
/// payload and returns its tag. Deliberately untested — all logic worth testing
/// lives in the pure parser.
pub async fn fetch_latest_tag() -> Result<String, FetchError> {
    parse_tag_name(&get_release_body().await?)
}

/// Parses the assets array out of a raw release response body. Pure and
/// synchronous, mirroring [`parse_tag_name`]; malformed JSON yields a precise
/// [`FetchError::MalformedJson`] and never panics.
fn parse_assets(body: &str) -> Result<Vec<ReleaseAsset>, FetchError> {
    let payload: ReleasePayload =
        serde_json::from_str(body).map_err(|e| FetchError::MalformedJson(e.to_string()))?;
    Ok(payload.assets)
}

/// Resolves the two download URLs `download_and_verify` needs, failing closed
/// before any download happens:
/// - no asset matches `platform_filename` -> [`FetchError::AssetNotFound`];
/// - no `checksums.txt` asset is published -> [`FetchError::ChecksumsAssetMissing`]
///   (D10 — never warn-and-proceed).
///
/// Pure over an already-fetched asset list, so the fail-closed branches are
/// unit-testable with fixtures and no network.
fn resolve_asset_urls<'a>(
    assets: &'a [ReleaseAsset],
    platform_filename: &str,
) -> Result<(&'a str, &'a str), FetchError> {
    let platform = find_asset(assets, platform_filename)
        .ok_or_else(|| FetchError::AssetNotFound(platform_filename.to_string()))?;
    let checksums =
        find_asset(assets, CHECKSUMS_ASSET_NAME).ok_or(FetchError::ChecksumsAssetMissing)?;
    Ok((
        platform.browser_download_url.as_str(),
        checksums.browser_download_url.as_str(),
    ))
}

/// Verifies `platform_bytes` against the `checksums.txt` body, failing closed:
/// - no entry for `platform_filename` -> [`FetchError::ChecksumEntryMissing`]
///   (D10 — a published-but-incomplete manifest never proceeds unverified);
/// - entry present but the hash disagrees -> [`FetchError::ChecksumMismatch`]
///   (D8).
///
/// Pure over already-downloaded bytes + body, so every fail-closed branch is
/// unit-testable with fixtures and no network.
fn verify_checksum(
    platform_filename: &str,
    platform_bytes: &[u8],
    checksums_body: &str,
) -> Result<(), FetchError> {
    let checksums = checksum::parse_checksums(checksums_body);
    let expected = checksums
        .get(platform_filename)
        .ok_or_else(|| FetchError::ChecksumEntryMissing(platform_filename.to_string()))?;
    if checksum::checksum_matches(platform_bytes, expected) {
        Ok(())
    } else {
        Err(FetchError::ChecksumMismatch(platform_filename.to_string()))
    }
}

/// Downloads an asset by URL, returning its raw bytes. `error_for_status` runs
/// before the body is read so an HTTP error status can never be mistaken for
/// asset content (mirrors [`get_release_body`]).
async fn download_bytes(url: &str) -> Result<Vec<u8>, FetchError> {
    let bytes = reqwest::Client::new()
        .get(url)
        .header(reqwest::header::USER_AGENT, "herdr-go")
        .send()
        .await
        .map_err(|e| FetchError::Request(e.to_string()))?
        .error_for_status()
        .map_err(|e| FetchError::Request(e.to_string()))?
        .bytes()
        .await
        .map_err(|e| FetchError::Request(e.to_string()))?;
    Ok(bytes.to_vec())
}

/// Downloads an asset by URL as UTF-8 text (the checksums.txt manifest). Same
/// `error_for_status`-before-body discipline as [`download_bytes`].
async fn download_text(url: &str) -> Result<String, FetchError> {
    reqwest::Client::new()
        .get(url)
        .header(reqwest::header::USER_AGENT, "herdr-go")
        .send()
        .await
        .map_err(|e| FetchError::Request(e.to_string()))?
        .error_for_status()
        .map_err(|e| FetchError::Request(e.to_string()))?
        .text()
        .await
        .map_err(|e| FetchError::Request(e.to_string()))
}

/// Downloads the platform release asset and returns its bytes ONLY after the
/// checksum published in the release's `checksums.txt` verifies against them.
/// Every corruption/absence path fails closed with a specific error (D8/D10):
/// unsupported host, missing platform asset, missing `checksums.txt` asset,
/// `checksums.txt` present but without an entry for our asset, and checksum
/// mismatch — none of them ever return `Ok`.
///
/// CROSS-SLICE INVARIANT (EP5 — binary swap/restart/rollback): D8's "verify
/// BEFORE overwrite" guarantee holds only if EP5 sources the bytes it installs
/// EXCLUSIVELY from this function's `Ok(bytes)`. EP5 must never add a second,
/// separate raw-download path that bypasses this checksum gate — doing so
/// silently defeats the entire integrity guarantee of the feature.
pub async fn download_and_verify() -> Result<Vec<u8>, FetchError> {
    let platform_filename = asset::expected_asset_filename_for_this_host().ok_or_else(|| {
        FetchError::AssetNotFound(format!(
            "unsupported host {}/{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    })?;

    let assets = parse_assets(&get_release_body().await?)?;
    let (asset_url, checksums_url) = resolve_asset_urls(&assets, &platform_filename)?;

    let asset_bytes = download_bytes(asset_url).await?;
    let checksums_body = download_text(checksums_url).await?;

    verify_checksum(&platform_filename, &asset_bytes, &checksums_body)?;
    Ok(asset_bytes)
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
        assert!(matches!(
            parse_tag_name(body).unwrap_err(),
            FetchError::MissingTag
        ));
    }

    #[test]
    fn empty_tag_name_returns_clear_error() {
        let body = r#"{"tag_name": ""}"#;
        assert!(matches!(
            parse_tag_name(body).unwrap_err(),
            FetchError::MissingTag
        ));
    }

    const PLATFORM_ASSET: &str = "herdr-go-x86_64-unknown-linux-musl.tar.gz";

    fn asset(name: &str) -> ReleaseAsset {
        ReleaseAsset {
            name: name.to_string(),
            browser_download_url: format!("https://example.test/{name}"),
        }
    }

    /// A release with both the platform binary and the checksums.txt manifest.
    fn full_assets() -> Vec<ReleaseAsset> {
        vec![asset(PLATFORM_ASSET), asset(CHECKSUMS_ASSET_NAME)]
    }

    #[test]
    fn find_asset_matches_exact_name_and_returns_none_otherwise() {
        let assets = full_assets();
        assert_eq!(
            find_asset(&assets, PLATFORM_ASSET).map(|a| a.name.as_str()),
            Some(PLATFORM_ASSET)
        );
        assert!(find_asset(&assets, CHECKSUMS_ASSET_NAME).is_some());
        assert!(find_asset(&assets, "herdr-go-x86_64-unknown-linux-musl.tar").is_none());
        assert!(find_asset(&assets, "does-not-exist.zip").is_none());
    }

    #[test]
    fn parse_assets_reads_name_and_url_and_tolerates_missing_url() {
        let body = r#"{
            "tag_name": "v0.1.2",
            "assets": [
                {"name": "herdr-go-x86_64-unknown-linux-musl.tar.gz",
                 "browser_download_url": "https://example.test/bin.tar.gz"},
                {"name": "checksums.txt"}
            ]
        }"#;
        let assets = parse_assets(body).unwrap();
        assert_eq!(assets.len(), 2);
        let bin = find_asset(&assets, PLATFORM_ASSET).unwrap();
        assert_eq!(bin.browser_download_url, "https://example.test/bin.tar.gz");
        // The checksums entry omitted its URL — defaulted, not a parse failure.
        assert_eq!(
            find_asset(&assets, CHECKSUMS_ASSET_NAME)
                .unwrap()
                .browser_download_url,
            ""
        );
    }

    #[test]
    fn verify_succeeds_when_checksum_matches() {
        let bytes = b"the-real-verified-binary-bytes";
        let body = format!("{}  {}\n", checksum::sha256_hex(bytes), PLATFORM_ASSET);

        // Asset resolution finds both URLs (fail-closed guards pass).
        let assets = full_assets();
        let (asset_url, checksums_url) = resolve_asset_urls(&assets, PLATFORM_ASSET).unwrap();
        assert_eq!(asset_url, format!("https://example.test/{PLATFORM_ASSET}"));
        assert_eq!(checksums_url, "https://example.test/checksums.txt");

        // And the checksum verifies against the real bytes.
        assert!(verify_checksum(PLATFORM_ASSET, bytes, &body).is_ok());
    }

    #[test]
    fn fails_when_platform_asset_missing() {
        // Only checksums.txt is published — the platform binary is absent.
        let assets = vec![asset(CHECKSUMS_ASSET_NAME)];
        assert!(matches!(
            resolve_asset_urls(&assets, PLATFORM_ASSET).unwrap_err(),
            FetchError::AssetNotFound(name) if name == PLATFORM_ASSET
        ));
    }

    #[test]
    fn fails_closed_when_checksums_txt_asset_missing() {
        // The platform binary is present, but NO checksums.txt asset exists.
        // D10: refuse to proceed — never install unverified.
        let assets = vec![asset(PLATFORM_ASSET)];
        assert!(matches!(
            resolve_asset_urls(&assets, PLATFORM_ASSET).unwrap_err(),
            FetchError::ChecksumsAssetMissing
        ));
    }

    #[test]
    fn fails_closed_when_entry_missing_for_asset() {
        // checksums.txt exists and has entries, but none for our platform asset.
        // D10: still fail closed — a partial manifest never proceeds unverified.
        let body = "aaaa1111  some-other-file.tar.gz\nbbbb2222  yet-another.zip\n";
        assert!(matches!(
            verify_checksum(PLATFORM_ASSET, b"whatever", body).unwrap_err(),
            FetchError::ChecksumEntryMissing(name) if name == PLATFORM_ASSET
        ));
    }

    #[test]
    fn fails_when_checksum_mismatches() {
        // The manifest has an entry for our asset, but it's the hash of other
        // content — the downloaded bytes must be rejected (D8).
        let body = format!(
            "{}  {}\n",
            checksum::sha256_hex(b"different-content"),
            PLATFORM_ASSET
        );
        assert!(matches!(
            verify_checksum(PLATFORM_ASSET, b"the-real-verified-binary-bytes", &body).unwrap_err(),
            FetchError::ChecksumMismatch(name) if name == PLATFORM_ASSET
        ));
    }
}
