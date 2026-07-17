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

use super::super::executor::TaskContext;
use super::super::Task;
use crate::agent_spawn::{run_agent, AgentCommand};
use crate::runs::{Run, RunEventKind};
use std::time::Duration;

/// Hard wall-clock cap for one agent drive step. Long enough for a real
/// `claude -p` step to make progress, but bounded so a wedged agent can never
/// hang a run tick — on expiry the child is killed+reaped and the run is marked
/// `Stalled` for the watchdog (P3.6) to recover.
const DRIVE_TIMEOUT: Duration = Duration::from_secs(600); // 10 minutes

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
        let enabled = std::env::var("SENSEI_RUN_DRIVE")
            .ok()
            .map(|v| Self::is_truthy(&v))
            .unwrap_or(false);
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
    let Some(run) = ctx
        .pg()
        .get_run(&run_id)
        .await
        .map_err(|e| format!("get_run failed: {e}"))?
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
        // Blocked: waiting on a hard-block gate (a human reply). No autonomous
        // progress until the gate clears, so the tick is a no-op — but we still
        // want a live heartbeat so a blocked run isn't mistaken for a crash.
        // Fall through to the heartbeat path.
        RelayRunStatus::Running | RelayRunStatus::Stalled | RelayRunStatus::Blocked => {
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
            // core is testable without touching process-global env. Blocked runs
            // still only heartbeat — no autonomous progress past a hard gate.
            let cfg = DriveConfig::from_env();
            if cfg.enabled && run.status != RelayRunStatus::Blocked {
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
            return flag(
                ctx,
                run,
                "no resolvable project working directory; skipping drive",
            )
            .await;
        }
    };

    // 2. Prompt = the run's goal (single-shot MVP). Empty goal → nothing to
    //    drive; flag and stop rather than spawn a no-op agent.
    let goal = run.goal.as_deref().map(str::trim).unwrap_or("");
    if goal.is_empty() {
        return flag(ctx, run, "run has no goal; nothing to drive").await;
    }
    let prompt = goal.to_string();

    // 3. Announce the step. `detail` stays logical/short — a bounded goal label,
    //    never the full prompt body or any code.
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
            &serde_json::json!({ "goal": label }),
        )
        .await
        .map_err(|e| format!("append_run_event(FeatureStarted) failed: {e}"))?;

    // 4. Build the command: `<agent_cmd> -p <prompt>` in the project cwd, under a
    //    bounded timeout. The spawned claude carries the sensei plugin, so its
    //    PreToolUse hook drives the existing `/hook/gate` — no gate wiring here.
    let cmd = AgentCommand::new(
        cfg.agent_cmd.clone(),
        vec!["-p".to_string(), prompt],
        cfg.timeout,
    )
    .with_cwd(&cwd);

    // Refresh liveness right before a potentially long spawn so the stall
    // detector doesn't trip mid-step.
    ctx.pg()
        .touch_run_heartbeat(&run_id)
        .await
        .map_err(|e| format!("touch_run_heartbeat failed: {e}"))?;

    // 5. Spawn + supervise. The primitive never panics and always kills+reaps.
    let phase = run.current_phase.clone();
    let feature = Some(label.clone());
    match run_agent(&cmd).await {
        Err(e) => {
            // Could not launch/reap the child (bad binary, EACCES, i/o). Not a
            // task error — record it logically and let the next tick retry.
            tracing::warn!(run_id = %run_id, error = %e, "relay drive: agent spawn failed");
            append(
                ctx,
                &run_id,
                RunEventKind::Flagged,
                phase.as_deref(),
                feature.as_deref(),
                serde_json::json!({ "note": "agent could not be started" }),
            )
            .await
        }
        Ok(out) if out.timed_out => {
            // Timeout → Stalled so the watchdog/P3.6 can recover it. The child
            // was already killed+reaped by the primitive.
            tracing::warn!(run_id = %run_id, "relay drive: agent step timed out");
            append(
                ctx,
                &run_id,
                RunEventKind::Stalled,
                phase.as_deref(),
                feature.as_deref(),
                serde_json::json!({ "note": "agent step timed out", "timeout_secs": cfg.timeout.as_secs() }),
            )
            .await?;
            ctx.pg()
                .update_run_status(&run_id, dojo_protocol::relay::RelayRunStatus::Stalled, None, None)
                .await
                .map_err(|e| format!("update_run_status(Stalled) failed: {e}"))
        }
        Ok(out) if out.exit_code == Some(0) => {
            // Clean exit → the step finished. Emit FeatureDone and leave the run
            // Running: the next tick advances again. We deliberately do NOT
            // complete_run(Done) here — a real completion signal arrives with
            // the plan loop (P3.5/P3.7); a single-shot 0-exit only means "this
            // step is done", not "the whole plan is done".
            append(
                ctx,
                &run_id,
                RunEventKind::FeatureDone,
                phase.as_deref(),
                feature.as_deref(),
                serde_json::json!({ "feature": label }),
            )
            .await
        }
        Ok(out) => {
            // Non-zero exit. Per progress-over-asking we record it as Flagged
            // and keep the run Running (rather than hard-Failing) so the next
            // tick can retry / a human can inspect — the exit code is logical
            // status, and we never dump stdout/stderr into the event.
            let code = out.exit_code;
            tracing::warn!(run_id = %run_id, exit_code = ?code, "relay drive: agent step exited non-zero");
            append(
                ctx,
                &run_id,
                RunEventKind::Flagged,
                phase.as_deref(),
                feature.as_deref(),
                serde_json::json!({ "note": "agent step exited non-zero", "exit_code": code }),
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
    if dir.is_dir() {
        Ok(Some(dir))
    } else {
        Ok(None)
    }
}

/// Emit a `Flagged` run_event with a short logical note (heartbeat is already
/// stamped by the caller) and return `Ok(())` — the "can't drive, don't spawn"
/// terminal for a tick. Status is left as-is (Flagged-but-running), matching
/// progress-over-asking.
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
    use crate::tasks::queue::TaskQueue;
    use crate::tasks::TaskKind;
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
            event_tx: { let (tx, _) = tokio::sync::broadcast::channel(16); tx },
            breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        });
        Some(Arc::new(TaskContext { queue, app_state, _graph_path: None, logger: sensei_logger::Logger::noop() }))
    }

    #[tokio::test]
    async fn empty_run_id_is_empty_work() {
        // No DB needed — the empty-path guard short-circuits before any query.
        let Some(ctx) = make_ctx().await else { return; };
        let task = Task::new(TaskKind::AdvanceRun, "", "");
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn non_uuid_run_id_is_empty_work() {
        let Some(ctx) = make_ctx().await else { return; };
        let task = Task::new(TaskKind::AdvanceRun, "", "not-a-uuid");
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn unknown_run_is_empty_work() {
        let Some(ctx) = make_ctx().await else { return; };
        let id = uuid::Uuid::new_v4().to_string();
        let task = Task::new(TaskKind::AdvanceRun, "", &id);
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn running_run_heartbeats_and_logs() {
        let Some(ctx) = make_ctx().await else { return; };
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
        let Some(ctx) = make_ctx().await else { return; };
        let pg = ctx.pg();
        let id = pg.create_run(&NewRun::default()).await.unwrap();
        pg.update_run_status(&id, RelayRunStatus::Paused, Some("2999-01-01T00:00:00Z"), Some("cap"))
            .await.unwrap();

        let task = Task::new(TaskKind::AdvanceRun, "", &id.to_string());
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 0, "a paused run is a no-op tick");

        // No heartbeat, no event.
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert!(run.heartbeat_at.is_none(), "paused run is not heartbeated");
        assert!(pg.list_run_events(&id, 10).await.unwrap().is_empty(), "no event for a paused tick");

        pg_delete_run(pg, &id).await;
    }

    #[tokio::test]
    async fn terminal_run_is_noop() {
        let Some(ctx) = make_ctx().await else { return; };
        let pg = ctx.pg();
        let id = pg.create_run(&NewRun::default()).await.unwrap();
        pg.complete_run(&id, RelayRunStatus::Done).await.unwrap();

        let task = Task::new(TaskKind::AdvanceRun, "", &id.to_string());
        assert_eq!(advance_run(&ctx, &task).await.unwrap(), 0, "a done run is a no-op tick");
        assert!(pg.list_run_events(&id, 10).await.unwrap().is_empty());

        pg_delete_run(pg, &id).await;
    }

    async fn pg_delete_run(pg: &crate::db::pg_store::PgStore, id: &uuid::Uuid) {
        sqlx_core::query::query("DELETE FROM activity.runs WHERE id = $1")
            .bind(id).execute(pg.pool()).await.unwrap();
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

        let project_id = pg
            .create_project(&format!("drive-test-{uniq}"), None, None)
            .await
            .unwrap();
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
            .bind(root_id).execute(pg.pool()).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(project_id).execute(pg.pool()).await.ok();
        std::fs::remove_dir_all(dir).ok();
    }

    #[tokio::test]
    async fn disabled_drive_heartbeats_only_no_feature_event() {
        // The full handler with env unset (the process default) must keep exact
        // P3.2 behavior: one housekeeping tick, NO FeatureStarted. We do not set
        // SENSEI_RUN_DRIVE at all, so this asserts the OFF-by-default posture
        // without touching env (and thus without racing enabled tests).
        let Some(ctx) = make_ctx().await else { return; };
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
        let Some(ctx) = make_ctx().await else { return; };
        let pg = ctx.pg();
        // A real dir so resolve_cwd yields a spawnable cwd.
        let (project_id, root_id, dir) = seed_project_with_cwd(pg).await;

        let id = pg
            .create_run(&NewRun {
                project_id: Some(project_id),
                goal: Some("drive the next step".into()),
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
        assert!(kinds.contains(&RunEventKind::FeatureStarted), "expected FeatureStarted, got {kinds:?}");
        assert!(kinds.contains(&RunEventKind::FeatureDone), "expected FeatureDone, got {kinds:?}");
        assert!(!kinds.contains(&RunEventKind::Flagged), "clean exit should not Flag");

        // current_feature was set to the short label; status left Running.
        let run = pg.get_run(&id).await.unwrap().unwrap();
        assert_eq!(run.current_feature.as_deref(), Some("drive the next step"));
        assert_eq!(run.status, RelayRunStatus::Running, "0-exit leaves the run Running for the next tick");

        pg_delete_run(pg, &id).await;
        cleanup_project(pg, &project_id, &root_id, &dir).await;
    }

    #[tokio::test]
    async fn enabled_drive_without_project_flags_and_does_not_spawn() {
        let Some(ctx) = make_ctx().await else { return; };
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
        let Some(ctx) = make_ctx().await else { return; };
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
        let Some(ctx) = make_ctx().await else { return; };
        let pg = ctx.pg();
        let (project_id, root_id, dir) = seed_project_with_cwd(pg).await;

        let id = pg
            .create_run(&NewRun {
                project_id: Some(project_id),
                goal: Some("drive a failing step".into()),
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
}
