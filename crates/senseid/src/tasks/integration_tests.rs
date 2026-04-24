//! Integration tests for the task queue system.
//! Tests file operations (create, modify, delete) and SSE progress tracking.

#[cfg(test)]
mod tests {
    use crate::tasks::*;
    use crate::tasks::queue::TaskQueue;
    use crate::tasks::progress::TaskEvent;
    use crate::types::{NodeKind, HierarchyNode};
    use crate::indexer::graph::GraphDb;

    // ── File operation tests ────────────────────────────────────────

    #[tokio::test]
    async fn delete_file_removes_nodes_and_edges() {
        let graph = GraphDb::open_memory().unwrap();

        // Create a file with functions
        let mut file = HierarchyNode::group("file:/repo/src/main.ts".into(), "main".into(), NodeKind::File, "repo".into());
        file.file = Some("/repo/src/main.ts".into());
        graph.merge_node(&file).unwrap();

        let fn_node = HierarchyNode::function(
            "fn:/repo/src/main.ts:hello:1".into(), "hello".into(), NodeKind::Function,
            "/repo/src/main.ts".into(), 1, None, None, None, 1, "repo".into(),
        );
        graph.merge_node(&fn_node).unwrap();
        graph.merge_edge("file:/repo/src/main.ts", "fn:/repo/src/main.ts:hello:1", "EXPORTS_FN").unwrap();

        // Verify nodes exist
        let nodes = graph.get_nodes("repo").unwrap();
        assert_eq!(nodes.len(), 2);

        // Delete file
        graph.delete_by_file("/repo/src/main.ts", "repo").unwrap();

        // Nodes should be gone
        let nodes = graph.get_nodes("repo").unwrap();
        assert_eq!(nodes.len(), 0);
        let edges = graph.get_edges("repo").unwrap();
        assert_eq!(edges.len(), 0);
    }

    #[tokio::test]
    async fn delete_folder_removes_all_files_under_it() {
        let graph = GraphDb::open_memory().unwrap();

        // Create files in a folder
        for name in &["a.ts", "b.ts", "c.ts"] {
            let path = format!("/repo/src/old/{}", name);
            let mut f = HierarchyNode::group(format!("file:{}", path), name.to_string(), NodeKind::File, "repo".into());
            f.file = Some(path.clone());
            graph.merge_node(&f).unwrap();

            let fn_id = format!("fn:{}:main:1", path);
            graph.merge_node(&HierarchyNode::function(
                fn_id.clone(), "main".into(), NodeKind::Function, path, 1, None, None, None, 1, "repo".into(),
            )).unwrap();
        }

        // Create module node
        graph.merge_node(&HierarchyNode::group("mod:repo:src/old".into(), "src/old".into(), NodeKind::Module, "repo".into())).unwrap();

        // Also a file outside the folder (should survive)
        let mut survivor = HierarchyNode::group("file:/repo/src/keep.ts".into(), "keep".into(), NodeKind::File, "repo".into());
        survivor.file = Some("/repo/src/keep.ts".into());
        graph.merge_node(&survivor).unwrap();

        assert_eq!(graph.get_nodes("repo").unwrap().len(), 8); // 3 files + 3 fns + 1 module + 1 survivor

        // Delete the folder
        let nodes = graph.get_nodes("repo").unwrap();
        for node in &nodes {
            if node.file.starts_with("/repo/src/old/") {
                graph.delete_node(&node.id).unwrap();
            }
        }
        graph.delete_node("mod:repo:src/old").unwrap();

        let remaining = graph.get_nodes("repo").unwrap();
        assert_eq!(remaining.len(), 1); // only survivor
        assert_eq!(remaining[0].name, "keep");
    }

    #[tokio::test]
    async fn modify_file_replaces_nodes() {
        let graph = GraphDb::open_memory().unwrap();

        // Initial file with one function
        let mut file = HierarchyNode::group("file:/repo/src/app.ts".into(), "app".into(), NodeKind::File, "repo".into());
        file.file = Some("/repo/src/app.ts".into());
        graph.merge_node(&file).unwrap();
        graph.merge_node(&HierarchyNode::function(
            "fn:/repo/src/app.ts:oldFn:1".into(), "oldFn".into(), NodeKind::Function,
            "/repo/src/app.ts".into(), 1, None, None, None, 1, "repo".into(),
        )).unwrap();
        graph.merge_edge("file:/repo/src/app.ts", "fn:/repo/src/app.ts:oldFn:1", "EXPORTS_FN").unwrap();

        assert_eq!(graph.get_nodes("repo").unwrap().len(), 2);

        // Simulate modify: delete old, create new
        graph.delete_by_file("/repo/src/app.ts", "repo").unwrap();
        assert_eq!(graph.get_nodes("repo").unwrap().len(), 0);

        // Re-create with different function
        graph.merge_node(&file).unwrap();
        graph.merge_node(&HierarchyNode::function(
            "fn:/repo/src/app.ts:newFn:1".into(), "newFn".into(), NodeKind::Function,
            "/repo/src/app.ts".into(), 1, None, None, None, 1, "repo".into(),
        )).unwrap();
        graph.merge_edge("file:/repo/src/app.ts", "fn:/repo/src/app.ts:newFn:1", "EXPORTS_FN").unwrap();

        let nodes = graph.get_nodes("repo").unwrap();
        assert_eq!(nodes.len(), 2);
        let fns: Vec<_> = nodes.iter().filter(|n| n.kind == "function").collect();
        assert_eq!(fns[0].name, "newFn");
    }

