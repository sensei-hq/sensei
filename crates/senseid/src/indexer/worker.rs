use std::sync::Arc;
use crate::api::routes::AppState;
use crate::indexer::queue::{IndexQueue, IndexEvent, BroadcastProgress};
use crate::indexer::pipeline::{index_repo_with_progress, index_dirty_files};

/// Spawn the background index worker. Processes jobs one at a time from the queue.
pub fn spawn_worker(queue: Arc<IndexQueue>, state: AppState) {
    tokio::spawn(async move {
        loop {
            let job = queue.next_job().await;
            let repo_id = job.repo_id.clone();
            let repo_path = job.repo_path.clone();

            // Check for dirty files — if present, do incremental re-index
            let dirty = state.dirty_tracker.drain(&repo_id).await;
            let has_dirty = !dirty.code_files.is_empty() || !dirty.doc_files.is_empty();

            if has_dirty && !job.force && dirty.code_files.len() < 50 {
                // Incremental only when not forced and dirty set is small
                tracing::info!(
                    "Incremental re-index {} — {} code files, {} doc files",
                    repo_id, dirty.code_files.len(), dirty.doc_files.len()
                );

                let _ = queue.sender().send(IndexEvent::Started {
                    repo_id: repo_id.clone(),
                    files_total: dirty.code_files.len() as u32 + dirty.doc_files.len() as u32,
                });

                let progress = BroadcastProgress::new(queue.sender().clone());
                let state_clone = state.clone();
                let repo_id_clone = repo_id.clone();
                let dirty_code = dirty.code_files.clone();

                let result = tokio::task::spawn_blocking(move || {
                    let graph = state_clone.graph.blocking_lock();

                    // 1. Re-index dirty code files
                    let (index_result, changed_fn_ids) = index_dirty_files(
                        &graph, &repo_path, &repo_id_clone, &dirty_code, &progress
                    )?;

                    // 2. Doc drift detection — find docs linked to changed files/functions
                    let changed_file_ids: Vec<String> = dirty_code.iter()
                        .map(|p| format!("file:{}", p.to_string_lossy()))
                        .collect();
                    let drifts = graph.find_drifted_docs(&changed_file_ids, &changed_fn_ids)?;

                    if !drifts.is_empty() {
                        tracing::info!("{} docs may have drifted for {}", drifts.len(), repo_id_clone);
                        graph.record_doc_drift(&drifts, &repo_id_clone)?;
                    }

                    // 3. Re-index dirty doc files (if any)
                    if !dirty.doc_files.is_empty() {
                        for doc_path in &dirty.doc_files {
                            // Re-run doc indexer for individual files
                            let abs = doc_path.to_string_lossy().to_string();
                            let content = std::fs::read_to_string(doc_path).unwrap_or_default();
                            if !content.is_empty() {
                                let rel = doc_path.strip_prefix(&repo_path)
                                    .unwrap_or(doc_path)
                                    .to_string_lossy().to_string();
                                let doc_id = format!("doc:{}", abs);
                                let title = content.lines()
                                    .find(|l| l.starts_with("# "))
                                    .map(|l| l[2..].trim().to_string())
                                    .unwrap_or(rel.clone());
                                graph.merge_doc(&doc_id, &abs, &title, "doc", &repo_id_clone).ok();
                            }
                        }
                    }

                    Ok::<_, String>(index_result)
                }).await;

                match result {
                    Ok(Ok(r)) => {
                        // Incremental doesn't detect libs — just update timestamp
                        { let s = state.store.lock().await; s.mark_indexed_timestamp(&repo_id).ok(); }
                        let _ = queue.sender().send(IndexEvent::Completed {
                            repo_id: repo_id.clone(),
                            files_indexed: r.files_indexed,
                            functions_indexed: r.functions_indexed,
                            types_indexed: r.types_indexed,
                            edges_created: r.edges_created,
                            duration_ms: r.duration_ms,
                        });
                        queue.mark_done(&repo_id).await;
                        tracing::info!("Incremental done {} — {} files, {}ms", repo_id, r.files_indexed, r.duration_ms);
                    }
                    Ok(Err(e)) => {
                        { let s = state.store.lock().await; s.mark_index_failed(&repo_id, &e).ok(); }
                        let _ = queue.sender().send(IndexEvent::Failed { repo_id: repo_id.clone(), error: e.clone() });
                        queue.mark_failed(&repo_id).await;
                        tracing::error!("Incremental index failed for {}: {}", repo_id, e);
                    }
                    Err(e) => {
                        let err = format!("Worker panic: {}", e);
                        { let s = state.store.lock().await; s.mark_index_failed(&repo_id, &err).ok(); }
                        let _ = queue.sender().send(IndexEvent::Failed { repo_id: repo_id.clone(), error: err });
                        queue.mark_failed(&repo_id).await;
                    }
                }
            } else {
                // Full re-index
                tracing::info!("Full index {} at {}", repo_id, repo_path);
                let _ = queue.sender().send(IndexEvent::Started {
                    repo_id: repo_id.clone(), files_total: 0,
                });

                let progress = BroadcastProgress::new(queue.sender().clone());
                let state_clone = state.clone();
                let repo_id_clone = repo_id.clone();

                let result = tokio::task::spawn_blocking(move || {
                    let graph = state_clone.graph.blocking_lock();
                    index_repo_with_progress(&graph, &repo_path, &repo_id_clone, &progress)
                }).await;

                match result {
                    Ok(Ok(r)) => {
                        let lib_count = r.libs.len();
                        {
                            let s = state.store.lock().await;
                            // Only update libs if we actually parsed files (avoid overwriting with empty on no-op index)
                            if r.files_indexed > 0 || r.libs.len() > 0 {
                                if let Err(e) = s.mark_indexed(&repo_id, &r.libs) {
                                    tracing::error!("mark_indexed failed for {}: {}", repo_id, e);
                                }
                            } else {
                                // Just update indexed_at timestamp without clearing libs
                                s.mark_indexed_timestamp(&repo_id).ok();
                            }
                        }
                        let _ = queue.sender().send(IndexEvent::Completed {
                            repo_id: repo_id.clone(),
                            files_indexed: r.files_indexed,
                            functions_indexed: r.functions_indexed,
                            types_indexed: r.types_indexed,
                            edges_created: r.edges_created,
                            duration_ms: r.duration_ms,
                        });
                        queue.mark_done(&repo_id).await;
                        tracing::info!(
                            "Indexed {} — {} files, {} fns, {} libs, {}ms",
                            repo_id, r.files_indexed, r.functions_indexed, lib_count, r.duration_ms
                        );
                    }
                    Ok(Err(e)) => {
                        { let s = state.store.lock().await; s.mark_index_failed(&repo_id, &e).ok(); }
                        let _ = queue.sender().send(IndexEvent::Failed { repo_id: repo_id.clone(), error: e.clone() });
                        queue.mark_failed(&repo_id).await;
                        tracing::error!("Index failed for {}: {}", repo_id, e);
                    }
                    Err(e) => {
                        let err = format!("Worker panic: {}", e);
                        { let s = state.store.lock().await; s.mark_index_failed(&repo_id, &err).ok(); }
                        let _ = queue.sender().send(IndexEvent::Failed { repo_id: repo_id.clone(), error: err });
                        queue.mark_failed(&repo_id).await;
                    }
                }
            }
        }
    });
}

/// After indexing a repo, check if it's part of any solution and run cross-repo analysis.
async fn run_cross_repo_analysis(state: &AppState, repo_id: &str) {
    let store = state.store.lock().await;
    let solutions = match store.list_solutions() {
        Ok(s) => s,
        Err(_) => return,
    };

    for solution in &solutions {
        let in_solution = solution.repos.iter().any(|r| r.repo_id == repo_id);
        if !in_solution || solution.repos.len() < 2 { continue; }

        let graph = state.graph.lock().await;
        match crate::indexer::cross_repo::analyze_solution(&store, &graph, solution) {
            Ok(analysis) => {
                if !analysis.links.is_empty() {
                    tracing::info!(
                        "Cross-repo: solution {} has {} links, {} inferred roles",
                        solution.name, analysis.links.len(), analysis.inferred_roles.len()
                    );
                }
            }
            Err(e) => tracing::warn!("Cross-repo analysis failed for {}: {}", solution.name, e),
        }
    }
}
