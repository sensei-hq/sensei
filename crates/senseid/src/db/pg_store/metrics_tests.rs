//! DB-touching tests for the metric WRITE path (#161).
//!
//! `upsert_project_metric_repo` took ten positional arguments, four of which are
//! interchangeable at the type level — `scope`, `identity`, `commit_sha` and
//! `source` are all string-ish, and two of them are `Option`. A call site reads
//! `SCOPE_USER, None, None, day, GRAIN_DAILY, …`: swap the two `None`s and
//! nothing complains, at compile time or at run time.
//!
//! These tests are the net for converting it to a typed row. They assert the
//! round trip of EVERY dimension with a distinct value, so a field that stops
//! being carried — or starts being carried into the wrong column — fails here.
//!
//! Guarded like the neighbouring pg_store tests: no test database means the
//! test no-ops rather than fails.
use super::*;

/// The ten columns read back, in the order the SELECT names them. A type alias
/// rather than an inline tuple because clippy is right that a ten-wide one is
/// unreadable — and because the read-back being positional is the same hazard
/// the write path just stopped having. The assertions below name every field, so
/// a transposition here fails rather than passing silently.
type StoredRow = (
    uuid::Uuid,
    uuid::Uuid,
    String,
    Option<String>,
    Option<String>,
    chrono::NaiveDate,
    String,
    f64,
    serde_json::Value,
    String,
);

/// A metric this test OWNS, so nothing else can remove it.
///
/// The first version borrowed one from the catalog with `ORDER BY key LIMIT 1`.
/// That is not stable on a shared database (#183): a concurrent test that mints
/// a metric sorting ahead of the catalog's first, uses it and deletes it leaves
/// this one holding an id that no longer exists, and the write fails on the
/// foreign key. Observed exactly that under the full `metrics` filter while
/// passing alone — which is the signature of borrowing another test's row.
async fn own_metric(pg: &PgStore, uniq: &uuid::Uuid) -> uuid::Uuid {
    let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.metrics
             (key, name, description, family, type, direction, purpose,
              how_to_read, formula, task_name)
         VALUES ($1, '_test metric', 'fixture', 'quality'::sensei.metric_family,
                 'ratio'::sensei.metric_type, 'higher_better'::sensei.metric_direction,
                 'fixture', 'fixture', 'fixture', '_test')
         RETURNING id",
    )
    .bind(format!("_test_metric_row_{uniq}"))
    .fetch_one(pg.pool())
    .await
    .expect("seed a metric");
    id
}

async fn drop_metric(pg: &PgStore, id: &uuid::Uuid) {
    sqlx_core::query::query("DELETE FROM sensei.metrics WHERE id = $1")
        .bind(id)
        .execute(pg.pool())
        .await
        .ok();
}

