//! Relay-engine (P3.2 + P3.3b) — advance one autonomous run by a single tick.
//!
//! The scheduler ([`crate::tasks::advance_run_scheduler`]) enqueues one
//! `AdvanceRun` task per active run each tick, carrying the run id in
//! `task.path`. This handler is the per-run tick body.
//!
//! **P3.2 scope:** the tick scaffolding + the lifecycle bits doable *without*
//! an agent — the liveness heartbeat and a housekeeping cadence event.
//!
//! **P3.3b scope (this chunk):** OFF-by-default agent drive. When
//! `SENSEI_RUN_DRIVE` is enabled the tick spawns `claude -p` headless (via the
//! P3.3a [`crate::agent_spawn`] primitive) to drive the run's next step in the
//! project's working directory, capturing the outcome as run_events. Disabled,
//! the handler keeps the exact P3.2 behavior (heartbeat + housekeeping only) so
//! the daemon never autonomously executes Claude until the owner opts in — the
//! same fail-safe posture as the feature-B hook gate.
//!
//! **Safety / zero-knowledge:** run_events carry only logical status (short
//! labels + exit code) — never stdout/stderr, diffs, or code (relay-engine
//! D10). The spawned `claude -p` runs with the sensei plugin, so its
//! `PreToolUse` hook fires and the existing `/hook/gate` handler applies gates;
//! no extra gate wiring is needed here.
//!
//! Resume is NOT handled here: the scheduler's `resume_due_runs` flips a
//! `paused` run whose `paused_until` has elapsed back to `running` (SQL-side)
//! before enqueueing this tick, so by the time a run reaches this handler a due
//! pause is already cleared. A still-`paused` run is a no-op tick.
//!
//! **Stall/crash recovery is NOT handled here either** (P3.6): the watchdog
//! ([`crate::tasks::watchdog_scheduler`]) exclusively owns `stalled` runs. This
//! handler treats a `stalled` run as a no-op and — critically — does **not**
//! heartbeat it, because a fresh heartbeat would mask the staleness the watchdog
//! escalates on (stalled → bounded recover → crashed). A clean drive step resets
//! the watchdog's bounded-recovery counter via `reset_run_recovery`.
//!
//! **P3.6 interaction — `stalled` runs are NOT touched here.** Stall detection
//! and bounded auto-recovery are owned exclusively by the watchdog
//! ([`crate::tasks::watchdog_scheduler`]), which measures staleness against
//! `heartbeat_at`. If this handler kept heartbeating a `stalled` run its
//! heartbeat would never go stale and the watchdog could never escalate it — so
//! a `stalled` run is a deliberate no-op tick here (see the status match below).

use super::super::Task;
use super::super::executor::TaskContext;
use crate::agent_spawn::AgentOutput;
use crate::relay_drivers::{DriveStep, RunDriver, driver_for};
use crate::run_limits::{LimitHit, detect_limit, resolve_paused_until};
use crate::runs::{Run, RunEventKind};
use std::time::Duration;

/// Hard wall-clock cap for one agent drive step. Long enough for a real
/// `claude -p` step to make progress, but bounded so a wedged agent can never
/// hang a run tick — on expiry the child is killed+reaped and the run is marked
/// `Stalled` for the watchdog (P3.6) to recover.
const DRIVE_TIMEOUT: Duration = Duration::from_secs(600); // 10 minutes

/// P3.4 limit-pause tuning. `LIMIT_JITTER_SECS` spreads the resume herd so many
/// paused runs don't all wake on the same instant (added to the parsed reset).
/// `LIMIT_DEFAULT_BACKOFF_MINS` is the capped backoff when a limit is detected
/// but its reset time is unparseable/ambiguous — the next tick re-checks
/// (relay-engine §5, "capped backoff + re-check").
const LIMIT_JITTER_SECS: i64 = 60;
const LIMIT_DEFAULT_BACKOFF_MINS: i64 = 30;

/// The intended outcome of one drive step, computed *purely* from the captured
/// [`AgentOutput`] (+ the wall clock for limit-reset math). Splitting the
/// decision out of the DB-writing [`drive_run`] makes the whole outcome map —
/// including the P3.4 limit-pause path — unit-testable without a database.
///
/// The DB-write wrapper ([`apply_outcome`]) turns each variant into the matching
/// run_event (+ status change). A limit **supersedes** the exit code: if the
/// output carries a limit message we pause, regardless of the code the CLI
/// exited with (Claude Code prints the limit as text and may exit 0 or non-zero).
#[derive(Debug, Clone, PartialEq)]
enum DriveOutcome {
    /// A rate/usage limit was detected in the output → pause the run until
    /// `paused_until` (RFC-3339), then let `resume_due_runs` auto-resume it.
    Limit { paused_until: String, reason: String, reset_at: Option<String> },
    /// The step finished cleanly (exit 0, no limit) → FeatureDone, stay Running.
    Done,
    /// The child hit the wall-clock timeout → Stalled for the watchdog (P3.6).
    Stalled { timeout_secs: u64 },
    /// A non-zero exit (no limit) → Flagged, stay Running (progress-over-asking).
    NonZeroExit { exit_code: Option<i32> },
}

/// Map a captured agent output to the intended [`DriveOutcome`]. Pure: no DB, no
/// env — `now`/`jitter`/`default_backoff`/`timeout_secs` are injected so the
/// limit-reset math is deterministic in tests. The **limit check comes first**: a
/// limit message supersedes the exit code (§5 — a limit is a pause, never a
/// failure/stop).
fn map_drive_outcome(
    out: &AgentOutput,
    now: chrono::DateTime<chrono::Utc>,
    jitter: chrono::Duration,
    default_backoff: chrono::Duration,
    timeout_secs: u64,
) -> DriveOutcome {
    // Limit detection scans combined stdout+stderr. This runs BEFORE the
    // exit-code mapping so a "you've hit your limit" line pauses the run even
    // when the CLI exits 0 (or non-zero) — the limit supersedes the code.
    let combined = format!("{}\n{}", out.stdout, out.stderr);
    if let Some(hit) = detect_limit(&combined, now) {
        let paused_until = resolve_paused_until(&hit, now, jitter, default_backoff);
        return DriveOutcome::Limit {
            paused_until: paused_until.to_rfc3339(),
            reason: hit.reason.clone(),
            reset_at: limit_reset_rfc3339(&hit),
        };
    }

    if out.timed_out {
        return DriveOutcome::Stalled { timeout_secs };
    }
    match out.exit_code {
        Some(0) => DriveOutcome::Done,
        code => DriveOutcome::NonZeroExit { exit_code: code },
    }
}

