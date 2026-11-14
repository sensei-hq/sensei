//! What the daemon needs at runtime to talk to trackers: the per-tracker locks,
//! the secret store and the transport factory. One value in `SharedState`, with
//! the production pieces by default and fakes in tests.

use std::sync::Arc;

use crate::db::pg_store::PgStore;
use crate::db::pg_store::trackers::TrackerRow;
use crate::tracker::secret::{KEYRING_TIMEOUT, KeyringSecretStore, SecretStore, bounded};
use crate::tracker::sync::TrackerLocks;
use crate::tracker::transport::{PolicyTransport, ReqwestRaw};
use crate::tracker::{AuthScheme, GetTransport, TrackerAdapter, TrackerError, WriteTransport};

/// 429 retries per request during sync; connect's `validate` uses none.
pub const SYNC_RETRIES: u8 = 3;

/// Builds the GET transport for a base URL and auth scheme.
pub trait TransportFactory: Send + Sync {
    /// `graphql`: the adapter's [`TrackerAdapter::reads_graphql`], the only way a
    /// transport may POST to `/graphql` (tracker-github §H2.2).
    fn transport(
        &self,
        base_url: &str,
        auth: AuthScheme,
        retries: u8,
        graphql: bool,
    ) -> Result<Arc<dyn GetTransport>, TrackerError>;
    /// Builds the write transport; only the write-back pass calls it (W10).
    fn write_transport(
        &self,
        base_url: &str,
        auth: AuthScheme,
        retries: u8,
    ) -> Result<Arc<dyn WriteTransport>, TrackerError>;
}

/// The production factory: reqwest, bounded timeouts, redirects off, R4 policy.
pub struct ReqwestTransportFactory;

impl TransportFactory for ReqwestTransportFactory {
    fn transport(
        &self,
        base_url: &str,
        auth: AuthScheme,
        retries: u8,
        graphql: bool,
    ) -> Result<Arc<dyn GetTransport>, TrackerError> {
        let raw = ReqwestRaw::new(base_url, auth)?;
        let raw = if graphql { raw.allow_graphql() } else { raw };
        Ok(Arc::new(PolicyTransport::new(raw, retries)))
    }
    fn write_transport(
        &self,
        base_url: &str,
        auth: AuthScheme,
        retries: u8,
    ) -> Result<Arc<dyn WriteTransport>, TrackerError> {
        Ok(Arc::new(PolicyTransport::new(ReqwestRaw::new(base_url, auth)?, retries)))
    }
}

/// Tracker runtime shared by the API and the scheduled tick.
pub struct TrackerRuntime {
    pub locks: TrackerLocks,
    pub secrets: Arc<dyn SecretStore>,
    pub transports: Arc<dyn TransportFactory>,
    /// R11: per-tracker pause after a failed discovery.
    pub discovery: crate::tracker::discover::DiscoverBackoff,
}

impl Default for TrackerRuntime {
    fn default() -> Self {
        TrackerRuntime {
            locks: TrackerLocks::default(),
            secrets: Arc::new(KeyringSecretStore),
            transports: Arc::new(ReqwestTransportFactory),
            discovery: Default::default(),
        }
    }
}

/// The transport for a stored tracker: its token read from the secret store
/// (bounded), then the adapter's auth scheme.
pub async fn transport_for(
    rt: &TrackerRuntime,
    adapter: &dyn TrackerAdapter,
    row: &TrackerRow,
    retries: u8,
) -> Result<Arc<dyn GetTransport>, TrackerError> {
    let (service, account) = (row.secret_service.clone(), row.account.clone());
    let token =
        bounded(rt.secrets.clone(), KEYRING_TIMEOUT, move |s| s.get(&service, &account)).await?;
    rt.transports.transport(
        &row.base_url,
        adapter.auth(&row.account, &token),
        retries,
        adapter.reads_graphql(),
    )
}

