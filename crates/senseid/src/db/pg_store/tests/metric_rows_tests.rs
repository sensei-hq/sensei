use super::*;

// ── Metrics: value store + active registry (Phase 3) ──────────────────

/// Seed a `sensei.metrics` registry row for a test. Dates are relative to the
/// DB's `current_date` (via `current_date + <offset> days`) so the active-window
/// tests don't flake at a local midnight boundary. `until_offset = None` leaves
/// `effective_until` NULL (never retired). `name` is set to `key` so facet
/// assertions have a known value.
pub(super) async fn seed_metric(
    s: &PgStore,
    key: &str,
    task_name: &str,
    from_offset: i32,
    until_offset: Option<i32>,
) -> uuid::Uuid {
    let row: (uuid::Uuid,) = query_as(
            "INSERT INTO sensei.metrics
                (key, name, description, family, type, direction, purpose, how_to_read, formula,
                 task_name, effective_from, effective_until)
             VALUES ($1, $1, 'test metric', 'quality'::sensei.metric_family, 'ratio'::sensei.metric_type,
                     'higher_better'::sensei.metric_direction, 'test purpose', 'test how', 'test formula',
                     $2, current_date + $3::int, current_date + $4::int)
             RETURNING id",
        )
        .bind(key).bind(task_name).bind(from_offset).bind(until_offset)
        .fetch_one(s.pool()).await.unwrap();
    row.0
}

