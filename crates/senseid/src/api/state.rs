use std::sync::Arc;
use crate::db::pg_store::PgStore;
use crate::tasks::queue::TaskQueue;
use gateway::Gateway;

pub struct SharedState {
    pub pg: PgStore,
    pub task_queue: Arc<TaskQueue>,
    pub gateway: Arc<Gateway>,
}

pub type AppState = Arc<SharedState>;
