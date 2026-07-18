//! Configuration — strict decoding, fail-closed allowlist, tokens env-only
//! (ported from airemote `internal/config`).
//!
//! Three refusals baked in:
//! - **Strict decoding** (`#[serde(deny_unknown_fields)]`): a mistyped key is a
//!   named startup error, never silently defaulted — "a mistyped security
//!   setting that falls back to a default is a setting the operator believes is
//!   in force and is not."
//! - **Empty allowlist fails closed**: zero allowed roots is a hard error, never
//!   "allow everything" (the service runs as an admin-capable account).
//! - **Tokens are never config fields**: the web session secret, GitHub token,
//!   and Telegram bot token are read only from the environment. Strict decoding
//!   turns any attempt to place one in the settings document into an error,
//!   because no field exists to receive it.

use std::net::SocketAddr;
use std::path::PathBuf;

use serde::Deserialize;

/// Fully validated runtime configuration. Construct via [`Config::load_str`]
/// (or [`Config::load_file`]); the raw deserialized form is never handed out.
#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub herdr_session: String,
    pub allowed_roots: Vec<PathBuf>,
    pub poll_interval_ms: u64,
    pub herdr_protocol: u32,
    pub static_dir: PathBuf,
    /// Path to herdr's Unix socket. Empty string means use the default
    /// (`~/.config/herdr/herdr.sock`).
    pub herdr_socket: String,
    /// Destination Telegram chat id for notifications. Not a secret (the bot
    /// token is); absent means notify stays on the null channel.
    pub telegram_chat_id: Option<String>,
}

/// Secrets, resolved separately from the environment — never from the config
/// document (airemote `bot-token-env-only`). Each is read by exactly one place.
#[derive(Debug, Clone, Default)]
pub struct Secrets {
    pub web_session_secret: Option<String>,
    pub github_token: Option<String>,
    pub telegram_bot_token: Option<String>,
}

impl Secrets {
    /// Read secrets from the process environment. Missing values stay `None`;
    /// callers that require one fail closed at point of use. Never logged.
    pub fn from_env() -> Self {
        Secrets {
            web_session_secret: non_empty_env("HERDCTL_WEB_SECRET"),
            github_token: non_empty_env("HERDCTL_GITHUB_TOKEN"),
            telegram_bot_token: non_empty_env("HERDCTL_TELEGRAM_TOKEN"),
        }
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.trim().is_empty())
}

/// The on-disk shape. `deny_unknown_fields` is the contract: an unexpected key
/// is a hard error. Note there is deliberately NO token field anywhere here.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(default = "default_bind")]
    bind_addr: String,
    herdr_session: String,
    #[serde(default)]
    allowed_roots: Vec<String>,
    #[serde(default = "default_poll_ms")]
    poll_interval_ms: u64,
    #[serde(default = "default_protocol")]
    herdr_protocol: u32,
    #[serde(default = "default_static_dir")]
    static_dir: String,
    #[serde(default)]
    herdr_socket: String,
    #[serde(default)]
    telegram_chat_id: Option<String>,
}

fn default_bind() -> String {
    // Loopback by default; operators expose it over the tailnet by binding the
    // tailscale interface address explicitly (decision bc4a65a4).
    "127.0.0.1:8787".to_string()
}
fn default_poll_ms() -> u64 {
    500
}
fn default_protocol() -> u32 {
    16
}
fn default_static_dir() -> String {
    "static".to_string()
}

