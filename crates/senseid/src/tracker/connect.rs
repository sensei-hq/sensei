//! Connect, re-connect and disconnect a tracker, and drain secret tombstones
//! (spec R3). The keyring is never touched while a DB transaction is open.

use std::sync::Arc;
use std::time::Duration;

use crate::db::pg_store::PgStore;
use crate::db::pg_store::trackers::{ConnectDbError, NewTracker};
use crate::tracker::secret::{KEYRING_TIMEOUT, SecretStore, bounded};
use crate::tracker::{ConnectParams, GetTransport, TrackerAdapter, TrackerLevel, ValidateError};

/// How long a new secret's pending tombstone waits before a drain may act on it.
pub const PENDING_GRACE: Duration = Duration::from_secs(10 * 60);

/// Why a connect was refused or failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectError {
    BoardAlreadyConnected { workspace_id: uuid::Uuid, workspace_name: String },
    Validate(ValidateError),
    CredentialStore,
    Db(String),
}

/// A persisted connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connected {
    pub tracker_id: uuid::Uuid,
    pub created: bool,
    /// The workspace's other boards on this site, moved to this connect's token.
    pub rekeyed: Vec<uuid::Uuid>,
}

/// Validate, store the token, and persist the connection (R3 steps 1–4). The
/// first sync is the caller's to start.
pub async fn connect(
    pg: &PgStore,
    store: Arc<dyn SecretStore>,
    adapter: &dyn TrackerAdapter,
    transport: &dyn GetTransport,
    workspace_id: uuid::Uuid,
    level: TrackerLevel,
    params: ConnectParams,
) -> Result<Connected, ConnectError> {
    let kind = adapter.kind().as_str();
    let owner = pg
        .tracker_board_owner(kind, &params.base_url, &params.board_ref)
        .await
        .map_err(ConnectError::Db)?;
    if let Some(owner) = owner.filter(|o| o.workspace_id != workspace_id) {
        return Err(ConnectError::BoardAlreadyConnected {
            workspace_id: owner.workspace_id,
            workspace_name: owner.workspace_name,
        });
    }
    let validated = adapter.validate(transport, &params).await.map_err(ConnectError::Validate)?;

    // 1. Record the new secret before it exists, so no crash can orphan it.
    let service = format!("{}{kind}.{}", store.namespace(), uuid::Uuid::new_v4());
    let grace =
        chrono::Duration::from_std(PENDING_GRACE).unwrap_or_else(|_| chrono::Duration::minutes(10));
    pg.write_pending_tombstone(&service, &params.account, chrono::Utc::now() + grace)
        .await
        .map_err(ConnectError::Db)?;

    // 2. Write it, outside any transaction, bounded.
    let (svc, account, token) = (service.clone(), params.account.clone(), params.token.clone());
    bounded(store.clone(), KEYRING_TIMEOUT, move |s| s.set(&svc, &account, &token))
        .await
        .map_err(|_| ConnectError::CredentialStore)?;

    // 3-4. Persist in one transaction; the swap/delete triggers tombstone the old secret.
    let row = NewTracker {
        workspace_id,
        kind: kind.to_string(),
        level: level.as_str().to_string(),
        base_url: params.base_url.clone(),
        account: params.account.clone(),
        external_scope: validated.external_scope,
        board_ref: params.board_ref.clone(),
        params: serde_json::json!({ "board_name": validated.board_name }),
        secret_service: service,
    };
    let (tracker_id, created, rekeyed) = match pg.connect_tracker(&row).await {
        Ok(ok) => ok,
        Err(ConnectDbError::BoardTaken) => {
            let owner = pg
                .tracker_board_owner(kind, &params.base_url, &params.board_ref)
                .await
                .map_err(ConnectError::Db)?
                .ok_or_else(|| ConnectError::Db("board taken, owner not found".into()))?;
            return Err(ConnectError::BoardAlreadyConnected {
                workspace_id: owner.workspace_id,
                workspace_name: owner.workspace_name,
            });
        }
        Err(ConnectDbError::PendingTombstoneGone) => {
            return Err(ConnectError::Db("new secret's pending tombstone was drained".into()));
        }
        Err(ConnectDbError::Db(e)) => return Err(ConnectError::Db(e)),
    };

    // Best effort, off the request path: the tick retries anything left.
    spawn_drain(pg, store);
    Ok(Connected { tracker_id, created, rekeyed })
}