/// The parsed reset instant of a limit hit as RFC-3339, or `None` when the reset
/// time was unparseable (a capped-backoff pause). Logical detail for the event —
/// never raw output.
fn limit_reset_rfc3339(hit: &LimitHit) -> Option<String> {
    hit.reset_at.map(|dt| dt.to_rfc3339())
}

/// Resolved drive configuration for one tick. Parsed from the environment at
/// the handler boundary ([`DriveConfig::from_env`]) and passed into the pure-ish
/// [`drive_run`] so the core drive logic is unit-testable WITHOUT touching the
/// process-global environment (env is shared across the whole test binary).
#[derive(Debug, Clone)]
pub struct DriveConfig {
    /// Master switch. OFF by default: when false the tick never spawns an agent
    /// and behaves exactly like P3.2 (heartbeat + housekeeping).
    pub enabled: bool,
    /// The agent program to exec. Defaults to `"claude"`; tests override it with
    /// a harmless stub (`echo`, `sh`) so no real Claude is ever spawned.
    pub agent_cmd: String,
    /// Hard wall-clock cap for the spawned step.
    pub timeout: Duration,
}

impl DriveConfig {
    /// Build the config from the two env vars, matching the feature-B gate's
    /// fail-safe convention (a feature is OFF unless explicitly enabled):
    ///
    /// - `SENSEI_RUN_DRIVE` — `"1"`/`"true"`/`"yes"`/`"on"` (case-insensitive)
    ///   enables the drive; anything else (incl. unset) keeps it OFF.
    /// - `SENSEI_RUN_AGENT_CMD` — the program to exec, default `"claude"`.
    pub fn from_env() -> Self {
        let enabled =
            std::env::var("SENSEI_RUN_DRIVE").ok().map(|v| Self::is_truthy(&v)).unwrap_or(false);
        let agent_cmd = std::env::var("SENSEI_RUN_AGENT_CMD")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "claude".to_string());
        Self { enabled, agent_cmd, timeout: DRIVE_TIMEOUT }
    }

    fn is_truthy(v: &str) -> bool {
        matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
    }
}

/// Handler for `TaskKind::AdvanceRun`: advance one run by a tick. Returns `1`
/// when the run was ticked (heartbeat + housekeeping event), `0` for empty work
/// — an unknown/invalid run id, a run that no longer exists, a terminal run
/// (`Done`/`Failed`/`Crashed`), or a still-`paused` run.
pub async fn advance_run(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    // Run id rides in task.path. Empty / non-UUID → empty work, not an error.
    let run_id_str = task.path.as_str();
    if run_id_str.is_empty() {
        return Ok(0);
    }
    let Ok(run_id) = uuid::Uuid::parse_str(run_id_str) else {
        return Ok(0);
    };

    // A run that was completed/deleted between enqueue and dispatch is empty work.
    let Some(run) = ctx.pg().get_run(&run_id).await.map_err(|e| format!("get_run failed: {e}"))?
    else {
        return Ok(0);
    };

    use dojo_protocol::relay::RelayRunStatus;
    match run.status {
        // Terminal — no further progress. (Crashed is unexpected-death, also
        // terminal for the tick: recovery is a later, deliberate action.)
        RelayRunStatus::Done | RelayRunStatus::Failed | RelayRunStatus::Crashed => Ok(0),
        // Still paused: the scheduler's resume_due_runs flips a *due* pause to
        // running before this handler ever sees it, so a run that reaches here
        // still paused is not due yet — a no-op tick. The handler never resumes.
        RelayRunStatus::Paused => Ok(0),
        // Stalled: the watchdog (P3.6) EXCLUSIVELY owns stalled runs. This tick
        // is a deliberate no-op — critically, we must NOT heartbeat a stalled
        // run here, because a fresh heartbeat would make it look alive and mask
        // the staleness the watchdog escalates on (stalled → bounded recover →
        // crashed). The watchdog's own recover_run refreshes the heartbeat when
        // it flips a run back to running.
        RelayRunStatus::Stalled => Ok(0),
        // Blocked: waiting on a hard-block gate (a human reply). No autonomous
        // progress until the gate clears, so the tick is a no-op — but we still
        // want a live heartbeat so a blocked run isn't mistaken for a crash.
        // Fall through to the heartbeat path.
        RelayRunStatus::Running | RelayRunStatus::Blocked => {
            // Liveness: keep the heartbeat fresh so stall detection sees an
            // advancing (or at least alive) run.
            ctx.pg()
                .touch_run_heartbeat(&run_id)
                .await
                .map_err(|e| format!("touch_run_heartbeat failed: {e}"))?;

            // Cadence log: a lightweight housekeeping marker so the observability
            // API/console shows the run is being serviced each tick. Never a diff
            // or raw tool output (relay-engine D10) — just a tick marker.
            ctx.pg()
                .append_run_event(
                    &run_id,
                    RunEventKind::Housekeeping,
                    run.current_phase.as_deref(),
                    run.current_feature.as_deref(),
                    &serde_json::json!({ "tick": true }),
                )
                .await
                .map_err(|e| format!("append_run_event failed: {e}"))?;

            // P3.3b: OFF-by-default agent drive. Parse env → cfg at the handler
            // boundary, then hand the pure-ish drive logic the cfg + run so the
            // core is testable without touching process-global env. Only a
            // Running run drives — a Blocked run still only heartbeats (no
            // autonomous progress past a hard gate), and Stalled never reaches
            // this arm (the watchdog owns it).
            let cfg = DriveConfig::from_env();
            if cfg.enabled && run.status == RelayRunStatus::Running {
                drive_run(ctx, &cfg, &run).await?;
            }

            Ok(1)
        }
    }
}

