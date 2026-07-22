//! Guarantees `static/` exists before `RustEmbed`'s derive macro scans it, so
//! `cargo build`/`test`/`clippy` never fail on a fresh checkout where
//! `npm run bundle` hasn't produced the web UI yet (`static/` is gitignored).
//!
//! Also computes the build fingerprint (git short sha + dirty state + local
//! build timestamp) consumed by `herdr_go::VERSION`. Deliberately emits no
//! rerun-scoping directive, leaving Cargo's default "always rerun" behavior
//! in place — required so the dirty flag and timestamp never go stale on a
//! rebuild that doesn't touch `.git/HEAD` (e.g. dirty-to-clean, or a rebuild
//! at the same commit).

use std::process::Command;

fn main() {
    std::fs::create_dir_all("static").expect("create static/ dir for embedding");

    let semver = std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION set by cargo");
    let sha_segment = git_sha_segment();
    let timestamp = build_timestamp();
    let fingerprint = format!("{semver} ({sha_segment}, {timestamp})");

    println!("cargo:rustc-env=HERDR_GO_FINGERPRINT={fingerprint}");
}

/// Short git sha, with a `-dirty` suffix when the working tree has
/// uncommitted changes (untracked files count as dirty, matching
/// `git status --porcelain`'s own behavior). Falls back to a placeholder
/// when git metadata is unavailable (no `.git`, `git` not on PATH) so the
/// build never fails over missing optional metadata.
fn git_sha_segment() -> String {
    let sha_output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();

    let sha = match sha_output {
        Ok(out) if out.status.success() => {
            let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if sha.is_empty() {
                return "nogit".to_string();
            }
            sha
        }
        _ => return "nogit".to_string(),
    };

    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map(|out| out.status.success() && !out.stdout.is_empty())
        .unwrap_or(false);

    if dirty {
        format!("{sha}-dirty")
    } else {
        sha
    }
}

/// Local build timestamp with UTC offset (D1/D4: build time, machine-local
/// timezone, not commit time or UTC). Falls back to UTC if the local offset
/// cannot be determined so the build never fails over it.
fn build_timestamp() -> String {
    let fmt = time::format_description::parse_borrowed::<2>(
        "[year]-[month]-[day]T[hour]:[minute]:[second][offset_hour sign:mandatory]:[offset_minute]",
    )
    .expect("format description parses");

    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());

    now.format(&fmt).expect("timestamp formats")
}
