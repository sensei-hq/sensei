use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{CorsLayer, Any};
use crate::db::Store;
use crate::indexer::graph::GraphDb;
use super::routes::{create_router, SharedState, AppState};

pub async fn start_server(store: Store, graph: GraphDb, port: u16) -> std::io::Result<()> {
    let state: AppState = Arc::new(SharedState {
        store: Mutex::new(store),
        graph: Mutex::new(graph),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = create_router(state).layer(cors);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("senseid listening on :{}", port);

    axum::serve(listener, app).await
}
