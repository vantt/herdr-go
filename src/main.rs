//! herdctl binary entry point — the sole composition root (decision 4e3ef1a1).
//! Adapters are wired into ports here and nowhere else.

use std::sync::Arc;

use herdctl::config::{Config, Secrets};
use herdctl::herdr::cli::CliHerdr;
use herdctl::herdr::fake::FakeHerdr;
use herdctl::herdr::{HerdrControl, HerdrStream};
use herdctl::notify::{NotifyService, Notifier, NullNotifier, TelegramNotifier};
use herdctl::store::{MemoryStore, SqliteStore, Store};
use herdctl::supervisor::{SpawnHerdr, Supervisor};
use herdctl::watcher::PollWatcher;
use herdctl::web::{router, AppState};

/// Parsed command line. Deliberately tiny — no arg-parsing dependency.
struct Args {
    config_path: Option<String>,
    demo: bool,
    bind: Option<String>,
}

fn parse_args() -> Args {
    let mut config_path = None;
    let mut demo = false;
    let mut bind = None;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--config" | "-c" => config_path = it.next(),
            "--demo" => demo = true,
            "--bind" | "-b" => bind = it.next(),
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
    Args {
        config_path,
        demo,
        bind,
    }
}

fn print_help() {
    println!(
        "herdctl {} — herdr-gateway\n\n\
         USAGE:\n  herdctl [--config <path>] [--demo] [--bind <addr>]\n\n\
         OPTIONS:\n  \
         -c, --config <path>   Path to the JSON config (see config.example.json)\n  \
             --demo            Run against an in-memory fake herdr (no live herdr needed)\n  \
         -b, --bind <addr>     Override the listen address, e.g. 0.0.0.0:8787 to reach it\n  \
                               from other devices on the LAN. Non-loopback binds print a\n  \
                               security notice (auth token is then the only boundary).\n  \
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
    let mut config = match (&args.config_path, args.demo) {
        (Some(path), _) => Config::load_file(std::path::Path::new(path))?,
        (None, true) => demo_config(),
        (None, false) => {
            anyhow::bail!(
                "no --config given and --demo not set. Try `herdctl --demo` or `herdctl --config config.json`."
            )
        }
    };

    // --bind overrides the listen address for any mode (typed error on a bad value).
    if let Some(b) = &args.bind {
        config.bind_addr = b
            .parse()
            .map_err(|_| anyhow::anyhow!("--bind is not a valid socket address: {b}"))?;
    }

    // Binding beyond loopback exposes the gateway on that interface — herdr's
    // socket has no auth, so the web token becomes the ONLY boundary. Say so.
    if !config.bind_addr.ip().is_loopback() {
        tracing::warn!(addr = %config.bind_addr, "binding to a non-loopback address");
        println!(
            "\n  ⚠ Listening on {} — reachable beyond this machine.\n    \
             herdr has no auth of its own, so the web login token is the only gate.\n    \
             Prefer a Tailscale/tailnet address, and put TLS (reverse proxy) in front\n    \
             if this is a shared LAN or the internet.\n",
            config.bind_addr
        );
    }

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

    // Durable store: sqlite next to the config in real runs, in-memory for demo.
    let store: Arc<dyn Store> = if args.demo {
        Arc::new(MemoryStore::new())
    } else {
        let path = config
            .static_dir
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("herdctl-state.sqlite");
        Arc::new(SqliteStore::open(&path)?)
    };

    // Notify channel: Telegram when a token + chat id are present, else null.
    let notifier: Arc<dyn Notifier> = match TelegramNotifier::new(
        secrets.telegram_bot_token.clone(),
        config.telegram_chat_id.clone(),
    ) {
        Some(t) => {
            tracing::info!("notify channel: telegram");
            Arc::new(t)
        }
        None => Arc::new(NullNotifier),
    };
    let notify = Arc::new(NotifyService::new(store.clone(), notifier));

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

        // Watcher feeds notify: a blocked/done change is recorded, then drained
        // at-least-once to the channel.
        let watcher = PollWatcher::new(
            control.clone(),
            std::time::Duration::from_millis(config.poll_interval_ms),
        );
        let notify_for_watch = notify.clone();
        tokio::spawn(async move {
            let notify = notify_for_watch;
            watcher
                .run_async(move |change| {
                    let notify = notify.clone();
                    async move {
                        tracing::info!(pane = %change.pane_id, status = change.status.as_str(), "agent status change");
                        if notify.record(&change).await {
                            notify.drain().await;
                        }
                    }
                })
                .await;
        });
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
