use std::sync::Arc;
use tokio::sync::broadcast;
use crate::db::pg_store::PgStore;
use crate::tasks::queue::TaskQueue;
use crate::api::events::StateEvent;
use gateway::Gateway;

pub struct SharedState {
    pub pg: PgStore,
    pub task_queue: Arc<TaskQueue>,
    pub gateway: Arc<Gateway>,
    pub event_tx: broadcast::Sender<StateEvent>,
    /// Per-adapter capture-watchdog circuit-breaker state (in-memory; resets on restart).
    pub breaker: std::sync::Arc<crate::assistants::BreakerMap>,
    /// On-demand model-provisioning supervisor. `Some` only in an
    /// `embedded-llama-cpp` build on the default instance (see
    /// [`crate::api::gateway_init::init_gateway`]); `None` otherwise — the
    /// provisioning HTTP handlers report "not available in this build" then.
    /// The type is present in every build (the `local-engine` dep is
    /// non-optional); only its construction is feature-gated.
    pub provisioning: Option<Arc<local_engine::ProvisioningSupervisor>>,
}

pub type AppState = Arc<SharedState>;
