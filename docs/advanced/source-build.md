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

## Windows

No published binary yet — see `docs/history/windows-support/` and `docs/history/windows-release-matrix/` for the Windows build and packaging that already exists in CI; only the end-user installer is missing.

## Intel Macs

No published binary yet (Apple Silicon only) — building from source above works the same way on Intel.