/// The drive body (P3.3b MVP), split out of [`advance_run`] so it takes its
/// config as a param and is unit-testable without reading the process
/// environment. Assumes the caller has already: confirmed the run is
/// `Running`/`Stalled`, stamped the heartbeat, logged the housekeeping tick, and
/// checked `cfg.enabled`.
///
/// Single-shot for the MVP: it uses the run's `goal` as the prompt and spawns
/// one `claude -p` step. Full per-feature plan decomposition and the gated agent
/// loop are later chunks (P3.5/P3.7); the outcome→status mapping here is chosen
/// to compose cleanly with that loop (see per-branch comments).
///
/// Never returns a hard error for a *drive* failure — a bad exit or a timeout is
/// recorded as a run_event + status change and the tick still succeeds. Only a
/// DB write failure bubbles up (the caller maps it to a task error).
async fn drive_run(ctx: &TaskContext, cfg: &DriveConfig, run: &Run) -> Result<(), String> {
    let run_id = run.id;

    // 1. Resolve the working directory from the run's project. No project, no
    //    resolvable repo root, or a path that isn't a real dir on disk → we do
    //    NOT spawn (a cwd-less agent could run anywhere). Flag it for triage and
    //    treat the tick as done.
    let cwd = match resolve_cwd(ctx, run).await? {
        Some(dir) => dir,
        None => {
            return flag(ctx, run, "no resolvable project working directory; skipping drive").await;
        }
    };

    // 2. Prompt = the run's goal (single-shot MVP). Empty goal → nothing to
    //    drive; flag and stop rather than spawn a no-op agent.
    let goal = run.goal.as_deref().map(str::trim).unwrap_or("");
    if goal.is_empty() {
        return flag(ctx, run, "run has no goal; nothing to drive").await;
    }
    let prompt = goal.to_string();

    // 2b. Stance gate. An unattended headless single-shot cannot ask a human
    //     mid-run, so it must not launch when the run author's resolved autonomy
    //     requires asking before an *ordinary* step (`ask_always`). Finer
    //     per-step gating (risky/guarded) is enforced in-agent by the spawned
    //     claude's PreToolUse hook — this is only the launch decision. Resolved
    //     best-effort: an author/folder/stance lookup miss degrades to the
    //     fallback stance (`ask_on_guarded`, which permits the launch), because
    //     the drive already sits behind the OFF-by-default master switch.
    let stance = resolve_run_stance(ctx, run, &cwd).await;
    if !crate::stance::autonomy_permits(&stance.autonomy, crate::stance::StepRisk::Ordinary) {
        return flag(
            ctx,
            run,
            &format!(
                "author autonomy stance ({}) asks before each step; unattended drive not launched",
                stance.autonomy
            ),
        )
        .await;
    }

    // 3. Announce the step. `detail` stays logical/short — a bounded goal label
    //    plus the autonomy the drive is operating under (observability), never
    //    the full prompt body or any code.
    let label = short_label(goal);
    ctx.pg()
        .set_run_progress(&run_id, run.current_phase.as_deref(), Some(&label))
        .await
        .map_err(|e| format!("set_run_progress failed: {e}"))?;
    ctx.pg()
        .append_run_event(
            &run_id,
            RunEventKind::FeatureStarted,
            run.current_phase.as_deref(),
            Some(&label),
            &serde_json::json!({ "goal": label, "autonomy": stance.autonomy }),
        )
        .await
        .map_err(|e| format!("append_run_event(FeatureStarted) failed: {e}"))?;

    // 4. Select the run-drive backend (P5.1). The driver owns *how* the step
    //    runs for its backend; for Claude that's the same `<agent_cmd> -p
    //    <prompt>` spawn in the project cwd under a bounded timeout. The spawned
    //    claude carries the sensei plugin, so its PreToolUse hook drives the
    //    existing `/hook/gate` — no gate wiring here. `driver_for` honors
    //    `cfg.agent_cmd` (so `SENSEI_RUN_AGENT_CMD` + the drive-smoke stub work).
    let driver = driver_for(cfg);

    // Refresh liveness right before a potentially long spawn so the stall
    // detector doesn't trip mid-step.
    ctx.pg()
        .touch_run_heartbeat(&run_id)
        .await
        .map_err(|e| format!("touch_run_heartbeat failed: {e}"))?;

    // 5. Drive one step + supervise. The primitive never panics and always
    //    kills+reaps; a bad exit / timeout rides in the returned AgentOutput.
    let phase = run.current_phase.clone();
    let feature = Some(label.clone());
    let step = DriveStep { cwd: &cwd, prompt: &prompt, timeout: cfg.timeout };
    let out = match driver.drive_step(step).await {
        Ok(out) => out,
        Err(e) => {
            // Could not launch/reap the child (bad binary, EACCES, i/o). Not a
            // task error — record it logically and let the next tick retry.
            // (No AgentOutput → no limit message to inspect, so this stays a
            // plain Flag; it never routes through the outcome map.)
            tracing::warn!(run_id = %run_id, error = %e, "relay drive: agent spawn failed");
            return append(
                ctx,
                &run_id,
                RunEventKind::Flagged,
                phase.as_deref(),
                feature.as_deref(),
                serde_json::json!({ "note": "agent could not be started" }),
            )
            .await;
        }
    };

    // 6. Decide the outcome PURELY from the captured output (+ the clock for the
    //    limit-reset math), then apply it. The limit check lives inside
    //    `map_drive_outcome` and runs before the exit-code mapping, so a
    //    "you've hit your limit" line pauses the run regardless of exit code
    //    (P3.4 / relay-engine §5). The gateway-routed 429 path is separate — it
    //    is normalized in `gateway-embedded` (relay-engine §8), NOT here.
    let outcome = map_drive_outcome(
        &out,
        chrono::Utc::now(),
        chrono::Duration::seconds(LIMIT_JITTER_SECS),
        chrono::Duration::minutes(LIMIT_DEFAULT_BACKOFF_MINS),
        cfg.timeout.as_secs(),
    );
    apply_outcome(ctx, &run_id, phase.as_deref(), feature.as_deref(), &label, outcome).await
}

