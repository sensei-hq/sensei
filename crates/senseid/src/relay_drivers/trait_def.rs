//! Relay P5.1 — the `RunDriver` backend seam.
//!
//! `RunDriver` is the **run-drive** contract (distinct from the config/capture
//! [`crate::assistants::Assistant`] trait, which detects an assistant and wires
//! sensei's MCP into its config). A `RunDriver` owns *how one drive step runs*
//! for a given backend — the one place P3's [`crate::tasks::handlers::advance_run`]
//! chooses a backend, extracted so P5.2 (ACP) / P5.3 (fallback) can add backends
//! without touching the pure outcome map / limit detection / watchdog (which
//! already read a backend-agnostic [`AgentOutput`]).
//!
//! **P5.1 keeps this MINIMAL** — just the step-execution seam. Gating, nudging,
//! and progress→segment observation are richer per-backend concerns owned by the
//! capability ladder (relay-engine §7); they land in later chunks and are noted
//! as `// FOLLOW-UP` here, not implemented now. For Claude specifically the gate
//! mechanism is out-of-band (the spawned `claude -p` carries the sensei plugin
//! whose blocking `PreToolUse` hook hits the daemon's `/hook/gate`), so the
//! driver does **not** own gating in P5.1.

use crate::agent_spawn::{AgentOutput, AgentSpawnError};
use std::future::Future;
use std::path::Path;
use std::time::Duration;

/// One drive step's inputs, borrowed for the duration of the call. The prompt is
/// already the backend-agnostic step text (P3 uses the run's goal); the driver
/// only decides *how* to run it on its backend.
pub struct DriveStep<'a> {
    /// The project working directory to run the step in (already resolved + an
    /// existing dir on disk — the caller never hands a driver a bogus cwd).
    pub cwd: &'a Path,
    /// The step prompt (logical text; never code/diffs — relay-engine D10).
    pub prompt: &'a str,
    /// Hard wall-clock cap for this step.
    pub timeout: Duration,
}

/// The feed richness a backend can offer (relay-engine §7 capability ladder).
/// Read by the (later) orchestrator to normalize each backend to the one
/// Execution→Segment→Item view; P5.1 only records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveCapability {
    /// Rich + passive: the assistant carries sensei's hooks, so gates fire
    /// out-of-band via `PreToolUse` (Claude Code today).
    Hooks,
    /// Rich + sensei-drives: the daemon hosts the run over ACP (Zed + adopters) — P5.2.
    Acp,
    /// Coarse: no hooks/ACP → running/idle/done + best-effort progress (aider /
    /// Codex / plain CLIs) — P5.3.
    Fallback,
}

/// The run-drive backend seam. One `RunDriver` per backend; P5.1 ships exactly
/// one (`ClaudeDriver`). Async is expressed via return-position `impl Future`
/// (the crate's house async-trait style — see [`crate::collective::anonymize`] /
/// [`crate::dojo::contribute`]), so no `async-trait` dep is needed and dispatch
/// stays static for the single backend that exists today.
pub trait RunDriver: Send + Sync {
    /// Stable backend id, e.g. `"claude"` (later `"acp"` / `"fallback:aider"`).
    fn id(&self) -> &str;

    /// This backend's feed richness (capability ladder above).
    fn capability(&self) -> DriveCapability;

    /// Run one drive step on this backend and return the captured output. A
    /// *drive* failure (bad exit, timeout) is NOT an error here — it rides in the
    /// returned [`AgentOutput`] (the caller's pure outcome map decides). Only a
    /// launch/reap failure is an [`AgentSpawnError`].
    fn drive_step(
        &self,
        step: DriveStep<'_>,
    ) -> impl Future<Output = Result<AgentOutput, AgentSpawnError>> + Send;

    // FOLLOW-UP (P5.2/P5.3): richer per-backend seam methods, added as the
    // capability ladder is built out — none are needed by P5.1's Claude backend
    // (its gate is out-of-band via the sensei-plugin PreToolUse hook):
    //   fn gate(&self, …)   -> impl Future<Output = GateDisposition> + Send;  // raise/await a gate for THIS backend
    //   fn nudge(&self, …)  -> impl Future<Output = Result<(), …>> + Send;     // steer (healthy) / unstick (stalled)
    //   fn observe(&self, …) -> DriveStatus;                                   // running/idle/done + progress
}
