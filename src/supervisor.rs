//! Supervisor — the gateway's watchdog over herdr (PRD §5.3/§9). NOT external
//! software (no supervisord): a tokio loop inside `herdctl` that pings herdr and
//! relaunches it when it is down. systemd watches the gateway; the gateway
//! watches herdr.
//!
//! Restarting herdr recovers workspace/tab/pane structure and relaunches agents
//! via native resume — it does **not** rescue an agent's in-flight work that
//! died with herdr (a herdr limitation, not the gateway's). The supervisor never
//! force-kills anything (agent-outlives-service).

use std::sync::Arc;
use std::time::Duration;

use crate::herdr::HerdrControl;

/// An action that (re)starts herdr. Injectable so the loop is testable without
/// spawning a real process.
#[async_trait::async_trait]
pub trait RestartAction: Send + Sync {
    async fn restart(&self) -> anyhow::Result<()>;
}

/// Production restart: spawn `herdr --session <name> server` headless.
pub struct SpawnHerdr {
    pub binary: String,
    pub session: String,
}

#[async_trait::async_trait]
impl RestartAction for SpawnHerdr {
    async fn restart(&self) -> anyhow::Result<()> {
        // Detached headless server; the supervisor re-pings to confirm recovery.
        tokio::process::Command::new(&self.binary)
            .arg("--session")
            .arg(&self.session)
            .arg("server")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        Ok(())
    }
}

/// Health of the supervised runtime, reported on each transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Up,
    Down,
}

pub struct Supervisor {
    control: Arc<dyn HerdrControl>,
    restart: Arc<dyn RestartAction>,
    interval: Duration,
    backoff: Duration,
}

impl Supervisor {
    pub fn new(
        control: Arc<dyn HerdrControl>,
        restart: Arc<dyn RestartAction>,
        interval: Duration,
        backoff: Duration,
    ) -> Self {
        Supervisor {
            control,
            restart,
            interval,
            backoff,
        }
    }

    /// One health check. When down, fire the restart action (after which the
    /// next check confirms recovery). Returns the observed health and whether a
    /// restart was attempted.
    pub async fn check_once(&self) -> (Health, bool) {
        match self.control.ping().await {
            Ok(_) => (Health::Up, false),
            Err(_) => {
                // Down — attempt one restart; backoff is applied by the loop.
                let _ = self.restart.restart().await;
                (Health::Down, true)
            }
        }
    }

    /// Run the supervision loop, invoking `on_transition` only when health
    /// flips (up→down or down→up), so callers can log/notify without spam.
    pub async fn run<F>(self, mut on_transition: F)
    where
        F: FnMut(Health) + Send,
    {
        let mut last: Option<Health> = None;
        loop {
            let (health, restarted) = self.check_once().await;
            if last != Some(health) {
                on_transition(health);
                last = Some(health);
            }
            let wait = if restarted { self.backoff } else { self.interval };
            tokio::time::sleep(wait).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::herdr::fake::FakeHerdr;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingRestart {
        count: Arc<AtomicUsize>,
    }
    #[async_trait::async_trait]
    impl RestartAction for CountingRestart {
        async fn restart(&self) -> anyhow::Result<()> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn up_herdr_does_not_restart() {
        let fake = Arc::new(FakeHerdr::new());
        let count = Arc::new(AtomicUsize::new(0));
        let sup = Supervisor::new(
            fake,
            Arc::new(CountingRestart { count: count.clone() }),
            Duration::from_millis(10),
            Duration::from_millis(10),
        );
        let (health, restarted) = sup.check_once().await;
        assert_eq!(health, Health::Up);
        assert!(!restarted);
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn down_herdr_triggers_restart() {
        let fake = Arc::new(FakeHerdr::new());
        fake.set_available(false);
        let count = Arc::new(AtomicUsize::new(0));
        let sup = Supervisor::new(
            fake.clone(),
            Arc::new(CountingRestart { count: count.clone() }),
            Duration::from_millis(10),
            Duration::from_millis(10),
        );
        let (health, restarted) = sup.check_once().await;
        assert_eq!(health, Health::Down);
        assert!(restarted);
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn recovery_after_restart_reports_up() {
        let fake = Arc::new(FakeHerdr::new());
        fake.set_available(false);
        let count = Arc::new(AtomicUsize::new(0));
        let sup = Supervisor::new(
            fake.clone(),
            Arc::new(CountingRestart { count: count.clone() }),
            Duration::from_millis(10),
            Duration::from_millis(10),
        );
        assert_eq!(sup.check_once().await.0, Health::Down);
        // Simulate the restart bringing herdr back up.
        fake.set_available(true);
        assert_eq!(sup.check_once().await.0, Health::Up);
    }
}
