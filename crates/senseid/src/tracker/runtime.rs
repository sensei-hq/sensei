//! What the daemon needs at runtime to talk to trackers: the per-tracker locks,
//! the secret store and the transport factory. One value in `SharedState`, with
//! the production pieces by default and fakes in tests.

use std::sync::Arc;

use crate::db::pg_store::PgStore;
use crate::db::pg_store::trackers::TrackerRow;
use crate::tracker::secret::{KEYRING_TIMEOUT, KeyringSecretStore, SecretStore, bounded};
use crate::tracker::sync::TrackerLocks;
use crate::tracker::transport::{PolicyTransport, ReqwestRaw};
use crate::tracker::{AuthScheme, GetTransport, TrackerAdapter, TrackerError};

/// 429 retries per request during sync; connect's `validate` uses none.
pub const SYNC_RETRIES: u8 = 3;

/// Builds the GET transport for a base URL and auth scheme.
pub trait TransportFactory: Send + Sync {
    fn transport(
        &self,
        base_url: &str,
        auth: AuthScheme,
        retries: u8,
    ) -> Result<Arc<dyn GetTransport>, TrackerError>;
}

/// The production factory: reqwest, bounded timeouts, redirects off, R4 policy.
pub struct ReqwestTransportFactory;

impl TransportFactory for ReqwestTransportFactory {
    fn transport(
        &self,
        base_url: &str,
        auth: AuthScheme,
        retries: u8,
    ) -> Result<Arc<dyn GetTransport>, TrackerError> {
        Ok(Arc::new(PolicyTransport::new(ReqwestRaw::new(base_url, auth)?, retries)))
    }
}

/// Tracker runtime shared by the API and the scheduled tick.
pub struct TrackerRuntime {
    pub locks: TrackerLocks,
    pub secrets: Arc<dyn SecretStore>,
    pub transports: Arc<dyn TransportFactory>,
}

impl Default for TrackerRuntime {
    fn default() -> Self {
        TrackerRuntime {
            locks: TrackerLocks::default(),
            secrets: Arc::new(KeyringSecretStore),
            transports: Arc::new(ReqwestTransportFactory),
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
    rt.transports.transport(&row.base_url, adapter.auth(&row.account, &token), retries)
}

/// Sync one stored tracker end to end, resolving its adapter and transport.
/// A credential failure is recorded like any other sync failure.
pub async fn sync_stored(
    pg: &PgStore,
    rt: &TrackerRuntime,
    row: &TrackerRow,
) -> Result<crate::db::pg_store::trackers::ApplyCounts, crate::tracker::sync::SyncError> {
    use crate::tracker::sync::{SYNC_DEADLINE, SyncError, sync_tracker};
    let adapter = crate::tracker::TrackerKind::parse(&row.kind)
        .and_then(crate::tracker::adapter_for_kind)
        .ok_or(SyncError::Failed(TrackerError::UnsupportedTracker))?;
    let transport = match transport_for(rt, adapter.as_ref(), row, SYNC_RETRIES).await {
        Ok(t) => t,
        Err(e) => {
            if let Err(db) = pg.mark_tracker_error(row.id, e.code()).await {
                tracing::warn!(tracker_id = %row.id, "tracker sync: could not record failure: {db}");
            }
            return Err(SyncError::Failed(e));
        }
    };
    sync_tracker(pg, &rt.locks, row.id, adapter.as_ref(), transport.as_ref(), SYNC_DEADLINE).await
}

#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::collections::HashMap;

    /// Serves a fixed fake board per base URL; an unknown base URL is a transport failure.
    #[derive(Default)]
    pub struct FakeFactory {
        pub boards: std::sync::Mutex<HashMap<String, Arc<dyn GetTransport>>>,
    }
    impl FakeFactory {
        pub fn serve(&self, base_url: &str, board: crate::tracker::test_support::FakeBoard) {
            self.boards.lock().unwrap().insert(base_url.to_string(), Arc::new(board));
        }
    }
    impl TransportFactory for FakeFactory {
        fn transport(
            &self,
            base_url: &str,
            _auth: AuthScheme,
            _retries: u8,
        ) -> Result<Arc<dyn GetTransport>, TrackerError> {
            self.boards.lock().unwrap().get(base_url).cloned().ok_or(TrackerError::Transport)
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
        });
        (rt, store, factory)
    }
}
