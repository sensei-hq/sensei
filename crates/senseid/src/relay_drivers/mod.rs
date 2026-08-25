//! Relay P5.1 — the run-drive backend adapter layer.
//!
//! [`RunDriver`] is the run-drive/supervise seam extracted from P3's hardcoded
//! `claude -p` spawn so later chunks can add backends (P5.2 ACP, P5.3 fallback)
//! without touching the backend-agnostic pure outcome map / limit detection /
//! watchdog. P5.1 ships exactly one backend — [`ClaudeDriver`] — behind the
//! trait, with zero behavior change.

pub mod acp;
pub mod claude;
pub mod fallback;
pub mod trait_def;

pub use acp::{AcpObserveDriver, acp_update_to_segments};
pub use claude::ClaudeDriver;
pub use fallback::{CoarseStatus, FallbackDriver, coarse_status, coarse_to_run_status};
pub use trait_def::{DriveCapability, DriveStep, RunDriver};

use crate::tasks::handlers::advance_run::DriveConfig;

/// Select the run driver for a tick from the resolved [`DriveConfig`]. P5.1 has
/// exactly one backend, so this always returns a [`ClaudeDriver`] carrying
/// `cfg.agent_cmd` (so `SENSEI_RUN_AGENT_CMD` — and the drive-smoke stub — stay
/// honored). Returns `impl RunDriver` (static dispatch) rather than a boxed trait
/// object because there is only one backend today.
///
// FOLLOW-UP: dispatch on a driver id for ACP/fallback (P5.2/P5.3) — at that
// point this returns `Box<dyn RunDriver>` and matches on a backend selector.
pub fn driver_for(cfg: &DriveConfig) -> impl RunDriver {
    ClaudeDriver { cmd: cfg.agent_cmd.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::Duration;

    #[test]
    fn driver_for_returns_claude_backend() {
        // The factory returns the Claude backend (the only one in P5.1).
        let cfg = DriveConfig {
            enabled: true,
            agent_cmd: "claude".into(),
            timeout: Duration::from_secs(10),
        };
        let driver = driver_for(&cfg);
        assert_eq!(driver.id(), "claude");
        assert_eq!(driver.capability(), DriveCapability::Hooks);
    }

    #[tokio::test]
    async fn driver_for_carries_cfg_agent_cmd() {
        // The custom agent_cmd (as SENSEI_RUN_AGENT_CMD would set) must be
        // threaded into the spawned program so the drive-smoke stub stays
        // honored. `impl RunDriver` erases the concrete type, so we prove the
        // threading behaviorally: a stub cmd of `echo` is what actually runs.
        let cfg = DriveConfig {
            enabled: true,
            agent_cmd: "echo".into(),
            timeout: Duration::from_secs(10),
        };
        let driver = driver_for(&cfg);
        let out = driver
            .drive_step(DriveStep {
                cwd: Path::new("."),
                prompt: "carried-through-marker",
                timeout: Duration::from_secs(10),
            })
            .await
            .expect("the cfg.agent_cmd stub (echo) should run");
        assert_eq!(out.exit_code, Some(0));
        assert!(
            out.stdout.contains("carried-through-marker"),
            "the factory's driver must run cfg.agent_cmd; stdout was {:?}",
            out.stdout
        );
    }
}
