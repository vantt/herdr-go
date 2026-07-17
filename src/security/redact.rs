//! Secret redaction — one shared redactor in front of every outbound structured
//! message and every external command's failure output (ported from airemote
//! `internal/security/redact.go`).
//!
//! Idempotent by construction (its own placeholders match none of its patterns).
//! Deliberately tuned against over-redaction — the bare word "password" in prose
//! is untouched, only an assignment is — "redacting so aggressively that prose is
//! mangled makes operators turn it off." Exactly one implementation exists.
//!
//! NOTE: Tier 2 raw terminal streams are NOT redacted (PRD §7) — the user sees
//! the real screen; redacting ANSI is both meaningless and render-breaking. This
//! redactor is for Tier 1 structured output and error text only.

/// Redact secret-shaped substrings, replacing each with a labeled placeholder
/// naming the secret class. Idempotent: running it twice changes nothing.
pub fn redact(input: &str) -> String {
    let mut s = input.to_string();
    for rule in RULES.iter() {
        s = rule.apply(&s);
    }
    s
}

struct Rule {
    label: &'static str,
    kind: RuleKind,
}

enum RuleKind {
    /// `key = value` / `key: value` assignment where the key names a secret.
    Assignment(&'static [&'static str]),
    /// A token with a fixed prefix (GitHub PATs, etc.).
    Prefixed(&'static str),
    /// A bearer/authorization header value.
    BearerHeader,
    /// A `scheme://user:password@host` credential embedded in a URL.
    UrlUserInfo,
}

const RULES: &[Rule] = &[
    Rule {
        label: "CREDENTIAL",
        kind: RuleKind::Assignment(&[
            "password", "passwd", "secret", "token", "api_key", "apikey",
            "access_key", "private_key", "client_secret", "auth",
        ]),
    },
    Rule {
        label: "GITHUB_TOKEN",
        kind: RuleKind::Prefixed("ghp_"),
    },
    Rule {
        label: "GITHUB_TOKEN",
        kind: RuleKind::Prefixed("github_pat_"),
    },
    Rule {
        label: "BOT_TOKEN",
        kind: RuleKind::BearerHeader,
    },
    Rule {
        label: "URL_CREDENTIAL",
        kind: RuleKind::UrlUserInfo,
    },
];

impl Rule {
    fn placeholder(&self) -> String {
        format!("[REDACTED:{}]", self.label)
    }

    fn apply(&self, input: &str) -> String {
        match &self.kind {
            RuleKind::Assignment(keys) => redact_assignments(input, keys, &self.placeholder()),
            RuleKind::Prefixed(prefix) => redact_prefixed(input, prefix, &self.placeholder()),
            RuleKind::BearerHeader => redact_bearer(input, &self.placeholder()),
            RuleKind::UrlUserInfo => redact_url_userinfo(input, &self.placeholder()),
        }
    }
}

fn is_placeholder_at(s: &str, idx: usize) -> bool {
    s[idx..].starts_with("[REDACTED:")
}

