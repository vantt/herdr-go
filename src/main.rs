//! herdr-go binary entry point — the sole composition root (decision 4e3ef1a1).
//! Adapters are wired into ports here and nowhere else.

use std::sync::Arc;

use herdr_go::config::{Config, Secrets};
use herdr_go::herdr::fake::FakeHerdr;
use herdr_go::herdr::socket::{resolve_socket_path, SocketHerdr};
use herdr_go::herdr::Herdr;
use herdr_go::notify::{Notifier, NotifyService, NullNotifier, TelegramNotifier};
use herdr_go::store::{MemoryStore, SqliteStore, Store};
use herdr_go::supervisor::{herdr_binary_from_env, SpawnHerdr, Supervisor};
use herdr_go::watcher::PollWatcher;
use herdr_go::web::{router, AppState};

/// Parsed command line. Deliberately tiny — no arg-parsing dependency.
struct Args {
    config_path: Option<String>,
    demo: bool,
    bind: Option<String>,
    doctor: bool,
    /// `doctor --check`: diagnose only, never prompt or write, even on a TTY
    /// (D15). Takes precedence over TTY auto-detection.
    check: bool,
}

/// Run default-state migration only for the normal default-config path.
/// Keeping the decision and operation in one seam lets the mode matrix test
/// the exact branch called by `main` without changing process-global HOME/XDG.
fn migrate_default_state_if<F>(args: &Args, migrate: F) -> std::io::Result<()>
where
    F: FnOnce() -> std::io::Result<()>,
{
    if !args.doctor && !args.demo && args.config_path.is_none() {
        migrate()?;
    }
    Ok(())
}

fn resolve_persisted_secret_if_needed<F>(
    demo: bool,
    secrets: &mut Secrets,
    ensure: F,
) -> std::io::Result<bool>
where
    F: FnOnce() -> std::io::Result<(String, bool)>,
{
    if demo || secrets.web_session_secret.is_some() {
        return Ok(false);
    }
    let (token, generated) = ensure()?;
    secrets.web_session_secret = Some(token);
    Ok(generated)
}

fn parse_args() -> Args {
    let mut config_path = None;
    let mut demo = false;
    let mut bind = None;
    let mut doctor = false;
    let mut check = false;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "doctor" => doctor = true,
            "--check" => check = true,
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
        doctor,
        check,
    }
}

