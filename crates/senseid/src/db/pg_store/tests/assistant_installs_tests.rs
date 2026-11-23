//! The assistant install rows (`sensei.assistant_installs`, #455): detection
//! and configure keep them current, and a watchdog tick that re-ran configure
//! re-detects. These need Postgres, so they live here rather than in the pure
//! `assistants` module (see `scripts/senseid-pure-coverage.sh`).

use crate::assistants::watchdog::{TickOutcome, resync_after_tick};
use crate::assistants::{AssistantStatus, sync_installs_with};
use crate::db::pg_store::PgStore;

/// A probe result, as `detect()` gives it: ids and state, no names (the
/// catalogue owns those).
fn status(id: &str, installed: bool, configured: bool) -> AssistantStatus {
    serde_json::from_value(serde_json::json!({
        "id": id, "family": id, "installed": installed, "configured": configured,
        "config_path": format!("/home/x/{id}.json"),
    }))
    .expect("a probe result carries no name")
}

async fn install_row(
    pg: &PgStore,
    id: &str,
) -> Option<(bool, bool, Option<String>, Option<String>, chrono::DateTime<chrono::Utc>)> {
    sqlx_core::query_as::query_as(
        "select installed, configured, config_path, configured_version, detected_at
           from sensei.assistant_installs where catalogue_id = $1",
    )
    .bind(id)
    .fetch_optional(pg.pool())
    .await
    .unwrap()
}

async fn configuration_stamp(
    pg: &PgStore,
    id: &str,
) -> (Option<chrono::DateTime<chrono::Utc>>, Option<String>) {
    sqlx_core::query_as::query_as(
        "select configured_at, configured_version
           from sensei.assistant_installs where catalogue_id = $1",
    )
    .bind(id)
    .fetch_one(pg.pool())
    .await
    .unwrap()
}

#[tokio::test]
async fn sync_installs_stores_one_row_per_catalogue_tool_and_updates_in_place() {
    let pg = PgStore::test_store().await;
    sqlx_core::query::query("delete from sensei.assistant_installs")
        .execute(pg.pool())
        .await
        .unwrap();
    let probe = vec![
        status("claude-code", true, true),
        status("cursor", true, false),
        status("muse", true, false),
    ];
    let rows = sync_installs_with(&pg, &probe, &[]).await.unwrap();
    let ids: Vec<&str> = rows.iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["claude", "cursor"], "{rows:?}");
    assert_eq!(rows[0]["name"], "Claude Code", "the catalogue names the row");
    assert_eq!(rows[1]["kind"], "editor");
    let n: i64 =
        sqlx_core::query_scalar::query_scalar("select count(*) from sensei.assistant_installs")
            .fetch_one(pg.pool())
            .await
            .unwrap();
    assert_eq!(n, 2, "muse has no catalogue row and is not stored");
    let (installed, configured, path, version, first) = install_row(&pg, "claude").await.unwrap();
    assert!(installed && configured);
    assert_eq!(path.as_deref(), Some("/home/x/claude-code.json"));
    assert_eq!(version, None, "detection does not stamp a configuration");
    let (_, configured, _, version, _) = install_row(&pg, "cursor").await.unwrap();
    assert!(!configured);
    assert_eq!(version, None, "never configured, no version");

    // A second run updates in place.
    let probe = vec![status("claude-code", true, true), status("cursor", true, true)];
    sync_installs_with(&pg, &probe, &[]).await.unwrap();
    let n: i64 =
        sqlx_core::query_scalar::query_scalar("select count(*) from sensei.assistant_installs")
            .fetch_one(pg.pool())
            .await
            .unwrap();
    assert_eq!(n, 2);
    let (_, configured, _, version, _) = install_row(&pg, "cursor").await.unwrap();
    assert!(configured, "configure is followed");
    assert_eq!(version, None, "seen configured, not configured by this run");
    let (_, _, _, _, second) = install_row(&pg, "claude").await.unwrap();
    assert!(second >= first, "detected_at moves with each run");
}

