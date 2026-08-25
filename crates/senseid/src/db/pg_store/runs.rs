use super::*;

#[allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]
impl PgStore {
    /// Create a run. `id`, `status` (`'running'`), and all timestamps are
    /// DB-defaulted; `plan_ref`/`max_concurrency` fall back to the DDL defaults
    /// (`''` / `1`) when the caller passes `None`. Returns the new run id.
    pub async fn create_run(&self, new: &NewRun) -> Result<uuid::Uuid, String> {
        let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.runs
                (project_id, plan_ref, goal, dojo_session_id, max_concurrency,
                 author_name, author_email, plan_graph)
             VALUES($1, COALESCE($2, ''), $3, $4, COALESCE($5, 1), $6, $7, $8) RETURNING id",
        )
        .bind(new.project_id)
        .bind(new.plan_ref.as_deref())
        .bind(new.goal.as_deref())
        .bind(new.dojo_session_id)
        .bind(new.max_concurrency)
        .bind(new.author_name.as_deref())
        .bind(new.author_email.as_deref())
        .bind(new.plan_graph.as_ref())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// Read a run's authored plan graph (jsonb), or `None` if the run has none
    /// (ad-hoc/cadence-derived) or does not exist. Kept off the 16-column
    /// `RUN_SELECT` tuple (same reason as `run_author`) and fetched on demand:
    /// only `publish_run` (authored-segment projection) and `update_task_status`
    /// (task-state write-back) need it.
    pub async fn run_plan_graph(
        &self,
        run_id: &uuid::Uuid,
    ) -> Result<Option<serde_json::Value>, String> {
        let row: Option<(Option<serde_json::Value>,)> =
            sqlx_core::query_as::query_as("SELECT plan_graph FROM activity.runs WHERE id = $1")
                .bind(run_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
        Ok(row.and_then(|(g,)| g))
    }

    /// Overwrite a run's authored plan graph (jsonb). Used by `update_task_status`
    /// to persist a task's new state (read-modify-write of the graph). A no-op-safe
    /// full replace — the caller owns merging.
    pub async fn set_run_plan_graph(
        &self,
        run_id: &uuid::Uuid,
        graph: &serde_json::Value,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.runs SET plan_graph = $2, updated_at = now() WHERE id = $1",
        )
        .bind(run_id)
        .bind(graph)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Read a run's stamped git author `(author_name, author_email)`. Kept off the
    /// wide `RUN_SELECT` tuple because sqlx caps tuple `FromRow` at 16 columns;
    /// `Run` reads stay 16-wide, and the author (a rarely-needed attribution
    /// field) is fetched on demand. `(None, None)` when the run is gone or was
    /// created without a resolvable git identity.
    pub async fn run_author(
        &self,
        run_id: &uuid::Uuid,
    ) -> Result<(Option<String>, Option<String>), String> {
        let row: Option<(Option<String>, Option<String>)> = sqlx_core::query_as::query_as(
            "SELECT author_name, author_email FROM activity.runs WHERE id = $1",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.unwrap_or((None, None)))
    }

    /// Fetch one run by id, or `None` if it does not exist.
    pub async fn get_run(&self, id: &uuid::Uuid) -> Result<Option<Run>, String> {
        let row: Option<(
            uuid::Uuid,
            Option<uuid::Uuid>,
            String,
            Option<String>,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<uuid::Uuid>,
            i32,
            String,
            Option<String>,
            Option<String>,
            String,
            String,
        )> = sqlx_core::query_as::query_as(&format!("{} WHERE id = $1", Self::RUN_SELECT))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        row.map(Self::map_run_row).transpose()
    }

    /// Runs that still need the scheduler's attention — `running`, `paused`,
    /// `stalled`, or `blocked` (uses the partial `runs_active_idx`).
    /// Newest-started first. `blocked` is included so a run waiting on a gate is
    /// still ticked (heartbeat) and shown in `GET /api/runs` — otherwise it
    /// drops out of the active set and looks crashed.
    pub async fn list_active_runs(&self) -> Result<Vec<Run>, String> {
        let rows: Vec<(
            uuid::Uuid, Option<uuid::Uuid>, String, Option<String>, String, Option<String>,
            Option<String>, Option<String>, Option<String>, Option<uuid::Uuid>,
            i32, String, Option<String>, Option<String>, String, String,
        )> = sqlx_core::query_as::query_as(&format!(
            "{} WHERE status IN ('running', 'paused', 'stalled', 'blocked') ORDER BY started_at DESC",
            Self::RUN_SELECT
        ))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        rows.into_iter().map(Self::map_run_row).collect()
    }

    /// The newest `running` or `stalled` run for a project, if any — the target
    /// of the workflow→run phase bridge ([`Self::advance_run_phase_for_project`]).
    /// `stalled` is included so an agent that went quiet (→ watchdog-stalled) and
    /// then resumes revives its run on the next `update_phase`. `paused`/`blocked`
    /// are excluded — a paused (limit-wait) or gate-blocked run shouldn't be
    /// silently advanced by a stray `update_phase`.
    pub async fn active_run_for_project(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Option<Run>, String> {
        let row: Option<(
            uuid::Uuid,
            Option<uuid::Uuid>,
            String,
            Option<String>,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<uuid::Uuid>,
            i32,
            String,
            Option<String>,
            Option<String>,
            String,
            String,
        )> = sqlx_core::query_as::query_as(&format!(
            "{} WHERE project_id = $1 AND status IN ('running', 'stalled') \
             ORDER BY started_at DESC LIMIT 1",
            Self::RUN_SELECT
        ))
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        row.map(Self::map_run_row).transpose()
    }

    /// Bridge a workflow phase transition onto a project's active run: append the
    /// pairing cadence events ([`crate::runs::phase_transition_events`]) and move
    /// the run's `current_phase`, so the run streams phases→segments to the relay
    /// while an agent works (`drive` stays OFF — this is status only). If the run
    /// had gone `stalled` (agent quiet), this fresh progress **revives** it to
    /// `running`. Returns the advanced run id, or `None` when there's no active
    /// run / no phase change. Best-effort: the caller logs and swallows errors so
    /// a bridge hiccup never fails the workflow-state write.
    pub async fn advance_run_phase_for_project(
        &self,
        project_id: &uuid::Uuid,
        phase: &str,
    ) -> Result<Option<uuid::Uuid>, String> {
        if phase.is_empty() {
            return Ok(None);
        }
        let Some(run) = self.active_run_for_project(project_id).await? else {
            return Ok(None);
        };
        let events = crate::runs::phase_transition_events(run.current_phase.as_deref(), phase);
        if events.is_empty() {
            return Ok(None);
        }
        // Agent progress on a stalled run = it's back → revive to running first,
        // so the appended events + the fresh heartbeat land on a running row.
        if run.status == dojo_protocol::relay::RelayRunStatus::Stalled {
            self.update_run_status(
                &run.id,
                dojo_protocol::relay::RelayRunStatus::Running,
                None,
                None,
            )
            .await?;
            self.append_run_event(
                &run.id,
                crate::runs::RunEventKind::Recovered,
                Some(phase),
                None,
                &serde_json::json!({ "via": "update_phase", "revived": true }),
            )
            .await?;
        }
        let detail = serde_json::json!({ "via": "update_phase" });
        for (kind, ph) in &events {
            self.append_run_event(&run.id, *kind, Some(ph), None, &detail).await?;
        }
        self.set_run_progress(&run.id, Some(phase), run.current_feature.as_deref()).await?;
        Ok(Some(run.id))
    }

    /// The timestamp (RFC-3339 text) of a run's newest **agent-progress** event —
    /// the stall signal's reference. Excludes the daemon's cadence/lifecycle kinds
    /// (`RunEventKind::is_progress() == false`, built from the enum so it never
    /// drifts) so the every-tick `housekeeping` marker can't mask an agent stall.
    /// `None` when the run has emitted no progress event yet (caller falls back to
    /// `started_at`).
    pub async fn last_progress_at(&self, run_id: &uuid::Uuid) -> Result<Option<String>, String> {
        let excluded: Vec<String> = crate::runs::RunEventKind::ALL
            .iter()
            .filter(|k| !k.is_progress())
            .map(|k| k.as_db_str().to_string())
            .collect();
        // `to_json(...)#>>'{}'` yields RFC-3339 (the format `parse_rfc3339` and the
        // rest of RUN_SELECT use) — NOT `::text`, whose `YYYY-MM-DD HH:MM:SS-05`
        // shape fails to parse and would silently fall back to started_at.
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "SELECT to_json(created_at)#>>'{}' FROM activity.run_events
              WHERE run_id = $1 AND kind::text <> ALL($2)
              ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .bind(run_id)
        .bind(&excluded)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(ts,)| ts))
    }