#[tokio::test]
async fn upsert_workspace_metric_is_idempotent() {
    // Two upserts of the same identity (metric x workspace x null folder x null
    // session x date x daily) collapse to ONE row: the second updates value,
    // props, source and bumps modified_at rather than duplicating.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let pid = s.create_workspace(&format!("_test:pm-idem:{uniq}"), None, None).await.unwrap();
    let rid =
        crate::tasks::test_support::seed_bare_repository(&s, &pid, &uuid::Uuid::new_v4()).await;
    let mid = seed_metric(&s, &format!("_test:pm-idem:{uniq}:ftr"), "ComputeFtr", 0, None).await;
    let day = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();

    let id1 = s
        .upsert_workspace_metric_repo(&MetricRow {
            metric_id: &mid,
            repository_id: &rid,
            scope: "user",
            identity: None,
            commit_sha: None,
            computed_on: day,
            grain: "daily",
            value: 0.5,
            props: &serde_json::json!({"numerator": 1, "denominator": 2}),
            source: "measured",
        })
        .await
        .unwrap();

    // Backdate modified_at so the second upsert's bump is observable.
    sqlx_core::query::query(
        "UPDATE sensei.workspace_metrics SET modified_at = now() - interval '1 hour' WHERE id = $1",
    )
    .bind(id1)
    .execute(s.pool())
    .await
    .unwrap();
    let (before,): (chrono::DateTime<chrono::Utc>,) =
        query_as("SELECT modified_at FROM sensei.workspace_metrics WHERE id = $1")
            .bind(id1)
            .fetch_one(s.pool())
            .await
            .unwrap();

    let id2 = s
        .upsert_workspace_metric_repo(&MetricRow {
            metric_id: &mid,
            repository_id: &rid,
            scope: "user",
            identity: None,
            commit_sha: None,
            computed_on: day,
            grain: "daily",
            value: 0.75,
            props: &serde_json::json!({"numerator": 3, "denominator": 4}),
            source: "estimated",
        })
        .await
        .unwrap();
    assert_eq!(id1, id2, "same identity upserts the same row (no duplicate)");

    let (n,): (i64,) = query_as(
        "SELECT count(*) FROM sensei.workspace_metrics
              WHERE metric_id = $1 AND workspace_id = $2 AND computed_on = $3 AND grain = 'daily'",
    )
    .bind(mid)
    .bind(pid)
    .bind(day)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(n, 1, "one row per identity — the second upsert updated in place");

    let (value, props, source, after): (f64, serde_json::Value, String, chrono::DateTime<chrono::Utc>) =
            query_as("SELECT value::float8, props, source::text, modified_at FROM sensei.workspace_metrics WHERE id = $1")
                .bind(id1).fetch_one(s.pool()).await.unwrap();
    assert_eq!(value, 0.75, "value updated to the second upsert's");
    assert_eq!(props, serde_json::json!({"numerator": 3, "denominator": 4}), "props updated");
    assert_eq!(source, "estimated", "source updated");
    assert!(after > before, "modified_at bumped past the backdated value");

    // cleanup — workspace_metrics rows cascade from the metric + workspace.
    sqlx_core::query::query("DELETE FROM sensei.metrics WHERE id = $1")
        .bind(mid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn active_metrics_excludes_retired_and_future() {
    // active_metrics() returns only rows live on current_date: the retired
    // (past effective_until) and not-yet-effective (future effective_from) rows
    // are excluded. Assertions are key-specific so the pre-seeded registry and
    // concurrent tests don't interfere.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let active_key = format!("_test:active:{uniq}");
    let retired_key = format!("_test:retired:{uniq}");
    let future_key = format!("_test:future:{uniq}");
    let today_retire_key = format!("_test:today-retire:{uniq}");
    let active_task = format!("ComputeActive_{uniq}");
    let retired_task = format!("ComputeRetired_{uniq}");
    let future_task = format!("ComputeFuture_{uniq}");
    let today_retire_task = format!("ComputeTodayRetire_{uniq}");
    seed_metric(&s, &active_key, &active_task, 0, None).await; // from today, no end
    seed_metric(&s, &retired_key, &retired_task, -10, Some(-1)).await; // ended yesterday
    seed_metric(&s, &future_key, &future_task, 1, None).await; // effective tomorrow
    // Retired EFFECTIVE TODAY: effective_until = current_date. The window is
    // half-open [from, until), so `until > current_date` is false today — this
    // row must already be inactive (locks the exclusive-upper-bound boundary).
    seed_metric(&s, &today_retire_key, &today_retire_task, -10, Some(0)).await;

    let metrics = s.active_metrics().await.unwrap();
    let keys: Vec<&str> = metrics.iter().map(|m| m.key.as_str()).collect();
    assert!(keys.contains(&active_key.as_str()), "active metric is returned");
    assert!(!keys.contains(&retired_key.as_str()), "retired metric is excluded");
    assert!(!keys.contains(&future_key.as_str()), "not-yet-effective metric is excluded");
    assert!(
        !keys.contains(&today_retire_key.as_str()),
        "a metric retired effective today is excluded (effective_until is exclusive)"
    );

    let tasks = s.active_task_names().await.unwrap();
    assert!(tasks.contains(&active_task), "active metric's task_name is present");
    assert!(!tasks.contains(&retired_task), "retired metric's task_name is absent");
    assert!(!tasks.contains(&future_task), "future metric's task_name is absent");
    assert!(
        !tasks.contains(&today_retire_task),
        "task_name of a metric retired effective today is not scheduled"
    );

    // The mapped Metric carries the facets/knobs.
    let active = metrics.iter().find(|m| m.key == active_key).unwrap();
    assert_eq!(active.family, "quality");
    assert_eq!(active.metric_type, "ratio");
    assert_eq!(active.direction, "higher_better");
    assert_eq!(active.weight, 1.0, "numeric weight defaults to 1");
    assert!(active.effective_until.is_none(), "active metric has no end date");

    sqlx_core::query::query("DELETE FROM sensei.metrics WHERE key IN ($1, $2, $3, $4)")
        .bind(&active_key)
        .bind(&retired_key)
        .bind(&future_key)
        .bind(&today_retire_key)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn active_metric_ids_maps_active_keys_for_task_name_only() {
    // active_metric_ids(task) returns key→id for ONLY the active metrics whose
    // task_name matches: a same-task retired metric and a different-task metric
    // are both absent, so a compute handler's `ids.get(key)` skips them.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let task = format!("ComputeSO_{uniq}"); // unique → no pre-seeded rows share it
    let other_task = format!("ComputeOther_{uniq}");
    let k_a = format!("_test:ami:{uniq}:a");
    let k_b = format!("_test:ami:{uniq}:b");
    let k_other = format!("_test:ami:{uniq}:other");
    let k_retired = format!("_test:ami:{uniq}:retired");
    let id_a = seed_metric(&s, &k_a, &task, 0, None).await; // active, our task
    let id_b = seed_metric(&s, &k_b, &task, 0, None).await; // active, our task
    seed_metric(&s, &k_other, &other_task, 0, None).await; // active, DIFFERENT task
    seed_metric(&s, &k_retired, &task, -10, Some(-1)).await; // our task but RETIRED

    let ids = s.active_metric_ids(&task).await.unwrap();
    assert_eq!(ids.len(), 2, "only this task's two ACTIVE keys (task_name is unique to this test)");
    assert_eq!(ids.get(&k_a).copied(), Some(id_a), "active key → its metric_id");
    assert_eq!(ids.get(&k_b).copied(), Some(id_b));
    assert!(!ids.contains_key(&k_other), "a key with a different task_name is excluded");
    assert!(!ids.contains_key(&k_retired), "a retired (inactive) key is excluded (never computed)");

    sqlx_core::query::query("DELETE FROM sensei.metrics WHERE key IN ($1, $2, $3, $4)")
        .bind(&k_a)
        .bind(&k_b)
        .bind(&k_other)
        .bind(&k_retired)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn resolve_folder_from_path_uses_aliases() {
    // A live folders.abs_path resolves; a folder_path_aliases OLD path resolves
    // to the CURRENT folder + workspace; an unknown path is an honest None.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (pid, fid) = create_test_workspace_and_folder(&s, &format!("resolve-{uniq}")).await;
    let abs_path = format!("/_test/resolve-{uniq}"); // create_test_folder sets abs_path = /_test/{suffix}
    let alias = format!("/_test/old-resolve-{uniq}");
    s.add_folder_path_alias(&alias, &fid, "rename").await.unwrap();

    assert_eq!(
        s.resolve_folder_by_path(&abs_path).await.unwrap(),
        Some((fid, pid)),
        "a live folders.abs_path resolves to (folder_id, workspace_id)"
    );
    assert_eq!(
        s.resolve_folder_by_path(&alias).await.unwrap(),
        Some((fid, pid)),
        "a folder_path_aliases old path resolves to the current folder + workspace"
    );
    assert_eq!(
        s.resolve_folder_by_path(&format!("/_test/unknown-{uniq}")).await.unwrap(),
        None,
        "an unknown path resolves to None (never fabricated)"
    );

    // A folder with NO workspace attached (folders.workspace_id null — a real,
    // reachable state: create_test_folder does not wire a workspace) resolves to
    // None. This pins the never-fabricate contract: the impl must NOT invent a
    // workspace id (e.g. from the folder id) when the folder has no workspace.
    let noproj_fid = create_test_folder(&s, &format!("noproj-{uniq}")).await;
    let noproj_path = format!("/_test/noproj-{uniq}");
    assert_eq!(
        s.resolve_folder_by_path(&noproj_path).await.unwrap(),
        None,
        "a folder without a workspace resolves to None (never a fabricated workspace id)"
    );

    // cleanup — the alias cascades on folder delete.
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
        .bind(noproj_fid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
        .bind(fid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn get_workspace_metrics_reads_views() {
    // After upserting two daily rows on different dates, get_workspace_metrics
    // returns the LATEST-per-metric value + props with the catalog facets
    // (name/type/unit/direction/purpose/how_to_read) joined from sensei.metrics.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let pid = s.create_workspace(&format!("_test:gpm:{uniq}"), None, None).await.unwrap();
    let rid =
        crate::tasks::test_support::seed_bare_repository(&s, &pid, &uuid::Uuid::new_v4()).await;
    let key = format!("_test:gpm:{uniq}:cov");
    let mid = seed_metric(&s, &key, "ComputeCoverage", 0, None).await;
    let d1 = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let d2 = chrono::NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(); // later => latest

    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &mid,
        repository_id: &rid,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: d1,
        grain: "daily",
        value: 0.5,
        props: &serde_json::json!({"numerator": 1, "denominator": 2}),
        source: "measured",
    })
    .await
    .unwrap();
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &mid,
        repository_id: &rid,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: d2,
        grain: "daily",
        value: 0.75,
        props: &serde_json::json!({"numerator": 3, "denominator": 4}),
        source: "measured",
    })
    .await
    .unwrap();

    let rows = s.get_workspace_metrics(&pid).await.unwrap();
    let row = rows.iter().find(|r| r.metric == key).expect("our metric is present");
    assert_eq!(row.date, d2, "latest date per metric wins");
    assert_eq!(row.value, 0.75, "latest value");
    assert_eq!(
        row.props,
        serde_json::json!({"numerator": 3, "denominator": 4}),
        "props from the latest row"
    );
    assert_eq!(row.name, key, "name facet joined from sensei.metrics (seed sets name = key)");
    assert_eq!(row.metric_type, "ratio", "type facet");
    assert_eq!(row.direction, "higher_better", "direction facet");
    assert_eq!(row.purpose, "test purpose", "purpose facet");
    assert_eq!(row.how_to_read, "test how", "how_to_read facet");
    assert!(row.unit.is_none(), "seed leaves unit null");

    // cleanup — workspace_metrics rows cascade from the metric + workspace.
    sqlx_core::query::query("DELETE FROM sensei.metrics WHERE id = $1")
        .bind(mid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

/// The deactivation set is REPLACED whole, so a re-enabled metric stops being
/// skipped.
///
/// An incremental upsert would leave the old row behind and keep skipping a
/// metric the tenant had switched back on — silently, and forever, because
/// nothing else would ever delete it.
#[tokio::test]
async fn replacing_deactivations_forgets_the_ones_no_longer_listed() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let pid = s.create_workspace(&format!("_test:deact:{uniq}"), None, None).await.unwrap();
    let rid =
        crate::tasks::test_support::seed_bare_repository(&s, &pid, &uuid::Uuid::new_v4()).await;
    // The gate compares repo_key, so the row needs one.
    let key = format!("github.com/_test/deact-{uniq}");
    sqlx_core::query::query("UPDATE sensei.repositories SET repo_key = $2 WHERE id = $1")
        .bind(rid)
        .bind(&key)
        .execute(s.pool())
        .await
        .unwrap();

    let two = std::collections::HashMap::from([(
        key.clone(),
        vec!["churn_rate".to_string(), "churn_concentration".to_string()],
    )]);
    s.replace_metric_deactivations(&two).await.unwrap();
    let back = s.metric_deactivations().await.unwrap();
    let mut got = back.get(&key).cloned().unwrap_or_default();
    got.sort();
    assert_eq!(got, vec!["churn_concentration".to_string(), "churn_rate".to_string()]);

    // One re-enabled: the plan now lists only churn_rate.
    let one = std::collections::HashMap::from([(key.clone(), vec!["churn_rate".to_string()])]);
    s.replace_metric_deactivations(&one).await.unwrap();
    assert_eq!(
        s.metric_deactivations().await.unwrap().get(&key),
        Some(&vec!["churn_rate".to_string()]),
        "churn_concentration must be forgotten, not left behind"
    );

    // All re-enabled: an EMPTY map must clear the table, not be treated as "no
    // news". This is the case an incremental writer gets wrong.
    s.replace_metric_deactivations(&std::collections::HashMap::new()).await.unwrap();
    assert!(
        !s.metric_deactivations().await.unwrap().contains_key(&key),
        "an empty plan clears every deactivation"
    );

    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

/// A repo_key the daemon has never registered names no repository, so it inserts
/// nothing rather than failing the whole replace.
#[tokio::test]
async fn a_deactivation_for_an_unknown_repository_is_dropped_not_fatal() {
    let s = pg_store().await;
    let unknown = std::collections::HashMap::from([(
        format!("github.com/_test/never-registered-{}", uuid::Uuid::new_v4()),
        vec!["churn_rate".to_string()],
    )]);
    // The dōjō may legitimately name a repository this install has not seen —
    // another machine registered it. Failing here would take down the whole
    // ruling for every repository that IS known.
    let written = s.replace_metric_deactivations(&unknown).await.expect("must not fail");
    assert_eq!(written, 0, "nothing local matched, so nothing was written");
}

/// A ratio/pct metric whose props carry NO numerator/denominator must still
/// produce a value, not NULL.
///
/// `sensei.workspace_metric_daily` recomputes ratio/pct metrics as
/// `sum(numerator) / sum(denominator)` across a day's repositories. `cache_reuse`
/// does not supply those: it is deliberately the MEAN OF PER-SESSION RATIOS,
/// because pooling is dominated by the longest session and hides the very signal
/// the metric exists to surface (see `tasks/handlers/metrics/usage.rs`).
///
/// So the view produced `NULL / NULLIF(NULL, 0)` = NULL, and
/// `get_workspace_metrics` — which decodes `value` as a non-nullable `f64` — failed
/// the whole read. Measured on the live DB: 68 NULL rows, all `cache_reuse`, and
/// `/api/workspaces/{id}/metrics` answered 500 for exactly the 10 workspaces that had
/// them. Every workspace WITHOUT cache-reuse data returned 200 with an empty grid,
/// which is why this hid for so long.
#[tokio::test]
async fn a_pct_metric_without_numerator_props_still_yields_a_value() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let pid = s.create_workspace(&format!("_test:pctprops:{uniq}"), None, None).await.unwrap();
    let rid =
        crate::tasks::test_support::seed_bare_repository(&s, &pid, &uuid::Uuid::new_v4()).await;
    let key = format!("_test:pctprops:{uniq}:reuse");
    let mid = seed_metric(&s, &key, "CacheReuseLike", 0, None).await;
    let day = chrono::NaiveDate::from_ymd_opt(2020, 3, 4).unwrap();

    // Exactly the shape `usage.rs` writes: no numerator, no denominator.
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &mid,
        repository_id: &rid,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: day,
        grain: "daily",
        value: 0.957,
        props: &serde_json::json!({
            "sessions": 12, "pooled_ratio": 0.981, "mean_of_session_ratios": 0.957
        }),
        source: "measured",
    })
    .await
    .unwrap();

    let rows = s.get_workspace_metrics(&pid).await.unwrap();
    let row = rows.iter().find(|r| r.metric == key).expect("the metric is present, not dropped");
    // The stored value survives. Re-pooling it would be wrong even if props
    // allowed: the mean-of-ratios IS the metric.
    assert!((row.value - 0.957).abs() < 1e-9, "value must survive the view, got {}", row.value);

    sqlx_core::query::query("DELETE FROM sensei.metrics WHERE id = $1")
        .bind(mid)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn get_workspace_metrics_excludes_a_retired_metric() {
    // A retired metric (past `effective_until`, e.g. workspace_health) keeps its
    // durable workspace_metrics rows — retirement is "in place, never hand-delete
    // a row" — so the values read MUST exclude it by the active window, or its
    // stale rows keep rendering as a signal card.
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let pid = s.create_workspace(&format!("_test:gpm-ret:{uniq}"), None, None).await.unwrap();
    let rid =
        crate::tasks::test_support::seed_bare_repository(&s, &pid, &uuid::Uuid::new_v4()).await;
    let active_key = format!("_test:gpm-ret:{uniq}:active");
    let retired_key = format!("_test:gpm-ret:{uniq}:retired");
    let active_mid = seed_metric(&s, &active_key, "ComputeActive", 0, None).await; // active, no end
    let retired_mid = seed_metric(&s, &retired_key, "ComputeRetired", -10, Some(-1)).await; // ended yesterday
    let d = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &active_mid,
        repository_id: &rid,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: d,
        grain: "daily",
        value: 0.5,
        props: &serde_json::json!({"numerator": 1, "denominator": 2}),
        source: "measured",
    })
    .await
    .unwrap();
    // The retired metric HAS a durable row — it just must not be read as active.
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &retired_mid,
        repository_id: &rid,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: d,
        grain: "daily",
        value: 0.9,
        props: &serde_json::json!({"numerator": 9, "denominator": 10}),
        source: "measured",
    })
    .await
    .unwrap();

    let keys: Vec<String> =
        s.get_workspace_metrics(&pid).await.unwrap().into_iter().map(|r| r.metric).collect();
    assert!(keys.contains(&active_key), "the active metric is returned");
    assert!(
        !keys.contains(&retired_key),
        "the retired metric is excluded from the values read — its stale rows must not render",
    );

    sqlx_core::query::query("DELETE FROM sensei.metrics WHERE id = ANY($1)")
        .bind(vec![active_mid, retired_mid])
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

