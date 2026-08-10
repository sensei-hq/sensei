//! `duplication` metric group computer (Phase 5.3).
//!
//! The THIRD real base-metric computer; it follows the `session_outcomes` /
//! `churn` template (resolve `key → metric_id` via the active registry, write
//! per-module + project rows to `sensei.project_metrics` via
//! [`PgStore::upsert_project_metric`]) for ONE project.
//!
//! v1 registry key (`task_name = "duplication"`):
//! - `duplication_ratio` (ratio, per-module + project): a POINT-IN-TIME SNAPSHOT of
//!   how much of the project's symbol surface is near-duplicated.
//!   `numerator` = # eligible symbols that participate in ≥1 duplicate cluster;
//!   `denominator` = # eligible symbols; `value` = num/den. Written per-module
//!   (grain `daily`, `folder_id` set) for each folder with eligible symbols, plus a
//!   project daily row (grain `daily`, `folder_id` NULL) = the whole-project figure.
//!
//! ## Snapshot semantics (why "eligible symbols", not "new symbols")
//! The catalog's original framing was "new symbols matching an existing symbol ÷ new
//! symbols" — a temporal (write-time) measure. `sensei.nodes` has NO reliable
//! creation timestamp: `modified_at` is bumped on every rescan (see the
//! `upsert_node_ex` upsert), so it cannot distinguish a freshly-created symbol from
//! an old one re-indexed by the version-rescan pass. Rather than mismeasure on a
//! `modified_at` proxy, v1 ships this as a SNAPSHOT over the current graph; the
//! temporal "new-duplicate" version is a deferred follow-up (see `docs/backlog.md`)
//! that needs an immutable `nodes.created_at` or a write-time computation.
//!
//! ## Detection reuse (DRY)
//! Duplicate detection is NOT reimplemented here. [`PgStore::duplication_stats_scoped`]
//! runs the SAME eligibility (`function`/`method` nodes with an embedding spanning
//! ≥3 lines) and the SAME cosine threshold ([`DUP_MIN_SIMILARITY`], the
//! `find_duplicates` default) as the on-demand duplicate finder, but returns the
//! per-folder eligible + duplicate-participant COUNTS the ratio needs (a node id +
//! folder is required for per-module aggregation, which the display-oriented
//! `find_duplicates_scoped` JSON does not carry). The project scope is
//! `folders.project_id` — the same linkage `churn` uses.
//!
//! Never-fabricate: every DB call propagates `Err`; a project/module with no
//! eligible symbols writes NO row (a 0/0 would be a fabricated zero). A real
//! denominator with 0 duplicate participants writes a real `0.0` (row written).

use crate::tasks::executor::TaskContext;

use super::MetricGroup;

/// `sensei.metric_grain` text value (duplication writes daily snapshot rows only).
const GRAIN_DAILY: &str = "daily";
/// `sensei.metric_source` text value — duplication is measured, not estimated.
const SOURCE_MEASURED: &str = "measured";

/// The registry `key` this computer produces.
const KEY_DUPLICATION_RATIO: &str = "duplication_ratio";

/// Cosine-similarity threshold for a "duplicate" — a pair at or above this counts.
/// `0.92` is the `find_duplicates` handler default, kept identical so the metric and
/// the on-demand review surface report the same clusters.
const DUP_MIN_SIMILARITY: f64 = 0.92;