/// Remove one of a workspace's trackers. `Ok(false)` when the workspace has no
/// such tracker. A secret the site's other trackers still use survives the drain.
pub async fn disconnect(
    pg: &PgStore,
    store: Arc<dyn SecretStore>,
    workspace_id: uuid::Uuid,
    tracker_id: uuid::Uuid,
) -> Result<bool, String> {
    if !pg.delete_tracker(workspace_id, tracker_id).await? {
        return Ok(false);
    }
    // The row is gone either way; a drain failure is the tick's to retry, not a
    // disconnect failure.
    spawn_drain(pg, store);
    Ok(true)
}

/// Drain in the background, so a slow or prompting keyring never holds a request.
fn spawn_drain(pg: &PgStore, store: Arc<dyn SecretStore>) {
    let pg = pg.clone();
    tokio::spawn(async move {
        if let Err(e) = drain_tombstones(&pg, store).await {
            tracing::warn!("tracker: tombstone drain failed: {e}");
        }
    });
}

/// How long a tombstone whose keyring delete failed waits before the next try,
/// so a permanently failing entry cannot hold the head of the queue.
pub const DELETE_RETRY: Duration = Duration::from_secs(15 * 60);

/// What a drain did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DrainReport {
    pub deleted: u32,
    pub referenced: u32,
    pub failed: u32,
}

/// Every keyring entry the tracker integration writes starts with this.
pub const SECRET_PREFIX: &str = "com.sensei.tracker.";

/// Delete every due tombstone's keyring entry, then the tombstone (R3 draining),
/// over the store's namespace.
pub async fn drain_tombstones(
    pg: &PgStore,
    store: Arc<dyn SecretStore>,
) -> Result<DrainReport, String> {
    let namespace = store.namespace();
    drain_matching(pg, store, &namespace).await
}