/// The datapoint→sessions drill-down returns ONLY that day's MEASURABLE sessions
/// for the workspace, scoped through the folder-join (`sensei.folders.workspace_id`),
/// newest-first, carrying the structural one-liner fields + summary. Guards the
/// three exclusions the old un-scoped digest got wrong: a different day, an
/// in-flight (`outcome IS NULL`) session, and a session whose FOLDER belongs to
/// another workspace (even when its own `workspace_id` column points here — proving
/// the scope is the folder-join, not `sessions.workspace_id`).
#[tokio::test]
async fn get_workspace_sessions_for_day_scopes_day_and_measurable() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let other = uuid::Uuid::new_v4();
    let (pid, fid) = crate::tasks::test_support::seed_metrics_workspace_folder(&s, &uniq).await;
    let (other_pid, other_fid) =
        crate::tasks::test_support::seed_metrics_workspace_folder(&s, &other).await;

    let day = chrono::NaiveDate::from_ymd_opt(2020, 6, 15).unwrap();
    let utc =
        |ts: &str| chrono::DateTime::parse_from_rfc3339(ts).unwrap().with_timezone(&chrono::Utc);
    let noon = utc("2020-06-15T12:00:00Z"); // in-day, earlier
    let evening = utc("2020-06-15T20:00:00Z"); // in-day, later → newest-first
    let prev = utc("2020-06-14T12:00:00Z"); // different day → excluded

    #[allow(clippy::too_many_arguments)]
    async fn ins(
        s: &PgStore,
        fid: uuid::Uuid,
        pid: uuid::Uuid,
        cs: &str,
        task: &str,
        outcome: Option<&str>,
        ftr: Option<bool>,
        turns: i32,
        corr: i32,
        summary: Option<&str>,
        at: chrono::DateTime<chrono::Utc>,
    ) {
        sqlx_core::query::query(
                "INSERT INTO activity.sessions
                    (folder_id, workspace_id, client_session_id, task, outcome, ftr, turns, corrections, summary, started_at)
                 VALUES ($1, $2, $3, $4, $5::sensei.session_outcome, $6, $7, $8, $9, $10)")
                .bind(fid).bind(pid).bind(cs).bind(task).bind(outcome).bind(ftr)
                .bind(turns).bind(corr).bind(summary).bind(at)
                .execute(s.pool()).await.unwrap();
    }

    // `client_session_id` is GLOBALLY unique (sessions_client_session_id_uniq),
    // so scope the ids to this run's uniq — otherwise a re-run collides.
    let sid = |n: &str| format!("cs-{n}-{}", uniq.simple());
    // The two measurable, in-day, in-workspace sessions we expect back.
    ins(
        &s,
        fid,
        pid,
        &sid("noon"),
        "task-a",
        Some("completed"),
        Some(true),
        3,
        0,
        Some("sum-a"),
        noon,
    )
    .await;
    ins(&s, fid, pid, &sid("eve"), "task-b", Some("corrected"), Some(false), 5, 2, None, evening)
        .await;
    // Exclusions:
    ins(&s, fid, pid, &sid("inflight"), "task-x", None, None, 0, 0, None, noon).await; // not measurable
    ins(&s, fid, pid, &sid("prevday"), "task-y", Some("completed"), Some(true), 1, 0, None, prev)
        .await; // other day
    // Folder in ANOTHER workspace, but its own workspace_id column points at pid —
    // folder-join scope must still exclude it.
    ins(
        &s,
        other_fid,
        pid,
        &sid("otherproj"),
        "task-z",
        Some("completed"),
        Some(true),
        1,
        0,
        None,
        noon,
    )
    .await;

    let rows = s.get_workspace_sessions_for_day(&pid, day).await.unwrap();
    assert_eq!(
        rows.len(),
        2,
        "only the two measurable, in-day, in-workspace (folder-join) sessions"
    );
    // Newest-first: evening then noon.
    assert_eq!(
        rows[0]["client_session_id"].as_str(),
        Some(sid("eve").as_str()),
        "ordered newest-first"
    );
    assert_eq!(rows[1]["client_session_id"].as_str(), Some(sid("noon").as_str()));
    // Structural one-liner fields + summary on the corrected (evening) session.
    let eve = &rows[0];
    assert_eq!(eve["outcome"], "corrected");
    assert_eq!(eve["ftr"], serde_json::json!(false));
    assert_eq!(eve["turns"], 5);
    assert_eq!(eve["corrections"], 2);
    assert_eq!(eve["task"], "task-b");
    assert!(eve["summary"].is_null(), "backfilled/absent summary is honest-null");
    assert!(eve["started_at"].as_str().is_some(), "started_at is an rfc3339 string");
    assert_eq!(rows[1]["summary"], "sum-a", "the existing summary column is returned when present");

    // Honest-empty: a day with no measurable session → [] (not a failure).
    let empty = s
        .get_workspace_sessions_for_day(&pid, chrono::NaiveDate::from_ymd_opt(2019, 1, 1).unwrap())
        .await
        .unwrap();
    assert!(empty.is_empty(), "no sessions that day → honest-empty list");

    // cleanup — sessions FK to the folder; drop them, then the workspaces (folders cascade).
    sqlx_core::query::query("DELETE FROM activity.sessions WHERE folder_id = ANY($1)")
        .bind(vec![fid, other_fid])
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = ANY($1)")
        .bind(vec![pid, other_pid])
        .execute(s.pool())
        .await
        .unwrap();
}