    #[tokio::test]
    async fn add_folder_creates_module_and_files() {
        let graph = GraphDb::open_memory().unwrap();

        // Simulate process_folder: create module
        let mut mod_node = HierarchyNode::group("mod:repo:src/new".into(), "src/new".into(), NodeKind::Module, "repo".into());
        mod_node.file = Some("src/new".into());
        mod_node.parent_id = Some("pkg:repo:(root)".into());
        graph.merge_node(&mod_node).unwrap();
        graph.merge_edge("pkg:repo:(root)", "mod:repo:src/new", "CONTAINS_MOD").unwrap();

        // Simulate process_file for files in new folder
        for (name, _line) in &[("index.ts", 1u32), ("utils.ts", 1)] {
            let path = format!("/repo/src/new/{}", name);
            let mut f = HierarchyNode::group(format!("file:{}", path), name.to_string(), NodeKind::File, "repo".into());
            f.file = Some(path.clone());
            graph.merge_node(&f).unwrap();
            graph.merge_edge("mod:repo:src/new", &format!("file:{}", path), "CONTAINS_FILE").unwrap();
        }

        let nodes = graph.get_nodes("repo").unwrap();
        let module_nodes: Vec<_> = nodes.iter().filter(|n| n.kind == "module").collect();
        let file_nodes: Vec<_> = nodes.iter().filter(|n| n.kind == "file").collect();
        assert_eq!(module_nodes.len(), 1);
        assert_eq!(file_nodes.len(), 2);

        let edges = graph.get_edges("repo").unwrap();
        let contains_file: Vec<_> = edges.iter().filter(|e| e.edge_type == "CONTAINS_FILE").collect();
        assert_eq!(contains_file.len(), 2);
    }

    // ── Unresolved refs tests ───────────────────────────────────────

    #[tokio::test]
    async fn unresolved_refs_stored_and_cleared() {
        let graph = GraphDb::open_memory().unwrap();

        graph.add_unresolved_ref("fn:a:foo:1", "calls", "bar", "repo").unwrap();
        graph.add_unresolved_ref("file:a.ts", "imports", "./utils", "repo").unwrap();
        graph.add_unresolved_ref("fn:a:baz:5", "parent", "MyClass", "repo").unwrap();

        let refs = graph.get_unresolved_refs("repo").unwrap();
        assert_eq!(refs.len(), 3);

        // Clear for specific source
        graph.clear_unresolved_refs_from("file:a.ts", "repo").unwrap();
        let refs = graph.get_unresolved_refs("repo").unwrap();
        assert_eq!(refs.len(), 2);

        // Clear all
        graph.clear_unresolved_refs("repo").unwrap();
        let refs = graph.get_unresolved_refs("repo").unwrap();
        assert_eq!(refs.len(), 0);
    }

    // ── SSE progress accumulation tests ─────────────────────────────

    #[tokio::test]
    async fn sse_accumulates_progress_counts() {
        let q = TaskQueue::new();
        let mut rx = q.sender().subscribe();

        // Enqueue 3 file tasks for repo1
        let _ids: Vec<u64> = vec![
            q.enqueue(Task::new(TaskKind::ProcessFile, "repo1", "a.ts")).await,
            q.enqueue(Task::new(TaskKind::ProcessFile, "repo1", "b.ts")).await,
            q.enqueue(Task::new(TaskKind::ProcessFile, "repo1", "c.ts")).await,
        ];

        // Progress: 3 pending
        let progress = q.progress().await;
        assert_eq!(progress["repo1"].total, 3);
        assert_eq!(progress["repo1"].pending, 3);
        assert_eq!(progress["repo1"].running, 0);

        // Drain queued events
        for _ in 0..3 { rx.recv().await.ok(); }

        // Start processing first task
        let t1 = q.next_task().await;
        rx.recv().await.ok(); // Started event

        let progress = q.progress().await;
        assert_eq!(progress["repo1"].running, 1);
        assert_eq!(progress["repo1"].pending, 2);

        // Complete first task
        q.complete(t1.id).await;
        rx.recv().await.ok(); // Completed event

        // Progress: 2 pending, 0 running
        let progress = q.progress().await;
        assert_eq!(progress["repo1"].pending, 2);
        assert_eq!(progress["repo1"].running, 0);

        // Process and complete remaining
        let t2 = q.next_task().await;
        q.complete(t2.id).await;
        let t3 = q.next_task().await;
        q.complete(t3.id).await;

        // All done — repo1 should not appear in progress (no active tasks)
        let progress = q.progress().await;
        assert!(progress.get("repo1").is_none() || progress["repo1"].total == 0);
    }