/// Sync one stored tracker end to end, resolving its adapter and transport.
/// A credential failure is recorded like any other sync failure.
pub async fn sync_stored(
    pg: &PgStore,
    rt: &TrackerRuntime,
    row: &TrackerRow,
) -> Result<crate::db::pg_store::trackers::ApplyCounts, crate::tracker::sync::SyncError> {
    use crate::tracker::sync::{SYNC_DEADLINE, SyncError, sync_with_credential};
    let adapter = crate::tracker::TrackerKind::parse(&row.kind)
        .and_then(crate::tracker::adapter_for_kind)
        .ok_or(SyncError::Failed(TrackerError::UnsupportedTracker))?;
    let transport = match transport_for(rt, adapter, row, SYNC_RETRIES).await {
        Ok(t) => t,
        Err(e) => {
            if let Err(db) = pg.mark_tracker_error(row.id, &row.secret_service, e.code()).await {
                tracing::warn!(tracker_id = %row.id, "tracker sync: could not record failure: {db}");
            }
            return Err(SyncError::Failed(e));
        }
    };
    let counts = sync_with_credential(
        pg,
        &rt.locks,
        row.id,
        adapter,
        transport.as_ref(),
        SYNC_DEADLINE,
        &row.secret_service,
    )
    .await?;
    // Spawned, never awaited: the caller doesn't wait for the pass (W2).
    let _ =
        crate::tracker::writeback::spawn_after_sync(pg, rt, row.id, &row.secret_service, adapter);
    Ok(counts)
}

/// R11 discovery for one stored tracker, resolving its adapter and transport.
pub async fn discover_stored(
    pg: &PgStore,
    rt: &TrackerRuntime,
    row: &TrackerRow,
    scope: crate::tracker::discover::Scope<'_>,
) -> Result<crate::tracker::discover::DiscoverOutcome, crate::tracker::discover::DiscoverError> {
    use crate::tracker::discover::{DiscoverError, discover};
    let adapter = crate::tracker::TrackerKind::parse(&row.kind)
        .and_then(crate::tracker::adapter_for_kind)
        .ok_or(DiscoverError::Failed(TrackerError::UnsupportedTracker))?;
    let transport =
        transport_for(rt, adapter, row, SYNC_RETRIES).await.map_err(DiscoverError::Failed)?;
    discover(pg, &rt.locks, &rt.discovery, row.id, adapter, transport.as_ref(), scope).await
}

