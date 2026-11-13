//! A ticket's SDD framework, pinned from its linked sessions against a real
//! database (blueprint 2026-11-10-ticket-sdd-framework B1, B11, B13–B19; #391).
//! Each test has its own database.
use super::*;
use crate::db::pg_store::TrackerSnapshotApply;
use crate::db::pg_store::trackers::schema_tests::insert_tracker;
use crate::db::pg_store::trackers::{PinScope, pin_ticket_frameworks};
use crate::tracker::test_support::repo_in;
use crate::tracker::{Link, LinkSource, Snapshot, TaskRow};
use serde_json::json;
use std::time::Duration;

/// A workspace with a Jira tracker (tickets AUTH-1, AUTH-2) and a repository folder
/// in it, so a session anchored there is one of the tracker's R1 candidates.
struct World {
    pg: PgStore,
    workspace: uuid::Uuid,
    tracker: uuid::Uuid,
    svc: String,
    repo: uuid::Uuid,
}

async fn world() -> World {
    let pg = PgStore::test_store().await;
    let workspace = pg
        .create_workspace(&format!("_test:tfw:{}", uuid::Uuid::new_v4()), None, None)
        .await
        .unwrap();
    let (tracker, svc) = tracker_in(&pg, workspace, "42").await;
    let repo = repo_in(&pg, workspace).await;
    World { pg, workspace, tracker, svc, repo }
}

async fn tracker_in(pg: &PgStore, workspace: uuid::Uuid, board: &str) -> (uuid::Uuid, String) {
    let svc = format!("svc-{}", uuid::Uuid::new_v4());
    let url = format!("https://f{}.atlassian.net", uuid::Uuid::new_v4().simple());
    (insert_tracker(pg, workspace, &url, board, &svc).await.unwrap(), svc)
}

/// A tracker sync of AUTH-1 (`10001`) and AUTH-2 (`10002`) relinking exactly `links`
/// (session, ticket external id).
async fn sync(pg: &PgStore, tracker: uuid::Uuid, svc: &str, links: &[(uuid::Uuid, &str)]) {
    let task = |ext: &str, key: &str| TaskRow {
        external_id: ext.into(),
        key: key.into(),
        ..Default::default()
    };
    let snapshot = Snapshot {
        tasks: vec![task("10001", "AUTH-1"), task("10002", "AUTH-2")],
        ..Default::default()
    };
    let links: Vec<Link> = links
        .iter()
        .map(|&(session_id, ext)| Link {
            session_id,
            task_external_id: ext.into(),
            source: LinkSource::Prompt,
            matched_key: if ext == "10001" { "AUTH-1".into() } else { "AUTH-2".into() },
            mentions: 1,
        })
        .collect();
    pg.apply_tracker_snapshot(
        tracker,
        &TrackerSnapshotApply {
            secret_service: svc,
            snapshot: &snapshot,
            params_patch: &json!({}),
            links: &links,
        },
    )
    .await
    .unwrap();
}

async fn relink(w: &World, links: &[(uuid::Uuid, &str)]) {
    sync(&w.pg, w.tracker, &w.svc, links).await;
}

/// A session in `repo` that started `minutes_ago`, completed or still running, with
/// `(sdd_framework, sdd_framework_source)` as #390 stores them.
async fn session(
    pg: &PgStore,
    repo: Option<uuid::Uuid>,
    minutes_ago: i32,
    completed: bool,
    detection: (Option<&str>, Option<&str>),
) -> uuid::Uuid {
    let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO activity.sessions
                (client_session_id, repo_folder_id, started_at, completed_at,
                 sdd_framework, sdd_framework_source)
         VALUES ($1, $2, now() - make_interval(mins => $3),
                 CASE WHEN $4 THEN now() END, $5::sensei.sdd_framework, $6)
         RETURNING id",
    )
    .bind(format!("tfw-{}", uuid::Uuid::new_v4()))
    .bind(repo)
    .bind(minutes_ago)
    .bind(completed)
    .bind(detection.0)
    .bind(detection.1)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    id
}

const OPENSPEC: (Option<&str>, Option<&str>) = (Some("openspec"), Some("behavioural"));
const SPEC_KIT: (Option<&str>, Option<&str>) = (Some("spec_kit"), Some("behavioural"));
const NONE: (Option<&str>, Option<&str>) = (None, None);