fn print_help() {
    println!(
        "herdr-go {} — herdr-go\n\n\
         USAGE:\n  herdr-go [--config <path>] [--demo] [--bind <addr>]\n\n\
         With no options, herdr-go auto-creates a working config +\n  \
         a persistent login token and runs against the local herdr.\n\n\
         COMMANDS:\n  \
         doctor                Check the environment; at a terminal, offer to fix each\n  \
         problem interactively. Add --check to diagnose only.\n\n\
         OPTIONS:\n  \
             --check           With `doctor`: diagnose only, never prompt or write\n  \
         -c, --config <path>   Path to the JSON config (default: ~/.config/herdr-go/config.json)\n  \
             --demo            Run against an in-memory fake herdr (no live herdr needed)\n  \
         -b, --bind <addr>     Override the listen address, e.g. 0.0.0.0:8787 to reach it\n  \
                               from other devices on the LAN. Non-loopback binds print a\n  \
                               security notice (auth token is then the only boundary).\n  \
         -h, --help            Show this help\n\n\
         ENV (secrets, never in config):\n  \
         HERDR_GO_WEB_SECRET      Web login token (required unless --demo)\n  \
         HERDR_GO_GITHUB_TOKEN    GitHub token for provisioning (optional)\n  \
         HERDR_GO_TELEGRAM_TOKEN  Telegram bot token for notify (optional)\n  \
         HERDR_GO_HERDR_BINARY    Herdr binary for supervisor restarts (optional)",
        herdr_go::VERSION
    );
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "herdr_go=info".into()),
        )
        .init();

    let args = parse_args();

    // Only normal default-config startup owns the default legacy directories.
    // Doctor, demo, and explicit-config runs must not move unrelated state.
    migrate_default_state_if(&args, herdr_go::config::migrate_legacy_state)?;

    // `herdr-go doctor` — diagnose the setup and exit. Interactive at a TTY
    // (offers guided fixes), read-only with `--check` or when not a terminal.
    if args.doctor {
        let ok = herdr_go::doctor::run(args.check).await;
        std::process::exit(if ok { 0 } else { 1 });
    }

    let mut secrets = Secrets::from_env();

    // Resolve config: an explicit file, a built-in demo config, or the
    // auto-created default (zero-config: `herdr-go` just runs against real herdr).
    let mut config = match (&args.config_path, args.demo) {
        (Some(path), _) => Config::load_file(std::path::Path::new(path))?,
        (None, true) => demo_config(),
        (None, false) => {
            let path = herdr_go::config::default_config_path();
            let created = !path.exists();
            let cfg = herdr_go::config::ensure_config(&path)?;
            if created {
                println!("  created default config → {}", path.display());
            }
            cfg
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
            "{}",
            herdr_go::doctor::non_loopback_bind_warning(&config.bind_addr)
        );
    }

    // In demo mode, mint a throwaway web token if none is set, and say so loudly.
    if args.demo && secrets.web_session_secret.is_none() {
        secrets.web_session_secret = Some("demo".to_string());
        tracing::warn!(
            "DEMO MODE: web login token is 'demo' — do not expose this beyond localhost"
        );
        println!("\n  ⚡ DEMO MODE — open the URL below and log in with token: demo\n");
    }

    // Non-demo: resolve (or create + persist) a durable web login token so a
    // plain `herdr-go` run is immediately usable without hand-setting a secret.
    if resolve_persisted_secret_if_needed(
        args.demo,
        &mut secrets,
        herdr_go::config::ensure_web_secret,
    )? {
        println!("  generated a protected web login token");
    }

    // Wire the herdr adapter: the real socket client, or the in-memory fake.
    let herdr: Arc<dyn Herdr> = if args.demo {
        Arc::new(FakeHerdr::new())
    } else {
        let sock = resolve_socket_path(&config.herdr_socket, &config.herdr_session)?;
        tracing::info!(session = %config.herdr_session, "herdr local endpoint client");
        Arc::new(SocketHerdr::new(sock))
    };

    // Durable store: sqlite under the per-user data dir in real runs, in-memory
    // for demo. Independent of static_dir (which is now an optional dev/disk
    // override, not always installed).
    let store: Arc<dyn Store> = if args.demo {
        Arc::new(MemoryStore::new())
    } else {
        let dir = herdr_go::config::data_dir();
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("herdr-go-state.sqlite");
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
            herdr.clone(),
            Arc::new(SpawnHerdr {
                binary: herdr_binary_from_env(),
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
            herdr.clone(),
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
        herdr,
        secrets.web_session_secret.clone(),
        config.herdr_protocol,
    )
    .with_agent_presets(config.agent_presets.clone());
    let app = router(state, &config.static_dir);

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    let addr = listener.local_addr()?;
    tracing::info!(%addr, session = %config.herdr_session, "herdr-go listening");
    println!(
        "  herdr-go {} listening on http://{}\n",
        herdr_go::VERSION,
        addr
    );
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

#[cfg(test)]
mod tests {
    use super::{migrate_default_state_if, resolve_persisted_secret_if_needed, Args};
    use herdr_go::config::Secrets;
    use std::cell::Cell;

    #[test]
    fn main_migration_seam_obeys_the_cli_mode_matrix() {
        struct Case {
            name: &'static str,
            config: Option<&'static str>,
            demo: bool,
            bind: Option<&'static str>,
            doctor: bool,
            migrates: bool,
        }

        let cases = [
            Case {
                name: "default",
                config: None,
                demo: false,
                bind: None,
                doctor: false,
                migrates: true,
            },
            Case {
                name: "bind-only",
                config: None,
                demo: false,
                bind: Some("127.0.0.1:9999"),
                doctor: false,
                migrates: true,
            },
            Case {
                name: "doctor",
                config: None,
                demo: false,
                bind: None,
                doctor: true,
                migrates: false,
            },
            Case {
                name: "doctor-with-bind",
                config: None,
                demo: false,
                bind: Some("127.0.0.1:9999"),
                doctor: true,
                migrates: false,
            },
            Case {
                name: "doctor-with-demo-and-config",
                config: Some("custom.json"),
                demo: true,
                bind: None,
                doctor: true,
                migrates: false,
            },
            Case {
                name: "demo",
                config: None,
                demo: true,
                bind: None,
                doctor: false,
                migrates: false,
            },
            Case {
                name: "demo-with-bind",
                config: None,
                demo: true,
                bind: Some("0.0.0.0:8787"),
                doctor: false,
                migrates: false,
            },
            Case {
                name: "explicit-config",
                config: Some("custom.json"),
                demo: false,
                bind: None,
                doctor: false,
                migrates: false,
            },
            Case {
                name: "explicit-config-with-demo",
                config: Some("custom.json"),
                demo: true,
                bind: None,
                doctor: false,
                migrates: false,
            },
        ];

        for case in cases {
            let called = Cell::new(false);
            let args = Args {
                config_path: case.config.map(str::to_owned),
                demo: case.demo,
                bind: case.bind.map(str::to_owned),
                doctor: case.doctor,
                check: false,
            };
            migrate_default_state_if(&args, || {
                called.set(true);
                Ok(())
            })
            .unwrap();
            assert_eq!(called.get(), case.migrates, "{}", case.name);
        }
    }

    #[test]
    fn token_protection_failure_aborts_startup_resolution() {
        let mut secrets = Secrets::default();
        let error = resolve_persisted_secret_if_needed(false, &mut secrets, || {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "unprotected token",
            ))
        })
        .unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(secrets.web_session_secret.is_none());
    }
}
