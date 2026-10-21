//! Relay P5.1 — the Claude Code [`RunDriver`] backend.
//!
//! Extracts P3's inline `claude -p` spawn out of
//! [`crate::tasks::handlers::advance_run::drive_run`] behind the [`RunDriver`]
//! seam, byte-for-byte: the same `AgentCommand::new(cmd, ["-p", prompt], timeout)
//! .with_cwd(cwd)` fed to the same [`crate::agent_spawn::run_agent`] primitive.
//! Zero behavior change — this is purely the backend extraction.
//!
//! Capability = [`DriveCapability::Hooks`]: the spawned `claude -p` carries the
//! sensei plugin whose blocking `PreToolUse` hook hits the daemon's `/hook/gate`,
//! so gating is out-of-band and the driver does not own it (relay-engine §5/§7).

use super::trait_def::{DriveCapability, DriveStep, RunDriver};
use crate::agent_spawn::{AgentCommand, AgentOutput, AgentSpawnError, run_agent};

/// Drives a run by spawning `claude -p <prompt>` headless (the P3 mechanism).
pub struct ClaudeDriver {
    /// The program to exec — `"claude"` by default, overridable via
    /// `SENSEI_RUN_AGENT_CMD` (carried in through the `DriveConfig` factory) so
    /// the drive smoke can still substitute a harmless stub.
    pub cmd: String,
}

impl RunDriver for ClaudeDriver {
    fn id(&self) -> &str {
        "claude"
    }

    fn capability(&self) -> DriveCapability {
        DriveCapability::Hooks
    }

    async fn drive_step(&self, step: DriveStep<'_>) -> Result<AgentOutput, AgentSpawnError> {
        run_agent(&self.command(&step)).await
    }
}

impl ClaudeDriver {
    /// Byte-for-byte the command P3's `drive_run` built inline: the agent
    /// program, `-p <prompt>`, the step timeout, run in the project cwd — plus
    /// `SENSEI_RUN_ID`, so the spawned claude's gate hook reports the session as
    /// unattended (#312, D28). The spawned claude carries the sensei plugin, so
    /// its PreToolUse hook drives the existing `/hook/gate` — no gate wiring here.
    fn command(&self, step: &DriveStep<'_>) -> AgentCommand {
        AgentCommand::new(
            self.cmd.clone(),
            vec!["-p".to_string(), step.prompt.to_string()],
            step.timeout,
        )
        .with_cwd(step.cwd)
        .with_env("SENSEI_RUN_ID", step.run_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::Duration;

    #[test]
    fn a_driven_step_marks_the_spawned_claude_with_its_run() {
        // The spawned claude's gate hook reads SENSEI_RUN_ID to tell the daemon
        // the session is unattended (#312, D28).
        let d = ClaudeDriver { cmd: "claude".into() };
        let cmd = d.command(&DriveStep {
            cwd: Path::new("."),
            prompt: "ship it",
            run_id: "0b7d6f1e-run",
            timeout: Duration::from_secs(10),
        });
        assert!(
            cmd.env.contains(&("SENSEI_RUN_ID".to_string(), "0b7d6f1e-run".to_string())),
            "env was {:?}",
            cmd.env
        );
        assert_eq!(cmd.args, vec!["-p".to_string(), "ship it".to_string()]);
    }

    #[test]
    fn id_is_claude_and_capability_is_hooks() {
        let d = ClaudeDriver { cmd: "claude".into() };
        assert_eq!(d.id(), "claude");
        assert_eq!(d.capability(), DriveCapability::Hooks);
    }

    #[tokio::test]
    async fn drive_step_with_stub_cmd_captures_zero_exit() {
        // `echo -p <prompt>` exits 0 with the args on stdout — a harmless stub
        // that never touches real claude but exercises the full spawn path.
        let d = ClaudeDriver { cmd: "echo".into() };
        let out = d
            .drive_step(DriveStep {
                cwd: Path::new("."),
                prompt: "ship the relay engine",
                run_id: "r-1",
                timeout: Duration::from_secs(10),
            })
            .await
            .expect("echo should run");
        assert_eq!(out.exit_code, Some(0));
        assert!(!out.timed_out);
        // The prompt is passed through as an arg (proves the `-p <prompt>` shape).
        assert!(out.stdout.contains("ship the relay engine"), "stdout was {:?}", out.stdout);
    }

    #[tokio::test]
    async fn drive_step_with_bogus_cmd_is_spawn_error() {
        // A program that does not exist → the primitive's Spawn error path (a
        // launch failure is an AgentSpawnError, not an AgentOutput).
        let d = ClaudeDriver { cmd: "definitely_not_a_real_program_xyzzy_42".into() };
        let err = d
            .drive_step(DriveStep {
                cwd: Path::new("."),
                prompt: "would drive",
                run_id: "r-1",
                timeout: Duration::from_secs(10),
            })
            .await
            .expect_err("nonexistent program should error");
        assert!(matches!(err, AgentSpawnError::Spawn(_)), "expected Spawn error, got {err:?}");
    }
}