/// Turn a pure [`DriveOutcome`] into its run_event(s) + status change. This is
/// the single DB-writing side of the outcome map; the decision itself is made in
/// [`map_drive_outcome`] so it is testable without a database. Event details stay
/// logical/stripped (labels, codes, reset time) — never stdout/stderr or code
/// (relay-engine D10).
async fn apply_outcome(
    ctx: &TaskContext,
    run_id: &uuid::Uuid,
    phase: Option<&str>,
    feature: Option<&str>,
    label: &str,
    outcome: DriveOutcome,
) -> Result<(), String> {
    use dojo_protocol::relay::RelayRunStatus;
    match outcome {
        DriveOutcome::Limit { paused_until, reason, reset_at } => {
            // A limit is a PAUSE, never a stop (§5, degrade-not-stop). Flip the
            // run to Paused with paused_until = reset + jitter; the already-built
            // `resume_due_runs` scheduler auto-resumes it when the window elapses.
            // We deliberately do NOT also emit FeatureDone/Flagged — the limit
            // supersedes the exit code.
            tracing::info!(run_id = %run_id, %paused_until, "relay drive: paused on limit; will auto-resume");
            ctx.pg()
                .update_run_status(
                    run_id,
                    RelayRunStatus::Paused,
                    Some(&paused_until),
                    Some(&reason),
                )
                .await
                .map_err(|e| format!("update_run_status(Paused) failed: {e}"))?;
            // Logical detail only: reason + reset window. Never raw output/code.
            let mut detail = serde_json::json!({ "reason": reason, "paused_until": paused_until });
            if let Some(reset_at) = reset_at {
                detail["reset_at"] = serde_json::json!(reset_at);
            }
            append(ctx, run_id, RunEventKind::PausedOnLimit, phase, feature, detail).await
        }
        DriveOutcome::Stalled { timeout_secs } => {
            // Timeout → Stalled so the watchdog/P3.6 can recover it. The child
            // was already killed+reaped by the primitive.
            tracing::warn!(run_id = %run_id, "relay drive: agent step timed out");
            append(
                ctx,
                run_id,
                RunEventKind::Stalled,
                phase,
                feature,
                serde_json::json!({ "note": "agent step timed out", "timeout_secs": timeout_secs }),
            )
            .await?;
            ctx.pg()
                .update_run_status(run_id, RelayRunStatus::Stalled, None, None)
                .await
                .map_err(|e| format!("update_run_status(Stalled) failed: {e}"))
        }
        DriveOutcome::Done => {
            // Clean exit → the step finished. Emit FeatureDone and leave the run
            // Running: the next tick advances again. We deliberately do NOT
            // complete_run(Done) here — a real completion signal arrives with
            // the plan loop (P3.5/P3.7); a single-shot 0-exit only means "this
            // step is done", not "the whole plan is done".
            // Real progress resets the watchdog's bounded-recovery counter
            // (P3.6) so a long run that recovered from an earlier stall doesn't
            // carry those attempts forward and prematurely escalate to crashed.
            ctx.pg()
                .reset_run_recovery(run_id)
                .await
                .map_err(|e| format!("reset_run_recovery failed: {e}"))?;
            append(
                ctx,
                run_id,
                RunEventKind::FeatureDone,
                phase,
                feature,
                serde_json::json!({ "feature": label }),
            )
            .await
        }
        DriveOutcome::NonZeroExit { exit_code } => {
            // Non-zero exit. Per progress-over-asking we record it as Flagged
            // and keep the run Running (rather than hard-Failing) so the next
            // tick can retry / a human can inspect — the exit code is logical
            // status, and we never dump stdout/stderr into the event.
            tracing::warn!(run_id = %run_id, ?exit_code, "relay drive: agent step exited non-zero");
            append(
                ctx,
                run_id,
                RunEventKind::Flagged,
                phase,
                feature,
                serde_json::json!({ "note": "agent step exited non-zero", "exit_code": exit_code }),
            )
            .await
        }
    }
}

/// Resolve the run's cwd: `project_id` → project root path (pg_store) → an
/// existing directory on disk. `Ok(None)` for every "can't resolve" case (no
/// project, no repo root, path not a real dir) so the caller can Flag+skip
/// without spawning. Only a DB error bubbles up.
async fn resolve_cwd(ctx: &TaskContext, run: &Run) -> Result<Option<std::path::PathBuf>, String> {
    let Some(project_id) = run.project_id else {
        return Ok(None);
    };
    let Some(path) = ctx
        .pg()
        .project_root_path(&project_id)
        .await
        .map_err(|e| format!("project_root_path failed: {e}"))?
    else {
        return Ok(None);
    };
    let dir = std::path::PathBuf::from(&path);
    if dir.is_dir() { Ok(Some(dir)) } else { Ok(None) }
}

/// Emit a `Flagged` run_event with a short logical note (heartbeat is already
/// stamped by the caller) and return `Ok(())` — the "can't drive, don't spawn"
/// terminal for a tick. Status is left as-is (Flagged-but-running), matching
/// progress-over-asking.
/// Resolve the behavioural stance the run's author operates under: the git
/// author email (stamped on the run) keyed against the repo at `cwd` on the
/// scope ladder.
///
/// Fails CLOSED on an UNRESOLVED identity (issue #109 class): if the run has no
/// attributable author, or the stance lookup errors, we return the STRICTEST
/// stance ([`ResolvedStance::strict`], `ask_always`) — never the generic
/// `fallback()` (`ask_on_guarded`), which is more permissive than a cautious
/// user would choose and would let the drive gate proceed unattended for a run
/// we can't attribute. A *resolved* author who simply set no stance still gets
/// their `fallback()` default (that's a known user's absence-of-choice).
async fn resolve_run_stance(
    ctx: &TaskContext,
    run: &Run,
    cwd: &std::path::Path,
) -> crate::stance::ResolvedStance {
    let author_email =
        ctx.pg().run_author(&run.id).await.ok().and_then(|(_, email)| email).unwrap_or_default();
    // No attributable author → strict. (resolve_stance("") would match no row
    // and hand back the permissive fallback(), so short-circuit before that.)
    if author_email.is_empty() {
        return crate::stance::ResolvedStance::strict();
    }
    let folder_id = ctx
        .pg()
        .get_repo_by_path(&cwd.to_string_lossy())
        .await
        .ok()
        .flatten()
        .and_then(|f| crate::api::util::json_uuid(&f["id"]));
    ctx.pg()
        .resolve_stance(&author_email, folder_id.as_ref())
        .await
        .unwrap_or_else(|_| crate::stance::ResolvedStance::strict())
}

async fn flag(ctx: &TaskContext, run: &Run, note: &str) -> Result<(), String> {
    append(
        ctx,
        &run.id,
        RunEventKind::Flagged,
        run.current_phase.as_deref(),
        run.current_feature.as_deref(),
        serde_json::json!({ "note": note }),
    )
    .await
}

/// Thin wrapper over `append_run_event` with the handler's error-string mapping.
async fn append(
    ctx: &TaskContext,
    run_id: &uuid::Uuid,
    kind: RunEventKind,
    phase: Option<&str>,
    feature: Option<&str>,
    detail: serde_json::Value,
) -> Result<(), String> {
    ctx.pg()
        .append_run_event(run_id, kind, phase, feature, &detail)
        .await
        .map(|_| ())
        .map_err(|e| format!("append_run_event({kind:?}) failed: {e}"))
}

