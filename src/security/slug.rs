//! Slug sanitization — untrusted text (repo names, branch labels) becomes a
//! filesystem/branch-safe slug via an **allowlist** charset operating on raw
//! bytes, not decoded characters (ported from airemote `internal/security/slug.go`).
//!
//! Byte-level is the point: "a foreign character, a malformed byte, and an
//! over-long slash encoding all collapse the same way — no decoding step for an
//! attacker to disagree with." An empty result (nothing survived) is an **error**,
//! never a silent fallback to the raw input.

/// Returned when sanitization yields nothing usable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmptySlug;

impl std::fmt::Display for EmptySlug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "slug is empty after sanitization (no allowed bytes survived)")
    }
}

impl std::error::Error for EmptySlug {}

const MAX_SLUG_LEN: usize = 64;

/// Allowed slug bytes: lowercase ASCII letters, digits, hyphen. Everything else
/// is dropped. Runs of dropped bytes collapse to a single hyphen; leading and
/// trailing hyphens are trimmed. Uppercase is lowercased (the one transform).
pub fn sanitize_slug(input: &str) -> Result<String, EmptySlug> {
    let mut out = Vec::with_capacity(input.len().min(MAX_SLUG_LEN));
    let mut last_was_hyphen = false;
    for &b in input.as_bytes() {
        let mapped = match b {
            b'a'..=b'z' | b'0'..=b'9' => Some(b),
            b'A'..=b'Z' => Some(b.to_ascii_lowercase()),
            _ => None,
        };
        match mapped {
            Some(c) => {
                out.push(c);
                last_was_hyphen = false;
            }
            None => {
                // Collapse any run of disallowed bytes to a single hyphen, but
                // never lead with one and never emit two in a row.
                if !out.is_empty() && !last_was_hyphen {
                    out.push(b'-');
                    last_was_hyphen = true;
                }
            }
        }
        if out.len() >= MAX_SLUG_LEN {
            break;
        }
    }
    // Trim a trailing hyphen produced by the collapse rule.
    while out.last() == Some(&b'-') {
        out.pop();
    }
    if out.is_empty() {
        return Err(EmptySlug);
    }
    // out is ASCII by construction, so from_utf8 cannot fail.
    Ok(String::from_utf8(out).expect("slug bytes are ASCII"))
}

/// A branch name is accepted only if it *already equals* what the sanitizer
/// would produce — the last gate before a value leaves the process, independent
/// of any caller (airemote `validateBranch`). Without this, a task titled
/// `../../../.ssh` would be created verbatim by herdr and nothing would notice.
pub fn is_canonical_slug(candidate: &str) -> bool {
    match sanitize_slug(candidate) {
        Ok(s) => s == candidate,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_slug() {
        assert_eq!(sanitize_slug("Hello World").unwrap(), "hello-world");
    }

    #[test]
    fn collapses_runs_and_trims() {
        assert_eq!(sanitize_slug("  a // b  ").unwrap(), "a-b");
        assert_eq!(sanitize_slug("---a---").unwrap(), "a");
    }

    #[test]
    fn traversal_text_becomes_harmless() {
        // The dangerous shape collapses to plain hyphens — no `..`, no slash.
        let s = sanitize_slug("../../../.ssh").unwrap();
        assert_eq!(s, "ssh");
        assert!(!s.contains('/'));
        assert!(!s.contains('.'));
    }

    #[test]
    fn foreign_bytes_and_malformed_collapse() {
        assert_eq!(sanitize_slug("café—münchen").unwrap(), "caf-m-nchen");
    }

    #[test]
    fn empty_is_error_not_silent_passthrough() {
        assert_eq!(sanitize_slug(""), Err(EmptySlug));
        assert_eq!(sanitize_slug("////"), Err(EmptySlug));
        assert_eq!(sanitize_slug("   "), Err(EmptySlug));
    }

    #[test]
    fn length_capped() {
        let long = "a".repeat(200);
        assert_eq!(sanitize_slug(&long).unwrap().len(), MAX_SLUG_LEN);
    }

    #[test]
    fn is_canonical_only_for_already_clean() {
        assert!(is_canonical_slug("my-repo-1"));
        assert!(!is_canonical_slug("My Repo"));
        assert!(!is_canonical_slug("../evil"));
        assert!(!is_canonical_slug(""));
    }
}