/// `configured_at` / `configured_version` say when, and under which sensei
/// version, sensei configured the tool. Detection only observes, so it
/// never stamps them: a tool found configured on first run has no time or
/// version, and one configured under an older sensei keeps its own.
#[tokio::test]
async fn detection_alone_never_stamps_the_configuration() {
    let pg = PgStore::test_store().await;
    sqlx_core::query::query("delete from sensei.assistant_installs")
        .execute(pg.pool())
        .await
        .unwrap();
    sqlx_core::query::query(
        "insert into sensei.assistant_installs
                (catalogue_id, installed, configured, configured_at, configured_version, detected_at)
         values ('cursor', true, true, '2026-01-02T03:04:05Z', '0.0.1', '2026-01-02T03:04:05Z')",
    )
    .execute(pg.pool())
    .await
    .unwrap();
    let probe = vec![status("claude-code", true, true), status("cursor", true, true)];
    sync_installs_with(&pg, &probe, &[]).await.unwrap();

    let (at, version) = configuration_stamp(&pg, "claude").await;
    assert_eq!((at, version.as_deref()), (None, None), "first seen configured: no stamp");
    let (at, version) = configuration_stamp(&pg, "cursor").await;
    assert_eq!(
        at.map(|t| t.to_rfc3339()).as_deref(),
        Some("2026-01-02T03:04:05+00:00"),
        "an older configuration keeps its time"
    );
    assert_eq!(version.as_deref(), Some("0.0.1"), "and its sensei version");
}

/// Configure (#455 "Configure keeps the row current"): the tools this
/// sensei just configured, and the probe confirms, read configured with the
/// time and this sensei version; the rest of the run only observes.
#[tokio::test]
async fn configuring_stamps_the_time_and_this_sensei_version() {
    let pg = PgStore::test_store().await;
    sqlx_core::query::query("delete from sensei.assistant_installs")
        .execute(pg.pool())
        .await
        .unwrap();
    sqlx_core::query::query(
        "insert into sensei.assistant_installs
                (catalogue_id, installed, configured, configured_at, configured_version, detected_at)
         values ('cursor', true, true, '2026-01-02T03:04:05Z', '0.0.1', '2026-01-02T03:04:05Z')",
    )
    .execute(pg.pool())
    .await
    .unwrap();
    let before = chrono::Utc::now();
    let probe = vec![
        status("claude-code", true, true),
        status("cursor", true, true),
        status("windsurf", true, false),
    ];
    let just_configured = ["cursor".to_string(), "windsurf".to_string()];
    sync_installs_with(&pg, &probe, &just_configured).await.unwrap();

    let (at, version) = configuration_stamp(&pg, "cursor").await;
    assert!(at.is_some_and(|t| t >= before), "configured now: {at:?}");
    assert_eq!(version.as_deref(), Some(env!("CARGO_PKG_VERSION")));
    let (at, version) = configuration_stamp(&pg, "claude").await;
    assert_eq!((at, version), (None, None), "not configured by this run");
    let (at, version) = configuration_stamp(&pg, "windsurf").await;
    assert_eq!((at, version), (None, None), "the probe does not see it configured");
}

// A tick that re-ran configure (auto-resolved, or tried and suspended)
// re-detects, so the install rows follow the repair (#455 review). Ticks
// that changed nothing leave the rows alone.
async fn seed_stale_cursor(pg: &PgStore) {
    sqlx_core::query::query("delete from sensei.assistant_installs")
        .execute(pg.pool())
        .await
        .unwrap();
    sqlx_core::query::query(
        "insert into sensei.assistant_installs (catalogue_id, installed, config_path, configured, detected_at)
         values ('cursor', true, '/stale/.cursor/mcp.json', false, '2000-01-01T00:00:00Z')",
    )
    .execute(pg.pool())
    .await
    .unwrap();
}

async fn cursor_redetected(pg: &PgStore) -> bool {
    sqlx_core::query_scalar::query_scalar(
        "select detected_at > '2001-01-01' from sensei.assistant_installs
          where catalogue_id = 'cursor'",
    )
    .fetch_one(pg.pool())
    .await
    .unwrap()
}

#[tokio::test]
async fn a_tick_that_reran_configure_redetects_the_installs() {
    let pg = PgStore::test_store().await;
    for outcome in [TickOutcome::Resolved, TickOutcome::Suspended] {
        seed_stale_cursor(&pg).await;
        assert_eq!(resync_after_tick(&pg, outcome, "cursor").await, None);
        assert!(cursor_redetected(&pg).await, "{outcome:?} left the install rows stale");
    }
}

#[tokio::test]
async fn a_tick_that_changed_nothing_leaves_the_installs_alone() {
    let pg = PgStore::test_store().await;
    for outcome in [
        TickOutcome::Skipped,
        TickOutcome::Healthy,
        TickOutcome::StaleWarned,
        TickOutcome::StaleAlreadyNotified,
    ] {
        seed_stale_cursor(&pg).await;
        assert_eq!(resync_after_tick(&pg, outcome, "cursor").await, None);
        assert!(!cursor_redetected(&pg).await, "{outcome:?} re-detected");
    }
}
