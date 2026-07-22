//! Pure checksum computation and checksums.txt parsing (D8/D10). No network
//! calls — downloading and composing this with the release fetch lives in a
//! later cell.

use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Computes the lowercase-hex SHA-256 digest of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

/// Parses sha256sum-format text (`<hex-hash>  <filename>` per line, as
/// produced by `scripts/generate-checksums.sh`) into a filename->hash map.
/// Lines that don't split into at least two whitespace-separated tokens are
/// skipped rather than treated as an error (defensive, not strict).
pub fn parse_checksums(body: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in body.lines() {
        let mut tokens = line.split_whitespace();
        let (Some(hash), Some(filename)) = (tokens.next(), tokens.next()) else {
            continue;
        };
        map.insert(filename.to_string(), hash.to_string());
    }
    map
}

/// Returns true if `sha256_hex(bytes)` matches `expected_hex`, case-insensitively.
pub fn checksum_matches(bytes: &[u8], expected_hex: &str) -> bool {
    sha256_hex(bytes).eq_ignore_ascii_case(expected_hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn parse_checksums_extracts_two_entries() {
        let body = "aaaa111  file-one.tar.gz\nbbbb222  file-two.zip\n";
        let map = parse_checksums(body);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("file-one.tar.gz"), Some(&"aaaa111".to_string()));
        assert_eq!(map.get("file-two.zip"), Some(&"bbbb222".to_string()));
    }

    #[test]
    fn checksum_matches_is_case_insensitive_and_rejects_wrong_hash() {
        let hash = sha256_hex(b"abc");
        assert!(checksum_matches(b"abc", &hash.to_uppercase()));
        assert!(checksum_matches(b"abc", &hash));
        assert!(!checksum_matches(
            b"abc",
            "0000000000000000000000000000000000000000000000000000000000000000"
        ));
    }
}
