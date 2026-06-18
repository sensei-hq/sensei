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
}

pub type AppState = Arc<SharedState>;
