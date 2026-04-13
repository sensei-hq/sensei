use std::sync::Arc;
use std::path::PathBuf;
use crate::api::routes::AppState;
use crate::indexer::graph::GraphDb;
use crate::indexer::queue::{IndexQueue, IndexEvent, BroadcastProgress};
use crate::indexer::pipeline::{index_repo_with_progress, index_dirty_files};

const DEFAULT_WORKERS: usize = 3;

/// Spawn N parallel index workers.
/// Each worker opens its own SQLite connection for the graph DB.
pub async fn spawn_workers(queue: Arc<IndexQueue>, state: AppState, num_workers: Option<usize>) {
    // Get the graph DB path from the shared graph (for workers to open their own connections)
    let graph_path = {
        let graph = state.graph.lock().await;
        graph.db_path().map(|p| p.parent().unwrap().to_path_buf())
    };

    let n = num_workers.unwrap_or(DEFAULT_WORKERS);
    for worker_id in 0..n {
        let queue = queue.clone();
        let state = state.clone();
        let graph_path = graph_path.clone();

        tokio::spawn(async move {
            tracing::info!("Index worker {} started", worker_id);

            loop {
                let job = queue.next_job().await;
                let repo_id = job.repo_id.clone();
                let repo_path = job.repo_path.clone();

                let dirty = state.dirty_tracker.drain(&repo_id).await;
                let has_dirty = !dirty.code_files.is_empty() || !dirty.doc_files.is_empty();

                let is_incremental = has_dirty && !job.force && dirty.code_files.len() < 50;
                let dirty_code = dirty.code_files.clone();
                let dirty_docs = dirty.doc_files.clone();
                let progress = BroadcastProgress::new(queue.sender().clone());
                let gp = graph_path.clone();

                if is_incremental {
                    tracing::info!(
                        "[w{}] Incremental {} — {} code, {} docs",
                        worker_id, repo_id, dirty_code.len(), dirty_docs.len()
                    );
                    let _ = queue.sender().send(IndexEvent::Started {
                        repo_id: repo_id.clone(),
                        files_total: dirty_code.len() as u32 + dirty_docs.len() as u32,
                    });

                    let rid = repo_id.clone();
                    let rp = repo_path.clone();
                    let result = tokio::task::spawn_blocking(move || {
                        let graph = open_worker_graph(&gp)?;

                        let (index_result, changed_fn_ids) = index_dirty_files(
                            &graph, &rp, &rid, &dirty_code, &progress
                        )?;

                        // Doc drift
                        let changed_file_ids: Vec<String> = dirty_code.iter()
                            .map(|p| format!("file:{}", p.to_string_lossy()))
                            .collect();
                        let drifts = graph.find_drifted_docs(&changed_file_ids, &changed_fn_ids)?;
                        if !drifts.is_empty() {
                            graph.record_doc_drift(&drifts, &rid)?;
                        }

                        // Re-index dirty docs
                        for doc_path in &dirty_docs {
                            let abs = doc_path.to_string_lossy().to_string();
                            let content = std::fs::read_to_string(doc_path).unwrap_or_default();
                            if !content.is_empty() {
                                let doc_id = format!("doc:{}", abs);
                                let title = content.lines()
                                    .find(|l| l.starts_with("# "))
                                    .map(|l| l[2..].trim().to_string())
                                    .unwrap_or_default();
                                graph.merge_doc(&doc_id, &abs, &title, "doc", &rid).ok();
                            }
                        }

                        Ok::<_, String>(index_result)
                    }).await;

                    let index_err = match result {
                        Ok(Ok(r)) => {
                            { let s = state.store.lock().await; s.mark_indexed_timestamp(&repo_id).ok(); }
                            let _ = queue.sender().send(IndexEvent::Completed {
                                repo_id: repo_id.clone(),
                                files_indexed: r.files_indexed, functions_indexed: r.functions_indexed,
                                types_indexed: r.types_indexed, edges_created: r.edges_created,
                                duration_ms: r.duration_ms,
                            });
                            queue.mark_done(&repo_id).await;
                            tracing::info!("[w{}] Incremental done {} — {}ms", worker_id, repo_id, r.duration_ms);
                            None
                        }
                        Ok(Err(e)) => Some(e),
                        Err(e) => Some(format!("panic: {}", e)),
                    };
                    if let Some(e) = index_err {
                        { let s = state.store.lock().await; s.mark_index_failed(&repo_id, &e).ok(); }
                        let _ = queue.sender().send(IndexEvent::Failed { repo_id: repo_id.clone(), error: e.clone() });
                        queue.mark_failed(&repo_id).await;
                        tracing::error!("[w{}] Incremental failed {}: {}", worker_id, repo_id, e);
                    }
                } else {
                    // Full re-index
                    tracing::info!("[w{}] Full index {}", worker_id, repo_id);
                    let _ = queue.sender().send(IndexEvent::Started {
                        repo_id: repo_id.clone(), files_total: 0,
                    });

                    let rid = repo_id.clone();
                    let rp = repo_path.clone();
                    let result = tokio::task::spawn_blocking(move || {
                        let graph = open_worker_graph(&gp)?;
                        index_repo_with_progress(&graph, &rp, &rid, &progress)
                    }).await;

                    match result {
                        Ok(Ok(r)) => {
                            {
                                let s = state.store.lock().await;
                                if r.files_indexed > 0 || !r.libs.is_empty() {
                                    s.mark_indexed(&repo_id, &r.libs).ok();
                                } else {
                                    s.mark_indexed_timestamp(&repo_id).ok();
                                }
                            }
                            let _ = queue.sender().send(IndexEvent::Completed {
                                repo_id: repo_id.clone(),
                                files_indexed: r.files_indexed, functions_indexed: r.functions_indexed,
                                types_indexed: r.types_indexed, edges_created: r.edges_created,
                                duration_ms: r.duration_ms,
                            });
                            queue.mark_done(&repo_id).await;
                            tracing::info!(
                                "[w{}] Indexed {} — {} files, {} fns, {} libs, {}ms",
                                worker_id, repo_id, r.files_indexed, r.functions_indexed, r.libs.len(), r.duration_ms
                            );
                        }
                        Ok(Err(e)) => {
                            { let s = state.store.lock().await; s.mark_index_failed(&repo_id, &e).ok(); }
                            let _ = queue.sender().send(IndexEvent::Failed { repo_id: repo_id.clone(), error: e.clone() });
                            queue.mark_failed(&repo_id).await;
                            tracing::error!("[w{}] Index failed {}: {}", worker_id, repo_id, e);
                        }
                        Err(e) => {
                            let err = format!("panic: {}", e);
                            { let s = state.store.lock().await; s.mark_index_failed(&repo_id, &err).ok(); }
                            let _ = queue.sender().send(IndexEvent::Failed { repo_id: repo_id.clone(), error: err });
                            queue.mark_failed(&repo_id).await;
                        }
                    }
                }
            }
        });
    }
}

/// Backward compat.
pub async fn spawn_worker(queue: Arc<IndexQueue>, state: AppState) {
    spawn_workers(queue, state, Some(DEFAULT_WORKERS)).await;
}

/// Open a worker-local graph DB connection.
fn open_worker_graph(path: &Option<PathBuf>) -> Result<GraphDb, String> {
    match path {
        Some(p) => GraphDb::open(p),
        None => GraphDb::open_memory(),
    }
}
