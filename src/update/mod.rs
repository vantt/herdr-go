//! Pure version-compare logic for `herdr-go update` (D2). No network calls —
//! fetching the latest release lives in the [`github`] submodule.

pub mod github;

/// A plain `major.minor.patch` version, ordered lexicographically by field.
/// Real `vantt/herdr-go` release tags carry no prerelease/build metadata, so
/// integer component comparison is sufficient (no semver crate needed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl SemVer {
    /// Parses a bare `X.Y.Z` string (no leading `v`, no suffix).
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some(SemVer {
            major,
            minor,
            patch,
        })
    }
}

/// Result of comparing the running binary's version against the latest
/// published release tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateStatus {
    /// Running version is already >= the latest tag — no download, no restart.
    UpToDate,
    /// The latest tag is a strictly higher semver than the running version.
    NewerAvailable(SemVer),
}

/// Extracts the semver prefix from a `herdr_go::VERSION`-shaped fingerprint,
/// e.g. `"0.1.2 (613e9bd-dirty, 2026-07-22T14:28:03+07:00)"` -> `0.1.2`.
/// The semver is the first whitespace-delimited token (`src/lib.rs:19-22`).
pub fn parse_running_version(fingerprint: &str) -> Option<SemVer> {
    let token = fingerprint.split_whitespace().next()?;
    SemVer::parse(token)
}

/// Extracts the semver from a GitHub release tag shaped `vX.Y.Z`.
pub fn parse_release_tag(tag: &str) -> Option<SemVer> {
    SemVer::parse(tag.strip_prefix('v').unwrap_or(tag))
}

/// Compares the running version fingerprint against the latest release tag.
/// Returns `None` if either string fails to parse. Per D2 and the business-
/// logic test matrix, equal or lower is never treated as an update.
pub fn compare(running_fingerprint: &str, latest_tag: &str) -> Option<UpdateStatus> {
    let running = parse_running_version(running_fingerprint)?;
    let latest = parse_release_tag(latest_tag)?;
    if latest > running {
        Some(UpdateStatus::NewerAvailable(latest))
    } else {
        Some(UpdateStatus::UpToDate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUNNING_FINGERPRINT: &str = "0.1.2 (613e9bd-dirty, 2026-07-22T14:28:03+07:00)";

    #[test]
    fn equal_tag_is_up_to_date() {
        assert_eq!(
            compare(RUNNING_FINGERPRINT, "v0.1.2"),
            Some(UpdateStatus::UpToDate)
        );
    }

    #[test]
    fn higher_patch_tag_is_newer_available() {
        assert_eq!(
            compare(RUNNING_FINGERPRINT, "v0.1.3"),
            Some(UpdateStatus::NewerAvailable(SemVer {
                major: 0,
                minor: 1,
                patch: 3
            }))
        );
    }

    #[test]
    fn lower_tag_never_reports_update() {
        assert_eq!(
            compare(RUNNING_FINGERPRINT, "v0.1.1"),
            Some(UpdateStatus::UpToDate)
        );
    }

    #[test]
    fn same_tag_again_never_reports_update() {
        assert_eq!(
            compare(RUNNING_FINGERPRINT, "v0.1.2"),
            Some(UpdateStatus::UpToDate)
        );
    }

    #[test]
    fn higher_minor_and_major_are_newer_available() {
        assert_eq!(
            compare("0.1.2 (sha, ts)", "v0.2.0"),
            Some(UpdateStatus::NewerAvailable(SemVer {
                major: 0,
                minor: 2,
                patch: 0
            }))
        );
        assert_eq!(
            compare("0.1.2 (sha, ts)", "v1.0.0"),
            Some(UpdateStatus::NewerAvailable(SemVer {
                major: 1,
                minor: 0,
                patch: 0
            }))
        );
    }

    #[test]
    fn malformed_input_fails_to_parse() {
        assert_eq!(parse_running_version(""), None);
        assert_eq!(parse_release_tag("not-a-version"), None);
        assert_eq!(compare(RUNNING_FINGERPRINT, "vX.Y.Z"), None);
    }
}
