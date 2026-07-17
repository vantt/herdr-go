//! herdctl binary entry point — the sole composition root (decision 4e3ef1a1).
//! Adapters are wired into ports here and nowhere else.

use std::sync::Arc;

use herdctl::config::{Config, Secrets};
use herdctl::herdr::cli::CliHerdr;
use herdctl::herdr::fake::FakeHerdr;
use herdctl::herdr::{HerdrControl, HerdrStream};
use herdctl::supervisor::{SpawnHerdr, Supervisor};
use herdctl::watcher::PollWatcher;
use herdctl::web::{router, AppState};

/// Parsed command line. Deliberately tiny — no arg-parsing dependency.
struct Args {
    config_path: Option<String>,
    demo: bool,
}

fn parse_args() -> Args {
    let mut config_path = None;
    let mut demo = false;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--config" | "-c" => config_path = it.next(),
            "--demo" => demo = true,
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => {
                eprintln!("unknown argument: {other}");
                print_help();
                std::process::exit(2);
            }
        }
    }
    Args { config_path, demo }
}

fn print_help() {
    println!(
        "herdctl {} — herdr-gateway\n\n\
         USAGE:\n  herdctl [--config <path>] [--demo]\n\n\
         OPTIONS:\n  \
         -c, --config <path>   Path to the JSON config (see config.example.json)\n  \
             --demo            Run against an in-memory fake herdr (no live herdr needed)\n  \
         -h, --help            Show this help\n\n\
         ENV (secrets, never in config):\n  \
         HERDCTL_WEB_SECRET      Web login token (required unless --demo)\n  \
         HERDCTL_GITHUB_TOKEN    GitHub token for provisioning (optional)\n  \
         HERDCTL_TELEGRAM_TOKEN  Telegram bot token for notify (optional)",
        herdctl::VERSION
    );
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "herdctl=info".into()),
        )
        .init();

    let args = parse_args();
    let mut secrets = Secrets::from_env();

    // Resolve config: an explicit file, else a built-in demo config, else error.
    let config = match (&args.config_path, args.demo) {
        (Some(path), _) => Config::load_file(std::path::Path::new(path))?,
        (None, true) => demo_config(),
        (None, false) => {
            anyhow::bail!(
                "no --config given and --demo not set. Try `herdctl --demo` or `herdctl --config config.json`."
            )
        }
    };

    // In demo mode, mint a throwaway web token if none is set, and say so loudly.
    if args.demo && secrets.web_session_secret.is_none() {
        secrets.web_session_secret = Some("demo".to_string());
        tracing::warn!("DEMO MODE: web login token is 'demo' — do not expose this beyond localhost");
        println!("\n  ⚡ DEMO MODE — open the URL below and log in with token: demo\n");
    }

    // Wire the herdr adapter behind the two ports.
    let (control, stream): (Arc<dyn HerdrControl>, Arc<dyn HerdrStream>) = if args.demo {
        let fake = Arc::new(FakeHerdr::new());
        (fake.clone(), fake)
    } else {
        let cli = Arc::new(CliHerdr::new(config.herdr_session.clone()));
        (cli.clone(), cli)
    };

    // Supervisor + watcher loops (only meaningful against a real herdr).
    if !args.demo {
        let sup = Supervisor::new(
            control.clone(),
            Arc::new(SpawnHerdr {
                binary: "herdr".into(),
                session: config.herdr_session.clone(),
            }),
            std::time::Duration::from_secs(5),
            std::time::Duration::from_secs(3),
        );
        tokio::spawn(sup.run(|health| {
            tracing::info!(?health, "herdr health transition");
        }));

        let watcher = PollWatcher::new(
            control.clone(),
            std::time::Duration::from_millis(config.poll_interval_ms),
        );
        tokio::spawn(watcher.run(|change| {
            tracing::info!(pane = %change.pane_id, status = change.status.as_str(), "agent status change");
        }));
    }

    let state = AppState::new(
        control,
        stream,
        secrets.web_session_secret.clone(),
        config.herdr_protocol,
    );
    let app = router(state, &config.static_dir);

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    let addr = listener.local_addr()?;
    tracing::info!(%addr, session = %config.herdr_session, "herdctl listening");
    println!("  herdctl {} listening on http://{}\n", herdctl::VERSION, addr);
    axum::serve(listener, app).await?;
    Ok(())
}

/// A self-contained config for `--demo` — loopback bind, a throwaway allowed
/// root, static served from ./static.
fn demo_config() -> Config {
    let json = format!(
        r#"{{ "bind_addr": "127.0.0.1:8787", "herdr_session": "demo",
             "allowed_roots": ["{}"], "static_dir": "static" }}"#,
        std::env::temp_dir().display()
    );
    Config::load_str(&json).expect("demo config is valid")
}