#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::collections::HashMap;

    /// Serves a fixed fake board per base URL; an unknown base URL is a transport failure.
    #[derive(Default)]
    pub struct FakeFactory {
        pub boards: std::sync::Mutex<HashMap<String, Arc<dyn GetTransport>>>,
        /// Write backends per base URL; an unserved one is a transport failure.
        pub writes:
            std::sync::Mutex<HashMap<String, Arc<crate::tracker::test_support::RecordingWrites>>>,
        /// The `retries` every write transport was built with.
        pub write_retries: std::sync::Mutex<Vec<u8>>,
    }

    /// A shared recording backend, so the test keeps a handle to its call log.
    struct SharedWrites(Arc<crate::tracker::test_support::RecordingWrites>);
    #[async_trait::async_trait]
    impl crate::tracker::transport::RawWrite for SharedWrites {
        async fn send(
            &self,
            method: crate::tracker::WriteMethod,
            path: &str,
            query: &[(&str, String)],
            body: &[u8],
        ) -> Result<crate::tracker::transport::RawResponse, ()> {
            self.0.send(method, path, query, body).await
        }
    }
    impl FakeFactory {
        /// Serve `board` for `base_url`; the handle lets a test read its call log.
        pub fn serve(
            &self,
            base_url: &str,
            board: crate::tracker::test_support::FakeBoard,
        ) -> Arc<crate::tracker::test_support::FakeBoard> {
            let board = Arc::new(board);
            self.boards.lock().unwrap().insert(base_url.to_string(), board.clone());
            board
        }
        /// Serve any transport for `base_url` (a recorded GitLab, for instance).
        pub fn serve_transport(&self, base_url: &str, t: Arc<dyn GetTransport>) {
            self.boards.lock().unwrap().insert(base_url.to_string(), t);
        }
        /// Record `writes` for `base_url`, behind the real 429 policy.
        pub fn serve_writes(
            &self,
            base_url: &str,
            writes: crate::tracker::test_support::RecordingWrites,
        ) -> Arc<crate::tracker::test_support::RecordingWrites> {
            let writes = Arc::new(writes);
            self.writes.lock().unwrap().insert(base_url.to_string(), writes.clone());
            writes
        }
    }
    impl TransportFactory for FakeFactory {
        fn transport(
            &self,
            base_url: &str,
            _auth: AuthScheme,
            _retries: u8,
            _graphql: bool,
        ) -> Result<Arc<dyn GetTransport>, TrackerError> {
            self.boards.lock().unwrap().get(base_url).cloned().ok_or(TrackerError::Transport)
        }
        fn write_transport(
            &self,
            base_url: &str,
            _auth: AuthScheme,
            retries: u8,
        ) -> Result<Arc<dyn WriteTransport>, TrackerError> {
            self.write_retries.lock().unwrap().push(retries);
            let writes = self
                .writes
                .lock()
                .unwrap()
                .get(base_url)
                .cloned()
                .ok_or(TrackerError::Transport)?;
            Ok(Arc::new(PolicyTransport::new(SharedWrites(writes), retries)))
        }
    }

    /// A runtime with an in-memory secret store and a fake transport factory.
    pub fn fake_runtime() -> (
        Arc<TrackerRuntime>,
        Arc<crate::tracker::secret::test_support::MemoryStore>,
        Arc<FakeFactory>,
    ) {
        let store = Arc::new(crate::tracker::secret::test_support::MemoryStore::default());
        let factory = Arc::new(FakeFactory::default());
        let rt = Arc::new(TrackerRuntime {
            locks: TrackerLocks::default(),
            secrets: store.clone(),
            transports: factory.clone(),
            discovery: Default::default(),
        });
        (rt, store, factory)
    }
}

#[cfg(test)]
mod factory_tests {
    use super::*;

    /// A server that counts the GraphQL POSTs it receives and answers `{"data": {}}`.
    async fn graphql_server() -> (String, Arc<std::sync::atomic::AtomicU32>) {
        let hits = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let h = Arc::clone(&hits);
        let app = axum::Router::new().route(
            "/graphql",
            axum::routing::post(move || {
                h.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async { axum::Json(serde_json::json!({"data": {"ok": true}})) }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (format!("http://{addr}"), hits)
    }

    #[tokio::test]
    async fn only_a_transport_built_for_graphql_posts_to_it() {
        // tracker-github §H2.2: a Jira or GitLab credential never leaves in a GraphQL POST.
        let (base, hits) = graphql_server().await;
        let q = crate::tracker::graphql::GraphqlQuery::new("query A { ok }", serde_json::json!({}))
            .unwrap();
        let auth = || AuthScheme::Header {
            name: "Authorization",
            token: crate::tracker::ApiToken("Bearer t".into()),
        };
        let plain = ReqwestTransportFactory.transport(&base, auth(), 0, false).unwrap();
        assert_eq!(plain.graphql(&q).await, Err(TrackerError::Transport));
        assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 0, "nothing was sent");
        let reader = ReqwestTransportFactory.transport(&base, auth(), 0, true).unwrap();
        assert_eq!(reader.graphql(&q).await, Ok(serde_json::json!({"ok": true})));
        assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 1);
        // Only GitHub reads GraphQL; Jira's and GitLab's credentials never reach it.
        for adapter in crate::tracker::adapters() {
            let reads = adapter.reads_graphql();
            assert_eq!(
                reads,
                adapter.kind() == crate::tracker::TrackerKind::Github,
                "{:?}",
                adapter.kind()
            );
        }
    }
}
