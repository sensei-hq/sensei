//! `knowledge` group reads (#227) — what has been distilled, and what is waiting.

use crate::db::pg_store::PgStore;

/// `(eligible_patterns, eligible_corrections)` — the two halves of the eligible
/// backlog, returned SPLIT rather than pre-summed so the caller can both add them
/// up and report the breakdown.
pub(crate) type EligibleCounts = (i64, i64);

impl PgStore {
    /// # memories created for this project within the rolling window.
    ///
    /// Bucketed on `created_at` — the stable "learned" timestamp — project-scoped
    /// by the `project_id` FK. NOT `modified_at`, which is bumped on every
    /// reinforcement and so cannot date a creation. A memory later archived still
    /// counts: it WAS distilled.
    ///
    /// The window is the rolling one only. This read has no single-day form: it is
    /// the numerator of a forward-only snapshot whose denominator cannot be
    /// reconstructed for a past day, so the computer skips a historical `as_of`
    /// entirely rather than asking this query for one.
    pub(crate) async fn knowledge_memories_created(
        &self,
        project_id: &uuid::Uuid,
        window_days: u32,
    ) -> Result<i64, String> {
        let (n,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*)::int8
           FROM sensei.memories
          WHERE project_id  = $1
            AND created_at >= now() - make_interval(days => $2::int)",
        )
        .bind(project_id)
        .bind(window_days as i32)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(n)
    }

    /// `(eligible_patterns, eligible_corrections)` — each a CURRENT snapshot of the
    /// items that have recurred `>= min_instances` times, from the two sources that
    /// are cleanly project-attributable:
    ///
    /// - `inference.detected_patterns` — the recurrence tally column is
    ///   `instance_count`; project-scoped by the NOT-NULL `project_id` FK.
    /// - `inference.corrections` — the recurrence tally column is `count` (there is
    ///   no `instance_count` here); a correction is a GLOBAL cluster attributed to
    ///   projects through its `project_ids` UUID ARRAY (it can span several), so
    ///   THIS project's share is membership: `$1 = ANY(project_ids)`.
    ///
    /// A snapshot, not a window — "how much is waiting to be distilled right now".
    /// Strictly scoped: another project's patterns or corrections never leak in.
    pub(crate) async fn knowledge_eligible_counts(
        &self,
        project_id: &uuid::Uuid,
        min_instances: i32,
    ) -> Result<EligibleCounts, String> {
        let (patterns, corrections): EligibleCounts = sqlx_core::query_as::query_as(
            "SELECT (SELECT count(*)::int8
                   FROM inference.detected_patterns
                  WHERE project_id     = $1
                    AND instance_count >= $2)                       AS eligible_patterns
              , (SELECT count(*)::int8
                   FROM inference.corrections
                  WHERE $1 = ANY(project_ids)
                    AND count >= $2)                                AS eligible_corrections",
        )
        .bind(project_id)
        .bind(min_instances)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok((patterns, corrections))
    }
}
