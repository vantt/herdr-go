# Develop from source

Install stable Rust and Node.js 22, then:

```bash
git clone https://github.com/vantt/herdr-go
cd herdr-go
cd web
npm ci
npm run bundle
cd ..
cargo build --release
```

Run `./target/release/herdr-go`, or use `./dev-deploy.sh` on Linux for the development user service.

## Version-bump discipline

`Cargo.toml`'s `version` is the single source of truth for the product version. A git hook enforces that functional changes (`src/`, `web/src/`, `scripts/`, `Cargo.toml`) bump it, and that it never moves backwards; CI re-checks the backwards case as a safety net (`scripts/check-version.sh`). One-time setup to enable the local hook:

```bash
git config core.hooksPath .githooks
```

Bypass for a specific commit with `SKIP_VERSION_CHECK=1 git commit ...` (visible in shell history, so bypasses stay traceable).

Bumping and releasing are separate: the hook bumps `Cargo.toml` on every functional commit, but nothing publishes a release until someone pushes a matching tag — so `main`'s version routinely sits ahead of the latest GitHub release between releases. `ci`'s `version` job prints that gap on every push to `main` (`scripts/check-version.sh drift`) as a heads-up, not a failure.

## Cutting a release

The only thing that actually publishes a release is pushing a tag matching `Cargo.toml`'s version — that triggers `.github/workflows/release.yml`, which builds every platform target and creates the GitHub Release. `scripts/release.sh` is the checked way to do that:

```bash
scripts/release.sh            # checks, confirms, tags, pushes
scripts/release.sh --dry-run  # checks and reports only, tags nothing
scripts/release.sh --yes      # skip the confirmation prompt
```

It refuses to tag unless: you're on `main`, the working tree is clean, local `main` matches `origin/main`, the version isn't already tagged, and `scripts/check-version.sh ci` passes. It also reports whether the `version`/`rust`/`web` CI jobs are green on the commit being released (advisory — it warns rather than blocks, since an unrelated known-broken job shouldn't hold a release hostage).

The manual fallback it replaces, if the script itself is unavailable: `git tag -a vX.Y.Z -m "herdr-go X.Y.Z" && git push origin vX.Y.Z`, from a clean `main` that matches `origin/main`.

## Windows

No published binary yet — see `docs/history/windows-support/` and `docs/history/windows-release-matrix/` for the Windows build and packaging that already exists in CI; only the end-user installer is missing.

## Intel Macs

No published binary yet (Apple Silicon only) — building from source above works the same way on Intel.
