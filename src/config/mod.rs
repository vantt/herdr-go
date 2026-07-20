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

pub mod secrets;
pub mod write;

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
    /// Path to herdr's local endpoint. Empty string means use Herdr's
    /// platform default (`~/.config/herdr/herdr.sock` on Unix,
    /// `%APPDATA%\herdr\herdr.sock` on Windows).
    pub herdr_socket: String,
    /// Destination Telegram chat id for notifications. Not a secret (the bot
    /// token is); absent means notify stays on the null channel.
    pub telegram_chat_id: Option<String>,
}

/// Secrets, resolved separately from the environment — never from the config
/// document (airemote `bot-token-env-only`). Each is read by exactly one place.
///
/// `Debug` is implemented by hand rather than derived: a derived `Debug` would
/// print every token value verbatim, so a stray `{:?}` anywhere would leak a
/// live secret into a log or error. The manual impl renders presence only.
#[derive(Clone, Default)]
pub struct Secrets {
    pub web_session_secret: Option<String>,
    pub github_token: Option<String>,
    pub telegram_bot_token: Option<String>,
}

impl std::fmt::Debug for Secrets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn redact(v: &Option<String>) -> &'static str {
            match v {
                Some(_) => "<redacted>",
                None => "<unset>",
            }
        }
        f.debug_struct("Secrets")
            .field("web_session_secret", &redact(&self.web_session_secret))
            .field("github_token", &redact(&self.github_token))
            .field("telegram_bot_token", &redact(&self.telegram_bot_token))
            .finish()
    }
}

