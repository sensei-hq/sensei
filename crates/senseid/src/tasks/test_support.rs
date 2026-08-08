//! Shared test-only helpers for the task engine.
//!
//! DRY: `make_ctx` was copy-pasted verbatim across ~8 handler test modules
//! (a qlty duplication finding). One definition lives here; variants that need
//! the event-broadcast receiver (`scan`) or an `Option` return (`publish_run`,
//! `advance_run`) keep their own local builder.

use std::sync::Arc;

/// A [`TaskContext`](crate::tasks::executor::TaskContext) backed by a fresh
/// `TaskQueue`, the test `PgStore`, and a noop gateway — the standard fixture for
/// task-handler unit tests.
pub(crate) async fn make_ctx() -> Arc<crate::tasks::executor::TaskContext> {
    let queue = Arc::new(crate::tasks::queue::TaskQueue::new());
    let gateway = crate::api::gateway_init::init_gateway_test().await;
    let app_state = Arc::new(crate::api::state::SharedState {
        task_queue: queue.clone(),
        pg: crate::db::pg_store::PgStore::connect_test().await.unwrap(),
        gateway,
        event_tx: { let (tx, _) = tokio::sync::broadcast::channel(16); tx },
        breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        provisioning: None,
    });
    Arc::new(crate::tasks::executor::TaskContext {
        queue,
        app_state,
        _graph_path: None,
        logger: sensei_logger::Logger::noop(),
    })
}
