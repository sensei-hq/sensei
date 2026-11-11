//! A session's SDD framework, derived by the session-signal pass against a real
//! database (blueprint 2026-11-10-ticket-sdd-framework B4–B12, #390). Each test has
//! its own database, so pass counts are exact.
use super::*;
use crate::sdlc::signals::DERIVE_VERSION;
use crate::tasks::session_signals::{Probe, pass, pass_with};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

const T0: i64 = 1_790_000_000_000;
const NOW: i64 = T0 + 86_400_000;

/// A repository folder row at `abs_path`.
async fn repo(s: &PgStore, abs_path: &str) -> uuid::Uuid {
    exec(
        s,
        "INSERT INTO sensei.folders_to_watch(id, path, name, status)
         VALUES ('00000000-0000-0000-0000-0000000000f1', '/_fw', '_fw', 'watching')
         ON CONFLICT DO NOTHING",
    )
    .await;
    let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path)
         VALUES ('00000000-0000-0000-0000-0000000000f1', 'git', $1, $1, $1) RETURNING id",
    )
    .bind(abs_path)
    .fetch_one(s.pool())
    .await
    .unwrap();
    id
}

/// A session in `repo_folder` with one event per `(event_type, tool, payload)`.
async fn session(
    s: &PgStore,
    repo_folder: Option<uuid::Uuid>,
    minutes_ago: i32,
    events: &[(&str, Option<&str>, serde_json::Value)],
) -> uuid::Uuid {
    let client = format!("fw-{}", uuid::Uuid::new_v4());
    let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO activity.sessions (client_session_id, repo_folder_id, started_at)
         VALUES ($1, $2, now() - make_interval(mins => $3)) RETURNING id",
    )
    .bind(&client)
    .bind(repo_folder)
    .bind(minutes_ago)
    .fetch_one(s.pool())
    .await
    .unwrap();
    for (i, (event_type, tool, payload)) in events.iter().enumerate() {
        add_event(s, &client, event_type, *tool, T0 + i as i64 * 1_000, payload.clone()).await;
    }
    id
}

async fn add_event(
    s: &PgStore,
    client: &str,
    event_type: &str,
    tool: Option<&str>,
    ts: i64,
    payload: serde_json::Value,
) {
    sqlx_core::query::query(
        "INSERT INTO activity.assistant_events (session_id, family, event_type, tool_name, ts, payload)
         VALUES ($1, 'claude', $2, $3, $4, $5)",
    )
    .bind(client)
    .bind(event_type)
    .bind(tool)
    .bind(ts)
    .bind(payload)
    .execute(s.pool())
    .await
    .unwrap();
}