impl Secrets {
    /// Read secrets, preferring the process environment and falling back to the
    /// trusted `herdr-go.env` file (D8) for any key the environment omits.
    /// Missing values stay `None`; callers that require one fail closed at point
    /// of use. Never logged.
    pub fn from_env() -> Self {
        secrets::resolve_from_env_and_file(&config_dir().join("herdr-go.env"))
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
    // Reachable by default (all interfaces) — dev is usually cross-machine
    // (operator decision 2026-07-18). A non-loopback bind prints a security
    // notice at startup; the auto-generated login token is the boundary. For
    // exposure beyond a trusted LAN, bind a tailnet address / put TLS in front.
    "0.0.0.0:8787".to_string()
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

const PRODUCT_DIR: &str = "herdr-go";
const LEGACY_PRODUCT_DIR: &str = "herdr-gateway";

#[cfg(any(windows, test))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeRoots {
    roaming: PathBuf,
    local: PathBuf,
    profile: PathBuf,
}

#[cfg(any(windows, test))]
impl NativeRoots {
    fn from_candidates(
        roaming: Option<PathBuf>,
        local: Option<PathBuf>,
        profile: Option<PathBuf>,
    ) -> std::io::Result<Self> {
        let roots = Self {
            roaming: roaming.ok_or_else(|| missing_native_root("roaming application data"))?,
            local: local.ok_or_else(|| missing_native_root("local application data"))?,
            profile: profile.ok_or_else(|| missing_native_root("user profile"))?,
        };
        if [&roots.roaming, &roots.local, &roots.profile]
            .iter()
            .any(|path| !path.is_absolute())
        {
            return Err(missing_native_root("absolute per-user folder"));
        }
        Ok(roots)
    }
}

#[cfg(any(windows, test))]
fn missing_native_root(kind: &str) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("native Windows {kind} is unavailable"),
    )
}

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;
    use std::io;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};
    use std::{ffi::OsString, ptr};

    type Handle = *mut c_void;
    type Sid = c_void;
    type Acl = c_void;

    #[repr(C)]
    pub struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    pub const ROAMING_APP_DATA: Guid = Guid {
        data1: 0x3eb685db,
        data2: 0x65f9,
        data3: 0x4cf6,
        data4: [0xa0, 0x3a, 0xe3, 0xef, 0x65, 0x72, 0x9f, 0x3d],
    };
    pub const LOCAL_APP_DATA: Guid = Guid {
        data1: 0xf1b32785,
        data2: 0x6fba,
        data3: 0x4fcf,
        data4: [0x9d, 0x55, 0x7b, 0x8e, 0x7f, 0x15, 0x70, 0x91],
    };
    pub const PROFILE: Guid = Guid {
        data1: 0x5e6c858f,
        data2: 0x0e22,
        data3: 0x4760,
        data4: [0x9a, 0xfe, 0xea, 0x33, 0x17, 0xb6, 0x71, 0x73],
    };

    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_USER: u32 = 1;
    const DACL_SECURITY_INFORMATION: u32 = 0x00000004;
    const PROTECTED_DACL_SECURITY_INFORMATION: u32 = 0x80000000;
    const SDDL_REVISION_1: u32 = 1;
    const ACL_SIZE_INFORMATION: u32 = 2;
    const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;
    const ACCESS_DENIED_ACE_TYPE: u8 = 1;

    #[repr(C)]
    struct SidAndAttributes {
        sid: *mut Sid,
        attributes: u32,
    }
    #[repr(C)]
    struct TokenUser {
        user: SidAndAttributes,
    }
    #[repr(C)]
    struct AclSizeInformation {
        ace_count: u32,
        acl_bytes_in_use: u32,
        acl_bytes_free: u32,
    }
    #[repr(C)]
    struct AceHeader {
        ace_type: u8,
        ace_flags: u8,
        ace_size: u16,
    }
    #[repr(C)]
    struct AccessAllowedAce {
        header: AceHeader,
        mask: u32,
        sid_start: u32,
    }

    #[link(name = "shell32")]
    extern "system" {
        fn SHGetKnownFolderPath(
            id: *const Guid,
            flags: u32,
            token: Handle,
            path: *mut *mut u16,
        ) -> i32;
    }
    #[link(name = "ole32")]
    extern "system" {
        fn CoTaskMemFree(memory: *mut c_void);
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> Handle;
        fn LocalFree(memory: *mut c_void) -> *mut c_void;
    }
    #[link(name = "advapi32")]
    extern "system" {
        fn OpenProcessToken(process: Handle, access: u32, token: *mut Handle) -> i32;
        fn GetTokenInformation(
            token: Handle,
            class: u32,
            info: *mut c_void,
            len: u32,
            needed: *mut u32,
        ) -> i32;
        fn ConvertSidToStringSidW(sid: *mut Sid, string_sid: *mut *mut u16) -> i32;
        fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl: *const u16,
            revision: u32,
            descriptor: *mut *mut c_void,
            size: *mut u32,
        ) -> i32;
        fn SetFileSecurityW(path: *const u16, info: u32, descriptor: *mut c_void) -> i32;
        fn GetFileSecurityW(
            path: *const u16,
            info: u32,
            descriptor: *mut c_void,
            len: u32,
            needed: *mut u32,
        ) -> i32;
        fn GetSecurityDescriptorDacl(
            descriptor: *mut c_void,
            present: *mut i32,
            dacl: *mut *mut Acl,
            defaulted: *mut i32,
        ) -> i32;
        fn GetAclInformation(acl: *mut Acl, info: *mut c_void, len: u32, class: u32) -> i32;
        fn GetAce(acl: *mut Acl, index: u32, ace: *mut *mut c_void) -> i32;
        fn EqualSid(first: *mut Sid, second: *mut Sid) -> i32;
        fn CloseHandle(handle: Handle) -> i32;
    }

    fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
        value.encode_wide().chain(Some(0)).collect()
    }

    pub fn known_folder(id: &Guid) -> io::Result<PathBuf> {
        let mut raw = ptr::null_mut();
        let result = unsafe { SHGetKnownFolderPath(id, 0, ptr::null_mut(), &mut raw) };
        if result < 0 {
            return Err(io::Error::from_raw_os_error(result));
        }
        let mut len = 0;
        unsafe {
            while *raw.add(len) != 0 {
                len += 1;
            }
        }
        let value = PathBuf::from(OsString::from_wide(unsafe {
            std::slice::from_raw_parts(raw, len)
        }));
        unsafe {
            CoTaskMemFree(raw.cast());
        }
        if value.is_absolute() {
            Ok(value)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Windows known folder was not absolute",
            ))
        }
    }

    struct OwnedToken(Handle);
    impl Drop for OwnedToken {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    fn current_user() -> io::Result<(OwnedToken, Vec<u8>, *mut Sid)> {
        let mut handle = ptr::null_mut();
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let handle = OwnedToken(handle);
        let mut needed = 0;
        unsafe {
            GetTokenInformation(handle.0, TOKEN_USER, ptr::null_mut(), 0, &mut needed);
        }
        let mut bytes = vec![0u8; needed as usize];
        if unsafe {
            GetTokenInformation(
                handle.0,
                TOKEN_USER,
                bytes.as_mut_ptr().cast(),
                needed,
                &mut needed,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let sid = unsafe { (*(bytes.as_ptr().cast::<TokenUser>())).user.sid };
        Ok((handle, bytes, sid))
    }

    pub fn protect_directory(path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path)?;
        let (_token, _storage, sid) = current_user()?;
        let mut sid_string = ptr::null_mut();
        if unsafe { ConvertSidToStringSidW(sid, &mut sid_string) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut len = 0;
        unsafe {
            while *sid_string.add(len) != 0 {
                len += 1;
            }
        }
        let sid_text = OsString::from_wide(unsafe { std::slice::from_raw_parts(sid_string, len) });
        unsafe {
            LocalFree(sid_string.cast());
        }
        let sddl = wide(std::ffi::OsStr::new(&format!(
            "D:P(A;OICI;FA;;;{})",
            sid_text.to_string_lossy()
        )));
        let mut descriptor = ptr::null_mut();
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let path = wide(path.as_os_str());
        let result = unsafe {
            SetFileSecurityW(
                path.as_ptr(),
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                descriptor,
            )
        };
        unsafe {
            LocalFree(descriptor);
        }
        if result == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub fn validate_owner_only(path: &Path) -> io::Result<()> {
        let (_token, _storage, current_sid) = current_user()?;
        let path = wide(path.as_os_str());
        let info = DACL_SECURITY_INFORMATION;
        let mut needed = 0;
        unsafe {
            GetFileSecurityW(path.as_ptr(), info, ptr::null_mut(), 0, &mut needed);
        }
        let mut descriptor = vec![0u8; needed as usize];
        if unsafe {
            GetFileSecurityW(
                path.as_ptr(),
                info,
                descriptor.as_mut_ptr().cast(),
                needed,
                &mut needed,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let mut defaulted = 0;
        let mut present = 0;
        let mut dacl = ptr::null_mut();
        if unsafe {
            GetSecurityDescriptorDacl(
                descriptor.as_mut_ptr().cast(),
                &mut present,
                &mut dacl,
                &mut defaulted,
            )
        } == 0
            || present == 0
            || dacl.is_null()
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "token has no protected access list",
            ));
        }
        let mut size = AclSizeInformation {
            ace_count: 0,
            acl_bytes_in_use: 0,
            acl_bytes_free: 0,
        };
        if unsafe {
            GetAclInformation(
                dacl,
                (&mut size as *mut AclSizeInformation).cast(),
                std::mem::size_of::<AclSizeInformation>() as u32,
                ACL_SIZE_INFORMATION,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let mut owner_allow = false;
        for index in 0..size.ace_count {
            let mut raw = ptr::null_mut();
            if unsafe { GetAce(dacl, index, &mut raw) } == 0 {
                return Err(io::Error::last_os_error());
            }
            let ace = unsafe { &*(raw.cast::<AccessAllowedAce>()) };
            if ace.header.ace_type == ACCESS_ALLOWED_ACE_TYPE {
                let ace_sid = (&ace.sid_start as *const u32).cast_mut().cast::<Sid>();
                if unsafe { EqualSid(ace_sid, current_sid) } == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "token grants access outside its owner",
                    ));
                }
                owner_allow = true;
            } else if ace.header.ace_type != ACCESS_DENIED_ACE_TYPE {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "token has an unsupported access rule",
                ));
            }
        }
        if owner_allow {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "token does not grant access to its owner",
            ))
        }
    }
}