/// Re-derive a session's stored detection (what #390's pass writes), and whether it
/// has completed.
async fn redetect(
    pg: &PgStore,
    id: uuid::Uuid,
    detection: (Option<&str>, Option<&str>),
    completed: bool,
) {
    sqlx_core::query::query(
        "UPDATE activity.sessions
            SET sdd_framework = $2::sensei.sdd_framework, sdd_framework_source = $3,
                completed_at = CASE WHEN $4 THEN now() END
          WHERE id = $1",
    )
    .bind(id)
    .bind(detection.0)
    .bind(detection.1)
    .bind(completed)
    .execute(pg.pool())
    .await
    .unwrap();
}

/// `(sdd_framework, sdd_framework_session_id, sdd_framework_at is set)` of a ticket.
async fn ticket(
    pg: &PgStore,
    tracker: uuid::Uuid,
    ext: &str,
) -> (Option<String>, Option<uuid::Uuid>, bool) {
    sqlx_core::query_as::query_as(
        "SELECT sdd_framework::text, sdd_framework_session_id, sdd_framework_at IS NOT NULL
           FROM sensei.tracker_tasks WHERE tracker_id = $1 AND external_id = $2",
    )
    .bind(tracker)
    .bind(ext)
    .fetch_one(pg.pool())
    .await
    .unwrap()
}

async fn auth_1(w: &World) -> (Option<String>, Option<uuid::Uuid>, bool) {
    ticket(&w.pg, w.tracker, "10001").await
}

fn pinned(fw: &str, s: uuid::Uuid) -> (Option<String>, Option<uuid::Uuid>, bool) {
    (Some(fw.into()), Some(s), true)
}

const UNPINNED: (Option<String>, Option<uuid::Uuid>, bool) = (None, None, false);

#[tokio::test]
async fn a_sync_pins_the_earliest_eligible_linked_session() {
    let w = world().await;
    let early = session(&w.pg, Some(w.repo), 60, true, OPENSPEC).await;
    let late = session(&w.pg, Some(w.repo), 30, true, SPEC_KIT).await;
    relink(&w, &[(late, "10001"), (early, "10001")]).await;
    assert_eq!(auth_1(&w).await, pinned("openspec", early));
    assert_eq!(ticket(&w.pg, w.tracker, "10002").await, UNPINNED, "no session linked");
}

/// B1, B13: first detected wins; no other session changes it, though it started
/// earlier and is linked or derived later.
#[tokio::test]
async fn another_session_never_changes_a_pinned_ticket() {
    let w = world().await;
    let first = session(&w.pg, Some(w.repo), 30, true, OPENSPEC).await;
    relink(&w, &[(first, "10001")]).await;
    let earlier = session(&w.pg, Some(w.repo), 90, true, SPEC_KIT).await;
    relink(&w, &[(first, "10001"), (earlier, "10001")]).await;
    assert_eq!(auth_1(&w).await, pinned("openspec", first), "a later sync");
    w.pg.pin_session_tickets(earlier).await.unwrap();
    assert_eq!(auth_1(&w).await, pinned("openspec", first), "the pass for the other session");
}

/// B11: an ambiguous, failed or absent detection never pins.
#[tokio::test]
async fn ambiguous_failed_and_absent_detections_never_pin() {
    let w = world().await;
    let ambiguous =
        session(&w.pg, Some(w.repo), 30, true, (Some("openspec"), Some("static_ambiguous"))).await;
    let failed = session(&w.pg, Some(w.repo), 20, true, (None, Some("probe_failed"))).await;
    let none = session(&w.pg, Some(w.repo), 10, true, NONE).await;
    relink(&w, &[(ambiguous, "10001"), (failed, "10001"), (none, "10001")]).await;
    assert_eq!(auth_1(&w).await, UNPINNED);
    let static_ = session(&w.pg, Some(w.repo), 5, true, (Some("bmad"), Some("static"))).await;
    relink(&w, &[(ambiguous, "10001"), (static_, "10001")]).await;
    assert_eq!(auth_1(&w).await, pinned("bmad", static_), "static is eligible");
}

/// B17: a session that is not one of the tracker's R1 candidates never pins.
#[tokio::test]
async fn a_session_outside_the_workspace_never_pins() {
    let w = world().await;
    let elsewhere = session(&w.pg, None, 60, true, OPENSPEC).await;
    let candidate = session(&w.pg, Some(w.repo), 30, true, SPEC_KIT).await;
    relink(&w, &[(elsewhere, "10001")]).await;
    assert_eq!(auth_1(&w).await, UNPINNED);
    relink(&w, &[(elsewhere, "10001"), (candidate, "10001")]).await;
    assert_eq!(auth_1(&w).await, pinned("spec_kit", candidate));
}