/// Phase 8.1: both FTR getters read `workspace_metrics` (via
/// `workspace_metric_daily`), NOT the retired `sensei.ftr_daily` /
/// `sensei.workspace_ftr_metrics` views. Seeds daily `ftr` rows across the 14d,
/// 7d, and prior-14d windows and asserts the re-sourced values + unchanged
/// response shape.
#[tokio::test]
async fn ftr_getters_read_workspace_metrics() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let pid = s.create_workspace(&format!("_test:ftrget:{uniq}"), None, None).await.unwrap();
    // The REAL registry ftr metric — the getters filter `metric = 'ftr'`.
    let (ftr_mid,): (uuid::Uuid,) =
        sqlx_core::query_as::query_as("SELECT id FROM sensei.metrics WHERE key = 'ftr'")
            .fetch_one(s.pool())
            .await
            .expect("ftr metric seeded in registry");

    let today = chrono::Utc::now().date_naive();
    let d_recent = today - chrono::Duration::days(3); // 7d + 14d window
    let d_prev = today - chrono::Duration::days(20); // prior-14d window only
    // Repo-grain (_v2): seed a per-test repository so these shared-`ftr` rows key
    // on (ftr, repo, user, day) and can't collide with a sibling workspace's rows.
    let (rid,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'ftrget') RETURNING id",
    )
    .bind(format!("test/ftrget-{uniq}"))
    .fetch_one(s.pool())
    .await
    .unwrap();
    crate::tasks::test_support::link_repository_to_workspace(&s, &rid, &pid, "ftrget").await;
    // day A (today):     3/4 = 0.75
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &ftr_mid,
        repository_id: &rid,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: today,
        grain: "daily",
        value: 0.75,
        props: &serde_json::json!({"numerator": 3, "denominator": 4, "correction_count": 1}),
        source: "measured",
    })
    .await
    .unwrap();
    // day B (today-3):   1/2 = 0.50
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &ftr_mid,
        repository_id: &rid,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: d_recent,
        grain: "daily",
        value: 0.5,
        props: &serde_json::json!({"numerator": 1, "denominator": 2, "correction_count": 2}),
        source: "measured",
    })
    .await
    .unwrap();
    // day C (today-20):  1/2 = 0.50 — prior-14d window, excluded from 14d/7d
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &ftr_mid,
        repository_id: &rid,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: d_prev,
        grain: "daily",
        value: 0.5,
        props: &serde_json::json!({"numerator": 1, "denominator": 2, "correction_count": 3}),
        source: "measured",
    })
    .await
    .unwrap();

    // ── get_ftr_daily (per-workspace): value → ftr_rate, props.denominator →
    //    session_count; day C (older than 14d) excluded. Shape unchanged.
    let daily = s.get_ftr_daily(Some(&pid), 14).await.unwrap();
    assert_eq!(daily.len(), 2, "only the two rows inside the 14d window (day C excluded)");
    let a = daily
        .iter()
        .find(|r| r["day"].as_str() == Some(today.to_string().as_str()))
        .expect("today row");
    assert_eq!(
        a.as_object().unwrap().keys().collect::<Vec<_>>(),
        vec!["day", "ftr_rate", "session_count"],
        "exact response shape preserved"
    );
    assert!((a["ftr_rate"].as_f64().unwrap() - 0.75).abs() < 1e-9, "ftr_rate = stored value");
    assert_eq!(a["session_count"].as_i64(), Some(4), "session_count = props.denominator");
    let b = daily
        .iter()
        .find(|r| r["day"].as_str() == Some(d_recent.to_string().as_str()))
        .expect("today-3 row");
    assert!((b["ftr_rate"].as_f64().unwrap() - 0.5).abs() < 1e-9);
    assert_eq!(b["session_count"].as_i64(), Some(2));

    // ── get_ftr_daily (holistic): sums denominators across workspaces for the
    //    day; our contribution is a safe lower bound (other test data may add).
    let holistic = s.get_ftr_daily(None, 14).await.unwrap();
    let ht = holistic
        .iter()
        .find(|r| r["day"].as_str() == Some(today.to_string().as_str()))
        .expect("today holistic row");
    assert!(ht["ftr_rate"].as_f64().is_some(), "holistic ftr_rate present");
    assert!(
        ht["session_count"].as_i64().unwrap() >= 4,
        "holistic session_count sums denominators (>= our 4)"
    );

    // ── get_workspace_ftr headline: Σnum/Σden per window; shape unchanged.
    let ftr = s.get_workspace_ftr(&pid).await.unwrap();
    assert_eq!(
        ftr.as_object().unwrap().keys().collect::<Vec<_>>(),
        vec!["ftr14d", "ftr14dPrev", "ftrTrend", "sessions7d"],
        "exact response shape preserved"
    );
    assert!(
        (ftr["ftr14d"].as_f64().unwrap() - (4.0 / 6.0)).abs() < 1e-9,
        "ftr14d = Σnum/Σden over 14d = (3+1)/(4+2)"
    );
    assert!(
        (ftr["ftr14dPrev"].as_f64().unwrap() - 0.5).abs() < 1e-9,
        "ftr14dPrev = Σnum/Σden over prior-14d window (day C only) = 1/2"
    );
    assert_eq!(ftr["sessions7d"].as_i64(), Some(6), "sessions7d = Σdenominator over 7d = 4+2");
    assert!(
        ftr["ftrTrend"].as_array().is_some(),
        "ftrTrend is an array (trend reads sessions, not the store)"
    );

    // ── shared rate helper agrees with the headline.
    assert!(
        (s.get_workspace_ftr_rate(&pid).await.unwrap().unwrap() - (4.0 / 6.0)).abs() < 1e-9,
        "get_workspace_ftr_rate == ftr14d"
    );

    // cleanup — workspace_metrics rows cascade from the workspace (ftr metric kept).
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