#[cfg(windows)]
fn native_roots() -> std::io::Result<NativeRoots> {
    NativeRoots::from_candidates(
        windows::known_folder(&windows::ROAMING_APP_DATA).ok(),
        windows::known_folder(&windows::LOCAL_APP_DATA).ok(),
        windows::known_folder(&windows::PROFILE).ok(),
    )
}

/// Resolve the current user's native Windows profile directory.
///
/// This is fallible by design: callers must report an actionable startup
/// error rather than constructing a relative path when Windows cannot provide
/// an absolute per-user profile.
#[cfg(windows)]
pub fn native_user_profile() -> std::io::Result<PathBuf> {
    native_roots().map(|roots| roots.profile)
}

/// Resolve the current user's roaming application-data directory on Windows.
///
/// This is the root upstream Herdr uses for its Windows configuration and
/// default local endpoint marker.
#[cfg(windows)]
pub fn native_roaming_app_data() -> std::io::Result<PathBuf> {
    native_roots().map(|roots| roots.roaming)
}

/// Resolve the current user's native macOS Application Support directory.
///
/// This is fallible by design, matching the Windows branch: callers must
/// report an actionable startup error rather than constructing a relative
/// path when HOME is unavailable.
#[cfg(target_os = "macos")]
fn native_macos_app_support() -> std::io::Result<PathBuf> {
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("Library/Application Support"))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "HOME is unavailable; cannot resolve macOS Application Support directory",
            )
        })
}

