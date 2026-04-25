use std::sync::Arc;
use crate::db::pg_store::PgStore;
use crate::tasks::queue::TaskQueue;

pub struct SharedState {
    pub pg: PgStore,
    pub task_queue: Arc<TaskQueue>,
}

pub type AppState = Arc<SharedState>;