/// Every dimension a metric row carries survives the write, in its own column.
///
/// THE POINT IS THE DISTINCT VALUES. Each of the ten is given a value that is
/// NOT the default a call site would otherwise supply — `scope = repo` not
/// `user`, `grain = session` not `daily`, `source = federated` not `measured` — so the assertions fail on a swap as well as on a drop —
/// `identity` and `commit_sha` are adjacent `Option<&str>` parameters today and
/// a transposition between them is invisible to the compiler.
///
/// Mutation that must break this test: transpose the `.bind(identity)` and
/// `.bind(commit_sha)` lines in `upsert_project_metric_repo`.
#[tokio::test]
async fn every_dimension_of_a_metric_row_reaches_its_own_column() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let uniq = uuid::Uuid::new_v4();
    let metric_id = own_metric(&pg, &uniq).await;
    let (pid, fid) = crate::tasks::test_support::seed_metrics_project_folder(&pg, &uniq).await;
    let repository_id = crate::tasks::test_support::repository_for_folder(&pg, &fid).await;

    let props = serde_json::json!({ "numerator": 7, "denominator": 11, "tag": uniq.to_string() });
    let computed_on = chrono::NaiveDate::from_ymd_opt(2024, 2, 29).expect("a real date");

    let id = pg
        .upsert_project_metric_repo(&MetricRow {
            metric_id: &metric_id,
            repository_id: &repository_id,
            scope: "repo",
            identity: Some("identity-marker"),
            commit_sha: Some("commit-marker"),
            computed_on,
            grain: "session",
            value: 42.5,
            props: &props,
            source: "federated",
        })
        .await
        .expect("the write succeeds");

    let (
        got_metric,
        got_repo,
        got_scope,
        got_identity,
        got_commit,
        got_day,
        got_grain,
        got_value,
        got_props,
        got_source,
    ): StoredRow = sqlx_core::query_as::query_as(
        "SELECT metric_id, repository_id, scope::text, identity, commit_sha,
                computed_on, grain::text, value::float8, props, source::text
           FROM sensei.repository_metrics WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pg.pool())
    .await
    .expect("the row is readable");

    assert_eq!(got_metric, metric_id, "metric_id");
    assert_eq!(got_repo, repository_id, "repository_id");
    assert_eq!(got_scope, "repo", "scope — not the default 'user'");
    assert_eq!(got_identity.as_deref(), Some("identity-marker"), "identity");
    assert_eq!(got_commit.as_deref(), Some("commit-marker"), "commit_sha");
    assert_eq!(got_day, computed_on, "computed_on");
    assert_eq!(got_grain, "session", "grain — not the default 'daily'");
    assert!((got_value - 42.5).abs() < 1e-9, "value");
    assert_eq!(got_props, props, "props, whole");
    assert_eq!(got_source, "federated", "source — not the default 'measured'");

    crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, Some(&fid), &[]).await;
    drop_metric(&pg, &metric_id).await;
}

/// Re-writing the same key UPDATES rather than duplicating, and the mutable
/// fields move while the key fields do not.
///
/// The conflict target is seven columns wide
/// (`metric_id, repository_id, scope, identity, commit_sha, computed_on, grain`).
/// A conversion that drops one of them from the key would turn an update into a
/// second row, which no count-free assertion would notice.
///
/// Mutation that must break this test: remove `identity` from the `ON CONFLICT`
/// target list.
#[tokio::test]
async fn re_writing_one_key_updates_the_row_rather_than_adding_another() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let uniq = uuid::Uuid::new_v4();
    let metric_id = own_metric(&pg, &uniq).await;
    let (pid, fid) = crate::tasks::test_support::seed_metrics_project_folder(&pg, &uniq).await;
    let repository_id = crate::tasks::test_support::repository_for_folder(&pg, &fid).await;
    let day = chrono::NaiveDate::from_ymd_opt(2024, 3, 1).expect("a real date");

    // Borrowed, not moved: the read-back below needs the same pool.
    let write = |identity: &'static str, value: f64, source: &'static str| {
        let pg = &pg;
        async move {
            pg.upsert_project_metric_repo(&MetricRow {
                metric_id: &metric_id,
                repository_id: &repository_id,
                scope: "repo",
                identity: Some(identity),
                commit_sha: None,
                computed_on: day,
                grain: "daily",
                value,
                props: &serde_json::json!({ "v": value }),
                source,
            })
            .await
            .expect("write")
        }
    };

    let first = write("same", 1.0, "measured").await;
    let again = write("same", 2.0, "federated").await;
    assert_eq!(first, again, "the same key returns the same row id");

    // A DIFFERENT identity is a different row — that column is part of the key.
    let other = write("other", 3.0, "measured").await;
    assert_ne!(first, other, "identity is part of the conflict target");

    let (value, source): (f64, String) = sqlx_core::query_as::query_as(
        "SELECT value::float8, source::text FROM sensei.repository_metrics WHERE id = $1",
    )
    .bind(first)
    .fetch_one(pg.pool())
    .await
    .expect("readable");
    assert!((value - 2.0).abs() < 1e-9, "the second write's value won");
    assert_eq!(source, "federated", "and its source");

    crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, Some(&fid), &[]).await;
    drop_metric(&pg, &metric_id).await;
}