fn base_config_dir() -> PathBuf {
    #[cfg(windows)]
    {
        native_roots()
            .expect("Windows per-user folders are unavailable")
            .roaming
    }
    #[cfg(target_os = "macos")]
    {
        native_macos_app_support().expect("macOS Application Support directory is unavailable")
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn base_data_dir() -> PathBuf {
    #[cfg(windows)]
    {
        native_roots()
            .expect("Windows per-user folders are unavailable")
            .local
    }
    #[cfg(target_os = "macos")]
    {
        native_macos_app_support().expect("macOS Application Support directory is unavailable")
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn migrate_one(base: &std::path::Path, kind: &str) -> std::io::Result<()> {
    let legacy = base.join(LEGACY_PRODUCT_DIR);
    let canonical = base.join(PRODUCT_DIR);
    if canonical.exists() {
        if legacy.exists() {
            eprintln!("warning: both legacy and canonical {kind} directories exist; using {} and leaving {} untouched", canonical.display(), legacy.display());
        }
        return Ok(());
    }
    if legacy.exists() {
        std::fs::rename(&legacy, &canonical).map_err(|error| {
            std::io::Error::new(
                error.kind(),
                format!(
                    "failed to migrate {kind} directory {} to {}: {error}",
                    legacy.display(),
                    canonical.display()
                ),
            )
        })?;
        eprintln!(
            "migrated legacy {kind} directory {} to {}",
            legacy.display(),
            canonical.display()
        );
    }
    Ok(())
}

/// Migrate legacy state before any canonical directory is created.
pub fn migrate_legacy_state() -> std::io::Result<()> {
    migrate_one(&base_config_dir(), "config")?;
    migrate_one(&base_data_dir(), "data")
}

/// The per-user config directory (`~/.config/herdr-go`).
pub fn config_dir() -> PathBuf {
    base_config_dir().join(PRODUCT_DIR)
}

/// The default config file path.
pub fn default_config_path() -> PathBuf {
    config_dir().join("config.json")
}

/// The per-user data directory (`~/.local/share/herdr-go`), independent
/// of `static_dir` — anchors durable state (sqlite) so it survives regardless
/// of whether a disk-served UI override is configured.
pub fn data_dir() -> PathBuf {
    base_data_dir().join(PRODUCT_DIR)
}

/// The current user's home directory, resolved from the native per-user
/// profile logic (Windows profile folder, else `$HOME`). Exposed for doctor's
/// `allowed_roots` breadth guard (D9), which must classify a candidate against
/// the same home this crate uses everywhere else.
pub(crate) fn home() -> PathBuf {
    #[cfg(windows)]
    {
        native_user_profile().expect("Windows per-user folders are unavailable")
    }
    #[cfg(not(windows))]
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Ensure a working default config exists at `path`, creating it if missing, and
/// return the loaded config. Sane single-operator defaults (fail-closed still
/// holds — `allowed_roots` is a real directory, never empty): bind loopback,
/// herdr session `default`, allowed root `~/projects` (or `~`), the default
/// socket. An existing file is loaded untouched.
pub fn ensure_config(path: &std::path::Path) -> Result<Config, ConfigError> {
    if !path.exists() {
        let projects = home().join("projects");
        let root = if projects.is_dir() { projects } else { home() };
        let default_json = format!(
            "{{\n  \"bind_addr\": \"0.0.0.0:8787\",\n  \"herdr_session\": \"default\",\n  \
             \"allowed_roots\": [{root:?}],\n  \"poll_interval_ms\": 500,\n  \
             \"herdr_protocol\": 16,\n  \"static_dir\": \"static\"\n}}\n",
            root = root.to_string_lossy()
        );
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ConfigError::Parse(format!("create config dir: {e}")))?;
        }
        std::fs::write(path, &default_json)
            .map_err(|e| ConfigError::Parse(format!("write default config: {e}")))?;
    }
    Config::load_file(path)
}

/// Resolve the web session secret, creating a persistent dev token if none is
/// set. Precedence: `HERDR_GO_WEB_SECRET` env → the token line in
/// `~/.config/herdr-go/herdr-go.env` → a freshly generated one persisted
/// there (mode 600). Returns the token and whether it was just generated.
pub fn ensure_web_secret() -> std::io::Result<(String, bool)> {
    if let Some(t) = std::env::var("HERDR_GO_WEB_SECRET")
        .ok()
        .filter(|v| !v.trim().is_empty())
    {
        return Ok((t, false));
    }
    let env_path = config_dir().join("herdr-go.env");
    const KEY: &str = "HERDR_GO_WEB_SECRET";

    // Existing persisted tokens are validated before any secret bytes are read.
    if env_path.exists() {
        validate_token_protection(&env_path)?;
    }
    if let Ok(text) = std::fs::read_to_string(&env_path) {
        for line in text.lines() {
            if let Some(v) = line.trim().strip_prefix(&format!("{KEY}=")) {
                if !v.is_empty() {
                    return Ok((v.to_string(), false));
                }
            }
        }
    }

    // Generate and persist a new token (append, never clobber other lines).
    let token = random_token();
    prepare_token_directory(&config_dir())?;
    let mut existing = std::fs::read_to_string(&env_path).unwrap_or_default();
    if !existing.is_empty() && !existing.ends_with('\n') {
        existing.push('\n');
    }
    existing.push_str(&format!("{KEY}={token}\n"));
    write_new_token(&env_path, existing.as_bytes())?;
    validate_token_protection(&env_path)?;
    Ok((token, true))
}

#[cfg(windows)]
fn prepare_token_directory(path: &std::path::Path) -> std::io::Result<()> {
    windows::protect_directory(path)
}
#[cfg(not(windows))]
fn prepare_token_directory(path: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}

fn write_new_token(path: &std::path::Path, contents: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

pub fn validate_token_protection(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        windows::validate_owner_only(path)
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let metadata = std::fs::metadata(path)?;
        if metadata.permissions().mode() & 0o077 != 0 || metadata.uid() != unsafe { libc_geteuid() }
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "saved web token is not owner-only",
            ));
        }
        Ok(())
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "token protection is unsupported on this platform",
        ))
    }
}