/// Compute the `duplication` group for one project as a snapshot as of today.
/// `project_raw` is the project uuid carried in `task.folder_path`. Returns the
/// number of `project_metrics` rows written (`0` = honest-empty: no eligible
/// symbols, or the metric is inactive). Idempotent — re-running backfills in place
/// via the upsert identity.
pub(super) async fn compute(ctx: &TaskContext, project_raw: &str) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(project_raw)
        .map_err(|e| format!("duplication: bad project id {project_raw:?}: {e}"))?;
    let pg = ctx.pg();

    // Resolve key → metric_id for this group's ACTIVE metrics. An absent key is
    // inactive (retired / not-yet-effective / unseeded) → skipped: the computer
    // never writes a value for an inactive metric.
    let ids = pg.active_metric_ids(MetricGroup::Duplication.as_str()).await?;
    let Some(dup_id) = ids.get(KEY_DUPLICATION_RATIO).copied() else {
        return Ok(0);
    };

    // Per-folder (eligible, duplicate-participant) counts — detection + eligibility
    // reused from the shared store helper (DRY), scoped to this project's folders.
    let stats = pg.duplication_stats_scoped(&project_id, DUP_MIN_SIMILARITY).await?;
    let project_eligible: i64 = stats.iter().map(|(_, eligible, _)| eligible).sum();
    if project_eligible == 0 {
        // No eligible symbols → no denominator → NO row (a 0/0 would be a fabricated
        // zero, not a measured one).
        return Ok(0);
    }
    let project_dup: i64 = stats.iter().map(|(_, _, dup)| dup).sum();
    let day = super::today(pg).await?;

    let mut written = 0u32;

    // Project row: duplicate-participating symbols ÷ eligible symbols, project-wide.
    // denominator is > 0 here (guarded above). 0 duplicates over a real denominator
    // → a real 0.0 (row written, never suppressed).
    {
        let value = project_dup as f64 / project_eligible as f64;
        let props = serde_json::json!({ "numerator": project_dup, "denominator": project_eligible });
        pg.upsert_project_metric(
            &dup_id, &project_id, None, None, day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
        )
        .await?;
        written += 1;
    }

    // Per-module rows: one per folder that has eligible symbols. `duplication_stats_scoped`
    // only returns folders with ≥1 eligible symbol, so the denominator is always > 0.
    for (folder_id, eligible, dup) in &stats {
        if *eligible == 0 {
            continue; // defensive: no denominator → no row (the query won't emit this).
        }
        let value = *dup as f64 / *eligible as f64;
        let props = serde_json::json!({ "numerator": dup, "denominator": eligible });
        pg.upsert_project_metric(
            &dup_id, &project_id, Some(folder_id), None, day, GRAIN_DAILY, value, &props,
            SOURCE_MEASURED,
        )
        .await?;
        written += 1;
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::test_support::{
        cleanup_metrics_fixture, daily_project_metric_rows as daily_rows, make_ctx,
        module_metric_rows as module_rows, onehot_embedding_sql, seed_metrics_folder,
        seed_metrics_project_folder, seed_symbol_node, uniform_embedding_sql,
    };
    use sqlx_core::query_as::query_as;

    #[tokio::test]
    async fn duplication_ratio_snapshot_from_clusters() {
        // 5 eligible symbols; exactly 2 (identical embeddings → cosine 1.0) form a
        // near-duplicate pair; the other 3 are pairwise-orthogonal one-hots (cosine 0)
        // → not duplicates. Ratio = 2 participants / 5 eligible = 0.4, with exact
        // props, on BOTH the module row and the project row.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let base = format!("/_test/metrics-{uniq}");
        let dup = uniform_embedding_sql("0.1"); // identical vector → the duplicate pair

        seed_symbol_node(pg, &fid, "function", "dup_a", &format!("{base}/a.rs"), (1, 10), Some(&dup)).await;
        seed_symbol_node(pg, &fid, "function", "dup_b", &format!("{base}/b.rs"), (1, 10), Some(&dup)).await;
        for (i, name) in ["u1", "u2", "u3"].iter().enumerate() {
            let emb = onehot_embedding_sql((i + 1) as u32);
            seed_symbol_node(pg, &fid, "function", name, &format!("{base}/{name}.rs"), (1, 10), Some(&emb)).await;
        }

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 2, "one project row + one per-module row (single folder)");

        // ── Project row: 2/5 = 0.4 ──
        let daily = daily_rows(pg, &pid).await;
        let dr = daily.iter().find(|r| r.0 == "duplication_ratio").expect("duplication_ratio project row");
        assert!((dr.1 - 0.4).abs() < 1e-9, "value = 2 duplicate-participating / 5 eligible = 0.4");
        assert_eq!(dr.2["numerator"].as_i64(), Some(2), "numerator = # symbols in a duplicate cluster");
        assert_eq!(dr.2["denominator"].as_i64(), Some(5), "denominator = # eligible symbols");
        // Invariant: participants are a SUBSET of eligible symbols → numerator ≤ denominator.
        assert!(
            dr.2["numerator"].as_i64().unwrap() <= dr.2["denominator"].as_i64().unwrap(),
            "numerator (duplicate participants) must never exceed denominator (eligible symbols)",
        );

        // ── Per-module row (single folder) mirrors the project figure ──
        let modules = module_rows(pg, &pid, "duplication_ratio").await;
        assert_eq!(modules.len(), 1, "one per-module row (the fixture folder)");
        assert_eq!(modules[0].0, fid, "the module row is attributed to the folder");
        assert!((modules[0].1 - 0.4).abs() < 1e-9, "module ratio = 2/5 = 0.4");
        assert_eq!(modules[0].2["numerator"].as_i64(), Some(2), "module numerator = folder duplicate participants");
        assert_eq!(modules[0].2["denominator"].as_i64(), Some(5), "module denominator = folder eligible symbols");

        // ── Idempotency: re-run backfills in place, never duplicates ──
        let again = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(again, 2, "re-run recomputes the same rows");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 2, "idempotent upsert — still 2 rows after a second run");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn duplication_no_eligible_symbols_writes_zero_rows() {
        // Never-fabricate: a project with NO eligible symbols writes NO rows. Seeds an
        // ineligible function (embedding present but span < 3) and one with a NULL
        // embedding — both excluded — so the eligibility filter (not just "no nodes")
        // is what yields zero.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let base = format!("/_test/metrics-{uniq}");
        let emb = uniform_embedding_sql("0.1");
        // Embedded but span 1..2 (< 3 lines) → ineligible.
        seed_symbol_node(pg, &fid, "function", "too_short", &format!("{base}/s.rs"), (1, 2), Some(&emb)).await;
        // Eligible span but NO embedding → ineligible.
        seed_symbol_node(pg, &fid, "function", "no_embed", &format!("{base}/n.rs"), (1, 10), None).await;

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 0, "no eligible symbols → no denominator → zero rows written");

        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no project_metrics rows when nothing is eligible (never fabricated)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn duplication_real_zero() {
        // Eligible symbols but ZERO duplicates → a row IS written with value 0.0 /
        // numerator 0 (a real zero over a real denominator), never suppressed.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let base = format!("/_test/metrics-{uniq}");
        // 4 pairwise-orthogonal one-hots → no pair reaches 0.92 → 0 duplicates.
        for (i, name) in ["u1", "u2", "u3", "u4"].iter().enumerate() {
            let emb = onehot_embedding_sql((i + 1) as u32);
            seed_symbol_node(pg, &fid, "function", name, &format!("{base}/{name}.rs"), (1, 10), Some(&emb)).await;
        }

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 2, "project row + one per-module row (a real 0.0 is still written)");

        let daily = daily_rows(pg, &pid).await;
        let dr = daily.iter().find(|r| r.0 == "duplication_ratio").expect("duplication_ratio row IS written for a REAL zero");
        assert!(dr.1.abs() < 1e-9, "value is a real 0.0 (0 duplicates over 4 eligible), not a suppressed row");
        assert_eq!(dr.2["numerator"].as_i64(), Some(0), "numerator = 0 (no duplicate clusters)");
        assert_eq!(dr.2["denominator"].as_i64(), Some(4), "denominator = 4 eligible symbols (real denominator → row written)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn duplication_excludes_other_projects() {
        // Cross-project isolation: a SECOND project's duplicate symbols — sharing the
        // SAME embedding as A's — must NOT leak into A's ratio. Mutation-proof: if the
        // scope ignored project_id, B's uniform-0.1 nodes would cluster with A's
        // (all cosine 1.0), pushing A's ratio to 5/7 instead of 2/4.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq_a = uuid::Uuid::new_v4();
        let uniq_b = uuid::Uuid::new_v4();
        let (pid_a, fid_a) = seed_metrics_project_folder(pg, &uniq_a).await;
        let (pid_b, fid_b) = seed_metrics_project_folder(pg, &uniq_b).await; // a real SECOND project
        let base_a = format!("/_test/metrics-{uniq_a}");
        let base_b = format!("/_test/metrics-{uniq_b}");
        let dup = uniform_embedding_sql("0.1"); // the SAME vector used in both projects

        // Project A: 2 duplicates (uniform-0.1) + 2 dissimilar one-hots → 2/4 = 0.5.
        seed_symbol_node(pg, &fid_a, "function", "a_dup1", &format!("{base_a}/d1.rs"), (1, 10), Some(&dup)).await;
        seed_symbol_node(pg, &fid_a, "function", "a_dup2", &format!("{base_a}/d2.rs"), (1, 10), Some(&dup)).await;
        for (i, name) in ["a_u1", "a_u2"].iter().enumerate() {
            let emb = onehot_embedding_sql((i + 1) as u32);
            seed_symbol_node(pg, &fid_a, "function", name, &format!("{base_a}/{name}.rs"), (1, 10), Some(&emb)).await;
        }
        // Project B: 3 duplicates sharing A's embedding — must NOT touch A's aggregate.
        for name in ["b_dup1", "b_dup2", "b_dup3"] {
            seed_symbol_node(pg, &fid_b, "function", name, &format!("{base_b}/{name}.rs"), (1, 10), Some(&dup)).await;
        }

        let written = compute(&ctx, &pid_a.to_string()).await.unwrap();
        assert_eq!(written, 2, "only A's own folder produces rows (project + one module)");

        let daily = daily_rows(pg, &pid_a).await;
        let dr = daily.iter().find(|r| r.0 == "duplication_ratio").expect("A's duplication_ratio project row");
        assert!((dr.1 - 0.5).abs() < 1e-9, "A's ratio = 2/4 = 0.5 (B's shared-embedding dupes excluded)");
        assert_eq!(dr.2["numerator"].as_i64(), Some(2), "A's numerator = 2 (B's 3 excluded — not 5)");
        assert_eq!(dr.2["denominator"].as_i64(), Some(4), "A's denominator = 4 (B's 3 excluded — not 7)");

        let modules = module_rows(pg, &pid_a, "duplication_ratio").await;
        assert_eq!(modules.len(), 1, "exactly one module row — A's folder only, never B's");
        assert_eq!(modules[0].0, fid_a, "the module row is A's folder");
        assert!(!modules.iter().any(|(id, _, _)| *id == fid_b), "B's folder never appears in A's rows");

        cleanup_metrics_fixture(pg, &pid_a, Some(&fid_a), &[]).await;
        cleanup_metrics_fixture(pg, &pid_b, Some(&fid_b), &[]).await;
    }

    #[tokio::test]
    async fn duplication_distinct_counts_participants_not_pairs() {
        // A cluster of 3 MUTUALLY-similar symbols (all pairwise cosine 1.0) forms 3
        // ordered pairs / 6 directed pairs — but only 3 DISTINCT participants. The
        // ratio counts participants, not pairs: numerator = 3 (NOT 6/9), denominator
        // = 3, value = 1.0. This is the case a single 2-symbol pair can't pin: drop
        // `DISTINCT` from the `dup` CTE and this test goes red (numerator → 6).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let base = format!("/_test/metrics-{uniq}");
        let dup = uniform_embedding_sql("0.1"); // identical vector shared by all three

        for name in ["c1", "c2", "c3"] {
            seed_symbol_node(pg, &fid, "function", name, &format!("{base}/{name}.rs"), (1, 10), Some(&dup)).await;
        }

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 2, "project row + one per-module row");

        let daily = daily_rows(pg, &pid).await;
        let dr = daily.iter().find(|r| r.0 == "duplication_ratio").expect("duplication_ratio project row");
        let num = dr.2["numerator"].as_i64().unwrap();
        let den = dr.2["denominator"].as_i64().unwrap();
        assert_eq!(num, 3, "numerator = 3 DISTINCT participants (NOT 6 directed pairs — the DISTINCT proof)");
        assert_eq!(den, 3, "denominator = 3 eligible symbols");
        assert!(num <= den, "numerator (participants) must never exceed denominator (eligible)");
        assert!((dr.1 - 1.0).abs() < 1e-9, "value = 3/3 = 1.0 (the whole cluster is duplicated)");

        // The single per-module row matches (same folder).
        let modules = module_rows(pg, &pid, "duplication_ratio").await;
        assert_eq!(modules.len(), 1, "one per-module row");
        assert_eq!(modules[0].2["numerator"].as_i64(), Some(3), "module numerator = 3 distinct participants");
        assert_eq!(modules[0].2["denominator"].as_i64(), Some(3), "module denominator = 3 eligible");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn duplication_line_span_boundary() {
        // Eligibility requires `line_end - line_start >= 3`. Pin the cliff exactly: a
        // node with span 2 (line 1..3) is EXCLUDED; a node with span 3 (line 1..4) is
        // INCLUDED. Distinct one-hot embeddings so neither is a duplicate. Denominator
        // must be 1 (the span-3 node only): a `>= 2` mutation would give 2; a `>= 4`
        // mutation would give 0 (no row).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let base = format!("/_test/metrics-{uniq}");
        let below = onehot_embedding_sql(1);
        let at = onehot_embedding_sql(2);
        // span 2 (3-1) → below the >=3 cliff → excluded.
        seed_symbol_node(pg, &fid, "function", "span2", &format!("{base}/span2.rs"), (1, 3), Some(&below)).await;
        // span 3 (4-1) → exactly at the cliff → included.
        seed_symbol_node(pg, &fid, "function", "span3", &format!("{base}/span3.rs"), (1, 4), Some(&at)).await;

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 2, "a row IS written (the span-3 node is eligible → real denominator)");

        let daily = daily_rows(pg, &pid).await;
        let dr = daily.iter().find(|r| r.0 == "duplication_ratio").expect("duplication_ratio project row");
        assert_eq!(dr.2["denominator"].as_i64(), Some(1), "denominator = 1 (only the span==3 node; span==2 excluded)");
        assert_eq!(dr.2["numerator"].as_i64(), Some(0), "numerator = 0 (the one eligible node has no duplicate)");
        assert!(dr.1.abs() < 1e-9, "value = 0/1 = 0.0");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn duplication_cross_folder_attribution() {
        // A symbol whose duplicate partner lives in ANOTHER folder still counts as a
        // participant in ITS OWN folder. Two folders in the SAME project, each with one
        // eligible symbol; the two symbols are near-duplicates of each other (shared
        // embedding). Each module row counts its own symbol (numerator 1, denominator
        // 1); the project row is the union (numerator 2, denominator 2, value 1.0).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq_x = uuid::Uuid::new_v4();
        let uniq_y = uuid::Uuid::new_v4();
        let (pid, fid_x) = seed_metrics_project_folder(pg, &uniq_x).await;
        let fid_y = seed_metrics_folder(pg, &pid, &uniq_y).await; // second folder, SAME project
        let base_x = format!("/_test/metrics-{uniq_x}");
        let base_y = format!("/_test/metrics-{uniq_y}");
        let dup = uniform_embedding_sql("0.1"); // the shared, cross-folder duplicate

        seed_symbol_node(pg, &fid_x, "function", "x_sym", &format!("{base_x}/x.rs"), (1, 10), Some(&dup)).await;
        seed_symbol_node(pg, &fid_y, "function", "y_sym", &format!("{base_y}/y.rs"), (1, 10), Some(&dup)).await;

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 3, "project row + one module row per folder (X and Y)");

        // ── Each module counts ITS OWN symbol as a participant ──
        let modules = module_rows(pg, &pid, "duplication_ratio").await;
        assert_eq!(modules.len(), 2, "two per-module rows (one per folder)");
        let mx = modules.iter().find(|(id, _, _)| *id == fid_x).expect("folder X module row");
        let my = modules.iter().find(|(id, _, _)| *id == fid_y).expect("folder Y module row");
        assert_eq!(mx.2["numerator"].as_i64(), Some(1), "X: its own symbol is a participant (partner is in Y)");
        assert_eq!(mx.2["denominator"].as_i64(), Some(1), "X: 1 eligible symbol");
        assert_eq!(my.2["numerator"].as_i64(), Some(1), "Y: its own symbol is a participant (partner is in X)");
        assert_eq!(my.2["denominator"].as_i64(), Some(1), "Y: 1 eligible symbol");

        // ── Project row = union across folders ──
        let daily = daily_rows(pg, &pid).await;
        let dr = daily.iter().find(|r| r.0 == "duplication_ratio").expect("project row");
        assert_eq!(dr.2["numerator"].as_i64(), Some(2), "project numerator = Σ folder participants = 2");
        assert_eq!(dr.2["denominator"].as_i64(), Some(2), "project denominator = Σ folder eligible = 2");
        assert!((dr.1 - 1.0).abs() < 1e-9, "project value = 2/2 = 1.0");

        cleanup_metrics_fixture(pg, &pid, Some(&fid_x), &[]).await;
        // cleanup_metrics_fixture deletes only one folder; remove the second explicitly
        // (its nodes cascade on the folder delete).
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
            .bind(fid_y)
            .execute(pg.pool())
            .await
            .unwrap();
    }
}
