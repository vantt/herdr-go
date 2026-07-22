//! Pure platform release-asset name selection (D1). No network calls —
//! mirrors `install.sh`'s OS/arch case statement so `update`'s download step
//! (a later cell) knows which asset to fetch for the running host.

/// Returns the expected release asset filename for the given `(os, arch)`
/// pair, mirroring `install.sh`'s target-triple mapping exactly. `None` means
/// no published asset for that combination (matching install.sh's own
/// unsupported-combination errors).
pub fn expected_asset_filename(os: &str, arch: &str) -> Option<String> {
    let target = match (os, arch) {
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        _ => return None,
    };
    let ext = if os == "windows" { "zip" } else { "tar.gz" };
    Some(format!("herdr-go-{target}.{ext}"))
}

/// Thin wrapper resolving the expected asset filename for the currently
/// running host. Deliberately untested here — see `expected_asset_filename`
/// for the unit-tested pure mapping.
pub fn expected_asset_filename_for_this_host() -> Option<String> {
    expected_asset_filename(std::env::consts::OS, std::env::consts::ARCH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_x86_64() {
        assert_eq!(
            expected_asset_filename("linux", "x86_64"),
            Some("herdr-go-x86_64-unknown-linux-musl.tar.gz".to_string())
        );
    }

    #[test]
    fn linux_aarch64() {
        assert_eq!(
            expected_asset_filename("linux", "aarch64"),
            Some("herdr-go-aarch64-unknown-linux-musl.tar.gz".to_string())
        );
    }

    #[test]
    fn macos_aarch64() {
        assert_eq!(
            expected_asset_filename("macos", "aarch64"),
            Some("herdr-go-aarch64-apple-darwin.tar.gz".to_string())
        );
    }

    #[test]
    fn windows_x86_64() {
        assert_eq!(
            expected_asset_filename("windows", "x86_64"),
            Some("herdr-go-x86_64-pc-windows-msvc.zip".to_string())
        );
    }

    #[test]
    fn unsupported_combinations_return_none() {
        assert_eq!(expected_asset_filename("macos", "x86_64"), None);
        assert_eq!(expected_asset_filename("linux", "arm"), None);
        assert_eq!(expected_asset_filename("freebsd", "x86_64"), None);
    }
}