/// Redact `key<sep>value` where key is one of `keys` (case-insensitive) and sep
/// is `=` or `:`, optionally quoted. The value runs to whitespace/quote/comma.
fn redact_assignments(input: &str, keys: &[&str], placeholder: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    'outer: while i < bytes.len() {
        for key in keys {
            if lower[i..].starts_with(key) {
                let after_key = i + key.len();
                // Only treat as an assignment if the key is a whole word (the
                // char before is not alphanumeric), so "passwords" in prose is
                // not matched as "password".
                let boundary_ok = i == 0
                    || !bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_';
                if !boundary_ok {
                    continue;
                }
                if let Some(sep_pos) = next_nonspace(bytes, after_key) {
                    if bytes[sep_pos] == b'=' || bytes[sep_pos] == b':' {
                        // Emit key + separator verbatim, then the placeholder,
                        // and skip the value.
                        let val_start = skip_spaces_and_quote(bytes, sep_pos + 1);
                        let val_end = value_end(bytes, val_start);
                        if val_end > val_start {
                            out.push_str(&input[i..val_start]);
                            out.push_str(placeholder);
                            // preserve a trailing quote if present
                            if val_end < bytes.len()
                                && (bytes[val_end] == b'"' || bytes[val_end] == b'\'')
                            {
                                out.push(bytes[val_end] as char);
                                i = val_end + 1;
                            } else {
                                i = val_end;
                            }
                            continue 'outer;
                        }
                    }
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn next_nonspace(bytes: &[u8], mut i: usize) -> Option<usize> {
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    if i < bytes.len() {
        Some(i)
    } else {
        None
    }
}

fn skip_spaces_and_quote(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i] == b' ' {
        i += 1;
    }
    if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
        i += 1;
    }
    i
}

fn value_end(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' ' || c == b'"' || c == b'\'' || c == b',' || c == b'\n' || c == b'\r' {
            break;
        }
        i += 1;
    }
    i
}

fn redact_prefixed(input: &str, prefix: &str, placeholder: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if !is_placeholder_at(input, i) && input[i..].starts_with(prefix) {
            let mut j = i + prefix.len();
            while j < bytes.len()
                && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_')
            {
                j += 1;
            }
            // Only redact if there is an actual token body after the prefix.
            if j > i + prefix.len() {
                out.push_str(placeholder);
                i = j;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn redact_bearer(input: &str, placeholder: &str) -> String {
    let needle = "bearer ";
    let lower = input.to_ascii_lowercase();
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if lower[i..].starts_with(needle) {
            let val_start = i + needle.len();
            let val_end = value_end(bytes, val_start);
            if val_end > val_start {
                out.push_str(&input[i..val_start]);
                out.push_str(placeholder);
                i = val_end;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn redact_url_userinfo(input: &str, placeholder: &str) -> String {
    // Match `scheme://user:pass@` and redact `user:pass`.
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(scheme_pos) = rest.find("://") {
        let after = scheme_pos + 3;
        // find the next '@' before a '/', ' ', or end
        let tail = &rest[after..];
        let at = tail.find('@');
        let slash = tail.find('/').unwrap_or(tail.len());
        let space = tail.find(' ').unwrap_or(tail.len());
        match at {
            Some(at_pos) if at_pos < slash && at_pos < space && tail[..at_pos].contains(':') => {
                out.push_str(&rest[..after]);
                out.push_str(placeholder);
                out.push('@');
                rest = &tail[at_pos + 1..];
            }
            _ => {
                out.push_str(&rest[..after]);
                rest = &rest[after..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_assignment_value_only() {
        let got = redact("password = hunter2 and more");
        assert_eq!(got, "password = [REDACTED:CREDENTIAL] and more");
    }

    #[test]
    fn leaves_prose_word_untouched() {
        // "password" as a bare word in prose (no assignment) is not mangled.
        let got = redact("please change your password soon");
        assert_eq!(got, "please change your password soon");
    }

    #[test]
    fn does_not_match_superstring_key() {
        let got = redact("passwords_count = 5");
        assert_eq!(got, "passwords_count = 5");
    }

    #[test]
    fn redacts_github_pat() {
        let got = redact("token is ghp_abcd1234EFGH here");
        assert_eq!(got, "token is [REDACTED:GITHUB_TOKEN] here");
    }

    #[test]
    fn redacts_bearer_header() {
        let got = redact("Authorization: Bearer 12345:AAbbcc");
        assert!(got.contains("[REDACTED:BOT_TOKEN]"));
        assert!(!got.contains("AAbbcc"));
    }

    #[test]
    fn redacts_url_embedded_credential() {
        let got = redact("clone https://user:s3cret@github.com/x/y.git");
        assert!(got.contains("[REDACTED:URL_CREDENTIAL]@github.com"));
        assert!(!got.contains("s3cret"));
    }

    #[test]
    fn plain_url_without_credential_untouched() {
        let got = redact("clone https://github.com/x/y.git");
        assert_eq!(got, "clone https://github.com/x/y.git");
    }

    #[test]
    fn is_idempotent() {
        let once = redact("password=hunter2 ghp_abcd1234 Bearer xyz:123");
        let twice = redact(&once);
        assert_eq!(once, twice);
    }
}