/// FIX 1 (DB-free): the shared headline builder serializes an absent 14d /
/// prior-14d FTR as JSON `null` — NEVER a fabricated `0.0`. Covers BOTH
/// `get_workspace_ftr` and `get_holistic_ftr`, which share this builder.
/// Mutation guard: reverting the builder to `.unwrap_or(0.0)` fails this.
#[test]
fn ftr_headline_json_absent_serializes_null_not_zero() {
    let absent = PgStore::ftr_headline_json(None, None, vec![], 0);
    assert!(absent["ftr14d"].is_null(), "absent ftr14d → JSON null, not 0.0");
    assert!(absent["ftr14dPrev"].is_null(), "absent ftr14dPrev → JSON null, not 0.0");
    assert_eq!(absent["sessions7d"].as_i64(), Some(0), "sessions7d is an honest count");
    let present = PgStore::ftr_headline_json(Some(0.5), Some(0.25), vec![0.5], 3);
    assert_eq!(
        present["ftr14d"].as_f64(),
        Some(0.5),
        "a present value still serializes as a number"
    );
    assert_eq!(present["ftr14dPrev"].as_f64(), Some(0.25));
}

/// FIX 1 (end-to-end): a workspace with zero stored `ftr` rows reports honest
/// `null` for the headline through `get_workspace_ftr` — never a fabricated 0%.
#[tokio::test]
async fn get_workspace_ftr_absent_is_null_not_zero() {
    let s = pg_store().await;
    let pid = s
        .create_workspace(&format!("_test:ftrnull:{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let ftr = s.get_workspace_ftr(&pid).await.unwrap();
    assert!(ftr["ftr14d"].is_null(), "no ftr rows → ftr14d is null, NOT 0.0");
    assert!(ftr["ftr14dPrev"].is_null(), "no ftr rows → ftr14dPrev is null, NOT 0.0");
    assert_eq!(ftr["sessions7d"].as_i64(), Some(0), "sessions7d is an honest 0 (a count)");
    assert!(ftr["ftrTrend"].as_array().is_some_and(|a| a.is_empty()), "no sessions → empty trend");
    assert_eq!(
        s.get_workspace_ftr_rate(&pid).await.unwrap(),
        None,
        "rate helper is None on no data"
    );
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

/// FIX 5 (window mutation guard): `ftr14d` must reach back the full 14 days.
/// The only stored row is 10 days old — inside 14d, outside 7d — so `ftr14d`
/// is 1.0 while `sessions7d` (7d) excludes it. Narrowing the 14d window to 7d
/// would make `ftr14d` null, failing the `.expect` below.
#[tokio::test]
async fn ftr14d_window_reaches_the_8_to_13_day_band() {
    let s = pg_store().await;
    let pid = s
        .create_workspace(&format!("_test:ftrwin:{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let (ftr_mid,): (uuid::Uuid,) = query_as("SELECT id FROM sensei.metrics WHERE key = 'ftr'")
        .fetch_one(s.pool())
        .await
        .unwrap();
    let d10 = chrono::Utc::now().date_naive() - chrono::Duration::days(10); // 8–13d band
    // Repo-grain (_v2): a per-test repository so the shared `ftr` row keys on
    // (ftr, repo, user, day) and can't collide with a sibling workspace's row.
    let (rid,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'ftrwin') RETURNING id",
    )
    .bind(format!("test/ftrwin-{}", uuid::Uuid::new_v4()))
    .fetch_one(s.pool())
    .await
    .unwrap();
    crate::tasks::test_support::link_repository_to_workspace(&s, &rid, &pid, "ftrwin").await;
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &ftr_mid,
        repository_id: &rid,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: d10,
        grain: "daily",
        value: 1.0,
        props: &serde_json::json!({"numerator": 2, "denominator": 2}),
        source: "measured",
    })
    .await
    .unwrap();

    let ftr = s.get_workspace_ftr(&pid).await.unwrap();
    assert!(
        (ftr["ftr14d"].as_f64().expect("ftr14d includes the 10-day-old row (14d window)") - 1.0)
            .abs()
            < 1e-9,
        "only row is 10d old → ftr14d = 1.0; a 7d-narrowed window would make this null"
    );
    assert_eq!(
        ftr["sessions7d"].as_i64(),
        Some(0),
        "sessions7d (7d window) excludes the 10-day-old row — proves 14d ≠ 7d"
    );
    assert_eq!(
        s.get_workspace_ftr_rate(&pid).await.unwrap(),
        Some(1.0),
        "rate helper (14d) includes it too"
    );

    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

/// FIX 3: the holistic (no-workspace) `get_ftr_daily` branch POOLS Σnum/Σden per
/// day — it must NOT average per-workspace daily rates (the `workspace_metrics`
/// ratio invariant). Two workspaces on one day with unequal denominators make
/// pooled ≠ avg-of-rates; the getter must match the pooled value.
#[tokio::test]
async fn holistic_ftr_daily_pools_not_average_of_rates() {
    let s = pg_store().await;
    let (ftr_mid,): (uuid::Uuid,) = query_as("SELECT id FROM sensei.metrics WHERE key = 'ftr'")
        .fetch_one(s.pool())
        .await
        .unwrap();
    // A day off the busy 'today' (compute-writing tests seed today) but inside 14d.
    let day = chrono::Utc::now().date_naive() - chrono::Duration::days(6);
    let p1 = s
        .create_workspace(&format!("_test:ftrpool1:{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let p2 = s
        .create_workspace(&format!("_test:ftrpool2:{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    // Repo-grain (_v2): each workspace needs its OWN repository — under the repo-grain
    // identity a NULL-repository `(ftr, day)` row is shared, so both workspaces would
    // collapse onto ONE row and the pooled Σnum/Σden could never be observed.
    let (r1,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'ftrpool1') RETURNING id",
    )
    .bind(format!("test/ftrpool1-{}", uuid::Uuid::new_v4()))
    .fetch_one(s.pool())
    .await
    .unwrap();
    crate::tasks::test_support::link_repository_to_workspace(&s, &r1, &p1, "ftrpool1").await;
    let (r2,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories (repo_key, name) VALUES ($1, 'ftrpool2') RETURNING id",
    )
    .bind(format!("test/ftrpool2-{}", uuid::Uuid::new_v4()))
    .fetch_one(s.pool())
    .await
    .unwrap();
    crate::tasks::test_support::link_repository_to_workspace(&s, &r2, &p2, "ftrpool2").await;
    // P1: 1/1 = 1.0 ; P2: 0/3 = 0.0 → avg-of-rates 0.5, pooled 1/4 = 0.25.
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &ftr_mid,
        repository_id: &r1,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: day,
        grain: "daily",
        value: 1.0,
        props: &serde_json::json!({"numerator": 1, "denominator": 1}),
        source: "measured",
    })
    .await
    .unwrap();
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &ftr_mid,
        repository_id: &r2,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: day,
        grain: "daily",
        value: 0.0,
        props: &serde_json::json!({"numerator": 0, "denominator": 3}),
        source: "measured",
    })
    .await
    .unwrap();

    let holistic = s.get_ftr_daily(None, 14).await.unwrap();
    let row = holistic
        .iter()
        .find(|r| r["day"].as_str() == Some(day.to_string().as_str()))
        .expect("holistic row for the seeded day");

    // Compare to the DIRECT pooled + avg over whatever exists globally for that
    // day (robust to other rows), and assert the getter matches POOLED, not avg.
    let (sum_num, sum_den, avg_rate): (f64, i64, f64) = query_as(
            "SELECT SUM((props->>'numerator')::float8), SUM((props->>'denominator')::int8)::int8, AVG(value)::float8 \
               FROM sensei.workspace_metric_daily WHERE metric = 'ftr' AND date = $1",
        ).bind(day).fetch_one(s.pool()).await.unwrap();
    let pooled = sum_num / sum_den as f64;
    assert!(
        (row["ftr_rate"].as_f64().unwrap() - pooled).abs() < 1e-9,
        "holistic ftr_rate is pooled Σnum/Σden, not an average of per-workspace rates"
    );
    assert_eq!(
        row["session_count"].as_i64(),
        Some(sum_den),
        "holistic session_count is Σdenominator"
    );
    assert!(
        (pooled - avg_rate).abs() > 1e-9,
        "seed makes pooled ({pooled}) differ from avg-of-rates ({avg_rate}) — so the check above is a real discriminator"
    );

    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = ANY($1)")
        .bind(vec![p1, p2])
        .execute(s.pool())
        .await
        .unwrap();
}

/// A repository in no workspace has metric rows with a NULL `workspace_id` in
/// `sensei.workspace_metrics`. Correlation cells are keyed by (workspace, day), so
/// such a row cannot take part — but it decoded into a non-optional uuid and
/// failed the whole portfolio read with a 500 (live, 2026-10-19: 725 rows from
/// 24 repositories).
#[tokio::test]
async fn correlations_read_past_a_repository_in_no_workspace() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (rid,): (uuid::Uuid,) =
        query_as("INSERT INTO sensei.repositories (name) VALUES ($1) RETURNING id")
            .bind(format!("_test:no-workspace:{uniq}"))
            .fetch_one(s.pool())
            .await
            .unwrap();
    let mid = seed_metric(&s, &format!("_test:no-workspace:{uniq}:m"), "ComputeFtr", 0, None).await;
    s.upsert_workspace_metric_repo(&MetricRow {
        metric_id: &mid,
        repository_id: &rid,
        scope: "user",
        identity: None,
        commit_sha: None,
        computed_on: chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap(),
        grain: "daily",
        value: 0.5,
        props: &serde_json::json!({}),
        source: "measured",
    })
    .await
    .unwrap();

    let got = s.get_metric_correlations(None).await;
    assert!(got.is_ok(), "the portfolio read failed: {got:?}");

    for (sql, id) in [
        ("DELETE FROM sensei.repository_metrics WHERE repository_id = $1", rid),
        ("DELETE FROM sensei.repositories WHERE id = $1", rid),
        ("DELETE FROM sensei.metrics WHERE id = $1", mid),
    ] {
        sqlx_core::query::query(sql).bind(id).execute(s.pool()).await.unwrap();
    }
}