/// [`drain_tombstones`] over the services starting with `prefix`.
pub async fn drain_matching(
    pg: &PgStore,
    store: Arc<dyn SecretStore>,
    prefix: &str,
) -> Result<DrainReport, String> {
    let mut report = DrainReport::default();
    for (service, account) in pg.due_tombstones(prefix, 500).await? {
        // A live tracker still uses it: drop the tombstone, never the secret.
        match pg.release_referenced_tombstone(&service).await? {
            None => continue,
            Some(true) => {
                report.referenced += 1;
                continue;
            }
            Some(false) => {}
        }
        let (svc, acct) = (service.clone(), account.clone());
        match bounded(store.clone(), KEYRING_TIMEOUT, move |s| s.delete(&svc, &acct)).await {
            Ok(()) => {
                pg.delete_tombstone(&service).await?;
                report.deleted += 1;
            }
            Err(e) => {
                tracing::warn!(secret_service = %service, code = e.code(), "tracker: keyring delete failed; retrying later");
                let later = chrono::Utc::now()
                    + chrono::Duration::from_std(DELETE_RETRY)
                        .unwrap_or_else(|_| chrono::Duration::minutes(15));
                pg.defer_tombstone(&service, later).await?;
                report.failed += 1;
            }
        }
    }
    Ok(report)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::tracker::jira::JiraAdapter;
    use crate::tracker::secret::test_support::MemoryStore;
    use crate::tracker::test_support::FakeBoard;
    use crate::tracker::{ApiToken, TrackerError, TrackerLevel};

    pub(crate) async fn workspace(pg: &PgStore) -> uuid::Uuid {
        pg.create_workspace(&format!("_test:tracker-connect:{}", uuid::Uuid::new_v4()), None, None)
            .await
            .unwrap()
    }

    /// A unique Jira site so parallel tests never collide on the board key.
    fn params(site: &str, board: &str) -> ConnectParams {
        ConnectParams {
            base_url: site.into(),
            board_ref: board.into(),
            account: "dev@example.com".into(),
            token: ApiToken("SENTINEL-TOKEN-XYZ".into()),
            project_key: None,
        }
    }
    fn site() -> String {
        format!("https://t{}.atlassian.net", uuid::Uuid::new_v4().simple())
    }

    /// A connected tracker (board 42 of a unique site) on a fresh workspace.
    pub(crate) async fn connected_tracker(pg: &PgStore) -> (uuid::Uuid, uuid::Uuid) {
        let p = workspace(pg).await;
        let c = connect(
            pg,
            Arc::new(MemoryStore::default()),
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Sprint,
            params(&site(), "42"),
        )
        .await
        .unwrap();
        (c.tracker_id, p)
    }

    async fn tombstone_due(pg: &PgStore, service: &str) -> Option<bool> {
        sqlx_core::query_as::query_as::<_, (bool,)>(
            "SELECT not_before <= now() FROM sensei.tracker_secret_tombstones WHERE secret_service = $1",
        )
        .bind(service)
        .fetch_optional(pg.pool())
        .await
        .unwrap()
        .map(|(d,)| d)
    }

    #[tokio::test]
    async fn a_new_connect_stores_the_secret_and_starts_pending() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg).await;
        let store = Arc::new(MemoryStore::default());
        let s = site();
        let c = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Sprint,
            params(&s, "42"),
        )
        .await
        .unwrap();
        assert!(c.created);
        let row = pg.tracker_by_id(c.tracker_id).await.unwrap().unwrap();
        assert_eq!((row.base_url.as_str(), row.board_ref.as_str()), (s.as_str(), "42"));
        assert_eq!(
            (row.external_scope.as_str(), row.account.as_str()),
            ("AUTH", "dev@example.com")
        );
        assert_eq!(row.params["board_name"], serde_json::json!("AUTH Scrum"));
        assert!(row.secret_service.starts_with(&format!("{}jira.", store.namespace)));
        assert!(row.secret_service.starts_with(SECRET_PREFIX));
        assert!(store.has(&row.secret_service));
        assert_eq!(
            tombstone_due(&pg, &row.secret_service).await,
            None,
            "the live secret has no tombstone"
        );
        assert!(
            row.last_success_at.is_none() && row.last_error.is_none(),
            "pending until the first sync"
        );
    }

    #[tokio::test]
    async fn a_keyring_failure_writes_no_row_and_leaves_a_pending_tombstone() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg).await;
        let store = Arc::new(MemoryStore::default());
        *store.fail_set.lock().unwrap() = true;
        let r = connect(
            &pg,
            store,
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Sprint,
            params(&site(), "42"),
        )
        .await;
        assert_eq!(r, Err(ConnectError::CredentialStore));
        assert!(pg.trackers_for_workspace(p).await.unwrap().is_empty());
        let (pending,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.tracker_secret_tombstones
              WHERE account = 'dev@example.com' AND not_before > now() + interval '5 minutes'",
        )
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert!(
            pending >= 1,
            "the new secret's pending tombstone stays for the grace-period drain"
        );
    }

    #[tokio::test]
    async fn a_board_owned_by_another_workspace_is_refused_before_any_write() {
        let pg = PgStore::test_store().await;
        let (p1, p2) = (workspace(&pg).await, workspace(&pg).await);
        let s = site();
        connect(
            &pg,
            Arc::new(MemoryStore::default()),
            &JiraAdapter,
            &FakeBoard::standard(),
            p1,
            TrackerLevel::Sprint,
            params(&s, "42"),
        )
        .await
        .unwrap();
        let store = Arc::new(MemoryStore::default());
        let r = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &FakeBoard::standard(),
            p2,
            TrackerLevel::Sprint,
            params(&s, "42"),
        )
        .await;
        assert!(
            matches!(r, Err(ConnectError::BoardAlreadyConnected { workspace_id, .. }) if workspace_id == p1),
            "{r:?}"
        );
        assert!(store.entries.lock().unwrap().is_empty(), "nothing stored for a refused connect");
    }

    #[tokio::test]
    async fn a_failed_validation_writes_nothing() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg).await;
        let store = Arc::new(MemoryStore::default());
        let denied = FakeBoard::standard().fail("/rest/api/3/myself", TrackerError::Unauthorized);
        let r = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &denied,
            p,
            TrackerLevel::Sprint,
            params(&site(), "42"),
        )
        .await;
        assert_eq!(
            r,
            Err(ConnectError::Validate(ValidateError::Tracker(TrackerError::Unauthorized)))
        );
        assert!(pg.trackers_for_workspace(p).await.unwrap().is_empty());
        assert!(store.entries.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn reconnecting_the_same_board_swaps_the_secret_and_resets_status() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg).await;
        let store = Arc::new(MemoryStore::default());
        let s = site();
        let first = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Sprint,
            params(&s, "42"),
        )
        .await
        .unwrap();
        let old = pg.tracker_by_id(first.tracker_id).await.unwrap().unwrap();
        pg.mark_tracker_error(first.tracker_id, &old.secret_service, "unauthorized").await.unwrap();
        let again = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Sprint,
            params(&s, "42"),
        )
        .await
        .unwrap();
        assert_eq!((again.tracker_id, again.created), (first.tracker_id, false));
        let new = pg.tracker_by_id(again.tracker_id).await.unwrap().unwrap();
        assert_ne!(new.secret_service, old.secret_service);
        assert!(
            new.last_error.is_none() && new.last_success_at.is_none(),
            "pending for the new credential"
        );
        assert!(store.has(&new.secret_service));
        // The old secret is tombstoned (or already drained right after commit), never the new one.
        assert_ne!(tombstone_due(&pg, &old.secret_service).await, Some(false));
        assert_eq!(tombstone_due(&pg, &new.secret_service).await, None);
    }

    /// Board 43 of `FakeBoard::standard()`'s site, beside its board 42.
    fn with_board_43() -> FakeBoard {
        FakeBoard::standard().with(
            "/rest/agile/1.0/board/43",
            0,
            serde_json::json!({"id": 43, "name": "Portfolio", "location": {"projectKey": "AUTH"}}),
        )
    }

    async fn drained(pg: &PgStore, store: &MemoryStore, service: &str) -> bool {
        for _ in 0..200 {
            if !store.has(service) && tombstone_due(pg, service).await.is_none() {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        false
    }

    #[tokio::test]
    async fn connecting_another_board_adds_a_tracker_at_its_level() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg).await;
        let store = Arc::new(MemoryStore::default());
        let s = site();
        let sprint = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Sprint,
            params(&s, "42"),
        )
        .await
        .unwrap();
        crate::tracker::sync::sync_tracker(
            &pg,
            &crate::tracker::sync::TrackerLocks::default(),
            sprint.tracker_id,
            &JiraAdapter,
            &FakeBoard::standard(),
            crate::tracker::sync::SYNC_DEADLINE,
        )
        .await
        .unwrap();
        let epic = connect(
            &pg,
            store,
            &JiraAdapter,
            &with_board_43(),
            p,
            TrackerLevel::Epic,
            params(&s, "43"),
        )
        .await
        .unwrap();
        assert!(epic.created && epic.tracker_id != sprint.tracker_id);
        let rows = pg.trackers_for_workspace(p).await.unwrap();
        let levels: Vec<(uuid::Uuid, &str, &str)> =
            rows.iter().map(|t| (t.id, t.board_ref.as_str(), t.level.as_str())).collect();
        assert_eq!(levels, [(epic.tracker_id, "43", "epic"), (sprint.tracker_id, "42", "sprint")]);
        let (kept,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.tracker_tasks WHERE tracker_id = $1",
        )
        .bind(sprint.tracker_id)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert!(kept > 0, "the first board and its tickets stay");
    }

    #[tokio::test]
    async fn reconnecting_a_board_at_another_level_moves_it_and_adds_nothing() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg).await;
        let store = Arc::new(MemoryStore::default());
        let s = site();
        let first = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Sprint,
            params(&s, "42"),
        )
        .await
        .unwrap();
        let again = connect(
            &pg,
            store,
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Epic,
            params(&s, "42"),
        )
        .await
        .unwrap();
        assert_eq!((again.tracker_id, again.created), (first.tracker_id, false));
        let rows = pg.trackers_for_workspace(p).await.unwrap();
        assert_eq!(rows.iter().map(|t| t.level.as_str()).collect::<Vec<_>>(), ["epic"]);
    }

    #[tokio::test]
    async fn boards_on_one_site_share_its_newest_credential() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg).await;
        let store = Arc::new(MemoryStore::default());
        let s = site();
        let a = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Sprint,
            params(&s, "42"),
        )
        .await
        .unwrap();
        let old = pg.tracker_by_id(a.tracker_id).await.unwrap().unwrap().secret_service;
        let b = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &with_board_43(),
            p,
            TrackerLevel::Epic,
            params(&s, "43"),
        )
        .await
        .unwrap();
        let (ra, rb) = (
            pg.tracker_by_id(a.tracker_id).await.unwrap().unwrap(),
            pg.tracker_by_id(b.tracker_id).await.unwrap().unwrap(),
        );
        assert_eq!(ra.secret_service, rb.secret_service, "one credential per site (D14)");
        assert_ne!(ra.secret_service, old, "re-keyed to the newest token");
        assert!(store.has(&rb.secret_service));
        assert!(drained(&pg, &store, &old).await, "the superseded secret is deleted");
    }

    #[tokio::test]
    async fn another_site_keeps_its_own_credential() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg).await;
        let store = Arc::new(MemoryStore::default());
        let a = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Sprint,
            params(&site(), "42"),
        )
        .await
        .unwrap();
        let before = pg.tracker_by_id(a.tracker_id).await.unwrap().unwrap().secret_service;
        connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Sprint,
            params(&site(), "42"),
        )
        .await
        .unwrap();
        let after = pg.tracker_by_id(a.tracker_id).await.unwrap().unwrap().secret_service;
        assert_eq!(before, after);
        assert!(store.has(&after));
    }

    #[tokio::test]
    async fn disconnecting_one_board_keeps_the_secret_its_site_still_uses() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg).await;
        let store = Arc::new(MemoryStore::default());
        let s = site();
        let a = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Sprint,
            params(&s, "42"),
        )
        .await
        .unwrap();
        let b = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &with_board_43(),
            p,
            TrackerLevel::Epic,
            params(&s, "43"),
        )
        .await
        .unwrap();
        let shared = pg.tracker_by_id(b.tracker_id).await.unwrap().unwrap().secret_service;
        assert_eq!(disconnect(&pg, store.clone(), p, a.tracker_id).await, Ok(true));
        // Let the post-commit drain run: it must drop the tombstone, not the secret.
        for _ in 0..200 {
            if tombstone_due(&pg, &shared).await.is_none() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(
            tombstone_due(&pg, &shared).await,
            None,
            "the drain ran and dropped the tombstone of a secret still in use"
        );
        assert!(store.has(&shared), "board 43 still uses the site's credential");
        assert_eq!(
            pg.trackers_for_workspace(p).await.unwrap().iter().map(|t| t.id).collect::<Vec<_>>(),
            [b.tracker_id]
        );
    }

    #[tokio::test]
    async fn connecting_a_board_rekeys_its_siblings_and_clears_their_error() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg).await;
        let store = Arc::new(MemoryStore::default());
        let s = site();
        let a = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Sprint,
            params(&s, "42"),
        )
        .await
        .unwrap();
        let old = pg.tracker_by_id(a.tracker_id).await.unwrap().unwrap().secret_service;
        pg.mark_tracker_error(a.tracker_id, &old, "unauthorized").await.unwrap();
        let b = connect(
            &pg,
            store,
            &JiraAdapter,
            &with_board_43(),
            p,
            TrackerLevel::Epic,
            params(&s, "43"),
        )
        .await
        .unwrap();
        assert_eq!(b.rekeyed, [a.tracker_id], "the caller re-syncs the boards it re-keyed");
        let a_row = pg.tracker_by_id(a.tracker_id).await.unwrap().unwrap();
        assert_eq!(a_row.last_error, None, "the old token's failure no longer describes the board");
        assert!(a.rekeyed.is_empty());
    }

    #[tokio::test]
    async fn a_drain_racing_the_last_disconnect_still_deletes_the_secret() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg).await;
        let store = Arc::new(MemoryStore::default());
        let s = site();
        let a = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Sprint,
            params(&s, "42"),
        )
        .await
        .unwrap();
        let b = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &with_board_43(),
            p,
            TrackerLevel::Epic,
            params(&s, "43"),
        )
        .await
        .unwrap();
        let shared = pg.tracker_by_id(b.tracker_id).await.unwrap().unwrap().secret_service;
        // Board 42 goes first: the secret's tombstone is due while board 43 still uses it.
        assert!(pg.delete_tracker(p, a.tracker_id).await.unwrap());
        // Board 43's delete is in flight while a drain looks at the tombstone.
        let mut tx = pg.pool().begin().await.unwrap();
        sqlx_core::query::query("DELETE FROM sensei.trackers WHERE id = $1")
            .bind(b.tracker_id)
            .execute(&mut *tx)
            .await
            .unwrap();
        let drain = tokio::spawn({
            let (pg, store, prefix) = (pg.clone(), store.clone(), shared.clone());
            async move { drain_matching(&pg, store, &prefix).await }
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        tx.commit().await.unwrap();
        drain.await.unwrap().unwrap();
        assert!(
            !store.has(&shared) || tombstone_due(&pg, &shared).await.is_some(),
            "the secret is deleted, or still recorded for the next drain; never orphaned"
        );
    }

    #[tokio::test]
    async fn disconnect_names_a_tracker_of_its_own_workspace() {
        let pg = PgStore::test_store().await;
        let (t, p) = connected_tracker(&pg).await;
        let other = workspace(&pg).await;
        let store = Arc::new(MemoryStore::default());
        assert_eq!(disconnect(&pg, store, other, t).await, Ok(false));
        assert!(pg.tracker_by_id(t).await.unwrap().is_some(), "another workspace's tracker stays");
        let _ = p;
    }

    #[tokio::test]
    async fn persisting_refuses_a_secret_whose_pending_tombstone_was_drained() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg).await;
        let t = crate::db::pg_store::trackers::NewTracker {
            workspace_id: p,
            kind: "jira".into(),
            level: "sprint".into(),
            base_url: site(),
            account: "dev@example.com".into(),
            external_scope: "AUTH".into(),
            board_ref: "42".into(),
            params: serde_json::json!({}),
            secret_service: format!("com.sensei.tracker.jira.{}", uuid::Uuid::new_v4()),
        };
        // No pending tombstone was written (or a drain removed it).
        assert_eq!(
            pg.connect_tracker(&t).await,
            Err(crate::db::pg_store::trackers::ConnectDbError::PendingTombstoneGone)
        );
        assert!(pg.trackers_for_workspace(p).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn disconnect_removes_the_row_and_the_secret() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg).await;
        let store = Arc::new(MemoryStore::default());
        let c = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Sprint,
            params(&site(), "42"),
        )
        .await
        .unwrap();
        let service = pg.tracker_by_id(c.tracker_id).await.unwrap().unwrap().secret_service;
        assert_eq!(disconnect(&pg, store.clone(), p, c.tracker_id).await, Ok(true));
        assert!(pg.trackers_for_workspace(p).await.unwrap().is_empty());
        // The post-commit drain runs in the background.
        for _ in 0..200 {
            if !store.has(&service) && tombstone_due(&pg, &service).await.is_none() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(!store.has(&service), "keyring entry deleted after commit");
        assert_eq!(tombstone_due(&pg, &service).await, None, "and its tombstone drained");
        assert_eq!(disconnect(&pg, store, p, c.tracker_id).await, Ok(false));
    }

    #[tokio::test]
    async fn draining_skips_pending_and_referenced_and_keeps_failures() {
        let pg = PgStore::test_store().await;
        let store = Arc::new(MemoryStore::default());
        let now = chrono::Utc::now();
        let prefix = format!("{}drain.", store.namespace);
        let (due, pending) = (format!("{prefix}due"), format!("{prefix}pending"));
        pg.write_pending_tombstone(&due, "a", now - chrono::Duration::seconds(1)).await.unwrap();
        pg.write_pending_tombstone(&pending, "a", now + chrono::Duration::minutes(10))
            .await
            .unwrap();

        *store.fail_delete.lock().unwrap() = true;
        let report = drain_matching(&pg, store.clone(), &prefix).await.unwrap();
        assert_eq!(report.failed, 1, "only the due tombstone is attempted");
        assert_eq!(
            tombstone_due(&pg, &due).await,
            Some(false),
            "a keyring failure keeps the tombstone, deferred (not retried at once)"
        );
        // Once its retry time comes round, a working keyring deletes it.
        pg.defer_tombstone(&due, now - chrono::Duration::seconds(1)).await.unwrap();

        *store.fail_delete.lock().unwrap() = false;
        let report = drain_matching(&pg, store.clone(), &prefix).await.unwrap();
        assert_eq!(report.deleted, 1);
        assert_eq!(tombstone_due(&pg, &due).await, None);
        assert!(store.deletes.lock().unwrap().contains(&due));
        assert_eq!(tombstone_due(&pg, &pending).await, Some(false), "not due yet");

        // A live tracker's secret, wrongly tombstoned: drop the tombstone, keep the secret.
        let (t, _p) = connected_tracker(&pg).await;
        let live = pg.tracker_by_id(t).await.unwrap().unwrap().secret_service;
        pg.write_pending_tombstone(&live, "dev@example.com", now - chrono::Duration::seconds(1))
            .await
            .unwrap();
        drain_matching(&pg, store.clone(), &live).await.unwrap();
        assert_eq!(tombstone_due(&pg, &live).await, None, "a referenced tombstone is dropped...");
        assert!(
            !store.deletes.lock().unwrap().contains(&live),
            "...without deleting the live secret"
        );

        pg.delete_tombstone(&pending).await.ok();
    }

    /// Seed `n` due tombstones that a slow keyring will take ages to delete.
    async fn slow_backlog(pg: &PgStore, store: &MemoryStore, n: usize) {
        for _ in 0..n {
            let service = format!("{}jira.backlog-{}", store.namespace, uuid::Uuid::new_v4());
            pg.write_pending_tombstone(
                &service,
                "a",
                chrono::Utc::now() - chrono::Duration::seconds(1),
            )
            .await
            .unwrap();
        }
        *store.delete_delay.lock().unwrap() = Some(std::time::Duration::from_secs(2));
    }

    #[tokio::test]
    async fn connect_does_not_wait_for_the_tombstone_drain() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg).await;
        let store = Arc::new(MemoryStore::default());
        slow_backlog(&pg, &store, 3).await;
        let started = std::time::Instant::now();
        connect(
            &pg,
            store,
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Sprint,
            params(&site(), "42"),
        )
        .await
        .unwrap();
        assert!(
            started.elapsed() < std::time::Duration::from_millis(1500),
            "took {:?}",
            started.elapsed()
        );
    }

    #[tokio::test]
    async fn disconnect_does_not_wait_for_the_drain_and_succeeds_when_it_fails() {
        let pg = PgStore::test_store().await;
        let p = workspace(&pg).await;
        let store = Arc::new(MemoryStore::default());
        let c = connect(
            &pg,
            store.clone(),
            &JiraAdapter,
            &FakeBoard::standard(),
            p,
            TrackerLevel::Sprint,
            params(&site(), "42"),
        )
        .await
        .unwrap();
        slow_backlog(&pg, &store, 3).await;
        *store.fail_delete.lock().unwrap() = true;
        let started = std::time::Instant::now();
        assert_eq!(
            disconnect(&pg, store, p, c.tracker_id).await,
            Ok(true),
            "the row is gone; a drain failure is not a disconnect failure"
        );
        assert!(
            started.elapsed() < std::time::Duration::from_millis(1500),
            "took {:?}",
            started.elapsed()
        );
    }

    #[tokio::test]
    async fn a_failed_delete_backs_off_so_it_cannot_block_the_queue() {
        let pg = PgStore::test_store().await;
        let store = Arc::new(MemoryStore::default());
        let prefix = format!("test.backoff.{}.", uuid::Uuid::new_v4());
        let service = format!("{prefix}stuck");
        pg.write_pending_tombstone(
            &service,
            "a",
            chrono::Utc::now() - chrono::Duration::seconds(1),
        )
        .await
        .unwrap();
        *store.fail_delete.lock().unwrap() = true;
        let report = drain_matching(&pg, store, &prefix).await.unwrap();
        assert_eq!(report.failed, 1);
        let (later,): (bool,) = sqlx_core::query_as::query_as(
            "SELECT not_before > now() + interval '5 minutes'
               FROM sensei.tracker_secret_tombstones WHERE secret_service = $1",
        )
        .bind(&service)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert!(later, "a failed tombstone is pushed back, not retried at the head every tick");
        pg.delete_tombstone(&service).await.ok();
    }
}