/// B16: a tracker's apply pins its own tickets, never another tracker's.
#[tokio::test]
async fn a_sync_pins_only_its_own_tracker() {
    let w = world().await;
    let (other, other_svc) = tracker_in(&w.pg, w.workspace, "43").await;
    let s = session(&w.pg, Some(w.repo), 30, true, NONE).await;
    sync(&w.pg, other, &other_svc, &[(s, "10001")]).await;
    redetect(&w.pg, s, OPENSPEC, true).await;
    relink(&w, &[]).await;
    assert_eq!(ticket(&w.pg, other, "10001").await, UNPINNED);
    sync(&w.pg, other, &other_svc, &[(s, "10001")]).await;
    assert_eq!(ticket(&w.pg, other, "10001").await, pinned("openspec", s));
}

/// B14: the ticket follows its running provenance session, is cleared when that
/// session turns ambiguous, and freezes once it completes.
#[tokio::test]
async fn a_ticket_follows_its_running_session_until_it_completes() {
    let w = world().await;
    let s = session(&w.pg, Some(w.repo), 30, false, (Some("openspec"), Some("static"))).await;
    relink(&w, &[(s, "10001")]).await;
    assert_eq!(auth_1(&w).await, pinned("openspec", s));

    redetect(&w.pg, s, SPEC_KIT, false).await;
    w.pg.pin_session_tickets(s).await.unwrap();
    assert_eq!(auth_1(&w).await, pinned("spec_kit", s), "followed while running");

    redetect(&w.pg, s, (Some("openspec"), Some("static_ambiguous")), false).await;
    w.pg.pin_session_tickets(s).await.unwrap();
    assert_eq!(auth_1(&w).await, UNPINNED, "ambiguous clears it");

    redetect(&w.pg, s, SPEC_KIT, true).await;
    w.pg.pin_session_tickets(s).await.unwrap();
    assert_eq!(auth_1(&w).await, pinned("spec_kit", s), "pinned afresh");

    redetect(&w.pg, s, OPENSPEC, true).await;
    w.pg.pin_session_tickets(s).await.unwrap();
    relink(&w, &[(s, "10001")]).await;
    assert_eq!(auth_1(&w).await, pinned("spec_kit", s), "frozen once completed");
}

#[tokio::test]
async fn losing_the_provenance_link_or_session_keeps_the_value() {
    let w = world().await;
    let s = session(&w.pg, Some(w.repo), 30, true, OPENSPEC).await;
    relink(&w, &[(s, "10001")]).await;
    relink(&w, &[]).await;
    assert_eq!(auth_1(&w).await, pinned("openspec", s), "relinked without it");
    sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1")
        .bind(s)
        .execute(w.pg.pool())
        .await
        .unwrap();
    assert_eq!(auth_1(&w).await, (Some("openspec".into()), None, true), "session deleted");
}

/// The pin is written on the caller's connection: it commits or rolls back with the
/// apply that ran it.
#[tokio::test]
async fn a_pin_rolls_back_with_its_transaction() {
    let w = world().await;
    let s = session(&w.pg, Some(w.repo), 30, true, NONE).await;
    relink(&w, &[(s, "10001")]).await;
    redetect(&w.pg, s, OPENSPEC, true).await;
    let mut tx = w.pg.pool().begin().await.unwrap();
    assert_eq!(pin_ticket_frameworks(&mut tx, PinScope::Tracker(w.tracker)).await.unwrap(), 1);
    tx.rollback().await.unwrap();
    assert_eq!(auth_1(&w).await, UNPINNED);
}

/// B18: a session pruned while the pin runs is skipped, not an FK failure.
#[tokio::test]
async fn a_session_pruned_during_the_pin_is_skipped() {
    let w = world().await;
    let s = session(&w.pg, Some(w.repo), 30, true, NONE).await;
    relink(&w, &[(s, "10001")]).await;
    redetect(&w.pg, s, OPENSPEC, true).await;
    let mut prune = w.pg.pool().begin().await.unwrap();
    sqlx_core::query::query("DELETE FROM activity.sessions WHERE id = $1")
        .bind(s)
        .execute(&mut *prune)
        .await
        .unwrap();
    let pg = w.pg.clone();
    let pin = tokio::spawn(async move { pg.pin_session_tickets(s).await });
    tokio::time::sleep(Duration::from_millis(300)).await;
    prune.commit().await.unwrap();
    assert_eq!(pin.await.unwrap(), Ok(0));
    assert_eq!(auth_1(&w).await, UNPINNED);
}

