# Build from source

Install stable Rust and Node.js 22, then:

```bash
git clone https://github.com/vantt/herdr-go
cd herdr-go
cd web && npm ci && npm run bundle && cd ..
cargo build --release
```

Run `target/release/herdctl`, or use `./dev-deploy.sh` on Linux for the development user service.
