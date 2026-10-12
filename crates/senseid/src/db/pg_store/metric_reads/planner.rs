//! `planner` group reads (#227) — the per-group DATA-day DISCOVERY.
//!
//! A group's data days are the distinct TRUE-OCCURRENCE days its source rows
//! carry — the day the work actually happened on, never an insert-time
//! `created_at`. Each query reads the same measurable base that group's computer
//! writes over, so a day returned here is a day that can genuinely produce a row.
//!
//! These are WHOLE-HISTORY reads, not windowed ones: the engine wants the
//! group's earliest day (its `min_date`) as much as its latest, so there is no
//! `day_filter`/`bind_day` here and no `window_days`/`as_of` parameter. The only
//! binding is the project.
//!
//! `churn` and `quality` are absent on purpose — their day set is the git commit
//! log, not a table, so the planner reads them through `git_commit_days` rather
//! than through this module.

use chrono::NaiveDate;

use crate::db::pg_store::PgStore;

impl PgStore {
    /// `session_outcomes` — days of measurable (`outcome is not null`) sessions,
    /// bucketed on `sessions.started_at`.
    pub(crate) async fn planner_session_outcome_days(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<NaiveDate>, String> {
        let sql = "SELECT DISTINCT date_trunc('day', s.started_at)::date AS day
                     FROM activity.sessions s
                    WHERE s.project_id = $1
                      AND s.outcome   IS NOT NULL";
        self.planner_days(sql, project_id).await
    }

    /// `autonomy` — the UNION of run-started days (`runs.started_at`) and
    /// `UserPromptSubmit`-event days (client `ts`, attributed via
    /// `sessions.client_session_id`). Only `UserPromptSubmit` — NOT `Stop` — anchors
    /// the event arm: `interruption_rate` (`Stop / UserPromptSubmit`) emits NO row on
    /// a `UserPromptSubmit = 0` day (a 0/0 would be fabricated), so a `Stop`-only day
    /// is not a measurable data day.
    pub(crate) async fn planner_autonomy_days(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<NaiveDate>, String> {
        let sql = "SELECT DISTINCT day FROM (
                       SELECT date_trunc('day', r.started_at)::date AS day
                         FROM activity.runs r
                        WHERE r.project_id = $1
                       UNION
                       SELECT date_trunc('day', to_timestamp(ae.ts / 1000.0))::date AS day
                         FROM activity.assistant_events ae
                         JOIN activity.sessions        s ON s.client_session_id = ae.session_id
                        WHERE s.project_id   = $1
                          AND ae.event_type  = 'UserPromptSubmit'
                   ) u";
        self.planner_days(sql, project_id).await
    }

    /// `usage` — days with token-accounted turns. Only claude_code carries the
    /// split so far, so a day of Zed/OpenCode-only work has no data day
    /// and is not planned — which is honest: we cannot measure reuse we
    /// never captured.
    pub(crate) async fn planner_usage_days(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<NaiveDate>, String> {
        let sql = "SELECT DISTINCT date_trunc('day', s.started_at)::date AS day
                     FROM activity.transcript_turns tt
                     JOIN activity.sessions         s ON s.client_session_id = tt.session_id
                    WHERE s.project_id  = $1
                      AND tt.tokens_in IS NOT NULL";
        self.planner_days(sql, project_id).await
    }

    /// `session_process` — days of sessions the LLM analyzer has SCORED
    /// (`props ? 'process'`), bucketed on `started_at` — the base the process
    /// computer writes over.
    pub(crate) async fn planner_session_process_days(
        &self,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<NaiveDate>, String> {
        let sql = "SELECT DISTINCT date_trunc('day', s.started_at)::date AS day
                     FROM activity.sessions s
                    WHERE s.project_id = $1
                      AND s.props ? 'process'";
        self.planner_days(sql, project_id).await
    }

    /// Run one project-keyed day-set query and collect its days. Every data-day
    /// read above has the same shape — a single `$1` project binding and a single
    /// `day` column — so the bind/fetch/collect lives here once. Private: the SQL
    /// is still assembled and owned inside this layer, never handed in.
    async fn planner_days(
        &self,
        sql: &str,
        project_id: &uuid::Uuid,
    ) -> Result<Vec<NaiveDate>, String> {
        let rows: Vec<(NaiveDate,)> = sqlx_core::query_as::query_as(sql)
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows.into_iter().map(|(d,)| d).collect())
    }
}