    #[tokio::test]
    async fn sse_events_track_task_lifecycle() {
        let q = TaskQueue::new();
        let mut rx = q.sender().subscribe();

        let id = q.enqueue(Task::new(TaskKind::ProcessFile, "repo", "file.ts")).await;

        // Event 1: Queued
        let evt = rx.recv().await.unwrap();
        match evt {
            TaskEvent::Queued { task_id } => assert_eq!(task_id, id),
            _ => panic!("expected Queued event"),
        }

        // Process
        let task = q.next_task().await;

        // Event 2: Started
        let evt = rx.recv().await.unwrap();
        match evt {
            TaskEvent::Started { task_id, repo_id, kind, path } => {
                assert_eq!(task_id, id);
                assert_eq!(repo_id, "repo");
                assert_eq!(kind, "process_file");
                assert_eq!(path, "file.ts");
            }
            _ => panic!("expected Started event"),
        }

        // Complete
        q.complete(task.id).await;

        // Event 3: Completed
        let evt = rx.recv().await.unwrap();
        match evt {
            TaskEvent::Completed { task_id, repo_id, kind } => {
                assert_eq!(task_id, id);
                assert_eq!(repo_id, "repo");
                assert_eq!(kind, "process_file");
            }
            _ => panic!("expected Completed event"),
        }
    }

    #[tokio::test]
    async fn sse_failed_event() {
        let q = TaskQueue::new();
        let mut rx = q.sender().subscribe();

        let id = q.enqueue(Task::new(TaskKind::ProcessFile, "repo", "bad.ts")).await;
        rx.recv().await.ok(); // Queued

        let task = q.next_task().await;
        rx.recv().await.ok(); // Started

        q.fail(task.id, "parse error".into()).await;

        let evt = rx.recv().await.unwrap();
        match evt {
            TaskEvent::Failed { task_id, error, .. } => {
                assert_eq!(task_id, id);
                assert_eq!(error, "parse error");
            }
            _ => panic!("expected Failed event"),
        }
    }

    // ── Multi-repo concurrency test ─────────────────────────────────

    #[tokio::test]
    async fn multi_repo_tasks_tracked_separately() {
        let q = TaskQueue::new();

        q.enqueue(Task::new(TaskKind::ProcessFile, "repo1", "a.ts")).await;
        q.enqueue(Task::new(TaskKind::ProcessFile, "repo1", "b.ts")).await;
        q.enqueue(Task::new(TaskKind::ProcessFile, "repo2", "x.ts")).await;

        let progress = q.progress().await;
        assert_eq!(progress["repo1"].total, 2);
        assert_eq!(progress["repo2"].total, 1);
    }

    // ── Barrier with file operations test ───────────────────────────

    #[tokio::test]
    async fn barrier_runs_after_all_file_tasks() {
        let q = TaskQueue::new();

        // Simulate: 3 files + resolve barrier
        let f1 = q.enqueue(Task::new(TaskKind::ProcessFile, "repo", "a.ts")).await;
        let f2 = q.enqueue(Task::new(TaskKind::ProcessFile, "repo", "b.ts")).await;
        let f3 = q.enqueue(Task::new(TaskKind::ProcessFile, "repo", "c.ts")).await;
        let resolve = q.enqueue(
            Task::new(TaskKind::ResolveEdges, "repo", "").blocked_by(vec![f1, f2, f3])
        ).await;
        let build = q.enqueue(
            Task::new(TaskKind::BuildConnections, "repo", "").blocked_by(vec![resolve])
        ).await;

        // Process all 3 file tasks
        for _ in 0..3 {
            let t = q.next_task().await;
            assert_eq!(t.kind, TaskKind::ProcessFile);
            q.complete(t.id).await;
        }

        // Now resolve should be runnable
        let t = q.next_task().await;
        assert_eq!(t.kind, TaskKind::ResolveEdges);
        assert_eq!(t.id, resolve);
        q.complete(t.id).await;

        // Now build_connections should be runnable
        let t = q.next_task().await;
        assert_eq!(t.kind, TaskKind::BuildConnections);
        assert_eq!(t.id, build);
        q.complete(t.id).await;
    }
}
