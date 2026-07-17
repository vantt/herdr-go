//! herdctl binary entry point — the sole composition root (decision 4e3ef1a1).
//!
//! Wiring of adapters into ports happens here and nowhere else. The init slice
//! only proves the binary builds and runs; real wiring lands in slice S6.

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "herdctl=info".into()),
        )
        .init();

    tracing::info!(version = herdctl::VERSION, "herdctl starting");
    println!("herdctl {} — herdr-gateway (skeleton)", herdctl::VERSION);
    Ok(())
}