    /// Set a run's status and (optionally) its pause fields, bumping
    /// `updated_at`. `paused_until`/`pause_reason` are written as given, so pass
    /// `None` for both to clear a pause on resume.
    pub async fn update_run_status(
        &self,
        id: &uuid::Uuid,
        status: RelayRunStatus,
        paused_until: Option<&str>,
        pause_reason: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.runs
                SET status       = $2::sensei.run_status,
                    paused_until = $3::timestamptz,
                    pause_reason = $4,
                    updated_at   = now()
              WHERE id = $1",
        )
        .bind(id)
        .bind(status.as_db_str())
        .bind(paused_until)
        .bind(pause_reason)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Update the run's current phase/feature progress markers (+ `updated_at`).
    pub async fn set_run_progress(
        &self,
        id: &uuid::Uuid,
        phase: Option<&str>,
        feature: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.runs
                SET current_phase = $2, current_feature = $3, updated_at = now()
              WHERE id = $1",
        )
        .bind(id)
        .bind(phase)
        .bind(feature)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Bump the run's liveness heartbeat to `now()` (drives stall detection).
    /// Also refreshes `updated_at`.
    pub async fn touch_run_heartbeat(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.runs SET heartbeat_at = now(), updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Persist the cloud `dojo.relay_sessions(id)` this run mirrors, so a later
    /// publish tick (and the console/app) can join the local run to its relay
    /// session. A plain uuid across the DB boundary (no cross-DB FK). Idempotent:
    /// the P1 bridge writes it once, on the first successful publish. Also bumps
    /// `updated_at`.
    pub async fn set_run_dojo_session_id(
        &self,
        id: &uuid::Uuid,
        dojo_session_id: &uuid::Uuid,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.runs SET dojo_session_id = $2, updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .bind(dojo_session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Mark a run terminal — sets `status` (expected `Done`/`Failed`) and stamps
    /// `completed_at = now()` (+ `updated_at`).
    pub async fn complete_run(
        &self,
        id: &uuid::Uuid,
        status: RelayRunStatus,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.runs
                SET status = $2::sensei.run_status, completed_at = now(), updated_at = now()
              WHERE id = $1",
        )
        .bind(id)
        .bind(status.as_db_str())
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Append one cadence event to `activity.run_events`. `detail` is the
    /// structured, stripped payload (never code/diffs). Returns the new
    /// `bigserial` id.
    pub async fn append_run_event(
        &self,
        run_id: &uuid::Uuid,
        kind: RunEventKind,
        phase: Option<&str>,
        feature: Option<&str>,
        detail: &serde_json::Value,
    ) -> Result<i64, String> {
        let (id,): (i64,) = sqlx_core::query_as::query_as(
            "INSERT INTO activity.run_events(run_id, kind, phase, feature, detail)
             VALUES($1, $2::sensei.run_event_kind, $3, $4, $5) RETURNING id",
        )
        .bind(run_id)
        .bind(kind.as_db_str())
        .bind(phase)
        .bind(feature)
        .bind(detail)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// A run's cadence events, newest first, capped at `limit`. `kind` arrives
    /// as text and is parsed with [`RunEventKind::from_db_str`]; an unknown
    /// value is a hard error, never a silent skip.
    pub async fn list_run_events(
        &self,
        run_id: &uuid::Uuid,
        limit: i64,
    ) -> Result<Vec<RunEvent>, String> {
        let rows: Vec<(
            i64,
            uuid::Uuid,
            String,
            Option<String>,
            Option<String>,
            serde_json::Value,
            String,
        )> = sqlx_core::query_as::query_as(
            "SELECT id, run_id, kind::text, phase, feature, detail,
                        to_json(created_at)#>>'{}'
                   FROM activity.run_events
                  WHERE run_id = $1
                  ORDER BY created_at DESC, id DESC
                  LIMIT $2",
        )
        .bind(run_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        rows.into_iter()
            .map(|(id, run_id, kind, phase, feature, detail, created_at)| {
                let kind = RunEventKind::from_db_str(&kind)
                    .ok_or_else(|| format!("unknown run_event_kind from DB: {kind:?}"))?;
                Ok(RunEvent { id, run_id, kind, phase, feature, detail, created_at })
            })
            .collect()
    }

    /// Flip every `paused` run whose `paused_until` has elapsed back to
    /// `running`, clearing the pause fields. The `<=` comparison runs SQL-side
    /// (`paused_until <= now()`) so we never parse RFC-3339 back into Rust just
    /// to compare clocks. Returns the ids of the runs that were resumed, so the
    /// scheduler can log a `Resumed` cadence event + kick an `AdvanceRun` tick
    /// for each. A run with `paused_until IS NULL` (an indefinite/manual pause)
    /// is never auto-resumed.
    pub async fn resume_due_runs(&self) -> Result<Vec<uuid::Uuid>, String> {
        let rows: Vec<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "UPDATE activity.runs
                SET status       = 'running'::sensei.run_status,
                    paused_until = NULL,
                    pause_reason = NULL,
                    updated_at   = now()
              WHERE status = 'paused'::sensei.run_status
                AND paused_until IS NOT NULL
                AND paused_until <= now()
             RETURNING id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    // ── Watchdog (P3.6) ────────────────────────────────────────────────

    /// The runs the watchdog can act on — `running` or `stalled` — with just the
    /// fields it needs to assess liveness: `(id, status text, heartbeat_at,
    /// started_at, recovery_attempts)`. Deliberately a lightweight query (NOT
    /// [`Self::RUN_SELECT`]/[`Self::map_run_row`]) so adding the watchdog never
    /// perturbs the `Run` row surface. Timestamps come back as RFC-3339 via the
    /// same `to_json(col)#>>'{}'` idiom as `RUN_SELECT`; `heartbeat_at` is
    /// `Option` (a run may not have heartbeated yet, so the caller falls back to
    /// `started_at`).
    pub async fn list_recoverable_runs(
        &self,
    ) -> Result<Vec<(uuid::Uuid, String, Option<String>, String, i32)>, String> {
        let rows: Vec<(uuid::Uuid, String, Option<String>, String, i32)> =
            sqlx_core::query_as::query_as(
                "SELECT id, status::text,
                        to_json(heartbeat_at)#>>'{}',
                        to_json(started_at)#>>'{}',
                        recovery_attempts
                   FROM activity.runs
                  WHERE status IN ('running', 'stalled')",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Bounded auto-recovery: flip a `stalled` run back to `running`, record the
    /// new attempt count, and refresh the heartbeat so the recovered run isn't
    /// immediately re-flagged stale on the next watchdog tick.
    pub async fn recover_run(&self, id: &uuid::Uuid, next_attempt: i32) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.runs
                SET status            = 'running'::sensei.run_status,
                    recovery_attempts = $2,
                    heartbeat_at      = now(),
                    updated_at        = now()
              WHERE id = $1",
        )
        .bind(id)
        .bind(next_attempt)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Reset the bounded-recovery counter to 0 on real progress (a clean drive
    /// step) so a long overnight run that recovered earlier doesn't prematurely
    /// give up later.
    pub async fn reset_run_recovery(&self, id: &uuid::Uuid) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE activity.runs SET recovery_attempts = 0, updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── PG Function Wrappers ───────────────────────────────────────────

    /// The folder (repo) a run's project maps to
    /// (`sensei.folders.project_id = activity.runs.project_id`). Lets the
    /// constitution federation resolve the run's ruleset. `None` when the run has
    /// no project or no folder is indexed for it.
    pub async fn run_folder_id(&self, run_id: &uuid::Uuid) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT f.id
               FROM activity.runs r
               JOIN sensei.folders f ON f.project_id = r.project_id
              WHERE r.id = $1
              LIMIT 1",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(id,)| id))
    }

    /// The slug of a run's project namespace (`sensei.namespaces` scope=project),
    /// or None when the run has no project or no project-scope namespace. Fed to
    /// the relay federation so the Worker can open the caller's billing seat on
    /// this project (proof the user is actively using sensei there).
    pub async fn run_project_slug(&self, run_id: &uuid::Uuid) -> Result<Option<String>, String> {
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "SELECT n.slug
               FROM activity.runs r
               JOIN sensei.folders f ON f.project_id = r.project_id
               JOIN sensei.folder_namespaces fn ON fn.folder_id = f.id
               JOIN sensei.namespaces n ON n.id = fn.namespace_id
              WHERE r.id = $1 AND n.scope_key = 'project'
              LIMIT 1",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|(slug,)| slug))
    }

    /// The run's project as `(slug, name)` for the dōjō `dojo.projects` display row:
    /// the project-scope namespace slug (as [`run_project_slug`]) plus the project's
    /// display name (`sensei.projects.name`). Both are the user's own project
    /// metadata, federated as-is. `None` when the run has no bound project namespace.
    pub async fn run_project_info(
        &self,
        run_id: &uuid::Uuid,
    ) -> Result<Option<(String, String)>, String> {
        let row: Option<(String, String)> = sqlx_core::query_as::query_as(
            "SELECT n.slug, p.name
               FROM activity.runs r
               JOIN sensei.projects p ON p.id = r.project_id
               JOIN sensei.folders f ON f.project_id = r.project_id
               JOIN sensei.folder_namespaces fn ON fn.folder_id = f.id
               JOIN sensei.namespaces n ON n.id = fn.namespace_id
              WHERE r.id = $1 AND n.scope_key = 'project'
              LIMIT 1",
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row)
    }
}
