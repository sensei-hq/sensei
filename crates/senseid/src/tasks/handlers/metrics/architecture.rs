//! `architecture` metric group — how the code is SHAPED, not how it is written.
//!
//! The `quality` family measures hygiene: duplication, smells, churn, coverage.
//! Nothing measured structure, even though the graph holds 4,078,536 edges and
//! 1,182,758 nodes and, as of 2026-09-28, exactly zero metrics read either
//! (#156). This is the first group that does.
//!
//! ## Tier 1 only, and that boundary is measured rather than cautious
//!
//! The obvious architecture metrics — coupling, instability, dependency cycles —
//! are all MODULE-level, and #156 measured what the module graph currently is:
//! collapsed to distinct module→module pairs, this whole repository yields
//! **13 edges**, two of them in one cycle, and all 13 TypeScript. Rust
//! contributes none (#146, #151, #152).
//!
//! A coupling number over 13 edges is a confident number over nothing, which is
//! the fabricated-signal failure the no-fabrication rule forbids. So the module
//! metrics are NOT here; they are blocked on the resolver, not on metric design.
//! The three below need no resolved module graph at all.
//!
//! ## `graph_confidence` gates the other two, and eventually the rest
//!
//! It is not a code-quality signal. It says how much of the graph any structural
//! metric could see — 40.4% of edges were placed when this was written. Read a
//! coupling number without it and you are reading two fifths of a codebase while
//! believing you read all of it.
//!
//! ## Honest-empty everywhere
//!
//! A repository with no edges writes NO `graph_confidence` row rather than 0.0,
//! because a zero would read as "nothing resolves" when the truth is "nothing was
//! indexed". Same for the other two. The reads enforce it — see
//! [`crate::db::pg_store::metric_reads::architecture`], where each query's guard
//! lives.

use crate::tasks::executor::TaskContext;

use super::MetricGroup;
use crate::db::pg_store::MetricRow;

const GRAIN_DAILY: &str = "daily";
const SOURCE_MEASURED: &str = "measured";
const SCOPE_REPO: &str = "repo";
const KEY_GRAPH_CONFIDENCE: &str = "graph_confidence";
const KEY_PUBLIC_SURFACE: &str = "public_surface_ratio";
const KEY_SYMBOL_SIZE_P95: &str = "symbol_size_p95";

