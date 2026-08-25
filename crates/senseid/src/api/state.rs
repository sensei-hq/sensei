use crate::api::events::StateEvent;
use crate::db::pg_store::PgStore;
use crate::tasks::queue::TaskQueue;
use gateway::Gateway;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct SharedState {
    pub pg: PgStore,
    pub task_queue: Arc<TaskQueue>,
    pub gateway: Arc<Gateway>,
    pub event_tx: broadcast::Sender<StateEvent>,
    /// Per-adapter capture-watchdog circuit-breaker state (in-memory; resets on restart).
    pub breaker: std::sync::Arc<crate::assistants::BreakerMap>,
    /// On-demand model-provisioning service (sensei-owned; downloads via the
    /// Ollama registry). `Some` only in an `embedded-llama-cpp` build on the
    /// default instance (see [`crate::api::gateway_init::init_gateway`]); `None`
    /// otherwise — the provisioning HTTP handlers report "not available in this
    /// build" then. The
    /// [`ModelProvisioning`](crate::api::model_provisioning::ModelProvisioning)
    /// type is present in every build (its deps — `local-engine`'s resolvers +
    /// `kernel::ReadinessProbe` — are non-optional); only its construction is
    /// feature-gated, so the field type is unconditional.
    pub provisioning: Option<Arc<crate::api::model_provisioning::ModelProvisioning>>,
}

pub type AppState = Arc<SharedState>;