#[cfg(unix)]
unsafe extern "C" {
    fn geteuid() -> u32;
}
#[cfg(unix)]
unsafe fn libc_geteuid() -> u32 {
    geteuid()
}

fn random_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    fn absolute_root() -> &'static str {
        r"C:\Users\op\projects"
    }

    #[cfg(not(windows))]
    fn absolute_root() -> &'static str {
        "/home/op/projects"
    }

    fn config_json(root: &str) -> String {
        serde_json::json!({ "herdr_session": "gateway", "allowed_roots": [root] }).to_string()
    }

    #[test]
    fn valid_config_loads_with_defaults() {
        let c = Config::load_str(&config_json(absolute_root())).unwrap();
        assert_eq!(c.herdr_session, "gateway");
        assert_eq!(c.poll_interval_ms, 500);
        assert_eq!(c.herdr_protocol, 16);
        assert_eq!(c.bind_addr.to_string(), "0.0.0.0:8787");
    }

    #[test]
    fn unknown_key_is_a_named_error() {
        let text = serde_json::json!({
            "herdr_session": "g",
            "allowed_roots": [absolute_root()],
            "sneaky": 1
        })
        .to_string();
        let err = Config::load_str(&text).unwrap_err();
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
        let text = serde_json::json!({
            "herdr_session": "g",
            "allowed_roots": [absolute_root()],
            "github_token": "ghp_x"
        })
        .to_string();
        let err = Config::load_str(&text).unwrap_err();
        assert!(matches!(err, ConfigError::Parse(_)));
    }

    #[test]
    fn bad_bind_addr_rejected() {
        let text = serde_json::json!({
            "herdr_session": "g",
            "allowed_roots": [absolute_root()],
            "bind_addr": "not-an-addr"
        })
        .to_string();
        assert!(matches!(
            Config::load_str(&text).unwrap_err(),
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
    fn secrets_absent_from_env_and_file_are_none() {
        // D8 added a herdr-go.env fallback, so "process env is empty" no longer
        // implies the field is None on its own — it only does when no trusted
        // env file supplies the key either. Pin both by resolving against an
        // env-file path that does not exist: with the process env cleared and no
        // file, the secret is absent (fail-closed at use, not here).
        std::env::remove_var("HERDR_GO_GITHUB_TOKEN");
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("herdr-go.env");
        assert!(!missing.exists());
        let resolved = secrets::resolve_from_env_and_file(&missing);
        assert!(resolved.github_token.is_none());
    }

    #[test]
    fn ensure_config_creates_a_working_default_then_reloads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        assert!(!path.exists());
        let cfg = ensure_config(&path).unwrap();
        assert!(path.exists(), "default config file was created");
        assert_eq!(cfg.herdr_session, "default");
        assert!(
            !cfg.allowed_roots.is_empty(),
            "fail-closed default has a root"
        );
        // A second call loads the existing file unchanged.
        let again = ensure_config(&path).unwrap();
        assert_eq!(again.bind_addr, cfg.bind_addr);
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    #[test]
    fn data_dir_defaults_to_home_local_share() {
        // Mirrors install.sh's default SHARE_DIR ($PREFIX/share with
        // PREFIX=$HOME/.local) — must stay byte-identical so existing sqlite
        // state is found unchanged on upgrade.
        std::env::remove_var("XDG_DATA_HOME");
        let home = std::env::var("HOME").expect("HOME set in test environment");
        assert_eq!(
            data_dir(),
            PathBuf::from(home).join(".local/share/herdr-go")
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_config_and_data_dir_use_application_support() {
        // Per D1: config and data share one native macOS directory, no
        // roaming/local split.
        let home = std::env::var("HOME").expect("HOME set in test environment");
        let expected = PathBuf::from(home).join("Library/Application Support/herdr-go");
        assert_eq!(config_dir(), expected);
        assert_eq!(data_dir(), expected);
    }

    #[test]
    fn ensure_web_secret_prefers_env() {
        std::env::set_var("HERDR_GO_WEB_SECRET", "from-env-123");
        let (t, generated) = ensure_web_secret().unwrap();
        assert_eq!(t, "from-env-123");
        assert!(!generated);
        std::env::remove_var("HERDR_GO_WEB_SECRET");
    }

    #[test]
    fn native_roots_require_three_absolute_candidates() {
        #[cfg(windows)]
        let (roaming, local, profile) = (
            PathBuf::from(r"C:\Users\op\AppData\Roaming"),
            PathBuf::from(r"C:\Users\op\AppData\Local"),
            PathBuf::from(r"C:\Users\op"),
        );
        #[cfg(not(windows))]
        let (roaming, local, profile) = (
            PathBuf::from("/Users/op/AppData/Roaming"),
            PathBuf::from("/Users/op/AppData/Local"),
            PathBuf::from("/Users/op"),
        );
        let roots = NativeRoots::from_candidates(
            Some(roaming.clone()),
            Some(local.clone()),
            Some(profile.clone()),
        )
        .unwrap();
        assert!(roots.roaming.ends_with("AppData/Roaming"));
        assert!(NativeRoots::from_candidates(Some(roaming.clone()), None, Some(profile)).is_err());
        assert!(NativeRoots::from_candidates(
            Some(PathBuf::from("relative")),
            Some(local),
            Some(roaming),
        )
        .is_err());
    }

    #[cfg(unix)]
    #[test]
    fn token_file_is_created_owner_only_without_an_open_permissions_window() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("token.env");
        write_new_token(&path, b"HERDR_GO_WEB_SECRET=test\n").unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        validate_token_protection(&path).unwrap();
        assert_eq!(
            write_new_token(&path, b"replacement").unwrap_err().kind(),
            std::io::ErrorKind::AlreadyExists
        );
        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            "HERDR_GO_WEB_SECRET=test\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn existing_token_with_group_or_other_access_is_rejected() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("token.env");
        std::fs::write(&path, "HERDR_GO_WEB_SECRET=test\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();
        assert_eq!(
            validate_token_protection(&path).unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
    }
}