/// A short, single-line feature label derived from a (possibly long,
/// multi-line) goal — the first line, trimmed and capped. Keeps
/// `current_feature`/event detail logical and bounded (never the full prompt).
fn short_label(goal: &str) -> String {
    const MAX: usize = 80;
    let first = goal.lines().next().unwrap_or("").trim();
    if first.chars().count() <= MAX {
        first.to_string()
    } else {
        let mut s: String = first.chars().take(MAX - 1).collect();
        s.push('…');
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::state::SharedState;
    use crate::runs::NewRun;
    use crate::tasks::TaskKind;
    use crate::tasks::queue::TaskQueue;
    use dojo_protocol::relay::RelayRunStatus;
    use std::sync::Arc;

    async fn make_ctx() -> Option<Arc<TaskContext>> {
        let queue = Arc::new(TaskQueue::new());
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let pg = crate::db::pg_store::PgStore::connect_test().await.ok()?;
        let app_state = Arc::new(SharedState {
            task_queue: queue.clone(),
            pg,
            gateway,
            event_tx: {
                let (tx, _) = tokio::sync::broadcast::channel(16);
                tx
            },
            breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            provisioning: None,
        });
        Some(Arc::new(TaskContext {
            queue,
            app_state,
            _graph_path: None,
            logger: sensei_logger::Logger::noop(),
        }))
    }

    #[tokio::test]
    async fn empty_run_id_is_empty_work() {
        // No DB needed — the empty-path guard short-circuits before any query.
        let Some(ctx) = make_ctx().await else {
            return;
        };
        let task = Task::new(TaskKind::AdvanceRun, "", "");
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn non_uuid_run_id_is_empty_work() {
        let Some(ctx) = make_ctx().await else {
            return;
        };
        let task = Task::new(TaskKind::AdvanceRun, "", "not-a-uuid");
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn unknown_run_is_empty_work() {
        let Some(ctx) = make_ctx().await else {
            return;
        };
        let id = uuid::Uuid::new_v4().to_string();
        let task = Task::new(TaskKind::AdvanceRun, "", &id);
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn running_run_heartbeats_and_logs() {
        let Some(ctx) = make_ctx().await else {
            return;
        };
        let pg = ctx.pg();
        let id = pg.create_run(&NewRun::default()).await.unwrap(); // defaults to running

        let task = Task::new(TaskKind::AdvanceRun, "", &id.to_string());
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 1, "a running run is ticked");

        // Heartbeat was stamped.
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert!(run.heartbeat_at.as_deref().unwrap().contains('T'), "heartbeat set");

        // A housekeeping tick event was appended.
        let events = pg.list_run_events(&id, 10).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, RunEventKind::Housekeeping);
        assert_eq!(events[0].detail["tick"], serde_json::json!(true));

        pg_delete_run(pg, &id).await;
    }

    #[tokio::test]
    async fn paused_run_is_noop() {
        let Some(ctx) = make_ctx().await else {
            return;
        };
        let pg = ctx.pg();
        let id = pg.create_run(&NewRun::default()).await.unwrap();
        pg.update_run_status(
            &id,
            RelayRunStatus::Paused,
            Some("2999-01-01T00:00:00Z"),
            Some("cap"),
        )
        .await
        .unwrap();

        let task = Task::new(TaskKind::AdvanceRun, "", &id.to_string());
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 0, "a paused run is a no-op tick");

        // No heartbeat, no event.
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert!(run.heartbeat_at.is_none(), "paused run is not heartbeated");
        assert!(
            pg.list_run_events(&id, 10).await.unwrap().is_empty(),
            "no event for a paused tick"
        );

        pg_delete_run(pg, &id).await;
    }

    #[tokio::test]
    async fn terminal_run_is_noop() {
        let Some(ctx) = make_ctx().await else {
            return;
        };
        let pg = ctx.pg();
        let id = pg.create_run(&NewRun::default()).await.unwrap();
        pg.complete_run(&id, RelayRunStatus::Done).await.unwrap();

        let task = Task::new(TaskKind::AdvanceRun, "", &id.to_string());
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 0, "a done run is a no-op tick");
        assert!(pg.list_run_events(&id, 10).await.unwrap().is_empty());

        pg_delete_run(pg, &id).await;
    }

    #[tokio::test]
    async fn stalled_run_is_noop_watchdog_owns_it() {
        // A stalled run must be a NO-OP here: the watchdog (P3.6) exclusively
        // owns stalled runs. Critically, advance_run must NOT heartbeat a stalled
        // run — a fresh heartbeat would mask the staleness the watchdog escalates
        // on. So: returns 0, no new event, heartbeat unchanged.
        let Some(ctx) = make_ctx().await else {
            return;
        };
        let pg = ctx.pg();
        let id = pg.create_run(&NewRun::default()).await.unwrap();
        pg.update_run_status(&id, RelayRunStatus::Stalled, None, None).await.unwrap();

        let task = Task::new(TaskKind::AdvanceRun, "", &id.to_string());
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 0, "a stalled run is a no-op tick");

        // No heartbeat stamped (would mask staleness) and no event appended.
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert!(run.heartbeat_at.is_none(), "a stalled run must NOT be heartbeated by advance_run");
        assert!(
            pg.list_run_events(&id, 10).await.unwrap().is_empty(),
            "no event for a stalled no-op tick"
        );

        pg_delete_run(pg, &id).await;
    }

    async fn pg_delete_run(pg: &crate::db::pg_store::PgStore, id: &uuid::Uuid) {
        sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
            .bind(id)
            .execute(pg.pool())
            .await
            .unwrap();
    }

    // ── P3.3b drive ────────────────────────────────────────────────────

    // --- Pure config/label unit tests (no DB, no env) ---

    #[test]
    fn drive_config_is_truthy_for_common_true_values() {
        for v in ["1", "true", "TRUE", " yes ", "On"] {
            assert!(DriveConfig::is_truthy(v), "{v:?} should enable the drive");
        }
        for v in ["", "0", "false", "no", "off", "nope", "2"] {
            assert!(!DriveConfig::is_truthy(v), "{v:?} should NOT enable the drive");
        }
    }

    #[test]
    fn short_label_is_first_line_bounded() {
        assert_eq!(short_label("  ship the relay engine  "), "ship the relay engine");
        // Only the first line survives.
        assert_eq!(short_label("first line\nsecond line\nthird"), "first line");
        // Long single lines are capped with an ellipsis.
        let long = "x".repeat(200);
        let out = short_label(&long);
        assert!(out.chars().count() <= 80, "label not capped: {}", out.chars().count());
        assert!(out.ends_with('…'), "long label should be ellipsized");
        // Empty / whitespace-only goal → empty label.
        assert_eq!(short_label("   "), "");
    }

    // --- Pure outcome-map unit tests (no DB, no env, no spawn) ---
    //
    // These drive the P3.4 decision directly: a hand-built AgentOutput + a fixed
    // clock in, the intended DriveOutcome out. No database, no `claude`.

    use chrono::{Duration as ChronoDuration, TimeZone, Utc};

    fn fixed_now() -> chrono::DateTime<chrono::Utc> {
        Utc.with_ymd_and_hms(2026, 7, 17, 8, 0, 0).unwrap()
    }

    fn out_with(
        stdout: &str,
        stderr: &str,
        exit_code: Option<i32>,
        timed_out: bool,
    ) -> AgentOutput {
        AgentOutput { stdout: stdout.into(), stderr: stderr.into(), exit_code, timed_out }
    }

    fn map(out: &AgentOutput) -> DriveOutcome {
        map_drive_outcome(
            out,
            fixed_now(),
            ChronoDuration::seconds(LIMIT_JITTER_SECS),
            ChronoDuration::minutes(LIMIT_DEFAULT_BACKOFF_MINS),
            600,
        )
    }

    #[test]
    fn outcome_the_exact_five_day_run_string_is_a_limit_pause() {
        // The literal line that stopped the 5-day run, printed on stdout with a
        // clean (0) exit — the limit must supersede the exit code and pause.
        let out = out_with(
            "Paused — you've hit your limit. Limits will reset at 11:29 AM · View usage",
            "",
            Some(0),
            false,
        );
        match map(&out) {
            DriveOutcome::Limit { paused_until, reason, reset_at } => {
                assert_eq!(reason, "rate/usage limit reached");
                assert!(reset_at.is_some(), "11:29 AM must parse to a reset time");
                // paused_until = reset + jitter; reset is next-11:29, so it is
                // strictly in the future relative to `now`.
                let pu = chrono::DateTime::parse_from_rfc3339(&paused_until).unwrap();
                assert!(pu.with_timezone(&Utc) > fixed_now(), "paused into the future");
            }
            other => panic!("expected Limit, got {other:?}"),
        }
    }

    #[test]
    fn outcome_limit_supersedes_a_nonzero_exit() {
        // Limit text on STDERR + a non-zero exit → still a Limit (not a Flag).
        let out = out_with("", "usage limit reached; resets at 23:29", Some(1), false);
        assert!(matches!(map(&out), DriveOutcome::Limit { .. }), "limit supersedes exit code");
    }

    #[test]
    fn outcome_limit_without_parseable_time_uses_default_backoff() {
        let out = out_with("Paused — you've hit your limit. Try again later.", "", Some(0), false);
        match map(&out) {
            DriveOutcome::Limit { paused_until, reset_at, .. } => {
                assert_eq!(reset_at, None, "no parseable reset time → reset_at None");
                // Falls back to now + default backoff (30 min).
                let expected = (fixed_now() + ChronoDuration::minutes(LIMIT_DEFAULT_BACKOFF_MINS))
                    .to_rfc3339();
                assert_eq!(paused_until, expected, "unparseable reset → capped backoff");
            }
            other => panic!("expected Limit, got {other:?}"),
        }
    }

    #[test]
    fn outcome_clean_exit_no_limit_is_done() {
        let out = out_with("Wrote 3 files. Build OK.", "", Some(0), false);
        assert_eq!(map(&out), DriveOutcome::Done);
    }

    #[test]
    fn outcome_timeout_is_stalled() {
        let out = out_with("partial...", "", None, true);
        assert_eq!(map(&out), DriveOutcome::Stalled { timeout_secs: 600 });
    }

    #[test]
    fn outcome_nonzero_exit_no_limit_is_flagged() {
        let out = out_with("", "error: compilation failed", Some(101), false);
        assert_eq!(map(&out), DriveOutcome::NonZeroExit { exit_code: Some(101) });
    }

    // --- drive_run behavior tests (DB-guarded; NEVER spawn real claude) ---
    //
    // These pass a hand-built DriveConfig straight into `drive_run`, so they
    // never read the process-global env and never race a sibling test over
    // SENSEI_RUN_DRIVE. The agent program is always a harmless stub
    // (`echo` / `sh`), so no real `claude` is ever executed.

    /// A DriveConfig with a stub agent program and a short timeout for tests.
    fn stub_cfg(agent_cmd: &str) -> DriveConfig {
        DriveConfig {
            enabled: true,
            agent_cmd: agent_cmd.to_string(),
            timeout: Duration::from_secs(10),
        }
    }

    /// Seed a project whose single repo-root folder points at a freshly-created,
    /// per-test on-disk dir, so `resolve_cwd` yields a spawnable cwd. Each call
    /// mints a UNIQUE dir + abs_path so the three enabled-drive tests never race
    /// on the shared temp dir or the `folders.abs_path` UNIQUE constraint.
    /// Returns `(project_id, watch_root_id, cwd_dir)` for cleanup.
    async fn seed_project_with_cwd(
        pg: &crate::db::pg_store::PgStore,
    ) -> (uuid::Uuid, uuid::Uuid, std::path::PathBuf) {
        let uniq = uuid::Uuid::new_v4();
        let dir = std::env::temp_dir().join(format!("sensei_drive_test_{uniq}"));
        std::fs::create_dir_all(&dir).unwrap();
        let abs = dir.to_string_lossy().into_owned();

        let project_id =
            pg.create_project(&format!("drive-test-{uniq}"), None, None).await.unwrap();
        let root_id = pg
            .add_watch_root(&format!("/_test/drive/{uniq}"), "drive_root", &serde_json::json!([]))
            .await
            .unwrap();
        pg.upsert_folder(&root_id, "standalone", "repo", "repo", &abs, None, Some(&project_id))
            .await
            .unwrap();
        (project_id, root_id, dir)
    }

    async fn cleanup_project(
        pg: &crate::db::pg_store::PgStore,
        project_id: &uuid::Uuid,
        root_id: &uuid::Uuid,
        dir: &std::path::Path,
    ) {
        // Deleting the watch root cascades its folders; then drop the project.
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
            .bind(root_id)
            .execute(pg.pool())
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(project_id)
            .execute(pg.pool())
            .await
            .ok();
        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn disabled_drive_heartbeats_only_no_feature_event() {
        // The full handler with env unset (the process default) must keep exact
        // P3.2 behavior: one housekeeping tick, NO FeatureStarted. We do not set
        // SENSEI_RUN_DRIVE at all, so this asserts the OFF-by-default posture
        // without touching env (and thus without racing enabled tests).
        let Some(ctx) = make_ctx().await else {
            return;
        };
        let pg = ctx.pg();
        // Even give it a resolvable goal — disabled means it still must not fire.
        let id = pg
            .create_run(&NewRun { goal: Some("do the thing".into()), ..Default::default() })
            .await
            .unwrap();

        let task = Task::new(TaskKind::AdvanceRun, "", &id.to_string());
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 1);

        let events = pg.list_run_events(&id, 10).await.unwrap();
        assert_eq!(events.len(), 1, "disabled drive appends only the housekeeping tick");
        assert_eq!(events[0].kind, RunEventKind::Housekeeping);
        assert!(
            !events.iter().any(|e| e.kind == RunEventKind::FeatureStarted),
            "disabled drive must not emit FeatureStarted"
        );

        pg_delete_run(pg, &id).await;
    }

    #[tokio::test]
    async fn enabled_drive_with_stub_emits_feature_started_then_done() {
        let Some(ctx) = make_ctx().await else {
            return;
        };
        let pg = ctx.pg();
        // A real dir so resolve_cwd yields a spawnable cwd.
        let (project_id, root_id, dir) = seed_project_with_cwd(pg).await;

        let id = pg
            .create_run(&NewRun {
                project_id: Some(project_id),
                goal: Some("drive the next step".into()),
                // Attributable run → resolves to the permissive fallback stance
                // (none set). Unattributed runs now fail CLOSED (strict) — see
                // resolve_run_stance — and would not drive.
                author_email: Some(format!("drive-stub-{}@test.local", uuid::Uuid::new_v4())),
                ..Default::default()
            })
            .await
            .unwrap();

        // `echo -p <prompt>` exits 0 → FeatureStarted then FeatureDone.
        let run = pg.get_run(&id).await.unwrap().unwrap();
        drive_run(&ctx, &stub_cfg("echo"), &run).await.unwrap();

        // Events are newest-first.
        let kinds: Vec<RunEventKind> =
            pg.list_run_events(&id, 10).await.unwrap().into_iter().map(|e| e.kind).collect();
        assert!(
            kinds.contains(&RunEventKind::FeatureStarted),
            "expected FeatureStarted, got {kinds:?}"
        );
        assert!(kinds.contains(&RunEventKind::FeatureDone), "expected FeatureDone, got {kinds:?}");
        assert!(!kinds.contains(&RunEventKind::Flagged), "clean exit should not Flag");

        // current_feature was set to the short label; status left Running.
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert_eq!(run.current_feature.as_deref(), Some("drive the next step"));
        assert_eq!(
            run.status,
            RelayRunStatus::Running,
            "0-exit leaves the run Running for the next tick"
        );

        pg_delete_run(pg, &id).await;
        cleanup_project(pg, &project_id, &root_id, &dir).await;
    }

    #[tokio::test]
    async fn enabled_drive_blocked_by_ask_always_author_stance() {
        let Some(ctx) = make_ctx().await else {
            return;
        };
        let pg = ctx.pg();
        let (project_id, root_id, dir) = seed_project_with_cwd(pg).await;

        // The run's author has an `ask_always` stance: an unattended headless
        // single-shot cannot ask a human before each step, so the drive must NOT
        // launch — even with a stub that WOULD exit 0 if it were spawned.
        let author = format!("ask-always-{}@test.local", uuid::Uuid::new_v4());
        sqlx_core::query::query(
            "INSERT INTO sensei.stances(user_key, namespace_id, autonomy) \
             VALUES ($1, NULL, 'ask_always'::sensei.stance_autonomy)",
        )
        .bind(&author)
        .execute(pg.pool())
        .await
        .unwrap();

        let id = pg
            .create_run(&NewRun {
                project_id: Some(project_id),
                goal: Some("would drive but stance blocks".into()),
                author_email: Some(author.clone()),
                ..Default::default()
            })
            .await
            .unwrap();

        let run = pg.get_run(&id).await.unwrap().unwrap();
        drive_run(&ctx, &stub_cfg("echo"), &run).await.unwrap();

        let kinds: Vec<RunEventKind> =
            pg.list_run_events(&id, 10).await.unwrap().into_iter().map(|e| e.kind).collect();
        assert!(kinds.contains(&RunEventKind::Flagged), "ask_always must Flag, got {kinds:?}");
        assert!(
            !kinds.contains(&RunEventKind::FeatureStarted),
            "ask_always must not announce or spawn, got {kinds:?}"
        );
        // Status untouched — the run stays Running for a human / a later tick.
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert_eq!(run.status, RelayRunStatus::Running);

        sqlx_core::query::query("DELETE FROM sensei.stances WHERE user_key = $1")
            .bind(&author)
            .execute(pg.pool())
            .await
            .ok();
        pg_delete_run(pg, &id).await;
        cleanup_project(pg, &project_id, &root_id, &dir).await;
    }

    #[tokio::test]
    async fn enabled_drive_without_project_flags_and_does_not_spawn() {
        let Some(ctx) = make_ctx().await else {
            return;
        };
        let pg = ctx.pg();
        // No project_id → no cwd. Use a bogus agent program to PROVE no spawn
        // happens (a spawn attempt of this name would surface a Spawn error path
        // that still Flags, but never a FeatureStarted).
        let id = pg
            .create_run(&NewRun { goal: Some("would drive".into()), ..Default::default() })
            .await
            .unwrap();

        let run = pg.get_run(&id).await.unwrap().unwrap();
        drive_run(&ctx, &stub_cfg("definitely_not_a_real_program_xyzzy"), &run).await.unwrap();

        let events = pg.list_run_events(&id, 10).await.unwrap();
        assert_eq!(events.len(), 1, "no-cwd drive emits exactly one Flagged event");
        assert_eq!(events[0].kind, RunEventKind::Flagged);
        assert!(
            !events.iter().any(|e| e.kind == RunEventKind::FeatureStarted),
            "no cwd → must not announce a feature or spawn"
        );

        pg_delete_run(pg, &id).await;
    }

    #[tokio::test]
    async fn enabled_drive_with_empty_goal_flags() {
        let Some(ctx) = make_ctx().await else {
            return;
        };
        let pg = ctx.pg();
        let (project_id, root_id, dir) = seed_project_with_cwd(pg).await;

        // Resolvable cwd but a blank goal → Flag, no spawn.
        let id = pg
            .create_run(&NewRun {
                project_id: Some(project_id),
                goal: Some("   ".into()),
                ..Default::default()
            })
            .await
            .unwrap();

        let run = pg.get_run(&id).await.unwrap().unwrap();
        drive_run(&ctx, &stub_cfg("echo"), &run).await.unwrap();

        let events = pg.list_run_events(&id, 10).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, RunEventKind::Flagged);

        pg_delete_run(pg, &id).await;
        cleanup_project(pg, &project_id, &root_id, &dir).await;
    }

    #[tokio::test]
    async fn enabled_drive_nonzero_exit_flags_and_stays_running() {
        let Some(ctx) = make_ctx().await else {
            return;
        };
        let pg = ctx.pg();
        let (project_id, root_id, dir) = seed_project_with_cwd(pg).await;

        let id = pg
            .create_run(&NewRun {
                project_id: Some(project_id),
                goal: Some("drive a failing step".into()),
                // A driven run must be attributable — an author resolves to a
                // stance (none set here → the permissive fallback). Without one,
                // resolve_run_stance now fails CLOSED (strict) and won't drive.
                author_email: Some(format!("drive-fail-{}@test.local", uuid::Uuid::new_v4())),
                ..Default::default()
            })
            .await
            .unwrap();

        // The drive always invokes `<agent_cmd> -p <prompt>`. `false` ignores
        // its args and exits non-zero (1), giving a deterministic failure path
        // WITHOUT spawning real claude. (A specific exit code would need a shell
        // script, but the args are fixed to `-p <prompt>`, so a program that
        // always fails is the clean stub.)
        let run = pg.get_run(&id).await.unwrap().unwrap();
        drive_run(&ctx, &stub_cfg("false"), &run).await.unwrap();

        let kinds: Vec<RunEventKind> =
            pg.list_run_events(&id, 10).await.unwrap().into_iter().map(|e| e.kind).collect();
        assert!(kinds.contains(&RunEventKind::FeatureStarted), "step was announced");
        assert!(kinds.contains(&RunEventKind::Flagged), "non-zero exit Flags, got {kinds:?}");
        assert!(!kinds.contains(&RunEventKind::FeatureDone), "non-zero exit is not Done");

        // progress-over-asking: Flagged-but-Running (not Failed).
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert_eq!(run.status, RelayRunStatus::Running, "non-zero exit keeps the run Running");

        pg_delete_run(pg, &id).await;
        cleanup_project(pg, &project_id, &root_id, &dir).await;
    }

    /// Write an executable stub that IGNORES its args (the drive passes
    /// `-p <prompt>`) and prints `line` to stdout, then exits 0 — so we can feed
    /// the real drive path a "you've hit your limit" message WITHOUT ever
    /// spawning `claude`. Returns the stub's path (used as `agent_cmd`).
    fn limit_stub_script(line: &str) -> std::path::PathBuf {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        let uniq = uuid::Uuid::new_v4();
        let path = std::env::temp_dir().join(format!("sensei_limit_stub_{uniq}.sh"));
        let mut f = std::fs::File::create(&path).unwrap();
        // `printf '%s\n'` avoids any echo-flag portability issues with the em
        // dash / interpunct in the limit copy.
        writeln!(f, "#!/bin/sh\nprintf '%s\\n' \"{line}\"\nexit 0").unwrap();
        f.flush().unwrap();
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).unwrap();
        path
    }

    #[tokio::test]
    async fn enabled_drive_on_limit_pauses_and_schedules_auto_resume() {
        // THE 5-day-run fix, end to end through the real drive path: a stub
        // agent prints the exact limit line on stdout and exits 0. The drive
        // must PAUSE the run (paused_until in the future) + emit PausedOnLimit,
        // and must NOT emit FeatureDone (the limit supersedes the 0 exit). No
        // real claude is spawned. `resume_due_runs` (tested elsewhere) then
        // auto-resumes when paused_until elapses.
        let Some(ctx) = make_ctx().await else {
            return;
        };
        let pg = ctx.pg();
        let (project_id, root_id, dir) = seed_project_with_cwd(pg).await;

        let id = pg
            .create_run(&NewRun {
                project_id: Some(project_id),
                goal: Some("drive the next step".into()),
                // Attributable run → permissive fallback stance (none set), so the
                // drive proceeds to the limit path. Unattributed → strict, no drive.
                author_email: Some(format!("drive-limit-{}@test.local", uuid::Uuid::new_v4())),
                ..Default::default()
            })
            .await
            .unwrap();

        let stub = limit_stub_script(
            "Paused — you've hit your limit. Limits will reset at 11:29 AM · View usage",
        );
        let cfg = DriveConfig {
            enabled: true,
            agent_cmd: stub.to_string_lossy().into_owned(),
            timeout: Duration::from_secs(10),
        };

        let run = pg.get_run(&id).await.unwrap().unwrap();
        drive_run(&ctx, &cfg, &run).await.unwrap();

        // The run is Paused with a future paused_until…
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert_eq!(
            run.status,
            RelayRunStatus::Paused,
            "a limit pauses the run (never fails/stops)"
        );
        let paused_until = run.paused_until.as_deref().expect("paused_until must be set");
        let pu = chrono::DateTime::parse_from_rfc3339(paused_until).unwrap();
        assert!(pu.with_timezone(&chrono::Utc) > chrono::Utc::now(), "paused into the future");
        assert!(run.pause_reason.as_deref().unwrap().contains("limit"), "logical reason recorded");

        // …a PausedOnLimit cadence event was emitted, and NO FeatureDone.
        let kinds: Vec<RunEventKind> =
            pg.list_run_events(&id, 10).await.unwrap().into_iter().map(|e| e.kind).collect();
        assert!(kinds.contains(&RunEventKind::FeatureStarted), "step was announced");
        assert!(
            kinds.contains(&RunEventKind::PausedOnLimit),
            "expected PausedOnLimit, got {kinds:?}"
        );
        assert!(!kinds.contains(&RunEventKind::FeatureDone), "a limit supersedes the exit code");
        assert!(!kinds.contains(&RunEventKind::Flagged), "a limit is a pause, not a flag");

        pg_delete_run(pg, &id).await;
        cleanup_project(pg, &project_id, &root_id, &dir).await;
        std::fs::remove_file(&stub).ok();
    }
}
