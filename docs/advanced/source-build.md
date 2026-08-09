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

## Windows

No published binary yet — see `docs/history/windows-support/` and `docs/history/windows-release-matrix/` for the Windows build and packaging that already exists in CI; only the end-user installer is missing.

## Intel Macs

No published binary yet (Apple Silicon only) — building from source above works the same way on Intel.