/// A session's detection reaches its tickets through the signals pass (B15).
async fn session_with_prompt(w: &World, prompt: &str) -> uuid::Uuid {
    let s = session(&w.pg, Some(w.repo), 30, true, NONE).await;
    sqlx_core::query::query(
        "INSERT INTO activity.assistant_events (session_id, family, event_type, ts, payload)
         SELECT client_session_id, 'claude', 'UserPromptSubmit',
                (extract(epoch FROM now()) * 1000)::bigint - 60000, $2
           FROM activity.sessions WHERE id = $1",
    )
    .bind(s)
    .bind(json!({ "prompt": prompt }))
    .execute(w.pg.pool())
    .await
    .unwrap();
    s
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[tokio::test]
async fn the_signals_pass_pins_the_session_s_tickets() {
    let w = world().await;
    let s = session_with_prompt(&w, "/opsx:apply add-auth").await;
    relink(&w, &[(s, "10001")]).await;
    assert_eq!(auth_1(&w).await, UNPINNED, "not detected yet");
    let r = crate::tasks::session_signals::pass(&w.pg, now_ms(), 10_000).await.unwrap();
    assert_eq!((r.derived, r.failed), (1, 0));
    assert_eq!(auth_1(&w).await, pinned("openspec", s));
}

/// B15: a pin that cannot take its ticket's lock fails on its own: the session's
/// write stands, and the next tracker sync pins it.
#[tokio::test]
async fn a_blocked_pin_leaves_the_session_derived_and_a_sync_heals_it() {
    let w = world().await;
    let s = session_with_prompt(&w, "/opsx:apply add-auth").await;
    relink(&w, &[(s, "10001")]).await;
    let mut sync_tx = w.pg.pool().begin().await.unwrap();
    sqlx_core::query::query(
        "SELECT 1 FROM sensei.tracker_tasks WHERE tracker_id = $1 AND external_id = '10001' FOR UPDATE",
    )
    .bind(w.tracker)
    .execute(&mut *sync_tx)
    .await
    .unwrap();
    let r = tokio::time::timeout(
        Duration::from_secs(20),
        crate::tasks::session_signals::pass(&w.pg, now_ms(), 10_000),
    )
    .await
    .expect("the pin's lock wait is bounded")
    .unwrap();
    assert_eq!((r.derived, r.failed), (1, 0), "a pin failure is not a session failure");
    let (fw,): (Option<String>,) = sqlx_core::query_as::query_as(
        "SELECT sdd_framework::text FROM activity.sessions WHERE id = $1",
    )
    .bind(s)
    .fetch_one(w.pg.pool())
    .await
    .unwrap();
    assert_eq!(fw.as_deref(), Some("openspec"), "the session's write stands");
    sync_tx.rollback().await.unwrap();
    assert_eq!(auth_1(&w).await, UNPINNED);
    relink(&w, &[(s, "10001")]).await;
    assert_eq!(auth_1(&w).await, pinned("openspec", s), "the next sync heals it");
}

#[tokio::test]
async fn the_ticket_detail_carries_the_pinned_framework() {
    let w = world().await;
    let s = session(&w.pg, Some(w.repo), 30, true, OPENSPEC).await;
    relink(&w, &[(s, "10001")]).await;
    let one = w.pg.tracker_task_detail(w.workspace, "AUTH-1").await.unwrap().unwrap();
    assert_eq!(one.sdd_framework.as_deref(), Some("openspec"));
    let two = w.pg.tracker_task_detail(w.workspace, "AUTH-2").await.unwrap().unwrap();
    assert_eq!(two.sdd_framework, None);
}

#[tokio::test]
async fn the_ticket_columns_reject_a_half_written_pin() {
    let w = world().await;
    relink(&w, &[]).await;
    for set in [
        "sdd_framework = 'openspec', sdd_framework_at = NULL",
        "sdd_framework = NULL, sdd_framework_at = now()",
    ] {
        let r = sqlx_core::query::query(&format!(
            "UPDATE sensei.tracker_tasks SET {set} WHERE tracker_id = $1"
        ))
        .bind(w.tracker)
        .execute(w.pg.pool())
        .await;
        assert!(r.is_err(), "{set} must be rejected");
    }
}

/// B14 + B13: a ticket in the session's scope only as its provenance (its link is
/// gone) that the follow clears is pinned afresh in the same call, from the other
/// eligible session still linked; and it counts once.
#[tokio::test]
async fn a_cleared_ticket_is_pinned_afresh_though_its_link_is_gone() {
    let w = world().await;
    let s1 = session(&w.pg, Some(w.repo), 60, false, (Some("bmad"), Some("static"))).await;
    let s2 = session(&w.pg, Some(w.repo), 30, true, (Some("openspec"), Some("static"))).await;
    relink(&w, &[(s1, "10001"), (s2, "10001")]).await;
    assert_eq!(auth_1(&w).await, pinned("bmad", s1));
    relink(&w, &[(s2, "10001")]).await;
    redetect(&w.pg, s1, (Some("bmad"), Some("static_ambiguous")), false).await;
    assert_eq!(w.pg.pin_session_tickets(s1).await, Ok(1), "one ticket changed");
    assert_eq!(auth_1(&w).await, pinned("openspec", s2));
}

/// The pass's pin never waits on a ticket another transaction holds (a tracker apply
/// mid-upsert): it pins what it can and leaves the rest to the next sync (B15), so
/// it can never hold one ticket while waiting for another.
#[tokio::test]
async fn the_pass_pin_skips_a_ticket_another_transaction_holds() {
    let w = world().await;
    let s = session(&w.pg, Some(w.repo), 30, true, NONE).await;
    relink(&w, &[(s, "10001"), (s, "10002")]).await;
    redetect(&w.pg, s, OPENSPEC, true).await;
    let mut apply = w.pg.pool().begin().await.unwrap();
    sqlx_core::query::query(
        "UPDATE sensei.tracker_tasks SET last_seen_at = now()
          WHERE tracker_id = $1 AND external_id = '10002'",
    )
    .bind(w.tracker)
    .execute(&mut *apply)
    .await
    .unwrap();
    let started = std::time::Instant::now();
    let r = w.pg.pin_session_tickets(s).await;
    assert!(started.elapsed() < Duration::from_secs(2), "waited {:?}", started.elapsed());
    assert_eq!(r, Ok(1));
    assert_eq!(auth_1(&w).await, pinned("openspec", s));
    apply.commit().await.unwrap();
    assert_eq!(ticket(&w.pg, w.tracker, "10002").await, UNPINNED, "left to the next sync");
}

/// B14: a running provenance session that turns ambiguous clears the ticket even
/// when its framework is unchanged: ambiguous is never a ticket's value (B11).
#[tokio::test]
async fn an_ambiguous_session_clears_its_ticket_though_its_framework_is_unchanged() {
    let w = world().await;
    let s = session(&w.pg, Some(w.repo), 30, false, (Some("openspec"), Some("static"))).await;
    relink(&w, &[(s, "10001")]).await;
    assert_eq!(auth_1(&w).await, pinned("openspec", s));
    redetect(&w.pg, s, (Some("openspec"), Some("static_ambiguous")), false).await;
    w.pg.pin_session_tickets(s).await.unwrap();
    assert_eq!(auth_1(&w).await, UNPINNED);
}

/// The sync's pin is part of its apply: an apply that fails after writing its links
/// leaves no pin.
#[tokio::test]
async fn an_apply_that_fails_leaves_no_pin() {
    let w = world().await;
    let s = session(&w.pg, Some(w.repo), 30, true, NONE).await;
    relink(&w, &[(s, "10001")]).await;
    redetect(&w.pg, s, OPENSPEC, true).await;
    // The apply's last write, after its links: made to fail.
    for sql in [
        "CREATE FUNCTION sensei.test_refuse() RETURNS trigger LANGUAGE plpgsql
            AS $$ BEGIN RAISE EXCEPTION 'refused'; END $$",
        "CREATE TRIGGER refuse_tracker_update BEFORE UPDATE ON sensei.trackers
            FOR EACH ROW EXECUTE FUNCTION sensei.test_refuse()",
    ] {
        sqlx_core::query::query(sql).execute(w.pg.pool()).await.unwrap();
    }
    let snapshot = Snapshot {
        tasks: vec![TaskRow {
            external_id: "10001".into(),
            key: "AUTH-1".into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let link = Link {
        session_id: s,
        task_external_id: "10001".into(),
        source: LinkSource::Prompt,
        matched_key: "AUTH-1".into(),
        mentions: 1,
    };
    let applied =
        w.pg.apply_tracker_snapshot(
            w.tracker,
            &TrackerSnapshotApply {
                secret_service: &w.svc,
                snapshot: &snapshot,
                params_patch: &json!({}),
                links: &[link],
            },
        )
        .await;
    assert!(applied.is_err(), "the apply failed");
    assert_eq!(auth_1(&w).await, UNPINNED);
}