pub(super) async fn compute(
    ctx: &TaskContext,
    project_raw: &str,
    as_of: Option<chrono::NaiveDate>,
) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(project_raw)
        .map_err(|e| format!("architecture: bad project id {project_raw:?}: {e}"))?;
    let pg = ctx.pg();
    let day = as_of.unwrap_or_else(|| chrono::Utc::now().date_naive());

    let ids = pg.active_metric_ids(MetricGroup::Architecture.as_str()).await?;
    let mut written = 0u32;

    if let Some(mid) = ids.get(KEY_GRAPH_CONFIDENCE).copied() {
        for (repository_id, placed, total) in
            pg.architecture_edge_confidence_by_repo(&project_id).await?
        {
            // `total > 0` is guaranteed by the HAVING, but the division is
            // spelled defensively anyway: a query edited later must not be able
            // to turn this into a NaN that serialises as `null` and reads as
            // "not measured".
            if total == 0 {
                continue;
            }
            let props = serde_json::json!({ "placed": placed, "total": total });
            pg.upsert_project_metric_repo(&MetricRow {
                metric_id: &mid,
                repository_id: &repository_id,
                scope: SCOPE_REPO,
                identity: None,
                commit_sha: None,
                computed_on: day,
                grain: GRAIN_DAILY,
                value: placed as f64 / total as f64,
                props: &props,
                source: SOURCE_MEASURED,
            })
            .await?;
            written += 1;
        }
    }

    if let Some(mid) = ids.get(KEY_PUBLIC_SURFACE).copied() {
        for (repository_id, exported, total) in
            pg.architecture_public_surface_by_repo(&project_id).await?
        {
            if total == 0 {
                continue;
            }
            let props = serde_json::json!({ "exported": exported, "declared": total });
            pg.upsert_project_metric_repo(&MetricRow {
                metric_id: &mid,
                repository_id: &repository_id,
                scope: SCOPE_REPO,
                identity: None,
                commit_sha: None,
                computed_on: day,
                grain: GRAIN_DAILY,
                value: exported as f64 / total as f64,
                props: &props,
                source: SOURCE_MEASURED,
            })
            .await?;
            written += 1;
        }
    }

    if let Some(mid) = ids.get(KEY_SYMBOL_SIZE_P95).copied() {
        for (repository_id, p95, over_100, callables) in
            pg.architecture_symbol_size_by_repo(&project_id).await?
        {
            let props = serde_json::json!({ "over_100_lines": over_100, "callables": callables });
            pg.upsert_project_metric_repo(&MetricRow {
                metric_id: &mid,
                repository_id: &repository_id,
                scope: SCOPE_REPO,
                identity: None,
                commit_sha: None,
                computed_on: day,
                grain: GRAIN_DAILY,
                value: p95,
                props: &props,
                source: SOURCE_MEASURED,
            })
            .await?;
            written += 1;
        }
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_bad_project_id_is_an_error_not_a_silent_zero() {
        let ctx = crate::tasks::test_support::make_ctx().await;
        assert!(compute(&ctx, "not-a-uuid", None).await.is_err());
    }

    /// A project with nothing indexed writes NO row.
    ///
    /// Honest-empty rather than 0.0: a zero `graph_confidence` reads as "nothing
    /// in this repository resolves", which is a damning and false statement about
    /// a repository that simply has not been walked yet.
    #[tokio::test]
    async fn an_unindexed_project_writes_no_row_rather_than_a_zero() {
        let ctx = crate::tasks::test_support::make_ctx().await;
        let pid = uuid::Uuid::new_v4();
        assert_eq!(compute(&ctx, &pid.to_string(), None).await.unwrap(), 0);
    }

    /// The whole chain, on data with a KNOWN answer.
    ///
    /// Four edges, two of them placed, so `graph_confidence` must be exactly 0.5 —
    /// a value picked because it cannot be produced by accident. Three
    /// declarations, one exported, so the surface ratio is 1/3. The two tests
    /// above prove the error and empty paths; only this one proves the arithmetic,
    /// and it reads the value back out of `project_metrics` rather than trusting
    /// the return count, because a handler that writes the wrong number still
    /// returns a healthy-looking row count.
    ///
    /// Mutations that must break it: swap the numerator and denominator in any of
    /// the three queries, or drop the `file_id IS NOT NULL` filter, which would
    /// pull reference stubs into a ratio about what the repository OWNS.
    #[tokio::test]
    async fn it_writes_the_value_the_data_implies() {
        let ctx = crate::tasks::test_support::make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = crate::tasks::test_support::seed_metrics_project_folder(pg, &uniq).await;
        // `seed_metrics_project_folder` already wires the folder to a repository;
        // this only reads it back, so nodes seeded into `fid` roll up to `rid`.
        let rid = crate::tasks::test_support::repository_for_folder(pg, &fid).await;

        // Three declarations, one exported. Spans chosen so p95 is not degenerate.
        let a = crate::tasks::test_support::seed_node(
            pg,
            &fid,
            "function",
            "a",
            "src/a.rs",
            None,
            None,
            Some(1),
            Some(10),
        )
        .await
        .unwrap();
        let b = crate::tasks::test_support::seed_node(
            pg,
            &fid,
            "function",
            "b",
            "src/b.rs",
            None,
            None,
            Some(1),
            Some(20),
        )
        .await
        .unwrap();
        let c = crate::tasks::test_support::seed_node(
            pg,
            &fid,
            "function",
            "c",
            "src/c.rs",
            None,
            None,
            Some(1),
            Some(30),
        )
        .await
        .unwrap();
        sqlx_core::query::query("UPDATE sensei.nodes SET is_exported = true WHERE id = $1")
            .bind(a)
            .execute(pg.pool())
            .await
            .unwrap();

        // A REFERENCE STUB: referenced somewhere, declared nowhere, so it carries
        // no file. It must NOT enter the denominator of a ratio about what this
        // repository OWNS — that is the entire job of the `file_id IS NOT NULL`
        // filter. Without a stub in the fixture, deleting that filter changed
        // nothing and the test passed, so the guard was imaginary.
        sqlx_core::query::query(
            "INSERT INTO sensei.nodes(folder_id, kind, name) \
             VALUES($1, 'function'::sensei.node_kind, 'stub_never_declared')",
        )
        .bind(fid)
        .execute(pg.pool())
        .await
        .unwrap();

        // Two placed, two unplaced → confidence is exactly 0.5.
        pg.insert_edge(&fid, &a, Some(&b), None, None, "calls").await.unwrap();
        pg.insert_edge(&fid, &b, Some(&c), None, None, "calls").await.unwrap();
        pg.insert_edge(&fid, &a, None, Some("gone_one"), None, "calls").await.unwrap();
        pg.insert_edge(&fid, &b, None, Some("gone_two"), None, "calls").await.unwrap();

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert!(written >= 3, "one row per architecture metric, got {written}");

        let confidence: f64 = sqlx_core::query_scalar::query_scalar(
            "SELECT pm.value::float8 FROM sensei.project_metrics pm \
               JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE m.key = 'graph_confidence' AND pm.repository_id = $1",
        )
        .bind(rid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert!((confidence - 0.5).abs() < 1e-9, "2 placed of 4 edges is 0.5, got {confidence}");

        let surface: f64 = sqlx_core::query_scalar::query_scalar(
            "SELECT pm.value::float8 FROM sensei.project_metrics pm \
               JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE m.key = 'public_surface_ratio' AND pm.repository_id = $1",
        )
        .bind(rid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert!(
            (surface - 1.0 / 3.0).abs() < 1e-9,
            "1 exported of 3 declarations is 1/3, got {surface}"
        );

        pg.delete_nodes_by_folder(&fid).await.unwrap();
    }
}
