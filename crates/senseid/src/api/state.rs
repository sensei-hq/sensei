use std::sync::Arc;
use tokio::sync::Mutex;
use crate::db::pg_store::PgStore;
use crate::indexer::graph::GraphDb;
use crate::tasks::queue::TaskQueue;

pub struct SharedState {
    pub pg: PgStore,
    pub graph: Mutex<GraphDb>,
    pub task_queue: Arc<TaskQueue>,
}

pub type AppState = Arc<SharedState>;