/// Every reason a config document is rejected, collected and reported together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    Parse(String),
    EmptyAllowedRoots,
    NonAbsoluteRoot(String),
    BadBindAddr(String),
    Multiple(Vec<ConfigError>),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Parse(e) => write!(f, "config parse error: {e}"),
            ConfigError::EmptyAllowedRoots => write!(
                f,
                "allowed_roots is empty — refusing to grant the whole filesystem (fail-closed)"
            ),
            ConfigError::NonAbsoluteRoot(p) => write!(f, "allowed root is not absolute: {p}"),
            ConfigError::BadBindAddr(a) => {
                write!(f, "bind_addr is not a valid socket address: {a}")
            }
            ConfigError::Multiple(errs) => {
                writeln!(f, "{} configuration error(s):", errs.len())?;
                for e in errs {
                    writeln!(f, "  - {e}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// Parse and validate a TOML/JSON config string. Collects every validation
    /// failure and reports them together. Unknown keys are rejected by serde
    /// with the key named in the parse error.
    pub fn load_str(text: &str) -> Result<Config, ConfigError> {
        // JSON is the wire format here (serde_json is already a dependency);
        // the loader shape is format-agnostic and can gain TOML later.
        let raw: RawConfig =
            serde_json::from_str(text).map_err(|e| ConfigError::Parse(e.to_string()))?;

        let mut errors = Vec::new();

        let bind_addr = match raw.bind_addr.parse::<SocketAddr>() {
            Ok(a) => Some(a),
            Err(_) => {
                errors.push(ConfigError::BadBindAddr(raw.bind_addr.clone()));
                None
            }
        };

        if raw.allowed_roots.is_empty() {
            errors.push(ConfigError::EmptyAllowedRoots);
        }
        let mut roots = Vec::new();
        for r in &raw.allowed_roots {
            let p = PathBuf::from(r);
            if !p.is_absolute() {
                errors.push(ConfigError::NonAbsoluteRoot(r.clone()));
            } else {
                roots.push(p);
            }
        }

        if !errors.is_empty() {
            return Err(if errors.len() == 1 {
                errors.pop().unwrap()
            } else {
                ConfigError::Multiple(errors)
            });
        }

        Ok(Config {
            bind_addr: bind_addr.expect("validated above"),
            herdr_session: raw.herdr_session,
            allowed_roots: roots,
            poll_interval_ms: raw.poll_interval_ms,
            herdr_protocol: raw.herdr_protocol,
            static_dir: PathBuf::from(raw.static_dir),
            herdr_socket: raw.herdr_socket,
            telegram_chat_id: raw.telegram_chat_id,
        })
    }

    /// Load config from a file path.
    pub fn load_file(path: &std::path::Path) -> Result<Config, ConfigError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::Parse(format!("{}: {e}", path.display())))?;
        Config::load_str(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = r#"{ "herdr_session": "gateway", "allowed_roots": ["/home/op/projects"] }"#;

    #[test]
    fn valid_config_loads_with_defaults() {
        let c = Config::load_str(OK).unwrap();
        assert_eq!(c.herdr_session, "gateway");
        assert_eq!(c.poll_interval_ms, 500);
        assert_eq!(c.herdr_protocol, 16);
        assert_eq!(c.bind_addr.to_string(), "127.0.0.1:8787");
    }

    #[test]
    fn unknown_key_is_a_named_error() {
        let text = r#"{ "herdr_session": "g", "allowed_roots": ["/a"], "sneaky": 1 }"#;
        let err = Config::load_str(text).unwrap_err();
        match err {
            ConfigError::Parse(msg) => {
                assert!(msg.contains("sneaky"), "error names the key: {msg}")
            }
            other => panic!("expected parse error naming the key, got {other:?}"),
        }
    }

    #[test]
    fn empty_allowed_roots_fails_closed() {
        let text = r#"{ "herdr_session": "g", "allowed_roots": [] }"#;
        assert_eq!(
            Config::load_str(text).unwrap_err(),
            ConfigError::EmptyAllowedRoots
        );
    }

    #[test]
    fn missing_allowed_roots_fails_closed() {
        let text = r#"{ "herdr_session": "g" }"#;
        assert_eq!(
            Config::load_str(text).unwrap_err(),
            ConfigError::EmptyAllowedRoots
        );
    }

    #[test]
    fn non_absolute_root_rejected() {
        let text = r#"{ "herdr_session": "g", "allowed_roots": ["relative/dir"] }"#;
        assert!(matches!(
            Config::load_str(text).unwrap_err(),
            ConfigError::NonAbsoluteRoot(_)
        ));
    }

    #[test]
    fn token_field_is_rejected_no_such_field() {
        // Attempting to smuggle a token into the document is a strict-decoding error.
        let text = r#"{ "herdr_session": "g", "allowed_roots": ["/a"], "github_token": "ghp_x" }"#;
        let err = Config::load_str(text).unwrap_err();
        assert!(matches!(err, ConfigError::Parse(_)));
    }

    #[test]
    fn bad_bind_addr_rejected() {
        let text =
            r#"{ "herdr_session": "g", "allowed_roots": ["/a"], "bind_addr": "not-an-addr" }"#;
        assert!(matches!(
            Config::load_str(text).unwrap_err(),
            ConfigError::BadBindAddr(_)
        ));
    }

    #[test]
    fn multiple_errors_collected() {
        let text = r#"{ "herdr_session": "g", "allowed_roots": [], "bind_addr": "x" }"#;
        match Config::load_str(text).unwrap_err() {
            ConfigError::Multiple(errs) => assert_eq!(errs.len(), 2),
            other => panic!("expected multiple errors, got {other:?}"),
        }
    }

    #[test]
    fn secrets_read_from_env_only() {
        // With no env set the secret is absent (fail-closed at use, not here).
        std::env::remove_var("HERDCTL_GITHUB_TOKEN");
        assert!(Secrets::from_env().github_token.is_none());
    }
}