async fn client_of(s: &PgStore, id: uuid::Uuid) -> String {
    let (c,): (String,) = sqlx_core::query_as::query_as(
        "SELECT client_session_id FROM activity.sessions WHERE id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    c
}

async fn framework(s: &PgStore, id: uuid::Uuid) -> (Option<String>, Option<String>) {
    sqlx_core::query_as::query_as(
        "SELECT sdd_framework::text, sdd_framework_source FROM activity.sessions WHERE id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap()
}

async fn signal_count(s: &PgStore, id: uuid::Uuid) -> i64 {
    let (n,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*) FROM activity.session_signals WHERE session_id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    n
}

async fn exec(s: &PgStore, sql: &str) {
    sqlx_core::query::query(sql).execute(s.pool()).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
}

fn prompt(text: &str) -> (&'static str, Option<&'static str>, serde_json::Value) {
    ("UserPromptSubmit", None, json!({"prompt": text}))
}
fn edit(path: &str) -> (&'static str, Option<&'static str>, serde_json::Value) {
    ("PostToolUse", Some("Edit"), json!({"tool_input": {"file_path": path}}))
}
fn skill(name: &str) -> (&'static str, Option<&'static str>, serde_json::Value) {
    ("PreToolUse", Some("Skill"), json!({"tool_input": {"skill": name}}))
}

fn denying_probe(calls: Arc<AtomicUsize>) -> Probe {
    Arc::new(move |p: &std::path::Path| {
        calls.fetch_add(1, Ordering::SeqCst);
        let _ = p;
        Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
    })
}

#[tokio::test]
async fn a_typed_command_and_a_skill_are_stored_as_behavioural() {
    let s = PgStore::test_store().await;
    let opsx = session(&s, None, 10, &[prompt("/opsx:apply add-auth")]).await;
    let ide = session(
        &s,
        None,
        9,
        &[prompt("<ide_opened_file>x</ide_opened_file>\n/speckit.plan add auth")],
    )
    .await;
    let bmad = session(&s, None, 8, &[skill("bmad-agent-pm")]).await;
    let sensei = session(&s, None, 7, &[prompt("/sensei:plan go"), skill("sensei:build")]).await;
    let r = pass(&s, NOW, 10_000).await.unwrap();
    assert_eq!(r.failed, 0);
    assert_eq!(framework(&s, opsx).await, (Some("openspec".into()), Some("behavioural".into())));
    assert_eq!(framework(&s, ide).await, (Some("spec_kit".into()), Some("behavioural".into())));
    assert_eq!(framework(&s, bmad).await, (Some("bmad".into()), Some("behavioural".into())));
    assert_eq!(framework(&s, sensei).await, (None, None), "sensei is not a framework (B3)");
}

#[tokio::test]
async fn edits_count_only_inside_the_session_repo() {
    let s = PgStore::test_store().await;
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    std::fs::create_dir(&a).unwrap();
    let a = a.to_str().unwrap().to_string();
    let folder = repo(&s, &a).await;
    let inside =
        session(&s, Some(folder), 10, &[edit(&format!("{a}/openspec/changes/x/tasks.md"))]).await;
    let outside = session(
        &s,
        Some(folder),
        9,
        &[edit(&format!("{}/b/openspec/changes/x/tasks.md", dir.path().display()))],
    )
    .await;
    pass(&s, NOW, 10_000).await.unwrap();
    assert_eq!(framework(&s, inside).await, (Some("openspec".into()), Some("behavioural".into())));
    assert_eq!(
        framework(&s, outside).await,
        (None, None),
        "another repo's edit is no evidence (B4)"
    );
}

#[tokio::test]
async fn the_repo_markers_on_disk_are_the_fallback() {
    let s = PgStore::test_store().await;
    let dir = tempfile::tempdir().unwrap();
    let kit = dir.path().join("kit");
    std::fs::create_dir_all(kit.join(".specify")).unwrap();
    let mixed = dir.path().join("mixed");
    std::fs::create_dir_all(mixed.join("openspec")).unwrap();
    std::fs::create_dir_all(mixed.join("_bmad")).unwrap();
    let bare = dir.path().join("bare");
    std::fs::create_dir_all(&bare).unwrap();
    let kit_s = session(&s, Some(repo(&s, kit.to_str().unwrap()).await), 10, &[prompt("hi")]).await;
    let mixed_s =
        session(&s, Some(repo(&s, mixed.to_str().unwrap()).await), 9, &[prompt("hi")]).await;
    let bare_s =
        session(&s, Some(repo(&s, bare.to_str().unwrap()).await), 8, &[prompt("hi")]).await;
    let gone = session(
        &s,
        Some(repo(&s, &format!("{}/gone", dir.path().display())).await),
        7,
        &[prompt("hi")],
    )
    .await;
    let r = pass(&s, NOW, 10_000).await.unwrap();
    assert_eq!(r.failed, 0, "a missing repo is not a failure (B6)");
    assert_eq!(framework(&s, kit_s).await, (Some("spec_kit".into()), Some("static".into())));
    assert_eq!(
        framework(&s, mixed_s).await,
        (Some("openspec".into()), Some("static_ambiguous".into()))
    );
    assert_eq!(framework(&s, bare_s).await, (None, None));
    assert_eq!(framework(&s, gone).await, (None, None));
}

#[tokio::test]
async fn a_failed_probe_is_an_explicit_state_that_keeps_the_signals_and_rests() {
    let s = PgStore::test_store().await;
    let folder = repo(&s, "/_fw/denied").await;
    let a = session(&s, Some(folder), 10, &[edit("/_fw/denied/src/a.rs")]).await;
    let b = session(&s, Some(folder), 9, &[prompt("hi")]).await;
    let calls = Arc::new(AtomicUsize::new(0));
    let probe = denying_probe(calls.clone());
    let r = pass_with(&s, NOW, 10_000, Arc::clone(&probe), Duration::from_secs(5)).await.unwrap();
    assert_eq!((r.derived, r.failed), (2, 0), "a failed probe does not fail the session (B10)");
    assert_eq!(framework(&s, a).await, (None, Some("probe_failed".into())));
    assert_eq!(framework(&s, b).await, (None, Some("probe_failed".into())));
    assert!(signal_count(&s, a).await > 0, "the session's signals are kept");
    assert_eq!(calls.load(Ordering::SeqCst), 1, "one probe per repo per pass");

    let again = pass_with(&s, NOW, 10_000, probe, Duration::from_secs(5)).await.unwrap();
    assert_eq!((again.derived, again.failed), (0, 0), "not retried until its inputs change");
}

#[tokio::test]
async fn a_probe_that_hangs_times_out_as_failed() {
    let s = PgStore::test_store().await;
    let folder = repo(&s, "/_fw/hung").await;
    let id = session(&s, Some(folder), 10, &[prompt("hi")]).await;
    let slow: Probe = Arc::new(|_: &std::path::Path| {
        std::thread::sleep(Duration::from_secs(3));
        Ok(true)
    });
    let r = tokio::time::timeout(
        Duration::from_secs(2),
        pass_with(&s, NOW, 10_000, slow, Duration::from_millis(200)),
    )
    .await
    .expect("the pass is not held by the probe")
    .unwrap();
    assert_eq!((r.derived, r.failed), (1, 0));
    assert_eq!(framework(&s, id).await, (None, Some("probe_failed".into())));
}

#[tokio::test]
async fn a_held_value_survives_its_repo_disappearing() {
    let s = PgStore::test_store().await;
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path().join("r");
    std::fs::create_dir_all(r.join(".specify")).unwrap();
    let id = session(&s, Some(repo(&s, r.to_str().unwrap()).await), 10, &[prompt("hi")]).await;
    pass(&s, NOW, 10_000).await.unwrap();
    assert_eq!(framework(&s, id).await, (Some("spec_kit".into()), Some("static".into())));

    std::fs::remove_dir_all(&r).unwrap();
    add_event(
        &s,
        &client_of(&s, id).await,
        "UserPromptSubmit",
        None,
        T0 + 60_000,
        json!({"prompt": "more"}),
    )
    .await;
    let again = pass(&s, NOW, 10_000).await.unwrap();
    assert_eq!(again.derived, 1);
    assert_eq!(framework(&s, id).await, (Some("spec_kit".into()), Some("static".into())), "B12");
}

#[tokio::test]
async fn an_old_mark_is_derived_again_once_after_new_sessions() {
    let s = PgStore::test_store().await;
    let old = session(&s, None, 600, &[prompt("/opsx:explore x")]).await;
    pass(&s, NOW, 10_000).await.unwrap();
    // As written before versions existed.
    exec(&s, "UPDATE activity.session_signal_marks SET derive_version = 0").await;
    exec(&s, "UPDATE activity.sessions SET sdd_framework = NULL, sdd_framework_source = NULL")
        .await;
    let new = session(&s, None, 1, &[prompt("/speckit.plan x")]).await;

    let first = pass(&s, NOW, 1).await.unwrap();
    assert_eq!(first.derived, 1);
    assert_eq!(
        framework(&s, new).await.0.as_deref(),
        Some("spec_kit"),
        "a new session goes first (B9)"
    );
    assert_eq!(framework(&s, old).await, (None, None));

    let second = pass(&s, NOW, 1).await.unwrap();
    assert_eq!(second.derived, 1);
    assert_eq!(framework(&s, old).await.0.as_deref(), Some("openspec"), "the backfill reached it");
    let (v,): (i32,) = sqlx_core::query_as::query_as(
        "SELECT derive_version FROM activity.session_signal_marks WHERE session_id = $1",
    )
    .bind(old)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(v, DERIVE_VERSION);
    let third = pass(&s, NOW, 10_000).await.unwrap();
    assert_eq!(third.derived, 0, "derived once");
}

#[tokio::test]
async fn a_failure_mark_carries_the_version_so_it_rests() {
    let s = PgStore::test_store().await;
    // A ts outside [2000, now + 1 day] fails derivation deterministically.
    let id = session(&s, None, 10, &[]).await;
    add_event(&s, &client_of(&s, id).await, "UserPromptSubmit", None, 1, json!({"prompt": "x"}))
        .await;
    let first = pass(&s, NOW, 10_000).await.unwrap();
    assert_eq!(first.failed, 1);
    let second = pass(&s, NOW, 10_000).await.unwrap();
    assert_eq!((second.derived, second.failed), (0, 0));
}

#[tokio::test]
async fn the_columns_reject_a_half_written_detection() {
    let s = PgStore::test_store().await;
    let id = session(&s, None, 10, &[]).await;
    for set in [
        "sdd_framework = 'openspec', sdd_framework_source = NULL",
        "sdd_framework = NULL, sdd_framework_source = 'static'",
        "sdd_framework = 'openspec', sdd_framework_source = 'probe_failed'",
        "sdd_framework = 'kiro', sdd_framework_source = 'guess'",
    ] {
        let r =
            sqlx_core::query::query(&format!("UPDATE activity.sessions SET {set} WHERE id = $1"))
                .bind(id)
                .execute(s.pool())
                .await;
        assert!(r.is_err(), "{set} must be rejected");
    }
    exec(
        &s,
        "UPDATE activity.sessions SET sdd_framework = NULL, sdd_framework_source = 'probe_failed'",
    )
    .await;
}
